//! The census behind issue #404: openings of seeds FROM to TO run DAYS game-days, with the
//! justice rows counted (accusations, verdicts, feuds, exiles, societies) and the breakdowns read
//! per person: how many each broke, their calm against everyone's, and which thoughts weighed on
//! them when they broke.
//!
//! Run: cargo run --release -p hf-sim --example justice_census [days [from to]]

use hf_sim::events::EventKind;
use std::collections::BTreeMap;

/// Per person: breaks, calm, and the bad thoughts' weight at each break by kind.
type Broke = (u32, f32, BTreeMap<String, f32>);

fn main() {
    let args: Vec<u64> = std::env::args()
        .skip(1)
        .filter_map(|a| a.parse().ok())
        .collect();
    let days = args.first().copied().unwrap_or(100);
    let (from, to) = match args.as_slice() {
        [_, a, b] => (*a, *b),
        _ => (1, 4),
    };
    for seed in from..=to {
        let mut w = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(seed));
        let mut rows: BTreeMap<String, u32> = Default::default();
        let mut verdicts: BTreeMap<String, u32> = Default::default();
        let mut broke: BTreeMap<u32, Broke> = Default::default();
        let mut calms: BTreeMap<u32, f32> = Default::default();
        for _ in 0..days * hf_sim::time::TICKS_PER_DAY {
            w.step();
            for e in &w.last_events {
                let k = e.kind_name();
                if matches!(
                    k,
                    "accused"
                        | "judged"
                        | "feud"
                        | "feud_inherited"
                        | "feud_ended"
                        | "exiled"
                        | "society_formed"
                        | "betrayed"
                        | "broke"
                        | "quarrelled"
                        | "fought"
                        | "killing"
                        | "maiming"
                ) {
                    *rows.entry(k.to_string()).or_default() += 1;
                }
                match &e.kind {
                    EventKind::Judged { verdict, what, .. } => {
                        *verdicts.entry(format!("{verdict} for {what}")).or_default() += 1;
                    }
                    EventKind::Quarrelled { a, b } => {
                        let op = |x: hf_sim::ids::AgentId, y: hf_sim::ids::AgentId| {
                            w.agents
                                .iter()
                                .find(|o| o.id == x)
                                .map_or(0.0, |o| o.opinion_of(y))
                        };
                        let worst = op(*a, *b).max(op(*b, *a));
                        let band = if worst < -0.3 {
                            "both under -0.3"
                        } else if worst < 0.0 {
                            "both under 0"
                        } else {
                            "one or both fond"
                        };
                        *rows.entry(format!("quarrel, {band}")).or_default() += 1;
                    }
                    EventKind::Broke { agent, kind } => {
                        *rows.entry(format!("broke as {kind}")).or_default() += 1;
                        let Some(a) = w.agents.iter().find(|a| a.id == *agent) else {
                            continue;
                        };
                        let b = broke.entry(agent.0).or_insert((
                            0,
                            a.genome.calm(),
                            Default::default(),
                        ));
                        b.0 += 1;
                        for t in a.thoughts.list.iter().filter(|t| t.weight < 0.0) {
                            *b.2.entry(format!("{:?}", t.kind)).or_default() += -t.weight;
                        }
                    }
                    _ => {}
                }
            }
            if w.meta.tick.is_multiple_of(hf_sim::time::TICKS_PER_DAY) {
                for a in w.agents.iter().filter(|a| {
                    a.alive && a.species == hf_sim::agents::Species::Person && a.community.is_some()
                }) {
                    calms.insert(a.id.0, a.genome.calm());
                }
            }
        }
        println!("seed {seed}, {days} days: {rows:?}");
        for (v, n) in &verdicts {
            println!("  judged {n:4} {v}");
        }
        let total: u32 = broke.values().map(|b| b.0).sum();
        let mut by: Vec<(&u32, &Broke)> = broke.iter().collect();
        by.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then(a.0.cmp(b.0)));
        let top: u32 = by.iter().take(6).map(|b| b.1 .0).sum();
        let mut all: Vec<f32> = calms.values().copied().collect();
        all.sort_by(f32::total_cmp);
        let rank = |c: f32| all.iter().filter(|x| **x < c).count();
        let quarter = all.get(all.len() / 4).copied().unwrap_or(0.0);
        let least_calm: u32 = broke.values().filter(|b| b.1 < quarter).map(|b| b.0).sum();
        println!(
            "  breaks {total} by {} people; the six who broke most {top}; the least calm quarter {least_calm}; {} people ever seen",
            broke.len(),
            all.len()
        );
        for (id, (n, calm, th)) in by.iter().take(6) {
            let mut th: Vec<(&String, &f32)> = th.iter().collect();
            th.sort_by(|a, b| b.1.total_cmp(a.1));
            let th: Vec<String> = th
                .iter()
                .take(5)
                .map(|(k, v)| format!("{k} {:.1}", *v / *n as f32))
                .collect();
            println!(
                "    agent {id}: {n} breaks, calm {calm:.2} (rank {} of {} from least calm); per break: {}",
                rank(*calm),
                all.len(),
                th.join(", ")
            );
        }
    }
}
