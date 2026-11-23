//! OST-1304: why most houses never get a table. A read-only census of every house site, applying
//! the same tests as step 6b of `systems::settlement` (complete walls, a `layout().table` spot,
//! fewer than 4 open build orders, fewer than 2 floor orders posted in the pass, the slot free)
//! and naming the first one that stops the table. It changes nothing and draws no randomness.

use crate::houses;
use crate::ids::TilePos;
use crate::items::ItemKind;
use crate::structures::StructureKind;
use crate::work::DesignationKind;
use crate::world::{Material, Shape};
use crate::world_state::World;

/// The first reason a house has no table, in the order step 6b tests them.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Cause {
    /// Walls not closed yet, so 6b skips the house.
    WallsOpen,
    /// `layout().table` is empty: no spot for a table.
    NoTableSpot,
    /// A table already stands or is ordered.
    HasTable,
    /// Global open floor/structure orders are 4 or more, so 6b skips every house.
    OpenBuildGate,
    /// Two or more floor tiles still wanting an order: `posted < 2` blocks the table.
    FloorBlocks,
    /// Nothing stops the order, and no planks exist in the world.
    NoPlanks,
    /// Nothing stops the order and planks exist: it should be posted next pass.
    ReadyToPost,
}

#[derive(Debug)]
pub struct HouseRow {
    pub community: u32,
    pub origin: TilePos,
    pub table_spots: usize,
    pub floor_wanting: usize,
    pub cause: Cause,
}

pub struct Census {
    pub rows: Vec<HouseRow>,
    pub hollows: usize,
    pub planks: u32,
    pub open_build: usize,
}

impl Census {
    pub fn count(&self, c: Cause) -> usize {
        self.rows.iter().filter(|r| r.cause == c).count()
    }
}

pub fn census(w: &World) -> Census {
    let planks: u32 = w
        .items
        .iter()
        .filter(|i| i.kind == ItemKind::Plank)
        .map(|i| i.count)
        .sum();
    let open_build = w
        .designations
        .iter()
        .filter(|d| {
            matches!(
                d.kind,
                DesignationKind::BuildFloor(_) | DesignationKind::BuildStructure(..)
            )
        })
        .count();
    let mut rows = Vec::new();
    let mut hollows = 0;
    for c in &w.communities {
        hollows += c.hollows.len();
        let gap = houses::bed_gap(w, c.id);
        for origin in c.house_sites.iter().copied() {
            let plan = houses::plan(w, origin);
            let spots = houses::layout(&plan, gap).table;
            let floor_wanting = plan
                .floor
                .iter()
                .filter(|p| {
                    let below = p.offset(0, 0, -1);
                    w.grid.get(below).material != Material::WOOD
                        && w.grid.get(below).shape == Shape::Wall
                        && !w.designations.iter().any(|d| d.pos == **p)
                        && !w.structures.iter().any(|s| s.pos == **p)
                })
                .count();
            let has_table = spots.first().is_some_and(|p| {
                w.structures
                    .iter()
                    .any(|s| s.pos == *p && s.kind == StructureKind::Table)
                    || w.designations.iter().any(|d| d.pos == *p)
            });
            let cause = if !houses::complete(w, &plan) {
                Cause::WallsOpen
            } else if spots.is_empty() {
                Cause::NoTableSpot
            } else if has_table {
                Cause::HasTable
            } else if open_build >= 4 {
                Cause::OpenBuildGate
            } else if floor_wanting >= 2 {
                Cause::FloorBlocks
            } else if planks == 0 {
                Cause::NoPlanks
            } else {
                Cause::ReadyToPost
            };
            rows.push(HouseRow {
                community: c.id.0,
                origin,
                table_spots: spots.len(),
                floor_wanting,
                cause,
            });
        }
    }
    Census {
        rows,
        hollows,
        planks,
        open_build,
    }
}
