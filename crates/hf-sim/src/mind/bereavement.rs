//! Mourning (OST-215): a loss that stays. Each death of someone close leaves a record on the
//! bereaved; while it is young it weighs on their mood, easing by half every third of its span.

use crate::fmath::Det;
use crate::ids::AgentId;
use crate::time::TICKS_PER_DAY;
use serde::{Deserialize, Serialize};

/// How the dead stood to the bereaved; sets how long the loss is carried.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tie {
    /// A partner or a child: forty days.
    Partner,
    /// A parent: twenty days.
    Parent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bereavement {
    pub who: AgentId,
    pub tie: Tie,
    /// 0.2..1: how close they were.
    pub bond: f32,
    pub since: u64,
}

/// Weight below which a loss is no longer felt and the record is dropped.
pub const FLOOR: f32 = 0.03;

impl Tie {
    pub fn span_days(self) -> f32 {
        match self {
            Tie::Partner => 40.0,
            Tie::Parent => 20.0,
        }
    }
}

impl Bereavement {
    /// The (negative) mood weight at `now`: `-0.45 * bond * 0.5^(age / (span / 3))`.
    pub fn weight(&self, now: u64) -> f32 {
        let age = now.saturating_sub(self.since) as f32 / TICKS_PER_DAY as f32;
        let half = self.tie.span_days() / 3.0;
        -0.45 * self.bond * 0.5_f32.d_powf(age / half)
    }
}

/// The summed weight of every open loss, clamped to -0.8.
pub fn total(list: &[Bereavement], now: u64) -> f32 {
    list.iter().map(|b| b.weight(now)).sum::<f32>().max(-0.8)
}
