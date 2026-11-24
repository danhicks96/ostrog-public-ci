//! The census behind OST-1308: how often the planner sends people after things nobody
//! has seen. Read-only: it only looks at the world, it changes nothing (not a sim change).
//!
//! After each hourly planner pass (`tick % TICKS_PER_HOUR == 0`) it diffs `w.designations` for new
//! `Hunt(id)` and `Mine` postings. A hunt is "unseen" when no living member of the posting
//! community holds `Known::Agent(id)` seen within 2 days (`SEEN_WITHIN`, 57,600 ticks). A mine
//! posting is "unseen" when its 8x8 cell (x/8, y/8, z) is not in the community's `explored` set
//! (`society.rs`, written by `perceive::apply`). It also records the posting's level against home's
//! and prints, per seed: postings by kind, the share with an unseen target, the deepest level mined
//! below home (home z minus posting z) and food per head and ore in store at days 100, 300 and 700.
//! "Food" is `ItemKind::is_food` units lying on the ground or carried by living people of the
//! community; "ore" is iron, copper and tin ore units on the ground; both count every community's
//! items on the map, not only the first's (the opening worlds hold one community).
//!
//! The test `tests/livestock.rs::the_planner_census_flags_an_unseen_deer` includes this file and
//! calls `unseen_hunt` directly. Logged in docs/history/FIXLOG.md under OST-1308.
//!
//! Run: cargo run --release -p hf-sim --example planner_census <seed> <days> [engine]
//! (engine: steady (default), relentless or whimsical).
// Fields and functions the example's own `main` does not read are read by the tests.
#![allow(dead_code)]
use hf_sim::engine::Engine;
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::mind::knowledge::Known;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::{Designation, DesignationKind};
use hf_sim::World;
use std::collections::BTreeSet;

/// A hunt target counts as seen when a living member saw it within this many ticks (2 days).
pub const SEEN_WITHIN: u64 = 2 * TICKS_PER_DAY;
/// The days at which food and ore are sampled (the umbrella's guard numbers).
pub const SAMPLE_DAYS: [u64; 3] = [100, 300, 700];

/// Whether a `Hunt(id)` posting's animal is one no living member of its community has seen in the
/// last `SEEN_WITHIN` ticks. None when `d` is not a hunt or names no community.
pub fn unseen_hunt(w: &World, d: &Designation) -> Option<bool> {
    let DesignationKind::Hunt(id) = d.kind else {
        return None;
    };
    let c = w.communities.iter().find(|c| Some(c.id) == d.community)?;
    let seen = c.members.iter().any(|m| {
        w.agents.iter().any(|a| {
            a.id == *m
                && a.alive
                && a.knowledge
                    .map
                    .get(&Known::Agent(id))
                    .is_some_and(|b| w.meta.tick.saturating_sub(b.seen_tick) <= SEEN_WITHIN)
        })
    });
    Some(!seen)
}

/// Whether a `Mine` posting's 8x8 cell is outside its community's explored set.
pub fn unseen_mine(w: &World, d: &Designation) -> Option<bool> {
    if d.kind != DesignationKind::Mine {
        return None;
    }
    let c = w.communities.iter().find(|c| Some(c.id) == d.community)?;
    let p = d.pos;
    Some(
        !c.explored
            .contains(&(p.x.div_euclid(8), p.y.div_euclid(8), p.z)),
    )
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Census {
    pub hunts: u32,
    pub hunts_unseen: u32,
    pub mines: u32,
    pub mines_unseen: u32,
    /// Mine postings on a level other than home's.
    pub mines_off_level: u32,
    /// Most levels below home any mine posting lay (home z minus posting z); 0 if none lower.
    pub deepest_below: i32,
    /// (day, food units per living member, ore units in store) at each of `SAMPLE_DAYS`.
    pub samples: Vec<(u64, f32, u32)>,
    seen_ids: BTreeSet<u32>,
}

impl Census {
    /// Look at the designations now; count each new hunt and mine posting once.
    pub fn observe(&mut self, w: &World) {
        for d in &w.designations {
            if !self.seen_ids.insert(d.id.0) {
                continue;
            }
            if let Some(u) = unseen_hunt(w, d) {
                self.hunts += 1;
                self.hunts_unseen += u as u32;
            } else if let Some(u) = unseen_mine(w, d) {
                self.mines += 1;
                self.mines_unseen += u as u32;
                if let Some(c) = w.communities.iter().find(|c| Some(c.id) == d.community) {
                    let below = c.home.z - d.pos.z;
                    self.mines_off_level += (below != 0) as u32;
                    self.deepest_below = self.deepest_below.max(below);
                }
            }
        }
    }

    pub fn sample(&mut self, w: &World, day: u64) {
        let people = w
            .agents
            .iter()
            .filter(|a| a.alive && a.community.is_some())
            .count()
            .max(1) as f32;
        let mut food = 0u32;
        let mut ore = 0u32;
        for it in &w.items {
            let carried_by_living = match it.place {
                ItemPlace::Carried(a) => w.agents.iter().any(|x| x.id == a && x.alive),
                _ => false,
            };
            if it.kind.is_food() && (matches!(it.place, ItemPlace::Ground(_)) || carried_by_living)
            {
                food += it.count;
            }
            if matches!(it.place, ItemPlace::Ground(_))
                && matches!(
                    it.kind,
                    ItemKind::IronOre | ItemKind::CopperOre | ItemKind::Ore
                )
            {
                ore += it.count;
            }
        }
        self.samples.push((day, food as f32 / people, ore));
    }

    pub fn line(&self, seed: u64, days: u64) -> String {
        let pct = |n: u32, d: u32| {
            if d == 0 {
                "n/a".to_string()
            } else {
                format!("{:.0}%", 100.0 * n as f32 / d as f32)
            }
        };
        let mut s = format!(
            "PLANNER seed {seed} {days}d: hunts {} (unseen {}, {}), mines {} (unseen {}, {}; off home level {}), deepest below home {}",
            self.hunts,
            self.hunts_unseen,
            pct(self.hunts_unseen, self.hunts),
            self.mines,
            self.mines_unseen,
            pct(self.mines_unseen, self.mines),
            self.mines_off_level,
            self.deepest_below,
        );
        for (d, f, o) in &self.samples {
            s += &format!("; day {d}: food/head {f:.1}, ore {o}");
        }
        s
    }
}

pub fn run(seed: u64, engine: Engine, days: u64) -> Census {
    let mut setup = hf_sim::setup::WorldSetup::new(seed);
    setup.engine = engine;
    let mut w = World::from_setup(&setup);
    let mut c = Census::default();
    for _ in 0..days * TICKS_PER_DAY {
        w.step();
        let t = w.meta.tick;
        if t.is_multiple_of(TICKS_PER_HOUR) {
            c.observe(&w);
        }
        if t.is_multiple_of(TICKS_PER_DAY) && SAMPLE_DAYS.contains(&(t / TICKS_PER_DAY)) {
            c.sample(&w, t / TICKS_PER_DAY);
        }
    }
    c
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (Some(seed), Some(days)) = (
        args.get(1).and_then(|a| a.parse::<u64>().ok()),
        args.get(2).and_then(|a| a.parse::<u64>().ok()),
    ) else {
        eprintln!("usage: planner_census <seed> <days> [steady|relentless|whimsical]");
        std::process::exit(2)
    };
    let engine = match args.get(3) {
        Some(e) => e.parse().unwrap_or_else(|e: String| {
            eprintln!("{e}");
            std::process::exit(2)
        }),
        None => Engine::default(),
    };
    println!("{}", run(seed, engine, days).line(seed, days));
}
