//! Furniture guard tests: structures that the planner should post on its own once the
//! community has what they need.

use hf_sim::ids::TilePos;
use hf_sim::items::ItemKind;
use hf_sim::structures::StructureKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

fn meadow(seed: u64, people: usize) -> (World, hf_sim::ids::CommunityId) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    let ids = w.found_party(people);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    w.ensure_nav();
    (w, cid)
}

#[test]
fn a_cooked_at_hearth_gets_a_pot() {
    // A hearth cooked at for years never got a pot. A Cook order posts (raw food is on
    // hand from the party's own provisions); with a Vessel in store, a CookingPot order follows,
    // made beside the hearth from that vessel.
    let (mut w, cid) = meadow(17701, 6);
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    let hearth = home.offset(-3, -3, 0);
    w.place_structure(StructureKind::Hearth, hearth, Material::STONE_BLOCK, true);
    w.create_lone_item(
        ItemKind::Vessel,
        1,
        hf_sim::items::ItemPlace::Ground(home),
        0.6,
        "test",
    );
    w.rebuild_caches();
    w.ensure_nav();

    w.run(TICKS_PER_DAY * 2);

    let pot = w.structures.iter().find(|s| {
        s.kind == StructureKind::CookingPot && s.progress >= 1.0 && s.pos.dist(hearth) <= 1
    });
    assert!(
        pot.is_some(),
        "a finished CookingPot stands within a tile of the hearth after two days"
    );
    let vessels: u32 = w
        .items
        .iter()
        .filter(|it| it.kind == ItemKind::Vessel)
        .map(|it| it.count)
        .sum();
    assert_eq!(vessels, 0, "the store's one Vessel went into the pot");
}

#[test]
fn no_vessel_in_store_posts_no_cooking_pot() {
    // With no Vessel on hand, nothing turns a hearth's cooking into a pot order.
    let (mut w, cid) = meadow(17702, 6);
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    let hearth = home.offset(-3, -3, 0);
    w.place_structure(StructureKind::Hearth, hearth, Material::STONE_BLOCK, true);
    // No Vessel anywhere near home.
    w.items.retain(|it| it.kind != ItemKind::Vessel);
    w.rebuild_caches();
    w.ensure_nav();

    w.run(TICKS_PER_DAY * 2);

    assert!(
        !w.designations.iter().any(|d| matches!(
            d.kind,
            hf_sim::work::DesignationKind::BuildStructure(StructureKind::CookingPot, _)
        )),
        "no CookingPot order is posted without a Vessel in store"
    );
    assert!(
        !w.structures
            .iter()
            .any(|s| s.kind == StructureKind::CookingPot),
        "no CookingPot stands without a Vessel in store"
    );
}
