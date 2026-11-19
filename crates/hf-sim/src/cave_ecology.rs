//! Habitat-backed fauna roles for natural caverns (M6, #493).
//!
//! The cavern itself is the source of truth for which underground animals belong there.  This
//! keeps placement deterministic and gives later observation/inspection work one simulation-backed
//! place to ask what a cavern can contain without turning the roster into a global fact for people.

use crate::world::gen::caverns::Cavern;

/// The fauna a natural cavern can support at world generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CaveFaunaProfile {
    /// Number of bats that roost in this cavern. Pools and deep caverns sustain larger roosts.
    pub bats: u8,
    /// Whether a pale crawler belongs to this cavern's deep habitat.
    pub crawler: bool,
    /// Whether this cavern is the preferred burrower habitat.
    pub burrower: bool,
}

/// Derive a cavern's fauna from its physical habitat, not from the order of the agent list.
pub fn profile(cavern: &Cavern) -> CaveFaunaProfile {
    CaveFaunaProfile {
        bats: 4 + u8::from(cavern.pool) * 2 + u8::from(cavern.deep),
        crawler: cavern.deep,
        burrower: cavern.underworld,
    }
}

/// Pick the one cavern that should host the burrower.
///
/// An underworld cavern is preferred because its exposed stone is the creature's natural deep
/// habitat.  If this world has no underworld cavern, the lowest deep cavern remains the fallback.
pub fn burrower_cavern(caverns: &[Cavern]) -> Option<&Cavern> {
    caverns
        .iter()
        .filter(|c| c.deep)
        // The habitat profile, not a second raw-flag rule, owns this preference
        // (FIXLOG 2026-10-30, PR #495 review).
        .min_by_key(|c| (!profile(c).burrower, c.centre.z, c.centre.x, c.centre.y))
}

/// Pick the cavern for the valley's cave slime (#423): a wet one of the deep, else any wet one.
/// None in a valley with no pool underground: a soft body dries out of the damp.
pub fn slime_cavern(caverns: &[Cavern]) -> Option<&Cavern> {
    caverns
        .iter()
        .filter(|c| c.pool)
        .min_by_key(|c| (!c.deep, c.centre.z, c.centre.x, c.centre.y))
}

/// Pick the cavern for the valley's cave centipede (#423): the shallowest one, where the bats
/// it hunts roost nearest the surface.
pub fn centipede_cavern(caverns: &[Cavern]) -> Option<&Cavern> {
    caverns
        .iter()
        .filter(|c| !c.deep)
        .min_by_key(|c| (-c.centre.z, c.centre.x, c.centre.y))
}

/// Pick the deepest cavern that supports the valley's one pale crawler.
pub fn crawler_cavern(caverns: &[Cavern]) -> Option<&Cavern> {
    caverns
        .iter()
        .filter(|c| profile(c).crawler)
        .min_by_key(|c| (c.centre.z, c.centre.x, c.centre.y))
}

/// Pick the cavern for the valley's blind cave bear (#336): a deep cavern that is not the pale
/// crawler's own — the crawler keeps the deepest, the bear takes the shallowest other deep
/// cavern. None in a valley whose only deep cavern is the crawler's: the control case.
pub fn blind_bear_cavern(caverns: &[Cavern]) -> Option<&Cavern> {
    let crawler = crawler_cavern(caverns).map(|c| (c.centre.x, c.centre.y, c.centre.z));
    caverns
        .iter()
        .filter(|c| c.deep)
        .filter(|c| Some((c.centre.x, c.centre.y, c.centre.z)) != crawler)
        .max_by_key(|c| (c.centre.z, c.centre.x, c.centre.y))
}

/// Pick the cavern for the valley's cave crocodile (#336): the shallowest pool cavern, where
/// something coming down to drink steps within reach. None in a valley with no pool underground.
pub fn croc_cavern(caverns: &[Cavern]) -> Option<&Cavern> {
    caverns
        .iter()
        .filter(|c| c.pool)
        .max_by_key(|c| (c.centre.z, c.centre.x, c.centre.y))
}
