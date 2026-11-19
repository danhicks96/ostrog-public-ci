//! #319, the later cascade families (cascade.rs): the lean winter (stores fail, the hungry
//! weaken, the weak sicken), the lost roof (rain on the stores, rot, vermin), and the lost
//! healer (the last healer dead, a wound festers undressed). Each step holds the row before it,
//! and each case has a prepared control that does not start the chain.

use hf_sim::body::{Illness, IllnessKind, Part, WoundKind};
use hf_sim::events::{EventKind, Severity};
use hf_sim::ids::{AgentId, CommunityId, EventId, TilePos};
use hf_sim::items::ItemKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

/// A flat meadow with a community of `n` at its middle, no food, no wildlife.
fn hamlet(seed: u64, n: usize) -> (World, CommunityId, Vec<AgentId>) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.agents.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    let cid = w.found_community(w.meta.founding_site);
    let ids = w.arrive_party(n, w.meta.founding_site, Some(cid));
    w.items.retain(|it| !it.kind.is_food());
    (w, cid, ids)
}

/// Day `d` of the year's winter (a year is twenty days, docs/TIME.md), at `hour`:`minute`.
fn winter(d: u64, hour: u64, minute: u64) -> u64 {
    (15 + d) * TICKS_PER_DAY + hour * TICKS_PER_HOUR + minute
}

fn find(w: &World, f: impl Fn(&EventKind) -> bool) -> Vec<(EventId, EventKind)> {
    w.events
        .pending
        .iter()
        .filter(|e| f(&e.kind))
        .map(|e| (e.id, e.kind.clone()))
        .collect()
}

fn idx(w: &World, id: AgentId) -> usize {
    w.agents.iter().position(|a| a.id == id).unwrap()
}

/// The lean winter: the morning pass finds the stores failed and the hungry weak, the daily
/// health pass gives the weak a fever from a sick neighbour, and each row holds the last.
fn lean_winter(seed: u64, stores: u32) -> World {
    let (mut w, _cid, ids) = hamlet(seed, 5);
    if stores > 0 {
        let home = w.meta.founding_site;
        w.drop_item(ItemKind::GrainSheaf, stores, home.offset(1, 0, 0));
    }
    for &id in &ids {
        let i = idx(&w, id);
        w.agents[i].needs.hunger = 0.1;
    }
    // One of them is down with a fever and the rest are about them.
    let sick = idx(&w, ids[0]);
    w.agents[sick].body.illness = Some(Illness {
        severity: 0.5,
        since: 0,
        kind: IllnessKind::Fever,
        treated: None,
    });
    for d in 0..4 {
        w.meta.tick = winter(d, 6, 53);
        hf_sim::cascade::step(&mut w);
        for &id in &ids {
            let i = idx(&w, id);
            w.agents[i].needs.hunger = 0.1;
        }
        w.meta.tick = winter(d + 1, 5, 600);
        hf_sim::systems::health::step(&mut w);
    }
    w
}

#[test]
fn a_failed_winter_store_weakens_the_hungry_and_the_weak_sicken() {
    let w = lean_winter(3, 0);
    let failed = find(&w, |k| matches!(k, EventKind::StoresFailed { .. }));
    assert_eq!(failed.len(), 1, "the stores' failure is recorded once");
    let weak = find(&w, |k| matches!(k, EventKind::Weakened { .. }));
    assert!(weak.len() >= 4, "the hungry weakened: {}", weak.len());
    for (_, k) in &weak {
        let EventKind::Weakened { cause, .. } = k else {
            unreachable!()
        };
        assert_eq!(*cause, failed[0].0, "the weakening holds the stores' row");
    }
    let sick = find(&w, |k| matches!(k, EventKind::SickenedWeak { .. }));
    assert!(!sick.is_empty(), "the weak caught the fever");
    for (_, k) in &sick {
        let EventKind::SickenedWeak { agent, cause, .. } = k else {
            unreachable!()
        };
        let row = weak
            .iter()
            .find(|(_, wk)| matches!(wk, EventKind::Weakened { agent: a, .. } if a == agent))
            .map(|(id, _)| *id)
            .expect("the sickened had been weakened");
        assert_eq!(*cause, row, "the sickening holds the weakening's row");
    }
}

#[test]
fn a_community_with_stores_does_not_start_the_lean_winter() {
    // The same hungry people, the same fever among them, but a full store.
    let w = lean_winter(3, 400);
    assert!(find(&w, |k| matches!(k, EventKind::StoresFailed { .. })).is_empty());
    assert!(find(&w, |k| matches!(k, EventKind::Weakened { .. })).is_empty());
    assert!(find(&w, |k| matches!(k, EventKind::SickenedWeak { .. })).is_empty());
}

/// A store at `store` under a roof that burned (a recorded row), and rain.
fn burned_store(seed: u64, roofed_again: bool, rain: bool) -> (World, TilePos, EventId) {
    let (mut w, _cid, _ids) = hamlet(seed, 1);
    let store = TilePos::new(34, 30, 2);
    w.drop_item(ItemKind::GrainSheaf, 40, store);
    w.meta.tick = TICKS_PER_DAY * 6 + 10 * TICKS_PER_HOUR;
    let tick = w.meta.tick;
    let burned = w.events.push(
        tick,
        Some(store),
        Severity::Notable,
        EventKind::Burned {
            at: store,
            what: "roof".into(),
        },
    );
    hf_sim::cascade::step(&mut w);
    if roofed_again {
        // Roofed again, as the builders do (building.rs).
        w.roofs.insert(
            store,
            hf_sim::building::Roof {
                material: hf_sim::world::Material::THATCH,
                quality: 0.7,
                built: tick,
                snow: 0.0,
            },
        );
    }
    w.weather.sky = if rain {
        hf_sim::systems::weather::Sky::Rain
    } else {
        hf_sim::systems::weather::Sky::Clear
    };
    for h in 0..8 {
        w.meta.tick = TICKS_PER_DAY * 6 + (11 + h) * TICKS_PER_HOUR + 53;
        hf_sim::cascade::step(&mut w);
    }
    (w, store, burned)
}

#[test]
fn rain_through_a_burned_roof_spoils_the_store_and_draws_vermin() {
    let (w, store, burned) = burned_store(5, false, true);
    // Eight hours of rain: spoiled at six, and a share more at eight.
    let spoiled = find(&w, |k| matches!(k, EventKind::RainSpoiled { .. }));
    assert_eq!(spoiled.len(), 2, "the rain spoiled the store, and went on");
    for (_, k) in &spoiled {
        let EventKind::RainSpoiled { at, cause, .. } = k else {
            unreachable!()
        };
        assert_eq!(
            (*at, *cause),
            (store, burned),
            "it holds the burned roof's row"
        );
    }
    let vermin = find(&w, |k| matches!(k, EventKind::VerminDrawn { .. }));
    assert_eq!(
        vermin.len(),
        1,
        "the spoiled food drew vermin, once to the place"
    );
    let EventKind::VerminDrawn { cause, .. } = &vermin[0].1 else {
        unreachable!()
    };
    assert_eq!(*cause, spoiled[0].0, "the vermin hold the spoiling's row");
    assert!(w.vermin.iter().any(|v| v.at == store));
}

#[test]
fn a_roof_put_back_or_dry_weather_keeps_the_store() {
    for (roofed, rain) in [(true, true), (false, false)] {
        let (w, _store, _) = burned_store(5, roofed, rain);
        assert!(
            find(&w, |k| matches!(k, EventKind::RainSpoiled { .. })).is_empty(),
            "roofed again {roofed}, rain {rain}: nothing spoiled"
        );
        assert!(w.vermin.is_empty());
        assert!(w
            .items
            .iter()
            .any(|it| it.kind == ItemKind::GrainSheaf && it.count == 40));
    }
}

/// A community of nine: `healers` of them know the poultice, and the eight others carry deep
/// undressed cuts. The first healer dies (a recorded row); four daily health passes follow.
fn wounded(seed: u64, healers: usize) -> (World, EventId) {
    let (mut w, _cid, ids) = hamlet(seed, 9);
    let now = w.meta.tick;
    for (k, &id) in ids.iter().enumerate() {
        let i = idx(&w, id);
        if k < healers {
            w.agents[i].techniques.learn(
                hf_sim::mind::Technique::Poultice,
                hf_sim::mind::techniques::Learned::Past,
                0.6,
                now,
            );
        } else {
            hf_sim::systems::health::wound(
                &mut w,
                i,
                Part::TORSO,
                WoundKind::Cut,
                0.6,
                0.0,
                "a fall",
                None,
            );
            w.agents[i].body.wounds.last_mut().unwrap().bleeding = 0.0;
        }
    }
    let dead = idx(&w, ids[0]);
    w.agents[dead].alive = false;
    w.agents[dead].death = Some("old age".into());
    let at = w.agents[dead].pos;
    let died = w.events.push(
        now,
        Some(at),
        Severity::Major,
        EventKind::Died {
            agent: ids[0],
            cause: "old age".into(),
        },
    );
    hf_sim::cascade::step(&mut w);
    for d in 1..5 {
        w.meta.tick = d * TICKS_PER_DAY + 5 * TICKS_PER_HOUR + 600;
        hf_sim::systems::health::step(&mut w);
    }
    (w, died)
}

#[test]
fn with_the_last_healer_dead_a_wound_festers_undressed() {
    let (w, died) = wounded(9, 1);
    let lost = find(&w, |k| matches!(k, EventKind::HealerLost { .. }));
    assert_eq!(lost.len(), 1, "the loss of the last healer is recorded");
    let EventKind::HealerLost { cause, .. } = &lost[0].1 else {
        unreachable!()
    };
    assert_eq!(*cause, died, "it holds the death's row");
    let fest = find(&w, |k| matches!(k, EventKind::WoundFestered { .. }));
    assert!(!fest.is_empty(), "an undressed wound festered");
    for (_, k) in &fest {
        let EventKind::WoundFestered { agent, cause } = k else {
            unreachable!()
        };
        assert_eq!(*cause, lost[0].0, "it holds the healer's loss");
        let a = &w.agents[idx(&w, *agent)];
        assert!(a.body.wounds.iter().any(|wd| wd.infected));
    }
}

#[test]
fn a_community_with_a_second_healer_records_no_such_loss() {
    // The same wounds turn bad at the same rate; with a healer still living, none is laid to
    // a healer's loss.
    let (w, _) = wounded(9, 2);
    let infected = w
        .agents
        .iter()
        .filter(|a| a.body.wounds.iter().any(|wd| wd.infected))
        .count();
    assert!(infected >= 1, "wounds still turn bad: {infected}");
    assert!(find(&w, |k| matches!(k, EventKind::HealerLost { .. })).is_empty());
    assert!(find(&w, |k| matches!(k, EventKind::WoundFestered { .. })).is_empty());
}

#[test]
fn the_later_families_are_deterministic() {
    let go = || {
        let a = lean_winter(4, 0);
        let (b, _, _) = burned_store(4, false, true);
        let (c, _) = wounded(4, 1);
        (
            hf_sim::digest::digest(&a),
            hf_sim::digest::digest(&b),
            hf_sim::digest::digest(&c),
        )
    };
    assert_eq!(go(), go());
}
