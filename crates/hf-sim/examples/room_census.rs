//! OST-1309: a census of the valley's rooms, per house, at days 100, 300 and 600. Read-only: it
//! only looks at the world. The alive report has no day section yet, so this example prints the
//! numbers that section is to carry. For each house in `Community.house_sites`:
//! floor tiles; beds and sleepers (the most people asleep in the house's beds in any game hour
//! of that day, since nobody keeps a bed); stools, chairs, tables and chests set on the floor;
//! lamps and hearths on the floor; the main wall material and `fine_stone`; and loose non-food
//! stacks lying on the floor. The room score (OST-1402) is not printed.
//! After the rows it prints the median and spread (min, max) of floor tiles per sleeper and
//! fittings per sleeper (beds, seats, tables, chests, lamps, hearths) over houses with a sleeper.
//!
//! Run: cargo run --release -p hf-sim --example room_census [seed [days...]]
//! (default seed 1, days 100 300 600). The tests (`tests/room_census.rs`) include this file and
//! call `census` directly, so the numbers printed here are the ones tested.
#![allow(dead_code)]
use std::collections::BTreeMap;

use hf_sim::ids::{StructureId, TilePos};
use hf_sim::mind::Activity;
use hf_sim::structures::StructureKind;
use hf_sim::world::Material;
use hf_sim::World;

/// One house's row.
#[derive(Clone, Debug, PartialEq)]
pub struct Room {
    pub origin: TilePos,
    pub floor: usize,
    pub beds: usize,
    pub sleepers: usize,
    pub stools: usize,
    pub chairs: usize,
    pub tables: usize,
    pub chests: usize,
    pub lamps: usize,
    pub hearths: usize,
    /// The commonest built wall material and how many of the wall tiles it is.
    pub wall: Option<(String, usize)>,
    pub fine_stone: Option<String>,
    /// Loose non-food stacks lying on the floor, and the units in them.
    pub stacks: usize,
    pub units: u32,
}

impl Room {
    pub fn fittings(&self) -> usize {
        self.beds
            + self.stools
            + self.chairs
            + self.tables
            + self.chests
            + self.lamps
            + self.hearths
    }
}

/// Beds of the house with someone asleep in them right now, counted per person.
fn asleep_in(w: &World, beds: &[StructureId]) -> usize {
    w.agents
        .iter()
        .filter(|a| {
            a.alive
                && matches!(a.activity, Activity::Sleeping { bed: Some(b), .. } if beds.contains(&b))
        })
        .count()
}

/// The ids of the beds standing on each house's floor, in `house_sites` order.
fn bed_ids(w: &World) -> Vec<Vec<StructureId>> {
    let mut out = Vec::new();
    for c in &w.communities {
        for o in &c.house_sites {
            let floor = hf_sim::houses::plan(w, *o).floor;
            out.push(
                w.structures
                    .iter()
                    .filter(|s| s.kind == StructureKind::Bed && floor.contains(&s.pos))
                    .map(|s| s.id)
                    .collect(),
            );
        }
    }
    out
}

/// The census of every house now. `sleepers` is the people asleep in its beds at this moment;
/// the caller raises it to the day's maximum with `max_sleepers`.
pub fn census(w: &World) -> Vec<Room> {
    let beds = bed_ids(w);
    let mut rows = Vec::new();
    let mut k = 0;
    for c in &w.communities {
        for o in &c.house_sites {
            let plan = hf_sim::houses::plan(w, *o);
            let on = |kind: StructureKind| {
                w.structures
                    .iter()
                    .filter(|s| s.kind == kind && plan.floor.contains(&s.pos))
                    .count()
            };
            let mut of: BTreeMap<Material, usize> = BTreeMap::new();
            for (q, _) in hf_sim::houses::walls(&plan.floor) {
                if let Some(m) = w.made_of(q) {
                    *of.entry(m).or_default() += 1;
                }
            }
            let wall = of
                .iter()
                .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
                .map(|(m, n)| (w.grid.materials.get(*m).name.clone(), *n));
            let fine_stone = plan
                .floor
                .first()
                .and_then(|p| hf_sim::houses::fine_stone(w, *p))
                .map(|m| w.grid.materials.get(m).name.clone());
            let (mut stacks, mut units) = (0, 0);
            for i in &w.items {
                if let hf_sim::items::ItemPlace::Ground(p) = i.place {
                    if !i.kind.is_food() && plan.floor.contains(&p) {
                        stacks += 1;
                        units += i.count;
                    }
                }
            }
            rows.push(Room {
                origin: *o,
                floor: plan.floor.len(),
                beds: beds[k].len(),
                sleepers: asleep_in(w, &beds[k]),
                stools: on(StructureKind::Stool),
                chairs: on(StructureKind::Chair),
                tables: on(StructureKind::Table),
                chests: on(StructureKind::Chest),
                lamps: on(StructureKind::Lamp),
                hearths: on(StructureKind::Hearth),
                wall,
                fine_stone,
                stacks,
                units,
            });
            k += 1;
        }
    }
    rows
}

/// Median and (min, max) of a list; None when empty.
pub fn spread(mut v: Vec<f64>) -> Option<(f64, f64, f64)> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let n = v.len();
    let med = if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    };
    Some((med, v[0], v[n - 1]))
}

pub fn print(day: u64, rows: &[Room]) {
    println!("DAY {day}: {} houses", rows.len());
    for r in rows {
        let wall = r
            .wall
            .as_ref()
            .map_or("none standing".to_string(), |(m, n)| format!("{m} x{n}"));
        println!(
            "  house ({},{},{}): floor {} beds {} sleepers {} stools {} chairs {} tables {} chests {} lamps {} hearths {} wall {} fine_stone {} loose {} stacks ({} units)",
            r.origin.x, r.origin.y, r.origin.z, r.floor, r.beds, r.sleepers, r.stools, r.chairs,
            r.tables, r.chests, r.lamps, r.hearths, wall,
            r.fine_stone.as_deref().unwrap_or("-"), r.stacks, r.units
        );
    }
    let slept: Vec<&Room> = rows.iter().filter(|r| r.sleepers > 0).collect();
    let line = |name: &str, v: Vec<f64>| match spread(v) {
        Some((m, lo, hi)) => println!(
            "  {name} per sleeper: median {m:.2} (min {lo:.2}, max {hi:.2}) over {} houses",
            slept.len()
        ),
        None => println!("  {name} per sleeper: n/a (no house with a sleeper)"),
    };
    line(
        "floor tiles",
        slept
            .iter()
            .map(|r| r.floor as f64 / r.sleepers as f64)
            .collect(),
    );
    line(
        "fittings",
        slept
            .iter()
            .map(|r| r.fittings() as f64 / r.sleepers as f64)
            .collect(),
    );
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let seed: u64 = a.first().and_then(|s| s.parse().ok()).unwrap_or(1);
    let mut days: Vec<u64> = a.iter().skip(1).filter_map(|s| s.parse().ok()).collect();
    if days.is_empty() {
        days = vec![100, 300, 600];
    }
    let mut w = World::from_setup(&hf_sim::setup::WorldSetup::new(seed));
    println!("ROOMS seed {seed}");
    let mut day = 0;
    for target in days {
        // Run to the day before, then watch the last day hour by hour for sleepers.
        while day + 1 < target {
            for _ in 0..hf_sim::time::TICKS_PER_DAY {
                w.step();
            }
            day += 1;
        }
        let mut most: Vec<usize> = Vec::new();
        for _ in 0..hf_sim::time::TICKS_PER_DAY / hf_sim::time::TICKS_PER_HOUR {
            for _ in 0..hf_sim::time::TICKS_PER_HOUR {
                w.step();
            }
            let now = census(&w);
            if most.len() < now.len() {
                most.resize(now.len(), 0);
            }
            for (m, r) in most.iter_mut().zip(&now) {
                *m = (*m).max(r.sleepers);
            }
        }
        day += 1;
        let mut rows = census(&w);
        for (r, m) in rows.iter_mut().zip(&most) {
            r.sleepers = r.sleepers.max(*m);
        }
        print(target, &rows);
    }
}
