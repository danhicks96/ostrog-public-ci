//! C58: a community's way of building is set by its first house and followed by the next.

use hf_sim::agents::Species;
use hf_sim::culture::CustomKind;
use hf_sim::ids::TilePos;
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::work::{Designation, DesignationKind, Requirement};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Tile};
use hf_sim::World;

fn valley(seed: u64) -> (World, hf_sim::ids::CommunityId) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(14, 14, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    let ids = w.found_party(4);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    w.meta.autoplan = true;
    w.ensure_nav();
    (w, cid)
}

fn way(w: &World, cid: hf_sim::ids::CommunityId) -> Option<CustomKind> {
    w.customs
        .iter()
        .find(|c| c.community == cid && matches!(c.kind, CustomKind::Building { .. }))
        .map(|c| c.kind.clone())
}

#[test]
fn the_first_house_sets_the_way_of_building() {
    let (mut w, cid) = valley(5810);
    // A house standing, its walls of pine (two of oak), its roof just laid in thatch.
    let o = TilePos::new(30, 30, 2);
    // C67: thatch roofs need reeds within reach of home, so a reed bed stands by the house.
    for i in 0..4 {
        w.plants.push(hf_sim::plants::Plant::new(
            TilePos::new(26 + i, 38, 2),
            hf_sim::plants::PlantKind::Reed,
            1.0,
            i as u32,
        ));
    }
    let mut k = 0;
    for y in 0..6 {
        for x in 0..7 {
            if x == 0 || y == 0 || x == 6 || y == 5 {
                let q = o.offset(x, y, 0);
                w.grid.set(q, Tile::solid(Material::WOOD));
                w.built_of
                    .insert(q, if k < 2 { Material::OAK } else { Material::PINE });
                k += 1;
            }
        }
    }
    for p in hf_sim::systems::settlement::house_interior(&w, o) {
        w.roofs.insert(
            p,
            hf_sim::building::Roof {
                material: Material::THATCH,
                quality: 0.6,
                built: 0,
                snow: 0.0,
            },
        );
    }
    let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
    c.house_sites.push(o);
    c.house_stage.push(2);
    assert!(way(&w, cid).is_none());
    w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(&mut w);
    assert_eq!(
        way(&w, cid),
        Some(CustomKind::Building {
            wood: Some(Material::PINE),
            roof: Material::THATCH
        })
    );
    let c = w
        .customs
        .iter()
        .find(|c| matches!(c.kind, CustomKind::Building { .. }))
        .unwrap();
    assert!(c.narrative.contains("first house"), "{}", c.narrative);
    assert_eq!(
        hf_sim::systems::settlement::roof_style(&w, cid, Material::WOOD),
        Material::THATCH,
        "later houses are roofed the same way"
    );
}

#[test]
fn a_later_wall_is_of_their_wood_when_it_is_to_hand() {
    let (mut w, cid) = valley(5811);
    w.customs.push(hf_sim::culture::Custom {
        id: hf_sim::culture::CustomId(0),
        community: cid,
        kind: CustomKind::Building {
            wood: Some(Material::OAK),
            roof: Material::WOOD,
        },
        origin: hf_sim::ids::EventId(0),
        origin_tick: 0,
        narrative: String::new(),
        intensity: 0.8,
        adopted_by: Vec::new(),
        formed: 0,
        lapsed: false,
        version: 1,
    });
    let id = w.spawn_agent(Species::Person, TilePos::new(30, 30, 2));
    let i = w.agents.iter().position(|a| a.id == id).unwrap();
    w.agents[i].needs.hunger = 1.0;
    w.agents[i].needs.rest = 1.0;
    w.join_community(id, cid);
    // No other timber; birch close by, oak a little further off.
    let logs: Vec<_> = w
        .items
        .iter()
        .filter(|i| i.kind == ItemKind::Log)
        .map(|i| (i.id, i.count))
        .collect();
    for (id, n) in logs {
        w.consume_item(id, n, "test");
    }
    let birch = w.create_item_of(
        ItemKind::Log,
        Material::BIRCH,
        1,
        ItemPlace::Ground(TilePos::new(31, 30, 2)),
        0.6,
        "t",
    );
    let oak = w.create_item_of(
        ItemKind::Log,
        Material::OAK,
        1,
        ItemPlace::Ground(TilePos::new(36, 30, 2)),
        0.6,
        "t",
    );
    // Everyone of the community knows where both lie, so whoever takes the order chooses between
    // them (#423 review: which member took it, and so whether they had seen the oak, followed
    // agent ids, and a beast added to the valley changed it).
    let tick = w.tick();
    for a in w.agents.iter_mut().filter(|a| a.community == Some(cid)) {
        for (item, at) in [
            (birch, TilePos::new(31, 30, 2)),
            (oak, TilePos::new(36, 30, 2)),
        ] {
            a.knowledge.note(
                hf_sim::mind::knowledge::Known::Item(item),
                at,
                tick,
                ItemKind::Log as u16,
            );
        }
    }
    let wall = TilePos::new(34, 33, 2);
    let did = w.alloc_designation_id();
    w.designations.push(Designation {
        id: did,
        kind: DesignationKind::BuildWall(Material::WOOD),
        pos: wall,
        requirements: vec![Requirement {
            kind: ItemKind::Log,
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
        community: Some(cid),
        owner: None,
    });
    w.meta.autoplan = false;
    w.run(TICKS_PER_HOUR * 6);
    assert_eq!(
        w.made_of(wall),
        Some(Material::OAK),
        "built of their wood, though birch lay nearer"
    );
}
