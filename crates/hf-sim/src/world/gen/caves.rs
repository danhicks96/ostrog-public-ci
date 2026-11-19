//! Caves: worm-like voids carved below ground where 3D noise sits in a narrow band.

use super::noise::value3;
use super::GenParams;
use crate::ids::TilePos;
use crate::rng::{hash64, purpose};
use crate::world::{Material, Shape, Tile, WorldGrid};

pub fn apply(seed: u64, p: &GenParams, grid: &mut WorldGrid) {
    if p.cave_density <= 0.0 {
        return;
    }
    let s = hash64(&[seed, purpose::WORLDGEN_CAVES]);
    let band = 0.035 * p.cave_density;
    for z in (p.z_min + 2)..=-2 {
        for y in 1..p.height - 1 {
            for x in 1..p.width - 1 {
                if grid.ground_z(x, y) - z < 3 {
                    continue;
                }
                let n = value3(s, x as f32 * 0.07, y as f32 * 0.07, z as f32 * 0.21);
                let n2 = value3(
                    s ^ 0x55,
                    x as f32 * 0.11 + 9.0,
                    y as f32 * 0.11,
                    z as f32 * 0.3,
                );
                if (n - 0.5).abs() < band && (n2 - 0.5).abs() < band * 2.4 {
                    // Carve the void; Open over the solid rock below is walkable.
                    grid.set(
                        TilePos::new(x, y, z),
                        Tile {
                            material: Material::AIR,
                            shape: Shape::Open,
                            flags: Default::default(),
                        },
                    );
                }
            }
        }
    }
}
