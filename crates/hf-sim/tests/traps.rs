//! #316: snares take small game at dawn, and catch a stranger by the leg.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn meadow() -> (World, usize) {
    let mut w = World::generate(3161, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(2);
    w.meta.autoplan = false;
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    (w, i)
}

#[test]
fn a_snare_takes_a_hare_at_dawn() {
    let (mut w, i) = meadow();
    let at = w.grid.surface(45, 45);
    hf_sim::traps::set(&mut w, i, at);
    let hare = w.spawn_agent(hf_sim::agents::Species::Hare, w.grid.surface(46, 45));
    let mut took = false;
    for d in 1..20u64 {
        w.meta.tick = d * TICKS_PER_DAY + 5 * TICKS_PER_HOUR + 333;
        let h = w.agents.iter().position(|a| a.id == hare).unwrap();
        if !w.agents[h].alive {
            break;
        }
        w.agents[h].pos = w.grid.surface(46, 45);
        hf_sim::traps::step(&mut w);
        took |= w
            .events
            .pending
            .iter()
            .any(|e| matches!(e.kind, EventKind::Snared { agent } if agent == hare));
    }
    assert!(took, "the snare took it");
    assert!(w.traps.is_empty(), "and is spent");
}

#[test]
fn a_stranger_walking_onto_a_snare_is_caught_by_the_leg() {
    let (mut w, i) = meadow();
    let at = w.grid.surface(40, 40);
    hf_sim::traps::set(&mut w, i, at);
    let s = w.spawn_agent(hf_sim::agents::Species::Person, at);
    let si = w.agents.iter().position(|a| a.id == s).unwrap();
    w.agents[si].band = Some(1);
    w.meta.tick = TICKS_PER_DAY + 9 * TICKS_PER_HOUR + 333;
    hf_sim::traps::step(&mut w);
    assert!(!w.agents[si].body.wounds.is_empty());
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Snared { agent } if agent == s)));
}
