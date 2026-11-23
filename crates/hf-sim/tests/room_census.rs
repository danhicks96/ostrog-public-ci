//! OST-1309: the room census counts a staged house correctly.
#[path = "../examples/room_census.rs"]
mod census;

use hf_sim::ids::TilePos;
use hf_sim::items::ItemKind;
use hf_sim::structures::StructureKind;
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

#[test]
fn census_counts_a_staged_house() {
    let mut w = World::generate(101901, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    w.found_party(4);
    let home = w.communities[0].home;
    let o = home.offset(-12, -12, 0);
    let plan = hf_sim::houses::old_plan(o);
    let at = |i: usize| plan.floor[i];
    w.communities[0].house_sites = vec![o];
    w.communities[0].house_plans = vec![plan.clone()];
    w.communities[0].house_stage = vec![3];
    for (kind, i) in [
        (StructureKind::Bed, 0),
        (StructureKind::Bed, 4),
        (StructureKind::Stool, 1),
        (StructureKind::Stool, 2),
        (StructureKind::Chair, 3),
        (StructureKind::Table, 5),
        (StructureKind::Chest, 6),
        (StructureKind::Lamp, 7),
        (StructureKind::Hearth, 8),
    ] {
        w.place_structure(kind, at(i), Material::WOOD, true);
    }
    // A bed and a stool outside the floor do not count.
    w.place_structure(
        StructureKind::Bed,
        o.offset(-3, -3, 0),
        Material::WOOD,
        true,
    );
    w.drop_item(ItemKind::Log, 5, at(9));
    w.drop_item(ItemKind::Meal, 2, at(10));
    w.drop_item(ItemKind::Log, 7, o.offset(-3, -3, 0));

    let rows = census::census(&w);
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert_eq!(r.floor, plan.floor.len());
    assert_eq!((r.beds, r.sleepers), (2, 0));
    assert_eq!((r.stools, r.chairs, r.tables, r.chests), (2, 1, 1, 1));
    assert_eq!((r.lamps, r.hearths), (1, 1));
    assert_eq!(r.fittings(), 9);
    assert_eq!(
        (r.stacks, r.units),
        (1, 5),
        "the meal is food, the log outside is not inside"
    );
    assert!(
        r.wall.is_none() && r.fine_stone.is_none(),
        "no walls were built"
    );

    let (med, lo, hi) = census::spread(vec![3.0, 1.0, 2.0]).unwrap();
    assert_eq!((med, lo, hi), (2.0, 1.0, 3.0));
}
