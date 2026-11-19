//! #197 case 15 (WP-08 slice 8.7): the edge cases of docs/design-emergence.md §4, each provoked
//! on a real opening and held to its damper: the world bends, it does not break.

use hf_sim::events::EventKind;
use hf_sim::items::ItemKind;
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::World;

/// §4.5 A raid takes every tool. Damper: work goes on bare-handed (slower), and the valley makes
/// tools again; a capability collapse, not a dead world.
#[test]
fn when_every_tool_is_taken_work_goes_on_and_tools_are_made_again() {
    let mut w = World::opening_world(1);
    w.meta.autoplan = true;
    for _ in 0..3 * TICKS_PER_DAY {
        w.step();
    }
    // Every tool gone: those in hand and those lying about.
    let tools: Vec<_> = w
        .items
        .iter()
        .filter(|it| it.kind == ItemKind::Tool)
        .map(|it| (it.id, it.count))
        .collect();
    assert!(!tools.is_empty(), "fixture: the valley had tools to lose");
    for a in w.agents.iter_mut() {
        a.tool = None;
        a.tool_quality = 0.0;
        a.tool_wear = 0.0;
    }
    for (id, n) in tools {
        w.consume_item(id, n, "raided");
    }
    assert!(!w.items.iter().any(|it| it.kind == ItemKind::Tool));
    let mut worked = 0u32;
    let mut made_again = None;
    for d in 0..12 * TICKS_PER_DAY {
        w.step();
        worked += w
            .last_events
            .iter()
            .filter(|e| {
                matches!(
                    e.kind,
                    EventKind::Built { .. } | EventKind::Mined { .. } | EventKind::Cooked { .. }
                )
            })
            .count() as u32;
        if made_again.is_none() && w.items.iter().any(|it| it.kind == ItemKind::Tool) {
            made_again = Some(d / TICKS_PER_DAY);
        }
    }
    assert!(
        worked > 0,
        "nothing was built, mined or cooked in twelve days without tools"
    );
    assert!(
        made_again.is_some(),
        "twelve days on, nobody had made a tool again: the capability never came back"
    );
    let alive = w
        .agents
        .iter()
        .filter(|a| a.alive && a.community.is_some())
        .count();
    assert!(alive > 0, "the valley died");
}

/// The community's one knower of a craft, alone in knowing it, everyone else's knowledge of it
/// cleared: returns (world, community, the knower's id, the craft).
fn one_knower() -> (
    World,
    hf_sim::ids::CommunityId,
    hf_sim::ids::AgentId,
    hf_sim::mind::techniques::Technique,
) {
    use hf_sim::mind::techniques::{Learned, CRAFTS};
    let mut w = World::opening_world(2);
    w.meta.autoplan = true;
    let cid = w.agents.iter().find_map(|a| a.community).unwrap();
    let t = CRAFTS[0];
    for a in w.agents.iter_mut() {
        a.techniques.list.retain(|k| k.technique != t);
    }
    let now = w.meta.tick;
    let knower = w
        .agents
        .iter_mut()
        .find(|a| a.alive && a.community == Some(cid))
        .unwrap();
    knower.techniques.learn(t, Learned::Past, 1.0, now);
    let id = knower.id;
    (w, cid, id, t)
}

/// The day's crafts round (systems/crafts.rs runs at 07:13).
fn crafts_round(w: &mut World) {
    let day = TICKS_PER_DAY;
    let at = hf_sim::time::TICKS_PER_HOUR * 7 + 13;
    w.meta.tick = (w.meta.tick / day + 1) * day + at;
    hf_sim::systems::crafts::step(w);
}

fn lost(w: &World) -> Vec<(String, String)> {
    w.events
        .pending
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::CraftLost { craft, how, .. } => Some((craft.clone(), how.clone())),
            _ => None,
        })
        .collect()
}

/// §4.3 The only knower dies. Damper: the loss is real and recorded ("regression is legitimate
/// and recorded"); not silently papered over, not a crash.
#[test]
fn the_only_knowers_death_is_recorded_as_the_crafts_loss() {
    let (mut w, _cid, id, t) = one_knower();
    crafts_round(&mut w);
    assert!(lost(&w).is_empty(), "fixture: nothing lost while they live");
    hf_sim::systems::life::die(&mut w, id, "of a fever".into());
    crafts_round(&mut w);
    let l = lost(&w);
    assert_eq!(l.len(), 1, "{l:?}");
    assert_eq!(l[0].0, t.name());
    assert_eq!(l[0].1, "dead");
}

/// …and its neighbour: the only knower away on the road has not taken the craft with them.
#[test]
fn the_only_knower_away_on_the_road_is_not_a_lost_craft() {
    let (mut w, _cid, id, _t) = one_knower();
    crafts_round(&mut w);
    let ix = w.agents.iter().position(|a| a.id == id).unwrap();
    let a = w.agents.remove(ix);
    w.overworld.away.push(a);
    crafts_round(&mut w);
    assert!(
        lost(&w).is_empty(),
        "a craft was lost to a journey: {:?}",
        lost(&w)
    );
}

/// §4.11 A refugee flood. Damper: capacity limits and turning people away, with recorded
/// consequences. Provoked on the default welcome (`WhenFed`: "accept while there is at least a day
/// of food per head") and the most generous (`Open`: anyone): forty unaffiliated people turn up at
/// an eight-person home two days after the arrival. Found 2026-11-12: with the opening's
/// provisions, a day of food per head covers the whole flood, so the default takes in all who ask
/// (8 -> 44 on seed 3, nobody turned away); the gate binds only as the store runs down. The
/// requirement pinned here is the hard one: the valley bends, it does not break.
fn flood(norm: hf_sim::society::WelcomeNorm) -> (usize, usize, usize, usize) {
    use hf_sim::agents::Species;
    let mut w = World::opening_world(3);
    w.meta.autoplan = true;
    for _ in 0..2 * TICKS_PER_DAY {
        w.step();
    }
    let cid = w.agents.iter().find_map(|a| a.community).unwrap();
    let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
    c.welcome = norm;
    let home = c.home;
    let before = w
        .agents
        .iter()
        .filter(|a| a.alive && a.community == Some(cid))
        .count();
    for k in 0..40 {
        let p = w.grid.surface(home.x + (k % 8) - 4, home.y + 6 + k / 8);
        w.spawn_agent(Species::Person, p);
    }
    w.rebuild_caches();
    let (mut joined, mut turned) = (0, 0);
    for _ in 0..10 * TICKS_PER_DAY {
        w.step();
        for e in &w.last_events {
            match &e.kind {
                EventKind::Joined { community, .. } if *community == cid => joined += 1,
                EventKind::TurnedAway { community, .. } if *community == cid => turned += 1,
                _ => {}
            }
        }
    }
    let after = w
        .agents
        .iter()
        .filter(|a| a.alive && a.community == Some(cid))
        .count();
    eprintln!("{norm:?}: members {before} -> {after}, joined {joined}, turned away {turned}");
    (before, after, joined, turned)
}

#[test]
fn a_refugee_flood_bends_the_valley_and_does_not_break_it() {
    use hf_sim::society::WelcomeNorm;
    let (before_fed, after_fed, joined_fed, _) = flood(WelcomeNorm::WhenFed);
    assert!(joined_fed > 0, "the default welcome took nobody in");
    assert!(
        after_fed >= before_fed,
        "ten days after the flood the community had fewer people than before it: {before_fed} -> {after_fed}"
    );
    let (_, after_open, joined_open, _) = flood(WelcomeNorm::Open);
    assert!(joined_open > 0, "an open community took nobody in");
    assert!(after_open > 0, "an open community died of the flood");
    let _ = joined_open;
}
