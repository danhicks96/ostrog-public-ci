//! #395: the bootstrap arrival affect. Coming through the portal is polarized by
//! temperament — the least calm freak out and break at once, the most calm snap to
//! survival work, the large middle oscillates in between — the fresh urgency weights
//! survival work up and leisure down on the existing #415 secured machinery, the high
//! tail's break is witnessed (panic is contagious), a steadier presence eases those
//! nearby, and the seven who are missing are dreaded and asked after. Like the shock
//! itself (#392) all of it decays within the first days.

use hf_sim::agents::Species;
use hf_sim::events::EventKind;
use hf_sim::genetics::locus::CALM;
use hf_sim::ids::TilePos;
use hf_sim::mind::{Activity, ThoughtKind};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn opening(seed: u64) -> World {
    World::from_setup(&hf_sim::setup::WorldSetup::new(seed))
}

/// The ten who came through: the Arrival thought is seeded on exactly them.
fn survivors(w: &World) -> Vec<usize> {
    (0..w.agents.len())
        .filter(|&i| {
            w.agents[i].alive
                && w.agents[i].species == Species::Person
                && w.agents[i].thoughts.has(ThoughtKind::Arrival)
        })
        .collect()
}

#[test]
fn the_shock_spans_freakout_snap_and_the_middle() {
    // Guard-before-fix (unmodified master): every survival_urge is exactly 1.5 at the
    // arrival (only the Arrival thought's fade feeds it), so no pole ever reads above
    // 1.5. The seeds carry both poles: a survivor with calm <= 0.15 and one with
    // calm >= 0.85, and someone of the middle between them.
    for seed in [3u64, 7, 11] {
        let w = opening(seed);
        let now = w.meta.tick;
        let pp = survivors(&w);
        assert_eq!(pp.len(), 10, "seed {seed}: the ten who came through");
        let urge = |a: &hf_sim::agents::Agent| hf_sim::systems::strain::survival_urge(a, now);
        let calmest = pp
            .iter()
            .map(|&i| &w.agents[i])
            .max_by(|a, b| a.genome.calm().total_cmp(&b.genome.calm()))
            .unwrap();
        assert!(
            calmest.genome.calm() >= 0.85,
            "seed {seed}: probed seed no longer has a survival-snap pole"
        );
        assert!(
            urge(calmest) > 1.5 + 1e-6,
            "seed {seed}: the steadiest does not snap to survival: {}",
            urge(calmest)
        );
        // The middle of the party is not the snap pole: at the same freshness its urge
        // is the unboosted 1.5, well below the snap pole's.
        let middle = pp
            .iter()
            .map(|&i| &w.agents[i])
            .find(|a| (0.3..0.7).contains(&a.genome.calm()));
        if let Some(m) = middle {
            assert!(
                urge(m) < urge(calmest) - 0.1,
                "seed {seed}: no spread between the middle and the snap pole"
            );
        }
    }
}

#[test]
fn the_panic_tail_breaks_at_once_and_the_others_see_it() {
    // Guard-before-fix: the arrival spike caps at 0.20 against a break threshold of at
    // least 0.55, so nothing can break in the first hours — a first-day break on master
    // takes a day of hardship to build, and the tail never breaks by the shock alone.
    for seed in [3u64, 7, 11] {
        let mut w = opening(seed);
        let pp = survivors(&w);
        assert!(
            pp.iter().any(|&i| w.agents[i].genome.calm() <= 0.15),
            "seed {seed}: probed seed no longer has a panic-tail survivor"
        );
        let mut breakers: Vec<hf_sim::ids::AgentId> = Vec::new();
        for _ in 0..2 * TICKS_PER_HOUR {
            w.step();
            for e in &w.last_events {
                if let EventKind::Broke { agent, .. } = &e.kind {
                    breakers.push(*agent);
                }
            }
        }
        let tail_broke = breakers.iter().any(|id| {
            w.agents
                .iter()
                .find(|a| a.id == *id)
                .is_some_and(|a| a.genome.calm() <= 0.15)
        });
        assert!(
            tail_broke,
            "seed {seed}: the panic tail did not break in the first hours: {breakers:?}"
        );
        assert!(
            breakers.len() <= 4,
            "seed {seed}: the whole party broke: {}",
            breakers.len()
        );
    }
    // Panic is contagious: in a flat world, one of two shocked survivors side by side is
    // the tail; the other sees the break and carries it. Guard-before-fix: without the
    // over-threshold spike there is no break at all, so nothing to witness.
    let mut w = World::generate(51, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = 12 * TICKS_PER_HOUR;
    w.rebuild_caches();
    let a = w.spawn_agent(Species::Person, TilePos::new(30, 30, 2));
    let b = w.spawn_agent(Species::Person, TilePos::new(31, 30, 2));
    let ia = w.agents.iter().position(|x| x.id == a).unwrap();
    let ib = w.agents.iter().position(|x| x.id == b).unwrap();
    w.agents[ia].genome.loci[CALM] = 10;
    w.agents[ib].genome.loci[CALM] = 200;
    hf_sim::systems::strain::arrival_shock(&mut w.agents[ia], w.meta.tick);
    hf_sim::systems::strain::arrival_shock(&mut w.agents[ib], w.meta.tick);
    let mut broke = false;
    for _ in 0..2 * TICKS_PER_HOUR {
        for x in w.agents.iter_mut() {
            x.needs.hunger = 1.0;
            x.needs.thirst = 1.0;
            x.needs.rest = 1.0;
            x.needs.warmth = 1.0;
            x.needs.social = 1.0;
            x.pos = if x.id == a {
                TilePos::new(30, 30, 2)
            } else {
                TilePos::new(31, 30, 2)
            };
            x.activity = Activity::Loitering {
                ticks_left: TICKS_PER_HOUR as u32,
            };
            x.activity_ticks = 0;
        }
        w.step();
        broke |= w
            .last_events
            .iter()
            .any(|e| matches!(&e.kind, EventKind::Broke { agent, .. } if *agent == a));
    }
    assert!(
        broke,
        "the tail survivor did not break beside their companion"
    );
    w.run(20);
    assert!(
        w.agents[ib].thoughts.has(ThoughtKind::SawBreak),
        "the companion did not carry the sight of the break"
    );
}

#[test]
fn fresh_arrivals_put_leisure_off() {
    // One founder, an engraving order a walk away, needs pinned except the social, which
    // is starved (0.25): in `choose`, `pressing = social_p * leisure` gates whether work
    // is considered at all, so on master both twins press at 0.75 from the first tick and
    // the work is never considered. The #395 gate divides the fresh twin's pressing by
    // the urge (0.50): the carving stays on the table all day and the leisure counts
    // split. Engraving is plain work — no #392 survival channel touches it — so on
    // master the twins decide exactly alike and the strict ordering fails.
    let day = |with_thought: bool| -> u64 {
        let mut w = World::generate(41, GenParams::small());
        w.flatten(2, 2, 60, 60, 1);
        w.plants.clear();
        w.meta.founding_site = TilePos::new(30, 30, 2);
        w.meta.tick = 8 * TICKS_PER_HOUR;
        let ids = w.found_party(1);
        w.drop_item(hf_sim::items::ItemKind::Meal, 10, TilePos::new(30, 30, 2));
        w.meta.autoplan = false;
        w.ensure_nav();
        let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
        w.agents[i].needs.social = 0.25;
        if with_thought {
            w.agents[i]
                .thoughts
                .add_shaped(ThoughtKind::Arrival, -0.35, 3.5, w.meta.tick);
        }
        let mut next_spot = 0i32;
        let mut leisure = 0u64;
        for _ in 0..TICKS_PER_DAY {
            // The work offer never runs out: one carving is three hours, and a fresh
            // order goes up as soon as the last is cut.
            if !w.designations.iter().any(|d| !d.done) {
                next_spot += 1;
                let at = w.grid.surface(48 + next_spot % 6, 28 + (next_spot / 6) % 6);
                let d = hf_sim::systems::settlement::make(
                    &mut w,
                    hf_sim::work::DesignationKind::Engrave,
                    at,
                    2,
                );
                w.designations.push(d);
            }
            {
                let a = &mut w.agents[i];
                a.needs.hunger = 1.0;
                a.needs.thirst = 1.0;
                a.needs.rest = 1.0;
                a.needs.warmth = 1.0;
            }
            w.step();
            let a = &w.agents[i];
            if matches!(
                a.activity,
                Activity::Wandering { .. } | Activity::Loitering { .. } | Activity::Talking { .. }
            ) {
                leisure += 1;
            }
        }
        leisure
    };
    let fresh = day(true);
    let calm = day(false);
    assert!(
        fresh < calm,
        "the fresh arrival did not put leisure off: {fresh} vs {calm}"
    );
}

#[test]
fn the_missing_are_dreaded_and_asked_after() {
    // Guard-before-fix: no Missing thought exists and no such event is ever pushed.
    let mut w = opening(7);
    let members: Vec<usize> = survivors(&w)
        .into_iter()
        .filter(|&i| w.agents[i].community.is_some())
        .collect();
    assert_eq!(members.len(), 3, "three begin at the landing");
    // By label, so this file compiles on unmodified master too: there the check fails
    // (no such thought), here it pins the new ThoughtKind::Missing.
    for &i in &members {
        assert!(
            w.agents[i]
                .thoughts
                .list
                .iter()
                .any(|t| t.kind.label() == "the missing"),
            "{} does not dread the missing",
            w.agents[i].name
        );
    }
    let mut asked = false;
    for _ in 0..2 * TICKS_PER_DAY {
        w.step();
        asked |= w.last_events.iter().any(
            |e| matches!(&e.kind, EventKind::Chose { what, .. } if what.contains("wondered where")),
        );
    }
    assert!(asked, "nobody was heard asking after the missing");
}

#[test]
fn a_steady_presence_eases_the_strain() {
    // Guard-before-fix: with and without a markedly calmer companion, stress eases at
    // the same hourly rate, so the strict ordering fails. The two came through the
    // portal (the shock leaves the Missing dread that makes one steadiable, #395);
    // their calm is set above the panic line so the spike itself does not break them.
    let stress_after = |with_steady: bool| -> f32 {
        let mut w = World::generate(2375, GenParams::small());
        w.flatten(2, 2, 60, 60, 1);
        w.meta.founding_site = TilePos::new(30, 30, 2);
        w.meta.tick = 0;
        w.rebuild_caches();
        let a = w.spawn_agent(Species::Person, TilePos::new(30, 30, 2));
        if let Some(x) = w.agents.iter_mut().find(|x| x.id == a) {
            x.genome.loci[CALM] = 60;
        }
        if with_steady {
            let b = w.spawn_agent(Species::Person, TilePos::new(31, 30, 2));
            if let Some(x) = w.agents.iter_mut().find(|x| x.id == b) {
                x.genome.loci[CALM] = 250;
            }
        }
        for x in w.agents.iter_mut() {
            hf_sim::systems::strain::arrival_shock(x, 0);
            x.stress = 0.5;
        }
        for _ in 0..12 {
            for x in w.agents.iter_mut() {
                x.needs.hunger = 1.0;
                x.needs.thirst = 1.0;
                x.needs.rest = 1.0;
                x.needs.warmth = 1.0;
                x.needs.social = 1.0;
                x.pos = if x.id == a {
                    TilePos::new(30, 30, 2)
                } else {
                    TilePos::new(31, 30, 2)
                };
                x.activity = Activity::Loitering {
                    ticks_left: TICKS_PER_HOUR as u32,
                };
                x.activity_ticks = 0;
            }
            w.run(TICKS_PER_HOUR);
        }
        w.agents
            .iter()
            .find(|x| x.id == a)
            .map(|x| x.stress)
            .unwrap()
    };
    let alone = stress_after(false);
    let steadied = stress_after(true);
    assert!(
        steadied + 0.005 < alone,
        "the calmer presence did not ease the strain: {steadied} vs {alone}"
    );
}
