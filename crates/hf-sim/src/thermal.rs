//! The season's grip (#346): people grow hardened to the cold by a winter out in it and lose it
//! in the warm months; a hard frost kills what is growing in the fields.

use crate::events::{EventKind, Severity};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::World;

/// Below this the night kills a field's growing crop.
pub const KILLING_FROST: f32 = -5.0;

/// Degrees Celsius per level of height (#268): the air above the founding site's ground
/// is this much colder per level, and warmer by the same per level dug below it — the
/// lapse `hazards::cold_step` always applied to warmth, now the one temperature rule
/// everywhere (the block facts long displayed the same 1.5).
pub const LAPSE_C_PER_LEVEL: f32 = 1.5;

/// The air temperature at height `z` when the valley reads `valley_c`, from the ground
/// of the founding site as the datum.
pub fn lapse_c(valley_c: f32, base_z: i32, z: i32) -> f32 {
    valley_c - LAPSE_C_PER_LEVEL * (z as f32 - base_z as f32)
}

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_HOUR != 555 {
        return;
    }
    // The valley reading and the founding site's ground: the frost and the hardening are
    // reckoned per tile and per person from these (#268's lapse). The gate is the coldest
    // any surface tile can be, on the highest ground, so a marginal night that is no frost
    // on the valley floor can still take a field on a height.
    let valley = w.weather.temperature_c;
    let base = w
        .grid
        .ground_z(w.meta.founding_site.x, w.meta.founding_site.y);
    let high = w.grid.ground.iter().max().copied().unwrap_or(base);
    let shoulder = matches!(
        w.calendar().season,
        crate::time::Season::Spring | crate::time::Season::Autumn
    );
    if shoulder
        && lapse_c(valley, base, high + 1) < KILLING_FROST
        && (tick % TICKS_PER_DAY) / TICKS_PER_HOUR == 4
    {
        let fields: Vec<(crate::ids::CommunityId, Vec<crate::ids::TilePos>)> = w
            .communities
            .iter()
            .filter(|c| !c.dissolved)
            .map(|c| (c.id, c.field_tiles.clone()))
            .collect();
        for (cid, tiles) in fields {
            let mut lost = 0;
            for p in w.plants.iter_mut() {
                // Rye stands the frost (#286).
                if p.alive
                    && (p.kind == crate::plants::PlantKind::Root
                        || (p.kind.is_crop() && !crate::plants::frost_hardy(p.kind)))
                    && p.growth < 1.0
                    && tiles.iter().any(|t| t.x == p.pos.x && t.y == p.pos.y)
                    // High ground frosts first (#268): a field on a height can be taken
                    // on a night a field on the valley floor survives.
                    && lapse_c(valley, base, p.pos.z) < KILLING_FROST
                {
                    p.alive = false;
                    lost += 1;
                }
            }
            if lost > 0 {
                let at = w.communities.iter().find(|c| c.id == cid).map(|c| c.home);
                w.events.push(
                    tick,
                    at,
                    Severity::Notable,
                    EventKind::Frosted {
                        community: cid,
                        plants: lost,
                    },
                );
            }
        }
    }
    // Once a day: hardening to the cold, or losing it, in the cold they are actually in
    // (#268: a keeper of the heights hardens sooner than the valley's reading suggests).
    if tick % TICKS_PER_DAY != 12 * TICKS_PER_HOUR + 555 {
        return;
    }
    for a in w.agents.iter_mut().filter(|a| a.alive) {
        let cold_below = a.species_def().climate.comfort_c[0];
        let temp = lapse_c(valley, base, a.pos.z);
        if temp < cold_below && a.needs.warmth < 0.9 {
            a.hardened = (a.hardened + 0.3).min(4.0);
        } else if temp > cold_below + 10.0 {
            a.hardened = (a.hardened - 0.2).max(0.0);
        }
    }
}
