//! P4 gates (docs/design-peoples.md section 11): each kind perceives with its own senses (sight
//! that the dark shortens unless their eyes are made for it, hearing through walls, a nose for
//! food), sleeps by its own cycle, feeds by its own diet, and feels heat and cold by its own
//! comfort.

use hf_sim::agents::Species;
use hf_sim::ids::TilePos;
use hf_sim::items::{ItemKind as K, ItemPlace};
use hf_sim::mind::knowledge::Known;
use hf_sim::species::{self, SpeciesId};
use hf_sim::time::{Calendar, TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material as M, Shape, Tile, TileFlags};
use hf_sim::World;

fn sp(key: &str) -> SpeciesId {
    species::by_key(key).unwrap()
}

fn flat(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.items.clear();
    w.agents.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    w
}

/// One person of `kind` at `at`, awake and idle.
fn person(w: &mut World, kind: SpeciesId, at: TilePos) -> usize {
    let id = w.spawn_agent(Species::Person, at);
    species::give(w, id, kind, None);
    w.agents.iter().position(|a| a.id == id).unwrap()
}

/// A minute of perception at `tick`.
fn look(w: &mut World, tick: u64) {
    for t in 0..20 {
        w.meta.tick = tick + t;
        hf_sim::systems::perceive::step(w);
    }
}

fn knows(w: &World, i: usize, k: Known) -> bool {
    w.agents[i].knowledge.get(k).is_some()
}

#[test]
fn the_dark_shortens_sight_unless_the_eyes_are_made_for_it() {
    let human = species::get(SpeciesId::HUMAN);
    let grisk = species::get(sp("grisk"));
    assert_eq!(human.sight_radius(1.0), 12);
    assert_eq!(human.sight_radius(0.0), 3);
    assert_eq!(
        grisk.sight_radius(0.0),
        12,
        "a Grisk sees in the dark as by day"
    );
    // In the game: food nine tiles off at night is seen by the Grisk, not the human.
    let mut w = flat(4201);
    let h = person(&mut w, SpeciesId::HUMAN, TilePos::new(20, 30, 2));
    let g = person(&mut w, sp("grisk"), TilePos::new(20, 32, 2));
    let a = w.create_item(
        K::Log,
        1,
        ItemPlace::Ground(TilePos::new(29, 30, 2)),
        0.5,
        "test",
    );
    let b = w.create_item(
        K::Log,
        1,
        ItemPlace::Ground(TilePos::new(29, 32, 2)),
        0.5,
        "test",
    );
    let night = 10 * TICKS_PER_DAY + 23 * TICKS_PER_HOUR;
    look(&mut w, night);
    assert!(
        !knows(&w, h, Known::Item(a)),
        "a human in the dark at nine tiles"
    );
    assert!(
        knows(&w, g, Known::Item(b)),
        "a Grisk in the dark at nine tiles"
    );
    // By day both see it.
    look(&mut w, 11 * TICKS_PER_DAY + 12 * TICKS_PER_HOUR);
    assert!(knows(&w, h, Known::Item(a)));
}

fn wall(w: &mut World, x: i32, y0: i32, y1: i32) {
    for y in y0..=y1 {
        for z in [2, 3] {
            w.grid.set(
                TilePos::new(x, y, z),
                Tile {
                    material: M::STONE_BLOCK,
                    shape: Shape::Wall,
                    flags: TileFlags::NONE,
                },
            );
        }
    }
    w.rebuild_caches();
}

#[test]
fn a_hungry_person_smells_the_cooking() {
    use hf_sim::save::{decode, encode, snapshot::Snapshot};
    let run = |threads| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                let mut w = flat(4206);
                wall(&mut w, 25, 20, 40);
                let hungry = person(&mut w, SpeciesId::HUMAN, TilePos::new(23, 28, 2));
                let fed = person(&mut w, SpeciesId::HUMAN, TilePos::new(23, 32, 2));
                let sleeping = person(&mut w, SpeciesId::HUMAN, TilePos::new(23, 30, 2));
                w.agents[hungry].needs.hunger = 0.3;
                w.agents[fed].needs.hunger = 0.8;
                w.agents[sleeping].needs.hunger = 0.3;
                w.agents[sleeping].activity = hf_sim::mind::Activity::Sleeping {
                    bed: None,
                    until: 20 * TICKS_PER_DAY,
                };
                let meal = w.create_item(
                    K::Meal,
                    1,
                    ItemPlace::Ground(TilePos::new(26, 28, 2)),
                    0.5,
                    "test",
                );
                let far = w.create_item(
                    K::Meal,
                    1,
                    ItemPlace::Ground(TilePos::new(28, 28, 2)),
                    0.5,
                    "test",
                );
                let raw = w.create_item(
                    K::RawMeat,
                    1,
                    ItemPlace::Ground(TilePos::new(26, 27, 2)),
                    0.5,
                    "test",
                );
                let below = w.create_item(
                    K::Meal,
                    1,
                    ItemPlace::Ground(TilePos::new(26, 28, 1)),
                    0.5,
                    "test",
                );
                assert!(!hf_sim::mind::perception::line_of_sight(
                    &w.grid,
                    w.agents[hungry].pos,
                    TilePos::new(26, 28, 2)
                ));
                look(&mut w, 10 * TICKS_PER_DAY + 12 * TICKS_PER_HOUR);
                assert!(
                    knows(&w, hungry, Known::Item(meal)),
                    "a hungry person smells the meal through the wall"
                );
                assert!(
                    !knows(&w, fed, Known::Item(meal)),
                    "being fed adds no cooking scent reach"
                );
                assert!(
                    !knows(&w, sleeping, Known::Item(meal)),
                    "a sleeping person does not look"
                );
                for item in [far, raw, below] {
                    assert!(
                        !knows(&w, hungry, Known::Item(item)),
                        "the cooking cue is bounded to nearby meals on this level"
                    );
                }
                let back = decode(&encode(&Snapshot::capture(&w)).unwrap())
                    .unwrap()
                    .restore();
                assert_eq!(hf_sim::digest::digest(&w), hf_sim::digest::digest(&back));
                hf_sim::digest::digest(&w)
            })
    };
    assert_eq!(run(1), run(4));
}

#[test]
fn hearing_and_smell_reach_through_walls() {
    let mut w = flat(4202);
    wall(&mut w, 25, 20, 40);
    let h = person(&mut w, SpeciesId::HUMAN, TilePos::new(22, 28, 2));
    let g = person(&mut w, sp("grisk"), TilePos::new(22, 32, 2));
    let s = person(&mut w, sp("screeling"), TilePos::new(22, 36, 2));
    // Someone awake behind the wall, within the Grisk's hearing (10) and nobody's sight.
    let other = person(&mut w, SpeciesId::HUMAN, TilePos::new(29, 32, 2));
    let oid = w.agents[other].id;
    // Meat behind the wall, within the Screeling's nose (6).
    let meat = w.create_item(
        K::RawMeat,
        2,
        ItemPlace::Ground(TilePos::new(27, 36, 2)),
        0.5,
        "test",
    );
    look(&mut w, 10 * TICKS_PER_DAY + 12 * TICKS_PER_HOUR);
    assert!(
        !knows(&w, h, Known::Agent(oid)),
        "a human does not see through stone"
    );
    assert!(knows(&w, g, Known::Agent(oid)), "a Grisk hears them");
    assert!(
        knows(&w, s, Known::Item(meat)),
        "a Screeling smells the meat"
    );
    assert!(!knows(&w, h, Known::Item(meat)));
}

/// OST-906: `SawFire` required only distance (6 tiles), so a fire behind a wall was "seen"
/// without sight. Burning tiles 4 tiles away: a wall between blocks the thought, the open
/// does not.
#[test]
fn a_fire_behind_a_wall_is_not_seen() {
    let mut w = flat(4205);
    wall(&mut w, 25, 20, 40);
    let blocked = person(&mut w, SpeciesId::HUMAN, TilePos::new(22, 30, 2));
    let open = person(&mut w, SpeciesId::HUMAN, TilePos::new(34, 30, 2));
    w.fires.push(hf_sim::systems::hazards::Fire {
        pos: TilePos::new(26, 30, 2),
        left: 1000,
        since: 0,
    });
    w.fires.push(hf_sim::systems::hazards::Fire {
        pos: TilePos::new(30, 30, 2),
        left: 1000,
        since: 0,
    });
    w.meta.tick = 0;
    hf_sim::systems::hazards::step(&mut w);
    let saw_fire = |w: &World, i: usize| {
        w.agents[i]
            .thoughts
            .list
            .iter()
            .any(|x| x.kind == hf_sim::mind::ThoughtKind::SawFire)
    };
    assert!(
        !saw_fire(&w, blocked),
        "a fire behind a wall, 4 tiles away, is not seen"
    );
    assert!(
        saw_fire(&w, open),
        "the same fire in the open, 4 tiles away, is seen"
    );
}

#[test]
fn a_nocturnal_people_sleeps_by_day() {
    let noon = 3 * TICKS_PER_DAY + 12 * TICKS_PER_HOUR;
    let midnight = 3 * TICKS_PER_DAY + 23 * TICKS_PER_HOUR;
    assert!(Calendar::rest_time(noon, true) && !Calendar::rest_time(noon, false));
    assert!(!Calendar::rest_time(midnight, true) && Calendar::rest_time(midnight, false));
    assert!(species::get(sp("grisk")).nocturnal());
    assert!(!species::get(SpeciesId::HUMAN).nocturnal());
}

#[test]
fn each_feeds_by_its_diet() {
    let human = species::get(SpeciesId::HUMAN);
    let screeling = species::get(sp("screeling"));
    assert_eq!(human.diet_factor(K::RawMeat), 1.0);
    assert!(screeling.diet_factor(K::RawMeat) > 1.0);
    assert!(screeling.diet_factor(K::Berries) < 1.0);
    // Through eating: the same meat fills a Screeling more than a human.
    let mut w = flat(4203);
    let h = person(&mut w, SpeciesId::HUMAN, TilePos::new(20, 30, 2));
    let s = person(&mut w, sp("screeling"), TilePos::new(24, 30, 2));
    let mut gain = Vec::new();
    for i in [h, s] {
        let at = w.agents[i].pos;
        let item = w.create_item(K::RawMeat, 1, ItemPlace::Ground(at), 0.5, "test");
        w.agents[i].needs.hunger = 0.2;
        w.agents[i].activity = hf_sim::mind::Activity::Eating {
            item,
            ticks_left: 1,
        };
        hf_sim::systems::act::step(&mut w);
        gain.push(w.agents[i].needs.hunger - 0.2);
    }
    assert!(gain[1] > gain[0] * 1.3, "{gain:?}");
}

#[test]
fn each_feels_heat_and_cold_by_its_comfort() {
    let mut w = flat(4204);
    let h = person(&mut w, SpeciesId::HUMAN, TilePos::new(20, 30, 2));
    let t = person(&mut w, sp("tharn"), TilePos::new(24, 30, 2));
    let s = person(&mut w, sp("screeling"), TilePos::new(28, 30, 2));
    let warmth_before: Vec<f32> = [h, t, s]
        .iter()
        .map(|i| w.agents[*i].needs.warmth)
        .collect();
    // 33 degrees at noon: the human (comfortable to 30 since the review of C38-C44) is hot,
    // the Tharn (to 36) is not.
    w.weather.temperature_c = 33.0;
    w.meta.tick = 20 * TICKS_PER_DAY + 12 * TICKS_PER_HOUR + 300;
    hf_sim::systems::hazards::step(&mut w);
    let hot = |w: &World, i: usize| {
        w.agents[i]
            .thoughts
            .list
            .iter()
            .any(|x| x.kind == hf_sim::mind::ThoughtKind::Hot)
    };
    assert!(hot(&w, h) && !hot(&w, t));
    // 10 degrees: the cold-blooded Screeling (comfortable from 18) loses warmth, the human not.
    w.weather.temperature_c = 10.0;
    w.meta.tick += TICKS_PER_HOUR;
    hf_sim::systems::hazards::step(&mut w);
    assert!(w.agents[s].needs.warmth < warmth_before[2]);
    assert!(w.agents[h].needs.warmth >= warmth_before[0]);
}
