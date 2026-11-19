//! P11 gates (docs/design-peoples.md section 8): a moon; the restless are only the
//! unremembered and are laid to rest by a rite; a curse spreads only by a recorded contact;
//! the Stone-sleep comes from the underworld's stone and ends in a statue; a suspicion has its
//! rows as sources; the unbinding lifts a curse.

use hf_sim::conditions::ConditionKind as C;
use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::time::{Calendar, TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::World;

fn pending(w: &World, f: impl Fn(&EventKind) -> bool) -> usize {
    w.events.pending.iter().filter(|e| f(&e.kind)).count()
}

/// Set the clock to `h`:07 on day `d` and run the conditions step once.
fn at(w: &mut World, d: u64, h: u64) {
    w.meta.tick = d * TICKS_PER_DAY + h * TICKS_PER_HOUR + 7;
    hf_sim::conditions::step(w);
}

fn home(w: &World) -> TilePos {
    w.communities[0].home
}

#[test]
fn two_full_moons_a_year_and_brighter_nights() {
    let year = hf_sim::time::TICKS_PER_YEAR;
    let nights = (0..year / TICKS_PER_HOUR)
        .map(|h| h * TICKS_PER_HOUR)
        .filter(|&t| Calendar::full_moon(t) && (t % TICKS_PER_DAY) / TICKS_PER_HOUR == 0)
        .count();
    assert_eq!(nights, 2, "two full-moon midnights in a twenty-day year");
    let full = (0..TICKS_PER_DAY * 10)
        .find(|&t| Calendar::full_moon(t))
        .unwrap();
    assert!(Calendar::moon(full) > Calendar::moon(full + 5 * TICKS_PER_DAY));
}

#[test]
fn the_restless_are_only_the_unremembered() {
    let mut w = World::opening_world(61);
    let h = home(&w);
    // A stranger nobody knew dies near the community, and one someone knew dies too.
    let lone = w.spawn_agent(hf_sim::agents::Species::Person, h.offset(6, 0, 0));
    let known = w.spawn_agent(hf_sim::agents::Species::Person, h.offset(-6, 0, 0));
    let friend = w.agents.iter().find(|a| a.community.is_some()).unwrap().id;
    let tick = w.meta.tick;
    for a in w.agents.iter_mut() {
        if a.id == friend {
            a.relate(known, 0.5, 0.6, tick);
        }
    }
    hf_sim::systems::life::die(&mut w, lone, "of cold".into());
    hf_sim::systems::life::die(&mut w, known, "of cold".into());
    // The friend's memory of the death settles into a long one, as a night's sleep does.
    for a in w.agents.iter_mut().filter(|a| a.id == friend) {
        a.memories.consolidate(tick, false, false);
    }
    let day = w.meta.tick / TICKS_PER_DAY + 11;
    at(&mut w, day, 6);
    let restless: Vec<_> = w
        .events
        .pending
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::Restless { agent } => Some(agent),
            _ => None,
        })
        .collect();
    assert!(restless.contains(&lone), "unremembered, unlaid: restless");
    assert!(
        !restless.contains(&known),
        "remembered by someone: not restless"
    );
    // When the one who remembered has forgotten, the dead is restless after all.
    for a in w.agents.iter_mut().filter(|a| a.id == friend) {
        a.memories
            .long
            .retain(|m| m.subject != hf_sim::mind::Subject::Person(known));
    }
    at(&mut w, day + 1, 6);
    assert_eq!(
        pending(
            &w,
            |k| matches!(k, EventKind::Restless { agent } if *agent == known)
        ),
        1
    );
}

#[test]
fn a_ghost_seen_is_laid_to_rest_by_its_rite() {
    let mut w = World::opening_world(62);
    let h = home(&w);
    let lone = w.spawn_agent(hf_sim::agents::Species::Person, h.offset(5, 0, 0));
    hf_sim::systems::life::die(&mut w, lone, "of a fever".into());
    let day = w.meta.tick / TICKS_PER_DAY + 11;
    at(&mut w, day, 6);
    // Someone awake near at dusk who holds that the dead must be given their rite.
    let i = w
        .agents
        .iter()
        .position(|a| a.alive && a.community.is_some())
        .unwrap();
    let t = w.meta.tick;
    w.agents[i].pos = h.offset(3, 0, 0);
    w.agents[i].activity = hf_sim::mind::Activity::Idle;
    w.agents[i].beliefs.hold(
        hf_sim::mind::beliefs::Topic::Burial,
        0.8,
        0.8,
        hf_sim::mind::beliefs::Source::Event(hf_sim::ids::EventId(1)),
        t,
    );
    at(&mut w, day, 19);
    let seen = w.events.pending.iter().find_map(|e| match &e.kind {
        EventKind::GhostSeen { of, by, cause } if *of == lone => Some((*by, cause.clone())),
        _ => None,
    });
    let (by, cause) = seen.expect("seen at dusk where they died");
    assert_eq!(by, w.agents[i].id);
    assert_eq!(
        cause, "of a fever",
        "the sighting tells how they died, truly"
    );
    assert_eq!(
        pending(
            &w,
            |k| matches!(k, EventKind::LaidToRest { agent, .. } if *agent == lone)
        ),
        1
    );
    assert_eq!(
        pending(
            &w,
            |k| matches!(k, EventKind::AtRest { agent, .. } if *agent == lone)
        ),
        1
    );
    let dead = w.agents.iter().find(|a| a.id == lone).unwrap();
    assert!(dead.conditions.is_empty());
}

#[test]
fn the_beast_hunger_spreads_only_by_a_bite() {
    let mut w = World::opening_world(63);
    let h = home(&w);
    let ids: Vec<_> = w
        .agents
        .iter()
        .filter(|a| a.alive && a.community.is_some())
        .map(|a| a.id)
        .collect();
    let (wolf, sleeper) = (ids[0], ids[1]);
    let iw = w.agents.iter().position(|a| a.id == wolf).unwrap();
    hf_sim::conditions::take(&mut w, iw, C::BeastHunger, "for a test", None);
    // Everyone else far away and awake, but one asleep out of doors near.
    for a in w.agents.iter_mut().filter(|a| a.alive) {
        if a.id == sleeper {
            a.pos = h.offset(8, 8, 0);
            a.activity = hf_sim::mind::Activity::Sleeping {
                bed: None,
                until: u64::MAX,
            };
        } else if a.id == wolf {
            a.pos = h.offset(6, 6, 0);
        } else {
            a.activity = hf_sim::mind::Activity::Idle;
        }
    }
    let mut bitten = 0;
    for d in 0..40u64 {
        at(&mut w, d, 1);
        bitten += pending(&w, |k| matches!(k, EventKind::Bitten { .. }));
        for a in w.agents.iter_mut().filter(|a| a.id == sleeper) {
            a.activity = hf_sim::mind::Activity::Sleeping {
                bed: None,
                until: u64::MAX,
            };
            a.body.blood = 1.0;
            a.body.wounds.clear();
        }
    }
    assert!(bitten > 0, "bitten at full moons");
    let bites: Vec<hf_sim::ids::EventId> = w
        .events
        .pending
        .iter()
        .filter(|e| matches!(e.kind, EventKind::Bitten { .. }))
        .map(|e| e.id)
        .collect();
    for a in &w.agents {
        for c in a.conditions.iter().filter(|c| c.kind == C::BeastHunger) {
            if a.id == wolf {
                continue;
            }
            assert_eq!(c.from, Some(wolf), "passed by the one who bit");
            assert!(bites.contains(&c.cause), "from a bite in the record");
        }
    }
    // Every bite fell on a full-moon night, and the one who went out was missed.
    for e in w
        .events
        .pending
        .iter()
        .filter(|e| matches!(e.kind, EventKind::Bitten { .. }))
    {
        assert!(Calendar::full_moon(e.tick));
    }
    assert!(
        pending(
            &w,
            |k| matches!(k, EventKind::Missing { agent } if *agent == wolf)
        ) >= 2
    );
    // And someone came to suspect them, from the record.
    let sus = w.events.pending.iter().find_map(|e| match &e.kind {
        EventKind::Suspected { by, of, .. } if *of == wolf => Some(*by),
        _ => None,
    });
    let by = sus.expect("a suspicion");
    let a = w.agents.iter().find(|a| a.id == by).unwrap();
    assert!(a
        .memories
        .short
        .iter()
        .any(|m| m.subject == hf_sim::mind::Subject::Person(wolf) && m.event.is_some()));
}

#[test]
fn the_stone_sleep_comes_from_the_underworld_and_ends_in_stone() {
    let mut found = false;
    for seed in 61..70 {
        let mut w = World::from_setup(&hf_sim::setup::WorldSetup::new(seed));
        let Some(k) = w.grid.caverns.iter().position(|c| c.underworld) else {
            continue;
        };
        found = true;
        let c = w.grid.caverns[k].clone();
        let i = w
            .agents
            .iter()
            .position(|a| a.alive && a.community.is_some())
            .unwrap();
        let who = w.agents[i].id;
        w.agents[i].pos = c.centre;
        hf_sim::underground::breaches(&mut w);
        let cond = w.agents[i]
            .conditions
            .iter()
            .find(|x| x.kind == C::StoneSleep)
            .cloned()
            .expect("touched the underworld's stone");
        assert!(w
            .events
            .pending
            .iter()
            .any(|e| e.id == cond.cause && matches!(e.kind, EventKind::Breached { .. })));
        // Forty days on, a statue.
        let day = w.meta.tick / TICKS_PER_DAY + 41;
        at(&mut w, day, 6);
        assert_eq!(
            pending(
                &w,
                |k| matches!(k, EventKind::TurnedToStone { agent, .. } if *agent == who)
            ),
            1
        );
        assert!(!w.agents[i].alive);
        let statue = w.items.iter().find(|it| {
            it.story
                .as_ref()
                .is_some_and(|s| s.origin.contains(&c.name) && s.origin.contains(&w.agents[i].name))
        });
        assert!(statue.is_some(), "the statue, its story the breach");
        break;
    }
    assert!(found, "some world shows the underworld's stone");
}

#[test]
fn the_unbinding_lifts_a_curse_from_one_suspected() {
    let mut w = World::opening_world(64);
    let ids: Vec<_> = w
        .agents
        .iter()
        .filter(|a| a.alive && a.community.is_some())
        .map(|a| a.id)
        .collect();
    let (cursed, healer) = (ids[0], ids[1]);
    let ic = w.agents.iter().position(|a| a.id == cursed).unwrap();
    hf_sim::conditions::take(&mut w, ic, C::Thirst, "for a test", None);
    let t = w.meta.tick;
    for a in w.agents.iter_mut().filter(|a| a.id == healer) {
        a.techniques.learn(
            hf_sim::mind::Technique::Unbinding,
            hf_sim::mind::techniques::Learned::Past,
            0.5,
            t,
        );
    }
    // Not yet suspected: nothing.
    let day = w.meta.tick / TICKS_PER_DAY + 1;
    at(&mut w, day, 6);
    assert_eq!(pending(&w, |k| matches!(k, EventKind::Unbound { .. })), 0);
    // Suspected (a memory of them from a row): lifted.
    for a in w.agents.iter_mut().filter(|a| a.id == healer) {
        a.remember(
            t,
            hf_sim::mind::MemKind::Hardship,
            hf_sim::mind::Subject::Person(cursed),
            -0.4,
            0.5,
            Some(hf_sim::ids::EventId(7)),
        );
    }
    at(&mut w, day + 1, 6);
    assert_eq!(
        pending(
            &w,
            |k| matches!(k, EventKind::Unbound { agent, by, .. } if *agent == cursed && *by == healer)
        ),
        1
    );
    assert!(w.agents[ic].conditions.iter().all(|c| c.kind != C::Thirst));
}

#[test]
fn eating_ones_own_dead_brings_the_thirst_only_under_hask() {
    for (engine, cursed) in [
        (hf_sim::engine::Engine::Relentless, true),
        (hf_sim::engine::Engine::Steady, false),
    ] {
        let mut w = World::opening_world(65);
        w.meta.engine = engine;
        let h = home(&w);
        // A mother and child, strangers to the valley, of a people who hold eating the dead
        // abhorrent; the child holds it less firmly than their people.
        let culture = w
            .realm
            .cultures
            .iter()
            .position(|c| c.stance(hf_sim::mind::beliefs::Topic::EatDead) <= -0.4)
            .expect("a people who hold it abhorrent") as u32;
        let mother = w.spawn_agent(hf_sim::agents::Species::Person, h.offset(15, 0, 0));
        let child = w.spawn_agent(hf_sim::agents::Species::Person, h.offset(16, 0, 0));
        hf_sim::species::raise_in(&mut w, child, culture);
        let ic = w.agents.iter().position(|a| a.id == child).unwrap();
        w.agents[ic].mother = Some(mother);
        let t = w.meta.tick;
        w.agents[ic].beliefs.hold(
            hf_sim::mind::beliefs::Topic::EatDead,
            0.1,
            0.9,
            hf_sim::mind::beliefs::Source::Event(hf_sim::ids::EventId(1)),
            t,
        );
        hf_sim::systems::life::die(&mut w, mother, "of hunger".into());
        w.agents[ic].needs.hunger = 0.0;
        w.agents[ic].starving_ticks = 10 * TICKS_PER_DAY as u32;
        let day = w.meta.tick / TICKS_PER_DAY + 1;
        at(&mut w, day, 13);
        assert_eq!(
            pending(
                &w,
                |k| matches!(k, EventKind::AteDead { agent, of } if *agent == child && *of == mother)
            ),
            1
        );
        assert_eq!(
            w.agents[ic].conditions.iter().any(|c| c.kind == C::Thirst),
            cursed,
            "{engine:?}"
        );
    }
}
