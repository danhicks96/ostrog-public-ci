//! Snares (#316): set by someone who has a way with animals, where small game runs or across
//! the way strangers came. Each morning a snare with a hare or a fox near takes it, and is
//! spent; a stranger who walks onto one is caught by the leg.

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, TilePos};
use crate::items::{ItemKind, ItemPlace};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::World;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Trap {
    pub pos: TilePos,
    pub by: AgentId,
    pub set: u64,
}

/// Where to set the next snare: with `hungry`, by the nearest small game within forty tiles;
/// after strangers have raided in the last ten days, at the camp side of home toward them.
pub fn spot(w: &World, home: TilePos, hungry: bool) -> Option<TilePos> {
    let tick = w.meta.tick;
    let raided = w.communities.iter().any(|c| {
        c.hardships.iter().any(|h| {
            h.1.contains("took from the store") && tick.saturating_sub(h.0) < 10 * TICKS_PER_DAY
        })
    });
    if raided {
        if let Some(b) = w.bands.first() {
            let c = b.camp;
            let p = TilePos::new(
                home.x + (c.x - home.x) / 3,
                home.y + (c.y - home.y) / 3,
                home.z,
            );
            let p = w.grid.surface(p.x, p.y);
            if w.grid.walkable(p) && !w.traps.iter().any(|t| t.pos == p) {
                return Some(p);
            }
        }
    }
    if !hungry {
        return None;
    }
    w.agents
        .iter()
        .filter(|a| {
            a.alive
                && matches!(
                    a.species,
                    crate::agents::Species::Hare | crate::agents::Species::Fox
                )
                && a.pos.dist(home) <= 40
                && w.grid.walkable(a.pos)
                && !w.traps.iter().any(|t| t.pos.dist(a.pos) < 4)
        })
        .min_by_key(|a| (a.pos.dist(home), a.id))
        .map(|a| a.pos)
}

/// `i` has set a snare at `at`.
pub fn set(w: &mut World, i: usize, at: TilePos) {
    let tick = w.meta.tick;
    let by = w.agents[i].id;
    w.traps.push(Trap {
        pos: at,
        by,
        set: tick,
    });
    w.agents[i]
        .skills
        .practise(crate::mind::Skill::Animals, 0.3);
    w.events.push(
        tick,
        Some(at),
        Severity::Minor,
        EventKind::SnareSet { agent: by },
    );
}

/// Each hour: a stranger on a snare is caught by the leg; at dawn, a snare with small game
/// within three tiles takes it. A snare is spent when it catches, or after ten days.
pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_HOUR != 333 || w.traps.is_empty() {
        return;
    }
    let dawn = tick % TICKS_PER_DAY == 5 * TICKS_PER_HOUR + 333;
    let traps = std::mem::take(&mut w.traps);
    let mut keep = Vec::new();
    for t in traps {
        if tick.saturating_sub(t.set) > 10 * TICKS_PER_DAY {
            continue;
        }
        // A stranger walking onto it.
        if let Some(j) = w
            .agents
            .iter()
            .position(|a| a.alive && a.band.is_some() && a.pos == t.pos)
        {
            crate::systems::health::wound(
                w,
                j,
                crate::body::Part::LEG_L,
                crate::body::WoundKind::Puncture,
                0.35,
                0.1,
                "caught in a snare",
                None,
            );
            let who = w.agents[j].id;
            w.events.push(
                tick,
                Some(t.pos),
                Severity::Notable,
                EventKind::Snared { agent: who },
            );
            continue;
        }
        if dawn {
            let mut r = crate::rng::stream(
                w.meta.seed,
                &[
                    crate::rng::purpose::ANIMALS,
                    0x3160,
                    t.pos.x as u64,
                    t.pos.y as u64,
                    tick,
                ],
            );
            let game = w.agents.iter().position(|a| {
                a.alive
                    && matches!(
                        a.species,
                        crate::agents::Species::Hare | crate::agents::Species::Fox
                    )
                    && a.pos.z == t.pos.z
                    && a.pos.dist(t.pos) <= 3
            });
            if let Some(g) = game {
                if r.gen_bool(0.6) {
                    let animal = w.agents[g].id;
                    w.agents[g].alive = false;
                    w.agents[g].death = Some("snared".into());
                    w.create_item(ItemKind::RawMeat, 2, ItemPlace::Ground(t.pos), 0.5, "snare");
                    w.events.push(
                        tick,
                        Some(t.pos),
                        Severity::Minor,
                        EventKind::Snared { agent: animal },
                    );
                    continue;
                }
            }
        }
        keep.push(t);
    }
    w.traps = keep;
}
