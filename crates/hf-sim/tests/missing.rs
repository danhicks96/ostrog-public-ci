//! The seven who are not here (design-organic.md section 4): each came through alone and is a
//! person in the valley from the first tick. Nothing decides their fate but the world; they
//! are found by being met, or by someone going where word says they were.

use hf_sim::agents::Species;
use hf_sim::events::EventKind;
use hf_sim::ids::{AgentId, TilePos};
use hf_sim::opening::{Rumour, SurvivorState, PARTY, STARTING};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world_state::World;

fn lost(w: &World) -> Vec<(u8, AgentId)> {
    w.survivor_pool
        .iter()
        .filter_map(|s| match s.state {
            SurvivorState::Lost {
                agent,
                found: false,
            } => Some((s.index, agent)),
            _ => None,
        })
        .collect()
}

fn with_the_party(w: &World) -> Vec<String> {
    w.agents
        .iter()
        .filter(|a| a.alive && a.species == Species::Person && a.community.is_some())
        .map(|a| a.name.clone())
        .collect()
}

#[test]
fn each_of_the_seven_is_a_person_somewhere() {
    let w = World::opening_world(3);
    let l = lost(&w);
    assert_eq!(l.len(), PARTY - STARTING, "seven came through alone");
    let home = w.meta.founding_site;
    for (_, id) in &l {
        let a = w
            .agents
            .iter()
            .find(|a| a.id == *id)
            .expect("a real person");
        assert!(a.alive, "nobody arrives dead by decree");
        assert!(a.community.is_none(), "{} is not with the others", a.name);
        assert!(!a.past.is_empty(), "and has lived");
        assert!(
            a.pos.dist(home) >= 30,
            "and is far from the landing: {}",
            a.pos
        );
    }
    assert_eq!(with_the_party(&w).len(), STARTING);
}

#[test]
fn where_they_are_is_seeded() {
    let a = World::opening_world(9);
    let b = World::opening_world(9);
    let pos = |w: &World| -> Vec<TilePos> {
        lost(w)
            .iter()
            .map(|(_, id)| w.agents.iter().find(|a| a.id == *id).unwrap().pos)
            .collect()
    };
    assert_eq!(pos(&a), pos(&b));
}

#[test]
fn missing_survivor_dies_of_a_cause() {
    // Over a hard stretch some of the lost may die. Every such death happens when it happens,
    // with a cause, as an event: nobody dies at the moment someone finds them.
    let mut deaths = 0;
    for seed in [2u64, 5, 8] {
        let mut w = World::opening_world(seed);
        let ids: Vec<AgentId> = lost(&w).iter().map(|x| x.1).collect();
        // Walking off the map for good (society.rs `leave_valley`) is not a death.
        let mut left: Vec<AgentId> = Vec::new();
        for _ in 0..TICKS_PER_DAY * 12 {
            w.step();
            for e in &w.last_events {
                if let EventKind::Left { agent } = &e.kind {
                    left.push(*agent);
                }
                if let EventKind::Died { agent, cause } = &e.kind {
                    if ids.contains(agent) {
                        deaths += 1;
                        assert!(!cause.is_empty(), "a death has a cause");
                        assert_eq!(e.tick, w.meta.tick, "recorded when it happened");
                        assert!(cause != "found dead where they fell");
                    }
                }
            }
        }
        for id in &ids {
            // Rejoined and gone off with a caravan is also somewhere.
            let a = w
                .agents
                .iter()
                .chain(w.overworld.away.iter())
                .find(|a| a.id == *id)
                .unwrap();
            if !a.alive && !left.contains(id) {
                assert!(a.death.is_some());
            }
        }
    }
    let _ = deaths;
}

#[test]
fn word_comes_only_from_someone_who_passed_them() {
    let mut w = World::opening_world(4);
    let (who, id) = lost(&w)[0];
    let at = w.agents.iter().find(|a| a.id == id).unwrap().pos;
    let home = w.meta.founding_site;
    // Two walkers: one whose way in passes right by them, one from the far side of the map.
    let near_start = w.grid.surface(at.x, at.y + 2);
    let near = w.spawn_agent(Species::Person, near_start);
    let far_start = w.grid.surface(
        (2 * home.x - at.x).clamp(2, w.grid.width - 3),
        (2 * home.y - at.y).clamp(2, w.grid.height - 3),
    );
    let far = w.spawn_agent(Species::Person, far_start);
    hf_sim::opening::passed_on_the_way(&mut w, &[near], near_start, home);
    hf_sim::opening::passed_on_the_way(&mut w, &[far], far_start, home);
    let heard = |w: &World, x: AgentId| w.agents.iter().find(|a| a.id == x).unwrap().heard.clone();
    let hn = heard(&w, near);
    assert!(
        hn.iter().any(|r| r.who == who),
        "the one who passed them says so"
    );
    let r = hn.iter().find(|r| r.who == who).unwrap();
    assert!(r.at.dist(at) <= 4, "roughly where: {} vs {}", r.at, at);
    assert!(
        !heard(&w, far).iter().any(|r| r.who == who),
        "the one who came from the far side knows nothing of them"
    );
    // Nobody else in the valley heard a thing.
    for a in w.agents.iter().filter(|a| a.id != near && a.id != far) {
        assert!(a.heard.is_empty(), "{} knows without being told", a.name);
    }
}

#[test]
fn rumour_needs_a_teller() {
    // Over days of an ordinary opening, anyone who sets out to look holds word of that person.
    for seed in [1u64, 6] {
        let mut w = World::opening_world(seed);
        for _ in 0..TICKS_PER_DAY * 6 {
            w.step();
            for s in &w.searches {
                let a = w.agents.iter().find(|a| a.id == s.by).unwrap();
                assert!(
                    a.heard.iter().any(|r| r.who == s.who),
                    "{} went looking with no word of them",
                    a.name
                );
            }
        }
    }
}

#[test]
fn met_by_sight_they_rejoin() {
    let mut w = World::opening_world(7);
    let (who, id) = lost(&w)[0];
    let at = w.agents.iter().find(|a| a.id == id).unwrap().pos;
    // One of the party walks up to them.
    let member = w.agents.iter().position(|a| a.community.is_some()).unwrap();
    w.agents[member].pos = w.grid.surface(at.x + 2, at.y);
    w.meta.tick = w.meta.epoch + 12 * TICKS_PER_HOUR;
    for _ in 0..300 {
        w.step();
        if with_the_party(&w).len() > STARTING {
            break;
        }
    }
    let slot = w.survivor_pool.iter().find(|s| s.index == who).unwrap();
    let rejoined = matches!(slot.state, SurvivorState::Starting);
    let a = w.agents.iter().find(|a| a.id == id).unwrap();
    assert!(
        rejoined || !a.alive,
        "met, they are with the others (or were already dead)"
    );
    if rejoined {
        assert!(a.community.is_some());
    }
}

#[test]
fn searching_the_wrong_ground_finds_nothing() {
    let mut w = World::opening_world(11);
    let (who, id) = lost(&w)[0];
    let at = w.agents.iter().find(|a| a.id == id).unwrap().pos;
    let wrong = w.grid.surface(
        (at.x + 30).clamp(2, w.grid.width - 3),
        (at.y + 30).clamp(2, w.grid.height - 3),
    );
    let out = hf_sim::opening::resolve_search(&mut w, who, wrong);
    assert!(!out.found);
    assert!(out.note.contains("no sign"));
    let out = hf_sim::opening::resolve_search(&mut w, who, at);
    assert!(out.found, "where they are, they are found");
    let again = hf_sim::opening::resolve_search(&mut w, who, at);
    assert!(!again.note.is_empty(), "asking again still answers");
    let _ = Rumour {
        who,
        at,
        heard: 0,
        from: None,
    };
}

#[test]
fn the_three_who_step_out_are_the_three_on_the_cards() {
    let mut rerolls = [0u32; PARTY];
    rerolls[0] = 3;
    rerolls[5] = 1;
    let w = World::opening_world_with(17, &[0, 5, 9], &rerolls);
    let card0 = hf_sim::opening::identity(w.meta.seed, 0, 3);
    let card5 = hf_sim::opening::identity(w.meta.seed, 5, 1);
    let here = with_the_party(&w);
    assert_eq!(here.len(), 3, "three begin together");
    assert!(here.contains(&card0.name), "{here:?}");
    assert!(here.contains(&card5.name), "{here:?}");
}

#[test]
fn asking_for_the_same_person_twice_gives_three_people() {
    let w = World::opening_world_with(19, &[4, 4, 4], &[0; PARTY]);
    let mut names = with_the_party(&w);
    assert_eq!(names.len(), 3);
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 3, "three different people");
    let starting = w
        .survivor_pool
        .iter()
        .filter(|s| matches!(s.state, SurvivorState::Starting))
        .count();
    assert_eq!(starting, 3, "the pool agrees");
}
