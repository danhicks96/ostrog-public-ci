//! Phase 7 gates: newcomers judged by a norm, splits with a recorded cause, trade between
//! two communities on one map.

use hf_sim::agents::Species;
use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::items::ItemKind;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::society::WelcomeNorm;
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn meadow(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    for i in 0..80 {
        w.plants.push(Plant::new(
            TilePos::new(4 + (i * 7) % 56, 4 + (i * 11) % 56, 2),
            PlantKind::BerryBush,
            1.0,
            i as u32,
        ));
    }
    w.meta.tick = hf_sim::time::TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    w
}

fn events_of<F: Fn(&EventKind) -> bool>(w: &World, f: F) -> usize {
    w.last_events.iter().filter(|e| f(&e.kind)).count()
}

#[test]
fn newcomer_by_norm() {
    // A fed community accepts a newcomer; a closed one turns them away.
    for (norm, expect_join) in [(WelcomeNorm::WhenFed, true), (WelcomeNorm::Closed, false)] {
        let mut w = meadow(61);
        w.meta.founding_site = TilePos::new(30, 30, 2);
        let ids = w.found_party(6);
        let cid = w
            .agents
            .iter()
            .find(|a| a.id == ids[0])
            .unwrap()
            .community
            .unwrap();
        w.communities[0].welcome = norm;
        w.drop_item(ItemKind::Meal, 40, TilePos::new(32, 32, 2));
        // A wanderer appears at the edge.
        let who = w.spawn_agent(Species::Person, TilePos::new(6, 30, 2));
        let mut joined = 0;
        let mut turned = 0;
        for _ in 0..TICKS_PER_DAY {
            w.step();
            joined += events_of(
                &w,
                |k| matches!(k, EventKind::Joined { agent, .. } if *agent == who),
            );
            turned += events_of(
                &w,
                |k| matches!(k, EventKind::TurnedAway { agent, .. } if *agent == who),
            );
            if joined + turned > 0 {
                break;
            }
        }
        let member = w.agents.iter().find(|a| a.id == who).unwrap().community == Some(cid);
        assert_eq!(
            member, expect_join,
            "norm {norm:?}: joined {joined} turned {turned}"
        );
        assert_eq!(joined + turned, 1, "exactly one verdict was recorded");
    }
}

#[test]
fn split_has_cause_event() {
    // Two cliques who dislike each other: cohesion drops, the group splits, and the Split event
    // names a leader, a count and a cause.
    let mut w = meadow(62);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    let ids = w.found_party(10);
    let tick = w.tick();
    for (i, a) in w.agents.iter_mut().enumerate() {
        for (j, other) in ids.iter().enumerate() {
            if i == j {
                continue;
            }
            let same = (i < 5) == (j < 5);
            a.relate(*other, if same { 0.6 } else { -1.0 }, 0.5, tick);
        }
    }
    let mut split = None;
    for _ in 0..TICKS_PER_DAY * 4 {
        w.step();
        for e in &w.last_events {
            if let EventKind::Split { members, cause, .. } = &e.kind {
                split = Some((*members, cause.clone()));
            }
        }
        if split.is_some() {
            break;
        }
    }
    let (members, cause) = split.expect("the group split");
    assert!(members >= 3, "a real faction left: {members}");
    assert!(!cause.is_empty());
    assert_eq!(w.communities.iter().filter(|c| !c.dissolved).count(), 2);
    let homes: Vec<TilePos> = w.communities.iter().map(|c| c.home).collect();
    assert!(
        homes[0].dist(homes[1]) >= 15,
        "the new community settles elsewhere"
    );
}

#[test]
fn two_communities_trade() {
    // One community has logs, the other meals; a carrier moves goods and both stocks change.
    let mut w = meadow(63);
    w.meta.founding_site = TilePos::new(18, 30, 2);
    let a_ids = w.found_party(5);
    let a_cid = w
        .agents
        .iter()
        .find(|a| a.id == a_ids[0])
        .unwrap()
        .community
        .unwrap();
    let b_home = TilePos::new(46, 30, 2);
    let b_cid = w.found_community(b_home);
    w.arrive_party(5, b_home, Some(b_cid));
    // Surpluses: A has lots of logs, B has lots of meals (and A none).
    w.items.retain(|it| it.kind != ItemKind::Meal);
    // Plenty: a house costs about 25 logs and five people eat ~20 meals a day.
    w.meta.autoplan = false;
    w.drop_item(ItemKind::Log, 60, TilePos::new(20, 32, 2));
    w.drop_item(ItemKind::Meal, 120, b_home.offset(2, 2, 0));
    let mut traded = 0;
    for _ in 0..TICKS_PER_DAY * 3 {
        w.step();
        traded += events_of(&w, |k| matches!(k, EventKind::Traded { .. }));
        if traded > 0 {
            break;
        }
    }
    assert!(
        traded >= 1,
        "a trade happened between {a_cid:?} and {b_cid:?}"
    );
    let meals_near_a = w.items.iter().filter(|it| it.kind == ItemKind::Meal && matches!(it.place, hf_sim::items::ItemPlace::Ground(p) if p.dist(TilePos::new(18, 30, 2)) <= 20)).map(|it| it.count).sum::<u32>();
    assert!(meals_near_a > 0, "meals arrived at the first community");
}
