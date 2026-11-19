//! #341: a child whose parents are gone is taken in.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR, TICKS_PER_YEAR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

#[test]
fn an_orphan_is_taken_in_by_the_one_who_thinks_best_of_them() {
    let mut w = World::generate(3411, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = 40 * TICKS_PER_YEAR;
    w.rebuild_caches();
    let ids = w.found_party(4);
    w.meta.autoplan = false;
    let idx: Vec<usize> = ids
        .iter()
        .map(|id| w.agents.iter().position(|a| a.id == *id).unwrap())
        .collect();
    let (child, kind_one) = (idx[0], idx[2]);
    let t = w.meta.tick;
    w.agents[child].born = t as i64 - 6 * TICKS_PER_YEAR as i64;
    w.agents[child].age_scale = 1.0;
    w.agents[child].mother = None;
    w.agents[child].father = None;
    let cid = w.agents[child].id;
    w.agents[kind_one].relate(cid, 0.9, 0.5, t);
    w.meta.tick = t - t % TICKS_PER_DAY + TICKS_PER_DAY + 10 * TICKS_PER_HOUR + 99;
    hf_sim::fostering::step(&mut w);
    assert_eq!(w.agents[child].foster, Some(w.agents[kind_one].id));
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Fostered { .. })));
}
