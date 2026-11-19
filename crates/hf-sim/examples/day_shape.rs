//! #926: what the hours spent "loitering" (23.8% of them, seeds 5-8) actually were, and when in
//! the day work happens. Steps and prints only; the tally itself is `view::day_shape::DayShape`.
//!
//! Run: cargo run --release -p hf-sim --example day_shape [days] [seed]

use std::fs;
use std::io::{BufRead, Seek, SeekFrom};

use hf_sim::debuglog;
use hf_sim::ids::AgentId;
use hf_sim::view::day_shape::DayShape;
use hf_sim::World;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let days: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(200);
    let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);

    let mut w = World::opening_world(seed);
    let log_path = std::env::temp_dir().join(format!("day_shape_{seed}.jsonl"));
    debuglog::configure(log_path.clone(), vec![], "day_shape".into());

    let mut day_shape = DayShape::new();
    let mut read_at: u64 = 0;
    let ticks = days * hf_sim::time::TICKS_PER_DAY;
    for _ in 0..ticks {
        w.step();
        debuglog::flush();
        read_at = read_decisions(&log_path, read_at, &mut day_shape);
        day_shape.observe_tick(&w);
    }
    debuglog::disable();
    let _ = fs::remove_file(&log_path);
    let _ = fs::remove_file(log_path.with_extension("jsonl.1"));

    let report = day_shape.report();
    for hour in 0..24 {
        let mut row = report.hours[hour].clone();
        row.sort_by(|a, b| b.1.total_cmp(&a.1));
        let total = report.total(hour);
        println!("hour {hour:02}: total {total:.1} h");
        for (label, hours) in row {
            println!("  {label}: {hours:.2} h");
        }
    }
}

/// Read every `decide` line appended to the log since byte `from`, feeding each into
/// `day_shape.observe_decision`; returns the new read offset.
fn read_decisions(path: &std::path::Path, from: u64, day_shape: &mut DayShape) -> u64 {
    let Ok(mut f) = fs::File::open(path) else {
        return from;
    };
    let Ok(len) = f.metadata().map(|m| m.len()) else {
        return from;
    };
    if len <= from {
        return from;
    }
    let _ = f.seek(SeekFrom::Start(from));
    let reader = std::io::BufReader::new(&f);
    for line in reader.lines().map_while(Result::ok) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        if v.get("t").and_then(|t| t.as_str()) != Some("decide") {
            continue;
        }
        let Some(agent) = v.get("agent").and_then(|a| a.as_u64()) else {
            continue;
        };
        let Some(chose) = v.get("chose").and_then(|c| c.as_str()) else {
            continue;
        };
        day_shape.observe_decision(AgentId(agent as u32), chose);
    }
    len
}
