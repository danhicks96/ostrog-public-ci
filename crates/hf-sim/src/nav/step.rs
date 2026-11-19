//! The single definition of "which tiles connect". A* and regions both use it.

use crate::ids::TilePos;
use crate::time::{LEVEL_CHANGE_TICKS, WALK_TICKS_PER_TILE};
use crate::world::{Shape, TileFlags, WorldGrid};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub to: TilePos,
    /// Cost in ticks for a walking inhabitant.
    pub cost: u32,
}

/// Cost of standing on / passing through `p` (walkability already checked).
pub fn step_cost(grid: &WorldGrid, p: TilePos, diagonal: bool) -> u32 {
    let t = grid.get(p);
    let mut c = WALK_TICKS_PER_TILE;
    if t.flags.has(TileFlags::WATER) {
        c += 4; // wading
    }
    if t.flags.has(TileFlags::DOOR) {
        c += 2;
    }
    // A worn path is quicker going (#84).
    if p.z > 0 && grid.get(p.offset(0, 0, -1)).material == crate::world::Material::PACKED_EARTH {
        c = c.saturating_sub(2).max(1);
    }
    // Rough ground underfoot is slower going (#268's roughness row, WP-05 5.6, FIXLOG
    // 2026-11-12): loose scree most, then mud, then gravel and sand; before this a scree slope
    // cost the same to cross as a meadow, so paths went straight over it. Ticks per docs/TIME.md.
    let below = p.offset(0, 0, -1);
    if grid.in_bounds(below) {
        c += rough(grid.get(below).material);
    }
    if diagonal {
        c = c * 14 / 10;
    }
    c
}

/// Extra ticks a step costs over the ground `m` (docs/TIME.md "Ground underfoot").
pub fn rough(m: crate::world::Material) -> u32 {
    use crate::world::Material as M;
    match m {
        M::SCREE => 3,
        M::MUD => 2,
        M::GRAVEL | M::SAND => 1,
        _ => 0,
    }
}

/// What a path counts for a step onto `p`: its walking cost, and far more for water over a
/// person's head (#418), so a way round a flood is taken where there is one. Moving still
/// costs `step_cost`.
fn plan_cost(grid: &WorldGrid, p: TilePos, diagonal: bool) -> u32 {
    step_cost(grid, p, diagonal) + if grid.over_head(p) { OVER_HEAD } else { 0 }
}

/// Ticks a path counts for a step into water over the head (#418).
pub const OVER_HEAD: u32 = 200;

/// All steps from `p` for a walker. Diagonals do not cut corners.
pub fn neighbours(grid: &WorldGrid, p: TilePos, out: &mut Vec<Step>) {
    out.clear();
    if !grid.walkable(p) {
        return;
    }
    let here = grid.get(p);
    // Horizontal, same z.
    const ORTHO: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    let mut ortho_ok = [false; 4];
    for (i, (dx, dy)) in ORTHO.iter().enumerate() {
        let q = p.offset(*dx, *dy, 0);
        if grid.walkable(q) {
            ortho_ok[i] = true;
            out.push(Step {
                to: q,
                cost: plan_cost(grid, q, false),
            });
        }
    }
    const DIAG: [((i32, i32), (usize, usize)); 4] = [
        ((1, -1), (0, 1)),
        ((1, 1), (1, 2)),
        ((-1, 1), (2, 3)),
        ((-1, -1), (3, 0)),
    ];
    for ((dx, dy), (a, b)) in DIAG {
        if ortho_ok[a] && ortho_ok[b] {
            let q = p.offset(dx, dy, 0);
            if grid.walkable(q) {
                out.push(Step {
                    to: q,
                    cost: plan_cost(grid, q, true),
                });
            }
        }
    }
    // Vertical: stairs/ladders at the same xy. A link between two levels exists when
    // either tile asserts it (the lower connects up, or the upper connects down), and it
    // works in both directions. Regions union links undirected, so the step rule must be
    // symmetric too; a one-way flight (Up stair over Up stair) once left a cellar that
    // regions called reachable and A* could not enter.
    {
        let q = p.offset(0, 0, 1);
        if grid.in_bounds(q) {
            let t = grid.get(q);
            let linked = here.shape.connects_up() || t.shape.connects_down();
            if linked && (grid.walkable(q) || t.shape.connects_down()) {
                out.push(Step {
                    to: q,
                    cost: LEVEL_CHANGE_TICKS,
                });
            }
        }
    }
    {
        let q = p.offset(0, 0, -1);
        if grid.in_bounds(q) {
            let t = grid.get(q);
            let linked = here.shape.connects_down() || t.shape.connects_up();
            if linked && (grid.walkable(q) || t.shape.connects_up()) {
                out.push(Step {
                    to: q,
                    cost: LEVEL_CHANGE_TICKS,
                });
            }
        }
    }
    // Ramps: from a RampUp tile you may step to an adjacent tile one level up; from any
    // tile you may step down onto an adjacent RampUp one level below.
    if here.shape == Shape::RampUp {
        for (dx, dy) in ORTHO {
            let q = p.offset(dx, dy, 1);
            if grid.walkable(q) {
                out.push(Step {
                    to: q,
                    cost: LEVEL_CHANGE_TICKS,
                });
            }
        }
    }
    for (dx, dy) in ORTHO {
        let q = p.offset(dx, dy, -1);
        if grid.in_bounds(q) && grid.get(q).shape == Shape::RampUp && grid.walkable(q) {
            out.push(Step {
                to: q,
                cost: LEVEL_CHANGE_TICKS,
            });
        }
    }
}
