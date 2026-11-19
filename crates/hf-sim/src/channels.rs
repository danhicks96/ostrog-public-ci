//! Dug channels (#325, option W0 of docs/design-fluids.md): a community whose fields lie dry
//! but whose valley holds open water a short way off digs a ditch from the water to the field.
//! Water is not simulated along it: a channel is a recorded line of cut tiles that starts at the
//! shore, never runs uphill from where the water stands, and carries water as far as the cut
//! reaches without a gap. Crops within three tiles of a tile the water reaches drink as they do
//! beside a pond (`systems::plants::watered`, tests/irrigation.rs). No per-tick work: the route
//! is found once, when someone proposes it (systems/projects.rs `channel`), and the planner
//! here only posts the next few tiles as orders (`DesignationKind::DigChannel`) each planning
//! hour. A cut that meets something built or sown over is given up, and why is on record; the
//! community does not try again for a while.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::events::{EventKind, Severity};
use crate::ids::{CommunityId, TilePos};
use crate::project::{open_of, open_of_mut, ProjectKind};
use crate::time::TICKS_PER_DAY;
use crate::world::Shape;
use crate::World;

/// The longest cut a community will dig, in tiles.
pub const MAX_LEN: usize = 30;
/// How many tiles are ordered at once.
pub const AHEAD: usize = 3;
/// Days before a community that gave up (or found no route) looks again.
pub const AGAIN_DAYS: u64 = 20;
/// A route is wanted only when at least this many field tiles would drink from its end.
pub const MIN_COVER: usize = 6;
/// A field is dry when fewer than this share of its tiles (in percent) drink already.
pub const DRY_BELOW_PCT: usize = 50;
/// How far from home the water a channel starts at may lie.
pub const SOURCE_RADIUS: i32 = 45;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Channel {
    pub community: CommunityId,
    /// The tiles of the cut, from the shore to the field. Empty: a search that found no route
    /// (kept only to hold the community off for a while).
    pub tiles: Vec<TilePos>,
    /// Which tiles are cut.
    pub cut: Vec<bool>,
    /// Every tile is cut.
    pub done: bool,
    /// Given up before it was finished.
    pub gave_up: bool,
    /// When it was begun, and when it ended (finished or given up).
    pub begun: u64,
    pub ended: u64,
}

impl Channel {
    /// How many tiles from the shore are cut without a gap: the water reaches this far.
    pub fn reached(&self) -> usize {
        self.cut.iter().take_while(|c| **c).count()
    }

    fn open(&self) -> bool {
        !self.done && !self.gave_up
    }
}

/// Whether water carried by a channel reaches within three tiles of `p`, on its level or the
/// one below (the same reach as a pond's, plants.rs `watered`).
pub fn waters(w: &World, p: TilePos) -> bool {
    w.channels.iter().any(|c| {
        c.tiles[..c.reached()].iter().any(|t| {
            (t.x - p.x).abs() <= 3 && (t.y - p.y).abs() <= 3 && (t.z == p.z || t.z == p.z - 1)
        })
    })
}

/// A surface tile a ditch can be cut in: open, solid ground under it, nothing built, grown
/// or ordered on it, not water.
fn diggable(w: &World, q: TilePos, fields: &BTreeSet<TilePos>) -> bool {
    let g = &w.grid;
    g.in_bounds(q)
        && q.x >= 1
        && q.y >= 1
        && q.x < g.width - 1
        && q.y < g.height - 1
        && g.walkable(q)
        && !g.get(q).is_water()
        && g.get(q.offset(0, 0, -1)).shape == Shape::Wall
        && !fields.contains(&q)
        && !w.flooded.contains(&q)
        && !w.dug.contains(&q)
        && !w.structures.iter().any(|s| s.pos == q)
        && !w.designations.iter().any(|d| !d.done && d.pos == q)
        && !w.plants.iter().any(|p| p.alive && p.pos == q)
}

/// How many of the given (dry) field tiles would drink from water that reaches `end` (within
/// three tiles, same reach as `waters`).
fn cover(fields: &[TilePos], end: TilePos) -> usize {
    fields
        .iter()
        .filter(|f| {
            (f.x - end.x).abs() <= 3
                && (f.y - end.y).abs() <= 3
                && (end.z == f.z || end.z == f.z - 1)
        })
        .count()
}

/// Whether the field already drinks enough: a pond, a well-side stream or a channel within
/// reach of at least half its tiles.
pub fn field_is_dry(w: &World, cid: CommunityId) -> Option<Vec<TilePos>> {
    let c = w.communities.iter().find(|c| c.id == cid)?;
    if c.field_tiles.len() < MIN_COVER {
        return None;
    }
    let wet = c
        .field_tiles
        .iter()
        .filter(|f| crate::systems::plants::watered(w, **f))
        .count();
    (wet * 100 < c.field_tiles.len() * DRY_BELOW_PCT).then(|| c.field_tiles.clone())
}

/// The shortest cut from the shore of water within reach of home to ground where at least
/// `MIN_COVER` field tiles that are dry now would drink, never running uphill (a tile no higher than the one
/// before it, and no more than one lower), reachable on foot. Tiles from the shore to the
/// field. Deterministic: tiles are visited in order.
pub fn route(w: &World, cid: CommunityId) -> Option<Vec<TilePos>> {
    let c = w.communities.iter().find(|c| c.id == cid)?;
    let nav = w.nav.as_ref()?;
    let home = c.home;
    let field_set: BTreeSet<TilePos> = c.field_tiles.iter().copied().collect();
    // Only tiles that do not drink already count towards what a cut is worth.
    let fields: Vec<TilePos> = c
        .field_tiles
        .iter()
        .copied()
        .filter(|f| !crate::systems::plants::watered(w, *f))
        .collect();
    let need = MIN_COVER.min(fields.len());
    if need == 0 {
        return None;
    }
    // Shores: the open ground beside open water (a surveyed body's, not flood water or the
    // workings'), no higher than a step above the water's surface.
    let mut starts: BTreeSet<TilePos> = BTreeSet::new();
    for b in &w.waters.bodies {
        for q in b.tiles.iter() {
            if q.dist(home) > SOURCE_RADIUS
                || !w.grid.get(*q).is_water()
                || w.flooded.contains(q)
                || w.waters.left.contains_key(q)
                || w.dug.contains(q)
            {
                continue;
            }
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let s = w.grid.surface(q.x + dx, q.y + dy);
                if s.z <= q.z + 1 && diggable(w, s, &field_set) && nav.reach(home, s) {
                    starts.insert(s);
                }
            }
        }
    }
    let mut from: BTreeMap<TilePos, Option<TilePos>> = BTreeMap::new();
    let mut queue: VecDeque<(TilePos, usize)> = VecDeque::new();
    for s in starts {
        from.insert(s, None);
        queue.push_back((s, 1));
    }
    while let Some((cur, len)) = queue.pop_front() {
        if cover(&fields, cur) >= need {
            let mut path = vec![cur];
            let mut at = cur;
            while let Some(Some(prev)) = from.get(&at) {
                path.push(*prev);
                at = *prev;
            }
            path.reverse();
            return Some(path);
        }
        if len >= MAX_LEN {
            continue;
        }
        for (dx, dy) in [(1, 0), (0, 1), (-1, 0), (0, -1)] {
            let n = w.grid.surface(cur.x + dx, cur.y + dy);
            if from.contains_key(&n) || n.z > cur.z || cur.z - n.z > 1 {
                continue;
            }
            if !diggable(w, n, &field_set) || !nav.reach(home, n) {
                continue;
            }
            from.insert(n, Some(cur));
            queue.push_back((n, len + 1));
        }
    }
    None
}

/// Starts a record for a community: the route found, or an empty search that holds it off.
pub fn begin(w: &mut World, cid: CommunityId, tiles: Vec<TilePos>) {
    let tick = w.meta.tick;
    // An old empty search is replaced; finished and abandoned cuts stay as the record.
    w.channels
        .retain(|c| !(c.community == cid && c.tiles.is_empty()));
    let none = tiles.is_empty();
    let n = tiles.len();
    w.channels.push(Channel {
        community: cid,
        tiles,
        cut: vec![false; n],
        done: false,
        gave_up: none,
        begun: tick,
        ended: if none { tick } else { 0 },
    });
}

/// Whether the community may look for a channel now: nothing open, nothing recent that ended
/// without water, and no more than two cut for it.
pub fn may_try(w: &World, cid: CommunityId) -> bool {
    let tick = w.meta.tick;
    let ours = || w.channels.iter().filter(|c| c.community == cid);
    ours().all(|c| !c.open())
        && ours().filter(|c| c.done).count() < 2
        && ours()
            .filter(|c| c.gave_up)
            .all(|c| tick.saturating_sub(c.ended) >= AGAIN_DAYS * TICKS_PER_DAY)
}

/// The planner's part (systems/settlement.rs), each planning hour: order the next tiles of the
/// open cut, and end it when it is whole or can go no further.
pub fn plan(w: &mut World, cid: CommunityId, _home: TilePos) {
    let Some(k) = w
        .channels
        .iter()
        .position(|c| c.community == cid && c.open())
    else {
        return;
    };
    let tick = w.meta.tick;
    let by = open_of(w, cid, |k| *k == ProjectKind::Channel).map(|p| p.by);
    let n = w.channels[k].tiles.len();
    if w.channels[k].cut.iter().all(|c| *c) {
        w.channels[k].done = true;
        w.channels[k].ended = tick;
        finish(
            w,
            cid,
            by,
            format!("the water runs {n} paces to the field"),
            true,
        );
        return;
    }
    let todo: Vec<TilePos> = (0..n)
        .filter(|i| !w.channels[k].cut[*i])
        .take(AHEAD)
        .map(|i| w.channels[k].tiles[i])
        .collect();
    for q in todo {
        let ordered = w.designations.iter().any(|d| !d.done && d.pos == q);
        if ordered {
            continue;
        }
        // The tile was free when the route was found; if it is not now, the cut stops here.
        let blocked = !w.grid.in_bounds(q)
            || !w.grid.walkable(q)
            || w.grid.get(q).is_water()
            || w.structures.iter().any(|s| s.pos == q);
        if blocked {
            w.channels[k].gave_up = true;
            w.channels[k].ended = tick;
            finish(
                w,
                cid,
                by,
                "the way was built over before the ditch was cut".into(),
                false,
            );
            return;
        }
        let d = crate::systems::settlement::make(w, crate::work::DesignationKind::DigChannel, q, 3);
        w.designations.push(d);
    }
}

/// The hook from the work system when the tile at `pos` has been dug: record the cut.
pub fn dug(w: &mut World, pos: TilePos) {
    for c in w.channels.iter_mut().filter(|c| c.open()) {
        if let Some(i) = c.tiles.iter().position(|t| *t == pos) {
            c.cut[i] = true;
        }
    }
}

fn finish(w: &mut World, cid: CommunityId, by: Option<crate::ids::AgentId>, why: String, ok: bool) {
    let tick = w.meta.tick;
    if let Some(p) = open_of_mut(w, cid, |k| *k == ProjectKind::Channel) {
        p.done = true;
    }
    let at = w.communities.iter().find(|c| c.id == cid).map(|c| c.home);
    if let Some(agent) = by {
        w.events.push(
            tick,
            at,
            Severity::Notable,
            EventKind::Chose {
                agent,
                what: if ok {
                    "finished the channel".into()
                } else {
                    "gave up on the channel".into()
                },
                because: why,
            },
        );
    }
}
