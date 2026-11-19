//! The census behind #1329: nobody could see at a glance that one trader and one town take all
//! the valley's trips, because no example reports caravans (`examples/away_probe.rs` follows
//! people off the map but does not tally trips). #1329's figures were counted by hand from
//! probe events.txt (seed 8 Mordric 36 of 63 trips and 62 of 63 to Nis; seed 7 Ulric 20 of 35
//! to Fioriokre). This example tallies the same things from `CaravanFormed` and
//! `CaravanReturned` instead, per community: trips by leader, trips by site and cargo kind,
//! returns by what came home (with the "nothing to show" count), the leader share and site
//! share #1329 defines (the largest count over the total trips), and trips to a site whose
//! grudge was 2 or more at departure.
//!
//! Run: cargo run --release -p hf-sim --example trade_census -- <seed> <days> <engine>
//! (engine is steady, relentless or whimsical). The audit's three runs: `trade_census 5 600
//! steady`, `trade_census 7 600 whimsical`, `trade_census 8 600 steady`.
#![allow(dead_code)]
use hf_sim::engine::Engine;
use hf_sim::events::EventKind;
use hf_sim::ids::{AgentId, CommunityId};
use hf_sim::World;
use std::collections::BTreeMap;

/// One community's tally over the run.
#[derive(Default, Clone, Debug, PartialEq)]
pub struct CommunityCensus {
    pub name: String,
    pub trips: u32,
    /// Trips by leader name.
    pub by_leader: BTreeMap<String, u32>,
    /// Trips by "<site>: <cargo>" (cargo is "" for a scout, who carries nothing out).
    pub by_site_cargo: BTreeMap<String, u32>,
    /// Trips that left while the site's grudge against the valley was 2 or more.
    pub grudge_trips: u32,
    /// Returns by what came home, keyed by the item name ("grain", "meal"); a return that
    /// brought nothing (`received` empty) is counted in `nothing`, not here.
    pub by_received: BTreeMap<String, u32>,
    pub returns: u32,
    pub nothing: u32,
}

impl CommunityCensus {
    /// The largest trip count by one leader, over all trips (#1329's leader share).
    pub fn leader_share(&self) -> f32 {
        share(&self.by_leader, self.trips)
    }

    /// The largest trip count to one site (any cargo), over all trips (#1329's site share).
    pub fn site_share(&self) -> f32 {
        let mut by_site: BTreeMap<String, u32> = BTreeMap::new();
        for (k, n) in &self.by_site_cargo {
            let site = k.split(':').next().unwrap_or(k);
            *by_site.entry(site.to_string()).or_default() += n;
        }
        share(&by_site, self.trips)
    }
}

fn share(by: &BTreeMap<String, u32>, total: u32) -> f32 {
    if total == 0 {
        return 0.0;
    }
    by.values().copied().max().unwrap_or(0) as f32 / total as f32
}

/// All communities' tallies, keyed by community name so the table prints in a stable order.
#[derive(Default, Clone, Debug, PartialEq)]
pub struct Census {
    pub by_community: BTreeMap<String, CommunityCensus>,
}

impl Census {
    fn community(&mut self, id: CommunityId, w: &World) -> &mut CommunityCensus {
        let name = w
            .communities
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| format!("community #{}", id.0));
        self.by_community
            .entry(name.clone())
            .or_insert_with(|| CommunityCensus {
                name,
                ..Default::default()
            })
    }

    /// Note one tick's events against the world as it stood right after `w.step()` (so a
    /// `CaravanFormed` event's caravan is still in `w.overworld.caravans`, with the site and
    /// cargo its label does not carry structured).
    pub fn note(&mut self, w: &World) {
        for e in &w.last_events {
            match &e.kind {
                EventKind::CaravanFormed {
                    caravan,
                    community,
                    leader,
                    ..
                } => {
                    let Some(c) = w.overworld.caravans.iter().find(|c| c.id == *caravan) else {
                        continue;
                    };
                    let (site_id, cargo) = match &c.purpose {
                        hf_sim::expedition::Purpose::Trade { site, carry } => {
                            (*site, carry.label().to_string())
                        }
                        hf_sim::expedition::Purpose::Scout { site } => (*site, String::new()),
                        hf_sim::expedition::Purpose::Fetch { site, carry, .. } => {
                            (*site, carry.label().to_string())
                        }
                    };
                    let site_name = w
                        .overworld
                        .site(site_id)
                        .map(|s| s.name.clone())
                        .unwrap_or_else(|| format!("site #{site_id}"));
                    let site_grudge = w.overworld.site(site_id).map(|s| s.grudge).unwrap_or(0);
                    let leader_name = agent_name(w, *leader);
                    let cc = self.community(*community, w);
                    cc.trips += 1;
                    *cc.by_leader.entry(leader_name).or_default() += 1;
                    let key = format!("{site_name}: {cargo}");
                    *cc.by_site_cargo.entry(key).or_default() += 1;
                    if site_grudge >= 2 {
                        cc.grudge_trips += 1;
                    }
                }
                EventKind::CaravanReturned {
                    community,
                    received,
                    ..
                } => {
                    let cc = self.community(*community, w);
                    cc.returns += 1;
                    if received.is_empty() {
                        cc.nothing += 1;
                    } else {
                        for part in received.split(", ") {
                            // Each part is "<n> <item>" (set_down, expedition.rs); drop the
                            // count, keep the item's name.
                            let item = part
                                .split_once(' ')
                                .map(|(_, rest)| rest)
                                .unwrap_or(part)
                                .to_string();
                            *cc.by_received.entry(item).or_default() += 1;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn agent_name(w: &World, a: AgentId) -> String {
    w.agents
        .iter()
        .chain(w.overworld.away.iter())
        .find(|x| x.id == a)
        .map(|x| x.name.clone())
        .unwrap_or_else(|| format!("#{}", a.0))
}

/// The opening world of `seed`, paced by `engine` (as `sharing_census::world` does; #1113
/// FIXLOG: a census that always makes Steady worlds measures worlds the audit never ran).
pub fn world(seed: u64, engine: Engine) -> World {
    let mut setup = hf_sim::setup::WorldSetup::new(seed);
    setup.engine = engine;
    World::from_setup(&setup)
}

/// Run an opening world of `seed` and `engine` for `days` game-days and tally its caravans.
pub fn run(seed: u64, engine: Engine, days: u64) -> Census {
    let mut w = world(seed, engine);
    let mut census = Census::default();
    for _ in 0..days * hf_sim::time::TICKS_PER_DAY {
        w.step();
        census.note(&w);
    }
    census
}

fn print_table(seed: u64, days: u64, engine: Engine, census: &Census) {
    println!("seed {seed}: {days} days, engine {engine:?}");
    for cc in census.by_community.values() {
        println!(
            "  {} — {} trips, {} returns ({} nothing to show)",
            cc.name, cc.trips, cc.returns, cc.nothing
        );
        println!(
            "    leader share {:.0}% (top leader of {}), site share {:.0}% (top site of {})",
            cc.leader_share() * 100.0,
            cc.by_leader.len(),
            cc.site_share() * 100.0,
            {
                let mut sites: std::collections::BTreeSet<&str> = Default::default();
                for k in cc.by_site_cargo.keys() {
                    sites.insert(k.split(':').next().unwrap_or(k));
                }
                sites.len()
            }
        );
        println!("    trips by leader: {:?}", cc.by_leader);
        println!("    trips by site and cargo: {:?}", cc.by_site_cargo);
        println!(
            "    trips to a site with grudge >= 2 at departure: {}",
            cc.grudge_trips
        );
        println!("    returns by what came home: {:?}", cc.by_received);
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let usage = "usage: trade_census <seed> <days> <engine (steady|relentless|whimsical)>";
    let seed: u64 = a.first().and_then(|s| s.parse().ok()).unwrap_or_else(|| {
        eprintln!("{usage}");
        std::process::exit(2);
    });
    let days: u64 = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(600);
    let engine: Engine = a
        .get(2)
        .map(|s| s.parse())
        .transpose()
        .unwrap_or_else(|e: String| {
            eprintln!("{e}");
            std::process::exit(2);
        })
        .unwrap_or_default();
    let census = run(seed, engine, days);
    print_table(seed, days, engine, &census);
}
