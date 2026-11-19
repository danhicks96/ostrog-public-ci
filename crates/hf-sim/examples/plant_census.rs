//! The plant census of #286: for seeds 1 to 4, an opening stepped some game-days (60 unless
//! given), counting by kind which plants occur (at the start and the end), which are eaten
//! (gathered from the plant and eaten there), which are used (cut, felled or harvested), and
//! which poison someone (a look-alike eaten in error).
//!
//! Run: cargo run --release -p hf-sim --example plant_census [days]
use std::collections::BTreeMap;

use hf_sim::events::EventKind as E;
use hf_sim::plants::PlantKind;

fn tally(w: &hf_sim::World) -> BTreeMap<PlantKind, u32> {
    let mut m = BTreeMap::new();
    for p in w.plants.iter().filter(|p| p.alive) {
        *m.entry(p.kind).or_default() += 1;
    }
    m
}

type Counts = BTreeMap<PlantKind, u32>;

/// One opening: its printed table, and what was eaten, used and poisoned and which kinds were
/// there.
fn census(seed: u64, days: u64) -> (String, [Counts; 4]) {
    use std::fmt::Write;
    let t = std::time::Instant::now();
    let mut w = hf_sim::World::opening_world(seed);
    let start = tally(&w);
    let mut eaten = Counts::new();
    let mut used = Counts::new();
    let mut poisoned = Counts::new();
    let mut poisoned_dead = 0u32;
    let mut at: BTreeMap<(i32, i32, i32), PlantKind> = BTreeMap::new();
    let label: BTreeMap<&str, PlantKind> = PlantKind::ALL.iter().map(|k| (k.label(), *k)).collect();
    for n in 0..days * hf_sim::time::TICKS_PER_DAY {
        // Where each plant stands, refreshed every game hour (a cut plant is gone after).
        if n % hf_sim::time::TICKS_PER_HOUR == 0 {
            at = w
                .plants
                .iter()
                .filter(|p| p.alive)
                .map(|p| ((p.pos.x, p.pos.y, p.pos.z), p.kind))
                .collect();
        }
        w.step();
        for e in &w.last_events {
            match &e.kind {
                E::Foraged { at: p, .. } => {
                    if let Some(k) = at.get(&(p.x, p.y, p.z)) {
                        *eaten.entry(*k).or_default() += 1;
                    }
                }
                E::Harvested { at: p, .. } => {
                    if let Some(k) = at.get(&(p.x, p.y, p.z)) {
                        *used.entry(*k).or_default() += 1;
                    }
                }
                E::Died { cause, .. } if cause == "poisoning" => {
                    poisoned_dead += 1;
                }
                E::FellIll { what, .. } => {
                    if let Some(k) = what
                        .strip_prefix("a poisoning from ")
                        .and_then(|l| label.get(l))
                    {
                        *poisoned.entry(*k).or_default() += 1;
                    }
                }
                _ => {}
            }
        }
    }
    let end = tally(&w);
    let people = w
        .agents
        .iter()
        .filter(|a| a.alive && a.species == hf_sim::agents::Species::Person)
        .count();
    let shunning = w.agents.iter().filter(|a| !a.shuns.is_empty()).count();
    let mut out = String::new();
    let _ = writeln!(
        out,
        "seed {seed}: {days} days in {:.0} s; {} kinds at the start, {} at the end; {people} people living, {shunning} who learned a look-alike, {poisoned_dead} dead of a poisoning",
        t.elapsed().as_secs_f32(),
        start.len(),
        end.len()
    );
    let _ = writeln!(
        out,
        "  {:<20} {:>6} {:>6} {:>6} {:>6} {:>6}",
        "kind", "start", "end", "eaten", "used", "poison"
    );
    let mut seen = Counts::new();
    for k in PlantKind::ALL {
        let get = |m: &Counts| m.get(&k).copied().unwrap_or(0);
        let (s, e, a, u, p) = (
            get(&start),
            get(&end),
            get(&eaten),
            get(&used),
            get(&poisoned),
        );
        if s + e + a + u + p > 0 {
            let _ = writeln!(
                out,
                "  {:<20} {s:>6} {e:>6} {a:>6} {u:>6} {p:>6}",
                k.label()
            );
        }
        if s + e > 0 {
            seen.insert(k, 1);
        }
    }
    (out, [seen, eaten, used, poisoned])
}

fn main() {
    let days: u64 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(60);
    // The four openings side by side, each on its own thread; printed in seed order.
    let runs: Vec<(String, [Counts; 4])> = std::thread::scope(|sc| {
        let hs: Vec<_> = (1..5u64)
            .map(|seed| sc.spawn(move || census(seed, days)))
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let mut all: [Counts; 4] = Default::default();
    for (out, counts) in &runs {
        print!("{out}");
        for (sum, c) in all.iter_mut().zip(counts) {
            for (k, n) in c {
                *sum.entry(*k).or_default() += n;
            }
        }
    }
    let names = |m: &Counts| -> Vec<String> {
        m.iter()
            .filter(|(_, n)| **n > 0)
            .map(|(k, n)| format!("{} {n}", k.label()))
            .collect()
    };
    println!("in how many of the four: {:?}", names(&all[0]));
    println!("eaten: {:?}", names(&all[1]));
    println!("used: {:?}", names(&all[2]));
    println!("poisoned someone: {:?}", names(&all[3]));
}
