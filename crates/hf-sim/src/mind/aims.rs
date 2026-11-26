//! Pure chances and reason codes kept for personal aims.
//!
//! These helpers do not choose actions or change the simulation.

use serde::{Deserialize, Serialize};

/// Why a slip occurred — recorded so the log shows the cause chain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlipReason {
    /// The person forgot the intention entirely.
    Forgot,
    /// The person remembered but chose not to act (distraction).
    Distracted,
    /// The person was too tired to act on it.
    TooTired,
    /// A higher-priority need interrupted.
    Preempted,
    /// The intention expired before it could be triggered.
    Expired,
    /// The person was injured or incapacitated.
    Incapacitated,
    /// The person deliberately abandoned it (no longer wanted).
    Abandoned,
    /// The intention was superseded by a new one.
    Superseded,
}

impl SlipReason {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Forgot => "forgot",
            Self::Distracted => "distracted",
            Self::TooTired => "too_tired",
            Self::Preempted => "preempted",
            Self::Expired => "expired",
            Self::Incapacitated => "incapacitated",
            Self::Abandoned => "abandoned",
            Self::Superseded => "superseded",
        }
    }
}

/// Calculate the chance of slipping (forgetting) an intention.
///
/// Factors:
/// - fatigue: 1.0 - rest (lower rest = higher fatigue)
/// - stress: worst need pressure (higher = more stress)
/// - distraction: number of active problems (more = more distracted)
/// - disposition: traits that affect attention (e.g., absentminded)
pub fn slip_chance(
    fatigue: f32,
    stress: f32,
    distraction: f32,
    disposition: f32,
    intention_strength: f32,
) -> f32 {
    // Base slip chance: small (1% for a rested, unstressed person)
    let base = 0.01;

    // Fatigue effect: tired people forget more
    let fatigue_factor = 1.0 + fatigue * 2.0;

    // Stress effect: stressed people forget more
    let stress_factor = 1.0 + stress * 1.5;

    // Distraction effect: busy people forget more
    let distraction_factor = 1.0 + distraction * 0.5;

    // Disposition effect: absentminded people forget more
    let disposition_factor = 1.0 + disposition * 1.0;

    // Intention strength: stronger intentions are harder to forget
    let strength_factor = 1.0 - intention_strength * 0.5;

    let chance = base * fatigue_factor * stress_factor * distraction_factor * disposition_factor;

    // Cap at a reasonable maximum (10% for a very bad day), then let a strong intention
    // lower it, so the strength still matters on the worst days.
    chance.min(0.10) * strength_factor
}

/// Calculate the chance of being reminded about an intention.
///
/// Factors:
/// - proximity to trigger: closer to trigger = more likely to be reminded
/// - stimulus strength: stronger stimuli are more likely to trigger memory
/// - intention strength: stronger intentions are more likely to be remembered
pub fn remind_chance(
    proximity_to_trigger: f32,
    stimulus_strength: f32,
    intention_strength: f32,
) -> f32 {
    let base = 0.07;
    let proximity_factor = proximity_to_trigger;
    let stimulus_factor = 1.0 + stimulus_strength * 0.5;
    let strength_factor = 1.0 + intention_strength * 0.3;

    let chance = base * proximity_factor * stimulus_factor * strength_factor;
    chance.min(0.50)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slip_chance_is_bounded() {
        // Rested, unstressed, not distracted, normal disposition, weak intention
        let chance = slip_chance(0.0, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(chance, 0.01);

        // Very tired, stressed, distracted, absentminded, weak intention
        let chance = slip_chance(1.0, 1.0, 1.0, 1.0, 0.0);
        assert!(chance > 0.01);
        assert!(chance <= 0.10);

        // Strong intention is harder to forget
        let chance_strong = slip_chance(1.0, 1.0, 1.0, 1.0, 1.0);
        let chance_weak = slip_chance(1.0, 1.0, 1.0, 1.0, 0.0);
        assert!(chance_strong < chance_weak);
    }

    #[test]
    fn remind_chance_is_bounded() {
        // Low proximity, weak stimulus, weak intention
        let chance = remind_chance(0.1, 0.1, 0.1);
        assert!(chance > 0.0);
        assert!(chance < 0.50);

        // High proximity, strong stimulus, strong intention
        let chance = remind_chance(1.0, 1.0, 1.0);
        assert!(chance > 0.1);
        assert!(chance <= 0.50);
    }
}
