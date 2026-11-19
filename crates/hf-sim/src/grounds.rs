//! Ground taken a tile at a time from need (C71, Dan's rule 1, D35): a stockpile, a field, a
//! cellar room or a quarry's gallery is not a block of a set size but a patch that starts at a
//! seed and spreads over the ground there is, as much as there is need for, and grows later as
//! the need does. Houses grow the same way in houses.rs.

use std::collections::BTreeSet;

use crate::ids::TilePos;
use crate::world::Shape;
use crate::world_state::World;

const N4: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// Whether `p` is open ground a patch may take: on its level, open above the ground, not
/// water, never flooded, not taken; a tree stands in the way unless `trees` (a field is cleared of them).
pub fn open_ground(w: &World, p: TilePos, taken: &BTreeSet<TilePos>, trees: bool) -> bool {
    let g = &w.grid;
    if p.x < 3 || p.y < 3 || p.x >= g.width - 3 || p.y >= g.height - 3 {
        return false;
    }
    let t = g.get(p);
    if t.shape != Shape::Open || t.is_water() || g.ground_z(p.x, p.y) + 1 != p.z {
        return false;
    }
    if taken.contains(&p) {
        return false;
    }
    // Not below the high-water line a flood has shown them (#393).
    if w.waters.flooded_once(p.x, p.y) {
        return false;
    }
    if !trees
        && w.plants
            .iter()
            .any(|pl| pl.alive && pl.pos == p && pl.kind.is_tree())
    {
        return false;
    }
    g.walkable(p) || trees
}

/// `open_ground` without trees, for a `taken` that already holds every living tree (as
/// `occupied(w, true)` in systems/settlement.rs does): the same answer without walking every
/// plant in the valley for each tile (#428: the quarry's site search looked at 2,601 tiles
/// that way and took 8 ms at 500 people).
pub fn open_ground_trees_taken(w: &World, p: TilePos, taken: &BTreeSet<TilePos>) -> bool {
    let g = &w.grid;
    if p.x < 3 || p.y < 3 || p.x >= g.width - 3 || p.y >= g.height - 3 {
        return false;
    }
    let t = g.get(p);
    if t.shape != Shape::Open || t.is_water() || g.ground_z(p.x, p.y) + 1 != p.z {
        return false;
    }
    if taken.contains(&p) || w.waters.flooded_once(p.x, p.y) {
        return false;
    }
    g.walkable(p)
}

/// Up to `n` tiles grown outward from `seeds` (which are not themselves returned when they
/// are already part of the patch), nearest the first seed first, each joined to the patch by
/// a side. Fewer when the ground runs out.
pub fn grow(
    w: &World,
    seeds: &[TilePos],
    n: usize,
    taken: &BTreeSet<TilePos>,
    trees: bool,
    include_seeds: bool,
) -> Vec<TilePos> {
    let Some(&origin) = seeds.first() else {
        return Vec::new();
    };
    let mut have: BTreeSet<TilePos> = seeds.iter().copied().collect();
    let mut out: Vec<TilePos> = Vec::new();
    if include_seeds {
        for s in seeds {
            if open_ground(w, *s, taken, trees) && !out.contains(s) {
                out.push(*s);
            }
        }
    }
    // Frontier: open tiles beside the patch, taken nearest the origin first (ties by position
    // for determinism).
    let mut frontier: BTreeSet<(i32, TilePos)> = BTreeSet::new();
    let push_round =
        |p: TilePos, frontier: &mut BTreeSet<(i32, TilePos)>, have: &BTreeSet<TilePos>| {
            for (dx, dy) in N4 {
                let q = p.offset(dx, dy, 0);
                if !have.contains(&q) && open_ground(w, q, taken, trees) {
                    frontier.insert((q.dist(origin), q));
                }
            }
        };
    for s in seeds {
        push_round(*s, &mut frontier, &have);
    }
    while out.len() < n {
        let Some(&(d, q)) = frontier.iter().next() else {
            break;
        };
        frontier.remove(&(d, q));
        if have.contains(&q) {
            continue;
        }
        have.insert(q);
        out.push(q);
        push_round(q, &mut frontier, &have);
    }
    out
}
