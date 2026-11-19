//! The 200-day census behind docs/reviews/conditions-2026-10-19: for seeds 1 to 4, an opening
//! stepped 200 game-days, counting the conditions' events (restless dead, sightings, curses,
//! bites, suspicions) and printing the first few as lines. It found no restless dead at all,
//! which led to the fix "the remembered dead wait" (docs/history/FIXLOG.md). Slow: minutes a seed.
//!
//! Run: cargo run --release -p hf-sim --example conditions_report
use hf_sim::events::EventKind as E;
fn main() {
    for seed in 1..5u64 {
        let mut w = hf_sim::World::opening_world(seed);
        let mut c = std::collections::BTreeMap::<&str, u32>::new();
        let mut samples: Vec<String> = Vec::new();
        for _ in 0..200 * hf_sim::time::TICKS_PER_DAY {
            w.step();
            for e in &w.last_events {
                let k = match &e.kind {
                    E::Restless { .. } => "restless",
                    E::GhostSeen { .. } => "ghost seen",
                    E::AtRest { .. } => "at rest",
                    E::Cursed { .. } => "cursed",
                    E::Bitten { .. } => "bitten",
                    E::Missing { .. } => "missing",
                    E::AteDead { .. } => "ate dead",
                    E::TurnedToStone { .. } => "stone",
                    E::Unbound { .. } => "unbound",
                    E::Suspected { .. } => "suspected",
                    E::Breached { .. } => "breached",
                    _ => continue,
                };
                *c.entry(k).or_default() += 1;
                if samples.len() < 4 {
                    samples.push(hf_sim::view::narrate::line(&w, e));
                }
            }
        }
        println!("seed {seed}: {:?}", c);
        for s in samples {
            println!("   {s}");
        }
    }
}
