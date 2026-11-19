//! Natural slopes: where a column meets ground one level higher, its surface tile becomes a
//! ramp up, so the landscape is traversable without anyone having to dig stairs (DF rule).

use super::GenParams;
use crate::ids::TilePos;
use crate::world::{Shape, Tile, TileFlags, WorldGrid};

pub fn apply(p: &GenParams, grid: &mut WorldGrid, water: &[bool]) {
    let mut ramps = Vec::new();
    for y in 0..p.height {
        for x in 0..p.width {
            let c = grid.col(x, y);
            if water[c] {
                continue;
            }
            let g = grid.ground_z(x, y);
            let surf = TilePos::new(x, y, g + 1);
            if grid.get(surf).shape != Shape::Open {
                continue;
            }
            for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= p.width || ny >= p.height {
                    continue;
                }
                if water[grid.col(nx, ny)] {
                    continue;
                }
                if grid.ground_z(nx, ny) == g + 1 {
                    let top_mat = grid.get(TilePos::new(nx, ny, g + 1)).material;
                    ramps.push((surf, top_mat));
                    break;
                }
            }
        }
    }
    for (pos, mat) in ramps {
        grid.set(
            pos,
            Tile {
                material: mat,
                shape: Shape::RampUp,
                flags: TileFlags::NONE,
            },
        );
    }
}
