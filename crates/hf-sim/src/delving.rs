//! Delving (#405, #274): the mine a community sinks below its quarry when what it wants lies
//! deeper than the quarry's two levels.
//!
//! Nobody digs for its own sake. A community sinks its shaft for a reason it has: ore for a
//! smelter it can work and no vein showing within reach, tin for bronze, silver or gold to set a
//! gem in, harder stone than any it has cut when strangers threaten its walls, and, for a people
//! who hold that the deep is there to be dug (DeepDigging), the gems and gold that lie only in
//! the deep. The shaft goes down a level at a time from the quarry's shaft (the planner's
//! `quarry_site`); at each level the miners look along the rock for what they want, and when it
//! shows they drive a gallery to it and follow it. With nothing found they go a level deeper,
//! and at the bottom of the world they drive galleries out along the rock. Rock that rings
//! hollow (a cavern within earshot of the workings) draws a gallery to it, unless the community
//! holds the deep is not to be dug.
//!
//! Depth costs: every cut takes longer the deeper it is (the spoil goes up further), harder rock
//! takes longer again (tiers.rs), the dark slows the work until someone sets a lamp
//! (systems/work.rs), wet rock lets water in (aquifer.rs) and a gallery left unheld in soft rock
//! comes down (underground.rs). A gallery that opens into a deep cavern is looked into: the one
//! who cut it goes in, and the breach and all that follows it is underground.rs's. Then the
//! community chooses: a cautious one walls the way (`sealed` once the wall stands), a greedy one
//! leaves it open and digs on (`deep_chosen` says which).
//!
//! Water (#433) is a reason a shaft could be sunk for (`Seek::Water`); its wells are aquifer.rs's
//! and nothing here chooses it.

use serde::{Deserialize, Serialize};

use crate::events::{EventKind, Severity};
use crate::ids::{CommunityId, TilePos};
use crate::items::ItemKind;
use crate::mind::beliefs::Topic;
use crate::systems::settlement::{self, Here};
use crate::work::DesignationKind;
use crate::world::{Material, Shape};
use crate::World;

/// How far along the rock of a level the miners see what it holds: a crosscut each way from
/// the workings, without cutting it tile by tile.
pub const LOOK: i32 = 6;
/// How far off a cavern is heard through the rock (drip, a draught).
pub const HEAR: i32 = 6;
/// How far a gallery is driven out from the shaft's foot at the bottom with nothing found.
pub const DRIVE: i32 = 24;
/// How long a worked-out mine is left before it is looked at again.
const REST_DAYS: u64 = 60;

/// What a mine is sunk for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Seek {
    /// Iron or copper for the smelter.
    Ore,
    /// Tin, for bronze.
    Tin,
    /// Silver or gold, to set a gem in.
    Precious,
    /// Harder stone than any cut yet, for walls against strangers.
    HardStone,
    /// The deep's gems and gold, by those who hold the deep is there to be dug.
    Deep,
    /// Water (#433): a hook only, never chosen here.
    Water,
}

impl Seek {
    pub fn why(self) -> &'static str {
        match self {
            Seek::Ore => "iron and copper for the smelter",
            Seek::Tin => "tin to make bronze",
            Seek::Precious => "silver or gold to set a gem in",
            Seek::HardStone => "harder stone for their walls",
            Seek::Deep => "the gems and gold of the deep",
            Seek::Water => "water",
        }
    }

    /// The rock that holds what is sought, given the hardest stone cut so far.
    fn mats(self, w: &World, hardest: f32) -> Vec<Material> {
        match self {
            Seek::Ore => vec![Material::IRON_ORE, Material::COPPER_ORE],
            Seek::Tin => vec![Material::TIN_ORE],
            Seek::Precious => vec![Material::SILVER_ORE, Material::GOLD_ORE],
            Seek::Deep => vec![Material::GEM_ROCK, Material::GOLD_ORE],
            Seek::HardStone => [
                Material::GRANITE,
                Material::SLATE,
                Material::MARBLE,
                Material::BASALT,
                Material::GABBRO,
                Material::GNEISS,
            ]
            .into_iter()
            .filter(|m| w.grid.materials.get(*m).hardness > hardest)
            .collect(),
            Seek::Water => vec![],
        }
    }
}

/// A community's mine: the shaft below its quarry and the galleries cut from it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mine {
    /// The surface tile over the shaft (the quarry's site).
    pub site: TilePos,
    /// The lowest walkable tile of the workings the shaft has reached, where it goes on down.
    pub foot: TilePos,
    /// What it was last dug for.
    pub seek: Option<Seek>,
    /// Every tile it ordered cut.
    pub cut: Vec<TilePos>,
    /// Its shaft has come down into the deep's rock.
    pub deep: bool,
    /// The level where harder stone was found, cut there by the quarry from then on.
    pub stone_level: Option<i32>,
    /// Worked out: nothing more is looked for until then.
    pub rest_until: u64,
    /// A deep cavern a gallery opened into and nobody has yet gone into (index).
    pub look: Option<usize>,
    /// After a breach: walled (true) or dug on (false).
    pub chose: Option<bool>,
}

/// The level the quarry cuts its stone at, once the mine found harder stone (#405).
pub fn stone_level(w: &World, cid: CommunityId) -> Option<i32> {
    w.communities
        .iter()
        .find(|c| c.id == cid)
        .and_then(|c| c.mine.as_ref())
        .and_then(|m| m.stone_level)
}

/// How a community's grown people lean on whether the deep is to be dug: the mean of their
/// DeepDigging stances, a cautious temper counting as a lean against.
pub fn stance(w: &World, cid: CommunityId) -> f32 {
    let tick = w.meta.tick;
    let (sum, n) = w
        .agents
        .iter()
        .filter(|a| {
            a.alive
                && a.community == Some(cid)
                && a.species == crate::agents::Species::Person
                && !a.is_child(tick)
        })
        .fold((0.0f32, 0u32), |(s, n), a| {
            let cautious = if a.traits.has(crate::mind::Trait::Cautious) {
                -0.3
            } else {
                0.0
            };
            (s + a.beliefs.lean(Topic::DeepDigging) + cautious, n + 1)
        });
    if n == 0 {
        0.0
    } else {
        sum / n as f32
    }
}

fn knows(w: &World, cid: CommunityId, t: crate::mind::Technique) -> bool {
    w.agents
        .iter()
        .any(|a| a.alive && a.community == Some(cid) && a.techniques.knows(t))
}

/// Why the community would dig deeper now, if it would: the first reason that holds.
fn wanted(w: &World, cid: CommunityId, home: TilePos, here: &Here, people: u32) -> Option<Seek> {
    if people < 8 {
        return None;
    }
    let stock = |k: ItemKind| settlement::count_kind(w, here, k);
    let of = |k: ItemKind, m: Material| settlement::count_of(w, here, k, m);
    // Nothing showing within the planner's reach (settlement.rs looks there first).
    let none_showing = |mats: &[Material]| settlement::nearest_face_of(w, home, 50, mats).is_none();
    // For a smelter that stands, worked by someone who knows how: not the day they arrive.
    let smelter = w.structures.iter().any(|s| {
        s.kind == crate::structures::StructureKind::Smelter
            && s.progress >= 1.0
            && here.holds(s.pos)
    });
    if stock(ItemKind::IronOre) + stock(ItemKind::CopperOre) < 4
        && smelter
        && knows(w, cid, crate::mind::Technique::Smelting)
        && none_showing(&[Material::IRON_ORE, Material::COPPER_ORE])
    {
        return Some(Seek::Ore);
    }
    if stock(ItemKind::CopperBar) >= 2
        && of(ItemKind::Ore, Material::TIN_ORE) + of(ItemKind::Bar, Material::TIN) < 2
        && none_showing(&[Material::TIN_ORE])
    {
        return Some(Seek::Tin);
    }
    if stock(ItemKind::Gem) >= 1
        && of(ItemKind::Bar, Material::SILVER)
            + of(ItemKind::Bar, Material::GOLD)
            + of(ItemKind::Ore, Material::SILVER_ORE)
            + of(ItemKind::Ore, Material::GOLD_ORE)
            == 0
        && none_showing(&[Material::SILVER_ORE, Material::GOLD_ORE])
    {
        return Some(Seek::Precious);
    }
    let c = w.communities.iter().find(|c| c.id == cid)?;
    let threatened = c.threatened > 0
        && w.meta.tick.saturating_sub(c.threatened) < 40 * crate::time::TICKS_PER_DAY;
    if threatened && !c.stones.is_empty() && hardest(w, cid) < 1.5 {
        return Some(Seek::HardStone);
    }
    // Settled a year first: the deep is not what a people sets about on arriving.
    let settled = w.meta.tick.saturating_sub(c.founded) >= crate::time::TICKS_PER_YEAR;
    if settled
        && stance(w, cid) >= 0.2
        && stock(ItemKind::Gem) < 4
        && none_showing(&[Material::GEM_ROCK, Material::GOLD_ORE])
    {
        return Some(Seek::Deep);
    }
    None
}

fn hardest(w: &World, cid: CommunityId) -> f32 {
    w.communities
        .iter()
        .find(|c| c.id == cid)
        .map(|c| {
            c.stones
                .iter()
                .map(|m| w.grid.materials.get(*m).hardness)
                .fold(0.0, f32::max)
        })
        .unwrap_or(0.0)
}

fn mine_of(w: &mut World, cid: CommunityId) -> Option<&mut Mine> {
    w.communities
        .iter_mut()
        .find(|c| c.id == cid)
        .and_then(|c| c.mine.as_mut())
}

/// One hour's planning of a community's mine on its own (tests, probes): what the planner does
/// for it in `plan_community`.
pub fn plan_now(w: &mut World, cid: CommunityId) {
    let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
        return;
    };
    let home = c.home;
    let people = c
        .members
        .iter()
        .filter(|m| w.agents.iter().any(|a| a.id == **m && a.alive))
        .count() as u32;
    let here = Here::of(w, cid, home);
    let was = w.meta.planning_for;
    w.meta.planning_for = Some(cid);
    plan(w, cid, home, &here, people);
    w.meta.planning_for = was;
}

/// Hourly, from the planner (settlement.rs `plan_community`, `w.meta.planning_for` set): sink
/// the mine, a cut at a time, while the community wants what lies deeper.
pub(crate) fn plan(w: &mut World, cid: CommunityId, home: TilePos, here: &Here, people: u32) {
    if w.meta.no_delving {
        return;
    }
    let tick = w.meta.tick;
    send_to_look(w, cid);
    let seek = wanted(w, cid, home, here, people);
    let has = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .is_some_and(|c| c.mine.is_some());
    // Nothing wanted: the mine waits, and `seek` keeps the last reason it was dug for (a row
    // is written when the reason changes, not each time the want comes back).
    let Some(seek) = seek else {
        return;
    };
    // Not while food is short (fate.rs: struggling or worse): the mine waits for better days.
    if w.communities
        .iter()
        .find(|c| c.id == cid)
        .is_some_and(|c| c.fate > 0)
    {
        return;
    }
    if !has {
        let Some(site) = settlement::quarry_site(w, home, cid) else {
            return;
        };
        if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
            c.mine = Some(Mine {
                site,
                foot: site,
                seek: None,
                cut: Vec::new(),
                deep: false,
                stone_level: None,
                rest_until: 0,
                look: None,
                chose: None,
            });
        }
    }
    let Some(m) = mine_of(w, cid) else {
        return;
    };
    if m.rest_until > tick {
        return;
    }
    if m.seek != Some(seek) {
        m.seek = Some(seek);
        let at = m.site;
        w.events.push(
            tick,
            Some(at),
            Severity::Notable,
            EventKind::MineSunk {
                community: cid,
                why: seek.why().to_string(),
            },
        );
    }
    dig(w, cid, seek);
}

/// The next cut of the mine, one at a time.
fn dig(w: &mut World, cid: CommunityId, seek: Seek) {
    let tick = w.meta.tick;
    let Some(m) = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .and_then(|c| c.mine.clone())
    else {
        return;
    };
    // One order of the mine's at a time, and the quarry's own shaft order first.
    let site_col = |p: TilePos| p.x == m.site.x && p.y == m.site.y;
    if w.designations.iter().any(|d| {
        !d.done
            && (m.cut.contains(&d.pos) || (site_col(d.pos) && d.kind == DesignationKind::DigStairs))
    }) {
        return;
    }
    // Where the shaft has got to: down every stair cut below the foot.
    let mut foot = m.foot;
    while matches!(
        w.grid.get(foot.offset(0, 0, -1)).shape,
        Shape::StairsUp | Shape::StairsBoth
    ) {
        foot = foot.offset(0, 0, -1);
    }
    if let Some(mm) = mine_of(w, cid) {
        mm.foot = foot;
    }
    let z = foot.z;
    // The walkable workings on this level: the foot and what the mine cut here.
    let level: Vec<TilePos> = std::iter::once(foot)
        .chain(
            m.cut
                .iter()
                .copied()
                .filter(|p| p.z == z && !w.grid.get(*p).shape.is_solid()),
        )
        .collect();
    let mats = seek.mats(w, hardest(w, cid));
    // 1. What they want, showing in the rock of this level.
    if let Some(t) = nearest_on_level(w, &level, LOOK, |p| {
        let t = w.grid.get(p);
        t.shape == Shape::Wall && mats.contains(&t.material)
    }) {
        if seek == Seek::HardStone && exposed(&level, t) {
            // Harder stone where the quarry can cut it: the quarry works this level now.
            if let Some(mm) = mine_of(w, cid) {
                mm.stone_level = Some(z);
            }
            return;
        }
        if let Some(p) = toward(w, &level, t) {
            order(w, cid, DesignationKind::Mine, Some(p));
            return;
        }
    }
    // Cautious as a body, not for a few wary members (the shelter rule's line, projects.rs).
    let cautious = stance(w, cid) < -0.15;
    // 2. A cavern heard through the rock, on this level's floor.
    if !cautious {
        if let Some(p) = heard(w, &level, z).and_then(|t| toward(w, &level, t)) {
            order(w, cid, DesignationKind::Mine, Some(p));
            return;
        }
    }
    // 3. Down a level: from the foot, or from a tile of the workings with solid rock under it
    // for two levels (a stair is not cut into a cave's roof); with none, a step aside first.
    let bottom = w.grid.z_min + 2;
    let into_deep = crate::underground::in_the_deep(w, foot.offset(0, 0, -1));
    if z > bottom && !(into_deep && cautious) {
        let solid_under = |p: TilePos| {
            let a = w.grid.get(p.offset(0, 0, -1)).shape == Shape::Wall;
            let b = p.z - 2 < w.grid.z_min || w.grid.get(p.offset(0, 0, -2)).shape == Shape::Wall;
            a && b && !w.designations.iter().any(|d| d.pos == p.offset(0, 0, -1))
        };
        let mut stands: Vec<TilePos> = level.iter().copied().filter(|p| solid_under(*p)).collect();
        stands.sort_by_key(|p| (p.dist(foot), p.x, p.y));
        if let Some(s) = stands.first().copied() {
            if let Some(mm) = mine_of(w, cid) {
                mm.foot = s;
            }
            order(w, cid, DesignationKind::DigStairs, Some(s.offset(0, 0, -1)));
            return;
        }
        let aside = sides(w, &level)
            .into_iter()
            .filter(|p| solid_under(*p))
            .min_by_key(|p| (p.dist(foot), p.x, p.y));
        if aside.is_some() {
            order(w, cid, DesignationKind::Mine, aside);
            return;
        }
    }
    // 4. At the bottom: galleries driven out each way along the rock.
    let drive = sides(w, &level)
        .into_iter()
        .filter(|p| (p.x == foot.x || p.y == foot.y) && p.dist(foot) <= DRIVE)
        .min_by_key(|p| (p.dist(foot), p.x, p.y));
    if drive.is_some() {
        order(w, cid, DesignationKind::Mine, drive);
        return;
    }
    if let Some(mm) = mine_of(w, cid) {
        mm.rest_until = tick + REST_DAYS * crate::time::TICKS_PER_DAY;
    }
}

/// Solid rock beside the walkable tiles of `level`, on the same level.
fn sides(w: &World, level: &[TilePos]) -> Vec<TilePos> {
    let mut out: Vec<TilePos> = Vec::new();
    for p in level {
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let q = p.offset(dx, dy, 0);
            if w.grid.in_bounds(q)
                && w.grid.get(q).shape == Shape::Wall
                && q.x > 0
                && q.y > 0
                && q.x < w.grid.width - 1
                && q.y < w.grid.height - 1
                && !out.contains(&q)
            {
                out.push(q);
            }
        }
    }
    out
}

fn exposed(level: &[TilePos], t: TilePos) -> bool {
    level
        .iter()
        .any(|p| p.z == t.z && (p.x - t.x).abs() + (p.y - t.y).abs() == 1)
}

/// The nearest tile on `level`'s z within `r` of any of its tiles that `f` holds for.
fn nearest_on_level(
    w: &World,
    level: &[TilePos],
    r: i32,
    f: impl Fn(TilePos) -> bool,
) -> Option<TilePos> {
    let z = level.first()?.z;
    let (x0, x1) = level
        .iter()
        .fold((i32::MAX, i32::MIN), |(a, b), p| (a.min(p.x), b.max(p.x)));
    let (y0, y1) = level
        .iter()
        .fold((i32::MAX, i32::MIN), |(a, b), p| (a.min(p.y), b.max(p.y)));
    let mut best: Option<(i32, i32, i32, TilePos)> = None;
    for y in (y0 - r).max(1)..=(y1 + r).min(w.grid.height - 2) {
        for x in (x0 - r).max(1)..=(x1 + r).min(w.grid.width - 2) {
            let p = TilePos::new(x, y, z);
            let d = level.iter().map(|q| q.dist(p)).min().unwrap_or(i32::MAX);
            if d > r || !f(p) {
                continue;
            }
            let key = (d, x, y, p);
            if best.is_none_or(|b| (key.0, key.1, key.2) < (b.0, b.1, b.2)) {
                best = Some(key);
            }
        }
    }
    best.map(|b| b.3)
}

/// The rock beside the workings nearest `t`: the next cut of a gallery driven to it.
fn toward(w: &World, level: &[TilePos], t: TilePos) -> Option<TilePos> {
    if exposed(level, t) && w.grid.get(t).shape == Shape::Wall {
        return Some(t);
    }
    sides(w, level)
        .into_iter()
        .min_by_key(|p| (p.dist(t), p.x, p.y))
}

/// The nearest open floor of a cavern within earshot of the workings on its floor level, not
/// already broken into from them.
fn heard(w: &World, level: &[TilePos], z: i32) -> Option<TilePos> {
    let near: Vec<&crate::world::gen::caverns::Cavern> = w
        .grid
        .caverns
        .iter()
        .filter(|c| c.centre.z == z && !(c.deep && c.opened))
        .collect();
    if near.is_empty() {
        return None;
    }
    let open = |p: TilePos| near.iter().any(|c| c.contains(p)) && !w.grid.get(p).shape.is_solid();
    let t = nearest_on_level(w, level, HEAR, open)?;
    // Already through: a tile of the workings beside the cavern.
    let through = level.iter().any(|p| {
        [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .iter()
            .any(|(dx, dy)| open(p.offset(*dx, *dy, 0)))
    });
    (!through).then_some(t)
}

/// Order a cut for the mine, slower the deeper it is (the spoil goes further up).
fn order(w: &mut World, cid: CommunityId, kind: DesignationKind, at: Option<TilePos>) {
    let Some(p) = at else {
        return;
    };
    if w.designations.iter().any(|d| !d.done && d.pos == p) {
        return;
    }
    let mut d = settlement::make(w, kind, p, 6);
    d.community = Some(cid);
    let below = (w.grid.ground_z(p.x, p.y) - p.z).max(0);
    let k = 1.0 + below as f32 / 15.0;
    d.work_total = (d.work_total as f32 * k).round() as u32;
    d.work_left = d.work_total;
    w.designations.push(d);
    if let Some(m) = mine_of(w, cid) {
        if !m.cut.contains(&p) {
            m.cut.push(p);
        }
    }
}

/// A deep cavern opened into and not yet gone into: the nearest grown member near the workings
/// goes in to look (the breach is underground.rs's when they stand in it).
fn send_to_look(w: &mut World, cid: CommunityId) {
    let Some((k, foot)) = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .and_then(|c| c.mine.as_ref())
        .and_then(|m| m.look.map(|k| (k, m.foot)))
    else {
        return;
    };
    let Some(c) = w.grid.caverns.get(k).cloned() else {
        return;
    };
    if c.opened {
        if let Some(m) = mine_of(w, cid) {
            m.look = None;
        }
        return;
    }
    let tick = w.meta.tick;
    let who = w
        .agents
        .iter()
        .enumerate()
        .filter(|(_, a)| {
            a.alive
                && a.community == Some(cid)
                && a.species == crate::agents::Species::Person
                && !a.is_child(tick)
                // Not in the middle of something: whoever is about the workings.
                && matches!(
                    a.activity,
                    crate::mind::Activity::Idle
                        | crate::mind::Activity::Wandering { .. }
                        | crate::mind::Activity::Loitering { .. }
                )
                && a.pos.dist(foot) <= 30
        })
        .min_by_key(|(_, a)| (a.pos.dist(foot), a.id))
        .map(|(i, _)| i);
    if let Some(i) = who {
        go_in(w, i, &c);
    }
}

fn go_in(w: &mut World, i: usize, c: &crate::world::gen::caverns::Cavern) {
    let me = w.agents[i].pos;
    let mut floor: Vec<TilePos> = crate::underground::floors(w, c).collect();
    floor.sort_by_key(|p| (p.dist(me), p.x, p.y));
    for p in floor.into_iter().take(4) {
        if w.path_agent_to(i, p) {
            w.agents[i].activity = crate::mind::Activity::Wandering { to: p };
            return;
        }
    }
}

/// Each tick: a cut into the deep's rock is the shaft reaching the deep; a cut that opens into
/// a deep cavern nobody has stood in sends the one who cut it in to look; a breach is followed
/// by the community's choice to wall the way or dig on.
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
        let Some(i) = w.agents.iter().position(|a| a.id == agent) else {
            continue;
        };
        let Some(cid) = w.agents[i].community else {
            continue;
        };
        if crate::underground::in_the_deep(w, at) {
            let reached = mine_of(w, cid).is_some_and(|m| {
                let first = !m.deep;
                m.deep = true;
                first
            });
            if reached {
                let rock = w
                    .grid
                    .materials
                    .get(w.grid.rock_at(w.grid.col(at.x, at.y), at.z))
                    .name
                    .clone();
                w.events.push(
                    tick,
                    Some(at),
                    Severity::Notable,
                    EventKind::ReachedDeep {
                        community: cid,
                        rock,
                    },
                );
            }
        }
        let opened_into = w.grid.caverns.iter().position(|c| {
            c.deep
                && !c.opened
                && [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .any(|(dx, dy)| c.contains(at.offset(*dx, *dy, 0)))
        });
        if let Some(k) = opened_into {
            if let Some(m) = mine_of(w, cid) {
                m.look = Some(k);
            }
        }
    }
    let breaches: Vec<(crate::ids::AgentId, String)> = w
        .last_events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Breached { agent, cavern } => Some((*agent, cavern.clone())),
            _ => None,
        })
        .collect();
    for (agent, cavern) in breaches {
        let Some(cid) = w
            .agents
            .iter()
            .find(|a| a.id == agent)
            .and_then(|a| a.community)
        else {
            continue;
        };
        let beast = w
            .last_events
            .iter()
            .any(|e| matches!(&e.kind, EventKind::BeastStirred { cavern: c, .. } if *c == cavern));
        let walled = beast || stance(w, cid) <= 0.0;
        let entry = w
            .grid
            .caverns
            .iter()
            .find(|c| c.name == cavern)
            .and_then(|c| c.breach);
        if walled {
            if let Some(entry) = entry {
                // underground.rs orders the wall itself when a beast stirs.
                let ordered = w.designations.iter().any(|d| {
                    !d.done && d.pos == entry && matches!(d.kind, DesignationKind::BuildWall(_))
                });
                if !ordered {
                    let mut d = settlement::make(
                        w,
                        DesignationKind::BuildWall(Material::STONE_BLOCK),
                        entry,
                        8,
                    );
                    d.community = Some(cid);
                    w.designations.push(d);
                }
            }
        }
        if let Some(m) = mine_of(w, cid) {
            m.chose = Some(walled);
            m.look = None;
        }
        w.events.push(
            tick,
            entry,
            Severity::Notable,
            EventKind::DeepChosen {
                community: cid,
                cavern,
                walled,
            },
        );
    }
}
