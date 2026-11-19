//! Deterministic, reproducible RNG. Ported from an earlier prototype.
//!
//! Every stream is hash-derived from `(world_seed, parts...)` so the same purpose at the
//! same coordinates reconstructs identically, and per-entity streams do not correlate the
//! way naive seed-splitting would. Purpose tags are `u64` constants in [`purpose`].

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

/// splitmix64-style avalanche mix of a slice of u64 "coordinates".
pub fn hash64(parts: &[u64]) -> u64 {
    let mut h: u64 = 0x9e37_79b9_7f4a_7c15;
    for &p in parts {
        h ^= p.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = h;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        h = z ^ (z >> 31);
    }
    h
}

/// Derive an independent ChaCha8 stream keyed by the world seed plus coordinates
/// (purpose tag, entity id, tick, ...).
pub fn stream(seed: u64, parts: &[u64]) -> ChaCha8Rng {
    let mut buf = Vec::with_capacity(parts.len() + 1);
    buf.push(seed);
    buf.extend_from_slice(parts);
    ChaCha8Rng::seed_from_u64(hash64(&buf))
}

/// Purpose tags. Each distinct random decision in the simulation gets its own tag so that
/// adding a new random draw somewhere never shifts the draws of unrelated systems.
///
/// Every new tag MUST be added to [`ALL`]; the uniqueness test below fails the build if two
/// tags ever share a value again (#780).
pub mod purpose {
    /// Every purpose tag, for the uniqueness guard. Keep in the same order as below.
    pub const ALL: &[u64] = &[
        WORLDGEN_HEIGHT,
        WORLDGEN_STRATA,
        WORLDGEN_CAVES,
        WORLDGEN_ORE,
        WORLDGEN_WATER,
        WORLDGEN_FLORA,
        WORLDGEN_FOUNDERS,
        WORLDGEN_CLIMATE,
        WORLDGEN_SURFACE,
        WORLDGEN_SCREE,
        WEATHER,
        PLANTS,
        AGENT_BIRTH,
        AGENT_DECISION,
        AGENT_PERCEPTION,
        AGENT_SOCIAL,
        AGENT_WORK,
        AGENT_SLEEP,
        ANIMALS,
        NAMES,
        OPENING,
        ENGINE,
        PAST,
        THREATS,
        DEITY,
        BUILDING,
        LEGEND,
        HISTORY,
        LORE,
        THIN,
        REALM_MAP,
        REALM_PEOPLES,
        REALM_HISTORY,
        DRINK_FLUX,
        WORLDGEN_FLORA_DRAW,
        CASCADE,
        ACCOUNTS,
        ECOLOGY,
        RESONANCE,
    ];
    pub const WORLDGEN_HEIGHT: u64 = 1;
    pub const WORLDGEN_STRATA: u64 = 2;
    pub const WORLDGEN_CAVES: u64 = 3;
    pub const WORLDGEN_ORE: u64 = 4;
    pub const WORLDGEN_WATER: u64 = 5;
    pub const WORLDGEN_FLORA: u64 = 6;
    pub const WORLDGEN_FOUNDERS: u64 = 7;
    /// The world's character (wet/dry, warm/cold, stony/soft) and its climate fields.
    pub const WORLDGEN_CLIMATE: u64 = 8;
    /// Surface ground patches within a biome.
    pub const WORLDGEN_SURFACE: u64 = 9;
    /// #289 (DEC-8 item 4): the loose-rock pass on steep bare ground. Was 56 until the
    /// collision with CASCADE was found (#780); 61 was free. Renumbering shifts the scree
    /// layout at worldgen — trajectory evidence rides #781's four-seed soak.
    pub const WORLDGEN_SCREE: u64 = 61;
    pub const WEATHER: u64 = 10;
    pub const PLANTS: u64 = 11;
    pub const AGENT_BIRTH: u64 = 20;
    pub const AGENT_DECISION: u64 = 21;
    pub const AGENT_PERCEPTION: u64 = 22;
    pub const AGENT_SOCIAL: u64 = 23;
    pub const AGENT_WORK: u64 = 24;
    pub const AGENT_SLEEP: u64 = 25;
    pub const ANIMALS: u64 = 30;
    pub const NAMES: u64 = 40;
    /// The Engine's daily roll (pacing, never events).
    pub const ENGINE: u64 = 42;
    /// The arrival: the ten survivors and where the missing seven ended up. Each slot's identity
    /// is keyed by (index, rerolls) so the setup screen's rerolls stay deterministic.
    pub const OPENING: u64 = 41;
    /// Lived pasts: households, the home village's history, each survivor's life (past.rs).
    pub const PAST: u64 = 43;
    /// Outside pressure: site stores, bands of outsiders, beasts that hunt people (threat.rs).
    pub const THREATS: u64 = 44;
    pub const DEITY: u64 = 45;
    pub const BUILDING: u64 = 46;
    pub const LEGEND: u64 = 47;
    pub const HISTORY: u64 = 48;
    pub const LORE: u64 = 49;
    pub const THIN: u64 = 50;
    /// The region map of the world the valley is part of (realm/map.rs).
    pub const REALM_MAP: u64 = 51;
    /// Peoples, their languages and convictions (realm/peoples.rs, realm/language.rs).
    pub const REALM_PEOPLES: u64 = 52;
    /// The world's years: growth, troubles, new settlements (realm/sites.rs).
    pub const REALM_HISTORY: u64 = 53;
    /// A drink from a fouled tile bringing the flux (water.rs `drink`, #453 part 3).
    pub const DRINK_FLUX: u64 = 54;
    /// The limited wild-flora community drawn for this world (#486).
    pub const WORLDGEN_FLORA_DRAW: u64 = 55;
    /// The vermin's bite in the flood cascade (cascade.rs, #319).
    pub const CASCADE: u64 = 56;
    /// Accounts of the colony's dead told in talk (accounts.rs, #452).
    pub const ACCOUNTS: u64 = 57;
    /// The valley's food web: starvation, clearings and kinds coming back (ecology.rs, #486).
    pub const ECOLOGY: u64 = 58;
    /// Hidden arrival changes drawn for the ten (resonance.rs, #308).
    pub const RESONANCE: u64 = 59;
    /// The one bounded fragment woken at the stone (mind/fragment.rs, #508 7.6).
    pub const RESONANCE_FRAGMENT: u64 = 60;
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn same_coordinates_same_stream() {
        let a: u64 = stream(7, &[1, 2, 3]).gen();
        let b: u64 = stream(7, &[1, 2, 3]).gen();
        assert_eq!(a, b);
    }

    #[test]
    fn different_purpose_different_stream() {
        let a: u64 = stream(7, &[1, 2, 3]).gen();
        let b: u64 = stream(7, &[1, 2, 4]).gen();
        assert_ne!(a, b);
    }

    /// #780: two purpose tags once shared the value 56 (WORLDGEN_SCREE and CASCADE), which
    /// would silently correlate two unrelated systems the moment either moved to `stream`.
    /// Every tag is listed in `purpose::ALL`; the list must not contain a duplicate.
    #[test]
    fn purpose_tags_are_unique() {
        for (i, a) in purpose::ALL.iter().enumerate() {
            for (j, b) in purpose::ALL.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "purpose tags {i} and {j} both claim {a}");
                }
            }
        }
    }
}
