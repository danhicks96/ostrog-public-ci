//! Ore veins: short random walks through rock, seeded per vein. What a vein holds follows the
//! rock it starts in (#267): coal in the sedimentary upper stratum, iron and copper in the
//! granite and slate, tin in the granite, silver in the slate; in the deep, whatever its rock
//! in the region, gold and gem pockets, copper in the basalt and gabbro and iron in the gneiss.

use rand::Rng;

use super::GenParams;
use crate::ids::TilePos;
use crate::rng::{purpose, stream};
use crate::world::{Material, Shape, Tile, WorldGrid};

pub fn apply(seed: u64, p: &GenParams, grid: &mut WorldGrid) {
    let area = (p.width * p.height) as f32;
    let veins = (area / 900.0).round().max(4.0) as u32;
    for v in 0..veins {
        let mut r = stream(seed, &[purpose::WORLDGEN_ORE, v as u64]);
        let x0 = r.gen_range(2..p.width - 2);
        let y0 = r.gen_range(2..p.height - 2);
        let g = grid.ground_z(x0, y0);
        let z0 = r.gen_range(p.z_min + 1..=(g - 2).max(p.z_min + 1));
        // What the rock holds depends on the rock: coal in the sedimentary upper stratum, iron
        // and copper in the granite and slate; in the deep, gems, and copper in the basalt and
        // gabbro or iron in the gneiss (#267).
        let c = grid.col(x0, y0);
        let roll = r.gen_range(0..10);
        let (mat, len) = match grid.strata.get(c) {
            Some(&(_, _, deep)) if z0 <= deep as i32 => {
                if roll < 5 {
                    (Material::GEM_ROCK, r.gen_range(3..8))
                } else if grid.rock_at(c, z0) == Material::GNEISS {
                    (Material::IRON_ORE, r.gen_range(6..24))
                } else {
                    (Material::COPPER_ORE, r.gen_range(6..24))
                }
            }
            Some(&(_, top, _)) if z0 <= top as i32 => (
                if roll < 6 {
                    Material::IRON_ORE
                } else {
                    Material::COPPER_ORE
                },
                r.gen_range(6..24),
            ),
            Some(_) => (
                if roll < 6 {
                    Material::COAL
                } else {
                    Material::IRON_ORE
                },
                r.gen_range(6..24),
            ),
            None => (
                match roll {
                    0..=4 => Material::IRON_ORE,
                    5..=7 => Material::COPPER_ORE,
                    _ => Material::COAL,
                },
                r.gen_range(6..24),
            ),
        };
        walk(&mut r, p, grid, mat, (x0, y0, z0), len);
    }
    // Tin, silver and gold (C32), each on a stream of its own so the veins above stay as they
    // were. Tin lies in the granite, where copper is too, so bronze is possible; silver in the
    // slate; gold is rare, a vein or none, in the deep, whatever its rock (#267: it looked for
    // basalt, so a valley whose deep was gabbro or gneiss had none).
    let deep_rock = |grid: &WorldGrid, at: TilePos| {
        let c = grid.col(at.x, at.y);
        grid.strata.get(c).is_some_and(|s| at.z <= s.2 as i32)
            && grid.get(at).material == grid.rock_at(c, at.z)
    };
    for (k, (mat, n, in_rock)) in [
        (
            Material::TIN_ORE,
            (veins / 4).max(1),
            Some(Material::GRANITE),
        ),
        (
            Material::SILVER_ORE,
            (veins / 6).max(1),
            Some(Material::SLATE),
        ),
        (Material::GOLD_ORE, 1, None),
    ]
    .into_iter()
    .enumerate()
    {
        for v in 0..n {
            let mut r = stream(
                seed,
                &[purpose::WORLDGEN_ORE, 0x7100 + k as u64 * 64 + v as u64],
            );
            if mat == Material::GOLD_ORE && r.gen_range(0..2) == 0 {
                continue;
            }
            // A few tries for a start in the right rock.
            for _ in 0..12 {
                let x0 = r.gen_range(2..p.width - 2);
                let y0 = r.gen_range(2..p.height - 2);
                let g = grid.ground_z(x0, y0);
                let z0 = r.gen_range(p.z_min + 1..=(g - 2).max(p.z_min + 1));
                let at = TilePos::new(x0, y0, z0);
                let host = match in_rock {
                    Some(rock) => grid.in_bounds(at) && grid.get(at).material == rock,
                    None => grid.in_bounds(at) && deep_rock(grid, at),
                };
                if host {
                    let len = if mat == Material::GOLD_ORE {
                        r.gen_range(3..7)
                    } else {
                        r.gen_range(5..16)
                    };
                    walk(&mut r, p, grid, mat, (x0, y0, z0), len);
                    break;
                }
            }
        }
    }
    // Gem pockets: small clusters in the deep, whatever its rock, a few to a valley (which gem
    // follows the rock: material.rs `gem_in`).
    if grid.strata.is_empty() {
        return;
    }
    let mut r = stream(seed, &[purpose::WORLDGEN_ORE, 0x6E3]);
    let pockets = ((area / 4000.0).round() as u32).clamp(3, 6);
    for _ in 0..pockets {
        let (x0, y0) = (r.gen_range(3..p.width - 3), r.gen_range(3..p.height - 3));
        let deep = grid.strata[grid.col(x0, y0)].2 as i32;
        if deep <= p.z_min + 1 {
            continue;
        }
        let z0 = r.gen_range(p.z_min + 1..=deep);
        for _ in 0..r.gen_range(4..9) {
            let q = TilePos::new(
                x0 + r.gen_range(-1..=1),
                y0 + r.gen_range(-1..=1),
                (z0 + r.gen_range(-1..=0)).max(p.z_min + 1),
            );
            if grid.in_bounds(q) && grid.get(q).shape == Shape::Wall && stone(grid, q) {
                grid.set(q, Tile::solid(Material::GEM_ROCK));
            }
        }
    }
}

/// A vein: a short random walk through solid rock below the surface, thickened sideways.
fn walk(
    r: &mut rand_chacha::ChaCha8Rng,
    p: &GenParams,
    grid: &mut WorldGrid,
    mat: Material,
    (x0, y0, z0): (i32, i32, i32),
    len: u32,
) {
    let (mut x, mut y, mut z) = (x0, y0, z0);
    for _ in 0..len {
        let pos = TilePos::new(x, y, z);
        if grid.in_bounds(pos)
            && grid.get(pos).shape == Shape::Wall
            && z < grid.ground_z(x, y) - 1
            && stone(grid, pos)
        {
            grid.set(pos, Tile::solid(mat));
            // thicken sideways
            let side = TilePos::new(x + r.gen_range(-1..=1), y + r.gen_range(-1..=1), z);
            if grid.in_bounds(side)
                && grid.get(side).shape == Shape::Wall
                && z < grid.ground_z(side.x, side.y) - 1
                && stone(grid, side)
            {
                grid.set(side, Tile::solid(mat));
            }
        }
        x = (x + r.gen_range(-1..=1)).clamp(1, p.width - 2);
        y = (y + r.gen_range(-1..=1)).clamp(1, p.height - 2);
        if r.gen_range(0..4) == 0 {
            z = (z + r.gen_range(-1..=1)).clamp(p.z_min + 1, p.z_max);
        }
    }
}

/// Whether the tile at `p` is rock a vein can run through: stone or another vein, not the
/// soil over it (#267: soil lies deeper on level ground now).
fn stone(grid: &WorldGrid, p: TilePos) -> bool {
    use crate::world::material::MaterialKind as K;
    matches!(
        grid.materials.get(grid.get(p).material).kind,
        K::Stone | K::Ore
    )
}
