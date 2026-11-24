//! OST-1305: where the valley wades and how often. Read-only. Each tick, a person whose position
//! changed onto a `TileFlags::WATER` tile (`systems::movement::wading_step`) is one wading step.
//! Every 30 days it prints wading steps per person per day, and the top 5 water tiles by steps
//! with their distance from the nearest home, the distinct people who crossed there and the days
//! on which anyone did. Entering a ford and stepping along it both count; stepping out onto dry
//! ground does not.
//!
//! Run: cargo run --release -p hf-sim --example crossing_census [days [from to]]
//! (default 300 days, seeds 1 to 4).
use hf_sim::agents::Species;
use hf_sim::ids::{AgentId, TilePos};
use hf_sim::systems::movement::wading_step;
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::World;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
struct Spot {
    steps: u64,
    people: BTreeSet<AgentId>,
    days: BTreeSet<u64>,
}

fn main() {
    let a: Vec<u64> = std::env::args()
        .skip(1)
        .filter_map(|s| s.parse().ok())
        .collect();
    let days = a.first().copied().unwrap_or(300);
    let (from, to) = (
        a.get(1).copied().unwrap_or(1),
        a.get(2).copied().unwrap_or(4),
    );
    for seed in from..=to {
        let mut w = World::from_setup(&hf_sim::setup::WorldSetup::new(seed));
        let mut last: BTreeMap<AgentId, TilePos> = BTreeMap::new();
        let mut spots: BTreeMap<(i32, i32), Spot> = BTreeMap::new();
        let (mut steps, mut person_days) = (0u64, 0u64);
        for tick in 0..days * TICKS_PER_DAY {
            w.step();
            let day = tick / TICKS_PER_DAY;
            if tick % TICKS_PER_DAY == 0 {
                person_days += w
                    .agents
                    .iter()
                    .filter(|p| p.alive && p.species == Species::Person)
                    .count() as u64;
            }
            for p in w
                .agents
                .iter()
                .filter(|p| p.alive && p.species == Species::Person)
            {
                if let Some(&b) = last.get(&p.id) {
                    if wading_step(b, p.pos, &w.grid) {
                        steps += 1;
                        let s = spots.entry((p.pos.x, p.pos.y)).or_default();
                        s.steps += 1;
                        s.people.insert(p.id);
                        s.days.insert(day);
                    }
                }
                last.insert(p.id, p.pos);
            }
            if (tick + 1) % (30 * TICKS_PER_DAY) == 0 {
                let d = day + 1;
                println!(
                    "seed {seed} day {d}: {:.3} wading steps per person per day ({steps} steps, {person_days} person-days)",
                    steps as f64 / person_days.max(1) as f64
                );
                let mut top: Vec<_> = spots.iter().collect();
                top.sort_by_key(|(k, s)| (std::cmp::Reverse(s.steps), **k));
                for (k, s) in top.iter().take(5) {
                    let dist = w
                        .communities
                        .iter()
                        .map(|c| (c.home.x - k.0).abs().max((c.home.y - k.1).abs()))
                        .min()
                        .map_or("no home".to_string(), |d| format!("{d} tiles"));
                    println!(
                        "  ({},{}) {} steps, {} people, {} days, {} from nearest home",
                        k.0,
                        k.1,
                        s.steps,
                        s.people.len(),
                        s.days.len(),
                        dist
                    );
                }
            }
        }
    }
}
