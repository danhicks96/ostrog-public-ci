//! Every death of a person and what became of the body (#566): laid to rest (with the body or
//! in its absence, by whom, how long after), or left unrested, and why. An opening is stepped
//! as `ostrog soak --opening` steps it.
//!
//! Run: cargo run --release -p hf-sim --example rite_census -- <seed> <days>
use std::collections::BTreeMap;

use hf_sim::agents::Species;
use hf_sim::events::EventKind as E;
use hf_sim::ids::AgentId;
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::World;

#[derive(Default)]
struct Watch {
    doer_seen: bool,
    reachable_seen: bool,
    living_last: usize,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(1);
    let days: u64 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(120);
    let mut w = World::from_setup(&hf_sim::setup::WorldSetup::new(seed));
    let mut died: BTreeMap<AgentId, (u64, String)> = BTreeMap::new();
    let mut watch: BTreeMap<AgentId, Watch> = BTreeMap::new();
    let mut tally: BTreeMap<String, u32> = BTreeMap::new();
    for _ in 0..days * TICKS_PER_DAY {
        w.step();
        let tick = w.meta.tick;
        for r in &w.rites_due {
            let Some(body) = w.agents.iter().find(|a| a.id == r.dead).map(|a| a.pos) else {
                continue;
            };
            let living: Vec<_> = w
                .agents
                .iter()
                .filter(|a| a.alive && a.community == Some(r.community))
                .collect();
            let wt = watch.entry(r.dead).or_default();
            wt.doer_seen |= r.doer.is_some();
            wt.living_last = living.len();
            if tick.is_multiple_of(200) && !wt.reachable_seen {
                wt.reachable_seen = living.iter().any(|a| {
                    a.pos.z == body.z && w.nav.as_ref().is_some_and(|n| n.reach(a.pos, body))
                });
            }
        }
        let evs = w.last_events.clone();
        let laid_now: Vec<AgentId> = evs
            .iter()
            .filter_map(|e| match &e.kind {
                E::LaidToRest { agent, .. } => Some(*agent),
                _ => None,
            })
            .collect();
        for e in evs {
            match &e.kind {
                E::Died { agent, cause } => {
                    let Some(a) = w.agents.iter().find(|a| a.id == *agent) else {
                        continue;
                    };
                    if a.species != Species::Person {
                        continue;
                    }
                    died.insert(*agent, (e.tick, cause.clone()));
                    if a.community.is_none() && !w.rites_due.iter().any(|r| r.dead == *agent) {
                        println!(
                            "d{} {:?} {} -> no community at death, unrested",
                            e.tick / TICKS_PER_DAY,
                            agent,
                            cause
                        );
                        *tally.entry("unrested: no community".into()).or_default() += 1;
                    }
                }
                E::LaidToRest {
                    agent,
                    by,
                    in_absence,
                    ..
                } => {
                    let (t, cause) = died.get(agent).cloned().unwrap_or_default();
                    println!(
                        "d{} {:?} {} -> laid to rest by {:?} in_absence={} after {}h",
                        t / TICKS_PER_DAY,
                        agent,
                        cause,
                        by,
                        in_absence,
                        (e.tick - t) / 1200
                    );
                    let k = if cause.contains("lost") {
                        format!("{cause}: laid in_absence={in_absence}")
                    } else {
                        format!("laid in_absence={in_absence}")
                    };
                    *tally.entry(k).or_default() += 1;
                }
                _ => {}
            }
        }
        // Rites that fell out of rites_due with no LaidToRest: unrested.
        let due: Vec<AgentId> = w.rites_due.iter().map(|r| r.dead).collect();
        let gone: Vec<AgentId> = watch
            .keys()
            .copied()
            .filter(|id| !due.contains(id))
            .collect();
        for id in gone {
            let wt = watch.remove(&id).unwrap();
            let unrested = w.agents.iter().any(|a| {
                a.id == id
                    && a.conditions
                        .iter()
                        .any(|c| c.kind == hf_sim::conditions::ConditionKind::Unrested)
            });
            if laid_now.contains(&id) {
                continue;
            }
            let (t, cause) = died.get(&id).cloned().unwrap_or_default();
            let why = if wt.living_last <= 1 {
                "nobody left"
            } else if !wt.reachable_seen {
                "nobody could reach"
            } else if !wt.doer_seen {
                "reachable, nobody took it on"
            } else {
                "a doer set, never arrived"
            };
            println!(
                "d{} {:?} {} -> unrested={} ({})",
                t / TICKS_PER_DAY,
                id,
                cause,
                unrested,
                why
            );
            *tally.entry(format!("unrested: {why}")).or_default() += 1;
        }
    }
    println!("TALLY seed {seed}:");
    for (k, n) in tally {
        println!("  {n:4}  {k}");
    }
}
