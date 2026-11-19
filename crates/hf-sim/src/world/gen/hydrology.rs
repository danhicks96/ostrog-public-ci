//! Water: columns on the valley's lowest ground whose moisture noise is high become lakes;
//! a stream meanders across. A wet
//! world (biome::Character) has more lake, a dry one less. Lake columns two or more tiles from
//! any shore are deep. Returns the set of water columns for flora and founding-site use.
//! `deepen` then gives the water its depth (#390), once the biomes are known.

use super::biome::{Biome, Character};
use super::noise::fbm2;
use super::GenParams;
use crate::ids::TilePos;
use crate::rng::{hash64, purpose};
use crate::world::{Material, Shape, Tile, TileFlags, WorldGrid};

pub fn apply(seed: u64, p: &GenParams, grid: &mut WorldGrid, ch: &Character) -> Vec<bool> {
    let s = hash64(&[seed, purpose::WORLDGEN_WATER]);
    let n = (p.width * p.height) as usize;
    let mut water = vec![false; n];
    let mut lake = vec![false; n];
    let sea = p.sea_level_fraction * (1.0 + 0.5 * ch.wetness);
    let scale = 1.0 / (p.width.max(p.height) as f32 * 0.5);
    // Lakes fill the valley's lowest ground: the lowest level that holds at least one column
    // in twenty (a handful of columns at a lower level would make only puddles).
    let lake_level = {
        let mut zs: Vec<i32> = grid.ground.clone();
        zs.sort_unstable();
        zs[(zs.len() / 20).min(zs.len() - 1)]
    };
    // A meandering stream: a band around a noisy curve across the map.
    for y in 0..p.height {
        for x in 0..p.width {
            let g = grid.ground_z(x, y);
            let moist = fbm2(
                s,
                x as f32 * scale * 3.0,
                y as f32 * scale * 3.0,
                4,
                2.0,
                0.5,
            );
            let t = x as f32 / p.width as f32;
            let curve = 0.5 + (fbm2(s ^ 0x77, t * 3.0, 0.0, 3, 2.0, 0.5) - 0.5) * 0.7;
            let dy = (y as f32 / p.height as f32 - curve).abs();
            let stream = dy < 0.018 && g <= 1;
            let is_lake = g <= lake_level && moist > 1.0 - sea;
            if stream || is_lake {
                let c = grid.col(x, y);
                water[c] = true;
                lake[c] = is_lake;
                // Water occupies the surface tile above the ground (the bed stays solid).
                let surf = TilePos::new(x, y, g + 1);
                grid.set(
                    surf,
                    Tile {
                        material: Material::WATER,
                        shape: Shape::Floor,
                        flags: TileFlags(TileFlags::WATER),
                    },
                );
            }
        }
    }
    // Deep centres: lake columns with water all around for two tiles.
    let mut deep = Vec::new();
    for y in 0..p.height {
        for x in 0..p.width {
            if !lake[grid.col(x, y)] {
                continue;
            }
            let open = (-2..=2).all(|dy| {
                (-2..=2).all(|dx| {
                    let (qx, qy) = (x + dx, y + dy);
                    qx >= 0 && qy >= 0 && qx < p.width && qy < p.height && water[grid.col(qx, qy)]
                })
            });
            if open {
                deep.push(TilePos::new(x, y, grid.ground_z(x, y) + 1));
            }
        }
    }
    for pos in deep {
        let mut t = grid.get(pos);
        t.flags = TileFlags(TileFlags::WATER | TileFlags::DEEP);
        grid.set(pos, t);
    }
    water
}

/// The deepest a lake goes below its surface tile, in levels.
pub const MAX_DEPTH: i32 = 4;

/// #390: water has depth. Each water column is carved down from its surface tile by its
/// distance to the nearest dry column (the map's edge counts as dry): one level at the shore,
/// two a tile in, up to `MAX_DEPTH` in the middle, so a lake shelves and a stream wide enough
/// has a channel down its middle. A fen's water stays one level deep over two levels of
/// waterlogged ground. Carving stops above open ground (a cave, or a lower bank beside), so
/// no water stands over or beside air. Every tile of a column carries its surface tile's
/// flags: a shallow column is shallow all the way down, as the walker sees it.
pub fn deepen(grid: &mut WorldGrid, water: &[bool], biome: &[Biome]) {
    let (w, h) = (grid.width, grid.height);
    let dry = |x: i32, y: i32| x < 0 || y < 0 || x >= w || y >= h || !water[grid.col(x, y)];
    let mut plan: Vec<(i32, i32, i32)> = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if !water[grid.col(x, y)] {
                continue;
            }
            let d = (1..=MAX_DEPTH)
                .find(|r| (-r..=*r).any(|dy| (-r..=*r).any(|dx| dry(x + dx, y + dy))))
                .unwrap_or(MAX_DEPTH);
            plan.push((x, y, d));
        }
    }
    for (x, y, d) in plan {
        let c = grid.col(x, y);
        let g = grid.ground_z(x, y);
        let surf = TilePos::new(x, y, g + 1);
        let mut top = grid.get(surf);
        if !top.is_water() {
            continue;
        }
        if biome.get(c) == Some(&Biome::Fen) {
            // Shallow water over waterlogged ground.
            if top.flags.has(TileFlags::DEEP) {
                top.flags = TileFlags(TileFlags::WATER);
                grid.set(surf, top);
            }
            for z in [g, g - 1] {
                let q = TilePos::new(x, y, z);
                let mut t = grid.get(q);
                if t.shape == Shape::Wall && z > grid.z_min {
                    t.material = Material::WET_SOIL;
                    grid.set(q, t);
                }
            }
            continue;
        }
        let bed = grid.get(TilePos::new(x, y, g)).material;
        let mut bottom = g + 1;
        for z in (g + 2 - d..=g).rev() {
            let under = TilePos::new(x, y, z - 1);
            let held = z - 1 > grid.z_min
                && grid.get(under).shape == Shape::Wall
                && [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().all(|(dx, dy)| {
                    let q = TilePos::new(x + dx, y + dy, z);
                    let t = grid.get(q);
                    !grid.in_bounds(q) || t.shape == Shape::Wall || t.is_water()
                });
            if !held {
                break;
            }
            grid.set(TilePos::new(x, y, z), top);
            bottom = z;
        }
        if bottom <= g {
            // The bed is the lake's own soil, carried down.
            let q = TilePos::new(x, y, bottom - 1);
            let mut t = grid.get(q);
            t.material = bed;
            grid.set(q, t);
        }
    }
}
