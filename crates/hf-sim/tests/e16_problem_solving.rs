//! E16 problem/reaction records are personal, bounded, and linked to real pipeline outcomes.

use hf_sim::chronicle::Chronicle;
use hf_sim::events::EventKind;
use hf_sim::ids::{AgentId, EventId, TilePos};
use hf_sim::mind::{
    Evidence, OutcomeKind, ProblemKind, ProblemStatus, Problems, ReactionAttempt, ReactionKind,
    ATTEMPT_CAP, PROBLEM_CAP,
};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Tile, TileFlags};
use hf_sim::{save, World};

fn at(x: i32) -> TilePos {
    TilePos::new(x, 3, 1)
}

fn meadow() -> World {
    let mut w = World::generate(476, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.flooded.clear();
    w.waters = Default::default();
    w.meta.autoplan = false;
    w.meta.tick = 0;
    w.rebuild_caches();
    w
}

fn person(w: &mut World, pos: TilePos) -> usize {
    let id = w.spawn_agent(hf_sim::agents::Species::Person, pos);
    let i = w.agents.iter().position(|a| a.id == id).unwrap();
    w.agents[i].needs.hunger = 1.0;
    w.agents[i].needs.rest = 1.0;
    w.agents[i].needs.warmth = 1.0;
    w.agents[i].needs.social = 1.0;
    w.agents[i].needs.comfort = 1.0;
    w.agents[i].needs.thirst = 1.0;
    i
}

fn water(w: &mut World, pos: TilePos, flags: u8) {
    let mut tile = w.grid.get(pos);
    tile.flags = TileFlags(tile.flags.0 | TileFlags::WATER | flags);
    w.grid.set(pos, tile);
}

#[test]
fn problems_are_not_global_alerts() {
    let mut swimmer = Problems::default();
    let witness = Problems::default();
    swimmer.observe(
        ProblemKind::WaterHazard,
        at(8),
        Evidence::Direct,
        0.8,
        12,
        Some(EventId(21)),
    );
    assert_eq!(swimmer.open_count(), 1);
    assert_eq!(witness.open_count(), 0);
}

#[test]
fn problem_trace_keeps_expectation_attempt_and_real_result() {
    let mut problems = Problems::default();
    let id = problems.observe(
        ProblemKind::Flooding,
        at(4),
        Evidence::ToldBy(AgentId(2)),
        0.6,
        4,
        Some(EventId(30)),
    );
    assert!(problems.choose(id, ReactionKind::MoveSupplies));
    assert!(problems.record_attempt(
        id,
        ReactionAttempt {
            tick: 5,
            reaction: ReactionKind::MoveSupplies,
            expected: Some(OutcomeKind::Safety),
            actual: Some(OutcomeKind::Loss),
            success: false,
            event: Some(EventId(31)),
            target: None,
        }
    ));
    assert!(problems.resolve(id, OutcomeKind::Loss, false, 6));
    let record = problems.get(id).unwrap();
    assert_eq!(record.evidence, Evidence::ToldBy(AgentId(2)));
    assert_eq!(record.attempts[0].expected, Some(OutcomeKind::Safety));
    assert_eq!(record.attempts[0].actual, Some(OutcomeKind::Loss));
    assert_eq!(record.outcome, Some(OutcomeKind::Loss));
}

#[test]
fn dangerous_water_escape_is_perceived_attempted_resolved_and_learned() {
    let mut w = meadow();
    let here = TilePos::new(30, 30, 2);
    water(&mut w, here.offset(-1, -1, 0), 0);
    water(&mut w, here, TileFlags::FULL);
    let swimmer = person(&mut w, here);
    let bystander = person(&mut w, here.offset(3, 0, 0));
    let swimmer_id = w.agents[swimmer].id;
    w.ensure_nav();
    let mut choices = Vec::new();
    for _ in 0..40 {
        w.step();
        choices.extend(w.last_events.iter().filter_map(|e| match &e.kind {
            EventKind::Chose { agent, what, .. } if *agent == swimmer_id => Some(what.clone()),
            _ => None,
        }));
    }

    let a = &w.agents[swimmer];
    assert!(a.alive && !hf_sim::systems::hazards::under_water(&w, a.pos));
    let problem = a
        .problems
        .records
        .iter()
        .find(|p| p.pos == here && p.kind == ProblemKind::WaterHazard)
        .expect("the current tile, not the first scanned water tile");
    assert_ne!(problem.id.0, 0);
    assert_eq!(problem.status, ProblemStatus::Resolved);
    assert_eq!(problem.outcome, Some(OutcomeKind::Safety));
    assert_eq!(problem.attempts.len(), 1);
    assert_eq!(problem.attempts[0].expected, Some(OutcomeKind::Safety));
    assert_eq!(problem.attempts[0].actual, Some(OutcomeKind::Safety));
    assert!(problem.attempts[0].success);
    assert!(problem.attempts[0].target.is_some());
    assert!(a
        .learning
        .lesson(
            hf_sim::mind::ObservationKind::WaterDepth,
            OutcomeKind::Safety,
            Some(here),
        )
        .is_some_and(|l| l.confirmations == 1));
    assert!(choices.iter().any(|s| s.contains("by moving to safety")));
    assert!(!choices.iter().any(|s| s.contains("by move to safety")));

    let other = &w.agents[bystander];
    assert!(other.problems.records.iter().all(|p| p.id != problem.id));
    assert!(other.problems.records.iter().all(|p| p.attempts.is_empty()));
    assert!(other.learning.lessons.is_empty());
}

#[test]
fn harmless_shallow_flooding_is_observed_without_a_reaction() {
    let mut w = meadow();
    let here = TilePos::new(30, 30, 2);
    let i = person(&mut w, here);
    water(&mut w, here, 0);
    w.flooded.insert(here);
    w.waters.surveyed = true;
    w.waters.bodies.push(hf_sim::floods::Body {
        tiles: vec![here],
        stage: 0.0,
        ..Default::default()
    });
    w.waters.flood_of.insert(here, 0);

    w.meta.tick = 19;
    w.step();
    assert!(w.agents[i].problems.records.is_empty());
    assert!(w.agents[i]
        .learning
        .observations
        .iter()
        .any(|o| o.pos == here && o.kind == hf_sim::mind::ObservationKind::Wetness));

    w.meta.tick = 50;
    hf_sim::systems::hazards::step(&mut w);
    assert!(w.agents[i].problems.records.is_empty());
}

#[test]
fn current_dangerous_tile_wins_over_an_earlier_scanned_water_tile() {
    let mut w = meadow();
    let here = TilePos::new(30, 30, 2);
    let i = person(&mut w, here);
    water(&mut w, here.offset(-1, -1, 0), 0);
    water(&mut w, here, TileFlags::FULL);

    w.meta.tick = 19;
    w.step();

    assert!(w.agents[i]
        .knowledge
        .get(hf_sim::mind::knowledge::Known::Water(here))
        .is_some());
    assert!(w.agents[i]
        .problems
        .records
        .iter()
        .any(|p| p.pos == here && p.kind == ProblemKind::WaterHazard));
}

#[test]
fn blocked_escape_fails_then_new_evidence_retries_the_same_problem() {
    let mut w = meadow();
    let here = TilePos::new(30, 30, 2);
    let exit = here.offset(1, 0, 0);
    let dry_tile = w.grid.get(exit);
    water(&mut w, here, TileFlags::FULL);
    let i = person(&mut w, here);
    for (dx, dy) in [
        (0, -1),
        (1, -1),
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
    ] {
        w.grid
            .set(here.offset(dx, dy, 0), Tile::solid(Material::GRANITE));
    }
    w.rebuild_caches();
    w.ensure_nav();

    hf_sim::systems::hazards::step(&mut w);
    let id = w.agents[i].problems.records[0].id;
    let failed = w.agents[i].problems.get(id).unwrap();
    assert_eq!(failed.status, ProblemStatus::Failed);
    assert_eq!(failed.attempts.len(), 1);
    assert_eq!(failed.attempts[0].actual, Some(OutcomeKind::NoChange));
    assert!(!failed.attempts[0].success);
    assert!(w.agents[i]
        .learning
        .lessons
        .iter()
        .any(|l| l.outcome == OutcomeKind::NoChange && l.contradictions == 1));

    w.grid.set(exit, dry_tile);
    w.rebuild_caches();
    w.ensure_nav();
    w.meta.tick = 5;
    hf_sim::systems::hazards::step(&mut w);
    assert_eq!(w.agents[i].problems.records.len(), 1);
    assert_eq!(w.agents[i].problems.records[0].id, id);
    assert_eq!(
        w.agents[i].problems.records[0].status,
        ProblemStatus::Attempted
    );
    for _ in 0..30 {
        w.step();
    }
    let retried = w.agents[i].problems.get(id).unwrap();
    assert_eq!(retried.status, ProblemStatus::Resolved);
    assert_eq!(retried.attempts.len(), 2);
    assert_eq!(retried.attempts[1].actual, Some(OutcomeKind::Safety));
    assert!(w.agents[i].alive);
}

#[test]
fn ids_survive_eviction_and_attempt_history_stays_bounded() {
    let mut problems = Problems::default();
    let mut kept = None;
    for x in 0..(PROBLEM_CAP + 4) {
        let id = problems.observe(
            ProblemKind::Flooding,
            at(x as i32),
            Evidence::Direct,
            0.5,
            x as u64,
            None,
        );
        if x == 20 {
            kept = Some(id);
        }
    }
    assert_eq!(problems.records.len(), PROBLEM_CAP);
    let kept = kept.unwrap();
    assert!(problems.get(kept).is_some());
    for n in 0..(ATTEMPT_CAP + 3) {
        assert!(problems.choose(kept, ReactionKind::MoveToSafety));
        assert!(problems.record_attempt(
            kept,
            ReactionAttempt {
                tick: n as u64,
                reaction: ReactionKind::MoveToSafety,
                expected: Some(OutcomeKind::Safety),
                actual: Some(OutcomeKind::NoChange),
                success: false,
                event: None,
                target: None,
            }
        ));
    }
    let record = problems.get(kept).unwrap();
    assert_eq!(record.attempts.len(), ATTEMPT_CAP);
    assert_eq!(record.attempts[0].tick, 3);
}

#[test]
fn pending_reaction_and_its_stable_identity_survive_save_load() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("reaction.hfsave");
    let mut w = meadow();
    let here = TilePos::new(30, 30, 2);
    water(&mut w, here, TileFlags::FULL);
    let i = person(&mut w, here);
    w.ensure_nav();
    hf_sim::systems::hazards::step(&mut w);
    let id = w.agents[i].problems.records[0].id;
    assert!(w.agents[i].problems.pending(id).is_some());

    let mut chronicle = Chronicle::in_memory().unwrap();
    save::write(&w, &mut chronicle, &path).unwrap();
    let (mut restored, _) = save::read(&path).unwrap();
    let restored_i = restored
        .agents
        .iter()
        .position(|a| a.id == w.agents[i].id)
        .unwrap();
    assert!(restored.agents[restored_i].problems.pending(id).is_some());
    for _ in 0..30 {
        restored.step();
    }
    let problem = restored.agents[restored_i].problems.get(id).unwrap();
    assert_eq!(problem.status, ProblemStatus::Resolved);
    assert_eq!(problem.attempts[0].actual, Some(OutcomeKind::Safety));
}
