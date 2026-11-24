//! A room's stats and its impressiveness (OST-1402).

use hf_sim::houses::room_stats;
use hf_sim::ids::TilePos;
use hf_sim::items::ItemKind;
use hf_sim::society::HousePlan;
use hf_sim::structures::StructureKind;
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

fn plan_of(w: usize, h: usize, x0: i32) -> HousePlan {
    let mut floor = Vec::new();
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            floor.push(TilePos::new(x0 + x, 20, 2).offset(0, y, 0));
        }
    }
    HousePlan {
        floor,
        ..HousePlan::default()
    }
}

#[test]
fn a_bare_hut_scores_below_a_furnished_house() {
    let mut w = World::generate(1021, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.structures.clear();
    w.items.clear();
    let hut = plan_of(3, 2, 10);
    let house = plan_of(4, 4, 30);
    let bare = room_stats(&w, &hut, 3);
    let bare_score = bare.impressiveness();

    let at = |dx, dy| TilePos::new(30 + dx, 20 + dy, 2);
    w.place_structure(StructureKind::Table, at(0, 0), Material::AIR, false);
    w.place_structure(StructureKind::Stool, at(1, 0), Material::AIR, false);
    w.place_structure(StructureKind::Stool, at(2, 0), Material::AIR, false);
    w.place_structure(StructureKind::Lamp, at(3, 3), Material::AIR, true);
    let tidy = room_stats(&w, &house, 2);
    assert!(tidy.light == 1.0 && tidy.furnish > 0.0);
    assert!(
        bare_score < tidy.impressiveness(),
        "{bare_score} vs {}",
        tidy.impressiveness()
    );

    for dx in 0..3 {
        w.drop_item(ItemKind::Log, 1, at(dx, 2));
    }
    let messy = room_stats(&w, &house, 2);
    assert!(messy.clutter > 0.0);
    assert!(messy.impressiveness() < tidy.impressiveness());
}

#[test]
fn a_laid_ground_floor_raises_beauty() {
    use hf_sim::world::Tile;
    let mut w = World::generate(1021, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.structures.clear();
    w.items.clear();
    let plan = plan_of(3, 2, 10);
    let before = room_stats(&w, &plan, 2).beauty;
    // The tile state a ground-level BuildFloor leaves: the tile below the room is built solid.
    for p in plan.floor.iter().copied() {
        let below = p.offset(0, 0, -1);
        w.grid.set(below, Tile::solid(Material::WOOD));
        w.built_of.insert(below, Material::WOOD);
    }
    let after = room_stats(&w, &plan, 2).beauty;
    assert!(after > before + 0.29, "{before} -> {after}");
}

#[test]
fn an_unlit_lamp_gives_no_light() {
    let mut w = World::generate(1021, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.structures.clear();
    w.items.clear();
    let plan = plan_of(3, 2, 10);
    let at = TilePos::new(10, 20, 2);
    w.place_structure(StructureKind::Lamp, at, Material::AIR, false);
    assert_eq!(room_stats(&w, &plan, 2).light, 0.0);
    w.structures[0].active = true;
    assert_eq!(room_stats(&w, &plan, 2).light, 1.0);
}

#[test]
fn a_laid_upper_floor_and_a_figurine_each_raise_beauty() {
    use hf_sim::world::Tile;
    let mut w = World::generate(1021, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.structures.clear();
    w.items.clear();
    let plan = plan_of(3, 2, 10);
    let before = room_stats(&w, &plan, 2).beauty;
    w.drop_item(ItemKind::Figurine, 1, plan.floor[0]);
    let with_figurine = room_stats(&w, &plan, 2);
    assert!(
        (with_figurine.beauty - before - 0.3).abs() < 1e-4,
        "{before}"
    );
    assert_eq!(
        with_figurine.clutter, 0.0,
        "a figurine is not loose clutter"
    );
    // A floor laid on the room tile's own level (an upper storey) counts as laid.
    let mut upper = plan_of(3, 2, 10);
    for p in upper.floor.iter_mut() {
        *p = p.offset(0, 0, 1);
    }
    let bare = room_stats(&w, &upper, 2).beauty;
    for p in upper.floor.iter().copied() {
        w.grid.set(p, Tile::floor(Material::WOOD));
    }
    let laid = room_stats(&w, &upper, 2).beauty;
    assert!(laid > bare + 0.29, "{bare} -> {laid}");
}

#[test]
fn a_body_awaiting_rites_on_the_floor_counts_as_clutter() {
    use hf_sim::setup::WorldSetup;
    use hf_sim::systems::rites::RiteDue;
    let mut w = World::from_setup(&WorldSetup::new(1));
    w.structures.clear();
    w.items.clear();
    let plan = plan_of(3, 2, 10);
    assert_eq!(room_stats(&w, &plan, 2).clutter, 0.0);
    let dead = w.agents[0].id;
    let community = w.communities[0].id;
    w.agents[0].pos = plan.floor[0];
    let due = RiteDue {
        dead,
        community,
        since: w.meta.tick,
        death: hf_sim::ids::EventId(0),
        doer: None,
        away: false,
    };
    w.rites_due.push(due);
    assert_eq!(
        room_stats(&w, &plan, 2).clutter,
        0.25,
        "one body is a quarter of the clutter"
    );
    // A body that died away is not on this floor.
    w.rites_due[0].away = true;
    assert_eq!(room_stats(&w, &plan, 2).clutter, 0.0);
}
