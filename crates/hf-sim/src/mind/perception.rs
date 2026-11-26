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

/// Look over a natural bank at a single-level surface drop. A scout's eyes are
/// inside the upper walking cell; the top of the lower water touches its bottom
/// plane. The entire ray therefore crosses only upper-level cells. A wet crest
/// or a floor over the pool can hide it even when the crest itself is in sight.
pub(crate) fn sees_spill_from_bank(
    w: &crate::World,
    from: TilePos,
    edge: crate::world::gen::watershed::SpillEdge,
) -> bool {
    use crate::world::Shape;
    let g = &w.grid;
    let pool = edge.lower;
    if from.z != edge.upper.z
        || pool.z + 1 != from.z
        || g.surface(from.x, from.y) != from
        || g.get(from).is_water()
        || crate::building::roofed(w, from)
        || crate::building::roofed(w, edge.upper)
        || crate::building::roofed(w, pool)
        || !line_of_sight(g, from, edge.upper)
    {
        return false;
    }
    let clear = |x, y| {
        let p = TilePos::new(x, y, from.z);
        let tile = g.get(p);
        p.z > g.ground_z(x, y)
            && !tile.is_water()
            && matches!(tile.shape, Shape::Open | Shape::RampTop)
            && !w.roofs.contains_key(&p.offset(0, 0, -1))
    };
    let (mut x, mut y) = (from.x, from.y);
    let (dx, dy) = ((pool.x - x).abs(), (pool.y - y).abs());
    let (sx, sy) = ((pool.x - x).signum(), (pool.y - y).signum());
    let (mut ix, mut iy) = (0, 0);
    while x != pool.x || y != pool.y {
        // Compare the times to cross the next x/y cell boundaries as fractions.
        // At an exact corner either side can block a view through the diagonal.
        let next_x = (2 * ix + 1) * dy;
        let next_y = (2 * iy + 1) * dx;
        if next_x == next_y && (!clear(x + sx, y) || !clear(x, y + sy)) {
            return false;
        }
        if next_x <= next_y {
            x += sx;
            ix += 1;
        }
        if next_y <= next_x {
            y += sy;
            iy += 1;
        }
        if !clear(x, y) {
            return false;
        }
    }
    true
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
