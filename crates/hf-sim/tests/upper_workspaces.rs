//! an already-grown upper room can become a real workshop when the ground is unavailable.

use hf_sim::building::Roof;
use hf_sim::ids::TilePos;
use hf_sim::project::{Project, ProjectKind};
use hf_sim::structures::StructureKind;
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

fn workshop_house(block_ground: bool) -> World {
    let mut w = World::generate(48386, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(20, 20, 2);
    w.meta.tick = 7 * TICKS_PER_HOUR;
    w.found_party(8);
    w.meta.autoplan = true;
    w.designations.clear();

    let floor: Vec<TilePos> = (25..=27)
        .flat_map(|y| (25..=27).map(move |x| TilePos::new(x, y, 2)))
        .collect();
    let ring = hf_sim::houses::walls(&floor);
    let door = ring
        .iter()
        .map(|(p, _)| *p)
        .find(|p| p.x == 26 && p.y == 28)
        .expect("house door");
    for (p, _) in &ring {
        if *p == door {
            w.grid.set(
                *p,
                Tile {
                    material: Material::WOOD,
                    shape: Shape::Wall,
                    flags: TileFlags(TileFlags::DOOR),
                },
            );
        } else {
            w.grid.set(*p, Tile::solid(Material::WOOD));
        }
    }
    for p in &floor {
        w.grid.set(*p, Tile::floor(Material::WOOD));
    }

    let lower_stair = floor[0];
    let upper_stair = lower_stair.offset(0, 0, 1);
    w.grid.set(
        lower_stair,
        Tile {
            material: Material::WOOD,
            shape: Shape::StairsUp,
            flags: TileFlags::NONE,
        },
    );
    let upper_floor: Vec<TilePos> = floor.iter().map(|p| p.offset(0, 0, 1)).collect();
    for p in &upper_floor {
        w.grid.set(
            *p,
            if *p == upper_stair {
                Tile {
                    material: Material::WOOD,
                    shape: Shape::StairsDown,
                    flags: TileFlags::NONE,
                }
            } else {
                Tile::floor(Material::WOOD)
            },
        );
        if *p != upper_stair {
            w.roofs.insert(
                *p,
                Roof {
                    material: Material::WOOD,
                    quality: 0.8,
                    built: 0,
                    snow: 0.0,
                },
            );
        }
    }
    for (p, _) in hf_sim::houses::walls(&upper_floor) {
        w.grid.set(p, Tile::solid(Material::WOOD));
    }

    let origin = TilePos::new(26, 26, 2);
    let plan = hf_sim::society::HousePlan {
        floor,
        doors: vec![door],
        upper_floor,
        upper_stair: Some(lower_stair),
        upper_reason: None,
    };
    let house_ground: std::collections::BTreeSet<_> = plan
        .floor
        .iter()
        .copied()
        .chain(
            hf_sim::houses::walls(&plan.floor)
                .into_iter()
                .map(|(p, _)| p),
        )
        .collect();
    let cid = w.communities[0].id;
    let proposer = w.agents[0].id;
    let c = &mut w.communities[0];
    c.house_sites = vec![origin];
    c.house_plans = vec![plan];
    c.house_stage = vec![3];
    if block_ground {
        for y in 1..59 {
            for x in 1..59 {
                let home_area = (18..=22).contains(&x) && (18..=22).contains(&y);
                let path = (x == 20 && (20..=22).contains(&y))
                    || (y == 20 && (20..=26).contains(&x))
                    || (x == 26 && (20..=28).contains(&y));
                let p = TilePos::new(x, y, 2);
                if !home_area && !path && !house_ground.contains(&p) {
                    let d = hf_sim::systems::settlement::make(
                        &mut w,
                        hf_sim::work::DesignationKind::BuildStructure(
                            StructureKind::Stockpile,
                            Material::SOIL,
                        ),
                        p,
                        0,
                    );
                    w.designations.push(d);
                }
            }
        }
    }
    w.projects.push(Project {
        id: 1,
        by: proposer,
        community: cid,
        kind: ProjectKind::Workshop(StructureKind::CarpenterBench),
        because: "learned carpentry".into(),
        source: None,
        proposed: w.meta.tick,
        work: None,
        done: false,
        site: None,
    });
    w.rebuild_caches();
    w.ensure_nav();
    w
}

#[test]
fn a_needed_workshop_uses_a_finished_upper_room_when_ground_is_unavailable() {
    let mut w = workshop_house(true);
    hf_sim::systems::settlement::step(&mut w);
    let order = w
        .designations
        .iter()
        .find(|d| {
            matches!(
                d.kind,
                hf_sim::work::DesignationKind::BuildStructure(StructureKind::CarpenterBench, _)
            )
        })
        .expect("the workshop is ordered");
    assert_eq!(order.pos.z, 3, "the upper room is the fallback site");
}

#[test]
fn a_reachable_ground_site_stays_preferred_over_the_upper_room() {
    let mut w = workshop_house(false);
    hf_sim::systems::settlement::step(&mut w);
    let order = w
        .designations
        .iter()
        .find(|d| {
            matches!(
                d.kind,
                hf_sim::work::DesignationKind::BuildStructure(StructureKind::CarpenterBench, _)
            )
        })
        .expect("the workshop is ordered");
    assert_eq!(order.pos.z, 2, "the ordinary same-level site wins");
}
