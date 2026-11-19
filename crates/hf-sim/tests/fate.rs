//! #353: how a community is faring, its last survivor, and the account of a valley that died.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn party(n: usize) -> (World, Vec<hf_sim::ids::AgentId>) {
    let mut w = World::generate(3531, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(n);
    w.meta.autoplan = false;
    (w, ids)
}

fn die(w: &mut World, id: hf_sim::ids::AgentId, cause: &str) {
    w.events.pending.clear();
    hf_sim::systems::life::die(w, id, cause.into());
    w.last_events = w.events.pending.clone();
    hf_sim::fate::observe(w);
}

fn evening(w: &mut World, day: u64) {
    w.meta.tick = day * TICKS_PER_DAY + 20 * TICKS_PER_HOUR + 7;
    hf_sim::fate::step(w);
}

#[test]
fn the_account_names_what_killed_them() {
    let a = hf_sim::fate::account(&[
        ("starvation".into(), 11),
        ("froze".into(), 5),
        ("old age".into(), 3),
        ("fever".into(), 1),
    ]);
    assert_eq!(a, "11 of starvation, 5 froze, 3 of old age");
}

#[test]
fn a_dwindling_community_collapses_and_its_last_one_is_on_record() {
    let (mut w, ids) = party(8);
    evening(&mut w, 11);
    for id in &ids[..7] {
        die(&mut w, *id, "starvation");
    }
    w.events.pending.clear();
    evening(&mut w, 12);
    assert!(w.events.pending.iter().any(|e| matches!(
        &e.kind,
        EventKind::Faring { now, .. } if now == "collapsed"
    )));
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::LastOne { agent, .. } if agent == ids[7])));
    // And when the last one goes, the ending says why.
    die(&mut w, ids[7], "froze");
    w.events.pending.clear();
    w.meta.tick += TICKS_PER_DAY;
    hf_sim::systems::moving::endings(&mut w);
    let how = w
        .events
        .pending
        .iter()
        .find_map(|e| match &e.kind {
            EventKind::CommunityEnded { how, .. } => Some(how.clone()),
            _ => None,
        })
        .expect("ended");
    assert_eq!(how, "died out (7 of starvation, 1 froze)");
}
