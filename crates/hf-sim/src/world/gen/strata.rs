//! Rock strata: three named layers by depth. The upper stratum is sedimentary (limestone or
//! sandstone by region), the middle is granite or slate (marble in patches under limestone),
//! split into sub-bands that alternate the two hard rocks (#267), and the deep is basalt, or
//! gabbro or gneiss by region (#267), with a coal seam lying flat
//! through the sandstone in places. The breaks
//! between them lie at nearly the same height across the valley, a step up or down here and
//! there, so a shaft shows recognisable layers and two shafts show the same ones.
//!
//! Over the rock lies the soil (#267, #268): three levels deep on the level ground of a plain
//! or a marsh, two on the low level ground of a valley or a coast, one on slopes and heights.

use super::noise::fbm2;
use super::GenParams;
use crate::rng::{hash64, purpose};
use crate::world::{Material, WorldGrid};

/// The heights of the two breaks before their wobble: the top of the middle stratum and the
/// top of the deep stratum, from the z range below ground.
pub fn breaks(p: &GenParams) -> (i32, i32) {
    let span = -p.z_min;
    (top_break(p.z_min), p.z_min + (span * 3 / 10).max(2))
}

/// The top of the middle stratum before its wobble, for a world reaching down to `z_min`.
pub fn top_break(z_min: i32) -> i32 {
    -((-z_min) / 5).max(2)
}

pub fn apply(seed: u64, p: &GenParams, grid: &mut WorldGrid) {
    let s = hash64(&[seed, purpose::WORLDGEN_STRATA]);
    let scale = 1.0 / (p.width.max(p.height) as f32 * 0.6);
    let (top, deep) = breaks(p);
    grid.strata = Vec::with_capacity((p.width * p.height) as usize);
    grid.deep = Vec::with_capacity((p.width * p.height) as usize);
    grid.bands = Vec::with_capacity((p.width * p.height) as usize);
    grid.soil = (0..p.height)
        .flat_map(|y| (0..p.width).map(move |x| (x, y)))
        .map(|(x, y)| soil_depth(p, grid, x, y))
        .collect();
    // One seam height for the whole valley, between the surface and the middle stratum, so a
    // seam found in one shaft is found at the same depth in the next.
    let seam_z = top + 1 + (hash64(&[s, 0x5EA]) % 3) as i32;
    // #267: how many breaks split the middle stratum, one for the whole valley (two or
    // three sub-bands), so two shafts read the same sequence.
    let extra = 1 + (hash64(&[s, 0x6B1]) % 2) as i32;
    for y in 0..p.height {
        for x in 0..p.width {
            let (fx, fy) = (x as f32 * scale * 3.0, y as f32 * scale * 3.0);
            let upper = if fbm2(s, fx, fy, 3, 2.0, 0.5) < 0.5 {
                Material::LIMESTONE
            } else {
                Material::SANDSTONE
            };
            let middle = if fbm2(s ^ 0x3A1, fx * 0.8, fy * 0.8, 3, 2.0, 0.5) < 0.55 {
                Material::GRANITE
            } else {
                Material::SLATE
            };
            let middle_base = middle;
            // Under limestone, here and there, the middle stratum is the same limestone baked
            // to marble (C32). A noise of its own, so the rest of the rock stays where it was.
            let middle = if upper == Material::LIMESTONE
                && fbm2(s ^ 0x3A7, fx * 0.9, fy * 0.9, 3, 2.0, 0.5) > 0.6
            {
                Material::MARBLE
            } else {
                middle
            };
            // A step up or down where the layers fold.
            let fold = |k: u64| -> i32 {
                let v = fbm2(s ^ k, fx * 0.6, fy * 0.6, 2, 2.0, 0.5);
                if v < 0.3 {
                    -1
                } else if v > 0.7 {
                    1
                } else {
                    0
                }
            };
            // The deep by region, on a broader noise than the strata above: gneiss under the
            // slate, gabbro in patches elsewhere, basalt most of all.
            let dn = fbm2(s ^ 0x3B3, fx * 0.5, fy * 0.5, 3, 2.0, 0.5);
            let deep_rock = if middle == Material::SLATE && dn > 0.45 {
                Material::GNEISS
            } else if dn > 0.65 {
                Material::GABBRO
            } else {
                Material::BASALT
            };
            let c = grid.col(x, y);
            let seam = if upper == Material::SANDSTONE
                && fbm2(s ^ 0x3C5, fx * 0.7, fy * 0.7, 3, 2.0, 0.5) > 0.5
                && seam_z < grid.ground[c]
            {
                seam_z as i8
            } else {
                i8::MIN
            };
            // The middle stratum in sub-bands, alternating with the other hard rock so a dug
            // shaft reads a sequence (granite over slate over granite, or the other way). The
            // breaks sit at even fractions of the band and wobble with their own folds.
            let top_c = top + fold(0x51);
            let deep_c = (deep + fold(0x52)).max(p.z_min + 1);
            let other = if middle_base == Material::GRANITE {
                Material::SLATE
            } else {
                Material::GRANITE
            };
            let span = top_c - deep_c;
            let mut bands: Vec<(Material, i8)> = Vec::new();
            if span >= 6 {
                for k in 1..=extra {
                    let b = deep_c + span * k / (extra + 1) + fold(0x53 + k as u64);
                    // Keep the breaks strictly ordered and clear of the stratum breaks, so
                    // every band is actually dug through somewhere in the valley.
                    if b >= deep_c + 2
                        && b <= top_c - 2
                        && bands.last().is_none_or(|&(_, t)| b >= t as i32 + 2)
                    {
                        // Alternate from the pushed bands, not the loop index: a break the
                        // ordering guard dropped must not change which rock comes next.
                        let rock = if bands.len().is_multiple_of(2) {
                            other
                        } else {
                            middle_base
                        };
                        bands.push((rock, b as i8));
                    }
                }
            }
            grid.deep.push((deep_rock, seam));
            grid.bedrock[c] = upper;
            grid.bands.push(bands);
            grid.strata.push((middle, top_c as i8, deep_c as i8));
        }
    }
}

/// Levels of soil over the rock at column (x, y), the surface tile among them: soil gathers
/// on low, level ground and washes off slopes and heights.
fn soil_depth(p: &GenParams, grid: &WorldGrid, x: i32, y: i32) -> u8 {
    use super::Landform as L;
    let g = grid.ground_z(x, y);
    let slope = [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .iter()
        .map(|(dx, dy)| (grid.ground_z(x + dx, y + dy) - g).abs())
        .max()
        .unwrap_or(0);
    if slope > 0 {
        return 1;
    }
    let low = g * 2 <= p.relief;
    match p.landform {
        L::Plain | L::Marsh => 3,
        _ if low => 2,
        _ => 1,
    }
}
