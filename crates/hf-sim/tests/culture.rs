//! Phase 8b gates: customs trace to real recorded events, and they change behaviour.

use hf_sim::chronicle::Chronicle;
use hf_sim::culture::CustomKind;
use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::mind::Activity;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::time::{Calendar, TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::{Session, World};

fn meadow(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    for i in 0..200 {
        let p = TilePos::new(3 + (i * 7) % 58, 3 + (i * 13 + i / 58) % 58, 2);
        if !w.plants.iter().any(|q| q.pos == p) {
            w.plants
                .push(Plant::new(p, PlantKind::BerryBush, 1.0, i as u32));
        }
    }
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    w
}

#[test]
fn custom_origin_traceable() {
    let mut s = Session::new(meadow(91), Chronicle::in_memory().unwrap());
    let ids = s.world.found_party(8);
    s.run(TICKS_PER_HOUR).unwrap();
    // A founder dies; by the next dawn the community keeps a rest day in their memory.
    hf_sim::systems::life::die(&mut s.world, ids[0], "old age".into());
    let died_event = s
        .world
        .events
        .pending
        .iter()
        .find(|e| matches!(e.kind, EventKind::Died { .. }))
        .map(|e| e.id)
        .expect("death recorded");
    s.run(TICKS_PER_DAY * 2).unwrap();
    let custom = s
        .world
        .customs
        .iter()
        .find(|c| matches!(c.kind, CustomKind::RestDay { .. }))
        .expect("a rest-day custom formed");
    assert_eq!(
        custom.origin, died_event,
        "the custom's origin is the death event"
    );
    assert!(custom
        .narrative
        .contains(&s.world.agents.iter().find(|a| a.id == ids[0]).unwrap().name));
    // The chronicle's taught tier points at the real row of that event.
    let hid = s
        .chronicle
        .history_id(died_event)
        .unwrap()
        .expect("death persisted in real history");
    let taught = s.chronicle.taught(50).unwrap();
    let row = taught
        .iter()
        .find(|t| t.0 == hid)
        .expect("taught history row for the custom");
    assert_eq!(
        row.4, "died",
        "the real event behind the telling is the death"
    );
    assert_eq!(row.3, custom.narrative);
    // Real history itself is untouched by the telling.
    assert!(s
        .chronicle
        .conn
        .execute("UPDATE real_history SET kind='x'", [])
        .is_err());
}

#[test]
fn custom_changes_behavior() {
    // A rest day: on that day members do no work; the day before they do.
    let mut w = meadow(92);
    let ids = w.found_party(8);
    // The meadow has no trees; without logs on hand there is no work on any day.
    w.drop_item(hf_sim::items::ItemKind::Log, 60, TilePos::new(34, 30, 2));
    w.run(TICKS_PER_HOUR * 2);
    hf_sim::systems::life::die(&mut w, ids[7], "old age".into());
    w.run(TICKS_PER_DAY * 2);
    let day = match w
        .customs
        .iter()
        .find(|c| matches!(c.kind, CustomKind::RestDay { .. }))
        .map(|c| c.kind.clone())
    {
        Some(CustomKind::RestDay { day_of_season, .. }) => day_of_season,
        _ => panic!("rest-day custom formed"),
    };
    // Run to the season after next so the anniversary falls inside the window; count work
    // activity on the rest day versus the day before it.
    let mut work_rest = 0u32;
    let mut work_before = 0u32;
    let mut rest_event = false;
    for _ in 0..TICKS_PER_DAY * 12 {
        w.step();
        let cal = Calendar::from_tick(w.tick());
        if w.last_events
            .iter()
            .any(|e| matches!(e.kind, EventKind::RestDay { .. }))
        {
            rest_event = true;
        }
        if cal.hour < 8 || cal.hour > 18 || w.tick() < TICKS_PER_DAY * 4 {
            continue;
        }
        // Only people who hold the custom are bound by it; newcomers who have not yet taken
        // it up may keep working.
        let holders: Vec<hf_sim::ids::AgentId> = w
            .customs
            .iter()
            .filter(|c| matches!(c.kind, CustomKind::RestDay { .. }))
            .flat_map(|c| c.adopted_by.clone())
            .collect();
        let working = w
            .agents
            .iter()
            .filter(|a| {
                a.alive && holders.contains(&a.id) && matches!(a.activity, Activity::Working { .. })
            })
            .count() as u32;
        if cal.day_of_season == day {
            work_rest += working;
        } else if cal.day_of_season + 1 == day || (day == 1 && cal.day_of_season == 5) {
            work_before += working;
        }
    }
    assert!(rest_event, "the rest day was observed and recorded");
    assert!(
        work_before > 20,
        "people work on ordinary days: {work_before}"
    );
    assert!(
        work_rest * 4 < work_before,
        "work drops on the rest day: rest {work_rest} vs before {work_before}"
    );
}

/// OST-907: a custom formed for a community that then leaves the valley should not be formed and
/// lapsed over and over from the same Split, every dawn, until the event ages out of the log.
#[test]
fn a_custom_is_not_reborn_from_the_same_cause() {
    let mut w = meadow(1324);
    let ids = w.found_party(4);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    let leader = ids[0];

    w.meta.tick = TICKS_PER_HOUR * 6;
    let tick = w.meta.tick;
    let split = EventKind::Split {
        from: cid,
        to: hf_sim::ids::CommunityId(999_999),
        leader,
        members: 2,
        cause: "a test leaving".into(),
    };
    let eid = w
        .events
        .push(tick, None, hf_sim::events::Severity::Major, split.clone());
    w.culture_log.push((eid, tick, split));

    let mut formed = 0;
    let mut lapsed = 0;
    for day in 0..30u64 {
        w.meta.tick = TICKS_PER_DAY * day + TICKS_PER_HOUR * 6;
        hf_sim::systems::culture::step(&mut w);
        for e in w.events.drain() {
            match &e.kind {
                EventKind::CustomFormed {
                    community, label, ..
                } if *community == cid && label.contains("take in no strangers") => formed += 1,
                EventKind::CustomLapsed {
                    community, label, ..
                } if *community == cid && label.contains("take in no strangers") => lapsed += 1,
                _ => {}
            }
        }
        if day == 2 {
            // The community leaves the valley: nobody of it is left.
            for a in w.agents.iter_mut().filter(|a| a.community == Some(cid)) {
                a.alive = false;
            }
            w.communities
                .iter_mut()
                .find(|c| c.id == cid)
                .unwrap()
                .dissolved = true;
        }
    }
    assert_eq!(formed, 1, "the custom should form once, not every dawn");
    assert_eq!(
        lapsed, 1,
        "the custom should lapse once when its holders are gone, not over and over"
    );
}

/// OST-907: once a rest-day custom lapses (its holders gone), a later death can still found a new
/// one — the fix against re-forming from the same cause must not block a genuinely new cause.
#[test]
fn a_new_death_can_found_a_new_rest_day() {
    let mut w = meadow(13242);
    let ids = w.found_party(4);

    w.meta.tick = TICKS_PER_HOUR * 6;
    hf_sim::systems::life::die(&mut w, ids[0], "old age".into());
    let died: Vec<_> = w
        .events
        .drain()
        .into_iter()
        .filter(|e| matches!(e.kind, EventKind::Died { .. }))
        .map(|e| (e.id, e.tick, e.kind))
        .collect();
    w.culture_log.extend(died);
    hf_sim::systems::culture::step(&mut w);
    let first = w
        .customs
        .iter()
        .find(|c| matches!(c.kind, CustomKind::RestDay { .. }))
        .expect("a rest-day custom formed")
        .id;
    let first_origin = w.customs.iter().find(|c| c.id == first).unwrap().origin;

    // Force it to lapse quickly, as if its holders were gone.
    w.customs
        .iter_mut()
        .find(|c| c.id == first)
        .unwrap()
        .intensity = 0.05;
    w.meta.tick = TICKS_PER_DAY + TICKS_PER_HOUR * 6;
    hf_sim::systems::culture::step(&mut w);
    assert!(
        w.customs.iter().find(|c| c.id == first).unwrap().lapsed,
        "the first rest day lapsed"
    );

    // A different person's death, a new cause, founds a new one.
    w.meta.tick = TICKS_PER_DAY * 2 + TICKS_PER_HOUR * 6;
    hf_sim::systems::life::die(&mut w, ids[1], "an accident".into());
    let died: Vec<_> = w
        .events
        .drain()
        .into_iter()
        .filter(|e| matches!(e.kind, EventKind::Died { .. }))
        .map(|e| (e.id, e.tick, e.kind))
        .collect();
    w.culture_log.extend(died);
    w.meta.tick = TICKS_PER_DAY * 3 + TICKS_PER_HOUR * 6;
    hf_sim::systems::culture::step(&mut w);
    let second = w
        .customs
        .iter()
        .find(|c| matches!(c.kind, CustomKind::RestDay { .. }) && c.id != first)
        .expect("a new rest-day custom formed from the new death");
    assert_ne!(
        second.origin, first_origin,
        "the new custom has a new cause"
    );
}

/// OST-507: a belief someone came to hold from what happened to them is told at the hearth to
/// whoever has not heard it from them this year, not every evening to the same circle.
#[test]
fn a_lived_belief_is_told_once_a_year_to_each_listener() {
    use hf_sim::events::EventKind;
    use hf_sim::mind::beliefs::Source;
    use hf_sim::mind::Activity;
    use hf_sim::past::Topic;
    use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR, TICKS_PER_YEAR};

    let mut w = hf_sim::World::opening_world(61);
    let cid = w.communities[0].id;
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    if hf_sim::systems::culture::hearth_of(&w, cid).is_none() {
        w.place_structure(
            hf_sim::structures::StructureKind::Hearth,
            home,
            hf_sim::world::Material::STONE_BLOCK,
            true,
        );
    }
    let h = hf_sim::systems::culture::hearth_of(&w, cid).unwrap();
    for a in w.agents.iter_mut() {
        a.pos = hf_sim::ids::TilePos::new(h.x + 30, h.y + 30, h.z);
    }
    // Five sitters: the first lived through something; none holds a story or a tale.
    let seat = |w: &mut hf_sim::World, k: usize| {
        let a = &mut w.agents[k];
        a.community = Some(cid);
        a.band = None;
        a.pos = h.offset(1, 0, 0);
        a.activity = Activity::Idle;
        a.stories.clear();
        a.tales.clear();
        a.beliefs = Default::default();
        // Grown, so anyone here may tell (OST-511).
        a.born = -30 * TICKS_PER_YEAR as i64;
    };
    for k in 0..4 {
        seat(&mut w, k);
    }
    let teller = w.agents[0].id;
    w.agents[0].beliefs.hold(
        Topic::Fire,
        0.8,
        0.8,
        Source::Event(hf_sim::ids::EventId(1)),
        0,
    );
    let told = |w: &mut hf_sim::World, tick: u64| {
        w.meta.tick = tick;
        w.events.pending.clear();
        for k in 0..w.agents.len() {
            if w.agents[k].community == Some(cid) && w.agents[k].pos.dist(h) <= 2 {
                w.agents[k].activity = Activity::Idle;
            }
        }
        hf_sim::systems::culture::step(w);
        w.events
            .pending
            .iter()
            .filter(|e| {
                matches!(&e.kind, EventKind::HearthTale { teller: t, what, .. }
                    if *t == teller && what.starts_with("how they came to hold"))
            })
            .count()
    };
    let evening = |day: u64| day * TICKS_PER_DAY + 19 * TICKS_PER_HOUR + TICKS_PER_HOUR / 2;
    let mut tellings = 0;
    for day in 1..=10 {
        if day == 6 {
            // A newcomer sits down.
            seat(&mut w, 4);
            assert_eq!(
                told(&mut w, evening(day)),
                1,
                "the newcomer hears it once more"
            );
            tellings += 1;
            continue;
        }
        tellings += told(&mut w, evening(day));
    }
    assert_eq!(tellings, 2, "once, and once more for the newcomer");
    // After a year it can be told again.
    let later = evening(10) + TICKS_PER_YEAR;
    assert_eq!(told(&mut w, later), 1, "a year on it can be told again");
}

/// A person saved before the hearth record existed still loads, with it empty (OST-507).
#[test]
fn an_old_save_loads_without_the_hearth_record() {
    let w = hf_sim::World::opening_world(61);
    let mut v = serde_json::to_value(&w.agents[0]).expect("an agent serialises");
    assert!(v.as_object_mut().unwrap().remove("hearth_heard").is_some());
    let a: hf_sim::agents::Agent = serde_json::from_value(v).expect("an old agent still loads");
    assert!(a.hearth_heard.is_empty());
}
