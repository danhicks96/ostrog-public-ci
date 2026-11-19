//! #285: goats tamed into a herd, kept near home, bred, and killed for meat when food is short.

use hf_sim::agents::Species;
use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::DesignationKind;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

#[test]
fn a_herd_is_tamed_grows_and_feeds_them_when_food_is_short() {
    let mut w = World::generate(2851, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(2);
    w.meta.autoplan = false;
    let home = w.communities[0].home;
    let cid = w.communities[0].id;
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let goats: Vec<_> = (0..3)
        .map(|k| w.spawn_agent(Species::Goat, w.grid.surface(home.x + 10 + k, home.y)))
        .collect();
    // The settlement asks for one tamed.
    hf_sim::livestock::plan(&mut w, home, false);
    assert!(w
        .designations
        .iter()
        .any(|d| matches!(d.kind, DesignationKind::Tame(_))));
    for g in &goats {
        hf_sim::livestock::tamed(&mut w, i, *g);
    }
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Tamed { .. })));
    let kept = |w: &World| {
        w.agents
            .iter()
            .filter(|a| a.alive && a.kept == Some(cid))
            .count()
    };
    assert_eq!(kept(&w), 3);
    // Food short: one is to be killed.
    hf_sim::livestock::plan(&mut w, home, true);
    assert!(w
        .designations
        .iter()
        .any(|d| matches!(d.kind, DesignationKind::Hunt(id) if goats.contains(&id))));
    // And the herd grows.
    for d in 1..200u64 {
        w.meta.tick = d * TICKS_PER_DAY + 11 * TICKS_PER_HOUR + 321;
        hf_sim::livestock::step(&mut w);
    }
    assert!(kept(&w) > 3, "kids born to the herd: {}", kept(&w));
}
