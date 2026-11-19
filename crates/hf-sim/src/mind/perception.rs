//! Local perception: what an agent can see this tick (same z, within radius, line of sight
//! not blocked by solid tiles). Produces knowledge updates; does not decide anything.

use crate::ids::TilePos;
use crate::world::WorldGrid;

pub const SIGHT_RADIUS: i32 = 12;

thread_local! {
    static SIGHT_LINES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many sight lines this thread has traced, for the cost tests (#205).
pub fn sight_lines() -> u64 {
    SIGHT_LINES.with(|c| c.get())
}

/// Bresenham line of sight on one z-level; solid tiles block, the endpoints do not.
pub fn line_of_sight(grid: &WorldGrid, from: TilePos, to: TilePos) -> bool {
    SIGHT_LINES.with(|c| c.set(c.get() + 1));
    if from.z != to.z {
        return false;
    }
    let (mut x0, mut y0) = (from.x, from.y);
    let (x1, y1) = (to.x, to.y);
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        if (x0, y0) == (x1, y1) {
            return true;
        }
        if (x0, y0) != (from.x, from.y) && grid.get(TilePos::new(x0, y0, from.z)).blocks_sight() {
            return false;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

/// Who is there to see something at `at` (#854): alive people, awake, on its level, within `r`
/// tiles and in line of sight, and not children when `grown_only`. Index order, so deterministic.
pub fn witnesses(w: &crate::World, at: TilePos, r: i32, grown_only: bool) -> Vec<usize> {
    let tick = w.meta.tick;
    (0..w.agents.len())
        .filter(|&i| {
            let a = &w.agents[i];
            a.alive
                && a.species == crate::agents::Species::Person
                && !a.activity.is_asleep()
                && !(grown_only && a.is_child(tick))
                && a.pos.z == at.z
                && a.pos.dist(at) <= r
                && line_of_sight(&w.grid, a.pos, at)
        })
        .collect()
}

pub fn can_see(grid: &WorldGrid, from: TilePos, to: TilePos) -> bool {
    let d = (from.x - to.x).abs().max((from.y - to.y).abs());
    d <= SIGHT_RADIUS && line_of_sight(grid, from, to)
}
