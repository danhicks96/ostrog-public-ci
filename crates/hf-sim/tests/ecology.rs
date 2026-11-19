//! #486: the valley's food web. Each wild kind lives on what it eats, so a change in one moves
//! the others: grazers on the browse, wolves and foxes on the game, crows on carrion, the browse
//! on the ground a carcass greens and the light a felled tree lets in. Every case is a staged
//! scenario with its control.

use hf_sim::agents::Species;
use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::time::{Season, TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

/// A flat empty meadow, no people, no animals, no plants, at dawn on day 0 (spring).
fn meadow(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.agents.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 5;
    w.rebuild_caches();
    w
}

/// Low browse every third tile over the middle of the meadow, fully grown.
fn browse(w: &mut World) {
    for y in (10..54).step_by(3) {
        for x in (10..54).step_by(3) {
            w.plants.push(Plant::new(
                TilePos::new(x, y, 2),
                PlantKind::Shrub,
                1.0,
                (x * 7 + y) as u32,
            ));
        }
    }
}

/// `n` grown animals of `species` about the middle, three years old.
fn herd(w: &mut World, species: Species, n: i32) -> Vec<hf_sim::ids::AgentId> {
    (0..n)
        .map(|k| {
            let p = TilePos::new(20 + (k * 5) % 24, 20 + (k * 3) % 24, 2);
            let id = w.spawn_agent(species, p);
            let a = w.agents.iter_mut().find(|a| a.id == id).unwrap();
            a.born = w.meta.tick as i64 - (hf_sim::time::TICKS_PER_YEAR * 3) as i64;
            id
        })
        .collect()
}

#[derive(Default, Debug)]
struct Tally {
    births: usize,
    died: Vec<String>,
    came: Vec<Species>,
}

fn run(w: &mut World, days: u64) -> Tally {
    let mut t = Tally::default();
    let mut seen = w.agents.iter().map(|a| a.id.0).max().unwrap_or(0);
    for _ in 0..days * TICKS_PER_DAY {
        w.step();
        let top = seen;
        for a in w.agents.iter().filter(|a| a.id.0 > top) {
            t.births += usize::from(a.species != Species::Person);
            seen = seen.max(a.id.0);
        }
        for e in &w.last_events {
            match &e.kind {
                EventKind::Died { cause, .. } => t.died.push(cause.clone()),
                EventKind::CameOver { agent } => {
                    let s = w.agents.iter().find(|a| a.id == *agent).unwrap().species;
                    t.came.push(s);
                    t.births -= 1;
                }
                _ => {}
            }
        }
    }
    t
}

fn alive(w: &World, s: Species) -> usize {
    w.agents
        .iter()
        .filter(|a| a.alive && a.species == s)
        .count()
}

#[test]
fn grazers_breed_on_the_browse_and_starve_without_it() {
    for seed in [1, 2, 3] {
        // With browse: fed, they bear young and none starve.
        let mut fed = meadow(seed);
        browse(&mut fed);
        herd(&mut fed, Species::Deer, 8);
        let t = run(&mut fed, 14);
        let starved = |t: &Tally| {
            t.died
                .iter()
                .filter(|c| *c == "starved, the grazing gone")
                .count()
        };
        assert!(t.births >= 1, "seed {seed}: fed deer bore young: {t:?}");
        assert_eq!(starved(&t), 0, "seed {seed}: none starved by browse: {t:?}");
        // Control, the same herd on bare ground: none bear young after the first days, and
        // they starve.
        let mut bare = meadow(seed);
        herd(&mut bare, Species::Deer, 8);
        let t = run(&mut bare, 14);
        assert!(
            starved(&t) >= 3,
            "seed {seed}: deer on bare ground starved: {t:?}"
        );
        assert!(
            alive(&bare, Species::Deer) < 8,
            "seed {seed}: the bare-ground herd shrank"
        );
    }
}

#[test]
fn wolves_live_on_the_game_and_starve_when_it_is_gone() {
    for seed in [1, 2, 3] {
        // Hares to take: the wolf eats and does not starve.
        let mut game = meadow(seed);
        browse(&mut game);
        herd(&mut game, Species::Hare, 24);
        herd(&mut game, Species::Wolf, 1);
        let t = run(&mut game, 16);
        let starved = |t: &Tally| {
            t.died
                .iter()
                .filter(|c| *c == "starved, the game gone")
                .count()
        };
        assert!(
            t.died.iter().any(|c| c == "taken by a wolf"),
            "seed {seed}: the wolf took hares: {t:?}"
        );
        assert_eq!(
            starved(&t),
            0,
            "seed {seed}: the wolf did not starve: {t:?}"
        );
        // Control, no game at all: the wolves starve.
        let mut empty = meadow(seed);
        herd(&mut empty, Species::Wolf, 2);
        let t = run(&mut empty, 16);
        assert!(
            starved(&t) >= 1,
            "seed {seed}: wolves with no game starved: {t:?}"
        );
    }
}

#[test]
fn foxes_starve_when_their_hare_prey_is_gone() {
    // #604: foxes are hunters like wolves. This direct guard keeps the starvation rule from
    // silently exempting foxes while their breeding and hunting already depend on hare kills.
    let mut w = meadow(12);
    let id = w.spawn_agent(Species::Fox, TilePos::new(30, 30, 2));
    let i = w.agents.iter().position(|a| a.id == id).unwrap();
    let tick = w.meta.tick + 11 * TICKS_PER_DAY + 1;
    assert_eq!(
        hf_sim::ecology::starving(&w.agents[i], tick, Season::Spring),
        Some("starved, the game gone")
    );
}

#[test]
fn foxes_hunt_hares() {
    let mut w = meadow(4);
    browse(&mut w);
    herd(&mut w, Species::Hare, 16);
    herd(&mut w, Species::Fox, 2);
    let t = run(&mut w, 8);
    assert!(
        t.died.iter().any(|c| c == "taken by a fox"),
        "a fox took a hare: {t:?}"
    );
}

#[test]
fn hunting_the_grazers_lets_the_browse_recover() {
    // The same browse and herd; in one valley the herd is hunted out on the first day.
    let mut grazed = meadow(5);
    browse(&mut grazed);
    herd(&mut grazed, Species::Deer, 12);
    let mut hunted = meadow(5);
    browse(&mut hunted);
    for id in herd(&mut hunted, Species::Deer, 12) {
        let a = hunted.agents.iter_mut().find(|a| a.id == id).unwrap();
        a.alive = false;
        a.death = Some("hunted".into());
    }
    run(&mut grazed, 4);
    run(&mut hunted, 4);
    let (g, h) = (
        hf_sim::ecology::browse(&grazed),
        hf_sim::ecology::browse(&hunted),
    );
    assert!(
        h > g + 5.0,
        "the hunted valley's browse is thicker: {h:.1} against {g:.1}"
    );
}

#[test]
fn a_spent_carcass_greens_the_ground_about_it() {
    let mut w = meadow(6);
    let at = TilePos::new(30, 30, 2);
    w.plants
        .push(Plant::new(at.offset(1, 0, 0), PlantKind::Shrub, 0.2, 1));
    w.plants
        .push(Plant::new(at.offset(8, 0, 0), PlantKind::Shrub, 0.2, 2));
    // The carcass is picked over by the hour.
    w.meta.tick = TICKS_PER_DAY + 10 * TICKS_PER_HOUR - 1;
    w.creatures.carrion.push((at, w.meta.tick, false));
    w.meta.tick += 1;
    hf_sim::creatures::step(&mut w);
    assert!(w.creatures.carrion.is_empty());
    let near = w.plants[0].growth;
    let far = w.plants[1].growth;
    assert!(
        (near - (0.2 + hf_sim::ecology::CARCASS_GREENING)).abs() < 1e-5,
        "the shrub by the carcass greened: {near}"
    );
    assert!((far - 0.2).abs() < 1e-6, "the shrub far off did not: {far}");
}

#[test]
fn browse_springs_up_where_a_tree_was_felled() {
    let stumps: Vec<TilePos> = (0..6).map(|k| TilePos::new(12 + k * 7, 30, 2)).collect();
    let mut felled = meadow(7);
    for (k, s) in stumps.iter().enumerate() {
        felled
            .plants
            .push(Plant::new(*s, PlantKind::Stump, 1.0, k as u32));
    }
    // Control: the same meadow without the stumps.
    let mut open = meadow(7);
    run(&mut felled, 10);
    run(&mut open, 10);
    let shrubs = |w: &World| {
        w.plants
            .iter()
            .filter(|p| p.alive && p.kind == PlantKind::Shrub)
            .map(|p| p.pos)
            .collect::<Vec<_>>()
    };
    let sprung = shrubs(&felled);
    assert!(
        !sprung.is_empty(),
        "browse sprang up by a stump in a spring and summer"
    );
    assert!(
        sprung
            .iter()
            .all(|p| stumps.iter().any(|s| s.dist(*p) == 1)),
        "only beside a stump: {sprung:?}"
    );
    assert!(shrubs(&open).is_empty(), "nothing sprang up on open ground");
}

#[test]
fn a_tree_reaches_harvest_size_in_about_three_years() {
    // D95/#1739: tree maturity is tuned to Dwarf Fortress's about-three-years, so a sapling
    // should first reach `mature()` (growth 0.95) between 2.5 and 3.5 game years (50-70 days)
    // under the standard weather, not master's old ~3.5-year pace.
    let mut w = meadow(1);
    w.plants.push(Plant::new(
        TilePos::new(30, 30, 2),
        PlantKind::Tree,
        0.02,
        0,
    ));
    let mut first_mature_day = None;
    for day in 1..=70u64 {
        run(&mut w, 1);
        if w.plants[0].mature() {
            first_mature_day = Some(day);
            break;
        }
    }
    let day = first_mature_day.expect("the oak sapling never reached mature() within 70 days");
    assert!(
        (50..=70).contains(&day),
        "matured on day {day}, outside the 50-70 day (2.5-3.5 year) window"
    );
}

#[test]
fn a_kind_gone_from_the_valley_comes_back_when_there_is_food() {
    // Game and browse, no wolves: wolves come over the hills.
    let mut w = meadow(8);
    browse(&mut w);
    herd(&mut w, Species::Deer, 14);
    let t = run(&mut w, 12);
    assert!(
        t.came.contains(&Species::Wolf),
        "a wolf came over to the game: {:?}",
        t.came
    );
    // Control: no game, no browse, no carrion: nothing comes.
    let mut bare = meadow(8);
    let t = run(&mut bare, 12);
    assert!(
        t.came.is_empty(),
        "nothing came to a bare valley: {:?}",
        t.came
    );
}

#[test]
fn the_food_web_is_deterministic() {
    let go = || {
        let mut w = meadow(9);
        browse(&mut w);
        herd(&mut w, Species::Deer, 6);
        herd(&mut w, Species::Hare, 10);
        herd(&mut w, Species::Wolf, 2);
        herd(&mut w, Species::Fox, 1);
        run(&mut w, 6);
        hf_sim::digest::digest(&w)
    };
    assert_eq!(go(), go());
}

#[test]
fn a_hungry_grazer_goes_to_what_it_eats() {
    // A berry bush close by (grazed, but not browsed: `graze_bite` does not take from it) and
    // a shrub further off. Sent to the nearest grazed plant, the deer stood hungry by the bush
    // for good; it goes to the shrub and eats (FIXLOG 2026-11-02 #486).
    let mut w = meadow(10);
    let at = TilePos::new(30, 30, 2);
    w.plants
        .push(Plant::new(at.offset(2, 0, 0), PlantKind::BerryBush, 1.0, 1));
    w.plants
        .push(Plant::new(at.offset(-8, 0, 0), PlantKind::Shrub, 1.0, 2));
    let id = w.spawn_agent(Species::Deer, at);
    let i = w.agents.iter().position(|a| a.id == id).unwrap();
    w.agents[i].needs.hunger = 0.3;
    for _ in 0..TICKS_PER_DAY / 2 {
        w.step();
    }
    let i = w.agents.iter().position(|a| a.id == id).unwrap();
    assert!(w.plants[1].growth < 1.0, "the deer browsed the shrub");
    assert!(
        w.agents[i].needs.hunger > 0.5,
        "and is fed: {}",
        w.agents[i].needs.hunger
    );
}
