//! Who sleeps rough with a bed standing, and why (#469). An opening is stepped as
//! `ostrog soak --opening` steps it. Each night slept (a `Slept` row of six hours or more,
//! the row that gives `SleptRough`) by a person of a community is counted in bed or rough; for
//! each rough night, the moment they lay down on the ground is classed by the beds standing
//! then:
//!   no free bed      every finished Bed/Tent/Nest was claimed (asleep in it or walking to it)
//!   none reachable   free beds stood, none of them reachable from where they lay down
//!   unknown          a free reachable bed stood, not in their knowledge and not in sight
//!   known            a free reachable bed was in their knowledge (the choice should take it)
//! with the distance to the nearest free reachable bed. Also counted: the people of a
//! community awake at 02:00 who are not sleeping.
//!
//! Run: cargo run --release -p hf-sim --example sleep_rough -- <seed> <days> [verbose]
use std::collections::BTreeMap;

use hf_sim::agents::Species;
use hf_sim::events::EventKind as E;
use hf_sim::ids::{AgentId, StructureId, TilePos};
use hf_sim::mind::{Activity, Known};
use hf_sim::structures::StructureKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::World;

fn claim(a: &Activity) -> Option<StructureId> {
    match a {
        Activity::Sleeping { bed: Some(id), .. } => Some(*id),
        Activity::Going { then, .. } => match **then {
            Activity::Sleeping { bed: Some(id), .. } => Some(id),
            _ => None,
        },
        _ => None,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(1);
    let days: u64 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(60);
    let verbose = args.get(3).is_some();
    let mut w = World::from_setup(&hf_sim::setup::WorldSetup::new(seed));
    let start = w.meta.tick;
    let mut lay: BTreeMap<AgentId, (String, i32)> = BTreeMap::new();
    let mut was_ground: BTreeMap<AgentId, bool> = BTreeMap::new();
    // Every bed each person has ever had in knowledge (sampled hourly), to tell a bed
    // forgotten from one never learned.
    let mut ever: BTreeMap<AgentId, std::collections::BTreeSet<StructureId>> = BTreeMap::new();
    let mut awake_doing: BTreeMap<String, u32> = BTreeMap::new();
    let mut why_unknown: BTreeMap<&'static str, u32> = BTreeMap::new();
    let mut nights: BTreeMap<String, u32> = BTreeMap::new();
    let mut dists: BTreeMap<String, Vec<i32>> = BTreeMap::new();
    let (mut in_bed, mut rough, mut awake2, mut people2) = (0u32, 0u32, 0u32, 0u32);
    let (mut beds_sum, mut ppl_sum, mut samples) = (0usize, 0usize, 0u32);
    while w.meta.tick < start + days * TICKS_PER_DAY {
        w.step();
        let tick = w.meta.tick;
        if tick.is_multiple_of(TICKS_PER_HOUR) {
            for a in w
                .agents
                .iter()
                .filter(|a| a.alive && a.species == Species::Person)
            {
                let e = ever.entry(a.id).or_default();
                for (k, _) in a.knowledge.iter_kind(|k| matches!(k, Known::Structure(_))) {
                    if let Known::Structure(id) = k {
                        e.insert(*id);
                    }
                }
            }
        }
        // Who lay down on the ground this tick.
        let mut fresh = Vec::new();
        for (i, a) in w.agents.iter().enumerate() {
            if !a.alive || a.species != Species::Person || a.community.is_none() {
                continue;
            }
            let ground = matches!(a.activity, Activity::Sleeping { bed: None, .. });
            let before = was_ground.insert(a.id, ground).unwrap_or(false);
            if ground && !before {
                fresh.push(i);
            }
        }
        if !fresh.is_empty() {
            let claimed: Vec<StructureId> =
                w.agents.iter().filter_map(|o| claim(&o.activity)).collect();
            let free: Vec<(StructureId, TilePos)> = w
                .structures
                .iter()
                .filter(|s| {
                    s.progress >= 1.0
                        && matches!(
                            s.kind,
                            StructureKind::Bed | StructureKind::Tent | StructureKind::Nest
                        )
                        && !claimed.contains(&s.id)
                })
                .map(|s| (s.id, s.pos))
                .collect();
            let nav = w.nav.as_ref().unwrap();
            for i in fresh {
                let a = &w.agents[i];
                let me = a.pos;
                let reach: Vec<&(StructureId, TilePos)> =
                    free.iter().filter(|(_, p)| nav.reach(me, *p)).collect();
                let near = reach.iter().map(|(_, p)| p.dist(me)).min().unwrap_or(-1);
                let known = reach.iter().any(|(id, _)| {
                    a.knowledge
                        .iter_kind(|k| matches!(k, Known::Structure(_)))
                        .any(|(k, _)| *k == Known::Structure(*id))
                });
                if !free.is_empty() && !reach.is_empty() && !known {
                    let had = ever
                        .get(&a.id)
                        .is_some_and(|e| reach.iter().any(|(id, _)| e.contains(id)));
                    let home = a
                        .community
                        .and_then(|c| w.communities.iter().find(|x| x.id == c))
                        .map(|c| c.home);
                    let near_home =
                        home.is_some_and(|h| reach.iter().any(|(_, p)| p.dist(h) <= 40));
                    let n_known = a
                        .knowledge
                        .iter_kind(|k| matches!(k, Known::Structure(_)))
                        .count();
                    let k = match (had, near_home) {
                        (true, true) => "forgotten, home bed",
                        (true, false) => "forgotten, not home",
                        (false, true) => "never known, home bed",
                        (false, false) => "never known, not home",
                    };
                    *why_unknown.entry(k).or_default() += 1;
                    if verbose {
                        println!("   unknown detail: {k}; knows {n_known} structures");
                    }
                }
                let class = if free.is_empty() {
                    "no free bed"
                } else if reach.is_empty() {
                    "none reachable"
                } else if known {
                    "known"
                } else {
                    "unknown"
                };
                if verbose {
                    println!(
                        "d{} h{:02} {:?} at {:?} rest {:.2} {} nearest {} ({} free, {} reachable)",
                        (tick - start) / TICKS_PER_DAY,
                        (tick % TICKS_PER_DAY) / TICKS_PER_HOUR,
                        a.id,
                        (me.x, me.y, me.z),
                        a.needs.rest,
                        class,
                        near,
                        free.len(),
                        reach.len()
                    );
                }
                lay.insert(a.id, (class.to_string(), near));
            }
        }
        if tick % TICKS_PER_DAY == 2 * TICKS_PER_HOUR {
            let ppl: Vec<_> = w
                .agents
                .iter()
                .filter(|a| a.alive && a.species == Species::Person && a.community.is_some())
                .collect();
            people2 += ppl.len() as u32;
            for a in ppl
                .iter()
                .filter(|a| !matches!(a.activity, Activity::Sleeping { .. }))
            {
                awake2 += 1;
                let what = match &a.activity {
                    Activity::Going { then, .. } => format!("going, then {}", then.name()),
                    o => o.name().to_string(),
                };
                *awake_doing
                    .entry(format!("{what} (rest {:.1})", a.needs.rest))
                    .or_default() += 1;
            }
            beds_sum += w
                .structures
                .iter()
                .filter(|s| {
                    s.progress >= 1.0
                        && matches!(
                            s.kind,
                            StructureKind::Bed | StructureKind::Tent | StructureKind::Nest
                        )
                })
                .count();
            ppl_sum += ppl.len();
            samples += 1;
        }
        for e in &w.last_events {
            if let E::Slept {
                agent,
                hours,
                in_bed: b,
            } = &e.kind
            {
                if *hours < 6.0 {
                    continue;
                }
                let Some(a) = w.agents.iter().find(|a| a.id == *agent) else {
                    continue;
                };
                if a.species != Species::Person || a.community.is_none() {
                    continue;
                }
                if *b {
                    in_bed += 1;
                } else {
                    rough += 1;
                    let (class, near) = lay
                        .get(agent)
                        .cloned()
                        .unwrap_or(("(not seen lying down)".into(), -1));
                    *nights.entry(class.clone()).or_default() += 1;
                    if near >= 0 {
                        dists.entry(class).or_default().push(near);
                    }
                }
            }
        }
    }
    println!(
        "SEED {seed} days {days}: person-nights in bed {in_bed}, rough {rough}; beds standing {:.1} for {:.1} people (mean at 02:00); awake at 02:00 {awake2} of {people2}",
        beds_sum as f32 / samples.max(1) as f32,
        ppl_sum as f32 / samples.max(1) as f32
    );
    let mut doing: Vec<_> = awake_doing.iter().collect();
    doing.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
    for (k, n) in doing.iter().take(8) {
        println!("  awake at 02:00: {k:40} {n:5}");
    }
    for (k, n) in &why_unknown {
        println!("  lay-downs beside an unknown free bed: {k:24} {n:5}");
    }
    for (k, n) in &nights {
        let d = dists.get(k).cloned().unwrap_or_default();
        let mut s = d.clone();
        s.sort();
        let med = s.get(s.len() / 2).copied().unwrap_or(-1);
        println!("  rough, {k:16} {n:5}  nearest free reachable bed median {med} tiles");
    }
}
