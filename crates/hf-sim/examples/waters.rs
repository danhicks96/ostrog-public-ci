//! The waters of a few seeds as the flood model sees them (#393): each body's kind, size,
//! catchment and how much faster than its own rain it rises, and how long the survey and an
//! hour's step take. `cargo run --release -p hf-sim --example waters [seeds]`.

use std::time::Instant;

use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn main() {
    let seeds: u64 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(4);
    for seed in 1..=seeds {
        let mut w = World::generate(seed, GenParams::default());
        let t = Instant::now();
        hf_sim::floods::survey(&mut w);
        let survey = t.elapsed();
        let t = Instant::now();
        for _ in 0..100 {
            hf_sim::floods::hour(&mut w);
        }
        let hour = t.elapsed() / 100;
        // And in a storm, with the water over its banks.
        w.weather.sky = hf_sim::systems::weather::Sky::Storm;
        let t = Instant::now();
        for _ in 0..24 {
            hf_sim::floods::hour(&mut w);
        }
        let storm = t.elapsed() / 24;
        println!(
            "seed {seed}: {} bodies, survey {:.2} ms, an hour {:.1} us, a storm hour {:.1} us ({} tiles flooded)",
            w.waters.bodies.len(),
            survey.as_secs_f64() * 1000.0,
            hour.as_secs_f64() * 1e6,
            storm.as_secs_f64() * 1e6,
            w.flooded.len()
        );
        let mut bodies: Vec<_> = w.waters.bodies.iter().collect();
        bodies.sort_by_key(|b| std::cmp::Reverse(b.area()));
        for b in bodies.iter().take(8) {
            println!(
                "  {:?} area {} shore {} catchment {} gathers {:.1} at {:?}",
                b.kind,
                b.area(),
                b.shore.len(),
                b.catchment,
                b.gathers(),
                b.tiles[0]
            );
        }
    }
}
