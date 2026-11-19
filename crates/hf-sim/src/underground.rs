//! The roof of the deep (Phase 18): what holds a dug gallery up, and what happens when it does
//! not.
//!
//! Rock spans so far on its own: soft sedimentary rock two tiles, granite and slate three,
//! basalt four. A dug tile below ground is held while a rock wall or a timber prop stands
//! within that span of it; natural caves and caverns stand as they formed. Cutting a tile that
//! would leave part of an excavation unheld may bring that whole part down at once, and close
//! under the surface the ground above sinks. Nobody is told to prop a gallery: a miner who
//! knows shoring (from their past, or from watching it done) props first and cuts after, and
//! anyone who lives through a collapse has learned it the hard way.

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, TilePos};
use crate::mind::techniques::{Learned, Technique};
use crate::mind::{MemKind, Subject};
use crate::structures::StructureKind;
use crate::work::DesignationKind;
use crate::world::{Material, Tile};
use crate::World;

/// How far the roof spans unaided, by its rock.
pub fn span(m: Material) -> i32 {
    match m {
        Material::LIMESTONE | Material::SANDSTONE | Material::CLAY | Material::SOIL => 2,
        Material::BASALT | Material::GABBRO | Material::GNEISS | Material::GEM_ROCK => 4,
        _ => 3,
    }
}

fn propped(w: &World, p: TilePos) -> bool {
    w.structures
        .iter()
        .any(|s| s.pos == p && s.kind == StructureKind::Support && s.progress >= 1.0)
}

/// The rock over a tile: the tile above if it is solid, else the column's rock there.
fn roof_rock(w: &World, p: TilePos) -> Material {
    let above = w.grid.get(p.offset(0, 0, 1));
    if above.shape.is_solid() {
        above.material
    } else {
        w.grid.rock_at(w.grid.col(p.x, p.y), p.z + 1)
    }
}

/// Whether the roof over `p` is held, as if `opening` (a solid tile about to be cut) were
/// already open.
pub fn held(w: &World, p: TilePos, opening: Option<TilePos>) -> bool {
    if (!w.dug.contains(&p) && Some(p) != opening) || p.z >= w.grid.ground_z(p.x, p.y) {
        return true;
    }
    let s = span(roof_rock(w, p));
    for dy in -s..=s {
        for dx in -s..=s {
            let q = p.offset(dx, dy, 0);
            if Some(q) == opening {
                continue;
            }
            if w.grid.get(q).shape.is_solid() || propped(w, q) {
                return true;
            }
        }
    }
    false
}

/// The part of the excavation around `p` (same level, connected through dug tiles, within
/// eight) that would stand unheld if `p` were cut.
pub fn unheld_if_opened(w: &World, p: TilePos) -> Vec<TilePos> {
    if p.z >= w.grid.ground_z(p.x, p.y) {
        return Vec::new();
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut todo = vec![p];
    seen.insert(p);
    let mut out = Vec::new();
    while let Some(q) = todo.pop() {
        if !held(w, q, Some(p)) {
            out.push(q);
        }
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let n = q.offset(dx, dy, 0);
            if n.dist(p) <= 8 && w.dug.contains(&n) && !seen.contains(&n) {
                seen.insert(n);
                todo.push(n);
            }
        }
    }
    out
}

/// A miner who knows shoring props before cutting: their own order to set a timber prop where
/// the roof would stand unheld, if there is not one already.
/// Returns false when there is nowhere they can reach to set one: then they will not make the
/// cut at all.
pub fn prop_first(w: &mut World, i: usize, unheld: &[TilePos]) -> bool {
    let me = w.agents[i].pos;
    let open: Vec<TilePos> = unheld
        .iter()
        .copied()
        .filter(|q| !w.grid.get(*q).shape.is_solid())
        .filter(|q| w.nav.as_ref().is_some_and(|n| n.reach(me, *q)))
        .collect();
    let Some(&first) = open.first() else {
        return false;
    };
    let (sx, sy) = open.iter().fold((0, 0), |(x, y), q| (x + q.x, y + q.y));
    let n = open.len() as i32;
    let mid = TilePos::new(sx / n, sy / n, first.z);
    let at = *open.iter().min_by_key(|q| q.dist(mid)).unwrap();
    let pending = w.designations.iter().any(|d| {
        !d.done
            && d.pos.dist(at) <= 1
            && matches!(
                d.kind,
                DesignationKind::BuildStructure(StructureKind::Support, _)
            )
    });
    if pending || propped(w, at) {
        return true;
    }
    let mut d = crate::systems::settlement::make(
        w,
        DesignationKind::BuildStructure(StructureKind::Support, Material::WOOD),
        at,
        8,
    );
    d.owner = Some(w.agents[i].id);
    d.community = w.agents[i].community;
    w.designations.push(d);
    let (tick, who) = (w.meta.tick, w.agents[i].id);
    w.events.push(
        tick,
        Some(at),
        Severity::Minor,
        EventKind::Propped { agent: who, at },
    );
    true
}

/// How far someone caught under a falling roof can scramble to get clear of it.
const SCRAMBLE: i32 = 3;

/// The roof comes down over `tiles`, all at once: they fill with rubble, props there are lost,
/// and where the rock above is thin the ground sinks. Whoever stands under it is hurt once and
/// scrambles clear to the nearest open ground beside the fall; with none within reach they are
/// buried.
pub fn collapse(w: &mut World, i: usize, at: TilePos, tiles: &[TilePos]) {
    let tick = w.meta.tick;
    let by = w.agents[i].id;
    let rock = w.grid.materials.get(roof_rock(w, at)).name.clone();
    let fall: std::collections::BTreeSet<TilePos> = tiles.iter().copied().collect();
    // Whoever is under it, each once.
    let caught: Vec<usize> = (0..w.agents.len())
        .filter(|&k| w.agents[k].alive && fall.contains(&w.agents[k].pos))
        .collect();
    let mut sank = false;
    for &t in &fall {
        w.grid.set(t, Tile::solid(Material::GRAVEL));
        w.dug.remove(&t);
        w.structures.retain(|s| s.pos != t);
        w.plants.retain(|p| p.pos != t);
        for d in w.designations.iter_mut().filter(|d| d.pos == t) {
            d.done = true;
        }
        // Thin rock: the ground above sinks into it.
        let g = w.grid.ground_z(t.x, t.y);
        if g - t.z <= 2 {
            let top = TilePos::new(t.x, t.y, g);
            w.grid.set(top, Tile::solid(Material::GRAVEL));
            let on = top.offset(0, 0, 1);
            w.plants.retain(|p| p.pos != on);
            w.structures.retain(|s| s.pos != on);
            w.roofs.remove(&on);
            sank = true;
        }
    }
    // The open ground beside the fall, now that every tile of it is rubble.
    let mut clear = std::collections::BTreeSet::new();
    for &t in &fall {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let q = t.offset(dx, dy, 0);
                if !fall.contains(&q)
                    && w.grid.walkable(q)
                    && !w.designations.iter().any(|d| {
                        d.pos == q && !d.done && matches!(d.kind, DesignationKind::BuildWall(_))
                    })
                {
                    clear.insert(q);
                }
            }
        }
    }
    let nearest = |p: TilePos| -> Option<TilePos> {
        clear
            .iter()
            .copied()
            .filter(|q| q.dist(p) <= SCRAMBLE)
            .min_by_key(|q| q.dist(p))
    };
    // What lay on the fallen tiles goes with the nearest open ground, or stays under the rubble.
    for it in w.items.iter_mut() {
        if let crate::items::ItemPlace::Ground(p) = it.place {
            if fall.contains(&p) {
                if let Some(q) = nearest(p) {
                    it.place = crate::items::ItemPlace::Ground(q);
                }
            }
        }
    }
    let mut hurt: Vec<AgentId> = Vec::new();
    let mut buried: Vec<AgentId> = Vec::new();
    for k in caught {
        match nearest(w.agents[k].pos) {
            Some(q) => {
                let a = &mut w.agents[k];
                a.pos = q;
                a.from = q;
                a.move_total = 0;
                a.path.clear();
            }
            None => buried.push(w.agents[k].id),
        }
        hurt.push(w.agents[k].id);
        w.agents[k].injured_until = tick + crate::time::TICKS_PER_DAY * 2;
        crate::systems::health::wound(
            w,
            k,
            crate::body::Part::LEG_L,
            crate::body::WoundKind::Break,
            0.6,
            0.2,
            "under the fallen rock",
            None,
        );
    }
    for d in w.designations.iter_mut() {
        if d.pos.dist(at) <= 2 && matches!(d.kind, DesignationKind::Mine) {
            d.done = true;
        }
    }
    let eid = w.events.push(
        tick,
        Some(at),
        Severity::Major,
        EventKind::Collapse {
            by,
            at,
            tiles: tiles.len() as u32,
            rock,
            sank,
            hurt: hurt.clone(),
        },
    );
    for id in buried {
        crate::systems::life::die(w, id, "buried under the fallen rock".into());
    }
    // The hard way: everyone who saw it come down knows now what an unpropped roof does.
    let near: Vec<usize> = (0..w.agents.len())
        .filter(|&k| {
            let a = &w.agents[k];
            a.alive
                && a.species == crate::agents::Species::Person
                && a.pos.z == at.z
                && a.pos.dist(at) <= 10
        })
        .collect();
    for k in near {
        w.agents[k].remember(
            tick,
            MemKind::Hardship,
            Subject::Place(at),
            -0.8,
            0.9,
            Some(eid),
        );
        if !w.agents[k].techniques.knows(Technique::Shoring) {
            w.agents[k]
                .techniques
                .learn(Technique::Shoring, Learned::Tried, 0.2, tick);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Light and the dark.

/// Radius of light each source gives, in tiles.
fn radius(kind: StructureKind) -> f32 {
    match kind {
        StructureKind::Hearth => 6.0,
        StructureKind::Lamp => 4.5,
        _ => 4.0,
    }
}

/// The glow of a fungus tree, in tiles.
pub const FUNGUS_GLOW: f32 = 2.6;

/// Rebuild the list of lights below ground: lit hearths, lamps and smelters, fires, and the
/// glowing fungus of the caverns.
pub fn rebuild_lights(w: &mut World) {
    let mut out = Vec::new();
    for s in w
        .structures
        .iter()
        .filter(|s| s.active && s.kind.emits_light())
    {
        out.push((s.pos, radius(s.kind), true));
    }
    for f in &w.fires {
        out.push((f.pos, 4.0, true));
    }
    for p in w
        .plants
        .iter()
        .filter(|p| p.alive && p.kind == crate::plants::PlantKind::FungusTree)
    {
        out.push((p.pos, FUNGUS_GLOW * (0.6 + 0.4 * p.growth), false));
    }
    out.retain(|(p, _, _)| p.z < w.grid.ground_z(p.x, p.y) + 1);
    w.lights = out;
}

/// Whether a tile is below ground (under rock, not merely under a roof).
pub fn below(w: &World, p: TilePos) -> bool {
    p.z <= w.grid.ground_z(p.x, p.y)
}

/// How much light falls on a tile below ground, 0 (dark) to 1. Above ground it is 1: the sky
/// and the people's own fires are another matter (weather.rs, hazards.rs).
pub fn light_at(w: &World, p: TilePos) -> f32 {
    lit(w, p, false)
}

/// Firelight only (lamps, hearths, fires), not the fungus glow: what the blind crawler shuns.
pub fn firelight_at(w: &World, p: TilePos) -> f32 {
    lit(w, p, true)
}

fn lit(w: &World, p: TilePos, fire_only: bool) -> f32 {
    if !below(w, p) {
        return 1.0;
    }
    w.lights
        .iter()
        .filter(|(q, _, fire)| q.z == p.z && (*fire || !fire_only))
        .map(|(q, r, _)| 1.0 - q.dist(p) as f32 / r)
        .fold(0.0, f32::max)
        .max(0.0)
}

/// Dark enough below that nobody can see the rock in front of them.
pub fn dark(w: &World, p: TilePos) -> bool {
    below(w, p) && light_at(w, p) < 0.15
}

/// Hourly: rebuild the lights. Every ten minutes: people below ground in the dark feel it, and
/// the bats stir when people come into their cavern. Every two hours the burrower digs on.
pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick.is_multiple_of(crate::time::TICKS_PER_HOUR) || w.lights.is_empty() {
        rebuild_lights(w);
    }
    if !tick.is_multiple_of(200) {
        return;
    }
    for i in 0..w.agents.len() {
        let a = &w.agents[i];
        if a.alive
            && a.species == crate::agents::Species::Person
            && !a.activity.is_asleep()
            && dark(w, a.pos)
        {
            w.agents[i]
                .thoughts
                .add(crate::mind::ThoughtKind::Dark, tick, None);
            // #322 (2026-10-26 fix-322-search): one brought back from the deep is afraid there.
            if w.agents[i].traits.has(crate::mind::Trait::FearsTheDark) {
                w.agents[i].feelings.feel(
                    crate::mind::feelings::FeelingKind::Fear,
                    None,
                    0.3,
                    tick,
                    None,
                );
            }
        }
    }
    stir_bats(w);
    breaches(w);
    rise(w);
    crate::stirring::step(w);
    if tick.is_multiple_of(2 * crate::time::TICKS_PER_HOUR) {
        burrow(w);
    }
}

/// People coming into a cavern send its bats up, once a day for each cavern.
fn stir_bats(w: &mut World) {
    let tick = w.meta.tick;
    let day = tick / crate::time::TICKS_PER_DAY;
    for k in 0..w.grid.caverns.len() {
        let c = w.grid.caverns[k].clone();
        let Some(who) = w
            .agents
            .iter()
            .find(|a| a.alive && a.species == crate::agents::Species::Person && c.contains(a.pos))
            .map(|a| a.id)
        else {
            continue;
        };
        let key = 0x8A70_0000 + k as u64;
        if w.creatures.stirred.get(&key).is_some_and(|d| *d == day) {
            continue;
        }
        let bats = w
            .agents
            .iter()
            .filter(|a| a.alive && a.species == crate::agents::Species::Bat && c.contains(a.pos))
            .count();
        if bats == 0 {
            continue;
        }
        w.creatures.stirred.insert(key, day);
        w.events.push(
            tick,
            Some(c.centre),
            Severity::Minor,
            EventKind::BatsStirred {
                agent: who,
                cavern: c.name.clone(),
                bats: bats as u32,
            },
        );
    }
}

/// The burrower digs on through the rock, a tile or two at a time. Its tunnels are natural
/// ground (they stand as caves do). Breaking into a dug gallery or cellar is news.
fn burrow(w: &mut World) {
    let tick = w.meta.tick;
    let mut r = crate::rng::stream(w.meta.seed, &[crate::rng::purpose::ANIMALS, 0xB0E0, tick]);
    for i in 0..w.agents.len() {
        let a = &w.agents[i];
        if !a.alive || a.species != crate::agents::Species::Burrower {
            continue;
        }
        let mut at = a.pos;
        let id = a.id;
        let heading = (rand::Rng::gen_range(&mut r, 0..4)) as usize;
        let (dx, dy) = [(1, 0), (-1, 0), (0, 1), (0, -1)][heading];
        for _ in 0..rand::Rng::gen_range(&mut r, 1..=3) {
            let next = at.offset(dx, dy, 0);
            if !w.grid.in_bounds(next)
                || next.x < 2
                || next.y < 2
                || next.x > w.grid.width - 3
                || next.y > w.grid.height - 3
                || !below(w, next.offset(0, 0, 1))
            {
                break;
            }
            let t = w.grid.get(next);
            if t.shape.is_solid() && t.material == Material::GEM_ROCK {
                // The gem-bearing rock is too hard for it: it turns back from that face.
                break;
            } else if t.shape.is_solid() {
                // It digs through, leaving its tunnel open on the rock below.
                w.grid.set(next, Tile::AIR);
                let into = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .map(|(ax, ay)| next.offset(*ax, *ay, 0))
                    .find(|q| w.dug.contains(q));
                if let Some(q) = into {
                    w.events.push(
                        tick,
                        Some(q),
                        Severity::Notable,
                        EventKind::BurrowedIn { agent: id, at: q },
                    );
                }
            } else if t.is_water() {
                break;
            }
            at = next;
        }
        if at != w.agents[i].pos {
            w.agents[i].pos = at;
            w.agents[i].path.clear();
            // Its digging is its going (#388: it never took up an activity, so it read as
            // idle and stuck for a whole run). The walk is over at once (systems/act.rs).
            w.agents[i].set_activity(crate::mind::Activity::Wandering { to: at });
        }
    }
}

/// Someone of the valley stands in a deep cavern for the first time (P10): the breach is a row.
/// Through it come people of the deep settlement nearest under the valley, if any lies within
/// reach, to the community as strangers do; and if a beast of the deep lairs within its reach,
/// those of the community who hear it learn its name and hold that the deep is not to be dug,
/// with the breach as the source. Nothing comes up that the world's record does not hold.
pub fn breaches(w: &mut World) {
    let tick = w.meta.tick;
    for k in 0..w.grid.caverns.len() {
        let c = w.grid.caverns[k].clone();
        if !c.deep || c.opened {
            continue;
        }
        let Some((who, cid)) = w
            .agents
            .iter()
            .find(|a| {
                a.alive
                    && a.species == crate::agents::Species::Person
                    && a.community.is_some()
                    && c.contains(a.pos)
            })
            .map(|a| (a.id, a.community))
        else {
            continue;
        };
        w.grid.caverns[k].opened = true;
        // #322: the way down is theirs who broke in.
        w.grid.caverns[k].gate_of = cid;
        let eid = w.events.push(
            tick,
            Some(c.centre),
            Severity::Major,
            EventKind::Breached {
                agent: who,
                cavern: c.name.clone(),
            },
        );
        // #273: the way they came in, and what the chamber is.
        if let Some(a) = w.agents.iter().find(|a| a.id == who) {
            let entry = if c.contains(a.from) { a.pos } else { a.from };
            w.grid.caverns[k].breach = Some(entry);
        }
        w.events.push(
            tick,
            Some(c.centre),
            Severity::Notable,
            EventKind::CavernFound {
                cavern: c.name.clone(),
                paces: ((2 * c.rx + 1) * (2 * c.ry + 1)) as u32,
                pool: c.pool,
                underworld: c.underworld,
            },
        );
        // Where the underworld's stone shows, the one who stood there first touched it (P11).
        if c.underworld {
            if let Some(i) = w.agents.iter().position(|a| a.id == who) {
                crate::conditions::take(
                    w,
                    i,
                    crate::conditions::ConditionKind::StoneSleep,
                    &format!("since they touched the underworld's rock in {}", c.name),
                    None,
                );
                if let Some(cd) = w.agents[i]
                    .conditions
                    .iter_mut()
                    .find(|x| x.kind == crate::conditions::ConditionKind::StoneSleep)
                {
                    cd.note = c.name.clone();
                    cd.cause = eid;
                }
            }
        }
        let from = deep_neighbour(w);
        if let Some((site, people, culture)) = from {
            if let Some(sp) = w.realm.peoples.get(people as usize).map(|p| p.species) {
                let ids = w.arrive_party(2, c.centre, None);
                for id in &ids {
                    crate::species::give(w, *id, sp, Some(people as usize));
                    crate::species::raise_in(w, *id, culture);
                }
                let home = cid
                    .and_then(|cid| w.communities.iter().find(|x| x.id == cid))
                    .map(|x| x.home);
                for id in &ids {
                    if let (Some(i), Some(home)) = (w.agents.iter().position(|a| a.id == *id), home)
                    {
                        if w.path_agent_to(i, home) {
                            w.agents[i].activity = crate::mind::Activity::Wandering { to: home };
                        }
                    }
                }
                w.events.push(
                    tick,
                    Some(c.centre),
                    Severity::Major,
                    EventKind::FromBelow {
                        agents: ids,
                        from: site.clone(),
                    },
                );
                // #322: a child taken to them before, grown among them, may be with them.
                crate::taken::among_them(w, &site, c.centre);
            }
        }
        let Some(region) = w.meta.setup.as_ref().and_then(|s| s.region) else {
            continue;
        };
        let beast =
            crate::realm::beasts::near(&w.realm, region, "deep_caverns").map(|b| b.name.clone());
        if let Some(beast) = beast {
            w.events.push(
                tick,
                Some(c.centre),
                Severity::Major,
                EventKind::BeastStirred {
                    beast: beast.clone(),
                    cavern: c.name.clone(),
                },
            );
            // C75, #273: it notices, and the valley sets about walling the way. #455: it comes
            // up when the noise made near the way has stirred it enough (stirring.rs), not two
            // days on whatever the valley does.
            w.creatures.stirring.push(crate::stirring::Stir {
                beast: beast.clone(),
                cavern: k,
                level: 0,
                warned: false,
                since: tick,
            });
            if let Some(entry) = w.grid.caverns[k].breach {
                let mut d = crate::systems::settlement::make(
                    w,
                    crate::work::DesignationKind::BuildWall(crate::world::Material::STONE_BLOCK),
                    entry,
                    8,
                );
                d.community = cid;
                w.designations.push(d);
            }
            for a in w
                .agents
                .iter_mut()
                .filter(|a| a.alive && a.community == cid)
            {
                a.beliefs.experience(
                    crate::mind::beliefs::Topic::DeepDigging,
                    -1.0,
                    0.5,
                    crate::mind::beliefs::Source::Event(eid),
                    tick,
                );
            }
        }
    }
}

/// The deep's nearest settlement under the valley, within twelve regions of it: its name, its
/// people and its culture (P10; #322 for the people a society bargains with).
pub fn deep_neighbour(w: &World) -> Option<(String, u32, u32)> {
    let region = w.meta.setup.as_ref().and_then(|s| s.region)?;
    let n = w.realm.map_size.max(1);
    let reach = |r: usize| {
        ((r % n) as i64 - (region % n) as i64)
            .abs()
            .max(((r / n) as i64 - (region / n) as i64).abs())
    };
    w.realm
        .sites
        .iter()
        .filter(|s| s.deep && s.alive() && reach(s.region) <= 12)
        .min_by_key(|s| (reach(s.region), s.id))
        .map(|s| (s.name.clone(), s.people, s.culture))
}

/// Whether a tile lies in the deep stratum, the basalt below the caverns (P10): digging there is
/// what the DeepDigging stance is about.
pub fn in_the_deep(w: &World, p: crate::ids::TilePos) -> bool {
    if !w.grid.in_bounds(p) {
        return false;
    }
    let c = w.grid.col(p.x, p.y);
    w.grid
        .strata
        .get(c)
        .is_some_and(|&(_, _, deep)| p.z <= deep as i32)
}

/// #322 (2026-10-26 fix-322-gate): someone standing at a walled way down cuts the wall out,
/// and the way to the deep is open again. Returns the cavern's index if they did.
pub fn open_gate(w: &mut World, i: usize) -> Option<usize> {
    let at = w.agents[i].pos;
    let k = w
        .grid
        .caverns
        .iter()
        .position(|c| c.sealed && c.breach.is_some_and(|b| b.dist(at) <= 1))?;
    let b = w.grid.caverns[k].breach?;
    w.grid.set(b, Tile::AIR);
    w.grid.caverns[k].sealed = false;
    let cavern = w.grid.caverns[k].name.clone();
    let agent = w.agents[i].id;
    w.events.push(
        w.meta.tick,
        Some(b),
        Severity::Major,
        EventKind::GateOpened { agent, cavern },
    );
    Some(k)
}

/// Dry standing room on a cavern's floor, near its middle first.
pub fn floors<'a>(
    w: &'a World,
    c: &'a crate::world::gen::caverns::Cavern,
) -> impl Iterator<Item = crate::ids::TilePos> + 'a {
    (0..60).filter_map(move |n: i32| {
        let p = crate::ids::TilePos::new(
            c.centre.x + (n % 11) - 5,
            c.centre.y + (n / 11) - 3,
            c.centre.z,
        );
        (c.contains(p) && w.grid.walkable(p) && !w.grid.get(p).is_water()).then_some(p)
    })
}

/// #273: a breach walled up is sealed; a beast stirred by one comes up when its time comes,
/// unless it finds the way walled.
pub fn rise(w: &mut World) {
    let tick = w.meta.tick;
    for k in 0..w.grid.caverns.len() {
        let c = &w.grid.caverns[k];
        let of = c.gate_of;
        if let (Some(b), false) = (c.breach, c.sealed) {
            if w.grid.get(b).shape.is_solid() {
                w.grid.caverns[k].sealed = true;
                // #322 (2026-10-26 fix-322-gate): the one who walled it, standing by the wall
                // when it closes, keeps the way.
                w.grid.caverns[k].keeper = w
                    .agents
                    .iter()
                    .filter(|a| {
                        a.alive
                            && a.species == crate::agents::Species::Person
                            && a.community.is_some()
                            && (of.is_none() || a.community == of)
                            && !a.is_child(tick)
                            && a.pos.dist(b) <= 3
                    })
                    .min_by_key(|a| (a.pos.dist(b), a.id))
                    .map(|a| a.id);
                let cavern = w.grid.caverns[k].name.clone();
                w.events.push(
                    tick,
                    Some(b),
                    Severity::Notable,
                    EventKind::Sealed { cavern },
                );
            }
        }
    }
    let due: Vec<(String, usize, u64)> = w
        .creatures
        .rising
        .iter()
        .filter(|r| r.2 <= tick)
        .cloned()
        .collect();
    w.creatures.rising.retain(|r| r.2 > tick);
    for (beast, k, _) in due {
        let Some(c) = w.grid.caverns.get(k).cloned() else {
            continue;
        };
        if c.sealed {
            w.events.push(
                tick,
                Some(c.centre),
                Severity::Notable,
                EventKind::Barred {
                    beast,
                    cavern: c.name.clone(),
                },
            );
            continue;
        }
        come_up(w, &beast, k);
    }
}

/// A beast of the deep comes up into cavern `k` (C75; #455 stirring.rs).
pub fn come_up(w: &mut World, beast: &str, k: usize) {
    let tick = w.meta.tick;
    let Some(c) = w.grid.caverns.get(k).cloned() else {
        return;
    };
    let floor = floors(w, &c).next();
    if let Some(p) = floor {
        let id = w.spawn_agent(crate::agents::Species::Crawler, p);
        if let Some(b) = w.agents.iter_mut().find(|a| a.id == id) {
            b.name = beast.to_string();
        }
        w.creatures.risen.push(id);
        w.events.push(
            tick,
            Some(p),
            Severity::Major,
            EventKind::Rose {
                beast: id,
                cavern: c.name.clone(),
            },
        );
    }
}
