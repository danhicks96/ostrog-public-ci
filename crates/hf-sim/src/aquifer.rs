//! Aquifers (#272, #267): water lies in the porous rock, sandstone and limestone, where the
//! layers fold down into a basin, in the levels resting on the middle stratum's granite, slate
//! or marble, which holds none and stops it sinking further. Granite, slate, marble and the
//! deep's rock are dry. A cut into wet rock lets the water in: the cut and the open tunnel
//! within three tiles of it fill shallow, on record. A world from before the strata keeps the
//! old seeded patches.
//!
//! Standing water (a lake, a stream, a flooded working) beside or over a cut breaks into it
//! (#390): it pours down the shaft and the workings joined to it, and the water it came from
//! gives up as much.

use std::collections::{BTreeMap, BTreeSet};

use crate::events::{EventKind, Severity};
use crate::ids::TilePos;
use crate::world::{Material, Shape, TileFlags};
use crate::World;

/// Whether the rock at `p` holds water.
pub fn wet(w: &World, p: TilePos) -> bool {
    if !w.grid.in_bounds(p) {
        return false;
    }
    held(w, p, w.grid.ground_z(p.x, p.y))
}

/// Whether the rock at `p` holds water, with the column's ground at `ground` (#433: a well's
/// shaft lowers its column's ground as it is dug, and the rock it cuts is reckoned from the
/// ground as it was).
pub fn held(w: &World, p: TilePos, ground: i32) -> bool {
    let g = &w.grid;
    if !g.in_bounds(p) {
        return false;
    }
    let depth = ground - p.z;
    // From two down (#405, FIXLOG 2026-10-27): the valley's own digging cuts one and two
    // levels down and never reached three, so no cut ever struck water. One down (a cellar, a
    // quarry's gallery) stays dry.
    if depth < 2 {
        return false;
    }
    let c = g.col(p.x, p.y);
    let Some(&(_, top, _)) = g.strata.get(c) else {
        return patch(w, p, depth);
    };
    let top = top as i32;
    // A basin: the break to the middle stratum lies lower here than across the valley.
    if top >= crate::world::gen::strata::top_break(g.z_min) || depth < g.soil_depth(c) {
        return false;
    }
    // Sandstone holds water through its lowest two levels, limestone in its lowest, in the
    // cracks the water has opened.
    match g.rock_at(c, p.z) {
        Material::SANDSTONE => p.z > top && p.z <= top + 2,
        Material::LIMESTONE => p.z == top + 1,
        _ => false,
    }
}

/// The seeded patches of a world from before the strata (#272): two to nine levels down,
/// in patches eight tiles across.
fn patch(w: &World, p: TilePos, depth: i32) -> bool {
    if !(2..=9).contains(&depth) {
        return false;
    }
    let h = (w.meta.seed
        ^ ((p.x / 8) as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ ((p.y / 8) as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F))
    .wrapping_mul(0xBF58_476D_1CE4_E5B9);
    (h >> 33).is_multiple_of(5)
}

/// The last tick's cuts: a cut into wet rock lets the water in.
pub fn observe(w: &mut World) {
    let tick = w.meta.tick;
    let cuts: Vec<(crate::ids::AgentId, TilePos)> = w
        .last_events
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::Mined { agent, at, .. } => Some((agent, at)),
            _ => None,
        })
        .collect();
    for (agent, at) in cuts {
        if w.grid.get(at).is_water() {
            continue;
        }
        let tiles = break_in(w, at);
        if tiles > 0 {
            w.events.push(
                tick,
                Some(at),
                Severity::Major,
                EventKind::WaterBrokeIn { agent, tiles },
            );
            continue;
        }
        // A well's shaft strikes water when the rock beside its cut holds it (#433, wells.rs):
        // the shaft's own column is open to the sky, so the fill below finds nothing to fill.
        if crate::wells::shaft_at(w, at) {
            if crate::wells::wet_beside(w, at) {
                let mut t = w.grid.get(at);
                t.flags = TileFlags(t.flags.0 | TileFlags::WATER);
                w.grid.set(at, t);
                w.events.push(
                    tick,
                    Some(at),
                    Severity::Major,
                    EventKind::StruckWater { agent, tiles: 1 },
                );
            }
            continue;
        }
        if !wet(w, at) {
            continue;
        }
        // Only the first cut into wet rock strikes water (#405, FIXLOG 2026-10-27): a cut
        // beside a dug tile that is itself in wet rock goes on into a water-bearing body the
        // workings already reached, and the water it gives is already in them. The cut is wet,
        // and nothing new is on record.
        let joined = [
            (1, 0, 0),
            (-1, 0, 0),
            (0, 1, 0),
            (0, -1, 0),
            (0, 0, 1),
            (0, 0, -1),
        ]
        .iter()
        .map(|(dx, dy, dz)| at.offset(*dx, *dy, *dz))
        .any(|q| w.dug.contains(&q) && wet(w, q));
        if joined {
            let mut t = w.grid.get(at);
            if !t.shape.is_solid() {
                t.flags = TileFlags(t.flags.0 | TileFlags::WATER);
                w.grid.set(at, t);
            }
            continue;
        }
        let mut filled = 0;
        for dz in [0, -1] {
            for dy in -3..=3 {
                for dx in -3..=3 {
                    let q = at.offset(dx, dy, dz);
                    if !w.grid.in_bounds(q) {
                        continue;
                    }
                    let mut t = w.grid.get(q);
                    if t.shape.is_solid() || t.is_water() || !crate::underground::below(w, q) {
                        continue;
                    }
                    t.flags = TileFlags(t.flags.0 | TileFlags::WATER);
                    w.grid.set(q, t);
                    filled += 1;
                }
            }
        }
        if filled > 0 {
            // The rock keeps giving: water to move through the workings (#323).
            w.seeping.push((at, 24));
            w.events.push(
                tick,
                Some(at),
                Severity::Major,
                EventKind::StruckWater {
                    agent,
                    tiles: filled,
                },
            );
        }
    }
}

/// The most water one break lets in, in tiles.
pub const POUR_MAX: usize = 400;
/// How far a body of water is followed to find its surface and shore.
const BODY_MAX: usize = 4096;

/// Whether standing water lies within `r` tiles of `p` (across, at its level, a level above,
/// or up to four below, where a lake's bed lies).
pub fn near_water(w: &World, p: TilePos, r: i32) -> bool {
    (-r..=r).any(|dy| {
        (-r..=r).any(|dx| {
            (p.z - 4..=p.z + 1).any(|z| {
                let q = TilePos::new(p.x + dx, p.y + dy, z);
                w.grid.in_bounds(q) && w.grid.get(q).is_water()
            })
        })
    })
}

/// The stone at `p` is damp: standing water lies within two tiles of it, at its level or the
/// one above, so a cut there is close to letting it in.
pub fn damp(w: &World, p: TilePos) -> bool {
    (-2..=2).any(|dy| {
        (-2..=2).any(|dx| {
            (0..=1).any(|dz| {
                let q = p.offset(dx, dy, dz);
                w.grid.in_bounds(q) && w.grid.get(q).is_water()
            })
        })
    })
}

/// #390: the stone of a cut is damp before the water breaks in. A cautious digger (the
/// `Cautious` trait) leaves the cut, and says why the first time; anyone else digs on.
/// Returns true when the cut is left.
pub fn left_damp(w: &mut World, i: usize, dix: usize, at: TilePos) -> bool {
    if !damp(w, at) || !w.agents[i].traits.has(crate::mind::Trait::Cautious) {
        return false;
    }
    let first = w.designations[dix].failures == 0;
    let d = &mut w.designations[dix];
    d.work_left = d.work_total;
    d.failures += 3;
    if first {
        let agent = w.agents[i].id;
        w.events.push(
            w.meta.tick,
            Some(at),
            Severity::Notable,
            EventKind::Chose {
                agent,
                what: "left a cut alone".into(),
                because: "the stone was damp, with water close behind it".into(),
            },
        );
    }
    true
}

/// #390: a cut opened beside standing water (a 4-neighbour at its level or the level above)
/// or under it lets the water in. It pours down into the cut and the open workings joined to
/// it, lowest first and no higher than the water's own surface, and the body it came from
/// gives up as many tiles, from its top level, shore first. Returns the tiles that filled.
pub fn break_in(w: &mut World, at: TilePos) -> u32 {
    let g = &w.grid;
    let src = [
        (0, 0, 1),
        (1, 0, 0),
        (-1, 0, 0),
        (0, 1, 0),
        (0, -1, 0),
        (1, 0, 1),
        (-1, 0, 1),
        (0, 1, 1),
        (0, -1, 1),
    ]
    .iter()
    .map(|(dx, dy, dz)| at.offset(*dx, *dy, *dz))
    .find(|q| g.in_bounds(*q) && g.get(*q).is_water());
    let Some(src) = src else {
        return 0;
    };
    const SIX: [(i32, i32, i32); 6] = [
        (1, 0, 0),
        (-1, 0, 0),
        (0, 1, 0),
        (0, -1, 0),
        (0, 0, 1),
        (0, 0, -1),
    ];
    // The body of water: every water tile joined to the source, as far as BODY_MAX.
    let mut body: Vec<TilePos> = vec![src];
    let mut seen: BTreeSet<TilePos> = BTreeSet::from([src]);
    let mut k = 0;
    while k < body.len() && body.len() < BODY_MAX {
        let p = body[k];
        k += 1;
        for (dx, dy, dz) in SIX {
            let q = p.offset(dx, dy, dz);
            if g.in_bounds(q) && !seen.contains(&q) && g.get(q).is_water() {
                seen.insert(q);
                body.push(q);
            }
        }
    }
    // Water already in the workings is not a new break-in (#405, FIXLOG 2026-10-27): a body
    // joined to a dug tile is water that broke in or was struck before, and a cut beside it
    // only lengthens the flooded working. Each cut along such a working was once recorded and
    // poured again.
    if body.iter().any(|p| w.dug.contains(p)) {
        return 0;
    }
    let surface = body.iter().map(|p| p.z).max().unwrap_or(src.z);
    // Where it can go: the cut, and the open space joined to it that is under the ground or
    // is a stair (a shaft's own column is open to the sky), no higher than the surface.
    let way_down = |q: TilePos| {
        let t = g.get(q);
        crate::underground::below(w, q)
            || matches!(
                t.shape,
                Shape::StairsUp | Shape::StairsDown | Shape::StairsBoth | Shape::Ladder
            )
    };
    let mut open: Vec<TilePos> = vec![at];
    let mut seen: BTreeSet<TilePos> = BTreeSet::from([at]);
    let mut k = 0;
    while k < open.len() && open.len() < BODY_MAX {
        let p = open[k];
        k += 1;
        for (dx, dy, dz) in SIX {
            let q = p.offset(dx, dy, dz);
            if q.z > surface || !g.in_bounds(q) || seen.contains(&q) {
                continue;
            }
            let t = g.get(q);
            if t.shape.is_solid() || t.is_water() || !way_down(q) {
                continue;
            }
            seen.insert(q);
            open.push(q);
        }
    }
    // Lowest first: water finds the bottom of the shaft before it rises in it.
    let order: BTreeMap<TilePos, usize> = open.iter().enumerate().map(|(i, p)| (*p, i)).collect();
    open.sort_by_key(|p| (p.z, order[p]));
    let n = open.len().min(body.len()).min(POUR_MAX);
    // Below the water's own surface a tile fills to its roof.
    for q in &open[..n] {
        let mut t = w.grid.get(*q);
        let full = if q.z < surface { TileFlags::FULL } else { 0 };
        t.flags = TileFlags(t.flags.0 | TileFlags::WATER | full);
        w.grid.set(*q, t);
    }
    // The source gives up as much: its top level first, the tiles with the most dry ground
    // beside them (the shore) before the middle.
    let mut give: Vec<TilePos> = body;
    give.sort_by_key(|p| {
        let wet_beside = [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .iter()
            .filter(|(dx, dy)| w.grid.get(p.offset(*dx, *dy, 0)).is_water())
            .count();
        (-p.z, wet_beside, *p)
    });
    for p in give.into_iter().take(n) {
        let t = w.grid.get(p);
        let dry = if t.material == Material::WATER {
            crate::world::Tile::AIR
        } else {
            crate::world::Tile {
                flags: t
                    .flags
                    .without(TileFlags::WATER | TileFlags::DEEP | TileFlags::FULL),
                ..t
            }
        };
        w.grid.set(p, dry);
        w.flooded.remove(&p);
    }
    n as u32
}

/// #323: water let in moves. Each hour every seep sends its water on: down into open space
/// under it first, else sideways into open floor beside it, a tile at a time, until the volume
/// it carries is spent.
pub fn flow(w: &mut World) {
    let tick = w.meta.tick;
    if tick % crate::time::TICKS_PER_HOUR != 123 || w.seeping.is_empty() {
        return;
    }
    let seeps = std::mem::take(&mut w.seeping);
    let mut keep = Vec::new();
    for (at, left) in seeps {
        if left == 0 {
            continue;
        }
        let open = |w: &World, q: TilePos| {
            w.grid.in_bounds(q) && !w.grid.get(q).shape.is_solid() && !w.grid.get(q).is_water()
        };
        let down = at.offset(0, 0, -1);
        let next = if open(w, down) {
            Some(down)
        } else {
            [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .map(|(dx, dy)| at.offset(*dx, *dy, 0))
                .find(|q| open(w, *q) && crate::underground::below(w, *q))
        };
        // Nowhere to go from here: it stands.
        if let Some(q) = next {
            let mut t = w.grid.get(q);
            t.flags = TileFlags(t.flags.0 | TileFlags::WATER);
            w.grid.set(q, t);
            keep.push((q, left - 1));
        }
    }
    w.seeping = keep;
}
