//! Creatures with depth (issue #51): roles you can see by watching, a unique beast, and a long
//! interaction between a beast and the settlement.

use hf_sim::agents::Species;
use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::World;

fn run_to_day(w: &mut World, day: u64, log: &mut Vec<EventKind>) {
    while w.since_arrival() < day * TICKS_PER_DAY {
        w.step();
        log.extend(w.last_events.iter().map(|e| e.kind.clone()));
    }
}

#[test]
fn the_bear_dens_for_winter_and_the_ducks_go_south() {
    let mut w = World::opening_world(7);
    let bears = w
        .agents
        .iter()
        .filter(|a| a.alive && a.species == Species::Bear)
        .count();
    let ducks = w
        .agents
        .iter()
        .filter(|a| a.alive && a.species == Species::Duck)
        .count();
    assert!(bears > 0 && ducks > 0);
    let mut log = Vec::new();
    run_to_day(&mut w, 17, &mut log);
    assert!(
        w.agents
            .iter()
            .filter(|a| a.alive && a.species == Species::Bear)
            .all(|a| a.denned),
        "in the den all winter"
    );
    assert_eq!(
        w.agents
            .iter()
            .filter(|a| a.alive && a.species == Species::Duck)
            .count(),
        0,
        "the ducks are away"
    );
    assert!(!w.creatures.migrants.is_empty());
    run_to_day(&mut w, 22, &mut log);
    assert!(
        w.agents
            .iter()
            .filter(|a| a.alive && a.species == Species::Bear)
            .all(|a| !a.denned),
        "awake in spring"
    );
    assert!(
        w.agents
            .iter()
            .any(|a| a.alive && a.species == Species::Duck),
        "the ducks came back"
    );
    for k in ["bear_denned", "bear_woke", "ducks_left", "ducks_returned"] {
        assert!(
            log.iter().any(|e| format!("{e:?}")
                .to_lowercase()
                .replace('_', "")
                .contains(&k.replace('_', ""))),
            "{k}"
        );
    }
}

#[test]
fn crows_come_down_where_something_died() {
    let mut w = World::opening_world(11);
    let crows: Vec<_> = w
        .agents
        .iter()
        .filter(|a| a.alive && a.species == Species::Crow)
        .map(|a| a.id)
        .collect();
    assert!(crows.len() >= 2);
    let spot = w.agents.iter().find(|a| a.id == crows[0]).unwrap().pos;
    let tick = w.meta.tick;
    let deer = w.spawn_agent(Species::Deer, spot);
    w.events.push(
        tick,
        Some(spot),
        hf_sim::events::Severity::Minor,
        EventKind::Died {
            agent: deer,
            cause: "in a test".into(),
        },
    );
    let mut log = Vec::new();
    for _ in 0..TICKS_PER_DAY {
        w.step();
        log.extend(w.last_events.iter().map(|e| e.kind.clone()));
        if log
            .iter()
            .any(|e| matches!(e, EventKind::CrowsGathered { .. }))
        {
            break;
        }
    }
    assert!(log
        .iter()
        .any(|e| matches!(e, EventKind::CrowsGathered { .. })));
}

#[test]
fn the_pack_keeps_together() {
    let mut w = World::opening_world(11);
    let mut samples = Vec::new();
    for _ in 0..3 * TICKS_PER_DAY {
        w.step();
        if w.meta.tick.is_multiple_of(TICKS_PER_HOUR) {
            let Some(l) = hf_sim::creatures::pack_leader(&w) else {
                continue;
            };
            let lp = w.agents[l].pos;
            for a in w
                .agents
                .iter()
                .filter(|a| a.alive && a.species == Species::Wolf)
            {
                samples.push(a.pos.dist(lp));
            }
        }
    }
    assert!(!samples.is_empty());
    let mean = samples.iter().sum::<i32>() as f32 / samples.len() as f32;
    assert!(mean < 8.0, "wolves stay near their leader: mean {mean:.1}");
}

#[test]
fn the_white_hart_comes_to_most_valleys_and_keeps_away() {
    let mut with = 0;
    for seed in 0..10u64 {
        let w = World::opening_world(seed);
        let n = w
            .agents
            .iter()
            .filter(|a| a.species == Species::Hart)
            .count();
        assert!(n <= 1, "never more than one");
        with += n;
    }
    assert!(with >= 4, "most valleys: {with}/10");
    // Wary: someone ten tiles off sends it away.
    let mut w = World::opening_world(7);
    let h = w
        .agents
        .iter()
        .position(|a| a.species == Species::Hart)
        .expect("a hart");
    let hp = w.agents[h].pos;
    let near = w.grid.surface(hp.x + 10, hp.y);
    let watcher = w.spawn_agent(Species::Person, near);
    let start = w.agents[h].pos.dist(near);
    for _ in 0..200 {
        if let Some(a) = w.agents.iter_mut().find(|a| a.id == watcher) {
            a.pos = near;
            a.from = near;
            a.path.clear();
        }
        w.step();
    }
    let end = w
        .agents
        .iter()
        .find(|a| a.species == Species::Hart)
        .unwrap()
        .pos
        .dist(near);
    assert!(end > start, "it moved off: {start} -> {end}");
}

#[test]
fn a_beast_that_keeps_hurting_people_is_hunted_down_and_its_pelt_kept() {
    let mut w = World::opening_world(11);
    w.meta.autoplan = true;
    for a in w.agents.iter_mut().filter(|a| a.community.is_some()) {
        a.beliefs.hold(
            hf_sim::mind::beliefs::Topic::Violence,
            0.7,
            0.9,
            hf_sim::mind::beliefs::Source::Event(hf_sim::ids::EventId(0)),
            0,
        );
    }
    let home = w.communities[0].home;
    let wp = w.grid.surface(home.x + 8, home.y + 4);
    let wolf = w.spawn_agent(Species::Wolf, wp);
    if let Some(a) = w.agents.iter_mut().find(|a| a.id == wolf) {
        a.name = "Greyjaw".into();
    }
    hf_sim::creatures::note_wound(&mut w, wolf);
    hf_sim::creatures::note_wound(&mut w, wolf);
    let mut log = Vec::new();
    for _ in 0..4 * TICKS_PER_DAY {
        w.step();
        log.extend(
            w.last_events
                .iter()
                .map(|e| (e.kind.clone(), hf_sim::view::narrate::line(&w, e))),
        );
        if log
            .iter()
            .any(|(e, _)| matches!(e, EventKind::BeastSlain { .. }))
        {
            break;
        }
    }
    let went = log
        .iter()
        .find(|(_, l)| l.contains("went after Greyjaw"))
        .expect("someone went after it");
    assert!(went.1.contains("hurt 2 people"), "{}", went.1);
    let slain = log
        .iter()
        .find(|(e, _)| matches!(e, EventKind::BeastSlain { .. }))
        .expect("brought down");
    assert!(
        slain.1.contains("Greyjaw") && slain.1.contains("2 people"),
        "{}",
        slain.1
    );
    let pelt = w
        .items
        .iter()
        .find(|it| {
            it.story
                .as_ref()
                .and_then(|s| s.name.clone())
                .is_some_and(|n| n == "Greyjaw's pelt")
        })
        .expect("the pelt is kept");
    let d = hf_sim::legend::describe(&w, pelt.id).unwrap();
    assert!(d.contains("taken from Greyjaw"), "{d}");
    let _ = TilePos::new(0, 0, 0);
}

/// Issue #94: a beast is not a person. No temperament, no skills, a beast's age; and the
/// three who begin are the world's first people (ids 0..3), the White Hart placed after them.
#[test]
fn beasts_are_not_people_and_the_founders_come_first() {
    for seed in [7u64, 3, 11] {
        let w = World::opening_world(seed);
        let founders: Vec<u32> = w
            .agents
            .iter()
            .filter(|a| a.community.is_some())
            .map(|a| a.id.0)
            .collect();
        assert_eq!(founders, vec![0, 1, 2], "seed {seed}: the founders' ids");
        let mut seen: Vec<Species> = Vec::new();
        for a in w.agents.iter().filter(|a| a.species != Species::Person) {
            if !seen.contains(&a.species) {
                seen.push(a.species);
            }
            assert!(
                a.traits.list.is_empty(),
                "seed {seed}: {} has a temperament {:?}",
                a.name,
                a.traits.list
            );
            assert!(
                hf_sim::mind::skills::ALL
                    .iter()
                    .all(|s| a.skills.level(*s) == 0.0),
                "seed {seed}: {} has skills",
                a.name
            );
            let age = a.age_years(w.meta.tick);
            assert!(age < 16, "seed {seed}: {} is {age}", a.name);
            assert!(a.id.0 > 2);
        }
        for s in [Species::Deer, Species::Bat, Species::Crawler] {
            assert!(seen.contains(&s), "seed {seed}: {s:?} still placed");
        }
        // People still have both.
        for a in w.agents.iter().filter(|a| a.species == Species::Person) {
            let age = a.age_years(w.meta.tick);
            assert!(age >= 14, "seed {seed}: {} is {age}", a.name);
        }
    }
    let w = World::opening_world(7);
    let hart = w
        .agents
        .iter()
        .find(|a| a.species == Species::Hart)
        .expect("seed 7 has the White Hart");
    assert_eq!(hart.name, "the White Hart");
}
