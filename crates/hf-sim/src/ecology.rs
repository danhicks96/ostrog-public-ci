//! The valley's food web (#486): each wild kind lives on what it eats, so a change in one moves
//! the others. Grazers bear young only when the browse has fed them and starve when it has been
//! gone too long; wolves and foxes bear young only after a kill, and hunters starve when the
//! game is gone; crows bear young where something has died; a carcass, once picked over, greens the ground
//! about it; and where a tree was felled, browse springs up in the light. A valley that loses a
//! kind entirely is found again from the hills when there is food for it.
//!
//! Every link reads a state other systems already keep (a plant's growth, a kill, a carcass, a
//! stump), and every death is a recorded `Died` with its cause. (FIXLOG 2026-11-02 #486.)

use rand::Rng;

use crate::agents::Species;
use crate::events::{EventKind, Severity};
use crate::ids::TilePos;
use crate::plants::{Plant, PlantKind};
use crate::rng::{purpose, stream};
use crate::time::{Season, TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::world_state::World;

/// The wild kinds that live on the browse (ducks feed on the water, which is not modelled, and
/// the White Hart is one of its own).
pub fn browser(s: Species) -> bool {
    matches!(
        s,
        Species::Deer | Species::Boar | Species::Hare | Species::Goat
    )
}

/// The wild kinds that live on other animals.
pub fn hunter(s: Species) -> bool {
    matches!(s, Species::Wolf | Species::Fox)
}

/// A grazer bears young only if it has eaten its fill within this many days.
const BROWSER_YOUNG_DAYS: u64 = 2;
/// A grazer that has not eaten its fill for this many days is starving; in winter, when nothing
/// grows, it lives longer on what it put on in autumn (a season is five days, docs/TIME.md).
const BROWSER_STARVE_DAYS: u64 = 6;
const BROWSER_STARVE_DAYS_WINTER: u64 = 9;
/// A wolf or fox bears young only within this many days of a kill.
const HUNTER_YOUNG_DAYS: u64 = 4;
/// A wolf or fox without a kill for this many days is starving.
const HUNTER_STARVE_DAYS: u64 = 10;
/// Crows bear young only within this many days of coming down on a carcass.
const CROW_YOUNG_DAYS: u64 = 6;
/// A starving animal dies on a given day at this chance.
const STARVE_CHANCE: f32 = 0.3;
/// What a picked-over carcass gives the low growth within two tiles of it.
pub const CARCASS_GREENING: f32 = 0.3;
/// A stump lets a shrub spring up beside it at this chance on a growing day.
const CLEARING_CHANCE: f64 = 1.0 / 15.0;
/// Browse does not spring up where this many plants already stand within two tiles.
const CLEARING_CROWD: usize = 3;

/// The animal `i` has eaten its fill now.
pub fn fed(w: &mut World, i: usize) {
    w.agents[i].fed = w.meta.tick;
}

/// A wolf takes a hare only once a day has passed since its last kill, a fox once two have:
/// a fed one lets the hares be, and a fox lives mostly on mice and beetles, which are not
/// modelled, so it neither lives on the hares alone nor starves without them.
pub fn hungry_hunter(a: &crate::agents::Agent, tick: u64) -> bool {
    let days = if a.species == Species::Fox { 2 } else { 1 };
    tick.saturating_sub(a.fed) >= days * TICKS_PER_DAY
}

/// Whether the wild animal `a` is fit to bear young today: a grazer, wolf, fox or crow only when
/// it has eaten lately; every other kind as before.
pub fn fit_to_bear(a: &crate::agents::Agent, tick: u64) -> bool {
    let days = if browser(a.species) {
        BROWSER_YOUNG_DAYS
    } else if hunter(a.species) {
        HUNTER_YOUNG_DAYS
    } else if a.species == Species::Crow {
        CROW_YOUNG_DAYS
    } else {
        return true;
    };
    a.kept.is_some() || tick.saturating_sub(a.fed) <= days * TICKS_PER_DAY
}

/// The cause a wild animal starving today dies of, if it has gone too long without food.
/// Crows also eat grain and grubs and do not starve for want of carrion.
pub fn starving(a: &crate::agents::Agent, tick: u64, season: Season) -> Option<&'static str> {
    if a.kept.is_some() || a.denned {
        return None;
    }
    let (days, cause) = if browser(a.species) {
        let d = if season == Season::Winter {
            BROWSER_STARVE_DAYS_WINTER
        } else {
            BROWSER_STARVE_DAYS
        };
        (d, "starved, the grazing gone")
    } else if hunter(a.species) {
        (HUNTER_STARVE_DAYS, "starved, the game gone")
    } else {
        return None;
    };
    (tick.saturating_sub(a.fed) > days * TICKS_PER_DAY).then_some(cause)
}

/// Whether a starving animal dies today (a seeded roll per animal and day).
pub fn starves_today(seed: u64, a: &crate::agents::Agent, tick: u64) -> bool {
    let mut r = stream(
        seed,
        &[
            purpose::ECOLOGY,
            0x57A2,
            a.id.0 as u64,
            tick / TICKS_PER_DAY,
        ],
    );
    r.gen::<f32>() < STARVE_CHANCE
}

/// Daily, after the breeding pass: animals the world has not yet seen eat are taken as fed
/// today (new young, old saves), browse springs up by stumps, and a kind gone from the valley
/// comes back over the hills when there is food for it.
pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_DAY != TICKS_PER_HOUR * 4 + 1 {
        return;
    }
    for a in w.agents.iter_mut() {
        if !a.alive || a.species == Species::Person {
            continue;
        }
        // Never seen to eat yet, or a grazer still more than half full: fed as of today.
        if a.fed == 0 || (browser(a.species) && a.needs.hunger >= 0.5) {
            a.fed = tick.max(1);
        }
    }
    clearings(w);
    incomers(w);
}

/// The growth on the low plants grazers eat, above what grazing leaves, over the valley's
/// surface.
pub fn browse(w: &World) -> f32 {
    let floor = crate::systems::animals::GRAZED_DOWN;
    w.plants
        .iter()
        .filter(|p| p.alive && crate::systems::animals::browsed(p.kind))
        .map(|p| (p.growth - floor).max(0.0))
        .sum()
}

/// A carcass picked over and gone (creatures.rs) greens the low growth about it.
pub fn carcass_spent(w: &mut World, at: TilePos) {
    for p in w.plants.iter_mut() {
        if p.alive
            && p.pos.z == at.z
            && p.pos.dist(at) <= 2
            && p.kind.grows()
            && !p.kind.is_crop()
            && !p.kind.yields_wood()
        {
            p.growth = (p.growth + CARCASS_GREENING).min(1.0);
        }
    }
}

/// Where a tree was felled the light comes in, and a shrub springs up beside the stump in the
/// growing seasons, unless the ground there is already crowded, worked or built on.
fn clearings(w: &mut World) {
    let season = w.calendar().season;
    if !matches!(season, Season::Spring | Season::Summer) {
        return;
    }
    let day = w.meta.tick / TICKS_PER_DAY;
    let stumps: Vec<TilePos> = w
        .plants
        .iter()
        .filter(|p| p.alive && p.kind == PlantKind::Stump)
        .map(|p| p.pos)
        .collect();
    for s in stumps {
        let mut r = stream(
            w.meta.seed,
            &[
                purpose::ECOLOGY,
                0xC1EA,
                s.x as u64,
                s.y as u64,
                s.z as u64,
                day,
            ],
        );
        if !r.gen_bool(CLEARING_CHANCE) {
            continue;
        }
        let crowd = w
            .plants
            .iter()
            .filter(|p| p.alive && p.pos.z == s.z && p.pos.dist(s) <= 2)
            .count();
        if crowd >= CLEARING_CROWD {
            continue;
        }
        let k = r.gen_range(0..4);
        let (dx, dy) = [(1, 0), (0, 1), (-1, 0), (0, -1)][k];
        let q = s.offset(dx, dy, 0);
        if !open_ground(w, q) {
            continue;
        }
        let variant = (q.x * 31 + q.y * 17) as u32;
        w.plants
            .push(Plant::new(q, PlantKind::Shrub, 0.05, variant));
    }
}

/// Ground a wild shrub can take: walkable surface on soil, with no plant, structure, item or
/// designation on it and outside every community's field.
fn open_ground(w: &World, q: TilePos) -> bool {
    if !w.grid.in_bounds(q) || !w.grid.walkable(q) || w.grid.surface(q.x, q.y) != q {
        return false;
    }
    if w.grid.get(q).is_water() {
        return false;
    }
    !w.plants.iter().any(|p| p.alive && p.pos == q)
        && !w.structures.iter().any(|s| s.pos == q)
        && !w.designations.iter().any(|d| d.pos == q)
        && !w
            .communities
            .iter()
            .any(|c| !c.dissolved && c.field_tiles.contains(&q))
}

/// A kind gone from the valley comes back from the hills when there is food for it: grazers
/// when the browse has grown thick, wolves and foxes when there is game, crows when something
/// has died. One at a time, at the valley's edge.
fn incomers(w: &mut World) {
    let tick = w.meta.tick;
    let day = tick / TICKS_PER_DAY;
    let count = |w: &World, s: Species| {
        w.agents
            .iter()
            .filter(|a| a.alive && a.species == s && a.kept.is_none())
            .count()
    };
    let browse = browse(w);
    let game = w
        .agents
        .iter()
        .filter(|a| a.alive && browser(a.species) && a.kept.is_none())
        .count();
    let carrion = !w.creatures.carrion.is_empty();
    let wanted: [(Species, bool); 5] = [
        (Species::Deer, browse >= 40.0),
        (Species::Hare, browse >= 20.0),
        (Species::Wolf, game >= 12),
        (Species::Fox, game >= 8),
        (Species::Crow, carrion || game >= 8),
    ];
    for (k, (species, food)) in wanted.into_iter().enumerate() {
        if !food || count(w, species) >= 2 {
            continue;
        }
        let mut r = stream(w.meta.seed, &[purpose::ECOLOGY, 0x1C0E, k as u64, day]);
        if !r.gen_bool(0.2) {
            continue;
        }
        let Some(at) = edge(w, &mut r) else {
            continue;
        };
        let id = w.spawn_agent(species, at);
        if let Some(a) = w.agents.iter_mut().find(|a| a.id == id) {
            a.born = tick as i64 - (crate::time::TICKS_PER_YEAR * 2) as i64;
            a.fed = tick;
        }
        w.events.push(
            tick,
            Some(at),
            Severity::Trace,
            EventKind::CameOver { agent: id },
        );
    }
}

/// Walkable surface near the valley's edge, a few tries from a seeded stream.
fn edge(w: &World, r: &mut rand_chacha::ChaCha8Rng) -> Option<TilePos> {
    let (wd, ht) = (w.grid.width, w.grid.height);
    (0..12).find_map(|_| {
        let side = r.gen_range(0..4);
        let along = |n: i32, r: &mut rand_chacha::ChaCha8Rng| r.gen_range(3..n - 3);
        let (x, y) = match side {
            0 => (along(wd, r), 3),
            1 => (along(wd, r), ht - 4),
            2 => (3, along(ht, r)),
            _ => (wd - 4, along(ht, r)),
        };
        let p = w.grid.surface(x, y);
        w.grid.walkable(p).then_some(p)
    })
}
