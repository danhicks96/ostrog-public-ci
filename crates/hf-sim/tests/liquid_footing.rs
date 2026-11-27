//! Natural shallow water is a ford on its own level. The level over it is air.
use hf_sim::body::{Part, WoundKind};
use hf_sim::ids::{ItemId, TilePos};
use hf_sim::items::{Item, ItemKind, ItemPlace};
use hf_sim::mind::Activity;
use hf_sim::systems::{health, movement, nav};
use hf_sim::time::LEVEL_CHANGE_TICKS;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

fn liquid(deep: bool) -> Tile {
    let flags = if deep {
        TileFlags::WATER | TileFlags::DEEP
    } else {
        TileFlags::WATER
    };
    Tile {
        material: Material::WATER,
        shape: Shape::Floor,
        flags: TileFlags(flags),
    }
}

fn pond() -> (World, TilePos) {
    let mut w = World::generate(758, GenParams::small());
    w.flatten(4, 4, 56, 56, 1);
    let p = TilePos::new(24, 24, 2);
    w.meta.founding_site = p.offset(4, 4, 0);
    w.found_party(1);
    for dz in -2..=0 {
        w.grid.set(p.offset(0, 0, dz), liquid(true));
    }
    w.grid
        .set(p.offset(0, 0, -3), Tile::solid(Material::GRANITE));
    (w, p)
}

#[test]
fn ford_deck_and_diagonal_use_real_support_through_thaw_and_drying() {
    let (mut w, p) = pond();
    let air = p.offset(0, 0, 1);
    let start = air.offset(-1, 0, 0);
    let across = air.offset(0, 1, 0);
    for q in [start, across, start.offset(0, 1, 0)] {
        w.grid.set(q, Tile::floor(Material::WOOD));
    }
    let mut steps = Vec::new();
    for temperature in [-25., 24.] {
        w.weather.temperature_c = temperature;
        assert_eq!(hf_sim::water::frozen_at(&w, p), temperature < 0.);
        for deep in [false, true] {
            w.grid.set(p, liquid(deep));
            assert!(
                !w.grid.walkable(p),
                "a water voxel below cannot hold a walker"
            );
            w.grid
                .set(p.offset(0, 0, -1), Tile::solid(Material::GRANITE));
            assert_eq!(w.grid.walkable(p), !deep, "one layer over a real bed");
            w.grid.set(p.offset(0, 0, -1), liquid(true));
            assert!(!w.grid.walkable(air));
            hf_sim::nav::neighbours(&w.grid, start, &mut steps);
            assert!(steps.iter().all(|edge| edge.to != air && edge.to != across));
        }
    }
    // Replay a previously accepted diagonal after either flank has lost support.
    let i = w.agents.iter().position(|a| a.community.is_some()).unwrap();
    let a = &mut w.agents[i];
    a.pos = start;
    a.from = start;
    a.path = vec![across];
    movement::step(&mut w);
    assert_eq!(w.agents[i].pos, start);
    assert!(w.agents[i].path_blocked && w.agents[i].path.is_empty());
    for shape in [
        Shape::Floor,
        Shape::StairsBoth,
        Shape::RampUp,
        Shape::RampTop,
        Shape::Ladder,
    ] {
        w.grid.set(
            air,
            Tile {
                shape,
                ..Tile::floor(Material::WOOD)
            },
        );
        assert!(w.grid.walkable(air));
    }
    w.grid.set(air, Tile::AIR);
    for flags in [TileFlags::NONE, TileFlags(TileFlags::WATER)] {
        w.grid.set(
            p,
            Tile {
                flags,
                ..Tile::floor(Material::WOOD)
            },
        );
        assert!(w.grid.walkable(air));
        hf_sim::nav::neighbours(&w.grid, start, &mut steps);
        assert!(steps.iter().any(|edge| edge.to == across));
    }
    w.grid.set(p, Tile::floor(Material::MUD));
    assert!(w.grid.walkable(air));
    w.grid.set(p, liquid(false));
    assert!(!w.grid.walkable(air));
    assert!(hf_sim::nav::step_cost(&w.grid, p, false) >= hf_sim::time::WALK_TICKS_PER_TILE + 4);
}

#[test]
fn legacy_stride_settles_at_elapsed_cost_and_only_mobile_people_can_take_a_foothold() {
    let (w, p) = pond();
    let cases = ["awake", "sleeping", "hurt", "held", "enclosed", "stacked"];
    for (surface_deep, scenario) in [false, true]
        .into_iter()
        .flat_map(|deep| cases.map(|case| (deep, case)))
    {
        let mut w = w.clone();
        let i = w.agents.iter().position(|a| a.community.is_some()).unwrap();
        w.grid.set(p, liquid(surface_deep));
        let above = p.offset(0, 0, 1);
        let north = p.offset(0, -1, 0);
        for (x, y) in [(0, -1), (1, 0), (-1, 0), (0, 1)] {
            w.grid
                .set(p.offset(x, y, 0), Tile::solid(Material::GRANITE));
        }
        if scenario != "enclosed" {
            w.grid.set(north, liquid(false));
        }
        if scenario == "stacked" {
            w.grid.set(north.offset(0, 0, -1), liquid(false));
            assert!(!w.grid.walkable(north));
        }
        let a = &mut w.agents[i];
        a.from = above.offset(1, 0, 0);
        a.pos = above;
        a.path = vec![above.offset(0, 1, 0)];
        a.move_ticks = 1;
        a.move_total = 4;
        if scenario == "sleeping" {
            a.activity = Activity::Sleeping {
                bed: None,
                until: u64::MAX,
            };
        }
        if scenario == "held" {
            a.held = Some((a.community.unwrap(), 1, 0));
        }
        if scenario == "hurt" {
            health::wound(
                &mut w,
                i,
                Part::LEG_L,
                WoundKind::Break,
                0.9,
                0.,
                "test",
                None,
            );
            assert!(w.agents[i].body.needs_care());
        }
        movement::step(&mut w);
        let a = &w.agents[i];
        assert_eq!(
            (a.from, a.pos, a.move_ticks, a.move_total),
            (above, p, 1, LEVEL_CHANGE_TICKS)
        );
        assert!(a.path.is_empty() && a.path_blocked);
        for _ in 1..LEVEL_CHANGE_TICKS {
            movement::step(&mut w);
            assert_eq!(w.agents[i].pos, p);
        }
        movement::step(&mut w);
        if scenario == "awake" || scenario == "hurt" {
            let a = &w.agents[i];
            assert_eq!((a.from, a.pos), (p, north));
            let (problem_id, attempt) = a.problems.pending_any().unwrap();
            assert_eq!(attempt.target, Some(north));
            assert_eq!(a.problems.get(problem_id).unwrap().pos, p);
            assert_eq!(
                a.problems.get(problem_id).unwrap().kind,
                hf_sim::mind::ProblemKind::WaterHazard
            );
            assert!(matches!(a.activity, Activity::Going {to,..} if to==north));
            assert_eq!(
                a.move_total,
                hf_sim::nav::step_cost(&w.grid, north, false)
                    .max(hf_sim::nav::step_cost(&w.grid, p, false))
                    * if scenario == "hurt" { 2 } else { 1 }
            );
            hf_sim::systems::hazards::step(&mut w);
            let completed = w.agents[i]
                .problems
                .get(problem_id)
                .unwrap()
                .attempts
                .last()
                .unwrap();
            assert!(completed.success);
            assert_eq!(completed.actual, Some(hf_sim::mind::OutcomeKind::Safety));
        } else {
            assert!(w.agents[i].problems.pending_any().is_none());
            for _ in 0..120 {
                movement::step(&mut w);
            }
            assert_eq!(w.agents[i].pos, p);
            assert!(!w.grid.over_head(p));
        }
    }
}

#[test]
fn diagonal_bank_requires_two_open_flanks_and_an_actual_bed() {
    let (w, p) = pond();
    let north = p.offset(0, -1, 0);
    let west = p.offset(-1, 0, 0);
    let bank = p.offset(-1, -1, 0);
    let cases = ["none", "wall", "flooded deck", "unsupported bank"];
    for (surface_deep, barrier) in [false, true]
        .into_iter()
        .flat_map(|deep| cases.map(|case| (deep, case)))
    {
        let mut w = w.clone();
        w.grid.set(p, liquid(surface_deep));
        for dx in -1..=1 {
            for dy in -1..=1 {
                if dx != 0 || dy != 0 {
                    w.grid
                        .set(p.offset(dx, dy, 0), Tile::solid(Material::GRANITE));
                }
            }
        }
        for flank in [north, west] {
            w.grid.set(flank, liquid(false));
            w.grid.set(flank.offset(0, 0, -1), liquid(false));
        }
        w.grid.set(bank, liquid(false));
        match barrier {
            "wall" => w.grid.set(west, Tile::solid(Material::GRANITE)),
            "flooded deck" => w.grid.set(
                west,
                Tile {
                    flags: TileFlags(TileFlags::WATER | TileFlags::DEEP),
                    ..Tile::floor(Material::WOOD)
                },
            ),
            "unsupported bank" => w.grid.set(bank.offset(0, 0, -1), liquid(false)),
            _ => false,
        };
        let i = w.agents.iter().position(|a| a.community.is_some()).unwrap();
        w.agents[i].from = p;
        w.agents[i].pos = p;
        movement::step(&mut w);
        if barrier == "none" {
            assert_eq!((w.agents[i].from, w.agents[i].pos), (p, bank));
            assert_eq!(
                w.agents[i].move_total,
                hf_sim::nav::step_cost(&w.grid, p, true)
                    .max(hf_sim::nav::step_cost(&w.grid, bank, true))
            );
        } else {
            assert_eq!(w.agents[i].pos, p, "{barrier}");
        }
    }
}

#[test]
fn loose_stack_preserves_its_story_and_stops_above_a_sealed_deck() {
    let (mut w, p) = pond();
    let above = p.offset(0, 0, 1);
    w.grid.set(p.offset(0, 0, -2), Tile::floor(Material::WOOD));
    w.grid.set(p.offset(0, 0, -3), liquid(true));
    let id = ItemId(90000);
    let story = hf_sim::legend::Story {
        maker_name: "Ansel".into(),
        uses: 32,
        ..Default::default()
    };
    w.items.push(Item {
        id,
        kind: ItemKind::Tool,
        count: 3,
        place: ItemPlace::Ground(above),
        quality: 0.8,
        material: Material::WOOD,
        story: Some(Box::new(story.clone())),
        rot: 0.,
    });
    w.rebuild_caches();
    w.ensure_nav();
    nav::step(&mut w);
    nav::step(&mut w);
    let item = w.items.iter().find(|item| item.id == id).unwrap();
    assert_eq!(item.place, ItemPlace::Ground(p.offset(0, 0, -1)));
    assert_eq!(item.count, 3);
    assert_eq!(item.story.as_deref(), Some(&story));
}

#[test]
fn a_new_stack_between_grid_edits_settles_identically_after_reopening() {
    let (mut w, p) = pond();
    nav::step(&mut w);
    w.grid.set(p, liquid(false));
    let id = w.create_item(ItemKind::Berries, 3, ItemPlace::Ground(p), 0.5, "harvest");
    let bytes = hf_sim::save::encode(&hf_sim::save::snapshot::Snapshot::capture(&w)).unwrap();
    let mut cold = hf_sim::save::decode(&bytes).unwrap().restore();
    assert_eq!(hf_sim::digest::digest(&w), hf_sim::digest::digest(&cold));
    for state in [&mut w, &mut cold] {
        nav::step(state);
        let it = state.items.iter().find(|item| item.id == id).unwrap();
        assert_eq!(it.place, ItemPlace::Ground(p.offset(0, 0, -2)));
        assert_eq!(it.count, 3);
    }
    assert_eq!(hf_sim::digest::digest(&w), hf_sim::digest::digest(&cold));
}

#[test]
fn overhead_water_refreshes_submerged_escape_across_a_chunk_boundary() {
    let (mut w, _) = pond();
    let x = hf_sim::world::CHUNK_SIZE as i32;
    let q = TilePos::new(x, 24, 2);
    let p = q.offset(-1, 0, 0);
    let bank = q.offset(1, 0, 0);
    for cell in [p, q] {
        w.grid.set(cell, liquid(false));
        w.grid.set(cell.offset(0, 0, -1), liquid(false));
        w.grid.set(cell.offset(0, 0, 1), liquid(false));
    }
    nav::step(&mut w);
    let i = w.agents.iter().position(|a| a.community.is_some()).unwrap();
    let id = w.agents[i].id;
    w.agents[i].pos = p;
    w.agents[i].from = p;
    w.agents[i].path = vec![bank, q];
    w.agents[i].activity = Activity::Going {
        to: bank,
        then: Box::new(Activity::Idle),
    };
    w.meta.tick = 4;
    let bytes = hf_sim::save::encode(&hf_sim::save::snapshot::Snapshot::capture(&w)).unwrap();
    let mut reopened = hf_sim::save::decode(&bytes).unwrap().restore();
    for world in [&mut w, &mut reopened] {
        nav::step(world);
        assert!(world.nav.as_ref().unwrap().reach(p, bank));
        assert!(world.grid.over_head(q));
        movement::step(world);
        let cost = hf_sim::nav::step_cost(&world.grid, q, false);
        let mut edges = Vec::new();
        hf_sim::nav::neighbours(&world.grid, p, &mut edges);
        assert_eq!(
            edges.iter().find(|e| e.to == q).unwrap().cost,
            cost + hf_sim::nav::step::OVER_HEAD
        );
        assert_eq!(
            (
                world.agents[i].from,
                world.agents[i].pos,
                world.agents[i].move_total
            ),
            (p, q, cost)
        );
        world.meta.tick = 5;
        hf_sim::systems::hazards::step(world);
        assert_eq!(world.drowning.get(&id), Some(&5));
        assert_eq!(world.agents[i].path, vec![bank]);
        for _ in 1..cost {
            movement::step(world);
            assert_eq!(world.agents[i].pos, q);
        }
        movement::step(world);
        assert_eq!((world.agents[i].from, world.agents[i].pos), (q, bank));
        world.meta.tick = 15;
        hf_sim::systems::hazards::step(world);
        assert!(world.agents[i].alive && !world.drowning.contains_key(&id));
        world.agents[i].move_total = 0;
        world.agents[i].path = vec![q];
        movement::step(world);
        assert_eq!(world.agents[i].pos, bank);
        assert!(world.agents[i].path_blocked);
        for cover in [Tile::AIR, liquid(false), Tile::AIR] {
            world.grid.set(q.offset(0, 0, 1), cover);
            nav::step(world);
            let expected = cover.is_water();
            assert_eq!(world.grid.walkable(q), expected);
            assert_eq!(world.nav.as_ref().unwrap().reach(p, q), expected);
            assert_eq!(hf_sim::nav::Nav::build(&world.grid).reach(p, q), expected);
            let mut copy = hf_sim::save::snapshot::Snapshot::capture(world).restore();
            copy.ensure_nav();
            assert_eq!(copy.nav.as_ref().unwrap().reach(p, q), expected);
        }
    }
    assert_eq!(
        hf_sim::digest::digest(&w),
        hf_sim::digest::digest(&reopened)
    );
}

#[test]
fn a_queued_diagonal_cannot_cross_an_exposed_unsupported_flank() {
    let (mut w, p) = pond();
    let start = p.offset(-1, 0, 0);
    let dest = p.offset(0, 1, 0);
    w.grid.set(p, liquid(false));
    assert!(w.grid.walkable(start) && w.grid.walkable(dest));
    let i = w.agents.iter().position(|a| a.community.is_some()).unwrap();
    w.agents[i].pos = start;
    w.agents[i].from = start;
    w.agents[i].path = vec![dest];
    movement::step(&mut w);
    assert_eq!(w.agents[i].pos, start);
    assert!(w.agents[i].path_blocked && w.agents[i].path.is_empty());
    let mut edges = Vec::new();
    hf_sim::nav::neighbours(&w.grid, start, &mut edges);
    assert!(!edges.iter().any(|e| e.to == dest));
    w.grid
        .set(p.offset(0, 0, -1), Tile::solid(Material::GRANITE));
    hf_sim::nav::neighbours(&w.grid, start, &mut edges);
    assert!(edges.iter().any(|e| e.to == dest));
}
