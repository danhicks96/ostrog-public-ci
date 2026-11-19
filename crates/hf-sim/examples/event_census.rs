//! Which event kinds fire in play (issue #130): openings of seeds 1 to 4, run DAYS game-days
//! (default 30), each kind counted; kinds that never fired are listed. The list of kinds is read
//! from events.rs's names at build time.
//!
//! Run: cargo run --release -p hf-sim --example event_census [days]

const NAMES: &str = include_str!("../src/events.rs");

fn main() {
    let days: u64 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(30);
    let all: std::collections::BTreeSet<String> = NAMES
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            let (_, rest) = l.split_once("} => \"")?;
            Some(rest.trim_end_matches("\",").to_string())
        })
        .collect();
    let mut total: std::collections::BTreeMap<String, u32> = Default::default();
    for seed in 1..5u64 {
        let mut w = hf_sim::World::opening_world(seed);
        for _ in 0..days * hf_sim::time::TICKS_PER_DAY {
            w.step();
            for e in &w.last_events {
                *total.entry(e.kind_name().to_string()).or_default() += 1;
            }
        }
        let people = w
            .agents
            .iter()
            .filter(|a| a.alive && a.species == hf_sim::agents::Species::Person)
            .count();
        eprintln!("seed {seed}: {people} people at day {days}");
    }
    println!("fired over 4 openings of {days} days:");
    for (k, n) in &total {
        println!("  {n:6} {k}");
    }
    let never: Vec<&String> = all.iter().filter(|k| !total.contains_key(*k)).collect();
    println!("never fired ({} of {}): {never:?}", never.len(), all.len());
}
