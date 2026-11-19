//! #483 structural consequences: an upper room must not float when its support is removed.

use hf_sim::building::{self, Roof};
use hf_sim::chronicle::Chronicle;
use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::structures::{Structure, StructureKind};
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile};
use hf_sim::World;

fn upper_room() -> (World, TilePos, TilePos, TilePos, TilePos) {
    let mut w = World::generate(48302, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(41, 45, 2);
    w.found_party(1);
    w.meta.autoplan = false;
    w.designations.clear();

    let support = TilePos::new(39, 40, 2);
    let floor = TilePos::new(40, 40, 3);
    let unrelated = TilePos::new(44, 40, 2);
    let upper_wall = TilePos::new(41, 40, 3);
    w.grid.set(support, Tile::solid(Material::WOOD));
    w.built_of.insert(support, Material::OAK);
    w.grid.set(floor, Tile::floor(Material::WOOD));
    w.built_of.insert(floor, Material::OAK);
    w.grid.set(upper_wall, Tile::solid(Material::WOOD));
    w.built_of.insert(upper_wall, Material::OAK);
    w.roofs.insert(
        floor,
        Roof {
            material: Material::WOOD,
            quality: 0.6,
            built: 0,
            snow: 0.0,
        },
    );
    let structure = w.alloc_structure_id();
    w.structures.push(Structure {
        id: structure,
        kind: StructureKind::CarpenterBench,
        pos: floor,
        material: Material::WOOD,
        progress: 1.0,
        active: false,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
    w.grid.set(unrelated, Tile::solid(Material::WOOD));
    w.built_of.insert(unrelated, Material::OAK);
    w.rebuild_caches();
    w.ensure_nav();
    (w, support, floor, unrelated, upper_wall)
}

#[test]
fn removing_the_only_lower_support_removes_the_upper_room_contents() {
    let (mut w, support, floor, _, upper_wall) = upper_room();
    w.grid.set(support, Tile::AIR);
    w.built_of.remove(&support);
    w.meta.tick = TICKS_PER_HOUR + 41;

    building::step(&mut w);

    assert_eq!(w.grid.get(floor).shape, Shape::Open);
    assert_eq!(w.grid.get(upper_wall).shape, Shape::Open);
    assert!(!w.roofs.contains_key(&floor));
    assert!(!w.structures.iter().any(|s| s.pos == floor));
    assert!(
        w.events
            .pending
            .iter()
            .any(|e| matches!(e.kind, EventKind::UpperFloorFell { .. })),
        "events: {:?}",
        w.events.pending
    );
}

#[test]
fn removing_a_non_supporting_lower_wall_leaves_the_upper_room_intact() {
    let (mut w, _, floor, unrelated, upper_wall) = upper_room();
    w.grid.set(unrelated, Tile::AIR);
    w.built_of.remove(&unrelated);
    w.meta.tick = TICKS_PER_HOUR + 41;

    building::step(&mut w);

    assert_eq!(w.grid.get(floor).shape, Shape::Floor);
    assert_eq!(w.grid.get(upper_wall).shape, Shape::Wall);
    assert!(w.roofs.contains_key(&floor));
    assert!(w.structures.iter().any(|s| s.pos == floor));
}

#[test]
fn an_upper_room_collapse_survives_save_load_with_its_event() {
    let (mut w, support, floor, _, upper_wall) = upper_room();
    w.grid.set(support, Tile::AIR);
    w.built_of.remove(&support);
    w.meta.tick = TICKS_PER_HOUR + 41;
    building::step(&mut w);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("upper-collapse.hfsave");
    let mut chronicle = Chronicle::in_memory().unwrap();
    hf_sim::save::write(&w, &mut chronicle, &path).unwrap();
    let (back, _) = hf_sim::save::read(&path).unwrap();

    assert_eq!(back.grid.get(floor).shape, Shape::Open);
    assert_eq!(back.grid.get(upper_wall).shape, Shape::Open);
    assert!(
        back.events
            .pending
            .iter()
            .any(|e| matches!(e.kind, EventKind::UpperFloorFell { .. })),
        "events: {:?}",
        back.events.pending
    );
}
