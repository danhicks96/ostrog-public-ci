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

/// OST-1308: the planner census (`examples/planner_census.rs`) marks a hunt unseen when no member
/// of the community has seen the animal, and seen once one has.
#[path = "../examples/planner_census.rs"]
mod planner_census;

#[test]
fn the_planner_census_flags_an_unseen_deer() {
    use hf_sim::mind::knowledge::{Belief, Known};
    let mut w = World::generate(
        1616,
        GenParams {
            width: 128,
            height: 128,
            ..GenParams::small()
        },
    );
    w.flatten(2, 2, 120, 120, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(2);
    let home = w.communities[0].home;
    // The valley's own game is out of the way: only the staged deer remain.
    for a in w.agents.iter_mut().filter(|a| a.species != Species::Person) {
        a.alive = false;
    }
    // Game 50 tiles away, ten head so the planner does not leave it to breed; nobody has seen it.
    let deer: Vec<_> = (0..10)
        .map(|k| w.spawn_agent(Species::Deer, w.grid.surface(home.x + 50, home.y + k)))
        .collect();
    // A hungry community: nothing in the larder.
    w.items.retain(|it| !it.kind.is_food());
    w.meta.tick = 5 * TICKS_PER_DAY;
    hf_sim::systems::settlement::step(&mut w);
    let hunt = w
        .designations
        .iter()
        .find(|d| matches!(d.kind, DesignationKind::Hunt(id) if deer.contains(&id)))
        .expect("a hungry community posts a hunt")
        .clone();
    assert_eq!(planner_census::unseen_hunt(&w, &hunt), Some(true));
    let mut c = planner_census::Census::default();
    c.observe(&w);
    assert_eq!((c.hunts, c.hunts_unseen), (1, 1));
    // Once a member has seen that animal, the same posting reads as seen.
    let DesignationKind::Hunt(id) = hunt.kind else {
        unreachable!()
    };
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let seen_tick = w.meta.tick;
    let pos = w.agents.iter().find(|a| a.id == id).unwrap().pos;
    w.agents[i].knowledge.map.insert(
        Known::Agent(id),
        Belief {
            pos,
            seen_tick,
            tag: 0,
            confidence: 1.0,
        },
    );
    assert_eq!(planner_census::unseen_hunt(&w, &hunt), Some(false));
}
