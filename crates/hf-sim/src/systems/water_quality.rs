//! Rot and the dead foul the water (part 2 of #453, epic #477). A water tile beside food well
//! on its way to rotten (`rot >= 0.6`, systems/spoil.rs) or beside a body whose rite has not
//! yet been done (systems/rites.rs `rites_due`: the dead lie where they fell until someone
//! comes to lay them to rest) is foul for a day at most, topped up each hour the source lies
//! there. Clearing is by flow and time: a running stream keeps little of its foulness from
//! hour to hour (about a two-hour half-life, clear in under a day from the worst), standing
//! water — a pond, a lake, a pond a flood left — loses about an hour of fouling an hour, so
//! it persists about a day, and flood water is cleaned by its own going down: the entry goes
//! with the tile. Salt and brackish water foul and clear the same as fresh — kept simple on
//! purpose (FIXLOG 2026-10-30 #453 part 2). Hours are game-hours (docs/TIME.md: 1 game-hour
//! is 1,200 ticks). Runs once a game hour, at the same tick of the hour as the waters' own
//! step (hazards.rs `flood_step`), after it in the pipeline.

use crate::floods::{body_at, top_water, WaterKind};
use crate::ids::TilePos;
use crate::items::ItemPlace;
use crate::time::TICKS_PER_HOUR;
use crate::world_state::World;

/// The hours a fouled tile stays foul while the water stands: set fresh with each fouling
/// hour, so a pond beside a corpse lies foul for about a day after the body is gone.
pub const FOUL_HOURS: u16 = 24;
/// The share of an hour's fouling running water keeps: 0.71 an hour is a half-life of about
/// two hours (0.71² ≈ 0.5), so a stream fouled to the worst is clean in under a day and a
/// light fouling in hours.
const STREAM_KEEP: u32 = 71;
/// The share standing water keeps: about an hour of fouling lost an hour, so a fouled pond or
/// lake takes about a day to clear.
const STANDING_KEEP: u32 = 97;

pub fn step(w: &mut World) {
    if w.meta.tick % TICKS_PER_HOUR != 300 {
        return; // the same tick of the hour as floods::hour (hazards.rs `flood_step`)
    }
    decay(w);
    foul(w);
}

/// Fouling sources this hour: food at `rot >= 0.6` lying on the ground, and the dead whose
/// rite has not yet been done, lying where they fell.
fn sources(w: &World) -> Vec<TilePos> {
    let mut out: Vec<TilePos> = w
        .items
        .iter()
        .filter(|it| it.kind.is_food() && it.rot >= 0.6)
        .filter_map(|it| match it.place {
            ItemPlace::Ground(p) => Some(p),
            _ => None,
        })
        .collect();
    for r in w.rites_due.iter().filter(|r| !r.away) {
        if let Some(a) = w.agents.iter().find(|a| a.id == r.dead && !a.alive) {
            out.push(a.pos);
        }
    }
    out
}

/// Each source fouls the water of the columns around it, one tile out (Chebyshev), with water
/// at about its own level.
fn foul(w: &mut World) {
    for s in sources(w) {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let Some(wp) = top_water(&w.grid, s.x + dx, s.y + dy) else {
                    continue;
                };
                if (wp.z - s.z).abs() > 1 {
                    continue;
                }
                w.waters
                    .foul
                    .entry(wp)
                    .and_modify(|h| *h = (*h).max(FOUL_HOURS))
                    .or_insert(FOUL_HOURS);
            }
        }
    }
}

/// An hour takes its share of the fouling off every fouled tile, by how the water there
/// moves; a tile whose water is gone (the flood went down, the pool dried) loses its entry.
fn decay(w: &mut World) {
    let entries: Vec<(TilePos, u16)> = w.waters.foul.iter().map(|(p, h)| (*p, *h)).collect();
    for (p, hours) in entries {
        if !w.grid.get(p).is_water() {
            w.waters.foul.remove(&p);
            continue;
        }
        let running = w.flooded.contains(&p)
            || matches!(body_at(w, p).map(|b| b.kind), Some(WaterKind::Stream));
        let keep = if running { STREAM_KEEP } else { STANDING_KEEP };
        let left = (hours as u32 * keep / 100) as u16;
        if left == 0 {
            w.waters.foul.remove(&p);
        } else {
            w.waters.foul.insert(p, left);
        }
    }
}
