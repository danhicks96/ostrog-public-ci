//! Phase 9b gates: digging reveals caves as recorded discoveries, and the community uses
//! a room below ground.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::items::ItemPlace;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::structures::StructureKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::{Designation, DesignationKind};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

fn meadow(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    for i in 0..200 {
        let p = TilePos::new(3 + (i * 7) % 58, 3 + (i * 13 + i / 58) % 58, 2);
        if !w.plants.iter().any(|q| q.pos == p) {
            w.plants
                .push(Plant::new(p, PlantKind::BerryBush, 1.0, i as u32));
        }
    }
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    w
}

fn open(p: TilePos, w: &mut World) {
    w.grid.set(
        p,
        Tile {
            material: Material::AIR,
            shape: Shape::Open,
            flags: TileFlags::NONE,
        },
    );
}

/// A shaft at (40,30) from z2 down to z0, a natural cave two tiles east at z0 (with
/// mushrooms, as worldgen makes them), one wall tile between, and an order to mine it. With
/// `cavern`, the cave is one of the named great chambers.
fn shaft_to_cave(cavern: bool) -> (World, hf_sim::ids::CommunityId) {
    let mut w = meadow(111);
    w.meta.autoplan = false;
    let ids = w.found_party(4);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    w.grid.set(
        TilePos::new(40, 30, 2),
        Tile {
            material: Material::STONE_BLOCK,
            shape: Shape::StairsDown,
            flags: TileFlags::NONE,
        },
    );
    w.grid.set(
        TilePos::new(40, 30, 1),
        Tile {
            material: Material::GRANITE,
            shape: Shape::StairsBoth,
            flags: TileFlags::NONE,
        },
    );
    w.grid.set(
        TilePos::new(40, 30, 0),
        Tile {
            material: Material::GRANITE,
            shape: Shape::StairsUp,
            flags: TileFlags::NONE,
        },
    );
    open(TilePos::new(41, 30, 0), &mut w);
    for x in 43..48 {
        for y in 28..33 {
            open(TilePos::new(x, y, 0), &mut w);
        }
    }
    w.plants.push(Plant::new(
        TilePos::new(44, 30, 0),
        PlantKind::Mushroom,
        1.0,
        1,
    ));
    // Order: mine the wall at (42,30,0).
    let id = w.alloc_designation_id();
    w.designations.push(Designation {
        id,
        kind: DesignationKind::Mine,
        pos: TilePos::new(42, 30, 0),
        requirements: vec![],
        work_left: 20 * 15,
        work_total: 20 * 15,
        max_workers: 1,
        priority: 9,
        created: w.tick(),
        failures: 0,
        done: false,
        community: Some(cid),
        owner: None,
    });
    if cavern {
        w.grid.caverns.push(hf_sim::world::gen::caverns::Cavern {
            name: "the Echoing Hall".into(),
            centre: TilePos::new(45, 30, 0),
            rx: 2,
            ry: 2,
            pool: true,
            deep: false,
            opened: false,
            underworld: false,
            breach: None,
            sealed: false,
            gate_of: None,
            keeper: None,
        });
    }
    (w, cid)
}

#[test]
fn dig_reveals_cave_event() {
    let (mut w, cid) = shaft_to_cave(false);
    let mut discovered = false;
    for _ in 0..TICKS_PER_HOUR * 6 {
        w.step();
        if w.last_events
            .iter()
            .any(|e| matches!(&e.kind, EventKind::Discovered { what, .. } if what == "a cave"))
        {
            discovered = true;
            break;
        }
    }
    assert!(
        discovered,
        "mining into the cave was recorded as a discovery"
    );
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    assert!(
        c.discoveries.iter().any(|(what, _, _)| what == "a cave"),
        "the community knows of the cave"
    );
    assert!(
        w.grid.walkable(TilePos::new(42, 30, 0)),
        "the way into the cave is open"
    );
}

#[test]
fn digging_into_a_cavern_names_it() {
    let (mut w, cid) = shaft_to_cave(true);
    let mut seen = Vec::new();
    for _ in 0..TICKS_PER_HOUR * 6 {
        w.step();
        for e in &w.last_events {
            if let EventKind::Discovered { what, .. } = &e.kind {
                seen.push(what.clone());
            }
        }
    }
    assert_eq!(
        seen,
        vec!["the Echoing Hall, a cavern grown with pale fungus around a still pool".to_string()],
        "named once, not once per tile"
    );
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    assert!(c
        .discoveries
        .iter()
        .any(|(what, _, _)| what.starts_with("the Echoing Hall")));
}

#[test]
fn underground_cellar_in_use() {
    // With the house up and six people, the community digs a cellar and stores go below.
    let mut w = meadow(112);
    let ids = w.found_party(8);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    // Build the house directly so the test stays short.
    let origin = TilePos::new(20, 22, 2);
    w.build_house(origin.x, origin.y, 7, 6, 1, Material::WOOD, Material::WOOD);
    w.communities
        .iter_mut()
        .find(|c| c.id == cid)
        .unwrap()
        .house_sites
        .push(origin);
    for i in 0..30 {
        w.drop_item(
            hf_sim::items::ItemKind::Log,
            3,
            TilePos::new(34 + i % 6, 34 + i / 6, 2),
        );
    }
    let mut below = false;
    let mut someone_down = false;
    for _ in 0..TICKS_PER_DAY * 8 {
        w.step();
        if w.structures
            .iter()
            .any(|s| s.kind == StructureKind::Stockpile && s.pos.z < 2)
        {
            below = w.items.iter().any(|it| matches!(it.place, ItemPlace::Ground(p) if p.z < 2 && w.structures.iter().any(|s| s.pos == p && s.kind == StructureKind::Stockpile)));
        }
        if w.agents
            .iter()
            .any(|a| a.alive && a.species == hf_sim::agents::Species::Person && a.pos.z < 2)
        {
            someone_down = true;
        }
        if below && someone_down {
            break;
        }
    }
    assert!(
        hf_sim::houses::plan(&w, origin)
            .floor
            .iter()
            .any(|p| w.grid.get(*p).shape == Shape::StairsDown),
        "stairs lead down from the house floor"
    );
    assert!(someone_down, "people went down to the cellar");
    assert!(below, "goods were stored in the cellar");
}
