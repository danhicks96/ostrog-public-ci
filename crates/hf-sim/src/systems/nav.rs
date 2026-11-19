//! Keep navigation regions in step with the grid (incremental, by chunk version).

use crate::world_state::World;

pub fn step(w: &mut World) {
    w.ensure_nav();
    let grid = &w.grid;
    let changed = match w.nav.as_mut() {
        Some(nav) => nav.update(grid) > 0,
        None => false,
    };
    if changed {
        if let Some(astar) = w.astar.as_mut() {
            astar.clear_failed_paths();
        }
        drop_unsupported(w);
        // A carving goes with its wall (dug out, burned, fallen): none is left on bare ground.
        let grid = &w.grid;
        w.engravings
            .retain(|p, _| grid.get(*p).shape == crate::world::Shape::Wall);
    }
}

/// Things lying where the floor has gone (a wall that burned under them, a tile dug out)
/// fall to the first footing below, a few levels at most; nothing floats.
fn drop_unsupported(w: &mut World) {
    let Some(nav) = w.nav.as_ref() else { return };
    let cs = crate::world::CHUNK_SIZE as i32;
    let dirty: std::collections::BTreeSet<_> = nav.last_dirty.iter().copied().collect();
    for k in 0..w.items.len() {
        let crate::items::ItemPlace::Ground(p) = w.items[k].place else {
            continue;
        };
        // Only where the tile itself or the one under it changed.
        let here = (p.x.div_euclid(cs), p.y.div_euclid(cs), p.z);
        let below = (here.0, here.1, p.z - 1);
        if !dirty.contains(&here) && !dirty.contains(&below) {
            continue;
        }
        if w.grid.walkable(p) {
            continue;
        }
        if let Some(q) = (1..=6)
            .map(|d| p.offset(0, 0, -d))
            .find(|q| w.grid.in_bounds(*q) && w.grid.walkable(*q))
        {
            w.items[k].place = crate::items::ItemPlace::Ground(q);
        }
    }
}
