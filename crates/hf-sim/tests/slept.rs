//! A night in a bed is a night in a bed (FIXLOG 2026-10-23: every night read as one on the
//! ground, because the bed was read back from a short memory that sleep's own consolidation
//! had just cleared).

use hf_sim::events::{EventKind, Severity};
use hf_sim::ids::TilePos;
use hf_sim::mind::ThoughtKind;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

#[test]
fn a_night_in_a_bed_is_not_a_night_on_the_ground() {
    let mut w = World::generate(1701, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(2);
    let tick = w.meta.tick;
    for (who, bed) in [(ids[0], true), (ids[1], false)] {
        let id = w.events.push(
            tick,
            None,
            Severity::Minor,
            EventKind::Slept {
                agent: who,
                hours: 8.0,
                in_bed: bed,
            },
        );
        let e = w
            .events
            .pending
            .iter()
            .find(|e| e.id == id)
            .unwrap()
            .clone();
        w.last_events = vec![e];
        hf_sim::systems::strain::observe(&mut w);
    }
    let has = |w: &World, id, k| {
        w.agents
            .iter()
            .find(|a| a.id == id)
            .unwrap()
            .thoughts
            .list
            .iter()
            .any(|t| t.kind == k)
    };
    assert!(has(&w, ids[0], ThoughtKind::SleptInBed));
    assert!(!has(&w, ids[0], ThoughtKind::SleptRough));
    assert!(has(&w, ids[1], ThoughtKind::SleptRough));
}
