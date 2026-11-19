//! The census behind the MP reviews' history tables (docs/reviews/stories-2026-10-18,
//! relations-2026-10-18, deep-2026-10-19): for seeds 1 to 4, a 1,000-year history from the default
//! setup, timed, with its stories, relations and the deep counted, and a few rows as lines.
//!
//! Run: cargo run --release -p hf-sim --example history_census [first_seed last_seed]
//! (Times depend on the machine and its load; the reviews say what else was running.)

use hf_sim::realm::sites::RealmEventKind as K;

fn main() {
    let args: Vec<u64> = std::env::args()
        .skip(1)
        .filter_map(|a| a.parse().ok())
        .collect();
    let (from, to) = match args.as_slice() {
        [a, b] => (*a, *b),
        _ => (1, 4),
    };
    for seed in from..=to {
        let mut s = hf_sim::setup::WorldSetup::new(seed);
        s.realm_years = 1000;
        let t = std::time::Instant::now();
        let mut w = hf_sim::World::from_setup(&s);
        let secs = t.elapsed().as_secs_f32();
        hf_sim::systems::flush::step(&mut w);
        let rows: Vec<K> = w
            .last_events
            .iter()
            .filter_map(|e| match &e.kind {
                hf_sim::events::EventKind::Realm { what } => Some(what.clone()),
                _ => None,
            })
            .collect();
        let r = &w.realm;
        let count = |f: &dyn Fn(&K) -> bool| rows.iter().filter(|k| f(k)).count();
        println!("seed {seed}: history {secs:.1} s, {} rows", rows.len());
        // Stories (P8).
        let mut kinds: std::collections::BTreeMap<String, usize> = Default::default();
        for st in &r.stories {
            *kinds.entry(format!("{:?}", st.kind)).or_default() += 1;
        }
        println!(
            "  stories {} (changed in retelling {}), cultures {}, grudges {}, gods {}, by kind {:?}",
            r.stories.len(),
            r.stories.iter().filter(|s| s.version > 0).count(),
            r.cultures.len(),
            r.cultures.iter().map(|c| c.grudges.len()).sum::<usize>(),
            r.cultures.iter().map(|c| c.gods.len()).sum::<usize>(),
            kinds
        );
        // Relations (P9).
        let across = rows
            .iter()
            .filter(|k| {
                matches!(k, K::Married { a, b, .. }
                if r.figures[*a as usize].people != r.figures[*b as usize].people)
            })
            .count();
        println!(
            "  traded {}, married {} ({across} across peoples), allied {}, battles with allies {}, fled {}, related culture pairs {}",
            count(&|k| matches!(k, K::Traded { .. })),
            count(&|k| matches!(k, K::Married { .. })),
            count(&|k| matches!(k, K::Allied { .. })),
            count(&|k| matches!(k, K::Battle { with, .. } if !with.is_empty())),
            count(&|k| matches!(k, K::Fled { .. })),
            r.relations.len()
        );
        // The deep (P10).
        let deep: Vec<String> = r
            .peoples
            .iter()
            .filter(|p| p.deep)
            .map(|p| format!("{} ({})", p.name, w.kind(p.species).name))
            .collect();
        println!(
            "  deep peoples {deep:?}; deep settlements {} (living {}); beasts {}; attacks {}; ruined {}",
            r.sites.iter().filter(|s| s.deep).count(),
            r.sites.iter().filter(|s| s.deep && s.alive()).count(),
            r.beasts.len(),
            count(&|k| matches!(k, K::BeastAttacked { .. })),
            r.beasts.iter().map(|b| b.ruined.len()).sum::<usize>()
        );
        if seed == from {
            for name in [
                "Traded",
                "Married",
                "Allied",
                "Fled",
                "BeastWoke",
                "BeastAttacked",
            ] {
                if let Some(k) = rows.iter().find(|k| format!("{k:?}").starts_with(name)) {
                    println!("  e.g. {}", hf_sim::realm::chronicle::line(r, k));
                }
            }
        }
    }
}
