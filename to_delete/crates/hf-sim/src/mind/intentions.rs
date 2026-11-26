//! Deferred intentions and their lifecycle (#570: mistakes and forgetting).
//!
//! A person can decide to do something later — "I'll lay stones when I pass the mud next"
//! or "I'll ask about the stranger once the harvest is in". These are intentions, not
//! immediate actions, and they need a record so they are traceable through the debug log.
//!
//! Each intention has a lifecycle:
//!   created -> deferred -> reminded -> triggered -> acted_on | abandoned | expired | superseded
//!
//! Every transition is recorded with a reason code, so a viewer can see why an intention
//! was dropped or completed.
//!
//! The system is debug-only when tracing is on; it does not change the simulation in any
//! other mode. Slip and forget rates depend on fatigue, stress, distraction, and disposition,
//! but are small by default and bounded so they cannot wreck productivity.

use crate::ids::{AgentId, EventId};
use serde::{Deserialize, Serialize};

/// What the person intends to do later. Kept small and concrete so the log is readable.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntentionKind {
    /// I will do X when Y happens.
    Conditional { action: String, trigger: String },
    /// I will do X after Z time passes.
    Temporal { action: String, duration_ticks: u64 },
    /// I will do X regardless, when I next have a chance.
    Opportunistic { action: String },
    /// I will do X to help person Y.
    Prosocial { action: String, target: AgentId },
    /// I will investigate/report what I saw.
    Investigate { stimulus: String },
    /// Other / fallback.
    Other { action: String },
}

impl IntentionKind {
    pub fn action_label(&self) -> &str {
        match self {
            Self::Conditional { action, .. } => action,
            Self::Temporal { action, .. } => action,
            Self::Opportunistic { action } => action,
            Self::Prosocial { action, .. } => action,
            Self::Investigate { stimulus } => stimulus,
            Self::Other { action } => action,
        }
    }
}

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

/// The current state of an intention.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentionState {
    /// Created but not yet deferred — the person just decided to do something later.
    Created,
    /// Deferred: waiting for a trigger or time.
    Deferred,
    /// Reminded: something brought it back to mind.
    Reminded,
    /// Triggered: the conditions are met, the person is about to act.
    Triggered,
    /// Acted on: the person completed the intention.
    ActedOn,
    /// Abandoned: the person chose not to act.
    Abandoned,
    /// Expired: the time passed or condition never met.
    Expired,
    /// Superseded: a newer intention replaced this one.
    Superseded,
}

impl IntentionState {
    pub fn label(&self) -> &str {
        match self {
            Self::Created => "created",
            Self::Deferred => "deferred",
            Self::Reminded => "reminded",
            Self::Triggered => "triggered",
            Self::ActedOn => "acted_on",
            Self::Abandoned => "abandoned",
            Self::Expired => "expired",
            Self::Superseded => "superseded",
        }
    }
}

/// A deferred intention with its full lifecycle.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Intention {
    /// Unique id for this intention.
    pub id: u64,
    /// Who holds this intention.
    pub agent: AgentId,
    /// What they intend to do.
    pub kind: IntentionKind,
    /// Current state.
    pub state: IntentionState,
    /// When the intention was created.
    pub created_at: u64,
    /// When the intention was last updated.
    pub last_updated: u64,
    /// The trigger or deadline (if temporal).
    pub deadline: Option<u64>,
    /// How many times this intention has been reminded.
    pub remind_count: u32,
    /// The cause chain — where this intention came from (event id or "internal").
    pub cause: Option<EventId>,
    /// The reason it was dropped (if terminal state).
    pub drop_reason: Option<SlipReason>,
}

impl Intention {
    /// Whether this intention is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            IntentionState::ActedOn
                | IntentionState::Abandoned
                | IntentionState::Expired
                | IntentionState::Superseded
        )
    }
}

/// The collection of intentions for one agent.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Intentions {
    /// All intentions held by this agent.
    pub records: Vec<Intention>,
    /// Next id to assign.
    pub next_id: u64,
}

impl Intentions {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
        }
    }

    /// Create a new intention.
    pub fn create(
        &mut self,
        agent: AgentId,
        kind: IntentionKind,
        deadline: Option<u64>,
        cause: Option<EventId>,
        tick: u64,
    ) -> IntentionId {
        let id = self.next_id;
        self.next_id = id + 1;
        self.records.push(Intention {
            id,
            agent,
            kind,
            state: IntentionState::Created,
            created_at: tick,
            last_updated: tick,
            deadline,
            remind_count: 0,
            cause,
            drop_reason: None,
        });
        IntentionId(id)
    }

    /// Transition the intention to a new state.
    pub fn transition(
        &mut self,
        id: IntentionId,
        new_state: IntentionState,
        tick: u64,
        reason: Option<SlipReason>,
    ) -> bool {
        let Some(intention) = self.records.iter_mut().find(|i| i.id == id.0) else {
            return false;
        };
        if intention.is_terminal() {
            return false;
        }
        intention.state = new_state;
        intention.last_updated = tick;
        if let Some(reason) = reason {
            intention.drop_reason = Some(reason);
        }
        true
    }

    /// Forget this intention (slip). Records the reason.
    pub fn forget(&mut self, id: IntentionId, tick: u64) -> bool {
        self.transition(
            id,
            IntentionState::Abandoned,
            tick,
            Some(SlipReason::Forgot),
        )
    }

    /// Remember this intention (reminder).
    pub fn remember(&mut self, id: IntentionId, tick: u64) -> bool {
        if self
            .records
            .iter()
            .any(|i| i.id == id.0 && i.state == IntentionState::Deferred)
        {
            self.transition(id, IntentionState::Reminded, tick, None);
            if let Some(i) = self.records.iter_mut().find(|i| i.id == id.0) {
                i.remind_count += 1;
            }
            true
        } else {
            false
        }
    }

    /// Trigger this intention (conditions met).
    pub fn trigger(&mut self, id: IntentionId, tick: u64) -> bool {
        self.transition(id, IntentionState::Triggered, tick, None)
    }

    /// Mark this intention as acted on.
    pub fn act(&mut self, id: IntentionId, tick: u64) -> bool {
        self.transition(id, IntentionState::ActedOn, tick, None)
    }

    /// Mark this intention as abandoned (deliberate).
    pub fn abandon(&mut self, id: IntentionId, tick: u64) -> bool {
        self.transition(
            id,
            IntentionState::Abandoned,
            tick,
            Some(SlipReason::Abandoned),
        )
    }

    /// Mark this intention as expired.
    pub fn expire(&mut self, id: IntentionId, tick: u64) -> bool {
        self.transition(id, IntentionState::Expired, tick, Some(SlipReason::Expired))
    }

    /// Supersede this intention with a new one.
    pub fn supersede(&mut self, id: IntentionId, new_id: IntentionId, tick: u64) -> bool {
        self.transition(
            id,
            IntentionState::Superseded,
            tick,
            Some(SlipReason::Superseded),
        );
        if let Some(i) = self.records.iter_mut().find(|i| i.id == new_id.0) {
            i.state = IntentionState::Created;
            i.last_updated = tick;
        }
        true
    }

    /// Check if any intentions have expired (deadline passed).
    pub fn check_expired(&mut self, tick: u64) -> Vec<IntentionId> {
        let mut expired = Vec::new();
        for intention in &self.records {
            if !intention.is_terminal()
                && intention.state == IntentionState::Deferred
                && intention.deadline.is_some_and(|d| d < tick)
            {
                expired.push(IntentionId(intention.id));
            }
        }
        for id in &expired {
            self.expire(*id, tick);
        }
        expired
    }

    /// Check if any intentions should be triggered (conditions met).
    pub fn check_triggered(&mut self, tick: u64) -> Vec<IntentionId> {
        let mut triggered = Vec::new();
        for intention in &self.records {
            if !intention.is_terminal()
                && intention.state == IntentionState::Deferred
                && intention.deadline.is_some_and(|d| d <= tick)
            {
                triggered.push(IntentionId(intention.id));
            }
        }
        for id in &triggered {
            self.trigger(*id, tick);
        }
        triggered
    }
}

/// Unique id for an intention.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IntentionId(pub u64);

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
    use crate::ids::AgentId;

    #[test]
    fn test_create_intention() {
        let mut intentions = Intentions::new();
        let agent = AgentId(1);
        let kind = IntentionKind::Temporal {
            action: "rest".to_string(),
            duration_ticks: 100,
        };
        let id = intentions.create(agent, kind, None, None, 1);
        assert_eq!(id.0, 1);
        assert_eq!(intentions.records.len(), 1);
        assert_eq!(intentions.records[0].state, IntentionState::Created);
    }

    #[test]
    fn test_transition_states() {
        let mut intentions = Intentions::new();
        let agent = AgentId(1);
        let kind = IntentionKind::Temporal {
            action: "rest".to_string(),
            duration_ticks: 100,
        };
        let id = intentions.create(agent, kind, None, None, 1);

        // Transition to deferred
        assert!(intentions.transition(id, IntentionState::Deferred, 1, None));
        assert_eq!(intentions.records[0].state, IntentionState::Deferred);

        // Transition to reminded
        assert!(intentions.transition(id, IntentionState::Reminded, 2, None));
        assert_eq!(intentions.records[0].state, IntentionState::Reminded);

        // Transition to triggered
        assert!(intentions.transition(id, IntentionState::Triggered, 3, None));
        assert_eq!(intentions.records[0].state, IntentionState::Triggered);

        // Transition to acted_on
        assert!(intentions.transition(id, IntentionState::ActedOn, 4, None));
        assert_eq!(intentions.records[0].state, IntentionState::ActedOn);
    }

    #[test]
    fn test_forget_intention() {
        let mut intentions = Intentions::new();
        let agent = AgentId(1);
        let kind = IntentionKind::Temporal {
            action: "rest".to_string(),
            duration_ticks: 100,
        };
        let id = intentions.create(agent, kind, None, None, 1);

        intentions.transition(id, IntentionState::Deferred, 1, None);
        assert!(intentions.forget(id, 2));
        assert_eq!(intentions.records[0].state, IntentionState::Abandoned);
        assert_eq!(intentions.records[0].drop_reason, Some(SlipReason::Forgot));
    }

    #[test]
    fn test_slip_chance() {
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
    fn test_remind_chance() {
        // Low proximity, weak stimulus, weak intention
        let chance = remind_chance(0.1, 0.1, 0.1);
        assert!(chance > 0.0);
        assert!(chance < 0.50);

        // High proximity, strong stimulus, strong intention
        let chance = remind_chance(1.0, 1.0, 1.0);
        assert!(chance > 0.1);
        assert!(chance <= 0.50);
    }

    #[test]
    fn test_check_expired() {
        let mut intentions = Intentions::new();
        let agent = AgentId(1);
        let kind = IntentionKind::Temporal {
            action: "rest".to_string(),
            duration_ticks: 10,
        };
        let id = intentions.create(agent, kind, Some(5), None, 1);

        intentions.transition(id, IntentionState::Deferred, 1, None);

        // Not expired yet
        let expired = intentions.check_expired(3);
        assert!(expired.is_empty());

        // Now expired
        let expired = intentions.check_expired(6);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0], id);
        assert_eq!(intentions.records[0].state, IntentionState::Expired);
    }

    #[test]
    fn test_is_terminal() {
        let mut intentions = Intentions::new();
        let agent = AgentId(1);
        let kind = IntentionKind::Temporal {
            action: "rest".to_string(),
            duration_ticks: 100,
        };
        let id = intentions.create(agent, kind, None, None, 1);

        // Not terminal
        assert!(!intentions.records[0].is_terminal());

        // Acted on
        intentions.transition(id, IntentionState::ActedOn, 1, None);
        assert!(intentions.records[0].is_terminal());
    }
}
