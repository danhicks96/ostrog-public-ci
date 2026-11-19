//! #483 engine-primitive scenarios: direct orders build upper rooms through material-backed
//! work, stairs connect them, and inhabitants use them. Production order proposal is deliberately
//! outside this slice. The fixtures place only the lower room; upper tiles and the bed are built.

use hf_sim::ids::TilePos;
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::mind::{Activity, Known};
use hf_sim::structures::StructureKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::DesignationKind;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

fn meadow() -> World {
    let mut w = World::generate(48301, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(41, 45, 2);
    w.meta.tick = 7 * TICKS_PER_HOUR;
    w.rebuild_caches();
    w.found_party(1);
    w.meta.autoplan = false;
    w.designations.clear();
    let outside = TilePos::new(41, 45, 2);
    w.agents[0].pos = outside;
    w.agents[0].from = outside;
    w.agents[0].needs.hunger = 1.0;
    w.agents[0].needs.thirst = 1.0;
    w.agents[0].needs.rest = 1.0;
    w.agents[0].needs.warmth = 1.0;
    w.agents[0].needs.social = 1.0;
    w.ensure_nav();
    w
}

fn order(w: &mut World, kind: DesignationKind, pos: TilePos, priority: u8) {
    let d = hf_sim::systems::settlement::make(w, kind, pos, priority);
    w.designations.push(d);
}

fn known_stack(w: &mut World, kind: ItemKind, count: u32, at: TilePos) {
    w.create_item_of(
        kind,
        Material::OAK,
        count,
        ItemPlace::Ground(at),
        0.6,
        "test",
    );
    let tick = w.meta.tick;
    let ids: Vec<_> = w
        .items
        .iter()
        .filter(|it| it.kind == kind && it.place == ItemPlace::Ground(at))
        .map(|it| it.id)
        .collect();
    for id in ids {
        w.agents[0]
            .knowledge
            .note(Known::Item(id), at, tick, kind as u16);
    }
}

#[test]
fn an_upper_floor_needs_support_from_walls_below() {
    let mut w = meadow();
    let at = TilePos::new(44, 44, 3);
    assert!(!hf_sim::building::floor_supported(&w, at));
    order(&mut w, DesignationKind::BuildFloor(Material::WOOD), at, 9);
    w.designations[0].requirements.clear();
    assert!(
        hf_sim::work::planner::plan_for(&w, w.agents[0].id, &w.designations[0]).is_none(),
        "an unsupported floor is not offered as workable"
    );

    w.grid.set(at.offset(1, 0, -1), Tile::solid(Material::WOOD));
    assert!(hf_sim::building::floor_supported(&w, at));
}

#[test]
fn stairs_supported_floors_and_a_bed_are_built_then_used_above() {
    let mut w = meadow();
    let outside = TilePos::new(41, 45, 2);
    let store = TilePos::new(41, 44, 2);
    let stair = TilePos::new(40, 40, 2);
    let upper_stair = stair.offset(0, 0, 1);
    let bed = TilePos::new(42, 42, 3);

    // A complete lower room with a door; the upper room does not exist yet.
    for y in 39..=43 {
        for x in 39..=43 {
            if x == 39 || x == 43 || y == 39 || y == 43 {
                let at = TilePos::new(x, y, 2);
                let mut tile = Tile::solid(Material::WOOD);
                if at == TilePos::new(41, 43, 2) {
                    tile.flags = TileFlags(TileFlags::DOOR);
                }
                w.grid.set(at, tile);
            }
        }
    }
    w.roofs.insert(
        stair,
        hf_sim::building::Roof {
            material: Material::WOOD,
            quality: 0.6,
            built: 0,
            snow: 0.0,
        },
    );
    w.rebuild_caches();
    w.ensure_nav();

    let floors: Vec<TilePos> = (40..=42)
        .flat_map(|y| (40..=42).map(move |x| TilePos::new(x, y, 3)))
        .filter(|p| *p != upper_stair)
        .collect();
    order(
        &mut w,
        DesignationKind::BuildStairs(Material::WOOD),
        stair,
        9,
    );
    for p in &floors {
        order(&mut w, DesignationKind::BuildFloor(Material::WOOD), *p, 8);
    }
    order(
        &mut w,
        DesignationKind::BuildStructure(StructureKind::Bed, Material::WOOD),
        bed,
        7,
    );
    known_stack(&mut w, ItemKind::Plank, 2 + floors.len() as u32, store);
    known_stack(&mut w, ItemKind::BedFrame, 1, store);

    let mut saw_stairs = false;
    let mut saw_floor = false;
    let mut saw_bed = false;
    for _ in 0..TICKS_PER_DAY {
        w.step();
        if w.grid.get(stair).shape == Shape::StairsUp {
            saw_stairs = true;
        }
        if floors.iter().any(|p| w.grid.get(*p).shape == Shape::Floor) {
            assert!(saw_stairs, "the connection is built before the upper floor");
            saw_floor = true;
        }
        if w.structures.iter().any(|s| s.pos == bed) {
            assert!(
                saw_floor,
                "the bed is set only after there is floor to reach"
            );
            saw_bed = true;
        }
        if saw_bed && w.designations.iter().all(|d| d.done) {
            break;
        }
    }
    let open: Vec<String> = w
        .designations
        .iter()
        .filter(|d| !d.done)
        .map(|d| {
            format!(
                "{:?} at {:?}: failures {}, work {}, missing {:?}",
                d.kind,
                d.pos,
                d.failures,
                d.work_left,
                d.missing().collect::<Vec<_>>()
            )
        })
        .collect();
    let stock: Vec<String> = w
        .items
        .iter()
        .filter(|it| matches!(it.kind, ItemKind::Plank | ItemKind::BedFrame))
        .map(|it| {
            format!(
                "{:?} {} at {:?}: known {}",
                it.kind,
                it.count,
                it.place,
                w.agents[0].knowledge.get(Known::Item(it.id)).is_some()
            )
        })
        .collect();
    assert!(
        saw_stairs && saw_floor && saw_bed,
        "the upper room was completed: stairs {saw_stairs}, floor {saw_floor}, bed {saw_bed}; open {open:?}; stock {stock:?}; agent {:?} at {:?}",
        w.agents[0].activity,
        w.agents[0].pos
    );
    assert!(w.designations.iter().all(|d| d.done));
    assert_eq!(w.grid.get(stair).shape, Shape::StairsUp);
    assert_eq!(w.grid.get(upper_stair).shape, Shape::StairsDown);
    assert!(!w.roofs.contains_key(&stair), "the stair opens the ceiling");
    assert!(floors.iter().all(|p| w.grid.get(*p).shape == Shape::Floor));
    assert_eq!(w.made_of(stair), Some(Material::OAK));
    assert_eq!(w.made_of(upper_stair), Some(Material::OAK));
    assert!(floors.iter().all(|p| w.made_of(*p) == Some(Material::OAK)));
    assert!(w
        .items
        .iter()
        .all(|it| it.kind != ItemKind::Plank && it.kind != ItemKind::BedFrame));

    w.rebuild_caches();
    w.ensure_nav();
    assert!(
        w.nav.as_ref().unwrap().reach(outside, bed),
        "the upper bed is reachable"
    );
    let bed_id = w.structures.iter().find(|s| s.pos == bed).unwrap().id;
    w.agents[0].pos = outside;
    w.agents[0].from = outside;
    w.agents[0].path.clear();
    w.agents[0].move_total = 0;
    w.agents[0].needs.rest = 0.1;
    w.agents[0].knowledge.note(
        Known::Structure(bed_id),
        bed,
        w.meta.tick,
        StructureKind::Bed as u16,
    );
    w.meta.tick = (w.meta.tick / TICKS_PER_DAY) * TICKS_PER_DAY + 23 * TICKS_PER_HOUR;
    let mut slept_upstairs = false;
    for _ in 0..TICKS_PER_DAY / 4 {
        w.step();
        if matches!(w.agents[0].activity, Activity::Sleeping { bed: Some(id), .. } if id == bed_id)
        {
            slept_upstairs = true;
            assert_eq!(w.agents[0].pos, bed);
            break;
        }
    }
    assert!(
        slept_upstairs,
        "the settler climbed to and used the upper bed"
    );
}

fn one_flight() -> (World, TilePos, TilePos, TilePos) {
    let mut w = meadow();
    let lower = TilePos::new(40, 40, 2);
    let middle = lower.offset(0, 0, 1);
    let top = lower.offset(0, 0, 2);
    w.grid.set(
        lower,
        Tile {
            material: Material::WOOD,
            shape: Shape::StairsUp,
            flags: TileFlags::NONE,
        },
    );
    w.grid.set(
        middle,
        Tile {
            material: Material::WOOD,
            shape: Shape::StairsDown,
            flags: TileFlags::NONE,
        },
    );
    w.rebuild_caches();
    w.ensure_nav();
    (w, lower, middle, top)
}

#[test]
fn a_second_flight_preserves_the_way_down() {
    let (mut w, lower, middle, top) = one_flight();
    order(
        &mut w,
        DesignationKind::BuildStairs(Material::WOOD),
        middle,
        9,
    );
    known_stack(&mut w, ItemKind::Plank, 2, TilePos::new(41, 45, 2));
    for _ in 0..6 * TICKS_PER_HOUR {
        w.step();
        if w.designations.iter().all(|d| d.done) {
            break;
        }
    }
    assert!(w.designations.iter().all(|d| d.done));
    assert_eq!(w.grid.get(lower).shape, Shape::StairsUp);
    assert_eq!(w.grid.get(middle).shape, Shape::StairsBoth);
    assert_eq!(w.grid.get(top).shape, Shape::StairsDown);
    w.rebuild_caches();
    w.ensure_nav();
    assert!(w.nav.as_ref().unwrap().reach(lower, top));
    assert!(w.nav.as_ref().unwrap().reach(top, lower));
}

#[test]
fn a_floor_order_does_not_erase_a_stair_tile() {
    let (mut w, _, middle, _) = one_flight();
    order(
        &mut w,
        DesignationKind::BuildFloor(Material::WOOD),
        middle,
        9,
    );
    w.designations[0].requirements.clear();
    assert!(hf_sim::work::planner::plan_for(&w, w.agents[0].id, &w.designations[0]).is_none());
    w.run(TICKS_PER_HOUR);
    assert!(!w.designations[0].done);
    assert_eq!(w.grid.get(middle).shape, Shape::StairsDown);
}

#[test]
fn upper_wood_materials_drive_provenance_fuel_and_burn_out() {
    let mut w = meadow();
    let at = TilePos::new(44, 44, 3);
    w.grid.set(at, Tile::floor(Material::WOOD));
    w.built_of.insert(at, Material::OAK);
    assert_eq!(w.made_of(at), Some(Material::OAK));
    let expected = (800.0 * w.grid.materials.get(Material::OAK).density
        / w.grid.materials.get(Material::WOOD).density) as u32;
    assert_eq!(hf_sim::systems::hazards::fuel_at(&w, at), expected);

    w.agents.clear();
    assert!(w.ignite(at, "in a test"));
    for _ in 0..TICKS_PER_DAY {
        w.step();
        if w.fires.is_empty() {
            break;
        }
    }
    assert_eq!(w.grid.get(at).shape, Shape::Open);
    assert_eq!(w.made_of(at), None);
}

#[test]
fn a_ground_floor_still_builds_on_the_tile_below() {
    let mut w = meadow();
    let at = TilePos::new(44, 44, 2);
    let below = at.offset(0, 0, -1);
    order(&mut w, DesignationKind::BuildFloor(Material::WOOD), at, 9);
    known_stack(&mut w, ItemKind::Plank, 1, TilePos::new(41, 45, 2));
    for _ in 0..6 * TICKS_PER_HOUR {
        w.step();
        if w.designations.iter().all(|d| d.done) {
            break;
        }
    }
    assert!(w.designations.iter().all(|d| d.done));
    assert_eq!(w.grid.get(below).shape, Shape::Wall);
    assert_eq!(w.grid.get(below).material, Material::WOOD);
    assert_eq!(w.made_of(below), Some(Material::OAK));
    assert_eq!(w.grid.get(at).shape, Shape::Open);
}
