//! The census behind #940: `spoken_for` (systems/decide.rs), the Talk checks and the Tend scan
//! read other people's live state without asking whether the chooser could see them. This
//! steps a world and tabs, by goal, the share of chosen Talk, Tend, Forage and EatItem whose
//! target (or, for a pick claim, the claimer) was out of the chooser's line of sight, plus the
//! day's "nothing to gather" and "they left" plan failures. Read-only.
//!
//! Run: cargo run --release -p hf-sim --example knowing_census [days] [seeds, e.g. 1,2,3,4]
use hf_sim::World;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let days: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(200);
    let seeds: Vec<u64> = args
        .get(2)
        .map(|s| s.split(',').filter_map(|x| x.parse().ok()).collect())
        .unwrap_or_else(|| vec![1, 2, 3, 4]);
    for seed in seeds {
        let mut w = World::opening_world(seed);
        for _ in 0..days * hf_sim::time::TICKS_PER_DAY {
            w.step();
            let mut k = std::mem::take(&mut w.knowing);
            k.observe_failures(&w);
            w.knowing = k;
        }
        println!("KNOWING seed {seed} days {days}: {}", w.knowing.report());
    }
}
