//! The world the valley is part of (M3, docs/design-world.md): the region map.

#[path = "support/cpu.rs"]
mod cpu;

use hf_sim::realm::{self, embark, RBiome, RealmParams};

fn small() -> RealmParams {
    RealmParams {
        size: 96,
        ..Default::default()
    }
}

#[test]
fn realm_is_deterministic() {
    let a = realm::generate(7, &small());
    let b = realm::generate(7, &small());
    let c = realm::generate(8, &small());
    assert_eq!(a.digest(), b.digest());
    assert_ne!(a.digest(), c.digest());
}

#[test]
fn rivers_reach_the_sea_or_a_lake() {
    for seed in 1..=4u64 {
        let m = realm::generate(seed, &small());
        let n = m.size * m.size;
        let mut rivers = 0;
        for start in (0..n).filter(|&i| m.river[i]) {
            rivers += 1;
            let mut i = start;
            let mut steps = 0;
            // Follow the water down; it must come to the sea or a lake without looping.
            while !m.biome[i].is_water() {
                i = m.down[i];
                assert!(
                    i != usize::MAX,
                    "seed {seed}: a river ran off the map's land"
                );
                steps += 1;
                assert!(steps <= n, "seed {seed}: a river loops");
            }
        }
        assert!(rivers > 20, "seed {seed}: only {rivers} river regions");
    }
}

#[test]
fn rain_shadows_fall_behind_ranges() {
    // The wind blows from the west: the land just east of high ground is drier than the land
    // just west of it.
    let (mut windward, mut lee, mut k) = (0.0f32, 0.0f32, 0);
    for seed in 1..=6u64 {
        let m = realm::generate(seed, &RealmParams::default());
        for i in 0..m.size * m.size {
            let (x, y) = m.xy(i);
            if m.height(i) < 0.45 || x < 6 || x + 6 >= m.size {
                continue;
            }
            let (w, e) = (m.idx(x - 5, y), m.idx(x + 5, y));
            if m.land(w) && m.land(e) {
                windward += m.rain[w];
                lee += m.rain[e];
                k += 1;
            }
        }
    }
    assert!(k > 100, "too few ranges to judge ({k})");
    assert!(
        windward > lee * 1.15,
        "windward {:.3} against lee {:.3}",
        windward / k as f32,
        lee / k as f32
    );
}

#[test]
fn a_world_has_many_kinds_of_country() {
    for seed in 1..=4u64 {
        let m = realm::generate(seed, &RealmParams::default());
        let n = (m.size * m.size) as f32;
        let mut kinds = 0;
        for b in [
            RBiome::Tundra,
            RBiome::Taiga,
            RBiome::TemperateForest,
            RBiome::Grassland,
            RBiome::Steppe,
            RBiome::Desert,
            RBiome::Savanna,
            RBiome::Rainforest,
            RBiome::Swamp,
            RBiome::Mountain,
            RBiome::Ice,
        ] {
            if m.biome.iter().filter(|x| **x == b).count() as f32 / n > 0.005 {
                kinds += 1;
            }
        }
        assert!(kinds >= 8, "seed {seed}: {kinds} kinds of country");
    }
}

#[test]
fn the_valley_brings_its_region_s_climate() {
    let m = realm::generate(3, &RealmParams::default());
    let i = embark::default_region(3, &m);
    assert!(embark::habitable(&m, i));
    // Wetter and warmer regions give wetter and warmer valleys.
    let land: Vec<usize> = (0..m.size * m.size)
        .filter(|&j| embark::habitable(&m, j))
        .collect();
    let wettest = *land
        .iter()
        .max_by(|a, b| m.rain[**a].partial_cmp(&m.rain[**b]).unwrap())
        .unwrap();
    let driest = *land
        .iter()
        .min_by(|a, b| m.rain[**a].partial_cmp(&m.rain[**b]).unwrap())
        .unwrap();
    assert!(embark::character_of(&m, wettest).wetness > embark::character_of(&m, driest).wetness);
    let warmest = *land
        .iter()
        .max_by(|a, b| m.temperature[**a].partial_cmp(&m.temperature[**b]).unwrap())
        .unwrap();
    let coldest = *land
        .iter()
        .min_by(|a, b| m.temperature[**a].partial_cmp(&m.temperature[**b]).unwrap())
        .unwrap();
    assert!(embark::character_of(&m, warmest).warmth > embark::character_of(&m, coldest).warmth);
}

#[test]
fn a_world_map_is_quick() {
    // CPU time, not wall time: a busy machine is not a slow map (#582).
    let t = cpu::CpuTimer::start();
    let m = realm::generate(1, &RealmParams::default());
    assert_eq!(m.size, 192);
    assert!(
        t.report("a world map").as_secs_f32() < 5.0,
        "{:?} of CPU",
        t.elapsed()
    );
}

#[test]
fn the_valley_is_a_region_of_its_world() {
    use hf_sim::setup::WorldSetup;
    let s = WorldSetup::new(3);
    let w = hf_sim::World::from_setup(&s);
    let setup = w.meta.setup.clone().unwrap();
    let region = setup.region.expect("the world chose the valley's region");
    let m = realm::generate(3, &RealmParams::default());
    assert!(embark::habitable(&m, region));
    // The valley's climate is its region's.
    let c = embark::character_of(&m, region);
    let got = setup.character.unwrap();
    assert_eq!(
        (got.wetness, got.warmth, got.stoniness),
        (c.wetness, c.warmth, c.stoniness)
    );
    // The world beyond has peoples and settlements, and none of them stands on the valley.
    assert!(w.realm.peoples.len() >= 4);
    assert!(
        w.realm.sites.len() > 50,
        "{} settlements",
        w.realm.sites.len()
    );
    assert!(w.realm.sites.iter().all(|x| x.region != region));
    // Its history is recorded before the arrival.
    let epoch = w.meta.epoch;
    let realm_events = w
        .events
        .pending
        .iter()
        .filter(|e| matches!(e.kind, hf_sim::events::EventKind::Realm { .. }))
        .count();
    assert!(realm_events > 1000, "{realm_events}");
    assert!(w
        .events
        .pending
        .iter()
        .filter(|e| matches!(e.kind, hf_sim::events::EventKind::Realm { .. }))
        .all(|e| e.tick < epoch));
}

#[test]
fn the_valley_s_neighbours_are_the_world_s_nearest_places() {
    use hf_sim::expedition::SiteKind;
    let w = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(5));
    let sites = &w.overworld.sites;
    let villages: Vec<_> = sites
        .iter()
        .filter(|s| s.kind == SiteKind::Village)
        .collect();
    assert!(!villages.is_empty(), "no settlement near the valley");
    for s in sites.iter().filter(|s| s.realm_site.is_some()) {
        let r = &w.realm.sites[s.realm_site.unwrap() as usize];
        assert_eq!(s.name, r.name, "a neighbour carries its own name");
        assert!(s.days >= 1 && s.days <= 12);
        if s.kind == SiteKind::Village {
            assert!(r.alive() && s.people > 0);
        }
        if s.kind == SiteKind::Ruin {
            assert!(!r.alive());
        }
    }
    for s in &w.overworld.sites {
        eprintln!(
            "{} {:?} {:?} {} days, {} people",
            s.name, s.kind, s.edge, s.days, s.people
        );
    }
}

#[test]
fn the_valley_s_past_belongs_to_its_neighbours() {
    // The old hamlets were founded by the people of the world's nearest settlement, named in
    // their tongue, and the founding says whose they were.
    let w = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(3));
    let region = w.meta.setup.as_ref().unwrap().region.unwrap();
    let people = w
        .realm
        .nearest_people(region)
        .expect("a people near the valley")
        .clone();
    let founded: Vec<_> = w
        .events
        .pending
        .iter()
        .filter_map(|e| match &e.kind {
            hf_sim::events::EventKind::OldFounded {
                site, people: p, ..
            } => Some((site.clone(), p.clone())),
            _ => None,
        })
        .collect();
    assert!(!founded.is_empty(), "the valley has a past");
    for (site, p) in &founded {
        assert_eq!(p, &people.name);
        let (_, nuclei, _) = people.language.sounds();
        assert!(
            nuclei.iter().any(|v| site.to_lowercase().contains(v)),
            "{site} in {}'s tongue",
            people.name
        );
    }
}

#[test]
fn the_wizard_shows_the_valley_the_world_will_have() {
    // What the player sees before Begin is what they get: the same region and climate.
    for (seed, region) in [(3u64, None), (7, None), (3, Some(0usize))] {
        let p = hf_sim::setup::preview(seed, None, region);
        let mut s = hf_sim::setup::WorldSetup::new(seed);
        s.region = region;
        let w = hf_sim::World::from_setup(&s);
        let got = w.meta.setup.unwrap();
        assert_eq!(got.region, Some(p.region), "seed {seed}");
        let c = got.character.unwrap();
        assert_eq!(
            (c.wetness, c.warmth, c.stoniness),
            (
                p.character.wetness,
                p.character.warmth,
                p.character.stoniness
            )
        );
    }
}

/// Habitable regions of a seed's world other than the one the seed would choose, far enough
/// apart to be different places.
fn other_regions(seed: u64, n: usize) -> (realm::RealmMap, usize, Vec<usize>) {
    let m = realm::generate(seed, &RealmParams::default());
    let default = embark::default_region(seed, &m);
    let picks: Vec<usize> = (0..m.size * m.size)
        .step_by(97)
        .filter(|&r| r != default && embark::habitable(&m, r))
        .take(n)
        .collect();
    assert_eq!(
        picks.len(),
        n,
        "seed {seed} has {n} other habitable regions"
    );
    (m, default, picks)
}

#[test]
fn each_region_of_the_world_is_its_own_valley() {
    // #560: choosing another place on the world map chose only its climate, so the preview
    // showed the same ground with other colours. Each region now has its own ground.
    let seed = 3;
    let (_, default, picks) = other_regions(seed, 2);
    let a = hf_sim::setup::preview(seed, None, Some(picks[0]));
    let b = hf_sim::setup::preview(seed, None, Some(picks[1]));
    let home = hf_sim::setup::preview(seed, None, Some(default));
    let differ = |x: &[i32], y: &[i32]| x.iter().zip(y).filter(|(p, q)| p != q).count();
    let n = a.heights.len();
    assert!(
        differ(&a.heights, &b.heights) > n / 4,
        "two regions, two grounds"
    );
    assert!(
        differ(&a.heights, &home.heights) > n / 4,
        "a region is not the seed's own ground"
    );
    // The wild kinds the world drew (D65) are the world's, wherever the valley is in it.
    let draw = |p: &hf_sim::setup::Preview| {
        p.lines
            .iter()
            .find(|l| l.starts_with("flora draw:"))
            .cloned()
    };
    assert!(draw(&a).is_some());
    assert_eq!(draw(&a), draw(&b));
    assert_eq!(draw(&a), draw(&home));
    // The same place is the same valley every time, and the seed's own region (chosen or not)
    // keeps the seed's own ground, so worlds made without choosing a region are unchanged.
    let again = hf_sim::setup::preview(seed, None, Some(picks[0]));
    assert_eq!(a.tiles, again.tiles);
    assert_eq!(a.plants, again.plants);
    let unchosen = hf_sim::setup::preview(seed, None, None);
    assert_eq!(unchosen.tiles, home.tiles);
    let own = hf_sim::World::generate(seed, unchosen_params(seed, &unchosen));
    let heights: Vec<i32> = (0..own.grid.height)
        .flat_map(|y| (0..own.grid.width).map(move |x| (x, y)))
        .map(|(x, y)| own.grid.ground_z(x, y))
        .collect();
    assert_eq!(heights, home.heights, "place 0 draws the seed's own ground");
}

fn unchosen_params(seed: u64, p: &hf_sim::setup::Preview) -> hf_sim::world::gen::GenParams {
    hf_sim::world::gen::GenParams {
        character: Some(p.character),
        landform: hf_sim::world::gen::Landform::of_seed(seed),
        ..Default::default()
    }
}

#[test]
fn the_wizard_shows_the_ground_the_world_will_have() {
    // #560: the preview of a chosen region is the ground the world is made on.
    let seed = 3;
    let (_, _, picks) = other_regions(seed, 12);
    let region = picks
        .into_iter()
        .find(|&r| {
            use hf_sim::world::gen::{ground_seed, Landform};
            Landform::of_seed(ground_seed(seed, r as u64 + 1)) != Landform::of_seed(seed)
        })
        .expect("choose a landing site whose landform differs from the default");
    let p = hf_sim::setup::preview(seed, None, Some(region));
    let mut s = hf_sim::setup::WorldSetup::new(seed);
    s.region = Some(region);
    s.history_years = 0;
    s.realm_years = 200;
    let w = hf_sim::World::from_setup(&s);
    assert_eq!(w.meta.params.place, region as u64 + 1);
    assert_ne!(w.meta.params.landform, s.landform());
    let mut same = 0;
    for y in 0..w.grid.height {
        for x in 0..w.grid.width {
            if w.grid.ground_z(x, y) == p.heights[(y * w.grid.width + x) as usize] {
                same += 1;
            }
        }
    }
    let n = (w.grid.width * w.grid.height) as usize;
    // The arrival places a stone and little else: the ground is the previewed ground.
    assert!(same * 100 >= n * 99, "{same} of {n} columns as previewed");
    assert_eq!(w.meta.founding_site, p.founding);
    let landing = w.events.pending.iter().find_map(|event| match &event.kind {
        hf_sim::events::EventKind::SetupChosen { valley, .. } => Some(valley),
        _ => None,
    });
    let expected = format!(
        "{} ({})",
        p.character.label(),
        w.meta.params.landform.name()
    );
    assert_eq!(landing, Some(&expected));
}
