//! Furniture that keeps what is put on it (#1773): a shelf only means something if what is
//! put on it stays there and is found there.

use hf_sim::houses::free_seat;
use hf_sim::ids::{CommunityId, TilePos};
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::structures::{Structure, StructureKind};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::{DesignationKind, ResKey};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Tile};
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

// OST-1307: a free stool or chair can be found and held.

fn put_seat_piece(w: &mut World, kind: StructureKind, p: TilePos) -> hf_sim::ids::StructureId {
    let id = w.alloc_structure_id();
    w.structures.push(Structure {
        id,
        kind,
        pos: p,
        material: Material::WOOD,
        progress: 1.0,
        active: false,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
    id
}

#[test]
fn free_seat_finds_the_stool_beside_the_table() {
    let mut w = World::generate(1004, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(20, 20, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    let ids = w.found_party(2);
    let (me, other) = (ids[0], ids[1]);
    let near = TilePos::new(20, 20, 2);
    let table = TilePos::new(24, 20, 2);
    put_seat_piece(&mut w, StructureKind::Table, table);
    // A stool apart from the table, nearest of all, then two beside it.
    let apart = put_seat_piece(&mut w, StructureKind::Stool, TilePos::new(21, 20, 2));
    let s1 = put_seat_piece(&mut w, StructureKind::Stool, TilePos::new(23, 20, 2));
    let s2 = put_seat_piece(&mut w, StructureKind::Stool, TilePos::new(25, 20, 2));
    w.ensure_nav();

    assert_eq!(free_seat(&w, near, 8, false, me).unwrap().0, apart);
    assert_eq!(free_seat(&w, near, 8, true, me).unwrap().0, s1);
    // A reserved seat is skipped, and so is one someone sits on.
    assert!(w
        .reservations
        .acquire_all(other, &[ResKey::Station(s1)], u64::MAX));
    assert_eq!(free_seat(&w, near, 8, true, me).unwrap().0, s2);
    w.reservations.release_all(other);
    let ai = w.agents.iter().position(|a| a.id == other).unwrap();
    w.agents[ai].pos = TilePos::new(23, 20, 2);
    assert_eq!(free_seat(&w, near, 8, true, me).unwrap().0, s2);
    w.agents[ai].pos = TilePos::new(40, 40, 2);

    // A table with one stool, on ground walled off (water here is waded, so only a wall cuts nav): nearest, but not returned.
    let island = TilePos::new(20, 31, 2);
    let far = put_seat_piece(&mut w, StructureKind::Stool, island);
    put_seat_piece(&mut w, StructureKind::Table, TilePos::new(20, 32, 2));
    for dy in -3i32..=3 {
        for dx in -3i32..=3 {
            if dx.abs() == 3 || dy.abs() == 3 || dx.abs() == 2 || dy.abs() == 2 {
                w.grid
                    .set(island.offset(dx, dy, 0), Tile::solid(Material::WOOD));
            }
        }
    }
    w.nav = None;
    w.ensure_nav();
    let near2 = TilePos::new(20, 28, 2);
    let got = free_seat(&w, near2, 8, false, me).map(|s| s.0);
    assert_ne!(got, Some(far));
    assert!(got.is_none());
}
