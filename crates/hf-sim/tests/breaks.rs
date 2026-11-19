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
