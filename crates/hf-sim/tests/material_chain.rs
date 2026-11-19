//! C67: what is built is made first. Walls want sawn planks or dressed stone, not logs, and the
//! benches that make them are ordered, then the crafts (docs/design-rules.md section 2).

use hf_sim::ids::{CommunityId, TilePos};
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::mind::ThoughtKind;
use hf_sim::structures::{Structure, StructureKind};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::{recipes, DesignationKind};
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

fn valley(seed: u64) -> (World, CommunityId) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(14, 14, 2);
    w.meta.tick = TICKS_PER_HOUR * 7;
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

fn a_morning(w: &mut World, cid: CommunityId) {
    let tick = w.meta.tick;
    for a in w.agents.iter_mut().filter(|a| a.community == Some(cid)) {
        a.thoughts.add(ThoughtKind::SleptRough, tick, None);
    }
    let day = w.meta.tick / TICKS_PER_DAY;
    w.meta.tick = day * TICKS_PER_DAY + hf_sim::systems::projects::AT;
    hf_sim::systems::projects::step(w);
    for _ in 0..2 {
        w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
        hf_sim::systems::settlement::step(w);
    }
}

#[test]
fn walls_want_made_things_and_the_benches_are_ordered_to_make_them() {
    let (mut w, cid) = valley(6711);
    a_morning(&mut w, cid);
    let walls: Vec<_> = w
        .designations
        .iter()
        .filter(|d| matches!(d.kind, DesignationKind::BuildWall(_)))
        .collect();
    assert!(!walls.is_empty(), "a house is marked out");
    for d in &walls {
        for (k, _) in d.missing() {
            assert!(
                matches!(k, ItemKind::Plank | ItemKind::StoneBlock),
                "a wall wants {k:?}"
            );
        }
    }
    assert!(w
        .designations
        .iter()
        .any(|d| d.kind == DesignationKind::BuildDoor
            && d.missing().all(|(k, _)| k == ItemKind::Door)));
    let bench = w
        .designations
        .iter()
        .find(|d| {
            matches!(
                d.kind,
                DesignationKind::BuildStructure(StructureKind::CarpenterBench, _)
            )
        })
        .expect("a carpenter's bench is ordered to saw the planks");
    let at = bench.pos;
    // Once the bench stands, planks are ordered sawn at it; and with planks in hand, the door.
    w.designations.retain(|d| d.pos != at);
    let id = w.alloc_structure_id();
    w.structures.push(Structure {
        id,
        kind: StructureKind::CarpenterBench,
        pos: at,
        material: Material::WOOD,
        progress: 1.0,
        active: false,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    w.create_item_of(
        ItemKind::Plank,
        Material::WOOD,
        2,
        ItemPlace::Ground(home),
        0.6,
        "test",
    );
    w.rebuild_caches();
    w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(&mut w);
    let crafts: Vec<u8> = w
        .designations
        .iter()
        .filter_map(|d| match d.kind {
            DesignationKind::Craft(r) => Some(r),
            _ => None,
        })
        .collect();
    assert!(crafts.contains(&recipes::PLANK), "{crafts:?}");
    assert!(crafts.contains(&recipes::DOOR), "{crafts:?}");
}
