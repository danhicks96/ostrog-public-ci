//! How threat is paced: one CSV row per 100 game-days of a world made from
//! `WorldSetup::new(seed)` with the chosen engine, so a change to the storyteller can be
//! measured before and after. It reads only `w.last_events` and the world; nothing in the sim
//! changes.
//!
//! Columns: bands (`BandSetOut`) and hosts (`Besieged`) counted apart, then their min, mean and
//! max size together; portions
//! raided (the sum of `Raided.count`); endings (`Routed`, `HostSated`, `SiegeLifted` split by
//! its `why`: "the wall held", "tired of waiting", anything else as `other_lifted`); the mean,
//! over the block's days, of grown (not child) living people in communities not dissolved, with
//! the world's food items, other items and finished structures at the block's last day; deaths
//! of people not of old age; and the largest gap in days between an ending (any of the endings
//! above) and the next band or host, counting only gaps that closed in the block.
//!
//! Run: cargo run --release -p hf-sim --example threat_census -- <seed> <days> <steady|relentless|whimsical>
use hf_sim::agents::Species;
use hf_sim::engine::Engine;
use hf_sim::events::Event;
use hf_sim::events::EventKind as E;
use hf_sim::setup::WorldSetup;
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::World;

const BLOCK_DAYS: u64 = 100;

/// One block's tallies.
#[derive(Default, Clone, Debug, PartialEq)]
pub struct Block {
    pub sizes: Vec<u32>,
    pub bands: u32,
    pub hosts: u32,
    pub raided: u32,
    pub routed: u32,
    pub sated: u32,
    pub wall_held: u32,
    pub tired: u32,
    pub other_lifted: u32,
    pub deaths: u32,
    pub max_gap_days: f64,
    grown_sum: f64,
    grown_days: u32,
}

#[derive(Default)]
pub struct Census {
    pub block: Block,
    last_end: Option<u64>,
}

impl Census {
    /// Reads one tick's events. `person_death` says whether a `Died` event was a person's.
    pub fn feed(&mut self, events: &[Event], person_death: &dyn Fn(&Event) -> bool) {
        for e in events {
            match &e.kind {
                E::BandSetOut { size, .. } | E::Besieged { size, .. } => {
                    self.block.sizes.push(*size);
                    if matches!(e.kind, E::BandSetOut { .. }) {
                        self.block.bands += 1;
                    } else {
                        self.block.hosts += 1;
                    }
                    if let Some(t) = self.last_end.take() {
                        let gap = (e.tick - t) as f64 / TICKS_PER_DAY as f64;
                        if gap > self.block.max_gap_days {
                            self.block.max_gap_days = gap;
                        }
                    }
                }
                E::Raided { count, .. } => self.block.raided += count,
                E::Routed { .. } => self.end(e.tick, |b| b.routed += 1),
                E::HostSated { .. } => self.end(e.tick, |b| b.sated += 1),
                E::SiegeLifted { why, .. } => {
                    let w = why.clone();
                    self.end(e.tick, |b| match w.as_str() {
                        "the wall held" => b.wall_held += 1,
                        "tired of waiting" => b.tired += 1,
                        _ => b.other_lifted += 1,
                    })
                }
                E::Died { cause, .. } if cause != "old age" && person_death(e) => {
                    self.block.deaths += 1
                }
                _ => {}
            }
        }
    }

    fn end(&mut self, tick: u64, f: impl FnOnce(&mut Block)) {
        f(&mut self.block);
        self.last_end = Some(tick);
    }

    /// One day's count of grown people in communities.
    pub fn sample_grown(&mut self, grown: usize) {
        self.block.grown_sum += grown as f64;
        self.block.grown_days += 1;
    }

    /// The block's row, then a fresh block (an open gap carries over).
    pub fn row(&mut self, label: &str, food: u32, other: u32, structures: usize) -> String {
        let b = std::mem::take(&mut self.block);
        let n = b.sizes.len();
        let (min, max) = (
            b.sizes.iter().min().copied().unwrap_or(0),
            b.sizes.iter().max().copied().unwrap_or(0),
        );
        let mean = if n == 0 {
            0.0
        } else {
            b.sizes.iter().sum::<u32>() as f64 / n as f64
        };
        let grown = if b.grown_days == 0 {
            0.0
        } else {
            b.grown_sum / b.grown_days as f64
        };
        format!(
            "{label},{},{},{min},{mean:.1},{max},{},{},{},{},{},{},{grown:.1},{food},{other},{structures},{},{:.1}",
            b.bands,
            b.hosts,
            b.raided,
            b.routed,
            b.sated,
            b.wall_held,
            b.tired,
            b.other_lifted,
            b.deaths,
            b.max_gap_days
        )
    }
}

pub const HEADER: &str = "days,bands,hosts,size_min,size_mean,size_max,portions_raided,routed,sated,wall_held,tired,other_lifted,grown_in_community,food_items,other_items,structures,deaths_not_old_age,max_gap_days";

fn grown_in_communities(w: &World) -> usize {
    w.agents
        .iter()
        .filter(|a| {
            a.alive
                && a.species == Species::Person
                && !a.is_child(w.tick())
                && a.community
                    .is_some_and(|c| w.communities.iter().any(|k| k.id == c && !k.dissolved))
        })
        .count()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let usage = "usage: threat_census <seed> <days> <steady|relentless|whimsical>";
    let seed: u64 = args.get(1).and_then(|a| a.parse().ok()).expect(usage);
    let days: u64 = args.get(2).and_then(|a| a.parse().ok()).expect(usage);
    let engine: Engine = args.get(3).expect(usage).parse().expect(usage);
    let mut setup = WorldSetup::new(seed);
    setup.engine = engine;
    let mut w = World::from_setup(&setup);
    let mut c = Census::default();
    println!("{HEADER}");
    for day in 1..=days {
        for _ in 0..TICKS_PER_DAY {
            w.step();
            let events = std::mem::take(&mut w.last_events);
            c.feed(&events, &|e| match &e.kind {
                E::Died { agent, .. } => w
                    .agents
                    .iter()
                    .find(|a| a.id == *agent)
                    .is_some_and(|a| a.species == Species::Person),
                _ => false,
            });
            w.last_events = events;
        }
        c.sample_grown(grown_in_communities(&w));
        if day % BLOCK_DAYS == 0 || day == days {
            let (mut food, mut other) = (0u32, 0u32);
            for i in &w.items {
                if i.kind.is_food() {
                    food += i.count;
                } else {
                    other += i.count;
                }
            }
            let done = w.structures.iter().filter(|s| s.progress >= 1.0).count();
            let from = (day - 1) / BLOCK_DAYS * BLOCK_DAYS;
            println!("{}", c.row(&format!("{from}-{day}"), food, other, done));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hf_sim::events::Severity;
    use hf_sim::ids::{AgentId, EventId};

    fn ev(tick: u64, kind: E) -> Event {
        Event {
            id: EventId(0),
            tick,
            at: None,
            severity: Severity::Minor,
            kind,
        }
    }

    /// The census counts bands and hosts with their sizes, splits endings by kind, leaves out
    /// old-age deaths, and measures the gap from an ending to the next arrival.
    #[test]
    fn counts_sizes_endings_deaths_and_gap() {
        let d = TICKS_PER_DAY;
        let who = AgentId(1);
        let evs = vec![
            ev(
                0,
                E::BandSetOut {
                    band: 1,
                    site: "A".into(),
                    size: 4,
                },
            ),
            ev(
                d,
                E::Raided {
                    band: 1,
                    what: "grain".into(),
                    count: 3,
                    open: false,
                },
            ),
            ev(
                2 * d,
                E::Routed {
                    site: "A".into(),
                    lost: 1,
                },
            ),
            ev(
                12 * d,
                E::Besieged {
                    site: "B".into(),
                    size: 10,
                },
            ),
            ev(
                14 * d,
                E::SiegeLifted {
                    site: "B".into(),
                    days: 2,
                    taken: 0,
                    captives: 0,
                    why: "the wall held".into(),
                },
            ),
            ev(
                15 * d,
                E::Died {
                    agent: who,
                    cause: "old age".into(),
                },
            ),
            ev(
                15 * d,
                E::Died {
                    agent: who,
                    cause: "cold".into(),
                },
            ),
        ];
        let mut c = Census::default();
        c.feed(&evs, &|_| true);
        c.sample_grown(6);
        c.sample_grown(8);
        let b = c.block.clone();
        assert_eq!(b.sizes, vec![4, 10]);
        assert_eq!((b.raided, b.routed, b.wall_held, b.deaths), (3, 1, 1, 1));
        assert!((b.max_gap_days - 10.0).abs() < 1e-9);
        assert_eq!(
            c.row("0-100", 5, 7, 2),
            "0-100,1,1,4,7.0,10,3,1,0,1,0,0,7.0,5,7,2,1,10.0"
        );
    }

    /// A gap that opens in one block closes in the next, and is counted there; when several
    /// endings come before the next arrival the gap runs from the last of them. A death the
    /// caller says was not a person's (an animal, or an agent no longer in the world) is left out.
    #[test]
    fn a_gap_crossing_a_block_counts_where_it_closes() {
        let d = TICKS_PER_DAY;
        let evs1 = vec![
            ev(
                90 * d,
                E::Routed {
                    site: "A".into(),
                    lost: 0,
                },
            ),
            ev(
                95 * d,
                E::HostSated {
                    site: "A".into(),
                    days: 5,
                    taken: 0,
                    captives: 0,
                },
            ),
        ];
        let mut c = Census::default();
        c.feed(&evs1, &|_| true);
        assert_eq!(c.block.max_gap_days, 0.0, "an open gap is not counted yet");
        let first = c.row("0-100", 0, 0, 0);
        assert!(first.starts_with("0-100,0,0,0,0.0,0,0,"), "{first}");
        let evs2 = vec![
            ev(
                110 * d,
                E::Besieged {
                    site: "B".into(),
                    size: 6,
                },
            ),
            ev(
                111 * d,
                E::Died {
                    agent: AgentId(7),
                    cause: "wolves".into(),
                },
            ),
        ];
        c.feed(&evs2, &|_| false);
        assert!(
            (c.block.max_gap_days - 15.0).abs() < 1e-9,
            "from the last ending"
        );
        assert_eq!((c.block.bands, c.block.hosts, c.block.deaths), (0, 1, 0));
    }
}
