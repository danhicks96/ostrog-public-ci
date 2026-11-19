//! #1584: what happens on the road. Steps an opening; after each step, every caravan whose `log`
//! grew has its new lines classified (`expedition::road_incident`), and each incident is counted
//! by kind, season, sky, and whether a site on the road's edge was hungry (`hungry_days >= 2`) or
//! aggrieved (`grudge >= 1`). Read-only: no rng, no change to the sim.
//! Run: cargo run --release -p hf-sim --example road_census -- <seed> <days> [steady|relentless|whimsical]
use std::collections::BTreeMap;

use hf_sim::engine::Engine;
use hf_sim::expedition::{road_incident, Purpose, RoadIncident};
use hf_sim::systems::weather;
use hf_sim::time::{Calendar, TICKS_PER_DAY};
use hf_sim::World;

fn site_of(p: &Purpose) -> u32 {
    match p {
        Purpose::Trade { site, .. } | Purpose::Scout { site } | Purpose::Fetch { site, .. } => {
            *site
        }
    }
}

fn bump(m: &mut BTreeMap<String, u32>, k: String) {
    *m.entry(k).or_default() += 1;
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(1);
    let days: u64 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(700);
    let engine: Engine = args
        .get(3)
        .map(|a| a.parse().expect("engine: steady, relentless or whimsical"))
        .unwrap_or_default();
    let mut setup = hf_sim::setup::WorldSetup::new(seed);
    setup.engine = engine;
    let mut w = World::from_setup(&setup);
    let mut seen: BTreeMap<u32, usize> = BTreeMap::new();
    let (mut kind, mut season, mut sky, mut side) = (
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
    );
    let (mut robbed_calm, mut fog_off, mut berries_winter) = (0u32, 0u32, 0u32);
    let mut deaths: BTreeMap<String, u32> = BTreeMap::new();
    for _ in 0..days * TICKS_PER_DAY {
        w.step();
        let cal = Calendar::from_tick(w.meta.tick);
        let sk = format!("{:?}", weather::current(&w).sky);
        let season_name = format!("{:?}", cal.season);
        for c in &w.overworld.caravans {
            let n = seen.entry(c.id).or_insert(0);
            let fresh: Vec<String> = c.log.iter().skip(*n).cloned().collect();
            *n = c.log.len();
            let Some(site) = w
                .overworld
                .sites
                .iter()
                .find(|s| s.id == site_of(&c.purpose))
            else {
                continue;
            };
            let pressed = w
                .overworld
                .sites
                .iter()
                .any(|s| s.edge == site.edge && (s.hungry_days >= 2 || s.grudge >= 1));
            for line in fresh {
                let Some(k) = road_incident(&line) else {
                    continue;
                };
                bump(&mut kind, format!("{k:?}"));
                bump(&mut season, format!("{season_name} {k:?}"));
                bump(&mut sky, format!("{sk} {k:?}"));
                bump(
                    &mut side,
                    format!("{:?} pressed={pressed} {k:?}", site.edge),
                );
                match k {
                    RoadIncident::Robbed | RoadIncident::Waylaid if !pressed => robbed_calm += 1,
                    RoadIncident::Fog if sk != "Fog" => fog_off += 1,
                    RoadIncident::Berries if season_name == "Winter" => berries_winter += 1,
                    _ => {}
                }
            }
        }
    }
    for a in &w.agents {
        if let Some(d) = &a.death {
            if d.contains("road") {
                bump(&mut deaths, d.clone());
            }
        }
    }
    println!("seed {seed} days {days} engine {engine:?}");
    for (name, m) in [
        ("by kind", &kind),
        ("by season", &season),
        ("by sky", &sky),
        ("by edge and pressure", &side),
        ("road deaths by cause", &deaths),
    ] {
        println!("{name}:");
        for (k, v) in m {
            println!("  {k}: {v}");
        }
    }
    println!("robberies/waylayings with no hungry or aggrieved site on that side: {robbed_calm}");
    println!("fog days whose sky was not Fog: {fog_off}");
    println!("berries in winter: {berries_winter}");
}
