//! #1020: a fitting keeps the quality it was made with when it is set (D57).

use hf_sim::agents::Species;
use hf_sim::ids::TilePos;
use hf_sim::items::ItemKind;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::structures::{Structure, StructureKind};
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
    w.plants.push(Plant::new(
        TilePos::new(8, 8, 2),
        PlantKind::BerryBush,
        1.0,
        1,
    ));
    w
}

fn set(w: &mut World, kind: StructureKind, item: ItemKind, quality: f32, at: TilePos) {
    let id = w.alloc_designation_id();
    w.designations.push(Designation {
        id,
        kind: DesignationKind::BuildStructure(kind, Material::AIR),
        pos: at,
        requirements: vec![Requirement {
            kind: item,
            needed: 1,
            delivered: 1,
            material: Material::AIR,
            want: None,
            quality,
        }],
        work_left: 60,
        work_total: 60,
        max_workers: 1,
        priority: 9,
        created: w.tick(),
        failures: 0,
        done: false,
        community: None,
        owner: None,
    });
}

#[test]
fn a_set_table_keeps_its_quality() {
    let mut w = meadow(10201);
    let id = w.spawn_agent(Species::Person, TilePos::new(30, 30, 2));
    let idx = w.agents.iter().position(|a| a.id == id).unwrap();
    w.agents[idx].needs.hunger = 1.0;
    w.agents[idx].needs.rest = 1.0;
    let at = TilePos::new(32, 30, 2);
    set(&mut w, StructureKind::Table, ItemKind::Table, 0.8, at);
    w.run(TICKS_PER_HOUR * 4);
    let s = w
        .structures
        .iter()
        .find(|s| s.kind == StructureKind::Table && s.pos == at)
        .expect("the table was set");
    assert!((s.quality - 0.8).abs() < 1e-4, "quality {}", s.quality);
}

#[test]
fn a_structure_from_an_old_save_is_middling() {
    let s = Structure {
        id: hf_sim::ids::StructureId(1),
        kind: StructureKind::Table,
        pos: TilePos::new(1, 1, 1),
        material: Material::AIR,
        progress: 1.0,
        active: false,
        condition: 1.0,
        owner: None,
        quality: 0.9,
    };
    let mut v = serde_json::to_value(&s).unwrap();
    v.as_object_mut().unwrap().remove("quality");
    let old: Structure = serde_json::from_value(v).unwrap();
    assert_eq!(old.quality, 0.5);
}
