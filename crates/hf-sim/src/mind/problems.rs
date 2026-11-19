//! Personal problems and reaction traces (E16).
//!
//! A problem is not a quest owned by the world. It is a person's bounded record that a
//! personally observed condition needs attention. The system that owns the physical response
//! records its pending attempt and actual outcome against the same stable personal record.

use crate::ids::{AgentId, EventId, TilePos};
use crate::mind::{Evidence, ObservationKind, OutcomeKind};
use serde::{Deserialize, Serialize};

pub const PROBLEM_CAP: usize = 32;
pub const ATTEMPT_CAP: usize = 8;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum ProblemKind {
    WaterHazard,
    Flooding,
    FoodSpoilage,
    FoodShortage,
    ResourceDepletion,
    HeatExposure,
    Fire,
    StructuralDanger,
    Other,
}

impl ProblemKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::WaterHazard => "dangerous water",
            Self::Flooding => "flooding",
            Self::FoodSpoilage => "food spoiling",
            Self::FoodShortage => "food shortage",
            Self::ResourceDepletion => "resources depleted",
            Self::HeatExposure => "dangerous heat",
            Self::Fire => "fire",
            Self::StructuralDanger => "structural danger",
            Self::Other => "a problem",
        }
    }

    pub fn cue(self) -> ObservationKind {
        match self {
            Self::WaterHazard => ObservationKind::WaterDepth,
            Self::Flooding => ObservationKind::Flooding,
            Self::FoodSpoilage => ObservationKind::Spoilage,
            Self::FoodShortage => ObservationKind::ResourceDepletion,
            Self::ResourceDepletion => ObservationKind::ResourceDepletion,
            Self::HeatExposure => ObservationKind::Heat,
            Self::Fire => ObservationKind::Fire,
            Self::StructuralDanger => ObservationKind::StructuralWeakness,
            Self::Other => ObservationKind::Other,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum ReactionKind {
    MoveToSafety,
    SeekShelter,
    MoveSupplies,
    PreserveFood,
    SearchResources,
    WarnOthers,
    Investigate,
    AskForHelp,
    Wait,
    Ignore,
    Flee,
    Other,
}

impl ReactionKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::MoveToSafety => "move to safety",
            Self::SeekShelter => "seek shelter",
            Self::MoveSupplies => "move supplies",
            Self::PreserveFood => "preserve food",
            Self::SearchResources => "search for resources",
            Self::WarnOthers => "warn others",
            Self::Investigate => "investigate",
            Self::AskForHelp => "ask for help",
            Self::Wait => "wait",
            Self::Ignore => "ignore it",
            Self::Flee => "flee",
            Self::Other => "try something else",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum ProblemStatus {
    Open,
    Attempted,
    Resolved,
    Failed,
    Abandoned,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct ProblemId(pub u64);

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct ReactionAttempt {
    pub tick: u64,
    pub reaction: ReactionKind,
    pub expected: Option<OutcomeKind>,
    pub actual: Option<OutcomeKind>,
    pub success: bool,
    pub event: Option<EventId>,
    #[serde(default)]
    pub target: Option<TilePos>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Problem {
    #[serde(default)]
    pub id: ProblemId,
    pub kind: ProblemKind,
    pub pos: TilePos,
    pub evidence: Evidence,
    pub opened: u64,
    pub last_seen: u64,
    /// 0..1, from the observer's point of view rather than an objective global urgency.
    pub urgency: f32,
    pub trigger: Option<EventId>,
    pub status: ProblemStatus,
    pub reaction: Option<ReactionKind>,
    pub attempts: Vec<ReactionAttempt>,
    pub outcome: Option<OutcomeKind>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Problems {
    pub records: Vec<Problem>,
    #[serde(default)]
    pub next_id: u64,
}

/// A warning transferred between people during a conversation.
#[derive(Clone, Copy, Debug)]
pub struct HeardProblem {
    pub teller: AgentId,
    pub kind: ProblemKind,
    pub pos: TilePos,
    pub urgency: f32,
    pub trust: f32,
    pub tick: u64,
    pub trigger: Option<EventId>,
}

impl Problems {
    /// Open or refresh a problem known by this person. New direct evidence reopens a terminal
    /// record without erasing its attempts (FIXLOG 2026-10-30, PR #476).
    pub fn observe(
        &mut self,
        kind: ProblemKind,
        pos: TilePos,
        evidence: Evidence,
        urgency: f32,
        tick: u64,
        trigger: Option<EventId>,
    ) -> ProblemId {
        let urgency = urgency.clamp(0.0, 1.0);
        if let Some(problem) = self
            .records
            .iter_mut()
            .find(|x| x.kind == kind && x.pos == pos)
        {
            if matches!(
                problem.status,
                ProblemStatus::Resolved | ProblemStatus::Failed | ProblemStatus::Abandoned
            ) {
                problem.status = ProblemStatus::Open;
                problem.reaction = None;
                problem.outcome = None;
            }
            problem.evidence = evidence;
            problem.last_seen = tick;
            problem.urgency = urgency;
            if trigger.is_some() {
                problem.trigger = trigger;
            }
            return problem.id;
        }
        if self.records.len() >= PROBLEM_CAP {
            let remove = self
                .records
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.urgency
                        .partial_cmp(&b.urgency)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then(a.last_seen.cmp(&b.last_seen))
                })
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.records.remove(remove);
        }
        self.next_id = self
            .next_id
            .max(self.records.iter().map(|p| p.id.0).max().unwrap_or(0) + 1);
        let id = ProblemId(self.next_id.max(1));
        self.next_id = id.0 + 1;
        self.records.push(Problem {
            id,
            kind,
            pos,
            evidence,
            opened: tick,
            last_seen: tick,
            urgency,
            trigger,
            status: ProblemStatus::Open,
            reaction: None,
            attempts: Vec::new(),
            outcome: None,
        });
        id
    }

    /// Assign identities to records from the first E16 saves, which predate persistent ids.
    /// Records made in-session always carry ids, and an empty ledger keeps `next_id` untouched,
    /// so a restore stays byte-identical to the world that was saved and a non-empty repair
    /// never advances the counter past its records (FIXLOG 2026-10-31, c146, PR #476).
    pub fn repair_ids(&mut self) {
        if self.records.is_empty() {
            return;
        }
        let mut used = std::collections::BTreeSet::new();
        let mut next = self.next_id;
        for problem in &mut self.records {
            if problem.id.0 == 0 || used.contains(&problem.id) {
                next = next.max(1);
                while used.contains(&ProblemId(next)) {
                    next += 1;
                }
                problem.id = ProblemId(next);
                next += 1;
            }
            used.insert(problem.id);
            next = next.max(problem.id.0 + 1);
        }
        self.next_id = next;
    }

    pub fn get(&self, id: ProblemId) -> Option<&Problem> {
        self.records.iter().find(|p| p.id == id)
    }

    pub fn get_mut(&mut self, id: ProblemId) -> Option<&mut Problem> {
        self.records.iter_mut().find(|p| p.id == id)
    }

    pub fn pending(&self, id: ProblemId) -> Option<ReactionAttempt> {
        self.get(id)?
            .attempts
            .iter()
            .rev()
            .find(|a| a.actual.is_none())
            .copied()
    }

    pub fn pending_any(&self) -> Option<(ProblemId, ReactionAttempt)> {
        self.records
            .iter()
            .find_map(|p| self.pending(p.id).map(|attempt| (p.id, attempt)))
    }

    pub fn choose(&mut self, id: ProblemId, reaction: ReactionKind) -> bool {
        let Some(problem) = self.get_mut(id) else {
            return false;
        };
        if !matches!(
            problem.status,
            ProblemStatus::Open | ProblemStatus::Attempted
        ) {
            return false;
        }
        problem.reaction = Some(reaction);
        problem.status = ProblemStatus::Attempted;
        true
    }

    /// Record a real attempt and its result. A failed attempt is still useful evidence and is
    /// not erased or converted into a successful solution.
    pub fn record_attempt(&mut self, id: ProblemId, attempt: ReactionAttempt) -> bool {
        let Some(problem) = self.get_mut(id) else {
            return false;
        };
        if !matches!(
            problem.status,
            ProblemStatus::Open | ProblemStatus::Attempted
        ) {
            return false;
        }
        if problem.attempts.len() >= ATTEMPT_CAP {
            problem.attempts.remove(0);
        }
        problem.reaction = Some(attempt.reaction);
        problem.status = ProblemStatus::Attempted;
        problem.attempts.push(attempt);
        if let Some(actual) = attempt.actual {
            problem.outcome = Some(actual);
        }
        true
    }

    pub fn complete_attempt(
        &mut self,
        id: ProblemId,
        outcome: OutcomeKind,
        success: bool,
        tick: u64,
    ) -> Option<ReactionAttempt> {
        let problem = self.get_mut(id)?;
        let attempt = problem
            .attempts
            .iter_mut()
            .rev()
            .find(|a| a.actual.is_none())?;
        attempt.actual = Some(outcome);
        attempt.success = success;
        let completed = *attempt;
        problem.last_seen = tick;
        problem.outcome = Some(outcome);
        problem.status = if success {
            ProblemStatus::Resolved
        } else {
            ProblemStatus::Failed
        };
        Some(completed)
    }

    pub fn resolve(
        &mut self,
        id: ProblemId,
        outcome: OutcomeKind,
        success: bool,
        tick: u64,
    ) -> bool {
        let Some(problem) = self.get_mut(id) else {
            return false;
        };
        problem.last_seen = tick;
        problem.outcome = Some(outcome);
        problem.status = if success {
            ProblemStatus::Resolved
        } else {
            ProblemStatus::Failed
        };
        true
    }

    pub fn abandon(&mut self, id: ProblemId, tick: u64) -> bool {
        let Some(problem) = self.get_mut(id) else {
            return false;
        };
        problem.last_seen = tick;
        problem.status = ProblemStatus::Abandoned;
        true
    }

    pub fn open_count(&self) -> usize {
        self.records
            .iter()
            .filter(|x| matches!(x.status, ProblemStatus::Open | ProblemStatus::Attempted))
            .count()
    }

    pub fn strongest_open(&self) -> Option<(usize, &Problem)> {
        self.records
            .iter()
            .enumerate()
            .filter(|(_, p)| matches!(p.status, ProblemStatus::Open | ProblemStatus::Attempted))
            .max_by(|(_, a), (_, b)| {
                a.urgency
                    .partial_cmp(&b.urgency)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Receive a practical warning from a person. Trust affects urgency, but the source remains
    /// attached so a later outcome can confirm or discredit the teller.
    pub fn hear(&mut self, report: HeardProblem) -> ProblemId {
        // FIXLOG 2026-10-29 feat/e16-social-knowledge: a listener does not learn deceptive
        // intent from the warning itself; later evidence may discredit the teller.
        self.observe(
            report.kind,
            report.pos,
            Evidence::ToldBy(report.teller),
            (report.urgency * (0.25 + 0.75 * report.trust.clamp(0.0, 1.0))).clamp(0.0, 1.0),
            report.tick,
            report.trigger,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: i32) -> TilePos {
        TilePos::new(x, 3, 1)
    }

    #[test]
    fn repeated_observation_refreshes_one_personal_problem() {
        let mut p = Problems::default();
        let a = p.observe(
            ProblemKind::Flooding,
            at(2),
            Evidence::Direct,
            0.4,
            10,
            Some(EventId(1)),
        );
        let b = p.observe(
            ProblemKind::Flooding,
            at(2),
            Evidence::Direct,
            0.8,
            20,
            None,
        );
        assert_eq!(a, b);
        assert_eq!(p.records.len(), 1);
        assert_eq!(p.records[0].last_seen, 20);
        assert_eq!(p.records[0].urgency, 0.8);
        assert_eq!(p.records[0].trigger, Some(EventId(1)));
    }

    #[test]
    fn failed_reaction_remains_visible_as_a_real_outcome() {
        let mut p = Problems::default();
        let id = p.observe(
            ProblemKind::WaterHazard,
            at(4),
            Evidence::Direct,
            0.9,
            2,
            Some(EventId(3)),
        );
        assert!(p.choose(id, ReactionKind::MoveToSafety));
        assert!(p.record_attempt(
            id,
            ReactionAttempt {
                tick: 3,
                reaction: ReactionKind::MoveToSafety,
                expected: Some(OutcomeKind::Safety),
                actual: Some(OutcomeKind::Displacement),
                success: false,
                event: Some(EventId(4)),
                target: None,
            }
        ));
        assert!(p.resolve(id, OutcomeKind::Displacement, false, 4));
        assert_eq!(p.get(id).unwrap().status, ProblemStatus::Failed);
        assert_eq!(p.get(id).unwrap().outcome, Some(OutcomeKind::Displacement));
        assert_eq!(p.get(id).unwrap().attempts[0].event, Some(EventId(4)));
    }

    #[test]
    fn renewed_evidence_reopens_terminal_records_without_losing_attempts() {
        let mut p = Problems::default();
        let id = p.observe(
            ProblemKind::WaterHazard,
            at(4),
            Evidence::Direct,
            0.9,
            2,
            None,
        );
        assert!(p.resolve(id, OutcomeKind::Safety, true, 3));
        assert_eq!(
            p.observe(
                ProblemKind::WaterHazard,
                at(4),
                Evidence::Direct,
                0.9,
                4,
                None
            ),
            id
        );
        assert_eq!(p.get(id).unwrap().status, ProblemStatus::Open);
        assert!(p.choose(id, ReactionKind::MoveToSafety));
        assert!(p.record_attempt(
            id,
            ReactionAttempt {
                tick: 5,
                reaction: ReactionKind::MoveToSafety,
                expected: Some(OutcomeKind::Safety),
                actual: None,
                success: false,
                event: None,
                target: Some(at(5)),
            }
        ));
        assert!(p
            .complete_attempt(id, OutcomeKind::NoChange, false, 6)
            .is_some());
        assert!(p.abandon(id, 6));
        assert_eq!(
            p.observe(
                ProblemKind::WaterHazard,
                at(4),
                Evidence::Direct,
                0.9,
                7,
                None
            ),
            id
        );
        assert_eq!(p.get(id).unwrap().status, ProblemStatus::Open);
        assert_eq!(p.get(id).unwrap().attempts.len(), 1);
    }
}
