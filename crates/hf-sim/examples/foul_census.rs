//! Count recorded foul-water illnesses and repeats on an opening, without changing it.
//! Run: cargo run --release -p hf-sim --example foul_census -- <seed> <days> [engine]
use hf_sim::events::EventKind;
use std::collections::BTreeMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let seed = args.first().map_or(Ok(7), |s| s.parse::<u64>())?;
    let days = args.get(1).map_or(Ok(600), |s| s.parse::<u64>())?;
    let mut setup = hf_sim::setup::WorldSetup::new(seed);
    setup.engine = args
        .get(2)
        .map_or(Ok(hf_sim::engine::Engine::Steady), |s| s.parse())?;
    let mut w = hf_sim::World::from_setup(&setup);
    let mut cases = BTreeMap::new();
    let (mut total, mut repeat, mut thirst_deaths) = (0u64, 0u64, 0u64);
    for day in 1..=days {
        for _ in 0..hf_sim::time::TICKS_PER_DAY {
            w.step();
            for event in &w.last_events {
                match &event.kind {
                    EventKind::FellIll { agent, what } if what == "the flux from foul water" => {
                        let previous = cases.entry(*agent).or_insert(0u64);
                        total += 1;
                        repeat += u64::from(*previous > 0);
                        *previous += 1;
                    }
                    EventKind::Died { cause, .. } if cause == "thirst" => thirst_deaths += 1,
                    _ => {}
                }
            }
        }
        if day.is_multiple_of(50) {
            eprintln!("seed {seed}: day {day}, foul illnesses {total}, repeats {repeat}");
        }
    }
    println!("seed,engine,days,flux_foul_total,flux_foul_first,flux_foul_repeat,thirst_deaths");
    println!(
        "{seed},{},{days},{total},{},{repeat},{thirst_deaths}",
        setup.engine,
        total - repeat
    );
    eprintln!("digest {}", hf_sim::digest::digest(&w));
    Ok(())
}
