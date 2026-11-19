//! The cascade census of #319: for seeds 1 to 4 (or those given), an opening stepped some
//! game-days (600 unless given), counting each cascade family's steps as they are recorded,
//! how many steps were followed by the next, the deaths that followed within ten days of a
//! weakening, a sickening of the weak or an undressed festering, and whether each community a
//! chain began in was still standing at the end.
//!
//! Run: cargo run --release -p hf-sim --example cascade_census [days] [seed...]
use std::collections::{BTreeMap, BTreeSet};

use hf_sim::agents::Species;
use hf_sim::events::EventKind as E;
use hf_sim::ids::{AgentId, CommunityId};
use hf_sim::time::TICKS_PER_DAY;

fn census(seed: u64, days: u64) -> String {
    use std::fmt::Write;
    let t = std::time::Instant::now();
    let mut w = hf_sim::World::opening_world(seed);
    let mut n: BTreeMap<&'static str, u32> = BTreeMap::new();
    // Agents a chain touched, and the tick it did.
    let mut touched: BTreeMap<AgentId, (u64, &'static str)> = BTreeMap::new();
    let mut deaths: BTreeMap<(&'static str, String), u32> = BTreeMap::new();
    let mut struck: BTreeSet<CommunityId> = BTreeSet::new();
    let mut rain_rows = BTreeSet::new();
    let mut flood_rot_rows = BTreeSet::new();
    for _ in 0..days * TICKS_PER_DAY {
        w.step();
        let tick = w.meta.tick;
        for e in &w.last_events {
            let key = match &e.kind {
                E::StoresFailed { community, .. } => {
                    struck.insert(*community);
                    "lean: stores failed"
                }
                E::Weakened { agent, .. } => {
                    touched.insert(*agent, (tick, "lean"));
                    "lean: weakened"
                }
                E::SickenedWeak { agent, .. } => {
                    touched.insert(*agent, (tick, "lean"));
                    "lean: sickened weak"
                }
                E::Burned { what, .. } if what == "roof" => "roof: burned",
                E::RainSpoiled { .. } => {
                    rain_rows.insert(e.id);
                    "roof: rain spoiled"
                }
                E::Drenched { .. } => {
                    flood_rot_rows.insert(e.id);
                    "flood: drenched"
                }
                E::VerminDrawn { cause, .. } => {
                    if rain_rows.contains(cause) {
                        "roof: vermin drawn"
                    } else if flood_rot_rows.contains(cause) {
                        "flood: vermin drawn"
                    } else {
                        "vermin drawn (other)"
                    }
                }
                E::VerminBite { .. } => "vermin: bite",
                E::Festered { .. } => "vermin: festered",
                E::HealerLost { community, .. } => {
                    struck.insert(*community);
                    "healer: last healer died"
                }
                E::RoofFell { .. } => "roof: fell",
                E::WoundFestered { agent, .. } => {
                    touched.insert(*agent, (tick, "healer"));
                    "healer: wound festered"
                }
                E::Died { agent, cause } => {
                    if let Some((at, fam)) = touched.get(agent) {
                        if tick.saturating_sub(*at) <= 10 * TICKS_PER_DAY {
                            *deaths.entry((fam, cause.clone())).or_default() += 1;
                        }
                    }
                    continue;
                }
                _ => continue,
            };
            *n.entry(key).or_default() += 1;
        }
    }
    let people = w
        .agents
        .iter()
        .filter(|a| a.alive && a.species == Species::Person)
        .count();
    let standing: Vec<String> = struck
        .iter()
        .map(|c| {
            let comm = w.communities.iter().find(|x| x.id == *c);
            let alive = comm
                .map(|x| {
                    x.members
                        .iter()
                        .filter(|m| w.agents.iter().any(|a| a.id == **m && a.alive))
                        .count()
                })
                .unwrap_or(0);
            let state = match comm {
                Some(x) if !x.dissolved => format!("standing, {alive} alive"),
                _ => "gone".into(),
            };
            format!("{}: {state}", comm.map(|x| x.name.as_str()).unwrap_or("?"))
        })
        .collect();
    let mut out = String::new();
    let _ = writeln!(out, "seed {seed}, {days} days, people at end {people}");
    for (k, v) in &n {
        let _ = writeln!(out, "  {k:<26} {v}");
    }
    let _ = writeln!(out, "  deaths within ten days of a chain step: {deaths:?}");
    let _ = writeln!(out, "  communities a chain began in: {standing:?}");
    let _ = writeln!(out, "  ({:.0} s)", t.elapsed().as_secs_f64());
    out
}

fn main() {
    let args: Vec<u64> = std::env::args()
        .skip(1)
        .filter_map(|a| a.parse().ok())
        .collect();
    let days = args.first().copied().unwrap_or(600);
    let seeds: Vec<u64> = if args.len() > 1 {
        args[1..].to_vec()
    } else {
        vec![1, 2, 3, 4]
    };
    let handles: Vec<_> = seeds
        .into_iter()
        .map(|s| std::thread::spawn(move || census(s, days)))
        .collect();
    for h in handles {
        print!("{}", h.join().unwrap());
    }
}
