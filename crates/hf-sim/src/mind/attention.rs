//! What a person notices (#569): attention is limited, and what reaches the observation ledger
//! depends on skill, the task in hand, mood and the stimulus. A farmer reads the ground, one who
//! knows animals reads their sign, a builder reads why a site is stuck. Below the threshold a
//! stimulus was there but not noticed: nothing enters the ledger (the Why? lens can show it for
//! the traced person, #567). A cue noticed before and unchanged fades from notice
//! (habituation) and returns when it changes.
//!
//! This only decides noticing; `systems::perceive` finds the stimuli, bounded and only for those
//! who could notice them.

use crate::agents::Agent;
use crate::mind::feelings::FeelingKind;
use crate::mind::Skill;

/// The salience a stimulus must reach to be noticed.
pub const NOTICE: f32 = 0.5;
/// Habituated: an unchanged cue noticed recently weighs this much.
pub const HABITUATED: f32 = 0.25;

/// The kinds of reading expertise adds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Domain {
    /// The ground: whether crops would do well on it (farming).
    Ground,
    /// Animal sign: what passed near, unseen (animals).
    Sign,
    /// Building: why a site is stuck (construction).
    Building,
}

impl Domain {
    pub fn skill(self) -> Skill {
        match self {
            Domain::Ground => Skill::Farming,
            Domain::Sign => Skill::Animals,
            Domain::Building => Skill::Construction,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Domain::Ground => "the ground",
            Domain::Sign => "animal sign",
            Domain::Building => "a stuck building",
        }
    }
}

/// How strongly this person attends to a domain now, before the stimulus: skill (0 to 20 maps
/// to 0.35..1.25), the task in hand, stress, tiredness and grief. A stimulus of strength 1 is
/// noticed when this reaches `NOTICE`.
pub fn attention(a: &Agent, d: Domain, on_task: bool) -> f32 {
    let skill = (a.skills.level(d.skill()) / 20.0).clamp(0.0, 1.0);
    let fatigue = 1.0 - a.needs.rest.clamp(0.0, 1.0);
    let grief = a.feelings.of(FeelingKind::Grief).clamp(0.0, 1.0);
    let task = if on_task { 1.3 } else { 1.0 };
    (0.35 + 0.9 * skill)
        * task
        * (1.0 - 0.35 * a.stress.clamp(0.0, 1.0))
        * (1.0 - 0.25 * fatigue)
        * (1.0 - 0.5 * grief)
}

/// The salience of one stimulus to this person: their attention times its strength, faded when
/// habituated.
pub fn salience(attention: f32, strength: f32, habituated: bool) -> f32 {
    attention * strength.clamp(0.0, 1.0) * if habituated { HABITUATED } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn habituation_hides_an_unchanged_cue_from_an_expert() {
        assert!(salience(1.0, 0.8, false) >= NOTICE);
        assert!(salience(1.0, 0.8, true) < NOTICE);
        assert!(salience(0.35, 0.8, false) < NOTICE, "a novice misses it");
    }
}
