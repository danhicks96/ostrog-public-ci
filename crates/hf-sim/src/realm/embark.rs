//! Embarking: which region the valley is, and the climate it brings to the valley's own
//! generation. C24 provides the mapping; the valley is generated from it from C27 on (until
//! then `Character::of_seed` still decides an opening world's climate).

use rand::Rng;

use super::map::{RBiome, RealmMap};
use crate::rng::{purpose, stream};
use crate::world::gen::biome::Character;

/// The valley climate a region gives: wetness from its rain (a rank among land regions), warmth
/// from its yearly temperature, stoniness from its height. Each in -1..1, as Character expects.
pub fn character_of(m: &RealmMap, i: usize) -> Character {
    let wetness = (m.rain[i] * 2.0 - 1.0).clamp(-1.0, 1.0);
    let warmth = ((m.temperature[i] - 9.0) / 13.0).clamp(-1.0, 1.0);
    let rocky = matches!(m.biome[i], RBiome::Mountain | RBiome::Tundra | RBiome::Ice);
    let stoniness = (m.height(i) * 2.4 - 0.6 + if rocky { 0.4 } else { 0.0 }).clamp(-1.0, 1.0);
    Character {
        wetness,
        warmth,
        stoniness,
    }
}

/// Could people settle here: land that is not ice, bare mountain, open water or deep desert,
/// with a growing season.
pub fn habitable(m: &RealmMap, i: usize) -> bool {
    m.land(i)
        && !matches!(m.biome[i], RBiome::Ice | RBiome::Mountain | RBiome::Desert)
        && (2.0..26.0).contains(&m.temperature[i])
}

/// The region a world embarks in when nobody chose one: a habitable region, preferring river
/// banks and lake shores, picked by the seed.
pub fn default_region(seed: u64, m: &RealmMap) -> usize {
    let n = m.size * m.size;
    let near_water = |i: usize| {
        let (x, y) = m.xy(i);
        (x.saturating_sub(1)..=(x + 1).min(m.size - 1)).any(|xx| {
            (y.saturating_sub(1)..=(y + 1).min(m.size - 1)).any(|yy| {
                let j = m.idx(xx, yy);
                m.river[j] || m.biome[j] == RBiome::Lake
            })
        })
    };
    let good: Vec<usize> = (0..n)
        .filter(|&i| habitable(m, i) && near_water(i))
        .collect();
    let pool: Vec<usize> = if good.is_empty() {
        (0..n).filter(|&i| habitable(m, i)).collect()
    } else {
        good
    };
    if pool.is_empty() {
        return (0..n).find(|&i| m.land(i)).unwrap_or(0);
    }
    let mut r = stream(seed, &[purpose::REALM_MAP, 9]);
    pool[r.gen_range(0..pool.len())]
}
