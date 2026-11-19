//! Desire paths (#84): ground walked often wears into a path of packed earth, quicker to walk;
//! one left alone grows back. Footsteps are counted in movement.rs (`World::wear`).

use crate::events::{EventKind, Severity};
use crate::ids::TilePos;
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::world::Material;
use crate::World;

/// Footsteps (fading by a tenth a day) that wear grass to a path.
pub const WORN: u16 = 60;
/// Below this a path grows back.
pub const GROWN: u16 = 8;
/// Wear at or above this (a third of the way to a path) kills a young tree standing on the
/// tile, as Dwarf Fortress does (#1830).
pub const TRAMPLE: u16 = 20;

fn soft(m: Material) -> bool {
    // M6 ground ecology (#493, FIXLOG 2026-10-30): any loose surface ground can become a
    // compacted desire path; already-compacted earth is the result, not another source material.
    m.is_loose_ground() && m != Material::PACKED_EARTH
}

/// Once a day: wear fades, worn ground becomes path, unused path grows back.
pub fn step(w: &mut World) {
    if w.meta.tick % TICKS_PER_DAY != 2 * TICKS_PER_HOUR + 77 {
        return;
    }
    let keys: Vec<(i32, i32)> = w.wear.keys().copied().collect();
    for (x, y) in keys {
        let n = w.wear[&(x, y)];
        let n = n - n / 10 - u16::from(n > 0 && n < 10);
        let ground = TilePos::new(x, y, w.grid.ground_z(x, y));
        let mut t = w.grid.get(ground);
        if n >= WORN && soft(t.material) {
            w.paths.insert((x, y), t.material);
            t.material = Material::PACKED_EARTH;
            w.grid.set(ground, t);
        } else if n < GROWN {
            if let Some(was) = w.paths.remove(&(x, y)) {
                if t.material == Material::PACKED_EARTH {
                    t.material = was;
                    w.grid.set(ground, t);
                }
            }
        }
        // #1830: heavy traffic kills a young tree on the tile, as Dwarf Fortress does. No
        // roll: the cause is the footsteps already counted.
        if n >= TRAMPLE {
            let mut trampled = false;
            for p in &mut w.plants {
                if p.alive && p.pos == ground && p.kind.is_green_tree() && p.growth < 0.3 {
                    p.alive = false;
                    trampled = true;
                }
            }
            if trampled {
                let tick = w.meta.tick;
                w.events.push(
                    tick,
                    Some(ground),
                    Severity::Trace,
                    EventKind::SaplingTrampled { at: ground },
                );
            }
        }
        if n == 0 {
            w.wear.remove(&(x, y));
        } else {
            w.wear.insert((x, y), n);
        }
    }
}
