//! Agents follow their paths. Each tile step takes its nav cost in ticks (docs/TIME.md), and
//! the renderer interpolates between `from` and `pos` using `move_ticks / move_total`.

use crate::agents::Agent;
use crate::nav::step_cost;
use crate::world_state::World;

/// How a step related to water (OST-1305): read by `examples/crossing_census.rs` and its test;
/// the simulation itself does not call it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Wade {
    /// Stepped from dry ground (or another level) onto a water tile.
    Entered,
    /// Stepped from one water tile onto another.
    Along,
}

/// Whether a person who was on `before` and is now on `after` has just waded: the position
/// changed and the new tile has water on it. Stepping out onto dry ground is not a wade.
pub fn wading_kind(
    before: crate::ids::TilePos,
    after: crate::ids::TilePos,
    grid: &crate::world::WorldGrid,
) -> Option<Wade> {
    if before == after || !grid.get(after).is_water() {
        return None;
    }
    Some(if grid.get(before).is_water() {
        Wade::Along
    } else {
        Wade::Entered
    })
}

/// Any wading step, entered or along.
pub fn wading_step(
    before: crate::ids::TilePos,
    after: crate::ids::TilePos,
    grid: &crate::world::WorldGrid,
) -> bool {
    wading_kind(before, after, grid).is_some()
}

pub fn step(w: &mut World) {
    let mut trodden: Vec<(i32, i32)> = Vec::new();
    step_inner(w, &mut trodden);
    // #84: footsteps on open ground, counted toward a path.
    for k in trodden {
        let e = w.wear.entry(k).or_insert(0);
        *e = e.saturating_add(1);
    }
}

fn step_inner(w: &mut World, trodden: &mut Vec<(i32, i32)>) {
    let tick_now = w.meta.tick;
    let seed = w.meta.seed;
    let grid = &w.grid;
    // (#324) Falls this tick: (agent index, levels fallen, into water, cause).
    let mut falls: Vec<(usize, i32, bool, &'static str)> = Vec::new();
    let mut footholds = Vec::new();
    for (ai, a) in w.agents.iter_mut().enumerate() {
        if !a.alive {
            continue;
        }
        // Record brief immersion even between the less frequent drowning checks.
        // This loop already visits every moving or stationary agent each tick.
        let tile = grid.get(a.pos);
        if a.species == crate::agents::Species::Person && grid.over_head_with_tile(a.pos, tile) {
            a.wet = 1.0;
        }
        // Terrain may change mid-stride. Replace the obsolete interpolation and pay
        // for the descent to the actual surface even if the person is asleep or hurt.
        if grid.air_over_liquid(a.pos) {
            a.from = a.pos;
            a.pos = a.pos.offset(0, 0, -1);
            a.move_total = crate::time::LEVEL_CHANGE_TICKS;
            a.move_ticks = 1;
            a.path.clear();
            a.path_blocked = true;
            continue;
        }
        let exposed_deep = tile.liquid_floor()
            && tile.flags.has(crate::world::TileFlags::DEEP)
            && !grid.get(a.pos.offset(0, 0, 1)).is_water();
        if exposed_deep || grid.unsupported_water_surface(a.pos, tile) {
            if a.is_moving() {
                a.move_ticks += 1;
            } else if !a.activity.is_asleep() && a.held.is_none() {
                // Only an immediately reachable foothold. The ordinary unwalkable-tile
                // recovery below would sink them another storey or move through a wall.
                if let Some(q) = adjacent_foothold(grid, a.pos) {
                    let diagonal = q.x != a.pos.x && q.y != a.pos.y;
                    let cost = step_cost(grid, a.pos, diagonal).max(step_cost(grid, q, diagonal));
                    let cost = if a.injured_until > tick_now {
                        cost * 2
                    } else {
                        cost
                    };
                    footholds.push((ai, q, cost));
                }
            }
            continue;
        }
        // Safety net: if the ground was built or dug away under someone, step to the nearest
        // walkable neighbour rather than being stuck inside a wall forever.
        if !a.is_moving() && !grid.walkable(a.pos) {
            // #324: open air under them (the floor dug or burned away): they fall to whatever
            // they can stand on below, and are hurt by the height. Only down through air or
            // water, never through rock (#444, FIXLOG 2026-10-28: on seed 4 a grass fire made
            // three people's tile unwalkable and they "fell" through six levels of soil and
            // slate into a closed cave). Standing on ground, it is the step aside below.
            if !grid.get(a.pos).shape.is_solid() && !grid.get(a.pos.offset(0, 0, -1)).is_solid() {
                let mut water = false;
                let mut land = None;
                for d in 1..=12 {
                    let q = a.pos.offset(0, 0, -d);
                    let t = grid.get(q);
                    if t.is_solid() {
                        break;
                    }
                    water |= t.is_water();
                    // Deep water takes them in; they swim out from there.
                    if grid.walkable(q) || t.flags.has(crate::world::TileFlags::DEEP) {
                        land = Some(d);
                        break;
                    }
                }
                if let Some(d) = land {
                    a.pos = a.pos.offset(0, 0, -d);
                    a.from = a.pos;
                    a.path.clear();
                    a.path_blocked = true;
                    falls.push((ai, d, water, "fell through the floor"));
                    continue;
                }
            }
            let mut found = None;
            'search: for r in 1..=2 {
                for dy in -r..=r {
                    for dx in -r..=r {
                        for dz in [0, -1, 1] {
                            let q = a.pos.offset(dx, dy, dz);
                            if grid.walkable(q) {
                                found = Some(q);
                                break 'search;
                            }
                        }
                    }
                }
            }
            if let Some(q) = found {
                a.pos = q;
                a.from = q;
                a.path.clear();
                a.path_blocked = true;
            }
            continue;
        }
        if a.is_moving() {
            a.move_ticks += 1;
            continue;
        }
        // Arrived at `pos`; take the next tile if any.
        let Some(next) = a.path.last().copied() else {
            a.move_total = 0;
            a.move_ticks = 0;
            a.from = a.pos;
            continue;
        };
        let adjacent = (next.x - a.pos.x).abs() <= 1
            && (next.y - a.pos.y).abs() <= 1
            && (next.z - a.pos.z).abs() <= 1;
        // Water risen over the head on the way ahead stops someone still on dry ground (#418).
        let into_flood = grid.over_head(next) && !grid.over_head(a.pos);
        let diagonal = next.x != a.pos.x && next.y != a.pos.y;
        // A queued diagonal from an old save may still have legal endpoints after
        // losing the flanking tile above a pond. Match the pathfinder's corner rule.
        let cuts_water_corner = diagonal
            && next.z == a.pos.z
            && [
                crate::ids::TilePos::new(next.x, a.pos.y, next.z),
                crate::ids::TilePos::new(a.pos.x, next.y, next.z),
            ]
            .into_iter()
            .any(|q| grid.air_over_liquid(q) || grid.unsupported_water_surface(q, grid.get(q)));
        if !grid.walkable(next) || !adjacent || into_flood || cuts_water_corner {
            a.path.clear();
            a.path_blocked = true;
            a.move_total = 0;
            a.from = a.pos;
            continue;
        }
        a.path.pop();
        let mut cost = if next.z != a.pos.z {
            crate::time::LEVEL_CHANGE_TICKS
        } else {
            step_cost(grid, next, diagonal)
        };
        if a.injured_until > tick_now {
            cost *= 2;
        }
        // #324: going down a level, now and then someone slips; more often when hurt.
        if next.z < a.pos.z && a.species == crate::agents::Species::Person {
            use rand::Rng;
            let mut r = crate::rng::stream(
                seed,
                &[
                    crate::rng::purpose::AGENT_WORK,
                    a.id.0 as u64,
                    tick_now,
                    0xFA11,
                ],
            );
            let p = if a.injured_until > tick_now {
                1e-4
            } else {
                3e-5
            };
            if r.gen_bool(p) {
                falls.push((ai, 1, false, "slipped going down and fell"));
            }
        }
        a.facing = Agent::facing_toward(a.pos, next);
        a.from = a.pos;
        a.pos = next;
        if a.species == crate::agents::Species::Person
            && next.z == grid.ground_z(next.x, next.y) + 1
        {
            trodden.push((next.x, next.y));
        }
        a.move_total = cost.max(1);
        a.move_ticks = 1;
    }
    for (i, q, cost) in footholds {
        crate::systems::hazards::attempt_foothold(w, i, q);
        let a = &mut w.agents[i];
        a.move_total = cost;
        a.move_ticks = 1;
        a.facing = Agent::facing_toward(a.pos, q);
        a.from = a.pos;
        a.pos = q;
        a.path.clear();
        a.path_blocked = false;
    }
    for (i, levels, water, cause) in falls {
        fall(w, i, levels, water, cause);
    }
}

/// Emergency placement spans at most one edge. A two-layer water column with no
/// DEEP flag is open to reach across, but is not a bed-supported place to stand.
fn adjacent_foothold(
    grid: &crate::world::WorldGrid,
    p: crate::ids::TilePos,
) -> Option<crate::ids::TilePos> {
    use crate::world::Shape;
    let side_open = |q| {
        let tile = grid.get(q);
        // A person can reach past open surface water even where they could not stand.
        let clear = if tile.liquid_floor() {
            !tile
                .flags
                .has(crate::world::TileFlags::DEEP | crate::world::TileFlags::FIRE)
        } else {
            tile.shape == Shape::Open && grid.walkable(q)
        };
        clear && !tile.flags.has(crate::world::TileFlags::DOOR) && !grid.over_head(q)
    };
    let cardinal = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    let diagonal = [(-1, -1), (1, -1), (1, 1), (-1, 1)];
    cardinal
        .into_iter()
        .map(|(dx, dy)| p.offset(dx, dy, 0))
        .filter(|&q| grid.water_foothold(q))
        .min_by_key(|&q| grid.get(q).is_water())
        .or_else(|| {
            diagonal
                .into_iter()
                .filter(|&(dx, dy)| side_open(p.offset(dx, 0, 0)) && side_open(p.offset(0, dy, 0)))
                .map(|(dx, dy)| p.offset(dx, dy, 0))
                .find(|&q| grid.water_foothold(q))
        })
}

/// #324, #444: a fall of `levels` storeys, hurting steeply by the height: one level bruises or
/// sprains, two or three can break bones and now and then kill, four or more most likely kill.
/// Deep water under them takes the first three levels of it. Death is recorded as "fell".
pub fn fall(w: &mut World, i: usize, levels: i32, into_water: bool, cause: &str) {
    use crate::body::{Part, WoundKind};
    use rand::Rng;
    if w.agents[i].species != crate::agents::Species::Person {
        return;
    }
    let h = if into_water {
        (levels - WATER_LEVELS).max(0)
    } else {
        levels
    };
    if h == 0 {
        return;
    }
    let mut r = crate::rng::stream(
        w.meta.seed,
        &[
            crate::rng::purpose::AGENT_WORK,
            w.agents[i].id.0 as u64,
            w.meta.tick,
            0xFA12,
        ],
    );
    let death = FALL_DEATH[(h as usize).min(FALL_DEATH.len() - 1)];
    if r.gen::<f32>() < death {
        let id = w.agents[i].id;
        crate::systems::life::die(w, id, "fell".into());
        return;
    }
    // Survived: a bruise or a sprain from a storey; from higher, breaks, more than one from
    // four or more, and bleeding inside.
    let wounds = if h >= 4 { 2 } else { 1 };
    for k in 0..wounds {
        let sev = (0.1 + 0.15 * h as f32 + r.gen::<f32>() * 0.15).min(1.0);
        let kind = if h >= 2 && r.gen::<f32>() < 0.3 + 0.2 * h as f32 {
            WoundKind::Break
        } else {
            WoundKind::Bruise
        };
        let part = if k == 0 && h == 1 {
            [Part::LEG_L, Part::LEG_R][r.gen_range(0..2)]
        } else {
            [
                Part::LEG_L,
                Part::LEG_R,
                Part::ARM_L,
                Part::ARM_R,
                Part::HEAD,
            ][r.gen_range(0..5)]
        };
        let bleeding = if h >= 4 { 0.3 } else { 0.0 };
        crate::systems::health::wound(w, i, part, kind, sev, bleeding, cause, None);
    }
}

/// The chance a fall kills, by the storeys fallen (0 to 6 and more).
pub const FALL_DEATH: [f32; 7] = [0.0, 0.0, 0.05, 0.25, 0.6, 0.8, 0.95];
/// Storeys of a fall that deep water under them takes.
pub const WATER_LEVELS: i32 = 3;
