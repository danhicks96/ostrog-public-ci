//! Houses grown from what a household needs (C67). A house is the floor its builders claimed,
//! a tile at a time over the ground they found, until there was room for those it was for; its
//! walls are whatever closes that floor in, and its door faces home. A household that outgrows
//! its house claims more floor beside it, and the old wall between gets a door. Nothing here
//! is a template: the shape is the ground's and the household's (Dan, 2026-10-21: everything
//! built after the colonists arrive is grown, not set down whole).
//!
//! Houses from saves before C67 have no recorded floor: they are the 7×6 they were built as.

use std::collections::{BTreeMap, BTreeSet};

use crate::ids::{CommunityId, TilePos};
use crate::society::HousePlan;
use crate::world::Shape;
use crate::world_state::World;

/// The largest floor a house grows to, annexes and all.
pub const MAX_FLOOR: usize = 64;

const N4: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// The plan of the house at `origin`: its recorded floor, or for an old house the 7×6 it was.
pub fn plan(w: &World, origin: TilePos) -> HousePlan {
    w.communities
        .iter()
        .find_map(|c| {
            let k = c.house_sites.iter().position(|o| *o == origin)?;
            c.house_plans
                .get(k)
                .filter(|p| !p.floor.is_empty())
                .cloned()
        })
        .unwrap_or_else(|| old_plan(origin))
}

/// The 7×6 every house was before C67, `origin` its north-west corner.
pub fn old_plan(origin: TilePos) -> HousePlan {
    let mut floor = Vec::new();
    for y in 1..5 {
        for x in 1..6 {
            floor.push(origin.offset(x, y, 0));
        }
    }
    HousePlan {
        floor,
        doors: vec![origin.offset(3, 5, 0)],
        ..HousePlan::default()
    }
}

/// The wall line round a floor: every tile touching it, corners too, that is not floor. Each
/// with whether it is a corner (it touches the floor only at a diagonal), which goes up first.
pub fn walls(floor: &[TilePos]) -> Vec<(TilePos, bool)> {
    let set: BTreeSet<TilePos> = floor.iter().copied().collect();
    let mut ring: BTreeMap<TilePos, bool> = BTreeMap::new();
    for f in floor {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let q = f.offset(dx, dy, 0);
                if set.contains(&q) {
                    continue;
                }
                let side = dx == 0 || dy == 0;
                let e = ring.entry(q).or_insert(true);
                if side {
                    *e = false;
                }
            }
        }
    }
    ring.into_iter().collect()
}

/// The tile of the floor nearest its middle: where the house is said to be.
pub fn heart(floor: &[TilePos]) -> Option<TilePos> {
    let n = floor.len() as i32;
    if n == 0 {
        return None;
    }
    let (sx, sy) = floor.iter().fold((0, 0), |a, p| (a.0 + p.x, a.1 + p.y));
    floor
        .iter()
        .copied()
        .min_by_key(|p| ((p.x * n - sx).pow(2) + (p.y * n - sy).pow(2), *p))
}

/// Whether every tile of the house's wall line stands (a door counts).
pub fn complete(w: &World, plan: &HousePlan) -> bool {
    walls(&plan.floor)
        .iter()
        .all(|(q, _)| w.grid.get(*q).shape == Shape::Wall)
}

/// How far apart beds stand for a community's bodies: the next bed no nearer than this many
/// tiles. Two for most; three for a big folk (half again the reference body's mass or more).
pub fn bed_gap(w: &World, cid: CommunityId) -> i32 {
    let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
        return 2;
    };
    let (n, sum) = w
        .agents
        .iter()
        .filter(|a| a.alive && c.members.contains(&a.id))
        .fold((0u32, 0.0f32), |a, ag| (a.0 + 1, a.1 + ag.mass_kg()));
    if n > 0 && sum / n as f32 >= 1.5 * crate::genetics::REFERENCE_KG {
        3
    } else {
        2
    }
}

/// Where things go inside a house, worked out from its floor and doors: the beds, farthest
/// from a door first and `gap` apart; the stair down to a cellar, on the farthest floor left;
/// the table, stools and chest by the door. The tiles just inside a door stay clear.
pub struct Layout {
    pub beds: Vec<TilePos>,
    pub stair: Option<TilePos>,
    pub table: Vec<TilePos>,
}

pub fn layout(plan: &HousePlan, gap: i32) -> Layout {
    let inside: BTreeSet<TilePos> = plan
        .doors
        .iter()
        .flat_map(|d| N4.iter().map(move |(x, y)| d.offset(*x, *y, 0)))
        .collect();
    let to_door = |p: TilePos| plan.doors.iter().map(|d| p.dist(*d)).min().unwrap_or(0);
    let mut far: Vec<TilePos> = plan
        .floor
        .iter()
        .copied()
        .filter(|p| !inside.contains(p))
        .collect();
    far.sort_by_key(|p| (std::cmp::Reverse(to_door(*p)), *p));
    let mut beds: Vec<TilePos> = Vec::new();
    for p in &far {
        if beds
            .iter()
            .all(|b| (b.x - p.x).abs().max((b.y - p.y).abs()) >= gap)
        {
            beds.push(*p);
        }
    }
    let rest: Vec<TilePos> = far.iter().copied().filter(|p| !beds.contains(p)).collect();
    let stair = rest.first().copied();
    let mut near: Vec<TilePos> = rest.iter().copied().filter(|p| Some(*p) != stair).collect();
    near.sort_by_key(|p| (to_door(*p), *p));
    near.truncate(4);
    Layout {
        beds,
        stair,
        table: near,
    }
}

/// Floor a house can take at `p`: open, flat, dry ground on the level `z` that no flood has
/// covered (#393), nobody's work or
/// structure on it, no wall of anything within two tiles (so there is a way round between
/// houses), and not within `keep_clear` of a home. A tree may stand on it: it is felled.
fn can_floor(w: &World, p: TilePos, z: i32, ground: &Ground, own: &BTreeSet<TilePos>) -> bool {
    let taken = &ground.taken;
    let g = &w.grid;
    if p.z != z || p.x < 3 || p.y < 3 || p.x >= g.width - 3 || p.y >= g.height - 3 {
        return false;
    }
    let t = g.get(p);
    if t.shape != Shape::Open || t.is_water() || g.ground_z(p.x, p.y) + 1 != z || !g.walkable(p) {
        return false;
    }
    if taken.contains(&p) || w.waters.flooded_once(p.x, p.y) {
        return false;
    }
    // Not over a living plant (#253: a berry bush inside the walls fed a shut-in for a year).
    if ground.living.contains(&p) {
        return false;
    }
    for dy in -2..=2 {
        for dx in -2..=2 {
            let q = p.offset(dx, dy, 0);
            if own.contains(&q) {
                continue;
            }
            if g.get(q).shape == Shape::Wall {
                return false;
            }
            // The ring round the floor must be ground the walls can stand on.
            if dx.abs() <= 1 && dy.abs() <= 1 {
                let tq = g.get(q);
                if tq.is_water() || g.ground_z(q.x, q.y) + 1 != z || taken.contains(&q) {
                    return false;
                }
            }
        }
    }
    true
}

/// What a house search reads of the ground, gathered once per search (#428: `can_floor` walked
/// every plant in the valley for each tile it looked at, and `grow` gathered the trees again
/// for each of up to 24 floors; a new house took 19 ms at 500 people).
struct Ground {
    /// `taken_ground`.
    taken: BTreeSet<TilePos>,
    /// Where a living plant stands.
    living: BTreeSet<TilePos>,
    /// Where a living tree stands.
    trees: BTreeSet<TilePos>,
}

impl Ground {
    fn of(w: &World) -> Ground {
        let alive = || w.plants.iter().filter(|pl| pl.alive);
        Ground {
            taken: taken_ground(w),
            living: alive().map(|pl| pl.pos).collect(),
            trees: alive()
                .filter(|pl| pl.kind.is_tree())
                .map(|pl| pl.pos)
                .collect(),
        }
    }
}

/// What no house may be grown over: structures, open work that is not felling, and the ground
/// round every home and the founding site.
fn taken_ground(w: &World) -> BTreeSet<TilePos> {
    let mut set: BTreeSet<TilePos> = w.structures.iter().map(|s| s.pos).collect();
    // The ground round anything that sparks (#241: a house grown round the community's
    // hearth burned its roof and beds every few weeks and was never roofed).
    let sparks = |k: crate::structures::StructureKind| {
        matches!(
            k,
            crate::structures::StructureKind::Hearth
                | crate::structures::StructureKind::Kiln
                | crate::structures::StructureKind::Furnace
        )
    };
    let hot: Vec<TilePos> = w
        .structures
        .iter()
        .filter(|s| sparks(s.kind))
        .map(|s| s.pos)
        .chain(w.designations.iter().filter_map(|d| match d.kind {
            crate::work::DesignationKind::BuildStructure(k, _) if sparks(k) => Some(d.pos),
            _ => None,
        }))
        .collect();
    // Two tiles (#257): a spark lands on the ring and spreads to its neighbours, so a wall or
    // roof beside the ring burned from it.
    for h in hot {
        for dy in -2..=2 {
            for dx in -2..=2 {
                set.insert(h.offset(dx, dy, 0));
            }
        }
    }
    set.extend(
        w.designations
            .iter()
            .filter(|d| d.kind != crate::work::DesignationKind::Chop)
            .map(|d| d.pos),
    );
    let mut homes: Vec<TilePos> = w.communities.iter().map(|c| c.home).collect();
    homes.push(w.meta.founding_site);
    for h in homes {
        for dy in -2..=2 {
            for dx in -2..=2 {
                set.insert(h.offset(dx, dy, 0));
            }
        }
    }
    for c in &w.communities {
        for o in &c.house_sites {
            let p = plan(w, *o);
            set.extend(p.floor.iter().copied());
            set.extend(walls(&p.floor).into_iter().map(|(q, _)| q));
        }
        // The wall's ring and the ground just inside it (C73): a house is not grown into it.
        if let Some(pal) = &c.palisade {
            for q in &pal.ring {
                for (dx, dy) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
                    set.insert(q.offset(dx, dy, 0));
                }
            }
        }
    }
    set
}

/// Whether a floor beds `sleepers` at `gap` with room about them: two tiles a sleeper and four
/// for the table and the way in, or twice that for a big folk.
fn enough(plan: &HousePlan, sleepers: u32, gap: i32) -> bool {
    floor_enough(plan.floor.len(), sleepers, gap)
        && layout(plan, gap).beds.len() >= sleepers as usize
}

/// The floor-size half of `enough`, which needs no door or layout.
fn floor_enough(tiles: usize, sleepers: u32, gap: i32) -> bool {
    let per = if gap >= 3 { 4 } else { 2 };
    tiles >= 4 + per * sleepers as usize
}

/// Claim floor outward from `from`, nearest the middle first (felling trees last), until the
/// household has room or the ground runs out. `own` is the house's own wall line, which an
/// annex may stand against.
#[allow(clippy::too_many_arguments)]
fn grow(
    w: &World,
    from: TilePos,
    sleepers: u32,
    gap: i32,
    ground: &Ground,
    own: &BTreeSet<TilePos>,
    home: TilePos,
) -> HousePlan {
    let (z, mid) = (from.z, from);
    let trees = &ground.trees;
    let cost = |p: TilePos| {
        let d = (p.x - mid.x).pow(2) + (p.y - mid.y).pow(2);
        d * 4 + if trees.contains(&p) { 9 } else { 0 }
    };
    let mut floor: Vec<TilePos> = Vec::new();
    let mut set: BTreeSet<TilePos> = BTreeSet::new();
    let mut frontier: BTreeSet<(i32, TilePos)> = BTreeSet::new();
    let mut seen: BTreeSet<TilePos> = BTreeSet::new();
    if can_floor(w, from, z, ground, own) {
        frontier.insert((cost(from), from));
        seen.insert(from);
    }
    let mut plan = HousePlan {
        floor: Vec::new(),
        doors: Vec::new(),
        ..HousePlan::default()
    };
    while let Some((c, p)) = frontier.iter().next().copied() {
        frontier.remove(&(c, p));
        floor.push(p);
        set.insert(p);
        for (dx, dy) in N4 {
            let q = p.offset(dx, dy, 0);
            if !set.contains(&q) && seen.insert(q) && can_floor(w, q, z, ground, own) {
                frontier.insert((cost(q), q));
            }
        }
        if floor.len() >= MAX_FLOOR {
            break;
        }
        // The door is only looked for once the floor is big enough to be enough (#428: a door
        // looked for at every tile grown made a new house for a party of 500, whose floor
        // never reached the size, take 17 ms on one thread; 3 ms without). What the loop
        // leaves in `plan` is replaced below.
        if floor.len() >= 4 && floor_enough(floor.len(), sleepers, gap) {
            plan.floor = floor.clone();
            plan.doors = door(w, &plan.floor, home).into_iter().collect();
            if !plan.doors.is_empty() && enough(&plan, sleepers, gap) {
                return plan;
            }
        }
    }
    plan.floor = floor;
    plan.doors = door(w, &plan.floor, home).into_iter().collect();
    plan
}

/// The door of a floor: a wall tile on a straight side, with open ground outside it, nearest
/// home.
fn door(w: &World, floor: &[TilePos], home: TilePos) -> Option<TilePos> {
    let set: BTreeSet<TilePos> = floor.iter().copied().collect();
    let ring: BTreeSet<TilePos> = walls(floor).into_iter().map(|(q, _)| q).collect();
    ring.iter()
        .copied()
        .filter(|q| {
            N4.iter().any(|(dx, dy)| {
                let inn = q.offset(*dx, *dy, 0);
                let out = q.offset(-*dx, -*dy, 0);
                set.contains(&inn)
                    && !set.contains(&out)
                    && !ring.contains(&out)
                    && w.grid.walkable(out)
            })
        })
        .min_by_key(|q| (q.dist(home), *q))
}

/// A new house for `sleepers` near `near` (C67): the first place, nearest `near`, where floor
/// grown from a tile gives them room; else the roomiest found that beds at least two.
pub fn new_house(
    w: &World,
    near: TilePos,
    home: TilePos,
    sleepers: u32,
    gap: i32,
) -> Option<HousePlan> {
    let ground = Ground::of(w);
    let none = BTreeSet::new();
    let reach = |p: TilePos| {
        w.nav
            .as_ref()
            .map(|n| n.reach(w.meta.founding_site, p))
            .unwrap_or(true)
    };
    // The seeds in ring order; the first 24 that can be floored and reached are tried.
    let seeds = (0..30i32)
        .flat_map(|r| {
            (-r..=r).flat_map(move |dy| {
                (-r..=r)
                    .filter(move |dx| dx.abs() == r || dy.abs() == r)
                    .map(move |dx| (dx, dy))
            })
        })
        .filter_map(|(dx, dy)| {
            let x = near.x + dx;
            let y = near.y + dy;
            let z = w.grid.ground_z(x, y) + 1;
            let seed = TilePos::new(x, y, z);
            (can_floor(w, seed, z, &ground, &none) && reach(seed)).then_some(seed)
        })
        .take(24);
    // Grown a batch at a time across threads (#428: one house took 39 ms at 500 people) and
    // judged in ring order, so the house chosen is the one the walk one by one chose.
    let batch = crate::par::threads().max(1);
    let mut best: Option<HousePlan> = None;
    let seeds: Vec<TilePos> = seeds.collect();
    for chunk in seeds.chunks(batch) {
        let grown = crate::par::map(chunk, |seed| {
            grow(w, *seed, sleepers, gap, &ground, &none, home)
        });
        for p in grown {
            if p.doors.is_empty() {
                continue;
            }
            if enough(&p, sleepers, gap) {
                return Some(p);
            }
            let beds = layout(&p, gap).beds.len();
            if beds >= 2
                && best
                    .as_ref()
                    .is_none_or(|b| beds > layout(b, gap).beds.len())
            {
                best = Some(p);
            }
        }
    }
    best
}

/// More floor for a house whose household has outgrown it (C67): grown from the ground just
/// outside its walls, against them, for `sleepers` more; and the wall tile where the old house
/// and the new floor meet, to be a door between them.
pub fn annex(
    w: &World,
    origin: TilePos,
    home: TilePos,
    sleepers: u32,
    gap: i32,
) -> Option<(Vec<TilePos>, TilePos)> {
    let old = plan(w, origin);
    let ring = walls(&old.floor);
    let own: BTreeSet<TilePos> = ring.iter().map(|(q, _)| *q).collect();
    let mut ground = Ground::of(w);
    let taken = &mut ground.taken;
    for q in &own {
        taken.remove(q);
    }
    // The ground beside a straight stretch of wall, two tiles out from the floor.
    let old_set: BTreeSet<TilePos> = old.floor.iter().copied().collect();
    let mut starts: Vec<(TilePos, TilePos)> = Vec::new();
    for (q, corner) in &ring {
        if *corner || old.doors.contains(q) {
            continue;
        }
        for (dx, dy) in N4 {
            let inn = q.offset(dx, dy, 0);
            let out = q.offset(-dx, -dy, 0);
            if old_set.contains(&inn) && !own.contains(&out) && !old_set.contains(&out) {
                starts.push((out, *q));
            }
        }
    }
    starts.sort_by_key(|(o, _)| (o.dist(home), *o));
    let room = MAX_FLOOR.saturating_sub(old.floor.len());
    // The ground outside the old house's doors stays open (#253: a room grown over the door's
    // step turned the door into one between two closed rooms, and whoever was inside when the
    // last wall went up stayed inside for a year).
    let door_steps: Vec<TilePos> = old
        .doors
        .iter()
        .flat_map(|d| {
            N4.iter()
                .map(|(dx, dy)| d.offset(*dx, *dy, 0))
                .filter(|q| !old_set.contains(q) && !own.contains(q))
                .collect::<Vec<_>>()
        })
        .collect();
    // Of the rooms that would do, the one that keeps the house most compact (#240: rooms
    // grown always toward home made a chain of two-tile cells, a door between each).
    let mut best: Option<(i32, Vec<TilePos>, TilePos)> = None;
    for (start, between) in starts.into_iter().take(8) {
        if room < 8 || !can_floor(w, start, start.z, &ground, &own) {
            continue;
        }
        let mut p = grow(w, start, sleepers, gap, &ground, &own, home);
        // The annex's own wall line must not cut through the old floor, nor the room or its
        // walls take the step outside a door.
        let ring = walls(&p.floor);
        if p.floor.len() < 4
            || ring.iter().any(|(q, _)| old_set.contains(q))
            || door_steps
                .iter()
                .any(|s| p.floor.contains(s) || ring.iter().any(|(q, _)| q == s))
        {
            continue;
        }
        p.doors.push(between);
        if p.floor.len() <= room && layout(&p, gap).beds.len() as u32 >= sleepers.min(2) {
            let all = old.floor.iter().chain(p.floor.iter());
            let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
            for q in all {
                x0 = x0.min(q.x);
                y0 = y0.min(q.y);
                x1 = x1.max(q.x);
                y1 = y1.max(q.y);
            }
            let perimeter = (x1 - x0 + 1) + (y1 - y0 + 1);
            if best.as_ref().is_none_or(|b| perimeter < b.0) {
                best = Some((perimeter, p.floor, between));
            }
        }
    }
    best.map(|(_, f, b)| (f, b))
}

/// Whether a dug room of `room`, entered by `way_in`, sleeps `sleepers` at `gap` (C67): beds
/// set as in a house, and two tiles a sleeper, four for a big folk.
pub fn room_sleeps(room: &[TilePos], way_in: TilePos, sleepers: u32, gap: i32) -> bool {
    let per = if gap >= 3 { 4 } else { 2 };
    room.len() >= per * sleepers as usize && room_beds(room, way_in, gap).len() >= sleepers as usize
}

/// Where the beds go in a dug room: as in a house, farthest from the way in first.
pub fn room_beds(room: &[TilePos], way_in: TilePos, gap: i32) -> Vec<TilePos> {
    layout(
        &HousePlan {
            floor: room.to_vec(),
            doors: vec![way_in],
            ..HousePlan::default()
        },
        gap,
    )
    .beds
}

/// A single upper sleeping footprint that can be grown over an existing house (#483). The
/// footprint is the lower room lifted one tile: this is deliberately a plan, not a prefab, and
/// keeps every lower wall as a real support. The chosen stair is recorded separately because its
/// upper tile is a stair opening, not a floor tile. Returns none when the standing house cannot
/// carry the footprint or the chosen opening is occupied.
pub fn upper_sleeping_candidate(
    w: &World,
    plan: &HousePlan,
    gap: i32,
) -> Option<(Vec<TilePos>, TilePos)> {
    if plan.floor.len() < 4 || !plan.upper_floor.is_empty() {
        return None;
    }
    let lower_stair = layout(plan, gap).stair?;
    let upper_stair = lower_stair.offset(0, 0, 1);
    if !w.grid.walkable(lower_stair)
        || w.grid.get(upper_stair).shape != Shape::Open
        || w.structures
            .iter()
            .any(|s| s.pos == lower_stair || s.pos == upper_stair)
        || w.designations
            .iter()
            .any(|d| !d.done && (d.pos == lower_stair || d.pos == upper_stair))
    {
        return None;
    }
    let upper_floor: Vec<TilePos> = plan.floor.iter().map(|p| p.offset(0, 0, 1)).collect();
    // At proposal time the upper room is still empty, so only its first supported edge can be
    // checked against the standing lower walls. The remaining contiguous tiles become
    // supportable as that edge is laid, and the worker's `floor_supported` guard is repeated for
    // every tile at execution time.
    if !upper_floor
        .iter()
        .filter(|p| **p != upper_stair)
        .any(|p| crate::building::floor_supported(w, *p))
    {
        return None;
    }
    Some((upper_floor, lower_stair))
}

/// The largest room a hollow or dugout is dug to.
pub const MAX_ROOM: usize = 40;

/// Whether a room of at most `MAX_ROOM` tiles could ever sleep `sleepers` (`room_sleeps`
/// wants two tiles a sleeper, four for a big folk). When it could not, a search for one finds
/// nothing, and need not look (#428: a party of 486 looked at 1,600 places, digging each to
/// `MAX_ROOM`, 17 ms).
pub fn a_room_could_sleep(sleepers: u32, gap: i32) -> bool {
    let per = if gap >= 3 { 4 } else { 2 };
    per * sleepers as usize <= MAX_ROOM
}

/// Dig a room outward from the tiles next to `from` (the room so far, or the first tile in),
/// a tile at a time, nearest `toward` first, through what `ok` allows, until `enough` says it
/// will do (C67). None if the rock runs out first.
pub fn dig_room(
    from: &[TilePos],
    first: Option<TilePos>,
    toward: TilePos,
    ok: &dyn Fn(TilePos) -> bool,
    enough: &dyn Fn(&[TilePos]) -> bool,
) -> Option<Vec<TilePos>> {
    let mut room: Vec<TilePos> = from.to_vec();
    let mut seen: BTreeSet<TilePos> = room.iter().copied().collect();
    let mut frontier: BTreeSet<(i32, TilePos)> = BTreeSet::new();
    let cost = |p: TilePos| (p.x - toward.x).pow(2) + (p.y - toward.y).pow(2);
    let add = |p: TilePos, seen: &mut BTreeSet<TilePos>, f: &mut BTreeSet<(i32, TilePos)>| {
        for (dx, dy) in N4 {
            let q = p.offset(dx, dy, 0);
            if seen.insert(q) && ok(q) {
                f.insert((cost(q), q));
            }
        }
    };
    match first {
        Some(f) if ok(f) && seen.insert(f) => {
            frontier.insert((cost(f), f));
        }
        Some(_) => return None,
        None => {
            for p in room.clone() {
                add(p, &mut seen, &mut frontier);
            }
        }
    }
    while !enough(&room) {
        if room.len() >= MAX_ROOM {
            return None;
        }
        let (c, p) = *frontier.iter().next()?;
        frontier.remove(&(c, p));
        room.push(p);
        add(p, &mut seen, &mut frontier);
    }
    Some(room)
}

/// #274: the stone most of the walls of the house at `p` (on its floor) are built of, when it
/// is a fine stone (tiers.rs `fine`): a marble house.
pub fn fine_stone(w: &World, p: TilePos) -> Option<crate::world::Material> {
    for c in &w.communities {
        for o in &c.house_sites {
            let plan = plan(w, *o);
            if !plan.floor.contains(&p) {
                continue;
            }
            return fine_stone_of(w, &plan);
        }
    }
    None
}

/// `fine_stone` for a known plan.
fn fine_stone_of(w: &World, plan: &HousePlan) -> Option<crate::world::Material> {
    let mut of: BTreeMap<crate::world::Material, u32> = BTreeMap::new();
    let ring = walls(&plan.floor);
    for (q, _) in &ring {
        if let Some(m) = w.made_of(*q) {
            *of.entry(m).or_default() += 1;
        }
    }
    let (m, n) = of
        .iter()
        .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
        .map(|(m, n)| (*m, *n))?;
    (n * 2 > ring.len() as u32 && crate::tiers::fine(w, m)).then_some(m)
}

/// How good a room is, a few honest numbers each from something physical, each 0..1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RoomStats {
    pub space: f32,
    pub furnish: f32,
    pub light: f32,
    pub beauty: f32,
    pub clutter: f32,
    /// Zero until a dirt source exists.
    pub dirt: f32,
}

impl RoomStats {
    /// One number, -1..1: what the room is worth to be in.
    pub fn impressiveness(&self) -> f32 {
        (0.3 * self.space + 0.3 * self.furnish + 0.15 * self.light + 0.25 * self.beauty
            - 0.3 * self.clutter
            - 0.3 * self.dirt)
            .clamp(-1.0, 1.0)
    }
}

/// Whether `p` has constructed flooring: a laid floor tile on its own level (upper floors), or a
/// built solid tile directly below an open room tile (ground floors, as `BuildFloor` makes them).
fn laid_floor(w: &World, p: TilePos) -> bool {
    use crate::world::{Shape, TileFlags};
    if w.grid.get(p).shape == Shape::Floor {
        return true;
    }
    let below = p.offset(0, 0, -1);
    let t = w.grid.get(below);
    t.shape == Shape::Wall && !t.flags.has(TileFlags::DOOR) && w.built_of.contains_key(&below)
}

/// The room stats of the house `plan`, for `sleepers` people. Pure: reads the world only.
pub fn room_stats(w: &World, plan: &HousePlan, sleepers: u32) -> RoomStats {
    use crate::items::{ItemKind, ItemPlace};
    use crate::structures::StructureKind as K;
    let floor: BTreeSet<TilePos> = plan.floor.iter().copied().collect();
    let ring: BTreeSet<TilePos> = walls(&plan.floor).into_iter().map(|(p, _)| p).collect();
    let n = sleepers.max(1) as f32;
    let space = ((floor.len() as f32 / n - 3.0) / 5.0).clamp(0.0, 1.0);
    let mut quality = 0.0;
    let mut light = false;
    for s in w.structures.iter().filter(|s| s.progress >= 1.0) {
        let on_floor = floor.contains(&s.pos);
        if on_floor && matches!(s.kind, K::Stool | K::Chair | K::Table | K::Chest | K::Shelf) {
            quality += s.quality;
        }
        if s.active && (on_floor || ring.contains(&s.pos)) && matches!(s.kind, K::Lamp | K::Hearth)
        {
            light = true;
        }
    }
    let furnish = (quality / (n * 1.5)).clamp(0.0, 1.0);
    let laid = if floor.is_empty() {
        0.0
    } else {
        floor.iter().filter(|p| laid_floor(w, **p)).count() as f32 / floor.len() as f32
    };
    let mut loose = 0;
    let mut figurine = false;
    for it in &w.items {
        let ItemPlace::Ground(p) = it.place else {
            continue;
        };
        if !floor.contains(&p) {
            continue;
        }
        if it.kind == ItemKind::Figurine {
            figurine = true;
        } else if !it.kind.is_food() || it.rot >= 0.5 {
            loose += 1;
        }
    }
    let bodies = w
        .rites_due
        .iter()
        .filter(|r| !r.away)
        .filter_map(|r| w.agents.iter().find(|a| a.id == r.dead))
        .filter(|a| floor.contains(&a.pos))
        .count();
    let carved = ring.iter().any(|p| w.engravings.contains_key(p));
    let beauty = 0.4 * f32::from(fine_stone_of(w, plan).is_some())
        + 0.3 * laid
        + 0.3 * f32::from(carved || figurine);
    RoomStats {
        space,
        furnish,
        light: f32::from(light),
        beauty,
        clutter: ((loose + bodies) as f32 / 4.0).clamp(0.0, 1.0),
        dirt: 0.0,
    }
}

/// Where one person could put up a lean-to: the nearest tile to `home` within 12 tiles
/// that is flat, walkable, dry (not water, mud or a flood's way), unbuilt, not ordered, not in a
/// house plan or on or inside a palisade, and a place `who` does not fear (`place_multiplier`
/// of 0.8 or more). Pure: scans in (y, x) order and keeps the first of the nearest, so the same
/// world always gives the same tile. Nothing calls it yet.
pub fn lean_to_site(w: &World, who: &crate::agents::Agent, home: TilePos) -> Option<TilePos> {
    use crate::systems::settlement::{occupied, palisade_holds};
    const REACH: i32 = 12;
    let g = &w.grid;
    let taken = occupied(w, true);
    let now = w.meta.tick;
    let in_plan = |p: TilePos| {
        w.communities.iter().any(|c| {
            c.house_plans
                .iter()
                .any(|h| h.floor.contains(&p) || h.doors.contains(&p) || h.upper_floor.contains(&p))
        })
    };
    let on_wall = |p: TilePos| {
        w.communities.iter().any(|c| {
            c.palisade
                .as_ref()
                .is_some_and(|pal| pal.ring.contains(&p) || palisade_holds(pal, p))
        })
    };
    let mut best: Option<(i32, TilePos)> = None;
    for y in (home.y - REACH)..=(home.y + REACH) {
        for x in (home.x - REACH)..=(home.x + REACH) {
            if x < 1 || y < 1 || x >= g.width - 1 || y >= g.height - 1 {
                continue;
            }
            let gz = g.ground_z(x, y);
            let p = TilePos::new(x, y, gz + 1);
            let d = home.dist(p);
            if d > REACH || best.is_some_and(|b| b.0 <= d) {
                continue;
            }
            let t = g.get(p);
            if t.shape != Shape::Open || t.is_water() || !g.walkable(p) {
                continue;
            }
            if g.get(TilePos::new(x, y, gz)).material == crate::world::Material::MUD
                || crate::systems::decide::unsafe_ground(w, p)
                || taken.contains(&p)
                || in_plan(p)
                || on_wall(p)
                || who.place_multiplier(p, now) < 0.8
            {
                continue;
            }
            best = Some((d, p));
        }
    }
    best.map(|b| b.1)
}

/// A free seat (OST-1307): the nearest finished stool or chair within `within`
/// tiles of `near` on the same level that someone can walk to, that no other person holds or
/// stands on, and, when `at_table`, that has a finished table orthogonally beside it. Ties go
/// to the lower id. Pure: nothing is reserved here.
pub fn free_seat(
    w: &World,
    near: TilePos,
    within: i32,
    at_table: bool,
    who: crate::ids::AgentId,
) -> Option<(crate::ids::StructureId, TilePos)> {
    use crate::structures::StructureKind as K;
    use crate::work::ResKey;
    let done = |k: &crate::structures::Structure| k.progress >= 1.0;
    w.structures
        .iter()
        .filter(|s| done(s) && matches!(s.kind, K::Stool | K::Chair))
        .filter(|s| s.pos.z == near.z)
        .filter(|s| (s.pos.x - near.x).abs().max((s.pos.y - near.y).abs()) <= within)
        .filter(|s| w.reservations.is_free_for(ResKey::Station(s.id), who))
        .filter(|s| !w.agents.iter().any(|a| a.id != who && a.pos == s.pos))
        .filter(|s| {
            !at_table
                || w.structures.iter().any(|t| {
                    done(t)
                        && t.kind == K::Table
                        && t.pos.z == s.pos.z
                        && (t.pos.x - s.pos.x).abs() + (t.pos.y - s.pos.y).abs() == 1
                })
        })
        .filter(|s| w.nav.as_ref().is_some_and(|n| n.reach(near, s.pos)))
        .min_by_key(|s| ((s.pos.x - near.x).abs().max((s.pos.y - near.y).abs()), s.id))
        .map(|s| (s.id, s.pos))
}

#[cfg(test)]
mod tests {
    use super::{a_room_could_sleep, floor_enough, MAX_ROOM};

    #[test]
    fn room_search_is_skipped_only_when_no_maximum_room_can_fit() {
        assert!(a_room_could_sleep(10, 3));
        assert!(!a_room_could_sleep(11, 3));
        assert!(a_room_could_sleep(20, 2));
        assert!(!a_room_could_sleep(21, 2));
        assert_eq!(MAX_ROOM, 40);
    }

    #[test]
    fn house_door_search_waits_until_the_floor_could_be_enough() {
        assert!(!floor_enough(4, 1, 2));
        assert!(floor_enough(6, 1, 2));
        assert!(!floor_enough(7, 1, 3));
        assert!(floor_enough(8, 1, 3));
    }
}
