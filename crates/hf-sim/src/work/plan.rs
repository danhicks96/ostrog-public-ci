//! A plan is a short list of steps toward one designation. Steps are concrete and checked
//! as they run; failure of any step aborts the plan (see `systems::act::work`).

use serde::{Deserialize, Serialize};

use crate::ids::{ItemId, StructureId, TilePos};
use crate::items::ItemKind;

use super::designation::DesignationId;

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Step {
    /// Walk to the tile (or adjacent to it when `adjacent`).
    GoTo { to: TilePos, adjacent: bool },
    /// Pick up `count` from an item stack on the ground (splits the stack).
    PickUp { item: ItemId, count: u32 },
    /// Drop the carried stack of `kind` at the tile (into a stockpile or at a site).
    DropAt {
        to: TilePos,
        kind: ItemKind,
        for_site: Option<DesignationId>,
    },
    /// Spend work ticks on the designation; `station` is the structure used, if any.
    Work {
        designation: DesignationId,
        station: Option<StructureId>,
    },
    /// Fill at the source before carrying water back to the store.
    FillWater { source: TilePos },
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub designation: DesignationId,
    pub steps: Vec<Step>,
    pub cursor: usize,
    /// Human-readable goal for feed/inspect.
    pub label: String,
}

impl Plan {
    pub fn current(&self) -> Option<&Step> {
        self.steps.get(self.cursor)
    }
    pub fn advance(&mut self) {
        self.cursor += 1;
    }
    pub fn finished(&self) -> bool {
        self.cursor >= self.steps.len()
    }
}
