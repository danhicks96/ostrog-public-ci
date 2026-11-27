//! Keep navigation regions in step with the grid (incremental, by chunk version).

use crate::world_state::World;

pub fn step(w: &mut World) {
    w.ensure_nav();
    let grid = &w.grid;
    let Some(nav) = w.nav.as_mut() else { return };
    let changed = nav.update(grid) > 0;
    // Ground stacks can be placed between terrain edits, including immediately before
    // saving. Use the same physical check on a warm nav as after loading a cold one.
    drop_unsupported(w);
    if changed {
        if let Some(astar) = w.astar.as_mut() {
            astar.clear_failed_paths();
        }
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
        // A single dense region lookup skips supported stacks without reading terrain.
        if nav.region_of(p) != crate::nav::regions::NO_REGION {
            continue;
        }
        let grid = &w.grid;
        if grid.air_over_liquid(p) {
            let mut q = p.offset(0, 0, -1);
            while q.z > grid.z_min && grid.get(q.offset(0, 0, -1)).liquid_floor() {
                q = q.offset(0, 0, -1);
            }
            // Stop on the water side of the first actual bed or deck.
            let bed = q.offset(0, 0, -1);
            if grid.in_bounds(bed) && grid.get(bed).supports_above() {
                w.items[k].place = crate::items::ItemPlace::Ground(q);
            }
            continue;
        }
        if dirty.is_empty() {
            continue;
        }
        let here = (p.x.div_euclid(cs), p.y.div_euclid(cs), p.z);
        let below = (here.0, here.1, p.z - 1);
        if !dirty.contains(&here) && !dirty.contains(&below) {
            continue;
        }
        if grid.get(p).liquid_floor() {
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
