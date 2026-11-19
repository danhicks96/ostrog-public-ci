//! The warmth census behind #1368: for each seed, steps an opening world for `days` days and
//! buckets every living person, each hour, by where they stand (`hazards::warmth_bucket`: see
//! that function for exactly how "hearth", "roofed, no hearth", "roofed, open side",
//! "tent/nest", "below ground" and "open" are told apart).
//!
//! Read-only; no simulation behaviour is touched. Person-hours, the share with warmth below
//! 0.35 ("cold" per `hazards::cold_step`), the share holding the `Cold` thought, frostbite
//! wounds and froze deaths are counted per bucket, split by season and by asleep/awake.
//!
//! Run: cargo run --release -p hf-sim --example warmth_census <days> [seed...]
//! e.g. cargo run --release -p hf-sim --example warmth_census 600 1 2 3 4

use hf_sim::agents::Species;
use hf_sim::events::EventKind as E;
use hf_sim::mind::ThoughtKind;
use hf_sim::systems::hazards::warmth_bucket;
use hf_sim::time::{Calendar, Season, TICKS_PER_HOUR};

#[derive(Default, Clone, Copy)]
struct Stats {
    person_hours: u64,
    warmth_low: u64,
    cold_thought: u64,
    frostbite: u64,
    froze: u64,
}

fn season_label(s: Season) -> &'static str {
    match s {
        Season::Spring => "spring",
        Season::Summer => "summer",
        Season::Autumn => "autumn",
        Season::Winter => "winter",
    }
}

/// `Season` has no `Ord`, so the census keys on this instead.
fn season_idx(s: Season) -> u8 {
    match s {
        Season::Spring => 0,
        Season::Summer => 1,
        Season::Autumn => 2,
        Season::Winter => 3,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let days: u64 = args.first().and_then(|a| a.parse().ok()).unwrap_or(600);
    let seeds: Vec<u64> = args[1.min(args.len())..]
        .iter()
        .filter_map(|a| a.parse().ok())
        .collect();
    let seeds = if seeds.is_empty() {
        vec![1, 2, 3, 4]
    } else {
        seeds
    };

    for seed in seeds {
        let mut w = hf_sim::World::opening_world(seed);
        // (bucket, season, awake) -> Stats
        let mut rows: std::collections::BTreeMap<(&'static str, u8, bool), Stats> =
            Default::default();
        for _ in 0..days * hf_sim::time::TICKS_PER_DAY {
            w.step();
            let tick = w.meta.tick;
            // Sample person-hours and the warmth/thought shares on the same hourly cadence
            // `hazards::cold_step` runs on.
            if tick % TICKS_PER_HOUR == 300 {
                let cal = Calendar::from_tick(tick);
                for a in &w.agents {
                    if !a.alive || a.species != Species::Person {
                        continue;
                    }
                    let b = warmth_bucket(&w, a.pos);
                    let key = (b, season_idx(cal.season), !a.activity.is_asleep());
                    let s = rows.entry(key).or_default();
                    s.person_hours += 1;
                    if a.needs.warmth < 0.35 {
                        s.warmth_low += 1;
                    }
                    if a.thoughts.has(ThoughtKind::Cold) {
                        s.cold_thought += 1;
                    }
                }
            }
            // Frostbite wounds and froze deaths, attributed to the bucket the person stood in
            // at the tick the event fired.
            for e in &w.last_events {
                let (agent, is_frostbite, is_froze) = match &e.kind {
                    E::Wounded {
                        agent, kind, cause, ..
                    } => (
                        *agent,
                        kind == "frostbite" && cause == "from the cold",
                        false,
                    ),
                    E::Died { agent, cause } => (*agent, false, cause == "froze"),
                    _ => continue,
                };
                if !is_frostbite && !is_froze {
                    continue;
                }
                if let Some(a) = w.agents.iter().find(|a| a.id == agent) {
                    let cal = Calendar::from_tick(tick);
                    let b = warmth_bucket(&w, a.pos);
                    let key = (b, season_idx(cal.season), !a.activity.is_asleep());
                    let s = rows.entry(key).or_default();
                    if is_frostbite {
                        s.frostbite += 1;
                    }
                    if is_froze {
                        s.froze += 1;
                    }
                }
            }
        }
        println!("seed {seed}, {days} days:");
        println!(
            "  {:<18} {:<7} {:<6} {:>12} {:>10} {:>10} {:>10} {:>6}",
            "bucket", "season", "awake", "person-hrs", "%<0.35", "%cold", "frostbite", "froze"
        );
        for ((b, season_i, awake), s) in &rows {
            if s.person_hours == 0 && s.frostbite == 0 && s.froze == 0 {
                continue;
            }
            let season = match season_i {
                0 => Season::Spring,
                1 => Season::Summer,
                2 => Season::Autumn,
                _ => Season::Winter,
            };
            let pct_low = if s.person_hours > 0 {
                100.0 * s.warmth_low as f32 / s.person_hours as f32
            } else {
                0.0
            };
            let pct_cold = if s.person_hours > 0 {
                100.0 * s.cold_thought as f32 / s.person_hours as f32
            } else {
                0.0
            };
            println!(
                "  {:<18} {:<7} {:<6} {:>12} {:>9.1}% {:>9.1}% {:>10} {:>6}",
                b,
                season_label(season),
                if *awake { "awake" } else { "asleep" },
                s.person_hours,
                pct_low,
                pct_cold,
                s.frostbite,
                s.froze
            );
        }
    }
}
