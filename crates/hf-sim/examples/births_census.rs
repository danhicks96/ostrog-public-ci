//! Count the actual birth gates on an opening without changing its decisions.
//! Run: cargo run --release -p hf-sim --example births_census -- <seed> <days> <engine>
//! Engines: steady, relentless, whimsical. Defaults: seed 1, 600 days, steady.
//! Counts partition paired couple-days, including absent and over-age partners. Crowd counts
//! rolls the uncrowded rate would have accepted but the existing crowd factor rejected.

#[path = "support/birth_census.rs"]
mod birth_census;

use hf_sim::systems::life::BirthGate;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let seed = args.first().map_or(Ok(1), |s| s.parse::<u64>())?;
    let days = args.get(1).map_or(Ok(600), |s| s.parse::<u64>())?;
    let engine = args
        .get(2)
        .map_or(Ok(hf_sim::engine::Engine::Steady), |s| s.parse())?;
    let mut setup = hf_sim::setup::WorldSetup::new(seed);
    setup.engine = engine;
    let mut w = hf_sim::World::from_setup(&setup);
    let mut counts = birth_census::BirthCounts::default();
    print!("seed,engine,days,couple_days,births");
    for gate in BirthGate::ALL {
        print!(",{}", gate.name());
    }
    println!();
    for day in 1..=days {
        for _ in 0..hf_sim::time::TICKS_PER_DAY {
            birth_census::observed_tick(&mut w, &mut counts);
        }
        if day.is_multiple_of(50) {
            eprintln!("seed {seed} {engine}: day {day}, births {}", counts.births);
        }
    }
    let total: u64 = counts.gates.values().sum();
    print!("{seed},{engine},{days},{total},{}", counts.births);
    for gate in BirthGate::ALL {
        print!(",{}", counts.gates.get(gate.name()).copied().unwrap_or(0));
    }
    println!();
    eprintln!("digest {}", hf_sim::digest::digest(&w));
    Ok(())
}
