//! Phase 16 gates: stacked bad thoughts break a mind; grief lowers mood for a week;
//! witnesses remember a break.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::mind::ThoughtKind;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn meadow(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    for i in 0..100 {
        w.plants.push(Plant::new(
            TilePos::new(4 + (i * 7) % 56, 4 + (i * 11) % 56, 2),
            PlantKind::BerryBush,
            1.0,
            i as u32,
        ));
    }
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    w
}

fn stack_bad(w: &mut World, i: usize) {
    let t = w.meta.tick;
    for k in [
        ThoughtKind::Quarrel,
        ThoughtKind::Quarrel,
        ThoughtKind::LostSomeone,
        ThoughtKind::SleptRough,
        ThoughtKind::Hungry,
        ThoughtKind::SawDeath,
    ] {
        w.agents[i].thoughts.add(k, t, None);
    }
}

#[test]
fn tantrum_from_stacked_thoughts() {
    let mut w = meadow(1601);
    let ids = w.found_party(6);
    w.build_house(
        24,
        24,
        9,
        7,
        1,
        hf_sim::world::Material::WOOD,
        hf_sim::world::Material::WOOD,
    );
    w.meta.autoplan = false;
    w.rebuild_caches();
    w.ensure_nav();
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    stack_bad(&mut w, i);
    let mood_before = {
        w.run(40);
        w.agents[i].mood
    };
    assert!(
        mood_before < -0.2,
        "stacked thoughts drag the mood down: {mood_before}"
    );
    let mut broke = None;
    for _ in 0..TICKS_PER_DAY * 4 {
        w.step();
        if let Some(e) = w
            .last_events
            .iter()
            .find(|e| matches!(&e.kind, EventKind::Broke { agent, .. } if *agent == ids[0]))
        {
            if let EventKind::Broke { kind, .. } = &e.kind {
                broke = Some(kind.clone());
            }
            break;
        }
        // Keep the strain on: the same bad week, every day.
        if w.tick().is_multiple_of(TICKS_PER_DAY) {
            stack_bad(&mut w, i);
        }
    }
    assert!(broke.is_some(), "the strain broke them within four days");
    assert!(w.agents[i].stress < 0.3, "the break released the stress");
}

#[test]
fn grief_changes_a_week() {
    let mut w = meadow(1602);
    let ids = w.found_party(6);
    w.meta.autoplan = false;
    w.ensure_nav();
    let (a, b) = (ids[0], ids[1]);
    let ia = w.agents.iter().position(|x| x.id == a).unwrap();
    let ib = w.agents.iter().position(|x| x.id == b).unwrap();
    w.agents[ia].partner = Some(b);
    w.agents[ib].partner = Some(a);
    // A week of ordinary life, sampled hourly, then the loss, then another week.
    let sample = |w: &mut World| -> f32 {
        let mut sum = 0.0;
        for _ in 0..24 * 7 {
            w.run(TICKS_PER_HOUR);
            sum += w.agents[ia].mood;
        }
        sum / (24.0 * 7.0)
    };
    let before = sample(&mut w);
    hf_sim::systems::life::die(&mut w, b, "a test".into());
    w.run(10);
    assert!(
        w.agents[ia].thoughts.has(ThoughtKind::LostSomeone),
        "grief is a thought"
    );
    let mut grief_days = 0;
    let mut sum = 0.0;
    for _ in 0..7 {
        for _ in 0..24 {
            w.run(TICKS_PER_HOUR);
            sum += w.agents[ia].mood;
        }
        if w.agents[ia].thoughts.has(ThoughtKind::LostSomeone) {
            grief_days += 1;
        }
    }
    let after = sum / (24.0 * 7.0);
    assert!(
        grief_days >= 6,
        "the grief thought lasts the week: {grief_days} of 7"
    );
    assert!(
        after < before - 0.1,
        "the week after was darker: {before:.2} -> {after:.2}"
    );
    assert!(
        w.agents[ia]
            .memories
            .long
            .iter()
            .any(|m| m.kind == hf_sim::mind::MemKind::Hardship
                && m.subject == hf_sim::mind::Subject::Person(b)),
        "the loss became a long memory about the person"
    );
}

#[test]
fn break_is_remembered_by_witnesses() {
    let mut w = meadow(1603);
    let ids = w.found_party(5);
    w.build_house(
        24,
        24,
        9,
        7,
        1,
        hf_sim::world::Material::WOOD,
        hf_sim::world::Material::WOOD,
    );
    w.meta.autoplan = false;
    w.rebuild_caches();
    w.ensure_nav();
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    // Everyone gathered in one spot, awake.
    for (k, id) in ids.iter().enumerate() {
        let j = w.agents.iter().position(|a| a.id == *id).unwrap();
        w.agents[j].pos = TilePos::new(34 + (k as i32 % 3), 34 + (k as i32 / 3), 2);
        w.agents[j].from = w.agents[j].pos;
        w.agents[j].needs.hunger = 0.9;
        w.agents[j].needs.rest = 0.9;
        // Standing about together for the next two hours.
        w.agents[j].activity = hf_sim::mind::Activity::Loitering {
            ticks_left: TICKS_PER_HOUR as u32 * 2,
        };
        w.agents[j].activity_ticks = 0;
        w.agents[j].path.clear();
    }
    w.agents[i].stress = 0.95;
    w.agents[i].mood = -0.8;
    let mut broke = false;
    for _ in 0..TICKS_PER_HOUR * 3 {
        w.step();
        if w.last_events
            .iter()
            .any(|e| matches!(&e.kind, EventKind::Broke { agent, .. } if *agent == ids[0]))
        {
            broke = true;
            break;
        }
    }
    assert!(broke, "the break happened in the first hours");
    w.run(20);
    let witnesses = ids[1..]
        .iter()
        .filter(|id| {
            let a = w.agents.iter().find(|a| a.id == **id).unwrap();
            a.thoughts.has(ThoughtKind::SawBreak)
                && a.memories
                    .short
                    .iter()
                    .any(|r| r.subject == hf_sim::mind::Subject::Person(ids[0]) && r.valence < 0.0)
        })
        .count();
    assert!(
        witnesses >= 2,
        "people nearby saw it and remember who: {witnesses}"
    );
}
