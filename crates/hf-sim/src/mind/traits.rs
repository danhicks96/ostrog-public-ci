//! Legacy binary traits and their concrete effects. New people also have the continuous
//! core profile in `dispositions.rs`; old saves with no profile keep using these weights.

use rand::seq::SliceRandom;
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Trait {
    Industrious,  // work goals ×1.3, idle ×0.6
    Easygoing,    // work ×0.8, comfort/social ×1.2
    Sociable,     // social need decays faster, talk ×1.5
    Solitary,     // social decays slower, talk ×0.6
    Curious,      // explore/wander ×1.5, discovery memories stronger
    Cautious,     // avoids danger places more, forage radius smaller
    Hearty,       // hunger decays slower
    Gourmet,      // meal quality matters more
    LightSleeper, // sleeps less, wakes earlier
    Tidy,         // haul ×1.4
    /// Appended (#322): brought back from the deep; never drawn at birth (not in `ALL`).
    FearsTheDark, // afraid whenever in the dark below
}

impl Trait {
    pub fn label(self) -> &'static str {
        match self {
            Trait::Industrious => "industrious",
            Trait::Easygoing => "easygoing",
            Trait::Sociable => "sociable",
            Trait::Solitary => "solitary",
            Trait::Curious => "curious",
            Trait::Cautious => "cautious",
            Trait::Hearty => "hearty",
            Trait::Gourmet => "gourmet",
            Trait::LightSleeper => "light sleeper",
            Trait::Tidy => "tidy",
            Trait::FearsTheDark => "fears the dark",
        }
    }
}

pub const ALL: [Trait; 10] = [
    Trait::Industrious,
    Trait::Easygoing,
    Trait::Sociable,
    Trait::Solitary,
    Trait::Curious,
    Trait::Cautious,
    Trait::Hearty,
    Trait::Gourmet,
    Trait::LightSleeper,
    Trait::Tidy,
];

/// Pairs that cannot both be held.
const EXCLUSIVE: [(Trait, Trait); 2] = [
    (Trait::Industrious, Trait::Easygoing),
    (Trait::Sociable, Trait::Solitary),
];

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Traits {
    pub list: Vec<Trait>,
}

impl Traits {
    pub fn random(r: &mut ChaCha8Rng) -> Traits {
        let n = r.gen_range(1..=3);
        let mut list: Vec<Trait> = Vec::new();
        let mut pool = ALL.to_vec();
        pool.shuffle(r);
        for t in pool {
            if list.len() >= n {
                break;
            }
            let clash = EXCLUSIVE
                .iter()
                .any(|(a, b)| (*a == t && list.contains(b)) || (*b == t && list.contains(a)));
            if !clash {
                list.push(t);
            }
        }
        list.sort();
        Traits { list }
    }

    pub fn has(&self, t: Trait) -> bool {
        self.list.contains(&t)
    }

    pub fn hunger_rate(&self) -> f32 {
        if self.has(Trait::Hearty) {
            0.8
        } else {
            1.0
        }
    }
    pub fn social_rate(&self) -> f32 {
        if self.has(Trait::Sociable) {
            1.5
        } else if self.has(Trait::Solitary) {
            0.6
        } else {
            1.0
        }
    }
    pub fn work_weight(&self) -> f32 {
        // Fix-site (FIXLOG 2026-11-16 #908): widened alongside the core profile's range.
        if self.has(Trait::Industrious) {
            1.6
        } else if self.has(Trait::Easygoing) {
            0.6
        } else {
            1.0
        }
    }
    pub fn talk_weight(&self) -> f32 {
        if self.has(Trait::Sociable) {
            1.5
        } else if self.has(Trait::Solitary) {
            0.6
        } else {
            1.0
        }
    }
    pub fn wander_weight(&self) -> f32 {
        if self.has(Trait::Curious) {
            1.5
        } else {
            1.0
        }
    }
    pub fn loiter_weight(&self) -> f32 {
        if self.has(Trait::Industrious) {
            0.6
        } else if self.has(Trait::Easygoing) {
            1.2
        } else {
            1.0
        }
    }
    pub fn sleep_hours(&self) -> f32 {
        if self.has(Trait::LightSleeper) {
            6.5
        } else {
            8.0
        }
    }
}
