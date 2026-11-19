//! Bounded personal experience readout for inspection (E16).
//!
//! This view is deliberately built from one selected agent only. It does not inspect the world,
//! event history, or any other agent, so the client cannot accidentally turn personal knowledge
//! into a global fact.

use crate::agents::Agent;
use crate::ids::TilePos;
use crate::mind::{Evidence, ObservationKind, OutcomeKind, ProblemStatus};

pub const OBSERVATION_LIMIT: usize = 12;
pub const LESSON_LIMIT: usize = 8;
pub const PROBLEM_LIMIT: usize = 8;
pub const REACTION_LIMIT: usize = 12;

#[derive(Clone, Debug, PartialEq)]
pub struct ExperienceReadout {
    pub observations: Vec<ObservationReadout>,
    pub lessons: Vec<LessonReadout>,
    pub problems: Vec<ProblemReadout>,
    pub reaction_trace: Vec<ReactionReadout>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObservationReadout {
    pub tick: u64,
    pub pos: TilePos,
    pub kind: String,
    pub strength: f32,
    pub full: bool,
    pub evidence: String,
    pub event: Option<u32>,
    /// What they made of it, when expertise read something into it (#569).
    pub reading: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LessonReadout {
    pub cue: String,
    pub outcome: String,
    pub pos: Option<TilePos>,
    pub confidence: f32,
    pub confirmations: u32,
    pub contradictions: u32,
    pub formed: u64,
    pub last_tested: u64,
    pub origins: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProblemReadout {
    pub kind: String,
    pub pos: TilePos,
    pub urgency: f32,
    pub status: String,
    pub evidence: String,
    pub opened: u64,
    pub last_seen: u64,
    pub trigger: Option<u32>,
    pub reaction: Option<String>,
    pub outcome: Option<String>,
    pub attempts: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReactionReadout {
    pub problem: String,
    pub pos: TilePos,
    pub tick: u64,
    pub reaction: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
    /// "pending" is used until an actual outcome is recorded; `success == false` alone is not
    /// enough to call an attempt a failure because pending attempts use that default value.
    pub outcome: String,
    pub event: Option<u32>,
}

/// Build a small, honest inspection snapshot from one person's bounded records.
pub fn inspect(a: &Agent) -> ExperienceReadout {
    // #480 review, FIXLOG 2026-10-29: refresh mutates old records in place, so recency uses
    // timestamps and latest insertion only as the deterministic tie break.
    let mut observations = a
        .learning
        .observations
        .iter()
        .enumerate()
        .collect::<Vec<_>>();
    observations
        .sort_by(|(a_index, a), (b_index, b)| b.tick.cmp(&a.tick).then(b_index.cmp(a_index)));
    let observations = observations
        .into_iter()
        .take(OBSERVATION_LIMIT)
        .map(|(_, o)| ObservationReadout {
            tick: o.tick,
            pos: o.pos,
            kind: observation_label(o.kind),
            strength: o.strength,
            full: o.full,
            evidence: evidence_label(o.evidence),
            event: o.event.map(|id| id.0),
            reading: o.reading.map(reading_label),
        })
        .collect();

    let mut lessons = a.learning.lessons.iter().enumerate().collect::<Vec<_>>();
    lessons.sort_by(|(a_index, a), (b_index, b)| {
        b.last_tested
            .cmp(&a.last_tested)
            .then(b.formed.cmp(&a.formed))
            .then(b_index.cmp(a_index))
    });
    let lessons = lessons
        .into_iter()
        .take(LESSON_LIMIT)
        .map(|(_, l)| LessonReadout {
            cue: observation_label(l.cue),
            outcome: outcome_label(l.outcome),
            pos: l.pos,
            confidence: l.confidence,
            confirmations: l.confirmations,
            contradictions: l.contradictions,
            formed: l.formed,
            last_tested: l.last_tested,
            origins: l.origins.iter().map(|id| id.0).collect(),
        })
        .collect();

    let mut selected = a.problems.records.iter().enumerate().collect::<Vec<_>>();
    selected.sort_by(|(a_index, a), (b_index, b)| {
        b.last_seen
            .cmp(&a.last_seen)
            .then(b.opened.cmp(&a.opened))
            .then(b_index.cmp(a_index))
    });
    let problems = selected
        .into_iter()
        .take(PROBLEM_LIMIT)
        .map(|(_, p)| ProblemReadout {
            kind: p.kind.label().to_string(),
            pos: p.pos,
            urgency: p.urgency,
            status: status_label(p.status),
            evidence: evidence_label(p.evidence),
            opened: p.opened,
            last_seen: p.last_seen,
            trigger: p.trigger.map(|id| id.0),
            reaction: p.reaction.map(|r| r.label().to_string()),
            outcome: p.outcome.map(outcome_label),
            attempts: p.attempts.len(),
        })
        .collect();

    let mut reactions = Vec::new();
    for (problem_index, p) in a.problems.records.iter().enumerate() {
        for (attempt_index, attempt) in p.attempts.iter().enumerate() {
            reactions.push((
                (attempt.tick, problem_index, attempt_index),
                ReactionReadout {
                    problem: p.kind.label().to_string(),
                    pos: p.pos,
                    tick: attempt.tick,
                    reaction: attempt.reaction.label().to_string(),
                    expected: attempt.expected.map(outcome_label),
                    actual: attempt.actual.map(outcome_label),
                    outcome: attempt_outcome(attempt.actual, attempt.success),
                    event: attempt.event.map(|id| id.0),
                },
            ));
        }
        if let Some(reaction) = p.reaction.filter(|_| p.attempts.is_empty()) {
            reactions.push((
                (p.last_seen, problem_index, usize::MAX),
                ReactionReadout {
                    problem: p.kind.label().to_string(),
                    pos: p.pos,
                    tick: p.last_seen,
                    reaction: reaction.label().to_string(),
                    expected: None,
                    actual: None,
                    outcome: "pending".to_string(),
                    event: None,
                },
            ));
        }
    }
    reactions.sort_by(|(a, _), (b, _)| b.cmp(a));
    let reaction_trace = reactions
        .into_iter()
        .take(REACTION_LIMIT)
        .map(|(_, reaction)| reaction)
        .collect();

    ExperienceReadout {
        observations,
        lessons,
        problems,
        reaction_trace,
    }
}

fn observation_label(kind: ObservationKind) -> String {
    match kind {
        ObservationKind::WaterDepth => "water depth",
        ObservationKind::WaterCurrent => "water current",
        ObservationKind::Flooding => "flooding",
        ObservationKind::Wetness => "wetness",
        ObservationKind::Saturation => "saturation",
        ObservationKind::Pressure => "pressure",
        ObservationKind::Heat => "heat",
        ObservationKind::Cold => "cold",
        ObservationKind::Fire => "fire",
        ObservationKind::Smoke => "smoke",
        ObservationKind::Spoilage => "spoilage",
        ObservationKind::FoodExposure => "food exposure",
        ObservationKind::ResourceDepletion => "resource depletion",
        ObservationKind::Regrowth => "regrowth",
        ObservationKind::StructuralWeakness => "structural weakness",
        ObservationKind::Shelter => "shelter",
        ObservationKind::Other => "other",
        ObservationKind::Soil => "the ground",
        ObservationKind::Tracks => "animal sign",
        ObservationKind::Joint => "a stuck building site",
    }
    .to_string()
}

/// What the person made of a cue, in words (#569).
pub fn reading_label(r: crate::mind::Reading) -> String {
    match r {
        crate::mind::Reading::Ground { pct } if pct >= 95 => {
            "good ground: crops would do well here".to_string()
        }
        crate::mind::Reading::Ground { pct } if pct >= 80 => {
            format!("fair ground: crops would grow at {pct}% of the pace")
        }
        crate::mind::Reading::Ground { pct } => {
            format!("poor ground: crops would grow at {pct}% of the pace")
        }
        crate::mind::Reading::Passed(s) => {
            format!("a {} passed near", format!("{s:?}").to_lowercase())
        }
        crate::mind::Reading::Lacks { kind, count } => {
            format!("stuck for want of {count} {}", kind.label())
        }
    }
}

fn outcome_label(outcome: OutcomeKind) -> String {
    match outcome {
        OutcomeKind::Harm => "harm",
        OutcomeKind::Illness => "illness",
        OutcomeKind::Loss => "loss",
        OutcomeKind::Displacement => "displacement",
        OutcomeKind::Death => "death",
        OutcomeKind::Safety => "safety",
        OutcomeKind::Benefit => "benefit",
        OutcomeKind::Discovery => "discovery",
        OutcomeKind::NoChange => "no change",
    }
    .to_string()
}

fn evidence_label(evidence: Evidence) -> String {
    match evidence {
        Evidence::Direct => "directly observed".to_string(),
        Evidence::Inferred => "inferred".to_string(),
        Evidence::ToldBy(id) => format!("told by Agent#{}", id.0),
        Evidence::MisledBy(id) => format!("misled by Agent#{}", id.0),
        Evidence::Remembered(id) => format!("remembered event #{}", id.0),
    }
}

fn status_label(status: ProblemStatus) -> String {
    match status {
        ProblemStatus::Open => "open",
        ProblemStatus::Attempted => "attempted",
        ProblemStatus::Resolved => "resolved",
        ProblemStatus::Failed => "failed",
        ProblemStatus::Abandoned => "abandoned",
    }
    .to_string()
}

fn attempt_outcome(actual: Option<OutcomeKind>, success: bool) -> String {
    match actual {
        None => "pending".to_string(),
        Some(_) if success => "succeeded".to_string(),
        Some(_) => "failed".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{AgentId, EventId};
    use crate::mind::{Learning, Observation, ProblemId, Problems, ReactionAttempt, ReactionKind};

    // This test uses the same selected-agent path as the bridge and proves that the read model
    // never needs a World to produce the four personal sections.
    #[test]
    fn readout_preserves_evidence_deltas_and_pending_outcomes() {
        let mut learning = Learning::default();
        let pos = TilePos::new(2, 3, 1);
        learning.observe(Observation {
            tick: 1,
            pos,
            kind: ObservationKind::WaterDepth,
            strength: 0.4,
            evidence: Evidence::Direct,
            event: None,
            full: false,
            reading: None,
        });
        learning.observe(Observation {
            tick: 2,
            pos,
            kind: ObservationKind::WaterDepth,
            strength: 0.9,
            evidence: Evidence::MisledBy(AgentId(7)),
            event: Some(EventId(8)),
            full: false,
            reading: None,
        });
        let mut problems = Problems::default();
        let problem = problems.observe(
            crate::mind::ProblemKind::WaterHazard,
            pos,
            Evidence::ToldBy(AgentId(7)),
            0.8,
            3,
            Some(EventId(9)),
        );
        assert!(problems.choose(problem, ReactionKind::MoveToSafety));
        let mut world = crate::World::stage_demo(1);
        let agent = world
            .agents
            .iter_mut()
            .find(|a| a.species == crate::agents::Species::Person)
            .expect("staged world has a person");
        agent.learning = learning;
        agent.problems = problems;
        let view = inspect(agent);
        assert_eq!(view.observations.len(), 2);
        assert!(!view.observations[0].full);
        assert_eq!(view.observations[0].evidence, "misled by Agent#7");
        assert_eq!(view.problems[0].evidence, "told by Agent#7");
        assert_eq!(view.reaction_trace[0].outcome, "pending");
        assert_eq!(view.reaction_trace[0].reaction, "move to safety");

        // A real attempt with no actual result remains pending rather than being called failed.
        agent.problems.record_attempt(
            ProblemId(0),
            ReactionAttempt {
                tick: 4,
                reaction: ReactionKind::MoveToSafety,
                expected: Some(OutcomeKind::Safety),
                actual: None,
                success: false,
                event: Some(EventId(10)),
                target: None,
            },
        );
        assert_eq!(inspect(agent).reaction_trace[0].outcome, "pending");
    }

    fn readout_for(learning: Learning, problems: Problems) -> ExperienceReadout {
        let mut world = crate::World::stage_demo(1);
        let agent = world
            .agents
            .iter_mut()
            .find(|a| a.species == crate::agents::Species::Person)
            .expect("staged world has a person");
        agent.learning = learning;
        agent.problems = problems;
        inspect(agent)
    }

    fn at(tick: u64) -> TilePos {
        TilePos::new(tick as i32, 2, 1)
    }

    #[test]
    fn recent_sections_follow_refresh_ticks_not_insertion_order() {
        let mut learning = Learning::default();
        for tick in 1..=(OBSERVATION_LIMIT as u64 + 1) {
            learning.observe(Observation {
                tick,
                pos: at(tick),
                kind: ObservationKind::WaterDepth,
                strength: 0.4,
                evidence: Evidence::Direct,
                event: Some(EventId(tick as u32)),
                full: false,
                reading: None,
            });
        }
        for tick in 1..=(LESSON_LIMIT as u64 + 1) {
            learning.learn(
                ObservationKind::WaterDepth,
                OutcomeKind::Safety,
                Some(at(tick)),
                true,
                tick,
                Some(EventId(tick as u32)),
            );
        }
        let mut problems = Problems::default();
        for tick in 1..=(PROBLEM_LIMIT as u64 + 1) {
            problems.observe(
                crate::mind::ProblemKind::WaterHazard,
                at(tick),
                Evidence::Direct,
                0.8,
                tick,
                Some(EventId(tick as u32)),
            );
        }

        learning.observe(Observation {
            tick: 100,
            pos: at(1),
            kind: ObservationKind::WaterDepth,
            strength: 0.4,
            evidence: Evidence::Direct,
            event: Some(EventId(1)),
            full: false,
            reading: None,
        });
        learning.learn(
            ObservationKind::WaterDepth,
            OutcomeKind::Safety,
            Some(at(1)),
            true,
            100,
            Some(EventId(100)),
        );
        problems.observe(
            crate::mind::ProblemKind::WaterHazard,
            at(1),
            Evidence::Direct,
            0.8,
            100,
            Some(EventId(100)),
        );

        let view = readout_for(learning, problems);
        assert_eq!(view.observations.len(), OBSERVATION_LIMIT);
        assert_eq!(view.observations[0].tick, 100);
        assert_eq!(view.observations[0].event, Some(1));
        assert_eq!(view.observations[1].tick, 13);
        assert!(!view.observations.iter().any(|o| o.tick == 2));

        assert_eq!(view.lessons.len(), LESSON_LIMIT);
        assert_eq!(view.lessons[0].formed, 1);
        assert_eq!(view.lessons[0].last_tested, 100);
        assert_eq!(view.lessons[0].pos, Some(at(1)));
        assert_eq!(view.lessons[0].origins, vec![1, 100]);
        assert_eq!(view.lessons[1].last_tested, 9);
        assert!(!view.lessons.iter().any(|l| l.last_tested == 2));

        assert_eq!(view.problems.len(), PROBLEM_LIMIT);
        assert_eq!(view.problems[0].opened, 1);
        assert_eq!(view.problems[0].last_seen, 100);
        assert_eq!(view.problems[0].pos, at(1));
        assert_eq!(view.problems[1].last_seen, 9);
        assert!(!view.problems.iter().any(|p| p.last_seen == 2));
    }

    #[test]
    fn reaction_trace_keeps_latest_attempts_across_problems() {
        let mut problems = Problems::default();
        for problem in 0..3 {
            let index = problems.observe(
                crate::mind::ProblemKind::WaterHazard,
                at(problem + 1),
                Evidence::Direct,
                0.8,
                1,
                None,
            );
            for attempt in 0..5 {
                let tick = problem * 10 + attempt + 1;
                assert!(problems.record_attempt(
                    index,
                    ReactionAttempt {
                        tick,
                        reaction: ReactionKind::MoveToSafety,
                        expected: Some(OutcomeKind::Safety),
                        actual: None,
                        success: false,
                        event: Some(EventId(tick as u32)),
                        target: None,
                    },
                ));
            }
        }

        let view = readout_for(Learning::default(), problems);
        assert_eq!(view.reaction_trace.len(), REACTION_LIMIT);
        let ticks = view
            .reaction_trace
            .iter()
            .map(|r| r.tick)
            .collect::<Vec<_>>();
        assert_eq!(ticks, vec![25, 24, 23, 22, 21, 15, 14, 13, 12, 11, 5, 4]);
    }

    #[test]
    fn equal_timestamps_use_latest_insertion_order() {
        let mut learning = Learning::default();
        for pos in [at(1), at(2)] {
            learning.observe(Observation {
                tick: 10,
                pos,
                kind: ObservationKind::WaterDepth,
                strength: 0.4,
                evidence: Evidence::Direct,
                event: None,
                full: false,
                reading: None,
            });
            learning.learn(
                ObservationKind::WaterDepth,
                OutcomeKind::Safety,
                Some(pos),
                true,
                10,
                None,
            );
        }
        let mut problems = Problems::default();
        for pos in [at(1), at(2)] {
            let index = problems.observe(
                crate::mind::ProblemKind::WaterHazard,
                pos,
                Evidence::Direct,
                0.8,
                10,
                None,
            );
            assert!(problems.record_attempt(
                index,
                ReactionAttempt {
                    tick: 20,
                    reaction: ReactionKind::MoveToSafety,
                    expected: None,
                    actual: None,
                    success: false,
                    event: None,
                    target: None,
                },
            ));
        }

        let view = readout_for(learning, problems);
        assert_eq!(view.observations[0].pos, at(2));
        assert_eq!(view.lessons[0].pos, Some(at(2)));
        assert_eq!(view.problems[0].pos, at(2));
        assert_eq!(view.reaction_trace[0].pos, at(2));
    }
}
