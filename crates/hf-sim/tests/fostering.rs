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

/// A six-year-old whose mother (and, in the `foster` variant, whose foster carer instead) is
/// moved to the away list in the given state. Returns whether the child was fostered over three
/// mornings.
fn fostered_when(away: &dyn Fn(&mut hf_sim::agents::Agent), alive: bool, as_foster: bool) -> bool {
    let mut w = World::generate(1583, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = 40 * TICKS_PER_YEAR;
    w.rebuild_caches();
    let ids = w.found_party(4);
    w.meta.autoplan = false;
    let pos = |w: &World, id| w.agents.iter().position(|a| a.id == id).unwrap();
    let (child, other) = (pos(&w, ids[0]), pos(&w, ids[1]));
    let t = w.meta.tick;
    w.agents[child].born = t as i64 - 6 * TICKS_PER_YEAR as i64;
    w.agents[child].age_scale = 1.0;
    w.agents[child].father = None;
    w.agents[child].mother = None;
    let id = w.agents[other].id;
    if as_foster {
        w.agents[child].foster = Some(id);
    } else {
        w.agents[child].mother = Some(id);
    }
    away(&mut w.agents[other]);
    w.agents[other].alive = alive;
    let m = w.agents.remove(other);
    w.overworld.away.push(m);
    let day = t - t % TICKS_PER_DAY + TICKS_PER_DAY;
    for d in 0..3 {
        w.meta.tick = day + d * TICKS_PER_DAY + 10 * TICKS_PER_HOUR + 99;
        hf_sim::fostering::step(&mut w);
    }
    w.events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Fostered { .. }))
}

#[test]
fn a_child_whose_mother_is_away_with_a_caravan_is_not_fostered() {
    let caravan = |a: &mut hf_sim::agents::Agent| a.caravan = Some(7);
    assert!(!fostered_when(&caravan, true, false));
    assert!(
        fostered_when(&caravan, false, false),
        "a dead mother away is gone"
    );
}

#[test]
fn a_parent_or_foster_away_on_a_journey_is_away_not_gone() {
    let journey = |a: &mut hf_sim::agents::Agent| {
        a.journey = Some((1, "here".to_string(), "there".to_string()))
    };
    assert!(!fostered_when(&journey, true, false));
    assert!(
        !fostered_when(&journey, true, true),
        "a foster away on a journey still holds"
    );
    assert!(fostered_when(&journey, false, true));
}

#[test]
fn a_parent_away_for_good_is_gone() {
    // Away with neither a caravan nor a journey (a leaver, a captive): not coming back to them.
    let gone = |_: &mut hf_sim::agents::Agent| {};
    assert!(fostered_when(&gone, true, false));
}

#[test]
fn an_orphan_goes_to_a_grandparent_first() {
    let mut w = World::generate(1213, GenParams::small());
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
    let (child, dead_parent, grand, neighbour) = (idx[0], idx[1], idx[2], idx[3]);
    let t = w.meta.tick;
    w.agents[child].born = t as i64 - 6 * TICKS_PER_YEAR as i64;
    w.agents[child].age_scale = 1.0;
    w.agents[child].mother = Some(w.agents[dead_parent].id);
    w.agents[child].father = None;
    w.agents[dead_parent].alive = false;
    w.agents[dead_parent].mother = Some(w.agents[grand].id);
    let (cid, gid) = (w.agents[child].id, w.agents[grand].id);
    w.agents[grand].relate(cid, 0.3, 0.2, t);
    w.agents[neighbour].relate(cid, 0.9, 0.5, t);
    w.meta.tick = t - t % TICKS_PER_DAY + TICKS_PER_DAY + 10 * TICKS_PER_HOUR + 99;
    hf_sim::fostering::step(&mut w);
    assert_eq!(w.agents[child].foster, Some(gid));
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Fostered { kin: 1, .. })));
}

/// Six people and a dead grandmother: a six-year-old orphan (mother dead), the mother, `a`, `b`,
/// `c` and a well-liked neighbour. Returns the world and their indexes.
fn kin_world() -> (World, [usize; 7]) {
    let mut w = World::generate(1306, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = 40 * TICKS_PER_YEAR;
    w.rebuild_caches();
    let ids = w.found_party(7);
    w.meta.autoplan = false;
    let mut idx = [0usize; 7];
    for (k, id) in ids.iter().enumerate() {
        idx[k] = w.agents.iter().position(|a| a.id == *id).unwrap();
    }
    let t = w.meta.tick;
    // idx: 0 child, 1 dead mother, 2 a, 3 b, 4 c, 5 neighbour, 6 dead grandmother.
    w.agents[idx[0]].born = t as i64 - 6 * TICKS_PER_YEAR as i64;
    w.agents[idx[0]].age_scale = 1.0;
    w.agents[idx[0]].mother = Some(w.agents[idx[1]].id);
    w.agents[idx[0]].father = None;
    w.agents[idx[1]].alive = false;
    w.agents[idx[6]].alive = false;
    w.agents[idx[1]].mother = Some(w.agents[idx[6]].id);
    (w, idx)
}

fn fostered_after(w: &mut World, child: usize, morning: u64) -> Option<(hf_sim::ids::AgentId, u8)> {
    w.agents[child].foster = None;
    w.events.pending.clear();
    let t = w.meta.tick;
    w.meta.tick = t - t % TICKS_PER_DAY + morning * TICKS_PER_DAY + 10 * TICKS_PER_HOUR + 99;
    hf_sim::fostering::step(w);
    w.events.pending.iter().find_map(|e| match e.kind {
        EventKind::Fostered { by, kin, .. } => Some((by, kin)),
        _ => None,
    })
}

#[test]
fn an_aunt_or_a_sibling_is_kin_and_beats_a_better_liked_neighbour() {
    let like = |w: &mut World, i: usize, c: usize, v: f32| {
        let (cid, t) = (w.agents[c].id, w.meta.tick);
        w.agents[i].relate(cid, v, 0.5, t);
    };
    // An aunt: shares the dead mother's mother.
    let (mut w, ix) = kin_world();
    let g = w.agents[ix[6]].id;
    w.agents[ix[2]].mother = Some(g);
    like(&mut w, ix[5], ix[0], 0.9);
    let got = fostered_after(&mut w, ix[0], 1);
    assert_eq!(got, Some((w.agents[ix[2]].id, 2)));
    // A sibling: shares the dead mother.
    let (mut w, ix) = kin_world();
    let m = w.agents[ix[1]].id;
    w.agents[ix[2]].mother = Some(m);
    like(&mut w, ix[5], ix[0], 0.9);
    let got = fostered_after(&mut w, ix[0], 1);
    assert_eq!(got, Some((w.agents[ix[2]].id, 3)));
}

#[test]
fn kin_tiers_rank_grandparent_then_aunt_then_sibling_then_the_old_rule() {
    let (mut w, ix) = kin_world();
    let (child, a, b, c, n) = (ix[0], ix[2], ix[3], ix[4], ix[5]);
    let (aid, mid, cid, t) = (
        w.agents[a].id,
        w.agents[ix[1]].id,
        w.agents[child].id,
        w.meta.tick,
    );
    // a is the dead mother's mother (grandparent); b shares her (an aunt); c shares the mother
    // (a sibling). The better the tier, the worse they are liked.
    w.agents[ix[1]].mother = Some(aid);
    w.agents[b].mother = Some(aid);
    w.agents[c].mother = Some(mid);
    w.agents[a].relate(cid, 0.1, 0.1, t);
    w.agents[b].relate(cid, 0.4, 0.3, t);
    w.agents[c].relate(cid, 0.7, 0.4, t);
    w.agents[n].relate(cid, 0.95, 0.6, t);
    let ids = [a, b, c, n].map(|k| w.agents[k].id);
    assert_eq!(fostered_after(&mut w, child, 1), Some((ids[0], 1)));
    w.agents[a].alive = false;
    assert_eq!(fostered_after(&mut w, child, 2), Some((ids[1], 2)));
    w.agents[b].alive = false;
    assert_eq!(fostered_after(&mut w, child, 3), Some((ids[2], 3)));
    // No kin left: the best-liked adult takes them, as before.
    w.agents[c].alive = false;
    assert_eq!(fostered_after(&mut w, child, 4), Some((ids[3], 0)));
}
