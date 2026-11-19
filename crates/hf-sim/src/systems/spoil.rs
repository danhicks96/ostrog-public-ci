//! Food goes off. Each food kind lasts its own number of days lying in the open in mild weather
//! (items.rs `keeps_days`); under a roof it lasts about three times as long, in a barrel, crate,
//! chest or pots (C34) about seven times, in frost about three times, in rain two thirds as long. A stack
//! that has gone off loses a quarter of itself at a time to rot, recorded in the ledger, and the
//! feed says once a day what rotted. Runs once a game hour.

use std::collections::BTreeMap;

use crate::events::{EventKind, Severity};
use crate::items::ItemPlace;
use crate::structures::StructureKind;
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::world_state::World;

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_HOUR != 37 {
        return;
    }
    // The valley reading: the datum the rot threshold compares against where the food has
    // no ground position (carried), and the base for the lapse where it does (#268).
    let valley = w.weather.temperature_c;
    let raining = w.weather.sky.wet();
    let kept: std::collections::BTreeSet<_> = w
        .structures
        .iter()
        .filter(|s| {
            s.progress >= 1.0
                && matches!(
                    s.kind,
                    StructureKind::Barrel | StructureKind::Crate | StructureKind::Chest
                )
        })
        .map(|s| s.pos)
        .collect();
    // Food on a tile where pots stand is kept in them (C34).
    let mut kept = kept;
    kept.extend(w.items.iter().filter_map(|it| match it.place {
        ItemPlace::Ground(p) if it.kind == crate::items::ItemKind::Vessel => Some(p),
        _ => None,
    }));
    let mut rotted: Vec<(usize, u32)> = Vec::new();
    for k in 0..w.items.len() {
        let it = &w.items[k];
        let Some(days) = it.kind.keeps_days() else {
            continue;
        };
        // A thing with a story does not rot: a trophy is tanned, a keepsake kept (FIXLOG
        // 2026-10-20 #170, C62, Dan's call: the White Hart's hide rotted in twenty days, unnamed).
        if it.story.is_some() {
            continue;
        }
        // On the ground, or carried in someone's hands, where it rots as in the open (#177:
        // food a hauler kept in hand after an abort never rotted).
        let p = match it.place {
            ItemPlace::Ground(p) => Some(p),
            ItemPlace::Carried(_) => None,
            _ => continue,
        };
        let mut rate = 1.0 / (days * 24.0);
        if p.is_some_and(|p| kept.contains(&p)) {
            rate *= 0.15;
        } else if p.is_some_and(|p| crate::building::roofed(w, p)) {
            rate *= 0.35;
        } else if raining {
            rate *= 1.5;
        }
        // The cold that preserves is the cold where the food lies (#268): carried food
        // rots at the valley reading, ground food at its own height's.
        let temp = p.map(|p| w.temp_at(p)).unwrap_or(valley);
        if temp < 2.0 {
            rate *= 0.3;
        }
        let it = &mut w.items[k];
        it.rot += rate;
        if it.rot >= 1.0 {
            it.rot = 0.75;
            let lose = it.count.div_ceil(4);
            rotted.push((k, lose));
        }
    }
    // Remove what rotted (highest index first, so the indices stay good), and remember how
    // much of each kind for the day's line.
    let mut today: BTreeMap<String, u32> = BTreeMap::new();
    for (k, lose) in rotted.into_iter().rev() {
        let (id, kind) = (w.items[k].id, w.items[k].kind);
        if w.consume_item(id, lose, "rotted") {
            // The kind's words, not its variant (#250: "2 grainsheaf, 1 rawmeat").
            *today.entry(kind.label().to_string()).or_default() += lose;
        }
    }
    for (kind, n) in today {
        *w.rotted_today.entry(kind).or_default() += n;
    }
    // Once a day, at dusk, the day's rot in one line.
    if tick % TICKS_PER_DAY == 18 * TICKS_PER_HOUR + 37 && !w.rotted_today.is_empty() {
        let what: Vec<String> = std::mem::take(&mut w.rotted_today)
            .into_iter()
            .map(|(k, n)| format!("{n} {k}"))
            .collect();
        let at = w.meta.founding_site;
        w.events.push(
            tick,
            Some(at),
            Severity::Minor,
            EventKind::Rotted {
                what: what.join(", "),
            },
        );
    }
}
