//! Cave mouths (#482): a rare natural opening in a hillside that leads, down a
//! sloping passage one tile wide, into a cave or cavern that is really there.
//!
//! The passage is cut only through solid rock and is walked by the same rules as everything
//! else: level stretches are open floor, each step down is a natural ramp (`Shape::RampUp`, as
//! on any slope), and the passage keeps a roof of rock over it from the entrance inward. A
//! candidate is found by searching outward from the face for the nearest connected cave room of
//! at least eight floors, then checked by walking it with the game's own step rules in both
//! directions; one that does not walk cleanly is put back as it was. The existing walkable
//! approach, including a natural ramp, is preserved. Most hills have no mouth.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use rand::Rng;

use super::GenParams;
use crate::ids::TilePos;
use crate::nav::step::{neighbours, Step};
use crate::rng::{purpose, stream};
use crate::world::{Material, Shape, Tile, TileFlags, WorldGrid};

/// How far (tiles, each way) a passage may wander from where it starts.
const REACH: i32 = 18;
/// The shallowest and deepest cave floor a mouth leads to; deeper caverns are reached the
/// hard way.
const FLOOR_TOP: i32 = -2;
const FLOOR_BOTTOM: i32 = -10;
/// Mouths are at least this far apart (tiles).
const APART: i32 = 24;
/// Candidate faces tried at most.
const TRIES: usize = 400;

/// A mouth that was cut.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mouth {
    /// The open ground at the foot of the face, where someone stands to go in.
    pub foot: TilePos,
    /// The first tile of the passage, in the face.
    pub entrance: TilePos,
    /// The cave floor it opens into.
    pub cave: TilePos,
    /// Tiles cut, the entrance included.
    pub length: usize,
}

const ORTHO: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

/// Whether `p` is a floor in a cave: open, with rock under it, well below the ground.
fn cave_floor(grid: &WorldGrid, p: TilePos) -> bool {
    grid.in_bounds(p)
        && grid.get(p).shape == Shape::Open
        && grid.walkable(p)
        && !grid.get(p).is_water()
        && p.z <= FLOOR_TOP
        && grid.ground_z(p.x, p.y) - p.z >= 3
}

const MIN_CAVE_ROOM: usize = 8;

/// Volume of every orthogonally connected, same-level cave-floor component. FIXLOG 2026-10-30,
/// branch 482-cave-mouths: one isolated open tile is not a cave room worth opening to the valley.
fn cave_rooms(grid: &WorldGrid) -> BTreeMap<TilePos, usize> {
    let mut volumes = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for y in 1..grid.height - 1 {
        for x in 1..grid.width - 1 {
            for z in FLOOR_BOTTOM..=FLOOR_TOP {
                let start = TilePos::new(x, y, z);
                if seen.contains(&start) || !cave_floor(grid, start) {
                    continue;
                }
                let mut room = Vec::new();
                let mut q = VecDeque::new();
                seen.insert(start);
                q.push_back(start);
                while let Some(at) = q.pop_front() {
                    room.push(at);
                    for (dx, dy) in ORTHO {
                        let next = at.offset(dx, dy, 0);
                        if !seen.contains(&next) && cave_floor(grid, next) {
                            seen.insert(next);
                            q.push_back(next);
                        }
                    }
                }
                let volume = room.len();
                for at in room {
                    volumes.insert(at, volume);
                }
            }
        }
    }
    volumes
}

fn is_cave_room(rooms: &BTreeMap<TilePos, usize>, at: TilePos) -> bool {
    rooms.get(&at).copied().unwrap_or(0) >= MIN_CAVE_ROOM
}

/// A cell of the passage: where, and whether it is a step down (a ramp) or level floor.
#[derive(Clone, Copy, Debug)]
struct Cell {
    at: TilePos,
    ramp: bool,
}

pub fn apply(seed: u64, p: &GenParams, grid: &mut WorldGrid, water: &[bool]) -> Vec<Mouth> {
    let mut out = Vec::new();
    if p.cave_density <= 0.0 {
        return out;
    }
    let mut r = stream(seed, &[purpose::WORLDGEN_CAVES, 0x40CE]);
    // None in a fifth of worlds, else one to three; fewer in small ones.
    let roll = r.gen_range(0..100);
    let mut want = match roll {
        0..=19 => 0,
        20..=54 => 1,
        55..=84 => 2,
        _ => 3,
    };
    if p.width * p.height < 100 * 100 {
        want = (want + 1) / 2;
    }
    if want == 0 {
        return out;
    }

    // Where the caves open near the top: the places a passage can end.
    let rooms = cave_rooms(grid);
    let mut near: BTreeSet<(i32, i32)> = BTreeSet::new();
    for y in 1..p.height - 1 {
        for x in 1..p.width - 1 {
            for z in FLOOR_BOTTOM..=FLOOR_TOP {
                if is_cave_room(&rooms, TilePos::new(x, y, z)) {
                    near.insert((x / 8, y / 8));
                    break;
                }
            }
        }
    }
    if near.is_empty() {
        return out;
    }

    // Faces: open ground at the foot, rock rising beside it with flanking rock on both sides.
    let mut faces: Vec<(TilePos, (i32, i32))> = Vec::new();
    for y in 2..p.height - 2 {
        for x in 2..p.width - 2 {
            if water[grid.col(x, y)] {
                continue;
            }
            let gf = grid.ground_z(x, y);
            let foot = TilePos::new(x, y, gf + 1);
            if gf + 1 < FLOOR_TOP + 1 || !grid.walkable(foot) || grid.get(foot).is_water() {
                continue;
            }
            for (dx, dy) in ORTHO {
                let (ax, ay) = (x + dx, y + dy);
                if water[grid.col(ax, ay)] || grid.ground_z(ax, ay) < gf + 1 {
                    continue;
                }
                // Flanked by solid hillside on both sides of the entrance so nobody walks in
                // from the side.
                let f1 = TilePos::new(ax - dy, ay + dx, gf + 1);
                let f2 = TilePos::new(ax + dy, ay - dx, gf + 1);
                if !grid.in_bounds(f1)
                    || !grid.in_bounds(f2)
                    || grid.get(f1).shape != Shape::Wall
                    || grid.get(f2).shape != Shape::Wall
                {
                    continue;
                }
                let (bx, by) = (ax + dx, ay + dy);
                if bx < 1 || by < 1 || bx >= p.width - 1 || by >= p.height - 1 {
                    continue;
                }
                if grid.ground_z(bx, by) < gf + 1 || water[grid.col(bx, by)] {
                    continue;
                }
                // Rock beyond, holding up the roof over the first step down.
                let (ex, ey) = (bx + dx, by + dy);
                if ex < 1 || ey < 1 || ex >= p.width - 1 || ey >= p.height - 1 {
                    continue;
                }
                if grid.ground_z(ex, ey) < gf + 1 {
                    continue;
                }
                // A cave must open somewhere within reach.
                let (cx, cy) = (x / 8, y / 8);
                let reach = REACH / 8 + 1;
                if (-reach..=reach)
                    .any(|i| (-reach..=reach).any(|j| near.contains(&(cx + i, cy + j))))
                {
                    faces.push((foot, (dx, dy)));
                }
            }
        }
    }
    // A shuffle by this world's own stream, then the higher ground first: mouths are in hills.
    for i in (1..faces.len()).rev() {
        let j = r.gen_range(0..=i);
        faces.swap(i, j);
    }
    faces.sort_by_key(|(f, _)| -f.z);

    let trace = std::env::var("HF_MOUTH_TRACE").is_ok();
    let (mut found, mut failed, mut apart) = (0, 0, 0);
    if trace {
        eprintln!(
            "mouths: want {want}, {} cave cells near, {} faces",
            near.len(),
            faces.len()
        );
    }
    for (foot, d) in faces.into_iter().take(TRIES) {
        if out.len() as i32 >= want {
            break;
        }
        if out
            .iter()
            .any(|m: &Mouth| m.foot.dist(foot) < APART || m.cave.dist(foot) < APART / 2)
        {
            apart += 1;
            continue;
        }
        let Some((path, cave)) = search(grid, &rooms, water, foot, d) else {
            continue;
        };
        found += 1;
        if let Some(m) = cut(grid, foot, &path, cave) {
            out.push(m);
        } else {
            failed += 1;
        }
    }
    if trace {
        eprintln!(
            "mouths: {found} routes found, {failed} failed the walk, {apart} too near another, {} cut",
            out.len()
        );
    }
    out
}

/// The shortest way through solid rock from the face at `foot` (a step in direction `d`) to a
/// cave room: level or one step down at a time, roofed and floored in rock, touching no open
/// air but the cave at its end.
fn search(
    grid: &WorldGrid,
    rooms: &BTreeMap<TilePos, usize>,
    water: &[bool],
    foot: TilePos,
    d: (i32, i32),
) -> Option<(Vec<Cell>, TilePos)> {
    let face = TilePos::new(foot.x + d.0, foot.y + d.1, foot.z);
    // FIXLOG 2026-10-31, branch 482-cave-mouths: search from the face as before, but enter on
    // a downward ramp beneath it. The face stays as the natural roof, so cutting the entrance
    // does not raise or lower ground_z.
    let start = face.offset(0, 0, -1);
    if grid.get(face).shape != Shape::Wall
        || grid.get(start).shape != Shape::Wall
        || grid.get(start.offset(0, 0, -1)).shape != Shape::Wall
    {
        return None;
    }
    let mut order = ORTHO.to_vec();
    order.sort_by_key(|&o| o != d);
    let mut from: BTreeMap<TilePos, (TilePos, bool)> = BTreeMap::new();
    let mut seen: BTreeSet<TilePos> = BTreeSet::new();
    seen.insert(face);
    let mut q = VecDeque::new();
    q.push_back((face, 0usize));
    while let Some((s, steps)) = q.pop_front() {
        if steps > 40 {
            continue;
        }
        for &(dx, dy) in &order {
            // The first step from the entrance goes straight into the hillside.
            if s == face && (dx, dy) != d {
                continue;
            }
            // FIXLOG 2026-10-30, branch 482-cave-mouths: explore both vertical siblings.
            // Treating a visited level cell as blocking its ramp alternative stranded every
            // route at the entrance's z-level. `cut` still rejects stacked cells in the path.
            for dz in [0, -1] {
                let c = s.offset(dx, dy, dz);
                if (c.x - start.x).abs() > REACH
                    || (c.y - start.y).abs() > REACH
                    || c.x < 1
                    || c.y < 1
                    || c.x >= grid.width - 1
                    || c.y >= grid.height - 1
                    || c.z < FLOOR_BOTTOM
                    || seen.contains(&c)
                    || water[grid.col(c.x, c.y)]
                {
                    continue;
                }
                if grid.get(c).shape != Shape::Wall
                    || grid.get(c.offset(0, 0, 1)).shape != Shape::Wall
                    || grid.get(c.offset(0, 0, -1)).shape != Shape::Wall
                {
                    continue;
                }
                // Touch no open air (or earlier passage cell) but the cave this ends in.
                let mut cave = None;
                let mut spoilt = false;
                for (ex, ey) in ORTHO {
                    let n = c.offset(ex, ey, 0);
                    if n == s {
                        continue;
                    }
                    if seen.contains(&n) {
                        spoilt = true;
                        break;
                    }
                    if grid.get(n).shape != Shape::Wall {
                        if is_cave_room(rooms, n) {
                            cave = cave.or(Some(n));
                        } else {
                            spoilt = true;
                        }
                    }
                }
                if !spoilt && dz == -1 {
                    // A ramp at `c` connects to all four orthogonal neighbours at `c.z + 1`;
                    // every one of them except `s` must be solid rock and not in the passage.
                    for (ex, ey) in ORTHO {
                        let u = c.offset(ex, ey, 1);
                        if u == s {
                            continue;
                        }
                        if grid.get(u).shape != Shape::Wall || seen.contains(&u) {
                            spoilt = true;
                            break;
                        }
                    }
                }
                if !spoilt && dz == 0 {
                    for (ex, ey) in ORTHO {
                        let b = c.offset(ex, ey, -1);
                        if b != s && seen.contains(&b) {
                            spoilt = true;
                            break;
                        }
                    }
                }
                if spoilt {
                    continue;
                }
                seen.insert(c);
                from.insert(c, (s, dz != 0));
                if let Some(cave) = cave {
                    let mut path = Vec::new();
                    let mut at = c;
                    loop {
                        let (prev, ramp) = from[&at];
                        path.push(Cell { at, ramp });
                        if prev == face {
                            break;
                        }
                        at = prev;
                    }
                    path.push(Cell {
                        at: start,
                        ramp: true,
                    });
                    path.reverse();
                    return Some((path, cave));
                }
                q.push_back((c, steps + 1));
            }
        }
    }
    None
}

/// Cut the passage under its existing rock brow, then walk it both ways with the step rules;
/// put everything back if any check fails.
fn cut(grid: &mut WorldGrid, foot: TilePos, path: &[Cell], cave: TilePos) -> Option<Mouth> {
    let start = path[0].at;
    let brow = start.offset(0, 0, 1);
    let sill = start.offset(0, 0, -1);
    // FIXLOG 2026-10-31, branch 482-cave-mouths: a synthetic brow raises ground_z and exposes
    // rock as biome ground; only a naturally roofed face may be opened.
    if grid.get(brow).shape != Shape::Wall {
        return None;
    }
    let sill_rock = grid.get(sill).material;
    let mut saved: Vec<(TilePos, Tile)> = vec![(brow, grid.get(brow)), (sill, grid.get(sill))];
    for c in path {
        saved.push((c.at, grid.get(c.at)));
        let below = c.at.offset(0, 0, -1);
        saved.push((below, grid.get(below)));
    }

    // Keep the walkable approach exactly as the natural-ramp stage left it. FIXLOG 2026-10-30,
    // branch 482-cave-mouths: flattening a RampUp foot severed the route from higher ground.
    // The entrance has a rock brow above and stone underfoot so it is roofed from the first step.
    grid.set(sill, Tile::solid(sill_rock));

    for c in path {
        // FIXLOG 2026-10-30, branch 482-cave-mouths: the cut's rock belongs to its actual
        // stratum, not necessarily to the column's upper bedrock.
        let old_mat = grid.get(c.at).material;
        let t = if c.ramp {
            Tile {
                material: old_mat,
                shape: Shape::RampUp,
                flags: TileFlags::NONE,
            }
        } else {
            Tile {
                material: Material::AIR,
                shape: Shape::Open,
                flags: TileFlags::NONE,
            }
        };
        grid.set(c.at, t);
    }

    let mut walk = vec![foot];
    walk.extend(path.iter().map(|c| c.at));
    walk.push(cave);
    let mut buf: Vec<Step> = Vec::new();
    let mut ok = true;
    for c in path {
        if grid.get(c.at.offset(0, 0, 1)).shape != Shape::Wall
            || grid.get(c.at.offset(0, 0, -1)).shape != Shape::Wall
        {
            ok = false;
        }
    }
    for (i, &a) in walk.iter().enumerate() {
        neighbours(grid, a, &mut buf);
        if i > 0 && !buf.iter().any(|s| s.to == walk[i - 1]) {
            ok = false;
        }
        if i + 1 < walk.len() && !buf.iter().any(|s| s.to == walk[i + 1]) {
            ok = false;
        }
        for (j, &b) in walk.iter().enumerate() {
            if j.abs_diff(i) > 1 && buf.iter().any(|s| s.to == b) {
                ok = false;
            }
        }
    }
    if !ok {
        for (at, old) in saved.into_iter().rev() {
            grid.set(at, old);
        }
        return None;
    }
    Some(Mouth {
        foot,
        entrance: start,
        cave,
        length: path.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_constructed_cut_restores_every_tile_and_ground_height() {
        let mut grid = WorldGrid::new(9, 9, -4, 4);
        for y in 1..8 {
            for x in 1..8 {
                let c = grid.col(x, y);
                grid.ground[c] = 2;
            }
        }
        let foot = TilePos::new(2, 3, 1);
        let start = TilePos::new(3, 3, 1);
        let second = TilePos::new(4, 3, 1);
        let cave = TilePos::new(5, 3, 1);
        grid.set(foot, Tile::AIR);
        grid.set(second.offset(0, 0, 1), Tile::AIR);
        grid.set(cave, Tile::AIR);
        let before = grid.clone();
        let path = [
            Cell {
                at: start,
                ramp: false,
            },
            Cell {
                at: second,
                ramp: false,
            },
        ];

        assert!(cut(&mut grid, foot, &path, cave).is_none());
        assert_eq!(grid.ground, before.ground);
        for z in grid.z_min..=grid.z_max {
            for y in 0..grid.height {
                for x in 0..grid.width {
                    let at = TilePos::new(x, y, z);
                    assert_eq!(grid.get(at), before.get(at), "tile {at} was not restored");
                }
            }
        }
    }
}
