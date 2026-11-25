//! Personal observations and lessons (E16).
//!
//! The world has one physical truth, but every person keeps their own partial record of what
//! they noticed and what they think an outcome taught them. This layer deliberately does not
//! decide actions: systems that own perception and planning will add observations and consult
//! lessons without creating a global problem solver.

use crate::ids::{EventId, TilePos};
use serde::{Deserialize, Serialize};

pub const OBSERVATION_CAP: usize = 128;
pub const LESSON_CAP: usize = 64;
pub const MAX_ORIGINS: usize = 6;

/// State-derived cues that can be noticed by a person. The enum is intentionally about
/// evidence, not conclusions: `WaterDepth` may lead to a correct or incorrect lesson later.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum ObservationKind {
    WaterDepth,
    /// Reserved: never written; wire when currents exist as simulation state (rule 10).
    WaterCurrent,
    Flooding,
    Wetness,
    Saturation,
    /// Reserved: never written; wire when pressure exists as simulation state (rule 10).
    Pressure,
    Heat,
    Cold,
    Fire,
    Smoke,
    Spoilage,
    /// Reserved: never written; wire when food exposure is tracked as state.
    FoodExposure,
    ResourceDepletion,
    /// Reserved: never written; wire when plant regrowth is read from state.
    Regrowth,
    StructuralWeakness,
    /// Reserved: never written; wire when shelter quality is read from state.
    Shelter,
    /// Reserved: never written; wire when a cue needs a catch-all.
    Other,
    /// What the ground would do for crops, read by one who knows farming (#569).
    Soil,
    /// Sign of an animal that passed unseen, read by one who knows animals (#569).
    Tracks,
    /// Why a building site is stuck, read by one who knows building (#569).
    Joint,
}

/// Cues no code writes yet. Each wiring slice removes its kind here and from the test's
/// hand-kept list of written kinds.
pub const RESERVED: &[ObservationKind] = &[
    ObservationKind::WaterCurrent,
    ObservationKind::Pressure,
    ObservationKind::FoodExposure,
    ObservationKind::Regrowth,
    ObservationKind::Shelter,
    ObservationKind::Other,
];

/// What expertise read into a cue (#569): detection alone says a cue was there; a reading says
/// what the person made of it. Only the noticing person's own reading, from the world as it is.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Reading {
    /// Crops on this ground would grow at this share of good ground's pace, in percent.
    Ground { pct: u8 },
    /// This kind of animal passed near.
    Passed(crate::agents::Species),
    /// The site is short of this many of this kind of thing.
    Lacks {
        kind: crate::items::ItemKind,
        count: u32,
    },
}

/// How the information reached this person. A told or remembered fact is not the same as a
/// direct observation, even when both happen to be true.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Evidence {
    Direct,
    Inferred,
    ToldBy(crate::ids::AgentId),
    Remembered(EventId),
    /// A later correction established that this person intentionally distorted an account.
    // FIXLOG 2026-10-29 feat/e16-social-knowledge: keep saved evidence variants append-only.
    MisledBy(crate::ids::AgentId),
}

/// A bounded, inspectable observation kept by one person.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Observation {
    pub tick: u64,
    pub pos: TilePos,
    pub kind: ObservationKind,
    /// 0..1: how strong or visible the cue was.
    pub strength: f32,
    pub evidence: Evidence,
    pub event: Option<EventId>,
    /// `true` for the first local snapshot; later records are material deltas.
    #[serde(default)]
    pub full: bool,
    /// What the person made of it, when their expertise read something into it (#569).
    #[serde(default)]
    pub reading: Option<Reading>,
}

/// A result category used by lessons. It records what happened, not what the person must do.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum OutcomeKind {
    Harm,
    Illness,
    Loss,
    Displacement,
    Death,
    Safety,
    Benefit,
    Discovery,
    NoChange,
}

/// A personal, revisable association between a cue and an outcome.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Lesson {
    pub cue: ObservationKind,
    pub outcome: OutcomeKind,
    pub pos: Option<TilePos>,
    /// 0..1: confidence in this association. It can remain wrong without becoming truth.
    pub confidence: f32,
    pub formed: u64,
    pub last_tested: u64,
    pub confirmations: u32,
    pub contradictions: u32,
    pub origins: Vec<EventId>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Learning {
    pub observations: Vec<Observation>,
    pub lessons: Vec<Lesson>,
}

impl Learning {
    /// Keep the most useful recent evidence without allowing an active mind to grow forever.
    pub fn observe(&mut self, mut observation: Observation) {
        observation.strength = observation.strength.clamp(0.0, 1.0);
        if let Some(previous) = self
            .observations
            .iter_mut()
            .rev()
            .find(|x| x.pos == observation.pos && x.kind == observation.kind)
        {
            let unchanged = (previous.strength - observation.strength).abs() < 0.05
                && previous.evidence == observation.evidence
                && previous.event == observation.event
                && previous.reading == observation.reading;
            if unchanged {
                previous.tick = observation.tick;
                previous.strength = observation.strength;
                return;
            }
            observation.full = false;
        } else {
            observation.full = true;
        }
        if self.observations.len() >= OBSERVATION_CAP {
            let remove = self
                .observations
                .iter()
                .enumerate()
                .min_by_key(|(_, x)| x.tick)
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.observations.remove(remove);
        }
        self.observations.push(observation);
    }

    /// Record a consequence associated with a cue. `confirmed` describes the evidence, not
    /// whether the outcome was desirable: an observed harm confirms that lesson just as an
    /// observed benefit does. Contradiction never silently rewrites the person's history.
    pub fn learn(
        &mut self,
        cue: ObservationKind,
        outcome: OutcomeKind,
        pos: Option<TilePos>,
        confirmed: bool,
        tick: u64,
        event: Option<EventId>,
    ) {
        if let Some(lesson) = self
            .lessons
            .iter_mut()
            .find(|x| x.cue == cue && x.outcome == outcome && x.pos == pos)
        {
            lesson.last_tested = tick;
            if confirmed {
                lesson.confirmations = lesson.confirmations.saturating_add(1);
                lesson.confidence = (lesson.confidence + 0.12).min(1.0);
            } else {
                lesson.contradictions = lesson.contradictions.saturating_add(1);
                lesson.confidence = (lesson.confidence - 0.16).max(0.05);
            }
            if let Some(id) = event {
                add_origin(lesson, id);
            }
            return;
        }

        if self.lessons.len() >= LESSON_CAP {
            let remove = self
                .lessons
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.confidence
                        .partial_cmp(&b.confidence)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then(a.last_tested.cmp(&b.last_tested))
                })
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.lessons.remove(remove);
        }
        let mut lesson = Lesson {
            cue,
            outcome,
            pos,
            confidence: if confirmed { 0.35 } else { 0.2 },
            formed: tick,
            last_tested: tick,
            confirmations: u32::from(confirmed),
            contradictions: u32::from(!confirmed),
            origins: Vec::new(),
        };
        if let Some(id) = event {
            add_origin(&mut lesson, id);
        }
        self.lessons.push(lesson);
    }

    pub fn lesson(
        &self,
        cue: ObservationKind,
        outcome: OutcomeKind,
        pos: Option<TilePos>,
    ) -> Option<&Lesson> {
        self.lessons
            .iter()
            .find(|x| x.cue == cue && x.outcome == outcome && x.pos == pos)
    }
}

fn add_origin(lesson: &mut Lesson, id: EventId) {
    if lesson.origins.last() != Some(&id) && lesson.origins.len() < MAX_ORIGINS {
        lesson.origins.push(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_cues_are_listed() {
        // Hand-kept: the kinds that perception or learning write today. Cold, Fire, Smoke,
        // Spoilage and ResourceDepletion are not written yet and are wired by separate slices;
        // add each here when it is.
        let written = [
            ObservationKind::Heat,
            ObservationKind::WaterDepth,
            ObservationKind::Flooding,
            ObservationKind::Wetness,
            ObservationKind::Saturation,
            ObservationKind::StructuralWeakness,
            ObservationKind::Soil,
            ObservationKind::Tracks,
            ObservationKind::Joint,
        ];
        assert_eq!(RESERVED.len(), 6);
        for k in RESERVED {
            assert!(!written.contains(k), "{k:?} is written but listed reserved");
        }
    }

    fn at(x: i32) -> TilePos {
        TilePos::new(x, 2, 1)
    }

    #[test]
    fn observations_are_personal_bounded_and_clamped() {
        let mut a = Learning::default();
        for tick in 0..(OBSERVATION_CAP as u64 + 2) {
            a.observe(Observation {
                tick,
                pos: at(tick as i32),
                kind: ObservationKind::WaterDepth,
                strength: 2.0,
                evidence: Evidence::Direct,
                event: None,
                full: false,
                reading: None,
            });
        }
        assert_eq!(a.observations.len(), OBSERVATION_CAP);
        assert_eq!(a.observations[0].tick, 2);
        assert_eq!(a.observations[0].strength, 1.0);
    }

    #[test]
    fn unchanged_cues_refresh_in_place_and_changes_are_deltas() {
        let mut a = Learning::default();
        let base = Observation {
            tick: 1,
            pos: at(1),
            kind: ObservationKind::Wetness,
            strength: 0.5,
            evidence: Evidence::Direct,
            event: None,
            full: false,
            reading: None,
        };
        a.observe(base);
        assert!(a.observations[0].full);
        a.observe(Observation { tick: 2, ..base });
        assert_eq!(a.observations.len(), 1);
        assert_eq!(a.observations[0].tick, 2);
        a.observe(Observation {
            tick: 3,
            strength: 0.9,
            ..base
        });
        assert_eq!(a.observations.len(), 2);
        assert!(!a.observations[1].full);
    }

    #[test]
    fn lessons_keep_real_origins_and_can_be_wrong() {
        let mut a = Learning::default();
        a.learn(
            ObservationKind::WaterCurrent,
            OutcomeKind::Displacement,
            Some(at(4)),
            false,
            10,
            Some(EventId(7)),
        );
        a.learn(
            ObservationKind::WaterCurrent,
            OutcomeKind::Displacement,
            Some(at(4)),
            true,
            20,
            Some(EventId(8)),
        );
        let l = a
            .lesson(
                ObservationKind::WaterCurrent,
                OutcomeKind::Displacement,
                Some(at(4)),
            )
            .unwrap();
        assert_eq!(l.contradictions, 1);
        assert_eq!(l.confirmations, 1);
        assert!(l.confidence > 0.2 && l.confidence < 0.5);
        assert_eq!(l.origins, vec![EventId(7), EventId(8)]);
    }

    #[test]
    fn separate_learning_records_do_not_broadcast_knowledge() {
        let mut swimmer = Learning::default();
        let bystander = Learning::default();
        swimmer.learn(
            ObservationKind::WaterCurrent,
            OutcomeKind::Displacement,
            Some(at(1)),
            true,
            5,
            Some(EventId(11)),
        );
        assert!(bystander.lessons.is_empty());
    }
}
