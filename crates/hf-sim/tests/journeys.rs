//! #347: away on their own and back, still one of their community.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

#[test]
fn a_curious_one_goes_away_alone_and_comes_back_still_theirs() {
    let mut w = World::generate(3471, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(6);
    w.meta.autoplan = false;
    w.ensure_nav();
    let cid = w.communities[0].id;
    // Everyone curious, and a peopled place they know of.
    for a in w.agents.iter_mut().filter(|a| a.community == Some(cid)) {
        a.traits.list = vec![hf_sim::mind::Trait::Curious];
    }
    let s = w
        .overworld
        .sites
        .iter_mut()
        .find(|s| s.people > 0)
        .expect("a peopled place");
    s.known_by.push(cid);
    let (mut went, mut back) = (None, None);
    for d in 15..800u64 {
        w.meta.tick = d * TICKS_PER_DAY + 7 * TICKS_PER_HOUR + 23;
        w.events.pending.clear();
        hf_sim::journeys::step(&mut w);
        for e in &w.events.pending {
            match &e.kind {
                EventKind::WentAway { agent, .. } if went.is_none() => went = Some(*agent),
                EventKind::CameBack { agent, .. } if Some(*agent) == went => back = Some(*agent),
                _ => {}
            }
        }
        if let Some(who) = went {
            if back.is_none() {
                // Away: off the map, still of the community.
                if let Some(a) = w.overworld.away.iter().find(|a| a.id == who) {
                    assert_eq!(a.community, Some(cid));
                    assert!(!w.agents.iter().any(|x| x.id == who));
                }
            }
        }
        if back.is_some() {
            break;
        }
    }
    let who = went.expect("someone set out");
    assert_eq!(back, Some(who), "and came back");
    let a = w.agents.iter().find(|a| a.id == who).unwrap();
    assert_eq!(a.community, Some(cid));
    assert!(a.journey.is_none());
    let _ = ids;
}
