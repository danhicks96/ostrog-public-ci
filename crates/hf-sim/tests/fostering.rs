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
