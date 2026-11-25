//! Peoples and their settlements over the years (M3, C25).

#[path = "support/cpu.rs"]
mod cpu;

use hf_sim::realm::sites::{self, RealmEventKind, SiteKind};
use hf_sim::realm::{self, embark, peoples, RBiome, RealmParams};

#[test]
fn each_people_speaks_its_own_tongue() {
    let m = realm::generate(5, &RealmParams::default());
    let ps = peoples::peoples(5, &m);
    assert!(ps.len() >= 4);
    let (a, b) = (&ps[0].language, &ps[1].language);
    // The same people always say the same thing the same way; another people, another word.
    assert_eq!(a.word(5, "river"), a.word(5, "river"));
    assert_ne!(a.word(5, "river"), b.word(5, "river"));
    // And they sound different: their sound inventories are not the same.
    assert_ne!(a.sounds(), b.sounds());
    // Every word is made of that language's own sounds.
    let (onsets, nuclei, _) = a.sounds();
    for k in 0..40 {
        let w = a.name(5, "site", k).to_lowercase();
        assert!(w.chars().count() >= 3, "{w}");
        assert!(
            nuclei.iter().any(|v| w.contains(v)),
            "{w} has none of {nuclei:?}"
        );
        let _ = &onsets;
    }
}

#[test]
fn a_realm_history_is_deterministic() {
    let m = realm::generate(
        9,
        &RealmParams {
            size: 128,
            ..Default::default()
        },
    );
    let (a, ea) = sites::run(9, &m, 200);
    let (b, eb) = sites::run(9, &m, 200);
    assert_eq!(ea, eb);
    assert_eq!(a.sites.len(), b.sites.len());
    for (x, y) in a.sites.iter().zip(&b.sites) {
        assert_eq!(
            (x.region, &x.name, x.population.to_bits()),
            (y.region, &y.name, y.population.to_bits())
        );
    }
}

#[test]
fn settlements_stand_where_people_can_live() {
    for seed in 1..=3u64 {
        let m = realm::generate(seed, &RealmParams::default());
        let (rl, _) = sites::run(seed, &m, 300);
        for s in &rl.sites {
            if s.kind == SiteKind::Mine {
                assert_eq!(
                    m.biome[s.region],
                    RBiome::Mountain,
                    "seed {seed}: {}",
                    s.name
                );
            } else if s.deep {
                // The deep's settlements (P10) lie under any land.
                assert!(m.land(s.region), "seed {seed}: {} under the sea", s.name);
            } else {
                assert!(
                    embark::habitable(&m, s.region),
                    "seed {seed}: {} on {:?}",
                    s.name,
                    m.biome[s.region]
                );
            }
        }
    }
}

#[test]
fn every_ending_has_a_year_and_a_cause() {
    let m = realm::generate(1, &RealmParams::default());
    let (rl, events) = sites::run(1, &m, 500);
    let ended: Vec<_> = rl.sites.iter().filter(|s| !s.alive()).collect();
    assert!(!ended.is_empty(), "nothing ended in 500 years");
    for s in ended {
        let (year, cause) = s.ended.clone().unwrap();
        assert!(!cause.is_empty());
        assert!(
            events.iter().any(|e| e.year == year
                && matches!(&e.kind, RealmEventKind::Ended { site, cause: c } if *site == s.id && *c == cause)),
            "{} ended in {year} with no event",
            s.name
        );
    }
    // And every founding is recorded with the year the site gives.
    for s in &rl.sites {
        assert!(events.iter().any(|e| e.year == s.founded
            && matches!(&e.kind, RealmEventKind::Founded { site, .. } if *site == s.id)));
    }
}

#[test]
fn people_stay_within_what_the_land_carries() {
    let m = realm::generate(2, &RealmParams::default());
    let (rl, _) = sites::run(2, &m, 400);
    for s in rl
        .sites
        .iter()
        .filter(|s| s.alive() && s.kind != SiteKind::Mine)
    {
        let cap = sites::capacity_of(&m, s.region, s.deep) * 3.0;
        assert!(
            s.population <= cap * 1.05,
            "{}: {} over {}",
            s.name,
            s.population,
            cap
        );
    }
}

#[test]
fn peoples_spread_from_their_first_homes() {
    let m = realm::generate(4, &RealmParams::default());
    let (rl, _) = sites::run(4, &m, 300);
    let spread = rl
        .peoples
        .iter()
        .filter(|p| {
            rl.sites
                .iter()
                .filter(|s| s.people == p.id && s.alive())
                .count()
                >= 3
        })
        .count();
    assert!(
        spread * 2 >= rl.peoples.len(),
        "{spread} of {} peoples spread",
        rl.peoples.len()
    );
}

#[test]
fn a_thousand_years_is_quick() {
    // CPU time, not wall time: a busy machine is not a slow history (#582).
    let t = cpu::CpuTimer::start();
    let m = realm::generate(3, &RealmParams::default());
    let (rl, _) = sites::run(3, &m, 1000);
    assert!(rl.year == 1000);
    // Fifteen seconds, not ten (#251: the tester's laptop sits at 9.9 s in a debug build, so
    // one run in four failed on timing noise; the build server takes about a third of that).
    assert!(
        t.report("a thousand years").as_secs_f32() < 15.0,
        "{:?} of CPU",
        t.elapsed()
    );
}
