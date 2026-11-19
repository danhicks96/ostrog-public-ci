//! Who is awake in the night, doing what, and why (#587). An opening is stepped as
//! `ostrog soak --opening` steps it. At each hour from 22:00 to 05:00 every person of a
//! community is counted asleep or awake; the awake are tallied by activity, by whether their
//! people are nocturnal, and by how rested they are (`needs.rest`, 1 is fully rested).
//!
//! Run: cargo run --release -p hf-sim --example night_awake -- <seed> <days> [verbose]
use std::collections::BTreeMap;

use hf_sim::agents::Species;
use hf_sim::mind::Activity;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::World;

fn what(a: &Activity) -> String {
    match a {
        Activity::Going { then, .. } => format!("going, then {}", what(then)),
        Activity::Working { plan } => {
            let s = format!("{plan:?}");
            let head: String = s.chars().take_while(|c| c.is_alphanumeric()).collect();
            format!("working {head}")
        }
        o => o.name().to_string(),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(3);
    let days: u64 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(60);
    let verbose = args.get(3).is_some();
    let mut w = World::from_setup(&hf_sim::setup::WorldSetup::new(seed));
    let start = w.meta.tick;
    let hours = [22u64, 23, 0, 1, 2, 3, 4, 5];
    let mut per_hour: BTreeMap<u64, (u32, u32)> = BTreeMap::new();
    let mut at2: BTreeMap<String, u32> = BTreeMap::new();
    let mut kinds: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    // When each person last lay down and woke, and the tallies of early waking.
    let mut lay: BTreeMap<hf_sim::ids::AgentId, u64> = BTreeMap::new();
    let mut asleep_before: BTreeMap<hf_sim::ids::AgentId, bool> = BTreeMap::new();
    let mut woke: BTreeMap<String, u32> = BTreeMap::new();
    let mut early_doing: BTreeMap<String, u32> = BTreeMap::new();
    // Hours slept a day by people kind (sum of Slept rows, counted per person-day).
    let mut slept_h: BTreeMap<String, (f32, u32)> = BTreeMap::new();
    while w.meta.tick < start + days * TICKS_PER_DAY {
        w.step();
        let tick = w.meta.tick;
        for e in &w.last_events {
            if let hf_sim::events::EventKind::Slept { agent, hours, .. } = &e.kind {
                if let Some(a) = w.agents.iter().find(|a| a.id == *agent) {
                    if a.species == Species::Person && a.community.is_some() {
                        let k = slept_h.entry(a.species_def().name.clone()).or_default();
                        k.0 += *hours;
                    }
                }
            }
        }
        for a in w
            .agents
            .iter()
            .filter(|a| a.alive && a.species == Species::Person && a.community.is_some())
        {
            let asleep = a.activity.is_asleep();
            let before = asleep_before.insert(a.id, asleep).unwrap_or(false);
            if asleep && !before {
                lay.insert(a.id, tick);
            }
            let h = (tick % TICKS_PER_DAY) / TICKS_PER_HOUR;
            if !asleep && before && !a.species_def().nocturnal() && (1..6).contains(&h) {
                let lh = lay.get(&a.id).map(|t| (t % TICKS_PER_DAY) / TICKS_PER_HOUR);
                *woke
                    .entry(format!(
                        "woke {h:02}h, lay down {}",
                        lh.map(|x| format!("{x:02}h")).unwrap_or("?".into())
                    ))
                    .or_default() += 1;
                let n = &a.needs;
                let why = if n.rest < 0.99 {
                    "not rested"
                } else if n.hunger < 0.3 {
                    "rested, hungry"
                } else if n.thirst < 0.3 {
                    "rested, thirsty"
                } else {
                    "rested, fed"
                };
                *woke
                    .entry(format!("  why: {why}, now {}", what(&a.activity)))
                    .or_default() += 1;
            }
        }
        if tick.is_multiple_of(TICKS_PER_DAY) {
            let mut per: BTreeMap<String, u32> = BTreeMap::new();
            for a in w
                .agents
                .iter()
                .filter(|a| a.alive && a.species == Species::Person && a.community.is_some())
            {
                *per.entry(a.species_def().name.clone()).or_default() += 1;
            }
            for (k, n) in per {
                slept_h.entry(k).or_default().1 += n;
            }
        }
        if !tick.is_multiple_of(TICKS_PER_HOUR) {
            continue;
        }
        let h = (tick % TICKS_PER_DAY) / TICKS_PER_HOUR;
        if !hours.contains(&h) {
            continue;
        }
        for a in w
            .agents
            .iter()
            .filter(|a| a.alive && a.species == Species::Person && a.community.is_some())
        {
            let awake = !a.activity.is_asleep();
            if awake && (h == 3 || h == 4) && !a.species_def().nocturnal() {
                *early_doing
                    .entry(format!("{h:02}h {}", what(&a.activity)))
                    .or_default() += 1;
            }
            let e = per_hour.entry(h).or_default();
            e.0 += 1;
            e.1 += awake as u32;
            if h == 2 {
                let def = a.species_def();
                let k = kinds
                    .entry(format!(
                        "{}{}",
                        def.name,
                        if def.nocturnal() { " (nocturnal)" } else { "" }
                    ))
                    .or_default();
                k.0 += 1;
                k.1 += awake as u32;
                if awake {
                    let key = format!(
                        "{}{} rest<{:.1}",
                        what(&a.activity),
                        if def.nocturnal() { " [nocturnal]" } else { "" },
                        (a.needs.rest * 10.0).ceil() / 10.0
                    );
                    *at2.entry(key).or_default() += 1;
                    if verbose {
                        println!(
                            "d{} {:?} {} {} rest {:.2} hunger {:.2} ill {:.2} child {}",
                            (tick - start) / TICKS_PER_DAY,
                            a.id,
                            def.name,
                            what(&a.activity),
                            a.needs.rest,
                            a.needs.hunger,
                            a.body.ill(),
                            a.is_child(tick)
                        );
                    }
                }
            }
        }
    }
    println!("SEED {seed} days {days}: awake share by hour (person-hours awake / counted)");
    for h in hours {
        let (n, a) = per_hour.get(&h).copied().unwrap_or_default();
        println!(
            "  {h:02}:00  {a:5} / {n:5}  {:.0}%",
            100.0 * a as f32 / n.max(1) as f32
        );
    }
    println!("  at 02:00 by people:");
    for (k, (n, a)) in &kinds {
        println!("    {k:30} {a:5} awake of {n:5}");
    }
    println!("  hours slept a person-day:");
    for (k, (h, n)) in &slept_h {
        println!("    {k:30} {:.1}", h / (*n).max(1) as f32);
    }
    let mut ws: Vec<_> = woke.into_iter().collect();
    ws.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    println!("  day-folk waking between 01:00 and 06:00:");
    for (k, n) in ws.iter().take(12) {
        println!("    {k:40} {n:5}");
    }
    let mut ed: Vec<_> = early_doing.into_iter().collect();
    ed.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    println!("  day-folk awake at 03:00 and 04:00 by activity:");
    for (k, n) in ed.iter().take(12) {
        println!("    {k:40} {n:5}");
    }
    let mut v: Vec<_> = at2.into_iter().collect();
    v.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    println!("  awake at 02:00 by activity:");
    for (k, n) in v.iter().take(15) {
        println!("    {k:50} {n:5}");
    }
}
