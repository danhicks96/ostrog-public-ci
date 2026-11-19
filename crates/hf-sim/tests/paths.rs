//! #84: ground walked often wears into a path, and grows back when left.

use hf_sim::ids::TilePos;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

#[test]
fn walking_counts_and_worn_ground_becomes_a_path_and_grows_back() {
    let mut w = World::generate(841, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(1);
    w.meta.autoplan = false;
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    // Steps counted.
    let a = w.grid.surface(40, 40);
    let b = w.grid.surface(41, 40);
    w.agents[i].pos = a;
    w.agents[i].path = vec![b];
    w.agents[i].move_total = 0;
    w.agents[i].move_ticks = 0;
    hf_sim::systems::movement::step(&mut w);
    assert_eq!(w.wear.get(&(41, 40)), Some(&1));
    // A well-trodden grass tile wears to packed earth.
    let g = TilePos::new(41, 40, w.grid.ground_z(41, 40));
    let mut t = w.grid.get(g);
    t.material = Material::GRASS;
    w.grid.set(g, t);
    w.wear.insert((41, 40), 200);
    w.meta.tick = TICKS_PER_DAY + 2 * TICKS_PER_HOUR + 77;
    hf_sim::paths::step(&mut w);
    assert_eq!(w.grid.get(g).material, Material::PACKED_EARTH);
    // Left alone, it grows back.
    for d in 2..60u64 {
        w.meta.tick = d * TICKS_PER_DAY + 2 * TICKS_PER_HOUR + 77;
        hf_sim::paths::step(&mut w);
    }
    assert_eq!(w.grid.get(g).material, Material::GRASS);
}

#[test]
fn heavily_walked_soil_compacts_and_can_regrow() {
    let mut w = World::generate(842, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    let g = TilePos::new(41, 40, w.grid.ground_z(41, 40));
    let mut t = w.grid.get(g);
    t.material = Material::SOIL;
    w.grid.set(g, t);
    w.wear.insert((g.x, g.y), 200);
    w.meta.tick = TICKS_PER_DAY + 2 * TICKS_PER_HOUR + 77;

    hf_sim::paths::step(&mut w);

    assert_eq!(w.grid.get(g).material, Material::PACKED_EARTH);
    assert_eq!(w.paths.get(&(g.x, g.y)), Some(&Material::SOIL));

    for d in 2..60u64 {
        w.meta.tick = d * TICKS_PER_DAY + 2 * TICKS_PER_HOUR + 77;
        hf_sim::paths::step(&mut w);
    }
    assert_eq!(w.grid.get(g).material, Material::SOIL);
}

#[test]
fn a_sapling_on_a_busy_path_dies_and_one_aside_lives() {
    // #1830: an oak sapling on a tile walked enough to be a third of the way to a path dies
    // at the next daily pass; one on a quiet tile lives on.
    let mut w = World::generate(843, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    let busy = TilePos::new(41, 40, w.grid.ground_z(41, 40));
    let quiet = TilePos::new(45, 40, w.grid.ground_z(45, 40));
    w.plants.push(Plant::new(busy, PlantKind::Tree, 0.1, 0));
    w.plants.push(Plant::new(quiet, PlantKind::Tree, 0.1, 1));
    w.wear.insert((busy.x, busy.y), 30);
    w.wear.insert((quiet.x, quiet.y), 5);
    for d in 1..=10u64 {
        w.meta.tick = d * TICKS_PER_DAY + 2 * TICKS_PER_HOUR + 77;
        hf_sim::paths::step(&mut w);
    }
    let alive_at = |w: &World, at: TilePos| {
        w.plants
            .iter()
            .find(|p| p.pos == at)
            .map(|p| p.alive)
            .unwrap_or(false)
    };
    assert!(!alive_at(&w, busy), "the sapling on the busy tile died");
    assert!(alive_at(&w, quiet), "the sapling on the quiet tile lived");
}
