//! The ecology census of #486: for seeds 1 to 4 (or those given), an opening stepped some
//! game-days (600 unless given), printing every 30 days the valley's browse (the growth left on
//! the low plants grazers eat, above what grazing leaves), its grazers, predators and crows,
//! and at the end the births and deaths of the wild kinds by cause. It shows whether the food
//! web holds over a long run: no guild dies out, none runs away.
//!
//! Run: cargo run --release -p hf-sim --example ecology_census [days] [seed...]
use std::collections::BTreeMap;

use hf_sim::agents::Species;
use hf_sim::events::EventKind as E;
use hf_sim::systems::settlement::tree_census;

const GRAZERS: [Species; 5] = [
    Species::Deer,
    Species::Boar,
    Species::Hare,
    Species::Goat,
    Species::Duck,
];
const ALL_WILD: [Species; 11] = [
    Species::Deer,
    Species::Boar,
    Species::Hare,
    Species::Wolf,
    Species::Fox,
    Species::Crow,
    Species::Bear,
    Species::Duck,
    Species::Goat,
    Species::Rats,
    Species::Centipede,
];
const HUNTERS: [Species; 3] = [Species::Wolf, Species::Fox, Species::Bear];

/// All living of one kind (the count `breed_and_age` tests against the cap).
fn wild_count(w: &hf_sim::World, s: Species) -> u32 {
    w.agents
        .iter()
        .filter(|a| a.alive && a.species == s)
        .count() as u32
}

fn count(w: &hf_sim::World, s: &[Species]) -> usize {
    w.agents
        .iter()
        .filter(|a| a.alive && !a.denned && s.contains(&a.species))
        .count()
}

/// The growth on the browsed plants above the grazed-down floor, and how many there are.
fn browse(w: &hf_sim::World) -> (f32, usize) {
    let floor = hf_sim::systems::animals::GRAZED_DOWN;
    let mut sum = 0.0;
    let mut n = 0;
    for p in w
        .plants
        .iter()
        .filter(|p| p.alive && hf_sim::systems::animals::browsed(p.kind))
    {
        sum += (p.growth - floor).max(0.0);
        n += 1;
    }
    (sum, n)
}

/// One tile of each 16x16 stretch that holds surface water (`fish::stock` is kept per stretch).
fn water_stretches(w: &hf_sim::World) -> Vec<hf_sim::ids::TilePos> {
    let g = &w.grid;
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for y in 0..g.height {
        for x in 0..g.width {
            let p = hf_sim::ids::TilePos::new(x, y, g.ground_z(x, y) + 1);
            if g.get(p).is_water() && seen.insert((x / 16, y / 16)) {
                out.push(p);
            }
        }
    }
    out
}

/// Mean and minimum fish stock over the given stretches (0, 0 with none).
fn fish_row(w: &hf_sim::World, spots: &[hf_sim::ids::TilePos]) -> (f32, f32) {
    if spots.is_empty() {
        return (0.0, 0.0);
    }
    let v: Vec<f32> = spots.iter().map(|p| hf_sim::fish::stock(w, *p)).collect();
    (
        v.iter().sum::<f32>() / v.len() as f32,
        v.iter().cloned().fold(f32::MAX, f32::min),
    )
}

fn census(seed: u64, days: u64) -> String {
    use std::fmt::Write;
    let t = std::time::Instant::now();
    let mut w = hf_sim::World::opening_world(seed);
    let spots = water_stretches(&w);
    let mut out = String::new();
    let _ = writeln!(
        out,
        "seed {seed}\n  day  browse  plants  deer boar hare goat duck | wolf fox bear | crow | people | ducks(here away) | fish(mean min) | trees(mature young stump) | near home(mature young stump)"
    );
    let mut births: BTreeMap<String, u32> = BTreeMap::new();
    let mut deaths: BTreeMap<(String, String), u32> = BTreeMap::new();
    let mut species_of: BTreeMap<u32, Species> = BTreeMap::new();
    let mut seen = w.agents.iter().map(|a| a.id.0).max().unwrap_or(0);
    for a in &w.agents {
        species_of.insert(a.id.0, a.species);
    }
    let mut came: BTreeMap<String, u32> = BTreeMap::new();
    let mut at_cap: BTreeMap<String, u64> = BTreeMap::new();
    let mut lows: BTreeMap<&str, usize> = BTreeMap::new();
    let mut highs: BTreeMap<&str, usize> = BTreeMap::new();
    let row = |w: &hf_sim::World, day: u64, out: &mut String| {
        let (b, n) = browse(w);
        let c = |s: Species| count(w, &[s]);
        let people = count(w, &[Species::Person]);
        let t = tree_census(w);
        let _ = writeln!(
            out,
            "  {day:>3}  {b:>6.1}  {n:>6}  {:>4} {:>4} {:>4} {:>4} {:>4} | {:>4} {:>3} {:>4} | {:>4} | {people:>4} | {:>4} {:>4} | {:>5.1} {:>4.1} | {:>5} {:>5} {:>5} | {:>5} {:>5} {:>5}",
            c(Species::Deer),
            c(Species::Boar),
            c(Species::Hare),
            c(Species::Goat),
            c(Species::Duck),
            c(Species::Wolf),
            c(Species::Fox),
            c(Species::Bear),
            c(Species::Crow),
            c(Species::Duck),
            w.creatures.migrants.len(),
            fish_row(w, &spots).0,
            fish_row(w, &spots).1,
            t.mature,
            t.young,
            t.stumps,
            t.mature_near_home,
            t.young_near_home,
            t.stumps_near_home,
        );
    };
    row(&w, 0, &mut out);
    for n in 0..days * hf_sim::time::TICKS_PER_DAY {
        w.step();
        // New agents by id: the list is not append-only (ducks fly south, the dead are pruned).
        let top = seen;
        for a in w.agents.iter().filter(|a| a.id.0 > top) {
            species_of.insert(a.id.0, a.species);
            if a.species != Species::Person {
                *births.entry(format!("{:?}", a.species)).or_default() += 1;
            }
            seen = seen.max(a.id.0);
        }
        for e in &w.last_events {
            if let E::CameOver { agent } = &e.kind {
                if let Some(s) = species_of.get(&agent.0) {
                    *came.entry(format!("{s:?}")).or_default() += 1;
                }
            }
            if let E::Died { agent, cause } = &e.kind {
                if let Some(s) = species_of.get(&agent.0) {
                    if *s != Species::Person {
                        *deaths.entry((format!("{s:?}"), cause.clone())).or_default() += 1;
                    }
                }
            }
        }
        if (n + 1) % hf_sim::time::TICKS_PER_DAY == 0 {
            let day = (n + 1) / hf_sim::time::TICKS_PER_DAY;
            for (name, set) in [
                ("grazers", &GRAZERS[..]),
                ("hunters", &HUNTERS[..]),
                ("crows", &[Species::Crow][..]),
            ] {
                let k = count(&w, set);
                let lo = lows.entry(name).or_insert(usize::MAX);
                *lo = (*lo).min(k);
                let hi = highs.entry(name).or_insert(0);
                *hi = (*hi).max(k);
            }
            for s in ALL_WILD {
                if wild_count(&w, s) as usize
                    == hf_sim::systems::animals::wild_cap(s).unwrap_or(usize::MAX)
                {
                    *at_cap.entry(format!("{s:?}")).or_default() += 1;
                }
            }
            if day.is_multiple_of(30) {
                row(&w, day, &mut out);
            }
        }
    }
    // An adult that came over the hills (`CameOver`) is an arrival, not a birth (#1665,
    // FIXLOG 2026-11-16): take those agents out of the births and print them apart.
    for (k, n) in &came {
        if let Some(b) = births.get_mut(k) {
            *b = b.saturating_sub(*n);
        }
    }
    births.retain(|_, n| *n > 0);
    let _ = writeln!(out, "  births (not incomers): {births:?}");
    let _ = writeln!(out, "  came over the hills: {came:?}");
    let share: Vec<String> = ALL_WILD
        .iter()
        .map(|s| {
            let d = at_cap.get(&format!("{s:?}")).copied().unwrap_or(0);
            format!("{s:?} {:.0}%", 100.0 * d as f64 / days.max(1) as f64)
        })
        .collect();
    let _ = writeln!(out, "  days at the cap: {}", share.join(", "));
    for ((s, cause), k) in &deaths {
        let _ = writeln!(out, "  died {s}: {cause} x{k}");
    }
    let _ = writeln!(
        out,
        "  range over the run (excluding day 0): grazers {}..{}, hunters {}..{}, crows {}..{}  ({:.0} s)",
        lows["grazers"],
        highs["grazers"],
        lows["hunters"],
        highs["hunters"],
        lows["crows"],
        highs["crows"],
        t.elapsed().as_secs_f64()
    );
    out
}

fn main() {
    let args: Vec<u64> = std::env::args()
        .skip(1)
        .filter_map(|a| a.parse().ok())
        .collect();
    let days = args.first().copied().unwrap_or(600);
    let seeds: Vec<u64> = if args.len() > 1 {
        args[1..].to_vec()
    } else {
        vec![1, 2, 3, 4]
    };
    let handles: Vec<_> = seeds
        .into_iter()
        .map(|s| std::thread::spawn(move || census(s, days)))
        .collect();
    for h in handles {
        print!("{}", h.join().unwrap());
    }
}
