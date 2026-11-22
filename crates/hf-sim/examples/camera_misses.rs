//! OST-004: how many Major events that name a person the camera never showed, by kind.
//! Steps an opening world headlessly and prints raw counts and shares. Measures only: the
//! simulation and the camera's scoring are unchanged.
//!
//! Run: cargo run --release -p hf-sim --example camera_misses [days] [seed]

use hf_sim::time::TICKS_PER_DAY;
use hf_sim::view::camera_miss::{measure, SAMPLE_STRIDE};
use hf_sim::World;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let days: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(60);
    let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);

    let mut w = World::opening_world(seed);
    let tally = measure(&mut w, days * TICKS_PER_DAY, SAMPLE_STRIDE);
    let report = tally.report();
    println!("seed {seed}, {days} days, a sample every {SAMPLE_STRIDE} ticks");
    println!("kind        missed / all");
    for (kind, missed, all) in tally.glance_major_counts() {
        let share = report
            .get(&format!("glance_missed_major_{kind}"))
            .unwrap_or(0.0);
        println!(
            "{kind:<10} {missed:>6} / {all:<6} {:.1}% missed",
            share * 100.0
        );
    }
    println!(
        "all        {:>6} / {:<6} {:.1}% missed (glance_missed_major)",
        tally.glance_major_counts().iter().map(|c| c.1).sum::<u32>(),
        report.get("glance_major_n").unwrap_or(0.0),
        report.get("glance_missed_major").unwrap_or(0.0) * 100.0
    );
    println!(
        "{} events still inside their hour at the end are not counted",
        tally.glance_major_pending()
    );
}
