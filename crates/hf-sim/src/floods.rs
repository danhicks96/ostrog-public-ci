//! Rivers, lakes and ponds that rise and fall with the weather (#393). The water standing at the
//! start is surveyed once into bodies, each with the ground whose rain runs into it (its
//! catchment). Every hour the rain falling on that ground raises each body by what it gathers
//! for its size, so a stream fed by a whole hillside rises fast and a pond on a hilltop barely
//! moves; water above its usual level drains away again a share at a time. A body high enough
//! goes over its banks onto the open ground beside it, lowest first, and draws back as it
//! falls. Snow lies on the ground through the winter and runs to the water as it melts, so a
//! heavy winter brings a spring flood.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use serde::{Deserialize, Serialize};

use crate::events::{EventKind, Severity};
use crate::ids::TilePos;
use crate::plants::PlantKind;
use crate::world::{Shape, TileFlags, WorldGrid};
use crate::world_state::World;

/// The most tiles under flood water at once, over the whole valley.
pub const MAX_FLOOD: usize = 600;
/// Levels a body rises for each unit of rain over a catchment as large as itself: a stream
/// gathering thirty times its size stands about a level up in a steady rain.
pub const RUNOFF: f32 = 0.003;
/// The most a body gathers for its size: a puddle in a hollow with half the valley draining to
/// it (seed 3 has one of a tile gathering 3,796) would otherwise flood in a drizzle.
pub const MAX_GATHER: f32 = 40.0;
/// The share of the water above a body's usual level that drains away each hour.
pub const OUTFLOW: f32 = 0.08;
/// Levels above its usual surface at which a body goes over banks level with it; ground a
/// level higher takes a level more. A well-fed stream stays in its banks through a day of
/// steady rain and goes over in a long one or a storm.
pub const RISE: f32 = 0.5;
/// Levels above its usual surface at which a body is in spate: its shallows and the flood
/// over ground no higher than its banks stand over a person's head, and what lies there is
/// carried off.
pub const SPATE: f32 = 1.5;
/// The highest a body rises, in levels.
pub const MAX_STAGE: f32 = 3.0;
/// Levels a body loses each dry hour above freezing at ten degrees, more in heat: a stream
/// in summer runs dry after four or five days with no rain.
pub const EVAP: f32 = 0.004;
/// Below this a stream's shallows and the ponds its floods left are dry ground.
pub const DRY: f32 = -1.0;
/// Above this a dry stream runs again.
pub const RUNS: f32 = -0.5;
/// The lowest a body falls.
pub const MIN_STAGE: f32 = -2.0;
/// Snow lying on the ground, in units of rain, that an hour of snowfall adds.
pub const SNOWFALL: f32 = 1.0;
/// Snow melted each hour for each degree above freezing: a spring day of ten degrees melts
/// two and a half hours' snowfall an hour.
pub const MELT: f32 = 0.25;
/// A body reaching the map's edge is fed from beyond it: each of its tiles at the edge counts
/// for this many columns of ground upstream.
pub const UPSTREAM: u32 = 32;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum WaterKind {
    /// Long and narrow: a stream or a river.
    Stream,
    #[default]
    Lake,
    /// Small and round.
    Pond,
}

impl WaterKind {
    pub fn noun(self) -> &'static str {
        match self {
            WaterKind::Stream => "the stream",
            WaterKind::Lake => "the lake",
            WaterKind::Pond => "the pond",
        }
    }
}

/// What the water is to drink (#453): the sea is salt, a lagoon or an estuary mouth where
/// fresh water stands against the sea is brackish, and everything else is fresh.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum WaterQuality {
    /// Inland water: streams, lakes and ponds away from the sea.
    #[default]
    Fresh,
    /// The sea: on a coast, the body on the map's low edge.
    Salt,
    /// A fresh body whose shore stands within two tiles of the sea.
    Brackish,
}

impl WaterQuality {
    pub fn word(self) -> &'static str {
        match self {
            WaterQuality::Fresh => "fresh",
            WaterQuality::Salt => "salt",
            WaterQuality::Brackish => "brackish",
        }
    }

    /// The `Known::Water` belief tag for this quality as perceived (#453, epic #477 part 4).
    pub fn tag(self) -> u16 {
        match self {
            WaterQuality::Fresh => TAG_WATER_FRESH,
            WaterQuality::Salt => TAG_WATER_SALT,
            WaterQuality::Brackish => TAG_WATER_BRACKISH,
        }
    }
}

/// The `tag` a `Known::Water` belief carries: the quality as the agent perceived it
/// (#453, epic #477 part 4). Foul is what the nose or eye makes of a fouled tile, not a
/// `WaterQuality` (any quality fouls). `TAG_WATER_FRESH` must stay 0: beliefs saved
/// before the tags load with tag 0 (serde default), and an old belief reads as fresh.
pub const TAG_WATER_FRESH: u16 = 0;
/// The water smelled or seen foul (a tile of `Waters::foul`).
pub const TAG_WATER_FOUL: u16 = 1;
pub const TAG_WATER_SALT: u16 = 2;
pub const TAG_WATER_BRACKISH: u16 = 3;

/// One body of standing or running water as it was surveyed.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Body {
    pub kind: WaterKind,
    /// The z of its top water tiles (the lowest, where they differ).
    pub surface: i32,
    /// Its top water tiles.
    pub tiles: Vec<TilePos>,
    /// Those of its top tiles with a dry side.
    pub shore: Vec<TilePos>,
    /// Columns of ground whose rain runs into it, its own included.
    pub catchment: u32,
    /// Levels above its usual surface now (below it when negative).
    pub stage: f32,
    /// In spate since it last fell back within its banks (said once a flood).
    #[serde(default)]
    pub high: bool,
    /// The water tiles a drought has taken out of it, as they were, to put back when it runs
    /// again (empty while it runs).
    #[serde(default)]
    pub dried: Vec<(TilePos, crate::world::Tile)>,
    /// What it is to drink (#453): marked at the survey, the sea salt, a lagoon brackish,
    /// everything else fresh. Saves from before the field load as fresh.
    #[serde(default)]
    pub quality: WaterQuality,
}

impl Body {
    pub fn area(&self) -> u32 {
        self.tiles.len().max(1) as u32
    }
    /// How much faster than its own rain it rises: its catchment over its area.
    pub fn gathers(&self) -> f32 {
        (self.catchment as f32 / self.area() as f32).min(MAX_GATHER)
    }
    /// The middle of its tiles.
    pub fn middle(&self) -> (i32, i32) {
        let n = self.tiles.len().max(1) as i64;
        let sx: i64 = self.tiles.iter().map(|p| p.x as i64).sum();
        let sy: i64 = self.tiles.iter().map(|p| p.y as i64).sum();
        ((sx / n) as i32, (sy / n) as i32)
    }
    /// Stage a tile `dz` levels above the usual surface needs before the water reaches it.
    pub fn reaches(&self, dz: i32) -> bool {
        self.stage >= RISE + dz.max(0) as f32
    }
}

/// The valley's waters (#393), saved whole: the survey is of the water as it stood, and the
/// grid changes after it.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Waters {
    pub surveyed: bool,
    pub bodies: Vec<Body>,
    /// Which body each flooded tile came from.
    pub flood_of: BTreeMap<TilePos, u16>,
    /// Snow lying on the ground through the winter, in units of rain (#393): it melts in the
    /// thaw and runs to the water as rain does.
    #[serde(default)]
    pub snow: f32,
    /// The most snow that lay this winter; cleared when the thaw has taken it all.
    #[serde(default)]
    pub winter_snow: f32,
    /// Snow melted in the last hour.
    #[serde(default)]
    pub melt: f32,
    /// Tiles a spate has put over a person's head (`TileFlags::FULL`).
    #[serde(default)]
    pub full: BTreeSet<TilePos>,
    /// Ponds a flood left in hollows as it fell, and the body that left each.
    #[serde(default)]
    pub left: BTreeMap<TilePos, u16>,
    /// The high-water line: every column a flood has covered. The planners keep homes and
    /// fields off it.
    #[serde(default)]
    pub high_water: BTreeSet<(i32, i32)>,
    /// Tiles whose water is foul, and the hours of fouling left on each (part 2 of #453,
    /// systems/water_quality.rs). Saves from before the field load with none.
    #[serde(default)]
    pub foul: BTreeMap<TilePos, u16>,
    /// The row of the flood this water is part of (set when the valley floods, #319): the
    /// cascade records the rot it causes with this row. None once the water has receded.
    #[serde(default)]
    pub flood_event: Option<crate::ids::EventId>,
}

impl Waters {
    /// Whether a flood has been seen over this column.
    pub fn flooded_once(&self, x: i32, y: i32) -> bool {
        self.high_water.contains(&(x, y))
    }
}

/// The top water tile of a column, if its surface is water.
pub fn top_water(g: &WorldGrid, x: i32, y: i32) -> Option<TilePos> {
    let mut p = g.surface(x, y);
    if !g.get(p).is_water() {
        return None;
    }
    while p.z < g.z_max && g.get(p.offset(0, 0, 1)).is_water() {
        p = p.offset(0, 0, 1);
    }
    Some(p)
}

const N4: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// Survey the water standing now into bodies and give each its catchment. Run once, at the
/// first hour of a world (or of an old save); tests that build water by hand run it again.
pub fn survey(w: &mut World) {
    let g = &w.grid;
    let (wd, ht) = (g.width, g.height);
    let n = (wd * ht) as usize;
    // Top water per column; water that is itself a flood is not a body.
    let mut top: Vec<Option<TilePos>> = vec![None; n];
    for y in 0..ht {
        for x in 0..wd {
            if let Some(p) = top_water(g, x, y) {
                if !w.flooded.contains(&p) {
                    top[g.col(x, y)] = Some(p);
                }
            }
        }
    }
    // Bodies: columns of water joined by a side.
    let mut label: Vec<i32> = vec![-1; n];
    let mut bodies: Vec<Body> = Vec::new();
    for y in 0..ht {
        for x in 0..wd {
            let c = g.col(x, y);
            if top[c].is_none() || label[c] >= 0 || bodies.len() >= u16::MAX as usize {
                continue;
            }
            let id = bodies.len() as i32;
            let mut b = Body::default();
            let mut stack = vec![(x, y)];
            label[c] = id;
            while let Some((cx, cy)) = stack.pop() {
                let p = top[g.col(cx, cy)].expect("labelled water");
                b.tiles.push(p);
                let mut dry_side = false;
                for (dx, dy) in N4 {
                    let (qx, qy) = (cx + dx, cy + dy);
                    if qx < 0 || qy < 0 || qx >= wd || qy >= ht {
                        continue;
                    }
                    let q = g.col(qx, qy);
                    if top[q].is_none() {
                        dry_side = true;
                    } else if label[q] < 0 {
                        label[q] = id;
                        stack.push((qx, qy));
                    }
                }
                if dry_side {
                    b.shore.push(p);
                }
            }
            b.tiles.sort();
            b.shore.sort();
            bodies.push(b);
        }
    }
    // Catchments: a priority flood from every body outward over the ground, lowest first, so
    // each column goes to the water its rain would run down to (over the lowest rim, where it
    // lies in a hollow). Ties go to the first pushed, so it is the same every run.
    let height = |c: usize| match top[c] {
        Some(p) => p.z - 1,
        None => g.ground[c],
    };
    let mut heap: BinaryHeap<Reverse<(i32, u64, usize)>> = BinaryHeap::new();
    let mut seq = 0u64;
    for (c, l) in label.iter().enumerate() {
        if *l >= 0 {
            heap.push(Reverse((height(c), seq, c)));
            seq += 1;
        }
    }
    let mut catch = vec![0u32; bodies.len()];
    while let Some(Reverse((h, _, c))) = heap.pop() {
        let l = label[c] as usize;
        catch[l] += 1;
        let (x, y) = (c as i32 % wd, c as i32 / wd);
        for (dx, dy) in N4 {
            let (qx, qy) = (x + dx, y + dy);
            if qx < 0 || qy < 0 || qx >= wd || qy >= ht {
                continue;
            }
            let q = g.col(qx, qy);
            if label[q] >= 0 {
                continue;
            }
            label[q] = l as i32;
            heap.push(Reverse((h.max(height(q)), seq, q)));
            seq += 1;
        }
    }
    for (b, c) in bodies.iter_mut().zip(catch) {
        let edge = b
            .tiles
            .iter()
            .filter(|p| p.x == 0 || p.y == 0 || p.x == wd - 1 || p.y == ht - 1)
            .count() as u32;
        b.catchment = c + edge * UPSTREAM;
        b.surface = b.tiles.iter().map(|p| p.z).min().unwrap_or(0);
        let (x0, x1) = minmax(b.tiles.iter().map(|p| p.x));
        let (y0, y1) = minmax(b.tiles.iter().map(|p| p.y));
        let long = (x1 - x0).max(y1 - y0) + 1;
        let area = b.tiles.len();
        b.kind = if area >= 12 && long >= 12 && (area as f32) < 2.2 * b.shore.len() as f32 {
            WaterKind::Stream
        } else if area < 40 {
            WaterKind::Pond
        } else {
            WaterKind::Lake
        };
    }
    // Water quality (#453, FIXLOG 2026-10-29): on a coast the sea is the body hugging the
    // map's lowest edge, and it is salt. The low side is read off the ground, not the
    // landform's name: it is where the coast's bias has sunk the land.
    if w.meta.params.landform == crate::world::gen::Landform::Coast {
        let mean = [
            (0..ht).map(|y| g.ground[g.col(0, y)] as f64).sum::<f64>() / ht as f64,
            (0..ht)
                .map(|y| g.ground[g.col(wd - 1, y)] as f64)
                .sum::<f64>()
                / ht as f64,
            (0..wd).map(|x| g.ground[g.col(x, 0)] as f64).sum::<f64>() / wd as f64,
            (0..wd)
                .map(|x| g.ground[g.col(x, ht - 1)] as f64)
                .sum::<f64>()
                / wd as f64,
        ];
        let low = mean
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap());
        if let Some((side, _)) = low {
            let at_edge = |b: &Body| match side {
                0 => b.tiles.iter().any(|p| p.x == 0),
                1 => b.tiles.iter().any(|p| p.x == wd - 1),
                2 => b.tiles.iter().any(|p| p.y == 0),
                _ => b.tiles.iter().any(|p| p.y == ht - 1),
            };
            for b in bodies.iter_mut() {
                if at_edge(b) {
                    b.quality = WaterQuality::Salt;
                }
            }
        }
    }
    // Brackish: a distinct fresh body whose shore stands within two tiles of the sea (a
    // lagoon behind a bar, or an estuary mouth).
    let salt: BTreeSet<(i32, i32)> = bodies
        .iter()
        .filter(|b| b.quality == WaterQuality::Salt)
        .flat_map(|b| b.tiles.iter().map(|p| (p.x, p.y)))
        .collect();
    if !salt.is_empty() {
        let near_salt = |p: &TilePos| {
            (-2..=2).any(|dy| (-2..=2).any(|dx| salt.contains(&(p.x + dx, p.y + dy))))
        };
        for b in bodies.iter_mut() {
            if b.quality == WaterQuality::Fresh && b.shore.iter().any(near_salt) {
                b.quality = WaterQuality::Brackish;
            }
        }
    }
    w.waters.bodies = bodies;
    w.waters.surveyed = true;
}

fn minmax(it: impl Iterator<Item = i32>) -> (i32, i32) {
    it.fold((i32::MAX, i32::MIN), |(a, b), v| (a.min(v), b.max(v)))
}

/// The body of the standing surface water at a tile, if that column holds any: the surveyed
/// body it belongs to, else the body a flood tile or a flood-left pond names. Flood water and
/// the ponds a flood left take their body's quality and kind.
pub fn body_at(w: &World, p: TilePos) -> Option<&Body> {
    w.waters
        .bodies
        .iter()
        .find(|b| b.tiles.iter().any(|t| t.x == p.x && t.y == p.y))
        .or_else(|| {
            w.waters
                .flood_of
                .get(&p)
                .or_else(|| w.waters.left.get(&p))
                .and_then(|&i| w.waters.bodies.get(i as usize))
        })
}

/// The quality of the standing surface water at a tile (#453), if that column holds any. Flood
/// water and the ponds a flood left take their body's quality.
pub fn quality(w: &World, p: TilePos) -> Option<WaterQuality> {
    body_at(w, p).map(|b| b.quality)
}

/// The rain falling this hour, in units of a steady rain.
fn rain(w: &World) -> f32 {
    w.weather.rainfall()
}

/// Hourly (hazards.rs): the waters rise with the rain on their catchments, go over their banks,
/// and fall back.
pub fn hour(w: &mut World) {
    if !w.waters.surveyed {
        survey(w);
    }
    let mut fed = rain(w);
    // Snow lies on the ground and melts in the thaw, feeding the water as rain does.
    let wa = &mut w.waters;
    if w.weather.sky == crate::systems::weather::Sky::Snow {
        wa.snow += SNOWFALL;
        wa.winter_snow = wa.winter_snow.max(wa.snow);
    }
    wa.melt = if w.weather.temperature_c > 0.0 {
        (MELT * w.weather.temperature_c).min(wa.snow)
    } else {
        0.0
    };
    wa.snow -= wa.melt;
    fed += wa.melt;
    if wa.snow <= 0.0 && wa.melt <= 0.0 {
        wa.winter_snow = 0.0;
    }
    // A dry hour above freezing takes water off every body, more in the heat.
    let temp = w.weather.temperature_c;
    let evap = if fed <= 0.0 && temp > 0.0 {
        EVAP * (1.0 + (temp - 10.0).max(0.0) / 10.0)
    } else {
        0.0
    };
    for b in w.waters.bodies.iter_mut() {
        b.stage += fed * b.gathers() * RUNOFF;
        if b.stage > 0.0 {
            b.stage -= b.stage * OUTFLOW;
        }
        b.stage = (b.stage - evap).clamp(MIN_STAGE, MAX_STAGE);
    }
    rise(w);
    spate(w);
    fall(w);
    drought(w);
    high_water(w, fed);
}

/// High water on record: once each time a body goes into spate, and as the spring flood when
/// it is the thaw rather than rain that raised it.
fn high_water(w: &mut World, fed: f32) {
    let tick = w.meta.tick;
    let spring = w.waters.melt > fed - w.waters.melt;
    for b in w.waters.bodies.iter_mut() {
        if b.stage < RISE {
            b.high = false;
        } else if b.stage >= SPATE && !b.high {
            b.high = true;
            let at = b.shore.first().or(b.tiles.first()).copied();
            w.events.push(
                tick,
                at,
                Severity::Major,
                EventKind::HighWater {
                    water: b.kind.noun().into(),
                    spring,
                },
            );
        }
    }
}

/// A stream fallen to `DRY` loses its shallows (the tiles one level deep: a channel cut
/// deeper keeps pools), and the ponds its floods left dry up; when the rain brings it back to
/// `RUNS`, the water is put back where nothing has been built.
fn drought(w: &mut World) {
    let tick = w.meta.tick;
    for bi in 0..w.waters.bodies.len() {
        let (kind, stage, dry) = {
            let b = &w.waters.bodies[bi];
            (b.kind, b.stage, !b.dried.is_empty())
        };
        if kind == WaterKind::Stream && !dry && stage < DRY {
            let mut dried = Vec::new();
            for p in w.waters.bodies[bi].tiles.clone() {
                let t = w.grid.get(p);
                if !t.is_water()
                    || t.flags.has(TileFlags::DEEP)
                    || w.grid.get(p.offset(0, 0, -1)).is_water()
                {
                    continue;
                }
                let gone = if t.material == crate::world::Material::WATER {
                    crate::world::Tile::AIR
                } else {
                    crate::world::Tile {
                        flags: t.flags.without(TileFlags::WATER | TileFlags::FULL),
                        ..t
                    }
                };
                w.grid.set(p, gone);
                dried.push((p, t));
            }
            let left: Vec<TilePos> = w
                .waters
                .left
                .iter()
                .filter(|(_, b)| **b as usize == bi)
                .map(|(p, _)| *p)
                .collect();
            for p in left {
                w.waters.left.remove(&p);
                let mut t = w.grid.get(p);
                t.flags = t.flags.without(TileFlags::WATER | TileFlags::FULL);
                w.grid.set(p, t);
            }
            if let Some(&(at, _)) = dried.first() {
                w.waters.bodies[bi].dried = dried;
                w.events.push(
                    tick,
                    Some(at),
                    Severity::Notable,
                    EventKind::RanDry {
                        at,
                        water: kind.noun().into(),
                    },
                );
            }
        } else if dry && stage > RUNS {
            let dried = std::mem::take(&mut w.waters.bodies[bi].dried);
            let at = dried[0].0;
            for (p, t) in dried {
                let now = w.grid.get(p);
                if now.shape == Shape::Open && !now.is_water() {
                    w.grid.set(p, t);
                }
            }
            w.events.push(
                tick,
                Some(at),
                Severity::Notable,
                EventKind::RanAgain {
                    at,
                    water: kind.noun().into(),
                },
            );
        }
    }
}

/// A body in spate: its shallows and its flood over ground no higher than its banks go over a
/// person's head (the drowning in hazards.rs reads `FULL`), and what lies loose in its flood is
/// carried off.
fn spate(w: &mut World) {
    let mut deep: BTreeSet<TilePos> = BTreeSet::new();
    for (bi, b) in w.waters.bodies.iter().enumerate() {
        if b.stage < SPATE {
            continue;
        }
        for p in &b.tiles {
            let t = w.grid.get(*p);
            if t.flags.has(TileFlags::WATER) && !t.flags.has(TileFlags::DEEP) {
                deep.insert(*p);
            }
        }
        for (p, o) in &w.waters.flood_of {
            if *o as usize == bi && b.stage - (p.z - b.surface) as f32 >= SPATE {
                deep.insert(*p);
            }
        }
    }
    let was = std::mem::take(&mut w.waters.full);
    for p in was.difference(&deep) {
        let mut t = w.grid.get(*p);
        t.flags = t.flags.without(TileFlags::FULL);
        w.grid.set(*p, t);
    }
    for p in deep.difference(&was) {
        let mut t = w.grid.get(*p);
        if t.is_water() {
            t.flags = t.flags.with(TileFlags::FULL);
            w.grid.set(*p, t);
        }
    }
    w.waters.full = deep;
    if w.waters.full.is_empty() {
        return;
    }
    // Loose things in the spate's flood go with the water.
    let gone: Vec<(crate::ids::ItemId, u32, TilePos)> = w
        .items
        .iter()
        .filter_map(|it| match it.place {
            crate::items::ItemPlace::Ground(p)
                if w.waters.full.contains(&p) && w.flooded.contains(&p) =>
            {
                Some((it.id, it.count, p))
            }
            _ => None,
        })
        .collect();
    let Some(at) = gone.first().map(|g| g.2) else {
        return;
    };
    let mut things = 0;
    for (id, n, _) in gone {
        if w.consume_item(id, n, "carried off by the flood") {
            things += n;
        }
    }
    let tick = w.meta.tick;
    w.events.push(
        tick,
        Some(at),
        Severity::Notable,
        EventKind::CarriedOff { at, things },
    );
}

/// Where a body's water goes over: open tiles over loose ground beside its shore or its flood
/// that its stage reaches, a ring an hour, lowest first.
fn rise(w: &mut World) {
    let tick = w.meta.tick;
    let mut count = vec![0usize; w.waters.bodies.len()];
    for b in w.waters.flood_of.values() {
        if let Some(c) = count.get_mut(*b as usize) {
            *c += 1;
        }
    }
    let mut taken: BTreeSet<TilePos> = BTreeSet::new();
    let mut new: Vec<(TilePos, u16)> = Vec::new();
    for (bi, b) in w.waters.bodies.iter().enumerate() {
        if !b.reaches(i32::MIN) || w.flooded.len() + new.len() >= MAX_FLOOD {
            continue;
        }
        // The higher it stands over its banks, the further it spreads.
        let cap = ((b.stage - RISE) * b.area() as f32 * 3.0) as usize;
        if count[bi] >= cap {
            continue;
        }
        let g = &w.grid;
        let mut cands: Vec<(i32, TilePos)> = Vec::new();
        let from = b.shore.iter().copied().chain(
            w.waters
                .flood_of
                .iter()
                .filter(|(_, o)| **o as usize == bi)
                .map(|(p, _)| *p),
        );
        for f in from {
            if !g.get(f).is_water() {
                continue;
            }
            for (dx, dy) in N4 {
                let (qx, qy) = (f.x + dx, f.y + dy);
                if qx < 1 || qy < 1 || qx >= g.width - 1 || qy >= g.height - 1 {
                    continue;
                }
                let q = g.surface(qx, qy);
                // Water climbs or falls a level at a time.
                if (q.z - f.z).abs() > 1 || taken.contains(&q) {
                    continue;
                }
                let t = g.get(q);
                if t.shape != Shape::Open || t.is_water() {
                    continue;
                }
                let below = g.get(q.offset(0, 0, -1));
                if below.shape != Shape::Wall || !below.material.is_loose_ground() {
                    continue;
                }
                let dz = q.z - b.surface;
                if !b.reaches(dz) {
                    continue;
                }
                taken.insert(q);
                cands.push((dz, q));
            }
        }
        // One ring an hour, of the lowest ground it reaches: the floodplain fills before the
        // bank level with the water, and the bank before ground a level up. Nearest the body's
        // middle first when the room runs out.
        let Some(low) = cands.iter().map(|c| c.0).min() else {
            continue;
        };
        let (mx, my) = b.middle();
        let mut ring: Vec<(i32, TilePos)> = cands
            .into_iter()
            .filter(|c| c.0 == low)
            .map(|(_, q)| ((q.x - mx).abs() + (q.y - my).abs(), q))
            .collect();
        ring.sort();
        for (_, q) in ring.into_iter().take(cap - count[bi]) {
            new.push((q, bi as u16));
        }
    }
    if new.is_empty() {
        return;
    }
    let first = w.flooded.is_empty();
    let mut n = 0u32;
    for (q, bi) in new {
        if w.flooded.len() >= MAX_FLOOD {
            break;
        }
        let mut t = w.grid.get(q);
        t.flags = t.flags.with(TileFlags::WATER);
        w.grid.set(q, t);
        w.flooded.insert(q);
        w.waters.flood_of.insert(q, bi);
        w.waters.high_water.insert((q.x, q.y));
        w.plants
            .retain(|pl| !(pl.pos == q && (pl.kind == PlantKind::Root || pl.kind.is_crop())));
        n += 1;
    }
    if first && n > 0 {
        let ev = w.events.push(
            tick,
            w.flooded.iter().next().copied(),
            Severity::Major,
            EventKind::Flooded { tiles: n },
        );
        // The cascade (#319) records the rot this flood causes with this row.
        w.waters.flood_event = Some(ev);
    }
}

/// Flood water whose body has fallen below it drains off, from the outside in: a tile goes
/// when it has dry ground or its body's own water beside it at its level or lower. Flood water
/// with no way down to either, through the flood, lies in a hollow: it is left as a pond.
fn fall(w: &mut World) {
    if w.flooded.is_empty() {
        return;
    }
    let tick = w.meta.tick;
    let g = &w.grid;
    let below_stage = |p: TilePos| {
        w.waters
            .flood_of
            .get(&p)
            .and_then(|b| w.waters.bodies.get(*b as usize))
            .is_none_or(|b| !b.reaches(p.z - b.surface))
    };
    // Tiles with a way off: straight to lower or level dry ground or open water, or through
    // flood that has one.
    let off = |p: TilePos, qx: i32, qy: i32| {
        if qx < 0 || qy < 0 || qx >= g.width || qy >= g.height {
            return true;
        }
        let q = top_water(g, qx, qy).unwrap_or(g.surface(qx, qy));
        q.z <= p.z && !w.flooded.contains(&q)
    };
    if !w.flooded.iter().any(|p| below_stage(*p)) {
        return;
    }
    let mut drains: BTreeSet<TilePos> = BTreeSet::new();
    for p in w.flooded.iter().copied() {
        if N4.iter().any(|(dx, dy)| off(p, p.x + dx, p.y + dy)) {
            drains.insert(p);
        }
    }
    let mut way = drains.clone();
    let mut queue: Vec<TilePos> = drains.iter().copied().collect();
    while let Some(p) = queue.pop() {
        for (dx, dy) in N4 {
            let q = g.surface(p.x + dx, p.y + dy);
            if q.z >= p.z && w.flooded.contains(&q) && way.insert(q) {
                queue.push(q);
            }
        }
    }
    let mut out: Vec<TilePos> = Vec::new();
    let mut ponds: Vec<TilePos> = Vec::new();
    for p in w.flooded.iter().copied() {
        if !below_stage(p) {
            continue;
        }
        if drains.contains(&p) {
            out.push(p);
        } else if !way.contains(&p) {
            ponds.push(p);
        }
    }
    for p in out {
        let mut t = w.grid.get(p);
        t.flags = t.flags.without(TileFlags::WATER | TileFlags::FULL);
        w.grid.set(p, t);
        w.flooded.remove(&p);
        w.waters.flood_of.remove(&p);
    }
    if !ponds.is_empty() {
        let at = ponds[0];
        for p in &ponds {
            w.flooded.remove(p);
            let b = w.waters.flood_of.remove(p).unwrap_or(u16::MAX);
            w.waters.left.insert(*p, b);
        }
        w.events.push(
            tick,
            Some(at),
            Severity::Notable,
            EventKind::PondLeft {
                at,
                tiles: ponds.len() as u32,
            },
        );
    }
    if w.flooded.is_empty() {
        w.waters.flood_event = None;
        w.events
            .push(tick, None, Severity::Notable, EventKind::Receded);
    }
}
