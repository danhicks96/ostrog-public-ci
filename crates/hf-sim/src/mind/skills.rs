//! Learned capability: level 0..20 with experience; separate from the genome by design.
use crate::fmath::Det;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Skill {
    Foraging,
    Farming,
    Cooking,
    Construction,
    Mining,
    Crafting,
    Social,
    Hauling,
    Animals,
}

pub const ALL: [Skill; 9] = [
    Skill::Foraging,
    Skill::Farming,
    Skill::Cooking,
    Skill::Construction,
    Skill::Mining,
    Skill::Crafting,
    Skill::Social,
    Skill::Hauling,
    Skill::Animals,
];

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct SkillState {
    pub level: f32,
    pub xp: f32,
    /// Innate aptitude 0.5..1.5 (from the genome later; random in Phase 3).
    pub aptitude: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Skills {
    pub states: [SkillState; 9],
}

impl Default for Skills {
    fn default() -> Self {
        Skills {
            states: [SkillState {
                level: 0.0,
                xp: 0.0,
                aptitude: 1.0,
            }; 9],
        }
    }
}

impl Skills {
    pub fn get(&self, s: Skill) -> &SkillState {
        &self.states[s as usize]
    }
    pub fn level(&self, s: Skill) -> f32 {
        self.states[s as usize].level
    }
    /// Work speed multiplier: 0.6 at level 0 to 1.8 at level 20.
    pub fn speed(&self, s: Skill) -> f32 {
        0.6 + self.level(s) / 20.0 * 1.2
    }
    /// Practise: xp grows with aptitude, levels follow a soft curve.
    pub fn practise(&mut self, s: Skill, hours: f32) {
        let st = &mut self.states[s as usize];
        st.xp += hours * st.aptitude;
        // level = 4 * sqrt(xp) capped at 20: 25 h -> 20? No: 4*5=20 at 25 h is too fast,
        // so use 20 * (1 - exp(-xp/120)): ~half way after 83 hours of practice.
        st.level = 20.0 * (1.0 - (-st.xp / 120.0).d_exp());
    }
}
