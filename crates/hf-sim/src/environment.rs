//! Read-only environmental facts available at a proposed worksite.

use crate::ids::TilePos;
use crate::World;

/// Authoritative environmental facts around a proposed mine or dig location.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorksiteClues {
    /// Whether the rock at the worksite is a water-bearing seam.
    pub water_bearing: bool,
    /// Whether standing water is close enough to make the worksite damp.
    pub damp: bool,
    /// Number of connected excavation tiles that would be unsupported if opened.
    pub unsupported_tiles: usize,
}

/// Read the simulation-backed environmental facts at `at` without changing the world.
///
/// Keep this as a projection of the same queries the hazards use, not a second estimate that
/// could disagree with the actual outcome (FIXLOG 2026-10-29, E16/#479).
pub fn worksite_clues(w: &World, at: TilePos) -> WorksiteClues {
    WorksiteClues {
        water_bearing: crate::aquifer::wet(w, at),
        damp: crate::aquifer::damp(w, at),
        unsupported_tiles: crate::underground::unheld_if_opened(w, at).len(),
    }
}
