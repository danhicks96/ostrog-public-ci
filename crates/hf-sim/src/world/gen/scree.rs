//! Loose rock on steep bare ground (#289, DEC-8's item 4): where the ground rises
//! steeply from a column and its family is bare soil, grass or sand, the surface lies
//! as scree in patches, or gravel where the pitch is a grade less — below crags, on
//! the shoulders of steep drops. Pure worldgen: no runtime state, seeded per column
//! from the world seed alone, so two generations of a seed read the same slopes.
//!
//! The opening valleys step one level at a time (#698: at this point in generation no
//! column stands two levels off a neighbour on seeds 1 to 4), so "steep" there is a knoll
//! or a ridge edge: a bare top that steps to its neighbours on two or more sides. Those
//! take loose rock only where the patch noise is high (about the top fifth of them, the
//! top twelfth as scree), so a valley shows a few patches, not a rash. A real drop of two
//! or more levels keeps the original, looser test.

use super::noise::fbm2;
use super::GenParams;
use crate::rng::{hash64, purpose};
use crate::world::{Material, Shape, TileFlags, WorldGrid};

/// Families whose steep ground can lie loose: bare earth, not the vegetated
/// (meadow, moss, peat) or already-loose (gravel, scree) covers.
fn bare(m: Material) -> bool {
    matches!(m, Material::SOIL | Material::GRASS | Material::SAND)
}

/// Patch noise above which a knoll or ridge-edge top lies loose (#698), and above which it
/// lies as scree rather than gravel. Measured on the opening worlds of seeds 1 to 4: about
/// 20% and 8% of such tops.
pub const KNOLL_LOOSE: f32 = 0.62;
pub const KNOLL_SCREE: f32 = 0.70;

pub fn apply(seed: u64, p: &GenParams, grid: &mut WorldGrid) {
    let s = hash64(&[seed, purpose::WORLDGEN_SCREE]);
    for y in 0..p.height {
        for x in 0..p.width {
            let g = grid.ground_z(x, y);
            // The local pitch: the steepest step to a neighbour, as ramps.rs and
            // strata.rs's soil reckon it.
            let rise = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .map(|(dx, dy)| (grid.ground_z(x + dx, y + dy) - g).abs())
                .max()
                .unwrap_or(0);
            // Sides on which the ground steps up or down from this column (#698).
            let sides = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .filter(|(dx, dy)| grid.ground_z(x + dx, y + dy) != g)
                .count();
            if rise < 2 && sides < 2 {
                continue;
            }
            let top = crate::ids::TilePos::new(x, y, g);
            let t = grid.get(top);
            if t.shape != Shape::Wall || !bare(t.material) {
                continue;
            }
            // A patch noise of its own, so the loose rock breaks up instead of
            // sheeting every steep tile: below the threshold the family keeps its
            // ground, in the middle it is gravel, at the top scree.
            let v = fbm2(s, x as f32 * 0.35, y as f32 * 0.35, 3, 2.0, 0.5);
            let m = if rise >= 2 {
                match (rise, v) {
                    (3.., v) if v > 0.35 => Material::SCREE,
                    (_, v) if v > 0.55 => Material::SCREE,
                    (_, v) if v > 0.35 => Material::GRAVEL,
                    _ => continue,
                }
            } else {
                // A knoll or ridge edge (#698): the noise here sits around 0.5, so these
                // thresholds keep about a fifth of such tops, the highest as scree.
                match v {
                    v if v > KNOLL_SCREE => Material::SCREE,
                    v if v > KNOLL_LOOSE => Material::GRAVEL,
                    _ => continue,
                }
            };
            let mut nt = t;
            nt.material = m;
            nt.flags = TileFlags::NONE;
            grid.set(top, nt);
        }
    }
}
