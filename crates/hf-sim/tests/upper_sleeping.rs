//! #483's first production slice: an existing house may grow an organic upper sleeping room
//! only when a colonist lacks a bed and the same-level house cannot grow an annex.

use hf_sim::building::Roof;
use hf_sim::ids::TilePos;
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

fn standing_house(fill_annex_site: bool) -> World {
    let mut w = World::generate(48321, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(20, 20, 2);
    w.meta.tick = 7 * TICKS_PER_HOUR;
    w.rebuild_caches();
    w.found_party(1);
    w.meta.autoplan = true;
    w.designations.clear();

    let floor: Vec<TilePos> = (40..=42)
        .flat_map(|y| (40..=42).map(move |x| TilePos::new(x, y, 2)))
        .collect();
    let ring = hf_sim::houses::walls(&floor);
    let door = ring
        .iter()
        .map(|(p, _)| *p)
        .find(|p| p.y == 43 && p.x == 41)
        .unwrap();
    for (p, _) in &ring {
        if *p == door {
            let mut tile = Tile::solid(Material::WOOD);
            tile.flags = hf_sim::world::TileFlags(hf_sim::world::TileFlags::DOOR);
            w.grid.set(*p, tile);
        } else {
            w.grid.set(*p, Tile::solid(Material::WOOD));
        }
    }
    for p in &floor {
        w.roofs.insert(
            *p,
            Roof {
                material: Material::WOOD,
                quality: 0.6,
                built: 0,
                snow: 0.0,
            },
        );
    }
    let plan = hf_sim::society::HousePlan {
        floor: floor.clone(),
        doors: vec![door],
        ..Default::default()
    };
    let c = &mut w.communities[0];
    c.house_sites = vec![TilePos::new(41, 41, 2)];
    c.house_plans = vec![plan];
    c.house_stage = vec![3];
    w.agents[0].needs.rest = 0.1;

    if fill_annex_site {
        let house: std::collections::BTreeSet<_> = floor
            .iter()
            .copied()
            .chain(ring.iter().map(|(p, _)| *p))
            .collect();
        for y in 34..=48 {
            for x in 34..=48 {
                let p = TilePos::new(x, y, 2);
                if !house.contains(&p) {
                    w.grid.set(p, Tile::solid(Material::GRANITE));
                }
            }
        }
    }
    w.rebuild_caches();
    w.ensure_nav();
    w
}

#[test]
fn unmet_sleeping_need_without_annex_proposes_upper_floor_stairs() {
    let mut w = standing_house(true);
    hf_sim::systems::settlement::step(&mut w);
    assert!(
        w.designations.iter().any(|d| {
            matches!(d.kind, hf_sim::work::DesignationKind::BuildStairs(_)) && d.pos.z == 2
        }),
        "an exhausted colonist with no same-level annex gets a material-backed upper-floor plan"
    );
}

#[test]
fn available_same_level_annex_wins_over_upper_floor() {
    let mut w = standing_house(false);
    hf_sim::systems::settlement::step(&mut w);
    assert!(!w
        .designations
        .iter()
        .any(|d| matches!(d.kind, hf_sim::work::DesignationKind::BuildStairs(_))));
}

fn next_hour(w: &mut World) {
    w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(w);
}

fn upper_orders(w: &World) -> Vec<hf_sim::work::DesignationKind> {
    w.designations
        .iter()
        .filter(|d| !d.done && d.pos.z == 3)
        .map(|d| d.kind)
        .collect()
}

#[test]
fn upper_sleeping_plan_is_persisted_and_its_stages_wait_in_order() {
    let mut w = standing_house(true);
    hf_sim::systems::settlement::step(&mut w);
    let mut plan = hf_sim::houses::plan(&w, w.communities[0].house_sites[0]);
    let lower_stair = plan.upper_stair.unwrap();
    let upper_stair = lower_stair.offset(0, 0, 1);
    assert!(
        !plan.upper_floor.is_empty(),
        "the proposal is persisted on the house"
    );
    assert!(w.designations.iter().any(|d| {
        d.pos == lower_stair && matches!(d.kind, hf_sim::work::DesignationKind::BuildStairs(_))
    }));
    assert!(
        upper_orders(&w).is_empty(),
        "stairs precede upper-floor orders"
    );

    // Complete only the stair, as the material-backed worker would, then let the hourly planner
    // reveal the next stage.
    w.designations.clear();
    w.grid.set(
        lower_stair,
        Tile {
            material: Material::WOOD,
            shape: Shape::StairsUp,
            flags: TileFlags::NONE,
        },
    );
    w.grid.set(
        upper_stair,
        Tile {
            material: Material::WOOD,
            shape: Shape::StairsDown,
            flags: TileFlags::NONE,
        },
    );
    w.rebuild_caches();
    w.ensure_nav();
    next_hour(&mut w);
    let orders = upper_orders(&w);
    assert!(orders
        .iter()
        .any(|k| matches!(k, hf_sim::work::DesignationKind::BuildFloor(_))));
    assert!(w
        .designations
        .iter()
        .filter(|d| { !d.done && matches!(d.kind, hf_sim::work::DesignationKind::BuildFloor(_)) })
        .all(|d| hf_sim::building::floor_supported(&w, d.pos)));
    assert!(!orders.iter().any(|k| matches!(
        k,
        hf_sim::work::DesignationKind::BuildWall(_)
            | hf_sim::work::DesignationKind::BuildRoof(_)
            | hf_sim::work::DesignationKind::BuildStructure(..)
    )));

    plan = hf_sim::houses::plan(&w, w.communities[0].house_sites[0]);
    w.designations.clear();
    for p in plan
        .upper_floor
        .iter()
        .copied()
        .filter(|p| *p != upper_stair)
    {
        w.grid.set(p, Tile::floor(Material::WOOD));
    }
    w.rebuild_caches();
    w.ensure_nav();
    next_hour(&mut w);
    let orders = upper_orders(&w);
    assert!(orders
        .iter()
        .any(|k| matches!(k, hf_sim::work::DesignationKind::BuildWall(_))));
    assert!(!orders.iter().any(|k| matches!(
        k,
        hf_sim::work::DesignationKind::BuildRoof(_)
            | hf_sim::work::DesignationKind::BuildStructure(..)
    )));

    let ring = hf_sim::houses::walls(&plan.upper_floor);
    w.designations.clear();
    for (p, _) in ring {
        w.grid.set(p, Tile::solid(Material::WOOD));
    }
    w.rebuild_caches();
    w.ensure_nav();
    next_hour(&mut w);
    let orders = upper_orders(&w);
    assert!(orders
        .iter()
        .any(|k| matches!(k, hf_sim::work::DesignationKind::BuildRoof(_))));
    assert!(!orders
        .iter()
        .any(|k| matches!(k, hf_sim::work::DesignationKind::BuildStructure(..))));

    w.designations.clear();
    for p in plan
        .upper_floor
        .iter()
        .copied()
        .filter(|p| *p != upper_stair)
    {
        w.roofs.insert(
            p,
            Roof {
                material: Material::WOOD,
                quality: 0.6,
                built: w.meta.tick,
                snow: 0.0,
            },
        );
    }
    w.rebuild_caches();
    w.ensure_nav();
    next_hour(&mut w);
    assert!(upper_orders(&w).iter().any(|k| matches!(
        k,
        hf_sim::work::DesignationKind::BuildStructure(hf_sim::structures::StructureKind::Bed, _)
    )));
}
