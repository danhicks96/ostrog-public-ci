//! Someone cut off below (#439): their people dig a way down to them. On seed 4 a fire took a
//! floor and three people fell through it into a closed cave seven levels down; nothing joined
//! the cave to anywhere, so nobody could go down to them and they could not come up. They lived
//! there four weeks on what grew in the dark and died of a fever, untended. A normal person
//! whose family is down a hole digs down to them.
//!
//! Each hour, for each person of a community below ground whom no way joins to their home, a
//! column of rock beside them that runs whole up to open ground their people can reach is cut
//! into a stair, a level at a time from the top (`DigStairs`, as a quarry shaft is dug). The
//! column is found again each hour by the same rule, so nothing needs keeping between hours.
//!
//! Someone cut off a storey up (#447) climbs down: on seed 3 a builder finishing a wall from the
//! tile it stood on was put off it on the far side, onto a ledge a level above the ground round
//! it that the wall had been the only way on and off, and starved there in sight of home.

use crate::agents::Species;
use crate::ids::TilePos;
use crate::time::TICKS_PER_HOUR;
use crate::work::DesignationKind;
use crate::world::Shape;
use crate::world_state::World;

/// How far round the one cut off a column for the way down is looked for, in tiles.
pub const REACH: i32 = 4;
/// The priority of the work: above building, as a fire's is.
pub const PRIORITY: u8 = 8;

pub fn step(w: &mut World) {
    if w.meta.tick % TICKS_PER_HOUR != 900 {
        return;
    }
    w.ensure_nav();
    let mut cut_off: Vec<(usize, TilePos, crate::ids::CommunityId, TilePos, bool)> = Vec::new();
    {
        let Some(nav) = w.nav.as_ref() else { return };
        for (i, a) in w.agents.iter().enumerate() {
            if !a.alive
                || a.species != Species::Person
                || a.leaving
                || a.band.is_some()
                || a.held.is_some()
            {
                continue;
            }
            let Some(cid) = a.community else { continue };
            let Some(c) = w.communities.iter().find(|c| c.id == cid && !c.dissolved) else {
                continue;
            };
            if !w.grid.walkable(a.pos) || nav.reach(a.pos, c.home) {
                continue;
            }
            // #523 (FIXLOG 2026-10-31, c138): a wholly stranded local group can still climb
            // itself down, even mid-step on a short walk; digging still requires someone home.
            let home_reachable = w
                .agents
                .iter()
                .any(|o| o.alive && o.community == Some(cid) && nav.reach(o.pos, c.home));
            cut_off.push((i, a.pos, cid, c.home, home_reachable));
        }
    }
    for (i, at, cid, home, home_reachable) in cut_off {
        if climb_down(w, i, at, home) || !home_reachable || !crate::underground::below(w, at) {
            continue;
        }
        let Some((x, y, top)) = shaft_column(w, at, home) else {
            continue;
        };
        // The highest rock left in the column is the next to cut; the stair is dug down from
        // the level above it.
        let next = (at.z..=top)
            .rev()
            .map(|z| TilePos::new(x, y, z))
            .find(|p| w.grid.get(*p).shape == Shape::Wall);
        let Some(next) = next else { continue };
        if w.designations.iter().any(|d| d.pos == next && !d.done) {
            continue;
        }
        let mut d = crate::systems::settlement::make(w, DesignationKind::DigStairs, next, PRIORITY);
        d.community = Some(cid);
        w.designations.push(d);
    }
}

/// #447: someone a storey above ground that leads home, with no way down, climbs down to it:
/// a bank or a wall's height is let down from by the hands, unhurt. Returns whether they did.
fn climb_down(w: &mut World, i: usize, at: TilePos, home: TilePos) -> bool {
    let Some(nav) = w.nav.as_ref() else {
        return false;
    };
    let to = [
        (0, 1),
        (1, 0),
        (0, -1),
        (-1, 0),
        (1, 1),
        (-1, 1),
        (1, -1),
        (-1, -1),
    ]
    .iter()
    .map(|(dx, dy)| at.offset(*dx, *dy, -1))
    .find(|q| {
        // Open air over it at their own level, so the way down is clear.
        !w.grid.get(q.offset(0, 0, 1)).is_solid()
            && w.grid.walkable(*q)
            && !w.grid.over_head(*q)
            && nav.reach(*q, home)
    });
    let Some(to) = to else {
        return false;
    };
    let tick = w.meta.tick;
    let a = &mut w.agents[i];
    a.pos = to;
    a.from = to;
    a.path.clear();
    a.move_total = 0;
    a.move_ticks = 0;
    a.activity = crate::mind::Activity::Idle;
    let id = a.id;
    w.events.push(
        tick,
        Some(to),
        crate::events::Severity::Notable,
        crate::events::EventKind::Chose {
            agent: id,
            what: "climbed down".into(),
            because: "there was no other way home".into(),
        },
    );
    true
}

/// A column beside `at` (within `REACH`) whose levels from `at`'s up are rock or a stair already
/// cut, standing on rock, with open ground or the stair's head on top that `home` reaches: the
/// nearest such, the same one each hour. Returns it and the highest level of its run.
fn shaft_column(w: &World, at: TilePos, home: TilePos) -> Option<(i32, i32, i32)> {
    let nav = w.nav.as_ref()?;
    let g = &w.grid;
    let stair = |s: Shape| matches!(s, Shape::StairsUp | Shape::StairsDown | Shape::StairsBoth);
    // (distance, x, y) to choose by, and the column with the top of its run.
    type Pick = ((i32, i32, i32), (i32, i32, i32));
    let mut best: Option<Pick> = None;
    for dy in -REACH..=REACH {
        for dx in -REACH..=REACH {
            let (x, y) = (at.x + dx, at.y + dy);
            if x < 1 || y < 1 || x >= g.width - 1 || y >= g.height - 1 {
                continue;
            }
            // The foot of the stair stands beside someone's pocket: next to a tile of it.
            let foot = TilePos::new(x, y, at.z);
            let beside = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(ex, ey)| {
                let q = foot.offset(*ex, *ey, 0);
                g.walkable(q) && nav.reach(at, q)
            });
            if !beside || g.get(foot.offset(0, 0, -1)).shape != Shape::Wall {
                continue;
            }
            // The run of rock and stair up from the foot, and what is on top of it.
            let mut z = at.z;
            while z < g.z_max && {
                let s = g.get(TilePos::new(x, y, z)).shape;
                s == Shape::Wall || stair(s)
            } {
                z += 1;
            }
            if z == at.z {
                continue;
            }
            let top = z - 1;
            let head = if stair(g.get(TilePos::new(x, y, top)).shape) {
                TilePos::new(x, y, top)
            } else {
                TilePos::new(x, y, z)
            };
            if !g.walkable(head) || !nav.reach(head, home) {
                continue;
            }
            let key = (dx.abs().max(dy.abs()), x, y);
            if best.is_none_or(|(k, _)| key < k) {
                best = Some((key, (x, y, top)));
            }
        }
    }
    best.map(|b| b.1)
}
