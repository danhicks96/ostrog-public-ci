//! The debug decision log (#565): JSON Lines, one object per line, in a file of its own. Off by
//! default; when off, the only cost in the decision loop is one relaxed atomic load per call.
//! It reads the world and never writes to it, and draws no random numbers, so a run's digest
//! is the same with the log on or off.
//!
//! The first line is the header (`"t":"header"`): seed, git commit, game version, region,
//! history years, the settings the run started with and the command line. Then, by `t`:
//! `decide` (agent, chosen goal, every scored option best first), `plan_failed` (from the
//! `PlanFailed` events, with the reason), `abandoned` (a plan dropped for a need or because no
//! way was found), and one `day` line per game day with counters. The file is capped at
//! `MAX_BYTES`: past it, it moves to `<path>.1` (replacing the older one) and a new file begins
//! with the same header.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use serde_json::{json, Value};

use crate::events::EventKind;
use crate::flight_recorder::Recorder;
use crate::ids::TilePos;
use crate::time::TICKS_PER_DAY;
use crate::World;
use std::collections::BTreeMap;

/// A file larger than this is moved aside and started again.
pub const MAX_BYTES: u64 = 32 * 1024 * 1024;

/// The git commit this build came from (`unknown` outside a checkout).
pub const COMMIT: &str = env!("HF_GIT_COMMIT");

static ENABLED: AtomicBool = AtomicBool::new(false);
static SINK: Mutex<Option<Sink>> = Mutex::new(None);

struct Sink {
    path: PathBuf,
    settings: Vec<(String, String)>,
    max_bytes: u64,
    command_line: String,
    out: Option<BufWriter<File>>,
    written: u64,
    day: Counters,
    recorder: Option<Recorder>,
    recorder_header: bool,
}

/// What one game day has counted so far; written as a `day` line and reset.
#[derive(Default)]
struct Counters {
    made: u64,
    failed: u64,
    abandoned: u64,
    starving: u64,
    parched: u64,
    exhausted: u64,
    failed_by_reason: BTreeMap<String, u64>,
}

/// Whether the log is on. Callers build their records only when this is true.
#[inline]
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Turn the log on, writing to `path`. `settings` are the run's `key=value` pairs for the
/// header. The file is opened (and its header written, with the world's seed) at the first
/// record, so a run that makes no choices leaves no file.
pub fn configure(path: PathBuf, settings: Vec<(String, String)>, command_line: String) {
    configure_capped(path, settings, command_line, MAX_BYTES);
}

/// `configure` with a different size cap (tests).
pub fn configure_capped(
    path: PathBuf,
    settings: Vec<(String, String)>,
    command_line: String,
    max_bytes: u64,
) {
    let mut g = SINK.lock().unwrap_or_else(|e| e.into_inner());
    *g = Some(Sink {
        path,
        settings,
        max_bytes,
        command_line,
        out: None,
        written: 0,
        day: Counters::default(),
        recorder: None,
        recorder_header: false,
    });
    ENABLED.store(true, Ordering::Relaxed);
}

/// Opt in to bounded recordings in a new local directory. Existing paths are not changed.
pub fn configure_recorder(path: PathBuf, settings: Vec<(String, String)>) -> std::io::Result<()> {
    let recorder = Recorder::create(&path)?;
    let mut g = SINK.lock().unwrap_or_else(|e| e.into_inner());
    *g = Some(Sink {
        path,
        settings,
        max_bytes: 0,
        command_line: String::new(),
        out: None,
        written: 0,
        day: Counters::default(),
        recorder: Some(recorder),
        recorder_header: false,
    });
    ENABLED.store(true, Ordering::Relaxed);
    Ok(())
}

/// Turn the log off and flush it.
pub fn disable() {
    ENABLED.store(false, Ordering::Relaxed);
    let mut g = SINK.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(s) = g.as_mut() {
        if let Some(r) = s.recorder.as_mut() {
            r.finish();
        }
        if let Some(o) = s.out.as_mut() {
            let _ = o.flush();
        }
    }
    *g = None;
}

/// Flush what is buffered (call at the end of a run or before reading the file).
pub fn flush() {
    let mut g = SINK.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(s) = g.as_mut() {
        if let Some(r) = s.recorder.as_mut() {
            r.flush();
        }
        if let Some(o) = s.out.as_mut() {
            let _ = o.flush();
        }
    }
}

/// Where the log is being written, when it is on.
pub fn path() -> Option<PathBuf> {
    let g = SINK.lock().unwrap_or_else(|e| e.into_inner());
    g.as_ref().map(|s| s.path.clone())
}

/// The header object for a world: what is needed to rebuild the run.
fn header(w: &World, s: &Sink) -> Value {
    let settings: serde_json::Map<String, Value> = s
        .settings
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();
    let (region, history_years, realm_years) = match &w.meta.setup {
        Some(u) => (
            json!(u.region),
            json!(u.history_years),
            json!(u.realm_years),
        ),
        None => (Value::Null, Value::Null, Value::Null),
    };
    json!({
        "t": "header",
        "format": 2,
        "seed": w.meta.seed,
        "commit": COMMIT,
        "game_version": env!("CARGO_PKG_VERSION"),
        "world": w.meta.name,
        "region": region,
        "history_years": history_years,
        "realm_years": realm_years,
        "start_tick": w.meta.tick,
        "settings": settings,
        "command_line": s.command_line,
    })
}

fn write_record(w: &World, s: &mut Sink, value: Value) {
    let start = if !s.recorder_header && s.recorder.is_some() {
        Some(header(w, s))
    } else {
        None
    };
    if let Some(r) = s.recorder.as_mut() {
        s.recorder_header = true;
        if let Some(mut start) = start {
            start["capture"] = json!(["decision", "path", "plan_failure", "abandonment", "day"]);
            start["replay_verified"] = json!(false);
            r.record(w.meta.tick, start);
        }
        r.record(w.meta.tick, value);
    } else {
        write_legacy_line(w, s, &value.to_string());
    }
}

fn write_legacy_line(w: &World, s: &mut Sink, line: &str) {
    if s.out.is_some() && s.written >= s.max_bytes {
        if let Some(mut o) = s.out.take() {
            let _ = o.flush();
        }
        let mut old = s.path.clone().into_os_string();
        old.push(".1");
        let _ = std::fs::rename(&s.path, PathBuf::from(old));
    }
    if s.out.is_none() {
        if let Some(parent) = s.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let Ok(f) = File::create(&s.path) else {
            // An unwritable path turns the log off instead of failing every choice.
            ENABLED.store(false, Ordering::Relaxed);
            return;
        };
        let mut o = BufWriter::new(f);
        let mut head = header(w, s).to_string();
        head.push('\n');
        let _ = o.write_all(head.as_bytes());
        s.written = head.len() as u64;
        s.out = Some(o);
    }
    if let Some(o) = s.out.as_mut() {
        let _ = o.write_all(line.as_bytes());
        let _ = o.write_all(b"\n");
        s.written += line.len() as u64 + 1;
    }
}

/// A decision: `options` are `(goal, score after weights, available)`, in any order.
pub fn decision(w: &World, agent: u32, chose: &str, mut options: Vec<(String, f32, bool)>) {
    options.sort_by(|a, b| b.1.total_cmp(&a.1));
    let avail: Vec<f32> = options.iter().filter(|o| o.2).map(|o| o.1).collect();
    let margin = match avail.as_slice() {
        [a, b, ..] => json!(a - b),
        _ => Value::Null,
    };
    let opts: Vec<Value> = options
        .iter()
        .map(|(g, sc, ok)| json!({"goal": g, "score": (sc * 1000.0).round() / 1000.0, "available": ok}))
        .collect();
    let v = json!({
        "t": "decide", "tick": w.meta.tick, "agent": agent, "chose": chose,
        "why": "highest score among the available options", "margin": margin, "options": opts,
    });
    let mut g = SINK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(s) = g.as_mut() else { return };
    s.day.made += 1;
    write_record(w, s, v);
}

/// Record the actual path attempt, not a deduction from a later activity.
pub fn path_attempt(w: &World, agent: u32, from: TilePos, to: TilePos, outcome: &str) {
    let mut g = SINK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(s) = g.as_mut().filter(|s| s.recorder.is_some()) else {
        return;
    };
    write_record(
        w,
        s,
        json!({
            "t": "path_attempt", "tick": w.meta.tick, "agent": agent,
            "from": [from.x, from.y, from.z], "to": [to.x, to.y, to.z], "outcome": outcome,
        }),
    );
}

/// A plan dropped: `activity` is what they were doing or about to do.
pub fn abandoned(w: &World, agent: u32, activity: &str, reason: &str) {
    let v = json!({"t": "abandoned", "tick": w.meta.tick, "agent": agent, "activity": activity, "reason": reason});
    let mut g = SINK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(s) = g.as_mut() else { return };
    s.day.abandoned += 1;
    write_record(w, s, v);
}

/// Called once at the end of each tick while the log is on: writes the tick's failed plans (from
/// the `PlanFailed` events), counts the needs that crossed into trouble, and closes the game day.
pub fn after_tick(w: &World) {
    let mut g = SINK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(s) = g.as_mut() else { return };
    for e in &w.last_events {
        match &e.kind {
            EventKind::PlanFailed {
                agent,
                goal,
                reason,
            } => {
                s.day.failed += 1;
                *s.day
                    .failed_by_reason
                    .entry(format!("{goal}/{reason}"))
                    .or_default() += 1;
                let v = json!({"t": "plan_failed", "tick": e.tick, "agent": agent.0, "goal": goal, "reason": reason});
                write_record(w, s, v);
            }
            EventKind::Starving { .. } => s.day.starving += 1,
            EventKind::Parched { .. } => s.day.parched += 1,
            EventKind::Exhausted { .. } => s.day.exhausted += 1,
            _ => {}
        }
    }
    if w.meta.tick.is_multiple_of(TICKS_PER_DAY) {
        let d = std::mem::take(&mut s.day);
        let v = json!({
            "t": "day", "day": w.meta.tick / TICKS_PER_DAY, "tick": w.meta.tick,
            "people": w.agents.iter().filter(|a| a.alive && a.species == crate::agents::Species::Person).count(),
            "plans_made": d.made, "plans_failed": d.failed, "plans_abandoned": d.abandoned,
            "need_crossings": {"starving": d.starving, "parched": d.parched, "exhausted": d.exhausted},
            "failed_by_reason": d.failed_by_reason,
        });
        write_record(w, s, v);
    }
}
