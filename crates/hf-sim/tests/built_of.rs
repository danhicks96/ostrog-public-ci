//! C56: a wall, a door and a floor are of the wood or stone that was brought for them.

use hf_sim::agents::Species;
use hf_sim::ids::TilePos;
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::work::{Designation, DesignationKind, Requirement};
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

fn meadow(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.meta.autoplan = false;
    w.rebuild_caches();
    w
}

fn post(w: &mut World, kind: DesignationKind, pos: TilePos, need: ItemKind) {
    let id = w.alloc_designation_id();
    w.designations.push(Designation {
        id,
        kind,
        pos,
        requirements: vec![Requirement {
            kind: need,
            needed: 1,
            delivered: 0,
            material: Material::AIR,
            want: None,
            quality: 0.0,
        }],
        work_left: 200,
        work_total: 200,
        max_workers: 1,
        priority: 9,
        created: w.tick(),
        failures: 0,
        done: false,
        community: None,
        owner: None,
    });
}

fn builder(w: &mut World) {
    let id = w.spawn_agent(Species::Person, TilePos::new(30, 30, 2));
    let i = w.agents.iter().position(|a| a.id == id).unwrap();
    w.agents[i].needs.hunger = 1.0;
    w.agents[i].needs.rest = 1.0;
}

#[test]
fn a_wall_is_of_the_wood_it_was_built_from() {
    let mut w = meadow(5601);
    builder(&mut w);
    w.create_item_of(
        ItemKind::Log,
        Material::BIRCH,
        2,
        ItemPlace::Ground(TilePos::new(32, 30, 2)),
        0.6,
        "test",
    );
    let wall = TilePos::new(36, 30, 2);
    let door = TilePos::new(36, 31, 2);
    post(
        &mut w,
        DesignationKind::BuildWall(Material::WOOD),
        wall,
        ItemKind::Log,
    );
    post(&mut w, DesignationKind::BuildDoor, door, ItemKind::Log);
    w.run(TICKS_PER_HOUR * 6);
    assert_eq!(
        w.grid.get(wall).material,
        Material::WOOD,
        "the tile stays timber to everything else"
    );
    assert_eq!(w.made_of(wall), Some(Material::BIRCH), "the wall is birch");
    assert_eq!(w.made_of(door), Some(Material::BIRCH), "the door is birch");
}

#[test]
fn a_stone_wall_is_of_its_stone() {
    let mut w = meadow(5602);
    builder(&mut w);
    w.create_item_of(
        ItemKind::StoneBlock,
        Material::GRANITE,
        1,
        ItemPlace::Ground(TilePos::new(32, 30, 2)),
        0.6,
        "test",
    );
    let wall = TilePos::new(36, 30, 2);
    post(
        &mut w,
        DesignationKind::BuildWall(Material::STONE_BLOCK),
        wall,
        ItemKind::StoneBlock,
    );
    w.run(TICKS_PER_HOUR * 6);
    assert_eq!(w.grid.get(wall).material, Material::STONE_BLOCK);
    assert_eq!(w.made_of(wall), Some(Material::GRANITE));
    // Once it is gone, what it was made of no longer counts.
    w.grid.set(wall, hf_sim::world::Tile::AIR);
    assert_eq!(w.made_of(wall), None);
}

#[test]
fn every_wall_material_has_a_sheet() {
    // Every wood and every rock that cuts to blocks can become a wall; each needs its art.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../godot/assets");
    let w = meadow(5603);
    let mut missing = Vec::new();
    for d in &w.grid.materials.defs {
        let wall_kind = matches!(d.kind, hf_sim::world::material::MaterialKind::Wood)
            || (matches!(d.kind, hf_sim::world::material::MaterialKind::Stone)
                && d.yields == Some(Material::STONE_BLOCK));
        if !wall_kind {
            continue;
        }
        let slug = hf_sim::items::slug(&d.name);
        for f in [
            format!("structures/wall_{slug}.png"),
            format!("overlays/wall_face_{slug}.png"),
        ] {
            if !root.join(&f).exists() {
                missing.push(f);
            }
        }
    }
    assert!(missing.is_empty(), "no art for: {missing:?}");
}
