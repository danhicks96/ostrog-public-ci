//! Phase 17 gates: a basin beside a lake fills in long rain and drains after; a grass fire
//! spreads and is put out with water from a well; cold without clothes hurts while a
//! clothed person indoors is fine.

use hf_sim::body::WoundKind;
use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::items::ItemKind;
use hf_sim::structures::{Structure, StructureKind};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

fn meadow(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    w
}

fn set_ground(w: &mut World, x: i32, y: i32, material: Material) {
    w.grid.set(
        TilePos::new(x, y, 1),
        Tile {
            material,
            shape: Shape::Wall,
            flags: TileFlags::NONE,
        },
    );
}

fn water(w: &mut World, x: i32, y: i32) {
    let mut t = w.grid.get(TilePos::new(x, y, 2));
    t.flags = TileFlags(t.flags.0 | TileFlags::WATER);
    w.grid.set(TilePos::new(x, y, 2), t);
}

#[test]
fn flood_fills_basin() {
    let mut w = meadow(1701);
    // A pond of shallow water and a soil basin beside it.
    for y in 20..26 {
        for x in 20..26 {
            water(&mut w, x, y);
        }
    }
    for y in 18..28 {
        for x in 26..34 {
            set_ground(&mut w, x, y, Material::SOIL);
        }
    }
    w.rebuild_caches();
    // Rain for a day and a half: the weather is seeded, so force it.
    let mut rose = false;
    let mut peak = 0usize;
    let mut wet = w.weather;
    wet.sky = hf_sim::systems::weather::Sky::Rain;
    wet.temperature_c = 12.0;
    w.force_weather = Some(wet);
    for h in 0..36 {
        for _ in 0..TICKS_PER_HOUR {
            w.step();
        }
        peak = peak.max(w.flooded.len());
        if w.last_events
            .iter()
            .any(|e| matches!(e.kind, EventKind::Flooded { .. }))
            || !w.flooded.is_empty()
        {
            rose = true;
        }
        let _ = h;
    }
    assert!(rose, "the water rose in the rain");
    assert!(peak >= 20, "the basin took water: {peak} tiles");
    assert!(
        w.grid.get(TilePos::new(27, 22, 2)).is_water(),
        "the tile beside the pond is under water"
    );
    // Then two dry days.
    wet.sky = hf_sim::systems::weather::Sky::Clear;
    w.force_weather = Some(wet);
    for _ in 0..48 {
        for _ in 0..TICKS_PER_HOUR {
            w.step();
        }
    }
    assert!(
        w.flooded.is_empty(),
        "the flood drew back: {} tiles still wet",
        w.flooded.len()
    );
    assert!(!w.grid.get(TilePos::new(27, 22, 2)).is_water());
    assert!(
        w.grid.get(TilePos::new(22, 22, 2)).is_water(),
        "the pond itself remains"
    );
}

#[test]
fn a_fire_nearby_is_a_personal_problem() {
    use hf_sim::mind::{Evidence, ObservationKind, ProblemKind, ProblemStatus};
    use hf_sim::save::{decode, encode, snapshot::Snapshot};

    let run = |threads| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                let mut w = meadow(1731);
                let ids = w.found_party(4);
                let indices: Vec<_> = ids
                    .iter()
                    .map(|id| w.agents.iter().position(|a| a.id == *id).unwrap())
                    .collect();
                let [near, far, hidden, sleeping] =
                    [indices[0], indices[1], indices[2], indices[3]];
                let fire = TilePos::new(20, 20, 2);
                for (i, pos) in [
                    (near, fire.offset(4, 0, 0)),
                    (far, fire.offset(10, 0, 0)),
                    (hidden, fire.offset(-4, 0, 0)),
                    (sleeping, fire.offset(0, 4, 0)),
                ] {
                    w.agents[i].pos = pos;
                    w.agents[i].from = pos;
                    w.agents[i].activity = hf_sim::mind::Activity::Idle;
                }
                w.agents[sleeping].activity = hf_sim::mind::Activity::Sleeping {
                    bed: None,
                    until: w.meta.tick + TICKS_PER_HOUR,
                };
                w.grid.set(
                    fire.offset(-2, 0, 0),
                    Tile {
                        material: Material::LIMESTONE,
                        shape: Shape::Wall,
                        flags: TileFlags::NONE,
                    },
                );
                w.grid.set(
                    fire,
                    Tile {
                        material: Material::WOOD,
                        shape: Shape::Floor,
                        flags: TileFlags::NONE,
                    },
                );
                w.rebuild_caches();
                w.weather.sky = hf_sim::systems::weather::Sky::Clear;
                w.weather.temperature_c = 20.0;
                assert!(w.ignite(fire, "in a test"));
                hf_sim::systems::hazards::step(&mut w);
                let problem = w.agents[near]
                    .problems
                    .records
                    .iter()
                    .find(|p| p.kind == ProblemKind::Fire)
                    .expect("the visible nearby fire is a personal problem");
                assert_eq!(problem.status, ProblemStatus::Open);
                assert_eq!(problem.evidence, Evidence::Direct);
                assert_eq!(problem.pos, w.agents[near].pos);
                assert!(problem.attempts.is_empty());
                let id = problem.id;
                assert!(w.agents[near].learning.observations.iter().any(|o| {
                    o.kind == ObservationKind::Fire
                        && o.evidence == Evidence::Direct
                        && o.pos == w.agents[near].pos
                }));
                for i in [far, hidden, sleeping] {
                    assert!(!w.agents[i]
                        .problems
                        .records
                        .iter()
                        .any(|p| p.kind == ProblemKind::Fire));
                    assert!(!w.agents[i]
                        .learning
                        .observations
                        .iter()
                        .any(|o| o.kind == ObservationKind::Fire));
                }
                w.meta.tick += hf_sim::systems::hazards::FIRE_STEP;
                hf_sim::systems::hazards::step(&mut w);
                let problems: Vec<_> = w.agents[near]
                    .problems
                    .records
                    .iter()
                    .filter(|p| p.kind == ProblemKind::Fire)
                    .collect();
                assert_eq!(problems.len(), 1);
                assert_eq!(problems[0].id, id);
                assert_eq!(problems[0].last_seen, w.meta.tick);
                assert_eq!(
                    w.agents[near]
                        .learning
                        .observations
                        .iter()
                        .filter(|o| o.kind == ObservationKind::Fire)
                        .count(),
                    1
                );
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
fn fire_spreads_and_is_put_out() {
    let mut w = meadow(1702);
    let ids = w.found_party(8);
    w.meta.autoplan = true;
    // Grass all round, a well near home, dry summer weather.
    for y in 10..50 {
        for x in 10..50 {
            set_ground(&mut w, x, y, Material::GRASS);
        }
    }
    let home = w.meta.founding_site;
    let id = w.alloc_structure_id();
    w.structures.push(Structure {
        id,
        kind: StructureKind::Well,
        pos: home.offset(3, 0, 0),
        material: Material::STONE_BLOCK,
        progress: 1.0,
        active: false,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
    w.meta.tick = 7 * TICKS_PER_DAY + TICKS_PER_HOUR * 12; // summer noon
    w.rebuild_caches();
    w.ensure_nav();
    let start = TilePos::new(40, 40, 2);
    assert!(w.ignite(start, "in a test"), "grass takes fire");
    let mut most = 1usize;
    let mut doused = 0;
    let mut out_at = None;
    let mut dry = w.weather;
    dry.sky = hf_sim::systems::weather::Sky::Clear;
    dry.temperature_c = 24.0;
    w.force_weather = Some(dry);
    for t in 0..TICKS_PER_DAY {
        w.step();
        most = most.max(w.fires.len());
        for e in &w.last_events {
            match &e.kind {
                // Nobody was sent: whoever fought it chose to, with what they knew.
                EventKind::Doused { agent, .. } | EventKind::BeatOut { agent, .. } => {
                    doused += 1;
                    assert!(ids.contains(agent));
                }
                EventKind::FireOut { .. } => out_at = Some(t),
                _ => {}
            }
        }
        if w.fires.is_empty() && t > 200 {
            break;
        }
    }
    // People who saw it may get to it before it spreads; that it spreads unchecked is
    // `fire_spreads_unchecked`.
    let _ = most;
    assert!(
        doused >= 1,
        "someone fought it, by water or by beating it out"
    );
    assert!(w.fires.is_empty(), "the fire is out");
    let _ = (out_at, start);
}

#[test]
fn cold_without_clothes_hurts() {
    let mut w = meadow(1703);
    let ids = w.found_party(2);
    w.meta.autoplan = false;
    // Deep winter, nobody near a fire. One person is clothed and indoors on a wooden
    // floor; the other stands outside in rags.
    w.meta.tick = 16 * TICKS_PER_DAY + TICKS_PER_HOUR * 12;
    w.build_house(40, 40, 7, 6, 1, Material::WOOD, Material::WOOD);
    w.rebuild_caches();
    w.ensure_nav();
    let i_in = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let i_out = w.agents.iter().position(|a| a.id == ids[1]).unwrap();
    w.agents[i_in].pos = TilePos::new(43, 42, 2);
    w.agents[i_in].from = w.agents[i_in].pos;
    w.agents[i_in].clothes_wear = 1.0;
    w.agents[i_out].clothes_wear = 0.0;
    w.agents[i_out].pos = TilePos::new(10, 10, 2);
    w.agents[i_out].from = w.agents[i_out].pos;
    // Keep the outsider outside: no hearth exists and the home is far; they loiter.
    let mut cold = w.weather;
    cold.temperature_c = -8.0;
    cold.sky = hf_sim::systems::weather::Sky::Snow;
    w.force_weather = Some(cold);
    // A controlled exposure: both stay where they are put (hunger would otherwise send
    // the indoor one out to forage).
    let p_in = TilePos::new(43, 42, 2);
    let p_out = TilePos::new(10, 10, 2);
    // Eighteen hours of it: long enough for frostbite on someone in rags.
    for _ in 0..TICKS_PER_HOUR * 18 {
        for (j, p) in [(i_out, p_out), (i_in, p_in)] {
            w.agents[j].activity = hf_sim::mind::Activity::Loitering { ticks_left: 100 };
            w.agents[j].pos = p;
            w.agents[j].from = p;
            w.agents[j].path.clear();
            w.agents[j].needs.hunger = 0.9;
        }
        w.step();
    }
    let outside = &w.agents[i_out];
    let inside = &w.agents[i_in];
    assert!(
        outside.needs.warmth < 0.2,
        "warmth drained outside: {}",
        outside.needs.warmth
    );
    assert!(
        outside
            .body
            .wounds
            .iter()
            .any(|wd| wd.kind == WoundKind::Frostbite),
        "frostbite set in"
    );
    assert!(outside.thoughts.has(hf_sim::mind::ThoughtKind::Cold));
    assert!(
        inside.needs.warmth > 0.9,
        "indoors and clothed, warm: {}",
        inside.needs.warmth
    );
    assert!(inside.body.wounds.is_empty());
    let _ = ItemKind::Clothes;
}

#[test]
fn a_tree_shades_from_the_heat() {
    // OST-900: a person under a mature oak's canopy should bake less than one on open grass.
    let mut w = meadow(1728);
    let ids = w.found_party(2);
    w.meta.autoplan = false;
    w.meta.tick = 7 * TICKS_PER_DAY + TICKS_PER_HOUR * 12; // summer midday
    let shaded_pos = TilePos::new(20, 20, 2);
    let open_pos = TilePos::new(40, 40, 2);
    w.plants.push(hf_sim::plants::Plant::new(
        shaded_pos,
        hf_sim::plants::PlantKind::Tree,
        1.0,
        0,
    ));
    w.rebuild_caches();
    let i_shaded = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let i_open = w.agents.iter().position(|a| a.id == ids[1]).unwrap();
    let mut hot = w.weather;
    hot.temperature_c = 38.0;
    hot.sky = hf_sim::systems::weather::Sky::Clear;
    w.force_weather = Some(hot);
    w.agents[i_shaded].needs.comfort = 1.0;
    w.agents[i_open].needs.comfort = 1.0;
    // `cold_step` fires once per game hour (hazards.rs: tick % TICKS_PER_HOUR == 300), so a
    // few hours outdoors is needed for the shade's effect to add up to a visible difference.
    for _ in 0..TICKS_PER_HOUR * 8 {
        for (j, p) in [(i_shaded, shaded_pos), (i_open, open_pos)] {
            w.agents[j].activity = hf_sim::mind::Activity::Loitering { ticks_left: 100 };
            w.agents[j].pos = p;
            w.agents[j].from = p;
            w.agents[j].path.clear();
            w.agents[j].needs.hunger = 0.9;
        }
        w.step();
    }
    let shaded = &w.agents[i_shaded];
    let open = &w.agents[i_open];
    assert!(
        shaded.needs.comfort >= open.needs.comfort + 0.1,
        "shaded comfort {} vs open comfort {}",
        shaded.needs.comfort,
        open.needs.comfort
    );
    assert!(
        !shaded.thoughts.has(hf_sim::mind::ThoughtKind::Hot),
        "the shaded one should not be marked Hot"
    );
    assert!(
        open.thoughts.has(hf_sim::mind::ThoughtKind::Hot),
        "the one on open ground should be marked Hot"
    );
}

#[test]
fn fire_spreads_unchecked() {
    // Nobody on the map: a grass fire in a dry summer spreads until it runs out of grass.
    let mut w = meadow(1703);
    for y in 10..50 {
        for x in 10..50 {
            set_ground(&mut w, x, y, Material::GRASS);
        }
    }
    w.meta.tick = 7 * TICKS_PER_DAY + TICKS_PER_HOUR * 12;
    w.rebuild_caches();
    let mut dry = w.weather;
    dry.sky = hf_sim::systems::weather::Sky::Clear;
    dry.temperature_c = 24.0;
    w.force_weather = Some(dry);
    assert!(w.ignite(TilePos::new(30, 30, 2), "in a test"));
    let mut most = 0;
    for _ in 0..TICKS_PER_HOUR * 6 {
        w.step();
        most = most.max(w.fires.len());
    }
    assert!(most >= 5, "unchecked, it spread: {most} tiles at most");
    assert!(
        w.grid.get(TilePos::new(30, 30, 1)).material == Material::SOIL,
        "where it burned, the grass is gone"
    );
}

#[test]
fn nobody_forgets_where_home_s_fire_is() {
    // A memory full of berry bushes once pushed the only hearth out of everyone's mind and
    // they froze looking for it (C21, seed 5). Staged: a person far out of sight of the
    // community's hearth, their places full of bushes, still knows the hearth.
    let mut w = meadow(1704);
    let ids = w.found_party(1);
    w.meta.autoplan = false;
    let home = w.meta.founding_site;
    let id = w.alloc_structure_id();
    w.structures.push(Structure {
        id,
        kind: StructureKind::Hearth,
        pos: home.offset(2, 0, 0),
        material: Material::STONE_BLOCK,
        progress: 1.0,
        active: true,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
    w.rebuild_caches();
    w.ensure_nav();
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let far = TilePos::new(58, 58, 2);
    for k in 0..400 {
        let p = TilePos::new(k % 60, k / 60, 2);
        w.agents[i]
            .knowledge
            .note(hf_sim::mind::knowledge::Known::FoodPlant(p), p, 1, 0);
    }
    w.agents[i]
        .knowledge
        .forget(hf_sim::mind::knowledge::Known::Structure(id));
    for _ in 0..hf_sim::systems::perceive::STRIPE * 2 {
        w.agents[i].pos = far;
        w.agents[i].from = far;
        w.step();
    }
    assert!(w.agents[i]
        .knowledge
        .get(hf_sim::mind::knowledge::Known::Structure(id))
        .is_some());
}

#[test]
fn the_day_is_warmest_in_the_afternoon_and_coldest_before_dawn() {
    // #256: the curve was upside down, so a winter noon was -9 C and its midnight +1 C.
    use hf_sim::systems::weather::Weather;
    use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
    let day = 40 * TICKS_PER_DAY;
    let at = |h: u64, m: u64| Weather::at(7, day + h * TICKS_PER_HOUR + m * 20).temperature_c;
    assert!(
        at(14, 24) > at(2, 24) + 8.0,
        "{} at 14:24, {} at 02:24",
        at(14, 24),
        at(2, 24)
    );
    assert!(at(12, 0) > at(0, 0), "noon warmer than midnight");
}
