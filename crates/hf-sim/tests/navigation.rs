//! Phase 2 gates: vertical paths via stairs, O(1) unreachable detection, incremental region
//! rebuild after a wall change, and path throughput.

#[path = "support/cpu.rs"]
mod cpu;

use std::time::Instant;

use hf_sim::ids::TilePos;
use hf_sim::nav::{find_path, AStarScratch, Nav, PathResult};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags, WorldGrid};
use hf_sim::World;

/// A flat 40×40 world with ground at z=0 (walk surface z=1).
fn flat() -> WorldGrid {
    WorldGrid::new(40, 40, -4, 4)
}

fn open(m: Material) -> Tile {
    Tile {
        material: m,
        shape: Shape::Open,
        flags: TileFlags::NONE,
    }
}

#[test]
fn vertical_path_via_stairs() {
    let mut g = flat();
    // Dig a room at z=-1 under (20..25, 20..25) with its floor on z=-2 rock.
    for y in 20..25 {
        for x in 20..25 {
            g.set(TilePos::new(x, y, -1), open(Material::AIR));
        }
    }
    // Stairs at (10,10): surface z=1 StairsDown, z=0 StairsBoth, z=-1 StairsUp inside a tunnel.
    g.set(
        TilePos::new(10, 10, 1),
        Tile {
            material: Material::STONE_BLOCK,
            shape: Shape::StairsDown,
            flags: TileFlags::NONE,
        },
    );
    g.set(
        TilePos::new(10, 10, 0),
        Tile {
            material: Material::GRANITE,
            shape: Shape::StairsBoth,
            flags: TileFlags::NONE,
        },
    );
    g.set(
        TilePos::new(10, 10, -1),
        Tile {
            material: Material::GRANITE,
            shape: Shape::StairsUp,
            flags: TileFlags::NONE,
        },
    );
    for x in 11..20 {
        g.set(TilePos::new(x, 10, -1), open(Material::AIR));
    }
    for y in 10..21 {
        g.set(TilePos::new(20, y, -1), open(Material::AIR));
    }
    let mut s = AStarScratch::new(&g);
    let start = TilePos::new(2, 2, 1);
    let goal = TilePos::new(22, 22, -1);
    match find_path(&g, &mut s, start, goal, 10_000) {
        PathResult::Found(p) => {
            assert_eq!(p[0], start);
            assert_eq!(*p.last().unwrap(), goal);
            assert!(
                p.contains(&TilePos::new(10, 10, 0)),
                "passes through the stairs"
            );
            for w in p.windows(2) {
                assert!(w[0].dist(w[1]) <= 4, "no teleports: {} -> {}", w[0], w[1]);
            }
        }
        other => panic!("expected a path, got {other:?}"),
    }
    let nav = Nav::build(&g);
    assert!(nav.reach(start, goal));
}

#[test]
fn unreachable_detected_o1() {
    let mut g = flat();
    // Wall off a 5×5 pen.
    for i in 10..=16 {
        g.set(TilePos::new(i, 10, 1), Tile::solid(Material::STONE_BLOCK));
        g.set(TilePos::new(i, 16, 1), Tile::solid(Material::STONE_BLOCK));
        g.set(TilePos::new(10, i, 1), Tile::solid(Material::STONE_BLOCK));
        g.set(TilePos::new(16, i, 1), Tile::solid(Material::STONE_BLOCK));
    }
    let nav = Nav::build(&g);
    let outside = TilePos::new(2, 2, 1);
    let inside = TilePos::new(13, 13, 1);
    assert!(!nav.reach(outside, inside));
    assert!(nav.reach(outside, TilePos::new(30, 30, 1)));
    // Many reach queries are cheap.
    // CPU time, not wall time: a busy machine is not a slow query (#582).
    let t = cpu::CpuTimer::start();
    let mut n = 0;
    for _ in 0..100_000 {
        if nav.reach(outside, inside) {
            n += 1;
        }
    }
    assert_eq!(n, 0);
    let us = t.report("100k reach queries").as_micros();
    assert!(us < 2_000_000, "100k reach queries took {us} us");
    // A* agrees and does not burn the budget on a walled pen.
    let mut s = AStarScratch::new(&g);
    assert_eq!(
        find_path(&g, &mut s, outside, inside, 10_000),
        PathResult::Unreachable
    );
}

#[test]
fn repeated_failed_agent_path_is_cached_until_navigation_changes() {
    let mut w = World::generate(5, GenParams::small());
    w.flatten(2, 2, 40, 40, 1);
    let start = TilePos::new(20, 20, 2);
    let goal = TilePos::new(30, 30, 2);
    let walls: Vec<_> = [(0, -1), (1, 0), (0, 1), (-1, 0)]
        .into_iter()
        .map(|(dx, dy)| goal.offset(dx, dy, 0))
        .collect();
    for p in &walls {
        w.grid.set(*p, Tile::solid(Material::STONE_BLOCK));
    }
    w.rebuild_caches();
    let id = w.spawn_agent(hf_sim::agents::Species::Person, start);
    let agent = w.agents.iter().position(|a| a.id == id).unwrap();

    assert!(!w.path_agent_to(agent, goal));
    assert_eq!(w.astar.as_ref().unwrap().failed_path_count(), 1);
    assert!(!w.path_agent_to(agent, goal));
    assert_eq!(w.astar.as_ref().unwrap().failed_path_count(), 1);

    for p in walls {
        w.grid.set(p, open(Material::AIR));
    }
    hf_sim::systems::nav::step(&mut w);
    assert_eq!(w.astar.as_ref().unwrap().failed_path_count(), 0);
    assert!(w.path_agent_to(agent, goal));
}

#[test]
fn region_rebuild_after_wall() {
    let mut g = flat();
    let mut nav = Nav::build(&g);
    let a = TilePos::new(5, 20, 1);
    let b = TilePos::new(35, 20, 1);
    assert!(nav.reach(a, b));
    // Build a wall across the whole map (crosses two chunks).
    for y in 0..40 {
        g.set(TilePos::new(20, y, 1), Tile::solid(Material::STONE_BLOCK));
    }
    let rebuilt = nav.update(&g);
    assert!(rebuilt > 0);
    assert!(!nav.reach(a, b), "wall separates the halves");
    // Open a door.
    let mut door = Tile::solid(Material::WOOD);
    door.flags = TileFlags(TileFlags::DOOR);
    g.set(TilePos::new(20, 20, 1), door);
    nav.update(&g);
    assert!(nav.reach(a, b), "door reconnects");
    // No-op update does nothing.
    assert_eq!(nav.update(&g), 0);
}

#[test]
fn path_throughput_on_generated_world() {
    let w = World::generate(5, GenParams::default());
    let g = &w.grid;
    let nav = Nav::build(g);
    let mut s = AStarScratch::new(g);
    let fs = w.meta.founding_site;
    let mut found = 0;
    let mut tried = 0;
    let t = Instant::now();
    for i in 0..2_000 {
        let x = (i * 37) % g.width;
        let y = (i * 91) % g.height;
        let goal = g.surface(x, y);
        if !g.walkable(goal) || !nav.reach(fs, goal) {
            continue;
        }
        tried += 1;
        if let PathResult::Found(_) = find_path(g, &mut s, fs, goal, 20_000) {
            found += 1;
        }
    }
    let ms = t.elapsed().as_secs_f64() * 1000.0;
    eprintln!(
        "paths tried {tried} found {found} in {ms:.1} ms ({:.3} ms/path), components {}",
        ms / tried.max(1) as f64,
        nav.component_count()
    );
    assert!(
        tried > 200,
        "most sampled surface tiles should be reachable from the founding site: {tried}"
    );
    assert_eq!(found, tried, "reach() and A* must agree");
}

/// A two-flight shaft whose middle tile is only an Up stair (as the cellar dig once left
/// it) must be walkable both ways: regions treat links as undirected, so A* must too.
#[test]
fn stairs_link_both_ways() {
    use hf_sim::ids::TilePos;
    use hf_sim::world::gen::GenParams;
    use hf_sim::world::{Material, Shape, Tile, TileFlags};
    use hf_sim::World;
    let mut w = World::generate(5, GenParams::small());
    w.flatten(2, 2, 40, 40, 1);
    w.rebuild_caches();
    let top = TilePos::new(20, 20, 2);
    let set = |w: &mut World, p: TilePos, shape: Shape, material: u16| {
        w.grid.set(
            p,
            Tile {
                material: Material(material),
                shape,
                flags: TileFlags::NONE,
            },
        )
    };
    let rock = w.grid.get(top.offset(0, 0, -1)).material.0;
    set(&mut w, top, Shape::StairsDown, rock);
    set(&mut w, top.offset(0, 0, -1), Shape::StairsUp, rock);
    set(&mut w, top.offset(0, 0, -2), Shape::StairsUp, rock);
    // A room at the bottom.
    for dx in 1..4 {
        set(&mut w, top.offset(dx, 0, -2), Shape::Floor, rock);
    }
    w.rebuild_caches();
    w.ensure_nav();
    let bottom = top.offset(3, 0, -2);
    let surface = TilePos::new(30, 30, 2);
    assert!(
        w.nav.as_ref().unwrap().reach(surface, bottom),
        "regions link the cellar"
    );
    let a = w.spawn_agent(hf_sim::agents::Species::Person, surface);
    let ia = w.agents.iter().position(|x| x.id == a).unwrap();
    assert!(w.path_agent_to(ia, bottom), "down into the cellar");
    w.agents[ia].pos = bottom;
    w.agents[ia].from = bottom;
    w.agents[ia].path.clear();
    assert!(w.path_agent_to(ia, surface), "and back up");
}

/// A wall that goes takes the footing of the tile on top of it: the region above a changed
/// chunk is recomputed too. (C34 soaks: food on a burned wall's top stayed "reachable", people
/// chose it every forty ticks and froze beside a reachable hearth.)
#[test]
fn a_wall_that_goes_takes_the_footing_above_it() {
    let mut g = flat();
    let wall = |m| Tile {
        material: m,
        shape: Shape::Wall,
        flags: TileFlags::NONE,
    };
    g.set(TilePos::new(10, 10, 1), wall(Material::WOOD));
    g.set(TilePos::new(11, 10, 1), wall(Material::WOOD));
    let mut nav = Nav::build(&g);
    let (a, b) = (TilePos::new(11, 10, 2), TilePos::new(10, 10, 2));
    assert!(nav.reach(a, b), "both wall tops are one footing");
    g.set(TilePos::new(10, 10, 1), open(Material::AIR));
    nav.update(&g);
    assert!(!g.walkable(b));
    assert!(!nav.reach(a, b), "nothing stands on air");
}

/// What lay on top of it falls to the footing below.
#[test]
fn things_on_a_wall_that_goes_fall() {
    let mut w = World::generate(5, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    let wall = Tile {
        material: Material::WOOD,
        shape: Shape::Wall,
        flags: TileFlags::NONE,
    };
    w.grid.set(TilePos::new(20, 20, 2), wall);
    w.rebuild_caches();
    w.ensure_nav();
    let top = TilePos::new(20, 20, 3);
    let id = w.create_item(
        hf_sim::items::ItemKind::Meal,
        1,
        hf_sim::items::ItemPlace::Ground(top),
        0.8,
        "test",
    );
    hf_sim::systems::nav::step(&mut w);
    w.grid.set(TilePos::new(20, 20, 2), open(Material::AIR));
    hf_sim::systems::nav::step(&mut w);
    let it = w.items.iter().find(|it| it.id == id).unwrap();
    assert_eq!(
        it.place,
        hf_sim::items::ItemPlace::Ground(TilePos::new(20, 20, 2)),
        "it fell to the ground"
    );
}
