//! "The same news again" (#1481, part of #1480): one rule for which rows are the same piece of news,
//! and a fold that tells a run of them once with a count. The feed, the catch-up card, biographies
//! and History all read the same rule, so a reader never sees thirty nightly suspicions of one
//! sleeper as thirty pieces of news.
//!
//! Read-only: nothing here changes the simulation, saves or digests.

use crate::events::{Event, EventKind};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::world_state::World;

/// The fold key of a row: its kind and its subject, for kinds that repeat; `None` for rows that
/// are always their own news (a death, a birth, a pairing and every kind not listed here).
pub fn key(e: &Event) -> Option<String> {
    use EventKind as K;
    Some(match &e.kind {
        // Who is suspected, whoever suspects them.
        K::Suspected { of, .. } => format!("suspected:{}", of.0),
        K::TrueDream { what, .. } => format!("true_dream:{what}"),
        // The rows do not name the flood: the same place, coarsely, is the same flood.
        K::CarriedOff { at, .. } => format!("carried_off:{}:{}:{}", at.x / 32, at.y / 32, at.z),
        // Formed and lapsed stay apart, so a count never says one for the other.
        K::CustomFormed {
            community, label, ..
        } => format!("custom_formed:{}:{label}", community.0),
        K::CustomLapsed {
            community, label, ..
        } => format!("custom_lapsed:{}:{label}", community.0),
        K::Burned { what, .. } => format!("burned:{what}"),
        K::Talked { a, b, .. } => {
            let (x, y) = if a.0 <= b.0 { (a.0, b.0) } else { (b.0, a.0) };
            format!("talked:{x}:{y}")
        }
        K::RoleTaken { agent, role } => format!("role_taken:{}:{role}", agent.0),
        _ => return None,
    })
}

/// Rows told once: the first row's history id, its tick and the last one's, how many, and the
/// line (the first row's narration, with "(x N, over <span>)" when there are more).
#[derive(Clone, Debug, PartialEq)]
pub struct Folded {
    pub first_id: i64,
    pub first_tick: u64,
    pub last_tick: u64,
    pub count: u32,
    pub line: String,
}

/// Fold `rows` (history id, tick, event), in order: rows with the same key join the first such
/// row's line; rows without a key are each their own line. Lines keep the order of their first
/// row, and each keeps that row's history id.
pub fn fold(rows: &[(i64, u64, Event)], w: &World) -> Vec<Folded> {
    let mut out: Vec<Folded> = Vec::new();
    let mut at: std::collections::BTreeMap<String, usize> = Default::default();
    for (id, tick, e) in rows {
        if let Some(k) = key(e) {
            if let Some(&i) = at.get(&k) {
                let f = &mut out[i];
                f.count += 1;
                f.last_tick = f.last_tick.max(*tick);
                continue;
            }
            at.insert(k, out.len());
        }
        out.push(Folded {
            first_id: *id,
            first_tick: *tick,
            last_tick: *tick,
            count: 1,
            line: crate::view::narrate::line(w, e),
        });
    }
    for f in &mut out {
        if f.count > 1 {
            f.line = format!(
                "{} (x {}{})",
                f.line,
                f.count,
                span(f.last_tick - f.first_tick)
            );
        }
    }
    out
}

/// ", over 3 days" / ", over 5 hours"; nothing within the hour.
fn span(ticks: u64) -> String {
    let days = ticks / TICKS_PER_DAY;
    let hours = ticks / TICKS_PER_HOUR;
    if days >= 1 {
        format!(", over {days} day{}", if days == 1 { "" } else { "s" })
    } else if hours >= 1 {
        format!(", over {hours} hour{}", if hours == 1 { "" } else { "s" })
    } else {
        String::new()
    }
}
