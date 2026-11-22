//! Fire, flood and cold. Fire lives on tiles (a flag the walker avoids) and eats grass,
//! trees, wood and thatch; rain and buckets put it out. Lakes and streams rise with the rain
//! their ground gathers and draw back in dry weather (floods.rs, #393). Cold outside without clothes or a fire
//! drains warmth to frostbite and death. Someone whose head is under water makes for dry
//! ground and drowns if they are under too long (#390).

use rand::Rng;

use crate::agents::Species;
use crate::body::{Part, WoundKind};
use crate::events::{EventKind, Severity};
use crate::ids::TilePos;
use crate::items::ItemKind;
use crate::mind::ThoughtKind;
use crate::rng::{purpose, stream};
use crate::structures::StructureKind;
use crate::systems::weather::Sky;
use crate::time::TICKS_PER_HOUR;
use crate::world::{Material, Shape, TileFlags};
use crate::world_state::World;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Fire {
    pub pos: TilePos,
    /// Ticks of fuel left.
    pub left: u32,
    pub since: u64,
}

pub const FIRE_STEP: u64 = 40;
pub use crate::floods::MAX_FLOOD;
/// Ticks someone can be under water before they drown: three minutes (a tick is three
/// seconds, docs/TIME.md), time to climb two levels of a flooded shaft and no more. A body
/// made for swimming holds out three times as long.
pub const BREATH_TICKS: u32 = 60;
/// How often the drowning are looked at, in ticks.
pub const DROWN_STEP: u64 = 5;
/// How often people look at the flood water round them, in ticks (five minutes).
pub const FLEE_STEP: u64 = 100;
/// Tiles from flood water at which someone awake gets out of its way (#418). A flood spreads a
/// ring of tiles an hour (floods.rs), so this is about two hours' warning. Asleep, only water
/// at their side wakes them.
pub const FLOOD_WARY: i32 = 2;
/// Tiles a place must be from flood water for someone getting out of its way to make for it.
pub const FLOOD_CLEAR: i32 = 4;
/// Levels above its usual surface at which a body's flood is run from: half a level short of
/// a spate (floods.rs `SPATE`), two hours or so before it is over a head on seed 2's stream.
pub const FLEE_STAGE: f32 = crate::floods::SPATE - 0.5;
/// How far someone looks for that place, in tiles.
pub const FLEE_RANGE: i32 = 20;

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    reaction_step(w);
    if tick.is_multiple_of(FIRE_STEP) {
        fire_step(w);
    }
    if tick.is_multiple_of(DROWN_STEP) {
        drown_step(w);
    }
    if tick % FLEE_STEP == 50 {
        flee_step(w);
    }
    // Nobody is sent to a fire: whoever is near and knows what to do decides (decide.rs).
    // Undertakings on fires that have gone out are cleared.
    if tick % 200 == 7 {
        w.clear_moot_douse();
        w.clear_stale_undertakings();
    }
    if tick % TICKS_PER_HOUR == 300 {
        flood_step(w);
        cold_step(w);
    }
}

/// What burns at `p`, and for how long: 0 means nothing.
/// What would burn at `p`, for the tests (C71).
pub fn fuel_at(w: &World, p: TilePos) -> u32 {
    fuel(w, p)
}

fn fuel(w: &World, p: TilePos) -> u32 {
    let t = w.grid.get(p);
    // A denser wood burns longer: oak longer than pine (C56), by what the built tile was made of.
    let dense = |q: TilePos| {
        w.made_of(q)
            .map(|m| w.grid.materials.get(m).density / w.grid.materials.get(Material::WOOD).density)
            .filter(|f| f.is_finite() && *f > 0.0)
            .unwrap_or(1.0)
    };
    if t.shape == Shape::Wall {
        return match t.material {
            Material::WOOD => (1000.0 * dense(p)) as u32,
            Material::THATCH => 600,
            _ => 0,
        };
    }
    if t.shape == Shape::Floor && t.material == Material::WOOD {
        return (800.0 * dense(p)) as u32;
    }
    if matches!(
        t.shape,
        Shape::StairsUp | Shape::StairsDown | Shape::StairsBoth
    ) && t.material == Material::WOOD
    {
        return (900.0 * dense(p)) as u32;
    }
    if let Some(r) = w.roofs.get(&p) {
        return if r.material == Material::THATCH {
            700
        } else {
            900
        };
    }
    if t.is_water() || t.flags.has(TileFlags::SNOW) {
        return 0;
    }
    let mut f = 0;
    if w.plants.iter().any(|pl| pl.pos == p && pl.alive) {
        f = f.max(400);
    }
    if w.structures.iter().any(|s| s.pos == p && burns(w, s)) {
        f = f.max(700);
    }
    // Bare grass carries a fire only in dry heat (#258: a lightning fire crossed forty tiles
    // of summer ground overnight and took three houses' roofs; the C62 rule burns trees, wood
    // and thatch). The heat is where the grass stands (#268): heights run hotter. Snow lying on
    // the cover (the client's winter look, #289's ground overlay) keeps the bare ground from
    // carrying it at all; plants and wood above the snow are another matter.
    if t.shape == Shape::Open
        && w.temp_at(p) > 22.0
        && !w.weather.sky.wet()
        && !crate::cover::snow_lies(w, p.offset(0, 0, -1))
    {
        let below = w.grid.get(p.offset(0, 0, -1));
        if below.material.is_vegetated() {
            f = f.max(300);
        }
    }
    f
}

fn wooden(k: StructureKind) -> bool {
    matches!(
        k,
        StructureKind::Bed
            | StructureKind::Table
            | StructureKind::Chair
            | StructureKind::Stool
            | StructureKind::Chest
            | StructureKind::Shelf
            | StructureKind::Workbench
            | StructureKind::Loom
            | StructureKind::Barrel
            | StructureKind::Crate
            | StructureKind::FencePost
            | StructureKind::Lamp
            | StructureKind::CarpenterBench
    )
}

/// A structure that burns: the wooden kinds, and a shrine of timber (#297).
fn burns(w: &World, s: &crate::structures::Structure) -> bool {
    wooden(s.kind)
        || (s.kind == StructureKind::Shrine
            && (s.material == Material::WOOD
                || w.grid.materials.get(s.material).kind
                    == crate::world::material::MaterialKind::Wood))
}

fn flammable_item(k: ItemKind) -> bool {
    matches!(
        k,
        ItemKind::Log
            | ItemKind::Plank
            | ItemKind::Cloth
            | ItemKind::Clothes
            | ItemKind::Fibre
            | ItemKind::GrainSheaf
            | ItemKind::Firewood
            | ItemKind::Hide
            | ItemKind::Poultice
    )
}

impl World {
    /// Start a fire at `p` if anything there burns. Returns true when it caught.
    pub fn ignite(&mut self, p: TilePos, cause: &str) -> bool {
        if self.fires.iter().any(|f| f.pos == p) {
            return false;
        }
        let f = fuel(self, p);
        if f == 0 {
            return false;
        }
        let tick = self.meta.tick;
        self.fires.push(Fire {
            pos: p,
            left: f,
            since: tick,
        });
        let mut t = self.grid.get(p);
        t.flags = TileFlags(t.flags.0 | TileFlags::FIRE);
        self.grid.set(p, t);
        if cause != "spread" {
            self.events.push(
                tick,
                Some(p),
                Severity::Major,
                EventKind::FireStarted {
                    at: p,
                    cause: cause.to_string(),
                },
            );
        } else {
            // A fire that has spread to something built is said once (#258: seventy-seven
            // "burned" rows with no row between the strike and the first roof).
            // A roof or a wall: a lamp or a pile burning outside the door is not the houses.
            let built =
                |q: TilePos| self.roofs.contains_key(&q) || self.grid.get(q).shape == Shape::Wall;
            if built(p) && !self.fires.iter().any(|f| f.pos != p && built(f.pos)) {
                self.events.push(
                    tick,
                    Some(p),
                    Severity::Major,
                    EventKind::FireStarted {
                        at: p,
                        cause: "spreading to the houses".to_string(),
                    },
                );
            }
        }
        crate::systems::work::displace_from(self, p);
        true
    }

    /// Put out the fire at `p` (and record it); returns true if one was there.
    pub fn extinguish(&mut self, p: TilePos) -> bool {
        let before = self.fires.len();
        self.fires.retain(|f| f.pos != p);
        if self.fires.len() == before {
            return false;
        }
        let mut t = self.grid.get(p);
        t.flags = TileFlags(t.flags.0 & !TileFlags::FIRE);
        self.grid.set(p, t);
        true
    }

    /// Douse orders whose tile is no longer burning are moot.
    /// Someone's own undertaking that they are no longer at (they died, or turned to something
    /// else an hour ago) is dropped, so it does not stand in anyone else's way. Not a work in
    /// a fey mood, which only the mood ends.
    pub fn clear_stale_undertakings(&mut self) {
        let now = self.meta.tick;
        let agents = &self.agents;
        let mut ds = std::mem::take(&mut self.designations);
        let gone = crate::work::split_off(&mut ds, |d| {
            let Some(o) = d.owner else { return true };
            if d.done {
                return false;
            }
            let Some(a) = agents.iter().find(|a| a.id == o) else {
                return false;
            };
            if !a.alive {
                return false;
            }
            // A work in a fey mood waits while they eat and sleep; the mood ends it
            // (systems/moods.rs).
            if a.fey.as_ref().is_some_and(|f| f.work == d.id) {
                return true;
            }
            let on_it = matches!(&a.activity, crate::mind::Activity::Working { plan }
                if plan.steps.iter().any(|s| matches!(s, crate::work::Step::Work { designation, .. } if *designation == d.id)));
            on_it || now.saturating_sub(d.created) < crate::time::TICKS_PER_HOUR
        });
        self.designations = ds;
        self.give_back(gone);
    }

    pub fn clear_moot_douse(&mut self) {
        let fires: Vec<TilePos> = self.fires.iter().map(|f| f.pos).collect();
        self.designations.retain(|d| {
            if d.kind == crate::work::DesignationKind::BeatOut {
                return fires.contains(&d.pos);
            }
            d.kind != crate::work::DesignationKind::Douse
                || [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .any(|(dx, dy)| fires.contains(&d.pos.offset(*dx, *dy, 0)))
        });
    }
}

fn burn_out(w: &mut World, p: TilePos) {
    let tick = w.meta.tick;
    // Ash lies where it burned until it grows over (ten days).
    w.scorched.insert(p.offset(0, 0, -1), tick);
    if w.scorched.len() > 4000 {
        let cut = tick.saturating_sub(10 * crate::time::TICKS_PER_DAY);
        w.scorched.retain(|_, t| *t >= cut);
    }
    let t = w.grid.get(p);
    let mut t2 = t;
    t2.flags = TileFlags(t.flags.0 & !TileFlags::FIRE);
    if t.shape == Shape::Wall && matches!(t.material, Material::WOOD | Material::THATCH) {
        // The wall is gone; what is left is open ground on whatever supported it.
        t2.shape = Shape::Open;
        t2.material = Material::AIR;
    } else if matches!(
        t.shape,
        Shape::Floor | Shape::StairsUp | Shape::StairsDown | Shape::StairsBoth
    ) && t.material == Material::WOOD
    {
        // A raised wooden floor or stair burns away; it does not become a floating soil tile.
        t2.shape = Shape::Open;
        t2.material = Material::AIR;
    }
    w.grid.set(p, t2);
    let below_p = p.offset(0, 0, -1);
    let below = w.grid.get(below_p);
    if below.material.is_vegetated() && below.shape == Shape::Wall {
        let mut b = below;
        b.material = Material::SOIL;
        w.grid.set(below_p, b);
    }
    w.plants.retain(|pl| pl.pos != p);
    if w.roofs.remove(&p).is_some() {
        w.events.push(
            tick,
            Some(p),
            Severity::Notable,
            EventKind::Burned {
                at: p,
                what: "roof".into(),
            },
        );
    }
    let burned_structures: Vec<String> = w
        .structures
        .iter()
        .filter(|s| s.pos == p && burns(w, s))
        .map(|s| format!("{:?}", s.kind).to_lowercase())
        .collect();
    let gone: Vec<crate::ids::StructureId> = w
        .structures
        .iter()
        .filter(|s| s.pos == p && burns(w, s))
        .map(|s| s.id)
        .collect();
    w.structures.retain(|s| !gone.contains(&s.id));
    let ids: Vec<(crate::ids::ItemId, u32, Option<String>)> = w
        .items
        .iter()
        .filter(|it| flammable_item(it.kind) && it.place == crate::items::ItemPlace::Ground(p))
        .map(|it| {
            // A thing with a story is named as it burns (#170, C62: it went as "1 hides").
            let named = it.story.as_ref().map(|s| {
                s.name
                    .as_deref()
                    .map(|n| n.trim_start_matches("the ").to_string())
                    .unwrap_or_else(|| crate::legend::noun(it.kind, it.material))
            });
            (it.id, it.count, named)
        })
        .collect();
    for (id, n, named) in ids {
        w.consume_item(id, n, "burned");
        if let Some(what) = named {
            w.events.push(
                tick,
                Some(p),
                Severity::Major,
                EventKind::Burned { at: p, what },
            );
        }
    }
    for what in burned_structures {
        w.events.push(
            tick,
            Some(p),
            Severity::Notable,
            EventKind::Burned { at: p, what },
        );
    }
}

fn fire_step(w: &mut World) {
    let tick = w.meta.tick;
    let seed = w.meta.seed;
    let raining = w.weather.sky.wet();
    let snowing = matches!(w.weather.sky, Sky::Snow);
    let temp = w.weather.temperature_c;
    let mut r = stream(seed, &[purpose::NAMES, 0xF1E, tick]);
    // Ignition: a hearth now and then when it is dry and warm where it stands (#268).
    if !raining {
        let hearths: Vec<TilePos> = w
            .structures
            .iter()
            .filter(|s| s.kind == StructureKind::Hearth && s.active && w.temp_at(s.pos) > 8.0)
            .map(|s| s.pos)
            .collect();
        for h in hearths {
            if r.gen::<f32>() < 1.0 / 20_000.0 {
                // A spark lands beside the hearth, never in it.
                let (dx, dy) = [
                    (1, 0),
                    (-1, 0),
                    (0, 1),
                    (0, -1),
                    (1, 1),
                    (-1, -1),
                    (1, -1),
                    (-1, 1),
                ][r.gen_range(0..8)];
                w.ignite(h.offset(dx, dy, 0), "from the hearth");
            }
        }
    }
    // Dry thunder: lightning from a warm overcast sky with no rain to put it out.
    // A deity may lean it (deity.rs): likelier, and in milder weather.
    let storms = w.lean(crate::deity::Lever::DryStorms, crate::deity::Scope::Valley);
    let dry_storm =
        matches!(w.weather.sky, Sky::Overcast) && temp > if storms > 1.0 { 9.0 } else { 15.0 };
    // A storm's lightning (#279): it strikes, though the rain gives it little to take.
    let wet_storm = w.weather.sky == Sky::Storm;
    if (dry_storm && r.gen::<f32>() < storms / 3_000.0)
        || (wet_storm && r.gen::<f32>() < 1.0 / 900.0)
    {
        let g = &w.grid;
        let x = r.gen_range(2..g.width - 2);
        let y = r.gen_range(2..g.height - 2);
        let p = g.surface(x, y);
        w.ignite(p, "by lightning");
    }
    if w.fires.is_empty() {
        return;
    }
    // Burn, spread, go out.
    let mut spread: Vec<TilePos> = Vec::new();
    let mut out: Vec<TilePos> = Vec::new();
    let fires = w.fires.clone();
    for f in &fires {
        let here = fuel(w, f.pos);
        let mut chance = if here >= 600 {
            0.04
        } else if here >= 400 {
            0.12
        } else {
            // Grass: slower than a wood, and only while the day is hot (#258: the night's cool
            // stops a grass fire now that the day's curve is the right way up).
            0.08
        };
        if raining {
            chance = 0.0;
        } else if snowing || temp < 2.0 {
            chance *= 0.3;
        } else if temp > 25.0 {
            chance *= 1.6;
        }
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let q = f.pos.offset(dx, dy, 0);
            if w.fires.iter().any(|g| g.pos == q) || spread.contains(&q) {
                continue;
            }
            if fuel(w, q) > 0 && r.gen::<f32>() < chance {
                spread.push(q);
            }
        }
    }
    for q in spread {
        w.ignite(q, "spread");
    }
    for f in w.fires.iter_mut() {
        f.left = f
            .left
            .saturating_sub(FIRE_STEP as u32 + if raining { 200 } else { 0 });
        if f.left == 0 {
            out.push(f.pos);
        }
    }
    let out_count = out.len() as u32;
    for p in out {
        w.fires.retain(|f| f.pos != p);
        burn_out(w, p);
    }
    // People standing in fire are burned and pushed off it.
    let burning: Vec<TilePos> = w.fires.iter().map(|f| f.pos).collect();
    let n = w.agents.len();
    for i in 0..n {
        if !w.agents[i].alive || w.agents[i].species != Species::Person {
            continue;
        }
        if burning.contains(&w.agents[i].pos) {
            crate::systems::health::wound(
                w,
                i,
                Part::LEG_L,
                WoundKind::Burn,
                0.35,
                0.0,
                "in the fire",
                None,
            );
            let p = w.agents[i].pos;
            crate::systems::work::displace_from(w, p);
        }
        // Within 6 tiles is not enough on its own: a fire behind a wall is not seen, so an
        // unobstructed line of sight is required too (OST-906).
        let seen = burning.iter().any(|b| {
            b.dist(w.agents[i].pos) <= 6
                && crate::mind::perception::line_of_sight(&w.grid, w.agents[i].pos, *b)
        });
        if seen {
            w.agents[i].thoughts.add(ThoughtKind::SawFire, tick, None);
        }
    }
    // The fire's end, with every tile it burned since the first start (#258: "burned 3" was
    // the tiles alight on the last step).
    w.fire_tiles += out_count;
    if w.fires.is_empty() && !fires.is_empty() {
        let burned = w.fire_tiles.max(fires.len() as u32);
        w.fire_tiles = 0;
        w.events.push(
            tick,
            Some(fires[0].pos),
            Severity::Notable,
            EventKind::FireOut {
                at: fires[0].pos,
                burned,
            },
        );
    }
}

fn flood_step(w: &mut World) {
    let raining = w.weather.sky.wet();
    if raining {
        w.rain_hours += 1;
        w.dry_hours = 0;
    } else {
        w.dry_hours += 1;
        if w.dry_hours >= 3 {
            w.rain_hours = 0;
        }
    }
    // The water rises with what its catchment gathers and falls back (floods.rs, #393).
    crate::floods::hour(w);
}

/// Under water: the tile is full to its roof, or there is water on it and on the one above,
/// so the head is under. Water standing shallow in a working (#272) is not.
pub fn under_water(w: &World, p: TilePos) -> bool {
    w.grid.over_head(p)
}

/// The hazard thresholds used by both perception and movement. Shallow lapping is wetness, not
/// an urgent personal problem (FIXLOG 2026-10-30, PR #476).
pub fn water_problem(
    w: &World,
    p: TilePos,
) -> Option<(crate::mind::ProblemKind, crate::mind::ObservationKind, f32)> {
    if w.flooded.contains(&p) && (under_water(w, p) || flood_near(w, p, 0)) {
        return Some((
            crate::mind::ProblemKind::Flooding,
            crate::mind::ObservationKind::Flooding,
            0.9,
        ));
    }
    let flags = w.grid.get(p).flags;
    if under_water(w, p) || flags.has(TileFlags::DEEP) {
        return Some((
            crate::mind::ProblemKind::WaterHazard,
            crate::mind::ObservationKind::WaterDepth,
            0.9,
        ));
    }
    None
}

/// Record only the personal danger of this person standing at this tile; no global alert is
/// made. The look's single water observation belongs to the perceive step's fused scan: a
/// second observation here double-noted deep water (FIXLOG 2026-10-31, c146, PR #476).
pub fn observe_water(w: &mut World, i: usize, p: TilePos) {
    if w.agents[i].pos != p {
        return;
    }
    let tick = w.meta.tick;
    if let Some((kind, _, urgency)) = water_problem(w, p) {
        let a = &mut w.agents[i];
        a.problems
            .observe(kind, p, crate::mind::Evidence::Direct, urgency, tick, None);
    }
}

fn observe_problem(
    w: &mut World,
    i: usize,
    kind: crate::mind::ProblemKind,
    urgency: f32,
) -> crate::mind::ProblemId {
    let tick = w.meta.tick;
    let at = w.agents[i].pos;
    let a = &mut w.agents[i];
    a.learning.observe(crate::mind::Observation {
        tick,
        pos: at,
        kind: kind.cue(),
        strength: urgency,
        evidence: crate::mind::Evidence::Direct,
        event: None,
        full: false,
        reading: None,
    });
    a.problems
        .observe(kind, at, crate::mind::Evidence::Direct, urgency, tick, None)
}

fn finish_reaction(
    w: &mut World,
    i: usize,
    outcome: crate::mind::OutcomeKind,
    success: bool,
    abandoned: bool,
) {
    let tick = w.meta.tick;
    let Some((id, _)) = w.agents[i].problems.pending_any() else {
        return;
    };
    let Some((cue, at)) = w.agents[i].problems.get(id).map(|p| (p.kind.cue(), p.pos)) else {
        return;
    };
    let Some(attempt) = w.agents[i]
        .problems
        .complete_attempt(id, outcome, success, tick)
    else {
        return;
    };
    if abandoned {
        w.agents[i].problems.abandon(id, tick);
    }
    // The lesson comes from the actual outcome, including a blocked or abandoned escape; it is
    // never inferred from having emitted a choice event (FIXLOG 2026-10-30, PR #476).
    if w.agents[i].alive {
        w.agents[i]
            .learning
            .learn(cue, outcome, Some(at), success, tick, attempt.event);
    }
}

fn reaction_step(w: &mut World) {
    for i in 0..w.agents.len() {
        let Some((id, attempt)) = w.agents[i].problems.pending_any() else {
            continue;
        };
        if !w.agents[i].alive {
            finish_reaction(w, i, crate::mind::OutcomeKind::Death, false, false);
            continue;
        }
        let Some(kind) = w.agents[i].problems.get(id).map(|p| p.kind) else {
            continue;
        };
        let at = w.agents[i].pos;
        let safe = match kind {
            crate::mind::ProblemKind::Flooding => !under_water(w, at) && !in_floods_way(w, at),
            crate::mind::ProblemKind::WaterHazard => {
                water_problem(w, at).is_none() && !in_floods_way(w, at)
            }
            _ => false,
        };
        if safe {
            finish_reaction(w, i, crate::mind::OutcomeKind::Safety, true, false);
        } else if w.agents[i].path_blocked {
            finish_reaction(w, i, crate::mind::OutcomeKind::NoChange, false, false);
        } else if !matches!(&w.agents[i].activity,
            crate::mind::Activity::Going { to, .. } if Some(*to) == attempt.target)
        {
            finish_reaction(w, i, crate::mind::OutcomeKind::NoChange, false, true);
        }
    }
}

/// A movement failure belongs to the pending personal attempt, not to an evictable record index.
pub fn path_failed(w: &mut World, i: usize) {
    if w.agents[i].problems.pending_any().is_some() {
        finish_reaction(w, i, crate::mind::OutcomeKind::NoChange, false, false);
    }
}

fn announcement(
    w: &mut World,
    i: usize,
    kind: crate::mind::ProblemKind,
) -> Option<crate::ids::EventId> {
    let tick = w.meta.tick;
    let key = match kind {
        crate::mind::ProblemKind::Flooding => 0x7400_0000,
        _ => 0xE160_0000,
    };
    let a = &mut w.agents[i];
    if a.work_cooldowns.get(&key).is_some_and(|t| *t > tick) {
        return None;
    }
    a.work_cooldowns
        .insert(key, tick + crate::time::TICKS_PER_DAY);
    let (id, at) = (a.id, a.pos);
    let (what, because) = match kind {
        crate::mind::ProblemKind::Flooding => ("made for higher ground", "the water was rising"),
        _ => (
            "responded to dangerous water by moving to safety",
            "they noticed dangerous water",
        ),
    };
    Some(w.events.push(
        tick,
        Some(at),
        Severity::Notable,
        EventKind::Chose {
            agent: id,
            what: what.into(),
            because: because.into(),
        },
    ))
}

fn attempt_escape(w: &mut World, i: usize, to: Option<TilePos>, kind: crate::mind::ProblemKind) {
    if w.agents[i].problems.pending_any().is_some() {
        finish_reaction(w, i, crate::mind::OutcomeKind::NoChange, false, true);
    }
    let id = observe_problem(w, i, kind, 0.9);
    w.agents[i]
        .problems
        .choose(id, crate::mind::ReactionKind::MoveToSafety);
    if let Some(to) = to {
        let then = if kind == crate::mind::ProblemKind::Flooding {
            TICKS_PER_HOUR as u32
        } else {
            120
        };
        crate::systems::decide::commit(
            w,
            i,
            crate::mind::Activity::Going {
                to,
                then: Box::new(crate::mind::Activity::Loitering { ticks_left: then }),
            },
        );
    }
    let started = to.is_some_and(|to| {
        matches!(&w.agents[i].activity, crate::mind::Activity::Going { to: t, .. } if *t == to)
    });
    let event = if started {
        announcement(w, i, kind)
    } else {
        None
    };
    let attempt = crate::mind::ReactionAttempt {
        tick: w.meta.tick,
        reaction: crate::mind::ReactionKind::MoveToSafety,
        expected: Some(crate::mind::OutcomeKind::Safety),
        actual: None,
        success: false,
        event,
        target: to,
    };
    w.agents[i].problems.record_attempt(id, attempt);
    if !started {
        finish_reaction(w, i, crate::mind::OutcomeKind::NoChange, false, false);
    }
}

/// #390: someone under water makes for the nearest dry ground they can reach, and drowns if
/// they are under for longer than they can hold their breath.
fn drown_step(w: &mut World) {
    if w.drowning.is_empty() && !w.agents.iter().any(|a| a.alive && under_water(w, a.pos)) {
        return;
    }
    let mut drowned = Vec::new();
    for i in 0..w.agents.len() {
        let a = &w.agents[i];
        let id = a.id;
        if !a.alive || !under_water(w, a.pos) {
            w.drowning.remove(&id);
            continue;
        }
        let swims = a
            .species_def()
            .body_plan()
            .parts
            .iter()
            .any(|d| d.roles.contains(&crate::plan::Role::Swim));
        let breath = if swims {
            BREATH_TICKS * 3
        } else {
            BREATH_TICKS
        };
        let under = w.drowning.entry(id).or_insert(0);
        *under += DROWN_STEP as u32;
        if *under >= breath {
            drowned.push(id);
            continue;
        }
        // Out by the nearest way, not on to wherever they were going: on seed 2 a man whose
        // way to a meal went under kept walking on through the flood to it (#418).
        let here = w.agents[i].pos;
        let to = dry_ground_near(w, here);
        if let Some(to) = to {
            let heading = matches!(&w.agents[i].activity,
                crate::mind::Activity::Going { to: t, .. }
                    if !w.grid.get(*t).is_water() && t.dist(here) <= to.dist(here) + 1);
            if heading {
                continue;
            }
        }
        let kind = water_problem(w, here)
            .map(|p| p.0)
            .unwrap_or(crate::mind::ProblemKind::WaterHazard);
        attempt_escape(w, i, to, kind);
    }
    for id in drowned {
        w.drowning.remove(&id);
        let flood = w
            .agents
            .iter()
            .find(|a| a.id == id)
            .is_some_and(|a| w.waters.full.contains(&a.pos));
        let how = if flood {
            "drowned in the flood"
        } else {
            "drowned in the flooded workings"
        };
        crate::systems::life::die(w, id, how.into());
    }
}

/// Flood water within `r` tiles of `p` (and a level up or down) that is dangerous: water a
/// spate has put over a person's head, or a flood from a body risen to `FLEE_STAGE`. A pond
/// lapping a tile over its bank is not run from (the first version of this fled it fifty
/// times a day and left people hungry in `tests/autonomy.rs`).
fn flood_near(w: &World, p: TilePos, r: i32) -> bool {
    let wa = &w.waters;
    for dz in -1..=1 {
        for dy in -r..=r {
            for dx in -r..=r {
                let q = p.offset(dx, dy, dz);
                if wa.full.contains(&q)
                    || wa
                        .flood_of
                        .get(&q)
                        .and_then(|b| wa.bodies.get(*b as usize))
                        .is_some_and(|b| b.stage >= FLEE_STAGE)
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Whether `p` is in a dangerous flood's way (#418): nobody chooses to go there (decide.rs).
pub fn in_floods_way(w: &World, p: TilePos) -> bool {
    !(w.flooded.is_empty() && w.waters.full.is_empty()) && flood_near(w, p, FLOOD_WARY)
}

/// #418: people who see flood water close by get out of its way to dry ground clear of it,
/// higher where there is higher ground, before it is over their heads: whatever they were at
/// waits, as it would for a fire. The sleeping wake only when it is at their side, so the
/// flood can still catch someone asleep. (FIXLOG 2026-10-27: on seed 2, 31 drowned; people
/// stood and worked at the flood's edge as it rose and walked into it on their way to bed.)
fn flee_step(w: &mut World) {
    if w.flooded.is_empty() && w.waters.full.is_empty() {
        return;
    }
    for i in 0..w.agents.len() {
        let a = &w.agents[i];
        if !a.alive || a.species != Species::Person || w.drowning.contains_key(&a.id) {
            continue;
        }
        let r = if a.activity.is_asleep() {
            1
        } else {
            FLOOD_WARY
        };
        if !flood_near(w, a.pos, r) {
            continue;
        }
        // Already on the way to ground clear of it.
        if matches!(&a.activity, crate::mind::Activity::Going { to, .. }
            if !flood_near(w, *to, FLOOD_CLEAR))
        {
            continue;
        }
        // This remains the sole flood-escape movement owner; E16 records its actual attempt and
        // result rather than scoring a second escape for idle people (FIXLOG 2026-10-30, PR #476).
        let to = high_ground_near(w, a.pos);
        attempt_escape(w, i, to, crate::mind::ProblemKind::Flooding);
    }
}

/// The nearest dry tile someone at `p` can walk to that is `FLOOD_CLEAR` tiles from any flood
/// water, the highest of the nearest ring that has one.
fn high_ground_near(w: &World, p: TilePos) -> Option<TilePos> {
    let nav = w.nav.as_ref()?;
    for d in 1..=FLEE_RANGE {
        let mut best: Option<(i32, TilePos)> = None;
        for dz in (-1..=3).rev() {
            for dy in -d..=d {
                for dx in -d..=d {
                    if dx.abs().max(dy.abs()) != d {
                        continue;
                    }
                    let q = p.offset(dx, dy, dz);
                    if best.is_some_and(|(bz, bq)| (-dz, q) >= (-bz, bq))
                        || !w.grid.walkable(q)
                        || w.grid.get(q).is_water()
                        || flood_near(w, q, FLOOD_CLEAR)
                        || !nav.reach(p, q)
                    {
                        continue;
                    }
                    best = Some((dz, q));
                }
            }
        }
        if let Some((_, q)) = best {
            return Some(q);
        }
    }
    None
}

/// The nearest dry tile someone at `p` can walk to, up to twelve tiles across and six levels
/// up (the way out of a flooded shaft is up).
fn dry_ground_near(w: &World, p: TilePos) -> Option<TilePos> {
    let nav = w.nav.as_ref()?;
    let mut best: Option<(i32, TilePos)> = None;
    for dz in 0..=6 {
        for dy in -12..=12 {
            for dx in -12..=12 {
                let q = p.offset(dx, dy, dz);
                let d = dx.abs().max(dy.abs()) + dz;
                if best.is_some_and(|(bd, bq)| (d, q) >= (bd, bq))
                    || !w.grid.walkable(q)
                    || w.grid.get(q).is_water()
                    || !nav.reach(p, q)
                {
                    continue;
                }
                best = Some((d, q));
            }
        }
    }
    best.map(|b| b.1)
}

/// Indoors: standing on a built floor.
/// Under a roof (building.rs), or under rock. A floor without a roof over it is not indoors.
pub fn indoors(w: &World, p: TilePos) -> bool {
    crate::building::roofed(w, p)
}

pub fn cold_step(w: &mut World) {
    let tick = w.meta.tick;
    let hearths: Vec<TilePos> = w
        .structures
        .iter()
        .filter(|s| s.kind == StructureKind::Hearth && s.active)
        .map(|s| s.pos)
        .collect();
    let mut fires: Vec<TilePos> = w.fires.iter().map(|f| f.pos).collect();
    // A band camped near the valley keeps a fire at its camp (the client draws it there): it
    // warms whoever sits by it. (C35 soaks: whole bands froze in their winter camps.)
    fires.extend(
        w.bands
            .iter()
            .filter(|b| matches!(b.state, crate::threat::BandState::Camped(_)))
            .map(|b| b.camp),
    );
    let people: Vec<TilePos> = w
        .agents
        .iter()
        .filter(|a| a.alive && a.species == Species::Person)
        .map(|a| a.pos)
        .collect();
    // Shade: the columns of living mature trees (OST-900). A person stood on the ground under
    // one of these, or its eight neighbours, is out of the sun even outdoors.
    let tree_columns: std::collections::BTreeSet<(i32, i32)> = w
        .plants
        .iter()
        .filter(|p| p.alive && p.kind.is_tree() && p.mature())
        .map(|p| (p.pos.x, p.pos.y))
        .collect();
    let n = w.agents.len();
    let mut dead = Vec::new();
    for i in 0..n {
        if !w.agents[i].alive || w.agents[i].species != Species::Person {
            continue;
        }
        let p = w.agents[i].pos;
        let warm_near = hearths.iter().any(|h| h.dist(p) <= 4 && h.z == p.z)
            || fires.iter().any(|f| f.dist(p) <= 3);
        // Below ground the air keeps the same through the year (#346): no frost reaches it.
        let inside = indoors(w, p) || crate::underground::below(w, p);
        let clothed = w.agents[i].clothes_wear > 0.2;
        // Huddling: someone else within a tile shares the warmth.
        let huddled = people
            .iter()
            .filter(|q| q.z == p.z && q.dist(p) <= 1)
            .count()
            >= 2;
        // The air where they stand (#268): 1.5 degrees colder each level above the
        // founding site's ground, warmer below it (below ground the season does not
        // reach at all — the `inside` rule above already keeps them from feeling it).
        let temp = w.temp_at(p);
        let a = &mut w.agents[i];
        // Each people feels cold below its own comfort, and heat above it (P4).
        let [cold_below, hot_above] = a.species_def().climate.comfort_c;
        // Hardened to the cold by a winter out in it (#346).
        let cold_below = cold_below - a.hardened;
        if temp < cold_below && !inside && !warm_near {
            let mut loss = 0.035;
            loss *= match (clothed, a.furred) {
                (true, true) => 0.3,
                (true, false) => 0.5,
                _ => 1.5,
            };
            // Wet clothes and fur strip heat faster (#1262): soaked through (wet 1.0) loses
            // heat at 1.8x the dry rate.
            loss *= 1.0 + 0.8 * a.wet;
            if temp < cold_below - 7.0 {
                loss *= 1.4;
            }
            if a.activity.is_asleep() {
                loss *= 1.3;
            }
            if huddled {
                loss *= 0.6;
            }
            // A small body loses heat faster than a big one (genetics.rs, P1).
            loss *= a.heat_loss();
            a.needs.warmth = (a.needs.warmth - loss).max(0.0);
        } else {
            a.needs.warmth = (a.needs.warmth + 0.2).min(1.0);
        }
        if temp > hot_above && !inside && !a.activity.is_asleep() {
            // Heat wears them down: comfort goes, and far above their comfort, rest too.
            // A grown tree's canopy is shade, even outdoors (OST-900): halve the
            // degrees over comfort when standing on the ground under one.
            let shaded = p.z == w.grid.ground_z(p.x, p.y) + 1
                && [
                    (-1, -1),
                    (-1, 0),
                    (-1, 1),
                    (0, -1),
                    (0, 0),
                    (0, 1),
                    (1, -1),
                    (1, 0),
                    (1, 1),
                ]
                .iter()
                .any(|(dx, dy)| tree_columns.contains(&(p.x + dx, p.y + dy)));
            let mut over = temp - hot_above;
            if shaded {
                over /= 2.0;
            }
            a.needs.comfort = (a.needs.comfort - 0.02 * (1.0 + over / 5.0)).max(0.0);
            if over > 8.0 {
                a.needs.rest = (a.needs.rest - 0.01).max(0.0);
            }
            // Only shade quiets the thought; open ground keeps it at any heat over comfort.
            if !shaded || over > 4.0 {
                a.thoughts.add(ThoughtKind::Hot, tick, None);
            }
        }
        if a.needs.warmth < 0.35 {
            a.thoughts.add(ThoughtKind::Cold, tick, None);
        }
        // Frostbite first; death only after hours frozen with it already set in.
        let frost_since = a
            .body
            .wounds
            .iter()
            .filter(|wd| wd.kind == WoundKind::Frostbite)
            .map(|wd| wd.since)
            .min();
        match frost_since {
            Some(since)
                if a.needs.warmth <= 0.0 && tick.saturating_sub(since) >= TICKS_PER_HOUR * 8 =>
            {
                dead.push(a.id);
            }
            None if a.needs.warmth < 0.1 => {
                crate::systems::health::wound(
                    w,
                    i,
                    Part::ARM_R,
                    WoundKind::Frostbite,
                    0.3,
                    0.0,
                    "from the cold",
                    None,
                );
            }
            _ => {}
        }
    }
    for id in dead {
        crate::systems::life::die(w, id, "froze".into());
    }
}

/// Which warmth bucket a tile falls in, for the `warmth_census` example (#1368): the shelter
/// categories #1371-#1373 need a before/after count for, so changes there show up here too.
///
/// - "hearth": walled and roofed, enclosed, with the same warmth source `cold_step` treats as
///   `warm_near` (a lit hearth within 4, or a fire within 3).
/// - "roofed, no hearth": roofed and enclosed, with no such warmth near.
/// - "roofed, open side": roofed, but not enclosed — until #1369 lands, "enclosed" is the same
///   3x3 test `building.rs::roof_old_houses` (lines 375-379 at master `dbf7f557`) uses, applied
///   to any tile rather than only a recorded house's floor: every one of the 8 neighbours is
///   either a built wall or itself roofed.
/// - "tent/nest": standing on a `Tent` or `Nest` structure.
/// - "below ground": `underground::below` — the season never reaches here (#346).
/// - "open": none of the above.
pub fn warmth_bucket(w: &World, p: TilePos) -> &'static str {
    if crate::underground::below(w, p) {
        return "below ground";
    }
    let roofed = crate::building::roofed(w, p);
    if roofed && enclosed(w, p) {
        return if warm_near(w, p) {
            "hearth"
        } else {
            "roofed, no hearth"
        };
    }
    if roofed {
        return "roofed, open side";
    }
    if w.structures
        .iter()
        .any(|s| s.pos == p && matches!(s.kind, StructureKind::Tent | StructureKind::Nest))
    {
        return "tent/nest";
    }
    "open"
}

/// The 3x3 enclosure test `building.rs::roof_old_houses` uses (floor-or-wall on every
/// neighbour), generalised to any tile: a neighbour counts as enclosing when it is a built wall
/// or itself roofed.
fn enclosed(w: &World, p: TilePos) -> bool {
    (-1..=1).all(|dy| {
        (-1..=1).all(|dx| {
            if dx == 0 && dy == 0 {
                return true;
            }
            let q = p.offset(dx, dy, 0);
            w.grid.get(q).shape == Shape::Wall || crate::building::roofed(w, q)
        })
    })
}

/// The same nearby-warmth test `cold_step` uses.
fn warm_near(w: &World, p: TilePos) -> bool {
    w.structures.iter().any(|s| {
        s.kind == StructureKind::Hearth && s.active && s.pos.dist(p) <= 4 && s.pos.z == p.z
    }) || w.fires.iter().any(|f| f.pos.dist(p) <= 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_oak_wall_burns_longer_than_a_pine_one() {
        let mut w = World::generate(5604, crate::world::gen::GenParams::small());
        w.flatten(2, 2, 40, 40, 1);
        let (oak, pine, plain) = (
            TilePos::new(10, 10, 2),
            TilePos::new(12, 10, 2),
            TilePos::new(14, 10, 2),
        );
        for p in [oak, pine, plain] {
            w.grid.set(p, crate::world::Tile::solid(Material::WOOD));
        }
        w.built_of.insert(oak, Material::OAK);
        w.built_of.insert(pine, Material::PINE);
        assert!(
            fuel(&w, oak) > fuel(&w, plain),
            "oak is denser than plain timber"
        );
        assert!(
            fuel(&w, pine) < fuel(&w, oak),
            "pine burns out sooner than oak"
        );
        assert_eq!(
            fuel(&w, plain),
            1000,
            "a wall of unknown wood burns as before"
        );
    }

    /// #1368: every warmth bucket the census needs is distinguishable by `warmth_bucket`.
    /// Without the fix there is no such function to call, so this fails to compile on master.
    #[test]
    fn warmth_bucket_tells_apart_every_shelter() {
        let mut w = World::generate(5604, crate::world::gen::GenParams::small());
        w.flatten(2, 2, 40, 40, 1);
        let z = 2;
        let open = TilePos::new(5, 5, z);
        assert_eq!(warmth_bucket(&w, open), "open");

        // A fully walled 3x3 room with a floor and a roof, no hearth.
        let room = TilePos::new(10, 10, z);
        for dy in -1..=1 {
            for dx in -1..=1 {
                let p = room.offset(dx, dy, 0);
                if dx == 0 && dy == 0 {
                    continue;
                }
                w.grid.set(p, crate::world::Tile::solid(Material::WOOD));
            }
        }
        w.roofs.insert(
            room,
            crate::building::Roof {
                material: Material::THATCH,
                quality: 0.6,
                built: 0,
                snow: 0.0,
            },
        );
        assert_eq!(
            warmth_bucket(&w, room),
            "roofed, no hearth",
            "enclosed and roofed but nothing warm near"
        );

        // The same room, with a lit hearth inside.
        w.structures.push(crate::structures::Structure {
            id: crate::ids::StructureId(0),
            kind: StructureKind::Hearth,
            pos: room,
            material: Material::STONE_BLOCK,
            progress: 1.0,
            active: true,
            condition: 1.0,
            owner: None,
            quality: 0.5,
        });
        assert_eq!(warmth_bucket(&w, room), "hearth");

        // Knock one wall tile out: the room is roofed but no longer enclosed.
        w.grid.set(room.offset(1, 0, 0), crate::world::Tile::AIR);
        assert_eq!(warmth_bucket(&w, room), "roofed, open side");
    }
}
