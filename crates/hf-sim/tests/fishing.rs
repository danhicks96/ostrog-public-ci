//! #299: the waters hold fish, fished down and coming back.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::World;

#[test]
fn a_bank_is_found_fish_are_caught_and_the_water_fills_again() {
    // A valley with water near its people.
    let mut found = None;
    for seed in 0..20u64 {
        let w = World::opening_world(seed);
        let home = w.communities[0].home;
        if let Some(spot) = hf_sim::fish::spot(&w, home) {
            found = Some((w, spot));
            break;
        }
    }
    let (mut w, spot) = found.expect("water near a landing in twenty seeds");
    assert!(w.grid.get(spot).is_water());
    let i = w
        .agents
        .iter()
        .position(|a| a.alive && a.community.is_some())
        .unwrap();
    let before = hf_sim::fish::stock(&w, spot);
    let meat = |w: &World| -> u32 {
        w.items
            .iter()
            .filter(|it| it.kind == hf_sim::items::ItemKind::RawMeat)
            .map(|it| it.count)
            .sum()
    };
    let m0 = meat(&w);
    for k in 0..6u64 {
        w.meta.tick += 1 + k;
        hf_sim::fish::caught(&mut w, i, spot);
    }
    assert!(meat(&w) > m0, "fish brought back");
    let after = hf_sim::fish::stock(&w, spot);
    assert!(after < before, "fished down: {before} -> {after}");
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Fished { .. })));
    w.meta.tick += 20 * TICKS_PER_DAY;
    assert!(
        hf_sim::fish::stock(&w, spot) > after,
        "and the water fills again"
    );
    let _ = TilePos::new(0, 0, 0);
}
