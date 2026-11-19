//! Movement follows planned paths at docs/TIME.md speeds and recovers when a tile closes.

use hf_sim::agents::Species;
use hf_sim::ids::TilePos;
use hf_sim::time::WALK_TICKS_PER_TILE;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Tile};
use hf_sim::World;

fn flat_world() -> World {
    let mut w = World::generate(1, GenParams::small());
    w.flatten(0, 0, 64, 64, 1);
    w.plants.clear();
    w
}

#[test]
fn walks_a_straight_path_at_walk_speed() {
    let mut w = flat_world();
    let start = TilePos::new(5, 5, 2);
    let id = w.spawn_agent(Species::Person, start);
    let idx = w.agents.iter().position(|a| a.id == id).unwrap();
    w.agents[idx].idle_ticks = u32::MAX; // keep the wander placeholder out of it
    assert!(w.path_agent_to(idx, TilePos::new(25, 5, 2)));
    assert_eq!(w.agents[idx].path.len(), 20);
    let t0 = w.tick();
    while !w.agents[idx].path.is_empty() || w.agents[idx].is_moving() {
        w.step();
        assert!(w.tick() - t0 < 1_000, "did not arrive");
    }
    assert_eq!(w.agents[idx].pos, TilePos::new(25, 5, 2));
    let ticks = w.tick() - t0;
    assert_eq!(
        ticks,
        20 * WALK_TICKS_PER_TILE as u64,
        "20 tiles at 4 ticks each"
    );
    assert_eq!(w.agents[idx].facing, 2, "facing east");
}

#[test]
fn blocked_tile_stops_the_path_and_flags_it() {
    let mut w = flat_world();
    let id = w.spawn_agent(Species::Person, TilePos::new(5, 10, 2));
    let idx = w.agents.iter().position(|a| a.id == id).unwrap();
    w.agents[idx].idle_ticks = u32::MAX;
    assert!(w.path_agent_to(idx, TilePos::new(15, 10, 2)));
    w.run(8); // two tiles in
    w.grid
        .set(TilePos::new(10, 10, 2), Tile::solid(Material::STONE_BLOCK));
    w.run(60);
    let a = &w.agents[idx];
    assert!(a.path_blocked, "blocked flag set");
    assert!(a.path.is_empty());
    assert!(a.pos.x < 10, "stopped before the wall at {}", a.pos);
    // Regions caught up: the wall tile is now unreachable, its neighbours still are.
    assert!(!w
        .nav
        .as_ref()
        .unwrap()
        .reach(a.pos, TilePos::new(10, 10, 2)));
    assert!(w
        .nav
        .as_ref()
        .unwrap()
        .reach(a.pos, TilePos::new(15, 10, 2)));
}
