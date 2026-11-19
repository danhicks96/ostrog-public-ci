//! The census behind #1113 (under #1112): how often someone is starving while others of their
//! community, close by, carry food. Read-only: it only looks at the world, it changes nothing.
//!
//! Each game-hour, for each living person of a community on the map whose hunger is under 0.15, it
//! counts the food units carried by the other living people of that community on the map within
//! 20 tiles, and whether any of those holders is the starving person's kin or their friend (their
//! opinion of the holder is 0.5 or more). People away (`w.overworld.away`: on the road, taken,
//! gone below) are left out, as starving people and as holders: their needs do not run, and the
//! census never walks `away`, so an item still marked as carried by one of them is not counted.
//! Most departures put what the person carried on the ground; `taken.rs` `down_again` (a child
//! sent below again) does not. It also tallies the events `Starving` and `Repaid`.
//! `GaveFood`, `Stole` and `SetAside` do not exist yet (#1112's slices add them), so the line
//! prints "n/a (no such event yet)" for them, never a 0 that could be read as a measurement. The
//! slice that adds each of those events must replace its "n/a" in `Census::line` with a real
//! count, matched by variant name in `Census::note` (so the compiler checks it), and add a test.
//!
//! The 30-day baseline in docs/history/FIXLOG.md is not the 600-day number: 30 opening days miss the
//! later famine. The audit ran seed 5 on Steady, seed 6 on Relentless, seed 7 on Whimsical and
//! seed 8 on Steady, and the baseline does the same: `--engine` picks the pacing (Engine) the
//! world is made with, the SHARING line names it, and without the flag the world is Steady, as
//! it always was. In 30 days seeds 5 to 8, on Steady, Relentless, Whimsical and Steady, have 9,
//! 8, 17 and 8 "is starving" lines; seed 7 on Whimsical has 17 in 30 days against 3,071 in 600
//! days in the audit (the earlier 30-day numbers ran seeds 6 and 7 on Steady, and were
//! replaced). (Lines are events, not the person-hours the census counts, so the two are never
//! set against each other.) The 600-day run is still owed, seed by seed with its engine, before
//! #1115's targets mean anything: a bare run of seeds 5 to 8 makes seeds 6 and 7 on Steady,
//! which the audit never ran.
//!
//! Logged in docs/history/FIXLOG.md under "#1113: nothing counted who starves while others nearby carry
//! food". The tests (`tests/sharing.rs`) include this file and call `starving_beside` and `Census`
//! directly, so the numbers printed here are the ones the tests check.
//!
//! Run: cargo run --release -p hf-sim --example sharing_census [days [from to]] [--engine steady|relentless|whimsical]
//! (default 600 days, seeds 5 to 8, Steady). The audit's worlds, one command each:
//! `600 5 5`, `600 6 6 --engine relentless`, `600 7 7 --engine whimsical`, `600 8 8`.
// Fields and functions the example's own `main` does not read are read by the tests.
#![allow(dead_code)]
use hf_sim::agents::Species;
use hf_sim::engine::Engine;
use hf_sim::events::{Event, EventKind};
use hf_sim::ids::AgentId;
use hf_sim::items::ItemPlace;
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::World;
use std::collections::BTreeMap;

/// A person counts as starving here when hunger (1 just fed, 0 empty) is under this. The sim's
/// own "is starving" (`Needs::starving`) is hunger at or under 0.02; the census looks earlier on
/// purpose, to see people who are close to empty while a meal is carried nearby (#1113's number).
pub const STARVING_BELOW: f32 = 0.15;
/// "Beside" means within this many tiles by `TilePos::dist`: Chebyshev distance in the plane plus
/// 4 for each level of height, so a holder 6 or more levels up or down is never beside (5 levels at the same x, y is 20, which is).
pub const BESIDE_TILES: i32 = 20;
/// A friend is someone the starving person's own opinion puts at this or above (their opinion of
/// the holder, not the holder's of them: the starving one is the one who would ask).
pub const FRIEND_OPINION: f32 = 0.5;

/// One starving person's surroundings at one sample.
#[derive(Clone, Debug, PartialEq)]
pub struct Beside {
    pub who: AgentId,
    /// Food units (`ItemKind::is_food`, summed over `count`) carried by others of the community
    /// within `BESIDE_TILES`. The person's own food and food on the ground are not counted.
    pub units: u32,
    /// Of those, the units carried by kin or friends.
    pub kin_or_friend_units: u32,
    /// How many other people carry any of it.
    pub holders: u32,
}

/// Every starving person at this moment, with the food carried by others of their community
/// within `BESIDE_TILES`, in agent order. Nothing is said of food that is kept for someone
/// (#1114 has not added a store yet; when it does, its units are added here).
pub fn starving_beside(w: &World) -> Vec<Beside> {
    // Only `w.agents` (people on the map) is walked, never `w.overworld.away`: an away person's
    // needs do not run, and an item still marked as carried by them (`down_again` in `taken.rs`
    // does not put a child's items on the ground) is not counted.
    let mut carried: BTreeMap<AgentId, u32> = BTreeMap::new();
    for it in &w.items {
        if let ItemPlace::Carried(by) = it.place {
            if it.kind.is_food() {
                *carried.entry(by).or_default() += it.count;
            }
        }
    }
    let people = |a: &&hf_sim::agents::Agent| {
        a.alive && a.species == Species::Person && a.community.is_some()
    };
    let mut out = Vec::new();
    for me in w
        .agents
        .iter()
        .filter(people)
        .filter(|a| a.needs.hunger < STARVING_BELOW)
    {
        let kin = hf_sim::feuds::kin(w, me.id);
        let mut b = Beside {
            who: me.id,
            units: 0,
            kin_or_friend_units: 0,
            holders: 0,
        };
        for other in w.agents.iter().filter(people) {
            if other.id == me.id || other.community != me.community {
                continue;
            }
            if me.pos.dist(other.pos) > BESIDE_TILES {
                continue;
            }
            let n = carried.get(&other.id).copied().unwrap_or(0);
            if n == 0 {
                continue;
            }
            b.units += n;
            b.holders += 1;
            if kin.contains(&other.id) || me.opinion_of(other.id) >= FRIEND_OPINION {
                b.kin_or_friend_units += n;
            }
        }
        out.push(b);
    }
    out
}

/// One seed's tallies.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Census {
    /// The pacing (Engine) of the world that was counted, read from the world itself
    /// (`w.meta.engine`) so the printed line cannot name one engine and count another.
    pub engine: Engine,
    /// Game-hours sampled.
    pub hours: u64,
    /// Starving people summed over the hours (person-hours).
    pub starving: u64,
    /// Of those, the ones with any food carried by another beside them.
    pub beside_food: u64,
    /// Of those, the ones with any of it carried by kin or a friend.
    pub beside_kin_food: u64,
    /// Food units beside the starving, summed over person-hours, and the part held by kin/friends.
    pub units: u64,
    pub kin_or_friend_units: u64,
    /// `Starving` events ("is starving" lines).
    pub starving_lines: u64,
    pub repaid: u64,
    // No `gave_food`, `stole` or `set_aside` here: no such event exists yet (see the header).
}

impl Census {
    /// Sample if `w` is on a game-hour boundary (the tick is a multiple of `TICKS_PER_HOUR`).
    pub fn sample(&mut self, w: &World) {
        if !w.meta.tick.is_multiple_of(TICKS_PER_HOUR) {
            return;
        }
        self.hours += 1;
        for b in starving_beside(w) {
            self.starving += 1;
            self.units += b.units as u64;
            self.kin_or_friend_units += b.kin_or_friend_units as u64;
            if b.units > 0 {
                self.beside_food += 1;
            }
            if b.kin_or_friend_units > 0 {
                self.beside_kin_food += 1;
            }
        }
    }

    /// Count the events of one step. `GaveFood`, `Stole` and `SetAside` are not events yet; the
    /// slice that adds one adds its arm here, by variant, with a field, a line entry and a test.
    pub fn note(&mut self, events: &[Event]) {
        for e in events {
            match &e.kind {
                EventKind::Starving { .. } => self.starving_lines += 1,
                EventKind::Repaid { .. } => self.repaid += 1,
                _ => {}
            }
        }
    }

    pub fn line(&self, seed: u64, days: u64) -> String {
        let engine = self.engine;
        let pct = |n: u64, of: u64| {
            if of == 0 {
                0.0
            } else {
                100.0 * n as f64 / of as f64
            }
        };
        format!(
            "SHARING seed {seed} engine {engine} days {days}: {} hours; starving (hunger < {STARVING_BELOW}) {} person-hours; \
             beside carried food {} ({:.1}%); beside kin/friend's food {} ({:.1}%); \
             units beside {} (kin/friend {}); is-starving lines {}; \
             repaid {}; gave_food n/a (no such event yet); stole n/a (no such event yet); \
             set_aside n/a (no such event yet)",
            self.hours,
            self.starving,
            self.beside_food,
            pct(self.beside_food, self.starving),
            self.beside_kin_food,
            pct(self.beside_kin_food, self.starving),
            self.units,
            self.kin_or_friend_units,
            self.starving_lines,
            self.repaid
        )
    }
}

/// The opening world of `seed`, paced by `engine`: `World::from_setup(WorldSetup::new(seed))`
/// with the engine set, as the opening soak and the audit's probe make it (FIXLOG #1113: the
/// audit ran seed 6 on Relentless and seed 7 on Whimsical, so a census that always made Steady
/// worlds measured worlds the audit never ran).
pub fn world(seed: u64, engine: Engine) -> World {
    let mut setup = hf_sim::setup::WorldSetup::new(seed);
    setup.engine = engine;
    World::from_setup(&setup)
}

/// Run an opening world of `seed` and `engine` for `days` game-days and tally it.
pub fn run(seed: u64, engine: Engine, days: u64) -> Census {
    let mut w = world(seed, engine);
    let mut c = Census {
        engine: w.meta.engine,
        ..Census::default()
    };
    for _ in 0..days * hf_sim::time::TICKS_PER_DAY {
        w.step();
        c.note(&w.last_events);
        c.sample(&w);
    }
    c
}

/// What the command line asked for.
pub struct Args {
    pub days: u64,
    pub from: u64,
    pub to: u64,
    pub engine: Engine,
}

/// `[days [from to]] [--engine NAME]`, the flag anywhere after the program name (as in
/// `deaths_probe`). The flag and its value are taken out first; of the rest, only the words that
/// are whole numbers count, as they always have. Without the flag the engine is Steady, so the
/// old command lines mean what they meant.
pub fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut engine = Engine::default();
    let mut numbers: Vec<u64> = Vec::new();
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        if a == "--engine" {
            let v = it
                .next()
                .ok_or("--engine needs a name: steady, relentless or whimsical")?;
            engine = v.parse()?;
        } else if let Ok(n) = a.parse() {
            numbers.push(n);
        }
    }
    let (from, to) = match numbers.as_slice() {
        [_, a, b] => (*a, *b),
        _ => (5, 8),
    };
    Ok(Args {
        days: numbers.first().copied().unwrap_or(600),
        from,
        to,
        engine,
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Args {
        days,
        from,
        to,
        engine,
    } = parse_args(&args).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(2)
    });
    for seed in from..=to {
        println!("{}", run(seed, engine, days).line(seed, days));
    }
}
