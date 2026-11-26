//! The census behind #487 (people teach, tell, and change from what they hear): over the opening
//! soaks, how many tellings were taken, refused or misheard, how many were withheld, and how
//! many people knew each told technique at the start and at the end.
//!
//! Run: cargo run --release -p hf-sim --example telling_census [days] [seeds, e.g. 1,2,3,4]
use hf_sim::events::EventKind as E;
use hf_sim::mind::telling::{Took, What};
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::World;
use std::collections::BTreeMap;

fn knowers(w: &World) -> BTreeMap<String, usize> {
    let mut m = BTreeMap::new();
    for t in hf_sim::social::TOLD_HOW {
        let n = w
            .agents
            .iter()
            .filter(|a| a.alive && a.techniques.knows(t))
            .count();
        m.insert(t.name().to_string(), n);
    }
    m
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let days: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(200);
    let seeds: Vec<u64> = args
        .get(2)
        .map(|s| s.split(',').filter_map(|x| x.parse().ok()).collect())
        .unwrap_or_else(|| vec![1, 2, 3, 4]);
    for seed in seeds {
        let mut w = World::opening_world(seed);
        let start = knowers(&w);
        let mut withheld = 0u64;
        let mut seen: BTreeMap<(u32, u64, u32, What), Took> = BTreeMap::new();
        let mut tallies: BTreeMap<(&str, &str), u64> = BTreeMap::new();
        for _ in 0..days * TICKS_PER_DAY {
            w.step();
            for e in &w.last_events {
                if let E::Chose { what, .. } = &e.kind {
                    if what.starts_with("kept ") && what.ends_with(" to themselves") {
                        withheld += 1;
                    }
                }
            }
            for a in w.agents.iter() {
                for t in &a.told {
                    if seen
                        .insert((a.id.0, t.tick, t.from.0, t.what), t.took)
                        .is_none()
                    {
                        let kind = match t.what {
                            What::Technique(_) => "technique",
                            What::LookAlike(_) => "look-alike",
                            What::Lesson { .. } => "lesson",
                        };
                        let took = match t.took {
                            Took::Taken => "taken",
                            Took::Refused => "refused",
                            Took::Misheard => "misheard",
                        };
                        *tallies.entry((kind, took)).or_default() += 1;
                    }
                }
            }
        }
        println!(
            "TELLING seed {seed} days {days}: {tallies:?}; withheld lines {withheld}; knowers start {start:?} end {:?}",
            knowers(&w)
        );
    }
}
