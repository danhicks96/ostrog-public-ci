//! The census behind #1120 (under #1119, epic #1531): what becomes of heirlooms and treasured
//! things. Read-only: it only looks at the world, it changes nothing.
//!
//! Every `SAMPLE_DAYS` game-days, for every storied item (`it.story.is_some()`), it counts where
//! the item sits (`Held`, `Carried`, on the `Ground`, or on the ground inside a `Stockpile`
//! structure, counted as "in store"), how many are cherished and how many are heirlooms, and for
//! each heirloom whether its last owner (`Story::owners.last()`) is alive and still holding it
//! (`Held` or `Carried` by them). It also tallies every `LostTreasure` thought
//! (`mind::ThoughtKind::LostTreasure`, added only when a cherished or named tool or garment
//! breaks or wears through) by cause, found by matching the thought's `since` tick against a
//! `ToolBroke` or `NewClothes` event naming the same agent on the same tick; a thought with no
//! matching event (which should not happen, since both call sites push one) counts as "unknown".
//!
//! Logged in docs/history/FIXLOG.md under "#1120: nothing counted what becomes of heirlooms and
//! treasured things". The test (`tests/legend.rs::possessions_census_sees_an_heirloom_set_down`)
//! includes this file and calls `Census::sample`/`Census::note` and `run` directly, so the
//! numbers printed here are the ones the test checks.
//!
//! Run: cargo run --release -p hf-sim --example possessions_census [days [from to]]
//! (default 600 days, seeds 5 to 8, as #1119's baseline needs).
// Fields and functions the example's own `main` does not read are read by the test.
#![allow(dead_code)]
use hf_sim::events::{Event, EventKind};
use hf_sim::items::ItemPlace;
use hf_sim::mind::ThoughtKind;
use hf_sim::structures::StructureKind;
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::World;
use std::collections::BTreeMap;

/// How often the item census is sampled.
pub const SAMPLE_DAYS: u64 = 50;

/// One sample's placement and heirloom tallies.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sample {
    pub tick: u64,
    pub held: u32,
    pub carried: u32,
    pub ground: u32,
    pub stored: u32,
    pub cherished: u32,
    pub heirloom: u32,
    /// Of the heirlooms, those whose last owner is alive and still holds the thing.
    pub heirloom_with_heir: u32,
}

/// One seed's tallies.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Census {
    pub samples: Vec<Sample>,
    /// `LostTreasure` thoughts, by cause ("a tool broke", "clothes wore through", "unknown").
    pub lost: BTreeMap<&'static str, u64>,
}

impl Census {
    /// Sample if `w` is on a `SAMPLE_DAYS` boundary.
    pub fn sample(&mut self, w: &World) {
        if !w.meta.tick.is_multiple_of(SAMPLE_DAYS * TICKS_PER_DAY) {
            return;
        }
        let mut s = Sample {
            tick: w.meta.tick,
            ..Sample::default()
        };
        for it in &w.items {
            let Some(story) = it.story.as_ref() else {
                continue;
            };
            match it.place {
                ItemPlace::Held(_) => s.held += 1,
                ItemPlace::Carried(_) => s.carried += 1,
                ItemPlace::Ground(p) => {
                    if w.structures
                        .iter()
                        .any(|st| st.kind == StructureKind::Stockpile && st.pos == p)
                    {
                        s.stored += 1;
                    } else {
                        s.ground += 1;
                    }
                }
            }
            if story.cherished {
                s.cherished += 1;
            }
            if story.heirloom {
                s.heirloom += 1;
                if let Some((heir, _, _)) = story.owners.last() {
                    let holds = matches!(
                        it.place,
                        ItemPlace::Held(h) | ItemPlace::Carried(h) if h == *heir
                    );
                    if holds && w.agents.iter().any(|a| a.id == *heir && a.alive) {
                        s.heirloom_with_heir += 1;
                    }
                }
            }
        }
        self.samples.push(s);
    }

    /// Tally `LostTreasure` thoughts added this step, by their cause.
    pub fn note(&mut self, w: &World, events: &[Event]) {
        let tick = w.meta.tick;
        for a in &w.agents {
            if !a
                .thoughts
                .list
                .iter()
                .any(|t| t.kind == ThoughtKind::LostTreasure && t.since == tick)
            {
                continue;
            }
            let cause = events
                .iter()
                .find_map(|e| match &e.kind {
                    EventKind::ToolBroke { agent, .. } if *agent == a.id => Some("a tool broke"),
                    EventKind::NewClothes { agent, .. } if *agent == a.id => {
                        Some("clothes wore through")
                    }
                    _ => None,
                })
                .unwrap_or("unknown");
            *self.lost.entry(cause).or_default() += 1;
        }
    }

    pub fn line(&self, seed: u64, days: u64) -> String {
        let last = self.samples.last().cloned().unwrap_or_default();
        let pct = |n: u32, of: u32| {
            if of == 0 {
                0.0
            } else {
                100.0 * n as f64 / of as f64
            }
        };
        let lost: String = self
            .lost
            .iter()
            .map(|(cause, n)| format!("{cause} {n}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "POSSESSIONS seed {seed} days {days}: at day {} storied items: {} held, {} carried, \
             {} on the ground, {} in store; cherished {}; heirlooms {} ({} with a living heir \
             who holds it, {:.1}%); lost treasures: {}",
            last.tick / TICKS_PER_DAY,
            last.held,
            last.carried,
            last.ground,
            last.stored,
            last.cherished,
            last.heirloom,
            last.heirloom_with_heir,
            pct(last.heirloom_with_heir, last.heirloom),
            if lost.is_empty() { "none" } else { &lost },
        )
    }
}

/// Run an opening world of `seed` for `days` game-days and tally it.
pub fn run(seed: u64, days: u64) -> Census {
    let mut w = World::opening_world(seed);
    w.meta.autoplan = true;
    let mut c = Census::default();
    for _ in 0..days * TICKS_PER_DAY {
        w.step();
        let events = w.last_events.clone();
        c.note(&w, &events);
        c.sample(&w);
    }
    c
}

/// `[days [from to]]`, as the other censuses take it.
pub fn parse_args(args: &[String]) -> (u64, u64, u64) {
    let numbers: Vec<u64> = args.iter().skip(1).filter_map(|a| a.parse().ok()).collect();
    let (from, to) = match numbers.as_slice() {
        [_, a, b] => (*a, *b),
        _ => (5, 8),
    };
    (numbers.first().copied().unwrap_or(600), from, to)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (days, from, to) = parse_args(&args);
    for seed in from..=to {
        println!("{}", run(seed, days).line(seed, days));
    }
}
