//! #271: mushrooms spread over the floors below ground, and stop where they crowd.

use hf_sim::plants::PlantKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::World;

#[test]
fn mushrooms_spread_below_ground_and_stop_where_they_crowd() {
    let mut w = World::opening_world(3);
    let count = |w: &World| {
        w.plants
            .iter()
            .filter(|p| p.alive && p.kind == PlantKind::Mushroom)
            .count()
    };
    let before = count(&w);
    assert!(before > 0, "a valley with cave mushrooms");
    for d in 1..120u64 {
        w.meta.tick = d * TICKS_PER_DAY + 3 * TICKS_PER_HOUR + 444;
        hf_sim::cave_life::step(&mut w);
    }
    let after = count(&w);
    assert!(after > before, "{before} -> {after}");
    // Each new patch was seeded only where fewer than eight grew within six tiles, so none
    // can stand among more than eight plus the four it could seed in a day beside it.
    let ms: Vec<_> = w
        .plants
        .iter()
        .filter(|p| p.alive && p.kind == PlantKind::Mushroom)
        .map(|p| p.pos)
        .collect();
    let crowded = ms
        .iter()
        .map(|p| ms.iter().filter(|q| q.z == p.z && q.dist(*p) <= 6).count())
        .max()
        .unwrap();
    assert!(crowded < before.max(60), "{crowded} at the most crowded");
}
