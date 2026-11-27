//! What an agent is committed to right now. One activity at a time; the decision system
//! picks a new one when this completes or is interrupted.

use serde::{Deserialize, Serialize};

use crate::ids::{AgentId, ItemId, StructureId, TilePos};

/// The remaining servings of a single break, including the one being eaten.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct BingePortions {
    pub origin: TilePos,
    pub servings_left: u8,
}

impl BingePortions {
    pub const MAX: u8 = 4;
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub enum Activity {
    #[default]
    Idle,
    /// Walking toward a place, then doing `then`.
    Going {
        to: TilePos,
        then: Box<Activity>,
    },
    /// Eating a carried or ground item.
    Eating {
        item: ItemId,
        ticks_left: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        binge: Option<BingePortions>,
    },
    /// Picking food from a plant at its tile.
    Foraging {
        plant: TilePos,
        ticks_left: u32,
    },
    /// Sleeping; `bed` is None when on the ground.
    Sleeping {
        bed: Option<StructureId>,
        until: u64,
    },
    Talking {
        with: AgentId,
        ticks_left: u32,
    },
    /// Standing about for a while (rest, look around).
    Loitering {
        ticks_left: u32,
    },
    Wandering {
        to: TilePos,
    },
    /// Carrying out a work plan (see `work::Plan`).
    Working {
        plan: crate::work::Plan,
    },
    /// Appended (#433): drinking at water or a well beside them, or from the water store;
    /// then back to `back`, where they were when they went for it, if that was a walk off.
    Drinking {
        ticks_left: u32,
        #[serde(default)]
        back: Option<TilePos>,
    },
}

impl Activity {
    pub fn name(&self) -> &'static str {
        match self {
            Activity::Idle => "idle",
            Activity::Going { .. } => "going",
            Activity::Eating { .. } => "eating",
            Activity::Foraging { .. } => "foraging",
            Activity::Sleeping { .. } => "sleeping",
            Activity::Talking { .. } => "talking",
            Activity::Loitering { .. } => "loitering",
            Activity::Wandering { .. } => "wandering",
            Activity::Working { .. } => "working",
            Activity::Drinking { .. } => "drinking",
        }
    }
    pub fn is_asleep(&self) -> bool {
        matches!(self, Activity::Sleeping { .. })
    }
    /// A short verb for the feed and inspect panel.
    pub fn describe(&self) -> String {
        match self {
            Activity::Idle => "standing about".into(),
            Activity::Going { then, .. } => format!("heading off to {}", then.name()),
            Activity::Eating { .. } => "eating".into(),
            Activity::Foraging { .. } => "gathering food".into(),
            Activity::Sleeping { bed: Some(_), .. } => "asleep in bed".into(),
            Activity::Sleeping { bed: None, .. } => "asleep on the ground".into(),
            Activity::Talking { .. } => "talking".into(),
            Activity::Loitering { .. } => "resting a moment".into(),
            Activity::Wandering { .. } => "wandering".into(),
            Activity::Working { plan } => plan.label.clone(),
            Activity::Drinking { .. } => "drinking".into(),
        }
    }
}
