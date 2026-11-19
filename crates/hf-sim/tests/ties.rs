//! #348: one who went with a split, kin left behind, belongs to both and goes across.

use hf_sim::events::{EventKind, Severity};
use hf_sim::ids::{CommunityId, TilePos};
use hf_sim::society::Community;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

#[test]
fn torn_between_two_and_going_across() {
    let mut w = World::generate(3481, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(20, 20, 2);
    w.rebuild_caches();
    let ids = w.found_party(2);
    w.meta.autoplan = false;
    w.ensure_nav();
    let old = w.communities[0].id;
    let new = CommunityId(w.next_community_id);
    w.next_community_id += 1;
    let far = w.grid.surface(45, 45);
    w.communities
        .push(Community::new(new, "Newcamp".into(), far, 0));
    let (a, b) = (
        w.agents.iter().position(|x| x.id == ids[0]).unwrap(),
        w.agents.iter().position(|x| x.id == ids[1]).unwrap(),
    );
    w.agents[a].partner = Some(ids[1]);
    w.agents[b].partner = Some(ids[0]);
    w.agents[a].community = Some(new);
    let leader = ids[0];
    let tick = w.meta.tick;
    let id = w.events.push(
        tick,
        None,
        Severity::Major,
        EventKind::Split {
            from: old,
            to: new,
            leader,
            members: 1,
            cause: "a test".into(),
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
    hf_sim::ties::observe(&mut w);
    assert_eq!(w.agents[a].tie, Some(old));
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Torn { .. })));
    let mut across = false;
    for d in 1..60u64 {
        w.meta.tick = d * TICKS_PER_DAY + 12 * TICKS_PER_HOUR + 7;
        w.agents[a].activity = hf_sim::mind::Activity::Idle;
        hf_sim::ties::step(&mut w);
        if matches!(
            w.agents[a].activity,
            hf_sim::mind::Activity::Wandering { .. }
        ) {
            across = true;
            break;
        }
    }
    assert!(across, "they go across to their kin");
}
