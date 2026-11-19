//! The new-game choices (setup.rs, Phase 17d): chosen before the world exists, each one shapes
//! what is simulated, and the same choices always make the same world.

use hf_sim::agents::Species;
use hf_sim::deity::{Lever, Trigger};
use hf_sim::engine::Engine;
use hf_sim::events::EventKind;
use hf_sim::mind::beliefs::Source;
use hf_sim::past::Topic;
use hf_sim::setup::WorldSetup;
use hf_sim::time::TICKS_PER_YEAR;
use hf_sim::world::gen::biome::{Biome, Character};
use hf_sim::World;

fn with(seed: u64, f: impl FnOnce(&mut WorldSetup)) -> World {
    let mut s = WorldSetup::new(seed);
    f(&mut s);
    World::from_setup(&s)
}

#[test]
fn same_choices_same_world() {
    let pick = |s: &mut WorldSetup| {
        s.engine = Engine::Whimsical;
        s.history_years = 300;
        // The valley this was written against (since M3 a valley takes its region's climate,
        // and a dry steppe valley may have no hamlets for a deity to answer).
        s.character = Some(hf_sim::world::gen::biome::Character::of_seed(31));
        s.chosen = [2, 5, 7];
        s.ideology = vec![(Topic::Hunger, -0.7)];
    };
    let a = with(31, pick);
    let b = with(31, pick);
    assert_eq!(hf_sim::digest::digest(&a), hf_sim::digest::digest(&b));
    let c = with(31, |s| {
        pick(s);
        s.engine = Engine::Relentless;
    });
    assert_ne!(
        a.history.answers, c.history.answers,
        "another deity watched a different past"
    );
}

#[test]
fn the_deity_watches_the_past() {
    // Each deity answers the old hamlets by its own motive, through the hamlets' own luck, and
    // every answer is on the record before the portal.
    for e in Engine::ALL {
        let (mut answers, mut followed) = (0, 0);
        for seed in 0..8 {
            let w = with(seed, |s| s.engine = e);
            for a in &w.history.answers {
                answers += 1;
                followed += a.followed as u32;
                match e {
                    Engine::Steady => assert!(
                        matches!(
                            (a.trigger, a.lever),
                            (Trigger::Plenty, Lever::Harvest) | (Trigger::Hardship, Lever::Harvest)
                        ) && ((a.trigger == Trigger::Plenty) == (a.strength < 1.0)),
                        "Aun lowers the high and lifts the low: {a:?}"
                    ),
                    Engine::Relentless => assert!(
                        a.trigger == Trigger::Plenty && a.strength > 1.0,
                        "Hask presses the strong: {a:?}"
                    ),
                    Engine::Whimsical => assert_eq!(a.trigger, Trigger::LongQuiet),
                }
            }
            let recorded = w
                .events
                .pending
                .iter()
                .filter(|ev| matches!(ev.kind, EventKind::OldAnswered { .. }))
                .filter(|ev| ev.tick < w.meta.epoch)
                .count();
            assert_eq!(recorded, w.history.answers.len());
        }
        assert!(answers >= 20, "{e:?} answered {answers} times in 8 pasts");
        if e == Engine::Relentless {
            // Untouched, a hamlet burns or sickens in about one year in twenty; leaned on, the
            // trouble Hask leaned toward came far more often.
            let rate = followed as f32 / answers as f32;
            assert!(
                rate > 0.15,
                "Hask's leans came to pass {rate:.2} of the time"
            );
        }
    }
}

#[test]
fn history_depth_is_chosen() {
    let none = with(4, |s| s.history_years = 0);
    assert!(none.history.sites.is_empty(), "no past, no ruins");
    for years in [50, 300] {
        let w = with(4, |s| s.history_years = years);
        assert_eq!(w.history.years, years);
        let oldest = w
            .events
            .pending
            .iter()
            .filter(|e| matches!(e.kind, EventKind::OldFounded { .. }))
            .map(|e| e.tick)
            .min()
            .expect("a hamlet was founded");
        assert!(w.meta.epoch - oldest <= years * TICKS_PER_YEAR);
    }
}

#[test]
fn the_valley_is_the_one_chosen() {
    let share = |w: &World, b: Biome| {
        let mut n = 0;
        for y in 0..w.grid.height {
            for x in 0..w.grid.width {
                n += (w.grid.biome_at(x, y) == Some(b)) as u32;
            }
        }
        n as f32 / (w.grid.width * w.grid.height) as f32
    };
    let wet = with(12, |s| {
        s.character = Some(Character {
            wetness: 1.0,
            warmth: 0.0,
            stoniness: -1.0,
        })
    });
    let dry = with(12, |s| {
        s.character = Some(Character {
            wetness: -1.0,
            warmth: 0.0,
            stoniness: -1.0,
        })
    });
    assert!(share(&wet, Biome::Fen) > share(&dry, Biome::Fen) + 0.2);
    assert!(share(&dry, Biome::Heath) > share(&wet, Biome::Heath) + 0.2);
}

#[test]
fn what_they_hold_in_common_has_a_source() {
    let w = with(6, |s| s.ideology = vec![(Topic::Strangers, -0.8)]);
    let choice = w
        .events
        .pending
        .iter()
        .find(|e| matches!(e.kind, EventKind::SetupChosen { .. }))
        .expect("the choice is on the record");
    let founders: Vec<_> = w
        .agents
        .iter()
        .filter(|a| a.species == Species::Person && a.community.is_some())
        .collect();
    assert_eq!(founders.len(), 3);
    for a in founders {
        let b = a.beliefs.get(Topic::Strangers).expect("held in common");
        assert!(b.sources.contains(&Source::Ideology(choice.id)));
    }
    let none = with(6, |_| {});
    assert!(none.agents.iter().all(|a| a
        .beliefs
        .list
        .iter()
        .all(|b| !b.sources.iter().any(|s| matches!(s, Source::Ideology(_))))));
}

#[test]
fn locals_sense_the_watcher() {
    // People from the neighbouring places bring what their elders noticed: the deity is never
    // named, but what kept happening is told, and told true.
    let mut brought = 0;
    for seed in 0..10 {
        let mut w = with(seed, |s| s.engine = Engine::Relentless);
        let p = w.meta.founding_site;
        let ids: Vec<_> = (0..6).map(|_| w.spawn_agent(Species::Person, p)).collect();
        hf_sim::lore::newcomers(&mut w, &ids);
        for a in w.agents.iter().filter(|a| ids.contains(&a.id)) {
            for t in a.tales.iter().filter(|t| t.claim.is_pattern()) {
                brought += 1;
                assert!(t.true_to(&w), "{:?} at site {}", t.claim, t.site);
            }
        }
    }
    assert!(
        brought >= 5,
        "newcomers told of the pattern {brought} times"
    );
}
