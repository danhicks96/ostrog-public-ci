//! #332: breakdowns of many kinds, each its own thing, and each person breaking their own way.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;
use std::collections::BTreeMap;

/// Everyone at the end of their rope, broken once a day for `days` days: (agent, kind) rows.
fn breaks(seed: u64, n: usize, days: u64) -> (World, Vec<(hf_sim::ids::AgentId, String)>) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    w.found_party(n);
    w.meta.autoplan = false;
    let mut out = Vec::new();
    for d in 0..days {
        for a in w.agents.iter_mut() {
            a.stress = 1.0;
            a.last_break = 0;
            a.needs.hunger = 1.0;
            a.activity = hf_sim::mind::Activity::Idle;
        }
        w.meta.tick = (d + 1) * 3 * TICKS_PER_DAY + 12 * TICKS_PER_HOUR + 900;
        w.events.pending.clear();
        hf_sim::systems::strain::step(&mut w);
        for e in &w.events.pending {
            if let EventKind::Broke { agent, kind } = &e.kind {
                out.push((*agent, kind.clone()));
            }
        }
    }
    (w, out)
}

#[test]
fn a_valley_breaks_in_many_ways_and_each_person_in_their_own() {
    let (_, rows) = breaks(3321, 20, 12);
    let kinds: std::collections::BTreeSet<&str> = rows.iter().map(|r| r.1.as_str()).collect();
    assert!(kinds.len() >= 5, "kinds seen: {kinds:?}");
    // Each person's most usual way is most of their breaks.
    let mut per: BTreeMap<hf_sim::ids::AgentId, BTreeMap<String, u32>> = BTreeMap::new();
    for (a, k) in &rows {
        *per.entry(*a).or_default().entry(k.clone()).or_default() += 1;
    }
    let (top, all): (u32, u32) = per.values().fold((0, 0), |(t, n), m| {
        (t + m.values().max().unwrap(), n + m.values().sum::<u32>())
    });
    assert!(top * 10 >= all * 4, "own way {top} of {all}");
}

#[test]
fn a_binge_eats_what_lies_near() {
    let mut w = World::generate(3322, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    w.found_party(6);
    w.meta.autoplan = false;
    let at = w.agents.iter().find(|a| a.alive).unwrap().pos;
    w.drop_item(hf_sim::items::ItemKind::Berries, 10, at);
    let food = |w: &World| -> u32 {
        w.items
            .iter()
            .filter(|it| it.kind.is_food())
            .map(|it| it.count)
            .sum()
    };
    let before = food(&w);
    // Everyone a glutton, at the end of their rope, beside the berries.
    for a in w.agents.iter_mut() {
        a.traits.list = vec![hf_sim::mind::Trait::Gourmet];
        a.pos = at;
    }
    let mut binged = false;
    for d in 0..10u64 {
        for a in w.agents.iter_mut() {
            a.stress = 1.0;
            a.last_break = 0;
        }
        w.meta.tick = (d + 1) * 3 * TICKS_PER_DAY + 12 * TICKS_PER_HOUR + 900;
        w.events.pending.clear();
        hf_sim::systems::strain::step(&mut w);
        binged |= w
            .events
            .pending
            .iter()
            .any(|e| matches!(&e.kind, EventKind::Broke { kind, .. } if kind == "a binge"));
    }
    assert!(binged);
    assert!(food(&w) < before, "the food went");
}

/// A lone, settled pair; `a` is at the end of their rope with nothing else against them.
fn pair_at_the_end_of_a_rope(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    w.found_party(2);
    w.meta.autoplan = false;
    w.meta.tick = 3 * TICKS_PER_DAY + 12 * TICKS_PER_HOUR + 900;
    w.events.pending.clear();
    let a = &mut w.agents[0];
    a.stress = 1.0;
    a.last_break = 0;
    a.thoughts.list.clear();
    a.feelings.list.clear();
    a.needs = hf_sim::mind::needs::Needs {
        hunger: 1.0,
        rest: 1.0,
        warmth: 1.0,
        social: 1.0,
        comfort: 1.0,
        thirst: 1.0,
        parched_ticks: 0,
    };
    a.activity = hf_sim::mind::Activity::Idle;
    w.agents[1].stress = 0.0;
    w
}

fn broke_over(w: &World, who: hf_sim::ids::AgentId) -> Vec<(Option<hf_sim::ids::AgentId>, String)> {
    let mut seen_broke = false;
    let mut out = Vec::new();
    for e in &w.events.pending {
        match &e.kind {
            EventKind::Broke { agent, .. } if *agent == who => seen_broke = true,
            EventKind::BrokeOver {
                agent,
                about,
                cause,
            } if *agent == who => {
                assert!(seen_broke, "BrokeOver must follow Broke");
                out.push((*about, cause.clone()));
            }
            _ => {}
        }
    }
    out
}

#[test]
fn a_break_names_the_person_it_is_over() {
    let mut w = pair_at_the_end_of_a_rope(3323);
    let (a, b) = (w.agents[0].id, w.agents[1].id);
    let tick = w.meta.tick;
    w.agents[0].feelings.feel(
        hf_sim::mind::feelings::FeelingKind::Anger,
        Some(b),
        0.7,
        tick,
        None,
    );
    hf_sim::systems::strain::step(&mut w);
    let rows = broke_over(&w, a);
    assert_eq!(rows.len(), 1, "one BrokeOver: {rows:?}");
    assert_eq!(rows[0].0, Some(b));
}

#[test]
fn a_break_with_no_cause_says_so() {
    let mut w = pair_at_the_end_of_a_rope(3324);
    let a = w.agents[0].id;
    hf_sim::systems::strain::step(&mut w);
    let rows = broke_over(&w, a);
    assert_eq!(rows.len(), 1, "one BrokeOver: {rows:?}");
    assert_eq!(rows[0].0, None);
    assert!(rows[0].1.contains("no reason"), "{rows:?}");
}
