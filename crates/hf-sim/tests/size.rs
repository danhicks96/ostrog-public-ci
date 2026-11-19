//! What size changes (#422): food eaten by mass, blows and loads by mass, what a big body can
//! knock down, and each animal's own size within its kind's, grown into with age.

use hf_sim::agents::Species;
use hf_sim::genetics::{body, locus};
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_YEAR};
use hf_sim::World;

fn world() -> World {
    let mut w = World::generate(4221, hf_sim::world::gen::GenParams::small());
    w.agents.clear();
    w.plants.clear();
    w
}

/// A grown animal of `species` at the surface of (x, y), of middling size unless `size` says.
fn beast(w: &mut World, species: Species, x: i32, y: i32, size: u8) -> usize {
    let p = w.grid.surface(x, y);
    let id = w.spawn_agent(species, p);
    let i = w.agents.iter().position(|a| a.id == id).unwrap();
    w.agents[i].born = w.meta.tick as i64 - 10 * TICKS_PER_YEAR as i64;
    w.agents[i].genome.body[body::SIZE] = size;
    w.agents[i].genome.loci[locus::BUILD] = size;
    // A spawn's runt/giant roll rides on its agent id, and world gen (populate's habitat
    // placement) decides which ids a test's animals get; these fixtures mean a plain animal
    // of `size`, so pin the roll and leave size claims to the assertions
    // (FIXLOG 2026-10-31, c145 / PR #495).
    let mut nonce = 0u8;
    while w.agents[i].build() != hf_sim::size::Build::Plain {
        w.agents[i].genome.loci[locus::LONGEVITY] = nonce;
        nonce += 1;
    }
    i
}

/// Five days of hunger and grazing for the animals at `idx`, each by a fern of its own;
/// returns the growth each ate in a day.
fn graze_days(w: &mut World, idx: &[usize]) -> Vec<f32> {
    let mut plants = Vec::new();
    for &i in idx {
        w.agents[i].needs.hunger = 1.0;
        plants.push(w.plants.len());
        w.plants
            .push(Plant::new(w.agents[i].pos, PlantKind::Fern, 1.0, 0));
    }
    let mut eaten = vec![0.0; idx.len()];
    for _ in 0..5 * TICKS_PER_DAY {
        w.meta.tick += 1;
        hf_sim::systems::needs::step(w);
        if w.meta.tick.is_multiple_of(40) {
            for (n, &i) in idx.iter().enumerate() {
                let k = plants[n];
                let before = w.plants[k].growth;
                hf_sim::systems::animals::graze_bite(w, i);
                eaten[n] += before - w.plants[k].growth;
                // A fresh plant every time, so none runs out.
                w.plants[k].growth = 1.0;
            }
        }
    }
    eaten.iter().map(|e| e / 5.0).collect()
}

#[test]
fn a_big_grazer_eats_more_than_a_small_one_and_people_are_as_they_were() {
    let mut w = world();
    let deer = beast(&mut w, Species::Deer, 30, 30, 128);
    let hare = beast(&mut w, Species::Hare, 40, 30, 128);
    let small_deer = beast(&mut w, Species::Deer, 30, 40, 0);
    let big_deer = beast(&mut w, Species::Deer, 40, 40, 255);
    let eaten = graze_days(&mut w, &[deer, hare, small_deer, big_deer]);
    let scale = |i: usize| hf_sim::genetics::hunger_scale(w.agents[i].body_kg(w.meta.tick));
    // A deer eats about as much more than a hare as its size says (mass to the 0.75).
    let want = scale(deer) / scale(hare);
    let got = eaten[0] / eaten[1];
    assert!(want > 8.0, "a deer is far hungrier than a hare: {want}");
    assert!(
        (got / want - 1.0).abs() < 0.15,
        "eaten {got} vs size {want}"
    );
    // Within the kind, the bigger one eats more.
    assert!(eaten[3] > eaten[2] * 1.2, "{eaten:?}");
    // A grown deer strips about two thirds of a plant's growth in a day.
    assert!((0.4..1.2).contains(&eaten[0]), "{eaten:?}");
    // A person of the human mean gets hungry as before.
    assert!((hf_sim::genetics::hunger_scale(hf_sim::genetics::REFERENCE_KG) - 1.0).abs() < 1e-6);
}

#[test]
fn grazing_strips_the_browse_and_leaves_peoples_food_alone() {
    let mut w = world();
    let deer = beast(&mut w, Species::Deer, 30, 30, 128);
    let at = w.agents[deer].pos;
    w.agents[deer].needs.hunger = 0.0;
    // Berries and reeds by it are not eaten; the shrub is, down to what grows back.
    w.plants.push(Plant::new(at, PlantKind::BerryBush, 1.0, 0));
    w.plants.push(Plant::new(at, PlantKind::Reed, 1.0, 0));
    w.plants
        .push(Plant::new(at.offset(1, 0, 0), PlantKind::Shrub, 0.5, 0));
    for _ in 0..20 {
        w.agents[deer].needs.hunger = 0.0;
        hf_sim::systems::animals::graze_bite(&mut w, deer);
    }
    assert_eq!(w.plants[0].growth, 1.0, "berries are people's food");
    assert_eq!(w.plants[1].growth, 1.0, "reeds are thatch");
    assert!(
        (w.plants[2].growth - hf_sim::systems::animals::GRAZED_DOWN).abs() < 1e-5,
        "the shrub is grazed down: {}",
        w.plants[2].growth
    );
}

/// The mean severity of `n` blows by `y` on `x` that land, healing `x` between them.
fn mean_blow(w: &mut World, y: usize, x: usize, n: usize) -> f32 {
    use rand::SeedableRng;
    let mut r = rand_chacha::ChaCha8Rng::seed_from_u64(422);
    let (mut sum, mut landed) = (0.0, 0);
    for _ in 0..n {
        w.agents[x].body = Default::default();
        w.agents[x].alive = true;
        if hf_sim::systems::combat::blow(w, y, x, &mut r, "in a test") {
            sum += w.agents[x]
                .body
                .wounds
                .iter()
                .map(|wd| wd.severity)
                .sum::<f32>();
            landed += 1;
        }
    }
    sum / landed.max(1) as f32
}

/// Blows by `y` until `x` is hurt past 1.0 (as `stand_against` fells a beast).
fn blows_to_fell(w: &mut World, y: usize, x: usize) -> u32 {
    use rand::SeedableRng;
    let mut r = rand_chacha::ChaCha8Rng::seed_from_u64(4220);
    w.agents[x].body = Default::default();
    let mut n = 0;
    while hf_sim::systems::combat::battered(w, x) < 1.0 && n < 200 {
        hf_sim::systems::combat::blow(w, y, x, &mut r, "in a test");
        n += 1;
    }
    n
}

#[test]
fn a_bear_hits_harder_than_a_person_and_a_hare_hardly_at_all() {
    let mut w = world();
    w.meta.founding_site = w.grid.surface(30, 30);
    let ids = w.found_party(2);
    let p = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let q = w.agents.iter().position(|a| a.id == ids[1]).unwrap();
    for i in [p, q] {
        // Both of the human mean, unarmed.
        w.agents[i].genome.body[body::SIZE] = 128;
        w.agents[i].genome.loci[locus::BUILD] = 128;
        w.agents[i].weapon = None;
        w.agents[i].armour = None;
    }
    // The game has no rats; the hare is its smallest grazer and stands in for one.
    let bear = beast(&mut w, Species::Bear, 34, 30, 128);
    let rat = beast(&mut w, Species::Hare, 36, 30, 128);
    let person = mean_blow(&mut w, p, q, 400);
    let by_bear = mean_blow(&mut w, bear, q, 400);
    let by_rat = mean_blow(&mut w, rat, q, 400);
    // A person's blow on a person is as before the change: 0.2 to 0.6, a mean near 0.4.
    assert!((0.33..0.47).contains(&person), "person on person {person}");
    assert!(by_bear > 2.0 * person, "bear {by_bear} vs person {person}");
    assert!(by_rat < 0.3 * person, "rat {by_rat} vs person {person}");
    // Felling: a bear takes a person many blows, a hare one.
    let on_bear = blows_to_fell(&mut w, p, bear);
    let on_rat = blows_to_fell(&mut w, p, rat);
    let on_person = blows_to_fell(&mut w, p, q);
    assert!(on_rat <= 1, "a hare falls to one blow: {on_rat}");
    assert!(
        on_bear > on_person && on_bear >= 3,
        "a bear takes more blows than a person: {on_bear} vs {on_person}"
    );
    assert!(on_bear < 40, "but it can be felled: {on_bear}");
}

#[test]
fn a_small_body_carries_less_and_wolves_drag_down_by_weight() {
    let mut w = world();
    w.meta.founding_site = w.grid.surface(30, 30);
    let ids = w.found_party(1);
    let p = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    w.agents[p].genome.body[body::SIZE] = 128;
    w.agents[p].genome.loci[locus::BUILD] = 128;
    let logs = hf_sim::items::ItemKind::Log;
    assert_eq!(
        w.agents[p].load(logs),
        logs.max_stack(),
        "a person carries a stack"
    );
    w.agents[p].genome.body[body::SIZE] = 0;
    w.agents[p].genome.loci[locus::BUILD] = 0;
    assert!(
        w.agents[p].load(logs) < logs.max_stack(),
        "a slight one less"
    );
    // Two grown wolves drag down a grown deer; two cubs cannot.
    let deer = beast(&mut w, Species::Deer, 40, 40, 128);
    let a = beast(&mut w, Species::Wolf, 41, 40, 128);
    let b = beast(&mut w, Species::Wolf, 39, 40, 128);
    for i in [a, b] {
        w.agents[i].pos = w.agents[deer].pos;
    }
    let tick = w.meta.tick;
    let pack_might: f32 = [a, b]
        .iter()
        .map(|&i| hf_sim::size::might(w.agents[i].body_kg(tick)))
        .sum();
    let claim = hf_sim::systems::animals::DRAG * hf_sim::size::might(w.agents[deer].body_kg(tick));
    assert!(
        hf_sim::systems::animals::brings_down(&w, deer),
        "pack might {pack_might:.3} vs dragged claim {claim:.3} (deer build {:?})",
        w.agents[deer].build()
    );
    let grown = w.agents[a].born;
    for i in [a, b] {
        w.agents[i].born = w.meta.tick as i64 - (TICKS_PER_YEAR / 4) as i64;
    }
    assert!(!hf_sim::systems::animals::brings_down(&w, deer), "cubs");
    for i in [a, b] {
        w.agents[i].born = grown;
    }
}

/// A wooden door of `wood` at `q`, built as the builders build one.
fn door(w: &mut World, q: hf_sim::ids::TilePos, wood: hf_sim::world::Material) {
    let mut t = hf_sim::world::Tile::solid(hf_sim::world::Material::WOOD);
    t.flags = hf_sim::world::TileFlags(hf_sim::world::TileFlags::DOOR);
    w.grid.set(q, t);
    w.built_of.insert(q, wood);
}

#[test]
fn a_heavy_beast_breaks_a_wooden_door_and_a_small_one_cannot() {
    use hf_sim::events::EventKind;
    use hf_sim::systems::animals::{can_break, try_break};
    use hf_sim::world::Material;
    let mut w = world();
    let bear = beast(&mut w, Species::Bear, 30, 30, 128);
    let crawler = beast(&mut w, Species::Crawler, 30, 31, 128);
    let wolf = beast(&mut w, Species::Wolf, 30, 32, 128);
    let at = w.agents[bear].pos;
    let pine = at.offset(1, 0, 0);
    let oak = at.offset(2, 0, 0);
    door(&mut w, pine, Material::PINE);
    door(&mut w, oak, Material::OAK);
    // A stone wall: nothing breaks it by weight.
    let stone = at.offset(3, 0, 0);
    w.grid
        .set(stone, hf_sim::world::Tile::solid(Material::STONE_BLOCK));
    w.built_of.insert(stone, Material::GRANITE);
    assert!(!can_break(&w, wolf, pine), "a wolf cannot break even pine");
    assert!(can_break(&w, crawler, pine) && !can_break(&w, crawler, oak));
    assert!(can_break(&w, bear, oak) && !can_break(&w, bear, stone));
    // A bear cub cannot.
    let grown = w.agents[bear].born;
    w.agents[bear].born = w.meta.tick as i64;
    assert!(!can_break(&w, bear, pine), "a cub");
    w.agents[bear].born = grown;
    assert!(!try_break(&mut w, wolf, pine));
    assert!(try_break(&mut w, bear, oak));
    assert!(w.grid.walkable(oak) || !w.grid.get(oak).shape.is_solid());
    let id = w.agents[bear].id;
    assert!(w.events.drain().iter().any(|e| matches!(
        &e.kind,
        EventKind::BrokeIn { beast, of, door: true } if *beast == id && of == "oak"
    )));
}

#[test]
fn the_beast_from_below_breaks_in_when_nobody_is_out() {
    let mut w = world();
    w.meta.founding_site = w.grid.surface(30, 30);
    let ids = w.found_party(1);
    let p = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let inside = w.agents[p].pos;
    // Under a roof, with a pine door two tiles off.
    w.grid.set(
        inside.offset(0, 0, 1),
        hf_sim::world::Tile::solid(hf_sim::world::Material::WOOD),
    );
    assert!(hf_sim::systems::hazards::indoors(&w, inside));
    let q = inside.offset(2, 0, 0);
    door(&mut w, q, hf_sim::world::Material::PINE);
    let c = beast(&mut w, Species::Crawler, q.x + 1, q.y, 128);
    w.agents[c].pos = q.offset(1, 0, 0);
    let id = w.agents[c].id;
    w.creatures.risen.push(id);
    assert!(hf_sim::systems::animals::break_in(&mut w, c));
    assert!(
        !w.grid.get(q).flags.has(hf_sim::world::TileFlags::DOOR),
        "the door is down"
    );
    // A wolf in the same place does nothing of the kind.
    door(&mut w, q, hf_sim::world::Material::PINE);
    let wolf = beast(&mut w, Species::Wolf, q.x + 1, q.y, 128);
    w.agents[wolf].pos = q.offset(1, 0, 0);
    assert!(!hf_sim::systems::animals::break_in(&mut w, wolf));
}

/// A genome that is a giant (or a runt) of its kind, found by drawing.
fn genome_of(build: hf_sim::size::Build) -> hf_sim::genetics::Genome {
    use rand::SeedableRng;
    let mut r = rand_chacha::ChaCha8Rng::seed_from_u64(42);
    loop {
        let mut g = hf_sim::genetics::Genome::random(&mut r);
        g.body[body::SIZE] = 128;
        g.loci[locus::BUILD] = 128;
        if hf_sim::size::build_of(&g) == build {
            return g;
        }
    }
}

#[test]
fn each_animal_has_its_own_size_and_grows_into_it() {
    use hf_sim::size::{build_of, Build};
    use rand::SeedableRng;
    // Runts and giants: about one in two hundred each.
    let mut r = rand_chacha::ChaCha8Rng::seed_from_u64(7);
    let (mut runts, mut giants) = (0, 0);
    for _ in 0..40_000 {
        match build_of(&hf_sim::genetics::Genome::random(&mut r)) {
            Build::Runt => runts += 1,
            Build::Giant => giants += 1,
            Build::Plain => {}
        }
    }
    assert!(
        (140..260).contains(&runts) && (140..260).contains(&giants),
        "{runts} {giants}"
    );
    // The spread within a kind: the plain ones of a herd lie within its range, which is 10 to
    // 20% either side of its mean.
    let mut w = world();
    let mut kg = Vec::new();
    for k in 0..60 {
        let id = w.spawn_agent(Species::Deer, w.grid.surface(20 + k % 10, 20 + k / 10));
        let a = w.agents.iter().find(|a| a.id == id).unwrap();
        if a.build() == Build::Plain {
            kg.push(a.mass_kg());
        }
    }
    let (lo, hi) = kg
        .iter()
        .fold((f32::MAX, 0.0f32), |(l, h), m| (l.min(*m), h.max(*m)));
    let mean = hf_sim::size::mean_kg(Species::Deer);
    assert!(lo >= mean * 0.8 && hi <= mean * 1.2, "{lo} {hi}");
    assert!(hi - lo > mean * 0.1, "they differ: {lo} {hi}");
    // Young grow into it: a newborn is three tenths of its grown size, a yearling deer about
    // two thirds, a two-year-old grown.
    let d = beast(&mut w, Species::Deer, 30, 30, 128);
    let grown = w.agents[d].mass_kg();
    let now = w.meta.tick;
    for (years, frac) in [(0.0, 0.3), (1.0, 0.65), (2.0, 1.0), (5.0, 1.0)] {
        w.agents[d].born = now as i64 - (years * TICKS_PER_YEAR as f32) as i64;
        let got = w.agents[d].body_kg(now) / grown;
        assert!((got - frac).abs() < 0.02, "{years} years: {got}");
    }
    // A runt is smaller and a giant bigger than a plain one of the same genes otherwise.
    let plain = w.agents[d].mass_kg();
    w.agents[d].genome = genome_of(Build::Giant);
    assert!((w.agents[d].mass_kg() / plain - 1.6).abs() < 0.01);
    w.agents[d].genome = genome_of(Build::Runt);
    assert!((w.agents[d].mass_kg() / plain - 0.6).abs() < 0.01);
    // People are never runts or giants by this.
    w.meta.founding_site = w.grid.surface(30, 30);
    let ids = w.found_party(1);
    let p = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    w.agents[p].genome = genome_of(Build::Giant);
    assert_eq!(w.agents[p].build(), Build::Plain);
}

#[test]
fn a_giant_is_hungrier_harder_to_drag_down_and_gets_a_name() {
    use hf_sim::size::Build;
    let mut w = world();
    let deer = beast(&mut w, Species::Deer, 40, 40, 128);
    let plain = beast(&mut w, Species::Deer, 30, 40, 128);
    w.agents[deer].genome = genome_of(Build::Giant);
    let eaten = graze_days(&mut w, &[deer, plain]);
    assert!(eaten[0] > 1.3 * eaten[1], "a giant eats more: {eaten:?}");
    // Two grown wolves cannot drag the giant down; three can.
    let wolves: Vec<usize> = (0..3)
        .map(|k| beast(&mut w, Species::Wolf, 41, 40 + k, 128))
        .collect();
    for &k in &wolves {
        w.agents[k].pos = w.agents[deer].pos;
    }
    w.agents[wolves[2]].alive = false;
    assert!(!hf_sim::systems::animals::brings_down(&w, deer), "two");
    w.agents[wolves[2]].alive = true;
    assert!(hf_sim::systems::animals::brings_down(&w, deer), "three");
    // Seen by people, it is given a name for its size.
    w.meta.founding_site = w.agents[deer].pos;
    w.found_party(1);
    hf_sim::systems::animals::name_giants(&mut w);
    assert_ne!(w.agents[deer].name, "deer");
    assert_eq!(w.agents[plain].name, "deer", "a plain one is not named");
    let id = w.agents[deer].id;
    assert!(w.events.drain().iter().any(|e| matches!(
        &e.kind,
        hf_sim::events::EventKind::Named { agent, because, .. } if *agent == id && because.contains("bigger")
    )));
}

#[test]
fn a_hungry_grazer_with_nothing_near_goes_looking() {
    // #422 part 5: hungry grazers on bare ground stood still, and fewer came near the hunters.
    let mut w = world();
    let deer: Vec<usize> = (0..20)
        .map(|k| {
            beast(
                &mut w,
                Species::Deer,
                20 + 3 * (k % 5),
                20 + 3 * (k / 5),
                128,
            )
        })
        .collect();
    for &d in &deer {
        w.agents[d].needs.hunger = 0.1;
    }
    for _ in 0..hf_sim::systems::animals::STRIPE {
        w.meta.tick += 1;
        hf_sim::systems::animals::step(&mut w);
    }
    let off = deer
        .iter()
        .filter(|&&d| !w.agents[d].path.is_empty() || w.agents[d].is_moving())
        .count();
    assert!(off >= 18, "{off} of 20 set off to look for forage");
}
