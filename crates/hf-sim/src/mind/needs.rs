//! Needs are 0..1 where 1 is fully satisfied. Rates come from docs/TIME.md.

use serde::{Deserialize, Serialize};

use crate::time::TICKS_PER_HOUR;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Need {
    Hunger,
    Rest,
    Warmth,
    Social,
    Comfort,
    /// Appended (#433): water to drink.
    Thirst,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Needs {
    pub hunger: f32,
    pub rest: f32,
    pub warmth: f32,
    pub social: f32,
    pub comfort: f32,
    /// Water (#433): 1 just drunk, 0 parched. Saves from before it have none, and are full.
    #[serde(default = "full")]
    pub thirst: f32,
    /// Ticks spent parched (thirst at zero), on the needs stripe (#433); a day and a half kills.
    #[serde(default)]
    pub parched_ticks: u32,
}

fn full() -> f32 {
    1.0
}

impl Default for Needs {
    fn default() -> Self {
        Needs {
            hunger: 0.8,
            rest: 0.9,
            warmth: 1.0,
            social: 0.7,
            comfort: 0.6,
            thirst: 1.0,
            parched_ticks: 0,
        }
    }
}

/// Hours for a need to go from full to empty under ordinary activity.
pub const HUNGER_HOURS: f32 = 6.0;
pub const REST_HOURS_AWAKE: f32 = 16.0;
pub const REST_HOURS_TO_RECOVER: f32 = 8.0;
pub const SOCIAL_HOURS: f32 = 24.0;
pub const COMFORT_HOURS: f32 = 36.0;
/// Hours from just drunk to parched while awake (#433): faster than hunger, so people drink
/// four or five times a day; asleep a quarter as fast. Heat and hard work hasten it.
pub const THIRST_HOURS: f32 = 4.0;
/// How long parched kills (#433): a day and a half.
pub const PARCHED_DAYS: f32 = 1.5;

impl Needs {
    pub fn get(&self, n: Need) -> f32 {
        match n {
            Need::Hunger => self.hunger,
            Need::Rest => self.rest,
            Need::Warmth => self.warmth,
            Need::Social => self.social,
            Need::Comfort => self.comfort,
            Need::Thirst => self.thirst,
        }
    }

    /// Per-tick decay while awake and active. `hunger_rate` scales with work.
    pub fn decay_awake(&mut self, ticks: u32, hunger_rate: f32) {
        self.burn(ticks, hunger_rate);
        let t = ticks as f32 / TICKS_PER_HOUR as f32;
        self.rest = (self.rest - t / REST_HOURS_AWAKE).max(0.0);
        self.social = (self.social - t / SOCIAL_HOURS).max(0.0);
        self.comfort = (self.comfort - t / COMFORT_HOURS).max(0.0);
    }

    /// Hunger alone, at `hunger_rate` times the waking rate (a grazing animal's, #422).
    pub fn burn(&mut self, ticks: u32, hunger_rate: f32) {
        let t = ticks as f32 / TICKS_PER_HOUR as f32;
        self.hunger = (self.hunger - t / HUNGER_HOURS * hunger_rate).max(0.0);
    }

    /// Thirst over `ticks` (#433): `rate` is 1 at rest in mild weather, more in heat and work;
    /// asleep a quarter of that.
    pub fn dry(&mut self, ticks: u32, rate: f32, asleep: bool) {
        let t = ticks as f32 / TICKS_PER_HOUR as f32;
        let r = if asleep { rate * 0.25 } else { rate };
        self.thirst = (self.thirst - t / THIRST_HOURS * r).max(0.0);
    }

    /// Per-tick change while asleep. Beds recover comfort; the ground does not.
    pub fn sleep(&mut self, ticks: u32, in_bed: bool, hunger_rate: f32) {
        let t = ticks as f32 / TICKS_PER_HOUR as f32;
        self.rest = (self.rest + t / REST_HOURS_TO_RECOVER).min(1.0);
        // Asleep, the body burns a quarter of the waking rate: a night costs a third of a day's food.
        self.hunger = (self.hunger - t / (HUNGER_HOURS * 4.0) * hunger_rate).max(0.0);
        if in_bed {
            self.comfort = (self.comfort + t / 12.0).min(1.0);
        } else {
            self.comfort = (self.comfort - t / 24.0).max(0.0);
        }
    }

    /// Pressure of the most pressing need (0 none, 1 critical).
    pub fn worst(&self) -> (Need, f32) {
        let mut best = (Need::Hunger, 1.0 - self.hunger);
        for (n, v) in [
            (Need::Rest, 1.0 - self.rest),
            (Need::Warmth, 1.0 - self.warmth),
            (Need::Social, 1.0 - self.social),
            (Need::Comfort, 1.0 - self.comfort),
            (Need::Thirst, 1.0 - self.thirst),
        ] {
            if v > best.1 {
                best = (n, v);
            }
        }
        best
    }

    pub fn starving(&self) -> bool {
        self.hunger <= 0.02
    }
    pub fn parched(&self) -> bool {
        self.thirst <= 0.02
    }
    pub fn exhausted(&self) -> bool {
        self.rest <= 0.02
    }
}
