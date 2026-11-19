//! Fishing (#299): the valley's waters hold fish, a stock to each stretch of water that is
//! fished down and comes back; when food runs short someone goes to the bank (settlement.rs
//! 7c, a `Fish` order) and comes back with what they caught. In the hard cold the water is
//! ice, and the ice now and then gives way.

use crate::events::{EventKind, Severity};
use crate::ids::TilePos;
use crate::items::{ItemKind, ItemPlace};
use crate::time::TICKS_PER_DAY;
use crate::World;
use rand::Rng;

/// The most a stretch of water (a 16-tile square) holds.
pub const FULL: f32 = 24.0;
/// What comes back to it a day.
pub const REGROW: f32 = 0.6;

fn key(p: TilePos) -> (i32, i32) {
    (p.x / 16, p.y / 16)
}

/// The fish in the stretch of water at `p` now.
pub fn stock(w: &World, p: TilePos) -> f32 {
    match w.fish.get(&key(p)) {
        Some(&(s, since)) => {
            let days = w.meta.tick.saturating_sub(since) as f32 / TICKS_PER_DAY as f32;
            (s + days * REGROW).min(FULL)
        }
        None => FULL,
    }
}

fn take(w: &mut World, p: TilePos, n: f32) {
    let now = (stock(w, p) - n).max(0.0);
    let t = w.meta.tick;
    w.fish.insert(key(p), (now, t));
}

/// Where to fish: the nearest surface water worth the going (`surface_spot`), else the pool of
/// a cavern the valley has opened (`cave_spot`, #336: the blind cave fish of the dark pools).
pub fn spot(w: &World, home: TilePos) -> Option<TilePos> {
    surface_spot(w, home).or_else(|| cave_spot(w, home))
}

/// How far from home a cavern's pool is still worth the going (tiles, 3D distance).
const CAVE_REACH: i32 = 60;

/// The pool of a cavern someone of the valley has stood in (`opened`), not walled up again,
/// within `CAVE_REACH` of `home`: a water tile of it with fish enough and a dry bank that can be
/// walked to from home (#336, WP-05 5.4: before this a pool cavern the valley had dug into was
/// never fished, FIXLOG 2026-11-12). The nearest such tile; None without one.
pub fn cave_spot(w: &World, home: TilePos) -> Option<TilePos> {
    let g = &w.grid;
    let nav = w.nav.as_ref()?;
    let mut best: Option<(i32, TilePos)> = None;
    for c in g.caverns.iter().filter(|c| c.pool && c.opened && !c.sealed) {
        if c.centre.dist(home) > CAVE_REACH {
            continue;
        }
        for z in [c.centre.z - 1, c.centre.z] {
            for y in c.centre.y - c.ry..=c.centre.y + c.ry {
                for x in c.centre.x - c.rx..=c.centre.x + c.rx {
                    let p = TilePos::new(x, y, z);
                    if !g.in_bounds(p) || !g.get(p).is_water() || stock(w, p) < 3.0 {
                        continue;
                    }
                    if !c.contains(TilePos::new(x, y, c.centre.z)) {
                        continue;
                    }
                    let bank = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(a, b)| {
                        let q = p.offset(*a, *b, 0);
                        g.walkable(q) && !g.get(q).is_water() && nav.reach(home, q)
                    });
                    if !bank {
                        continue;
                    }
                    let d = p.dist(home);
                    if best.is_none_or(|b| d < b.0) {
                        best = Some((d, p));
                    }
                }
            }
        }
    }
    best.map(|b| b.1)
}

/// A water tile within forty of `home`, at the surface, with a bank someone can stand on and
/// fish enough to be worth the going.
pub fn surface_spot(w: &World, home: TilePos) -> Option<TilePos> {
    let g = &w.grid;
    let mut best: Option<(i32, TilePos)> = None;
    for dy in -40..=40 {
        for dx in -40..=40 {
            let (x, y) = (home.x + dx, home.y + dy);
            if x < 1 || y < 1 || x >= g.width - 1 || y >= g.height - 1 {
                continue;
            }
            let z = g.ground_z(x, y) + 1;
            let p = TilePos::new(x, y, z);
            if !g.get(p).is_water() || stock(w, p) < 3.0 {
                continue;
            }
            let bank = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(a, b)| {
                g.walkable(p.offset(*a, *b, 0)) && !g.get(p.offset(*a, *b, 0)).is_water()
            });
            if !bank {
                continue;
            }
            let d = p.dist(home);
            if best.is_none_or(|b| d < b.0) {
                best = Some((d, p));
            }
        }
    }
    best.map(|b| b.1)
}

/// A morning's fishing at `water` by `i` is done.
pub fn caught(w: &mut World, i: usize, water: TilePos) {
    let tick = w.meta.tick;
    let who = w.agents[i].id;
    let skill = w.agents[i].skills.level(crate::mind::Skill::Animals);
    let mut r = crate::rng::stream(
        w.meta.seed,
        &[crate::rng::purpose::AGENT_WORK, who.0 as u64, tick, 0xF15A],
    );
    let ice = w.temp_at(water) < -2.0;
    if ice && r.gen_bool(0.01) {
        crate::systems::life::die(w, who, "drowned through the ice".into());
        return;
    }
    let s = stock(w, water);
    let n = ((s / FULL) * r.gen_range(1.0..4.0) * (1.0 + skill / 20.0)).floor() as u32;
    let n = if ice { n / 2 } else { n };
    take(w, water, n as f32);
    let at = w.agents[i].pos;
    if n > 0 {
        let id = w.create_item(ItemKind::RawMeat, n, ItemPlace::Ground(at), 0.5, "fish");
        w.agents[i].knowledge.note(
            crate::mind::knowledge::Known::Item(id),
            at,
            tick,
            ItemKind::RawMeat as u16,
        );
    }
    w.agents[i]
        .skills
        .practise(crate::mind::Skill::Animals, 0.5);
    w.events.push(
        tick,
        Some(at),
        Severity::Minor,
        EventKind::Fished {
            agent: who,
            count: n,
            ice,
        },
    );
}
