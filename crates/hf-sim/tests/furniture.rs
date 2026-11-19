//! Furniture that keeps what is put on it (#1773): a shelf only means something if what is
//! put on it stays there and is found there.

use hf_sim::ids::{CommunityId, TilePos};
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::structures::StructureKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::DesignationKind;
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

fn valley(seed: u64) -> (World, CommunityId, TilePos) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(14, 14, 2);
    w.meta.tick = TICKS_PER_HOUR * 7;
    w.rebuild_caches();
    let ids = w.found_party(6);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    w.meta.autoplan = true;
    w.ensure_nav();
    (w, cid, home)
}

/// #1773: a finished Shelf with two tools on its tile, a stockpile 6 tiles off, and a third
/// tool on bare floor beside it. After one game-day, no Haul has been posted for the two on
/// the shelf and they are still on its tile, while the third has been hauled.
#[test]
fn tools_on_a_shelf_stay_there() {
    let (mut w, _cid, home) = valley(1773);
    // Nothing else lying loose to haul first.
    w.items.clear();

    let shelf_pos = home.offset(2, 0, 0);
    w.place_structure(StructureKind::Shelf, shelf_pos, Material::WOOD, false);
    w.drop_item(ItemKind::Tool, 1, shelf_pos);
    w.drop_item(ItemKind::Tool, 1, shelf_pos);

    let stock_pos = home.offset(8, 0, 0);
    w.place_structure(StructureKind::Stockpile, stock_pos, Material::SOIL, true);

    let floor_pos = home.offset(3, 0, 0);
    w.drop_item(ItemKind::Tool, 1, floor_pos);

    w.rebuild_caches();
    w.ensure_nav();

    // Drive only the planner, hourly, across a game-day: what matters here is whether the
    // haul filter leaves the shelf tools alone and posts a Haul for the loose one, not the
    // unrelated behaviour of agents who might otherwise take a spare tool up to work with.
    for _ in 0..(TICKS_PER_DAY / TICKS_PER_HOUR) {
        w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
        hf_sim::systems::settlement::step(&mut w);
    }

    // The two tools on the shelf are still there, and never got a Haul posted for them.
    let shelf_tools: u32 = w
        .items
        .iter()
        .filter(|it| {
            it.kind == ItemKind::Tool && matches!(it.place, ItemPlace::Ground(p) if p == shelf_pos)
        })
        .map(|it| it.count)
        .sum();
    assert_eq!(
        shelf_tools, 2,
        "tools kept on a finished shelf should stay on the shelf"
    );
    assert!(
        !w.designations
            .iter()
            .any(|d| d.kind == DesignationKind::Haul && d.pos == shelf_pos),
        "no Haul should ever be posted for a tool kept on a finished shelf"
    );

    // The tool left on bare floor beside the shelf was loose, and has been hauled (a Haul
    // designation was posted for it, unlike the shelf tools).
    assert!(
        w.designations
            .iter()
            .any(|d| d.kind == DesignationKind::Haul && d.pos == floor_pos),
        "a tool left on bare floor should have been ordered hauled"
    );
}

/// #1773/#980: the tool-in-reach helper used by whittling and mending returns a tool lying on
/// a finished shelf's tile, not only one carried or in a chest.
#[test]
fn a_shelf_tool_is_in_reach_for_whittling() {
    let (mut w, _cid, home) = valley(1774);
    w.items.clear();

    let shelf_pos = home.offset(5, 0, 0);
    w.place_structure(StructureKind::Shelf, shelf_pos, Material::WOOD, false);
    w.drop_item(ItemKind::Tool, 1, shelf_pos);
    w.rebuild_caches();

    let person = home.offset(0, 0, 0);
    assert!(
        hf_sim::systems::work::tool_in_reach(&w, person, 8).is_some(),
        "a tool kept on a finished shelf within 8 tiles should count as in reach"
    );
}
