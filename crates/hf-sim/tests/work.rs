//! Phase 4 gates: prerequisite chains, contention without deadlock, lease expiry,
//! interrupted plans, vanished targets, and a zero-drift ledger.

use hf_sim::agents::Species;
use hf_sim::economy::live_counts;
use hf_sim::ids::TilePos;
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::mind::Activity;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::structures::StructureKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::{Designation, DesignationKind, Requirement, ResKey};
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

fn meadow(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    // Start mid-morning: nobody takes up work at night.
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.meta.autoplan = false;
    w.rebuild_caches();
    for i in 0..40 {
        w.plants.push(Plant::new(
            TilePos::new(6 + (i * 7) % 50, 6 + (i * 11) % 50, 2),
            PlantKind::BerryBush,
            1.0,
            i as u32,
        ));
    }
    w
}

fn post(
    w: &mut World,
    kind: DesignationKind,
    pos: TilePos,
    reqs: Vec<(ItemKind, u32)>,
    work: u32,
) -> hf_sim::work::DesignationId {
    let id = w.alloc_designation_id();
    w.designations.push(Designation {
        id,
        kind,
        pos,
        requirements: reqs
            .into_iter()
            .map(|(kind, needed)| Requirement {
                kind,
                needed,
                delivered: 0,
                material: Material::AIR,
                want: None,
                quality: 0.0,
            })
            .collect(),
        work_left: work,
        work_total: work,
        max_workers: 1,
        priority: 9,
        created: w.tick(),
        failures: 0,
        done: false,
        community: None,
        owner: None,
    });
    id
}

fn ledger_ok(w: &World) -> Vec<String> {
    let live = live_counts(&w.items);
    w.ledger
        .expected()
        .iter()
        .filter(|(k, v)| live.get(*k).copied().unwrap_or(0) != **v)
        .map(|(k, v)| {
            format!(
                "{k}: live {} expected {v}",
                live.get(k).copied().unwrap_or(0)
            )
        })
        .collect()
}

#[test]
fn low_spirits_work_slower() {
    // #946: mood and stress never touched work speed, so a grieving or breaking-down
    // person built exactly as fast as a contented one.
    let miserable = hf_sim::systems::work::spirit(-0.8, 0.6);
    let content = hf_sim::systems::work::spirit(0.8, 0.0);
    assert!(
        content >= miserable * 1.25,
        "the contented one works at least 1.25x the speed: content {content}, miserable {miserable}"
    );
}

#[test]
fn prereq_chain_cook_from_harvest() {
    // Grain in a field + a hearth: people must harvest, haul, and cook before meals exist.
    let mut w = meadow(31);
    w.meta.autoplan = true; // the community planner posts harvest, haul and cook orders
    let cid = w.found_community(TilePos::new(30, 30, 2));
    w.arrive_party(6, TilePos::new(30, 28, 2), Some(cid));
    for i in 0..6 {
        w.place_structure(
            StructureKind::Stockpile,
            TilePos::new(34 + i, 34, 2),
            Material::SOIL,
            false,
        );
    }
    w.place_structure(
        StructureKind::Hearth,
        TilePos::new(30, 26, 2),
        Material::STONE_BLOCK,
        true,
    );
    w.communities[0].field_site = Some(TilePos::new(38, 28, 2));
    for y in 0..4 {
        for x in 0..6 {
            w.plants.push(Plant::new(
                TilePos::new(38 + x, 28 + y, 2),
                PlantKind::Grain,
                1.0,
                1,
            ));
        }
    }
    let mut cooked = 0;
    let mut harvested = 0;
    for _ in 0..TICKS_PER_DAY * 2 {
        w.step();
        for e in &w.last_events {
            match e.kind {
                hf_sim::events::EventKind::Cooked { .. } => cooked += 1,
                hf_sim::events::EventKind::Harvested { .. } => harvested += 1,
                _ => {}
            }
        }
    }
    assert!(harvested >= 6, "crops harvested: {harvested}");
    assert!(cooked >= 3, "meals cooked from the harvest: {cooked}");
    assert!(w.items.iter().any(|it| it.kind == ItemKind::Meal) || cooked > 0);
    assert!(ledger_ok(&w).is_empty(), "ledger drift {:?}", ledger_ok(&w));
}

#[test]
fn cook_turns_raw_meat_into_meals() {
    // Issue #91: the planner sent the cook for meat first, but the cooking itself only took
    // berries, roots and grain, so the meat was carried forever and no meal came of it.
    let mut w = meadow(35);
    let hearth = TilePos::new(30, 26, 2);
    w.place_structure(StructureKind::Hearth, hearth, Material::STONE_BLOCK, true);
    let cook = w.spawn_agent(Species::Person, TilePos::new(30, 30, 2));
    w.drop_item(ItemKind::RawMeat, 4, TilePos::new(32, 30, 2));
    for _ in 0..2 {
        post(&mut w, DesignationKind::Cook, hearth, vec![], 200);
    }
    let mut cooked = 0;
    for _ in 0..TICKS_PER_HOUR * 6 {
        // Fed, so the meat is for cooking, not for eating raw.
        if let Some(a) = w.agents.iter_mut().find(|a| a.id == cook) {
            a.needs.hunger = 1.0;
        }
        w.step();
        cooked += w
            .last_events
            .iter()
            .filter(|e| matches!(e.kind, hf_sim::events::EventKind::Cooked { .. }))
            .count();
    }
    assert!(cooked >= 2, "meals cooked from meat: {cooked}");
    assert!(w.items.iter().any(|it| it.kind == ItemKind::Meal));
    let carried: u32 = w
        .items
        .iter()
        .filter(|it| it.kind == ItemKind::RawMeat && it.place == ItemPlace::Carried(cook))
        .map(|it| it.count)
        .sum();
    assert_eq!(carried, 0, "the cook is not left carrying meat");
    assert!(ledger_ok(&w).is_empty(), "ledger drift {:?}", ledger_ok(&w));
}

#[test]
fn contention_no_deadlock() {
    // 50 people, 5 log stacks, 30 wall sites: everyone competes for the same few logs.
    let mut w = meadow(32);
    for i in 0..50 {
        let p = TilePos::new(10 + (i % 25), 10 + (i / 25) * 2, 2);
        w.spawn_agent(Species::Person, p);
    }
    for i in 0..5 {
        w.drop_item(ItemKind::Log, 6, TilePos::new(20 + i * 2, 20, 2));
    }
    for i in 0..30 {
        post(
            &mut w,
            DesignationKind::BuildWall(Material::WOOD),
            TilePos::new(10 + i, 40, 2),
            vec![(ItemKind::Log, 1)],
            20 * 20,
        );
    }
    // Make sure everyone knows about the logs (they are in sight anyway).
    w.run(TICKS_PER_HOUR * 6);
    let built = w.designations.iter().filter(|d| d.done).count()
        + (10..40)
            .filter(|x| w.grid.get(TilePos::new(*x, 40, 2)).is_solid())
            .count();
    assert!(built >= 20, "most walls get built from 30 logs: {built}");
    let stalled = w
        .agents
        .iter()
        .filter(|a| {
            matches!(a.activity, Activity::Working { .. }) && a.activity_ticks > 20 * 60 * 2
        })
        .count();
    assert_eq!(stalled, 0, "nobody stuck in a plan for two hours");
    // No lease is held by someone who is not working.
    for (k, l) in &w.reservations.leases {
        let a = w.agents.iter().find(|a| a.id == l.holder).unwrap();
        assert!(
            matches!(a.activity, Activity::Working { .. }),
            "{:?} held by idle {}",
            k,
            a.name
        );
    }
    assert!(ledger_ok(&w).is_empty(), "ledger drift {:?}", ledger_ok(&w));
}

#[test]
fn lease_expiry_sweep() {
    let mut w = meadow(33);
    let id = w.spawn_agent(Species::Person, TilePos::new(30, 30, 2));
    let did = post(
        &mut w,
        DesignationKind::Chop,
        TilePos::new(40, 40, 2),
        vec![],
        100,
    );
    // A stale lease from a phantom holder expires and is swept.
    let phantom = hf_sim::ids::AgentId(999);
    assert!(w
        .reservations
        .acquire_all(phantom, &[ResKey::Designation(did)], w.tick() + 100));
    w.run(200);
    assert!(
        w.reservations.holder(ResKey::Designation(did)) != Some(phantom),
        "phantom lease swept"
    );
    assert!(w.reservations.expired_total >= 1);
    let _ = id;
}

#[test]
fn interrupted_plan_releases_and_keeps_items() {
    let mut w = meadow(34);
    let id = w.spawn_agent(Species::Person, TilePos::new(30, 30, 2));
    let idx = w.agents.iter().position(|a| a.id == id).unwrap();
    w.agents[idx].needs.hunger = 1.0;
    w.agents[idx].needs.rest = 1.0;
    w.drop_item(ItemKind::Log, 3, TilePos::new(32, 30, 2));
    post(
        &mut w,
        DesignationKind::BuildWall(Material::WOOD),
        TilePos::new(45, 30, 2),
        vec![(ItemKind::Log, 1)],
        20 * 20,
    );
    // Let them pick the log up and start walking.
    let mut carrying_seen = false;
    for _ in 0..20 * 60 {
        w.step();
        if w.items.iter().any(|it| it.place == ItemPlace::Carried(id)) {
            carrying_seen = true;
            break;
        }
    }
    assert!(carrying_seen, "picked up a log");
    // Wall off the site so the path breaks.
    for y in 25..36 {
        w.grid.set(
            TilePos::new(40, y, 2),
            hf_sim::world::Tile::solid(Material::STONE_BLOCK),
        );
    }
    w.run(20 * 60 * 2);
    assert!(
        w.reservations.held_by(id).is_empty()
            || matches!(w.agents[idx].activity, Activity::Working { .. }),
        "no leases held after aborting"
    );
    let logs: u32 = w
        .items
        .iter()
        .filter(|it| it.kind == ItemKind::Log)
        .map(|it| it.count)
        .sum();
    let delivered: u32 = w
        .designations
        .iter()
        .flat_map(|d| d.requirements.iter())
        .map(|r| r.delivered)
        .sum();
    // A wall that stands used its log (C62, #162: a finished order keeps no delivered count).
    let built = (w.grid.get(TilePos::new(45, 30, 2)).shape == hf_sim::world::Shape::Wall) as u32;
    assert_eq!(
        logs + delivered + built,
        3,
        "no log vanished when the plan was abandoned (live {logs}, delivered {delivered}, built {built})"
    );
    assert!(ledger_ok(&w).is_empty(), "ledger drift {:?}", ledger_ok(&w));
}

#[test]
fn target_vanishes_recovers() {
    let mut w = meadow(35);
    let id = w.spawn_agent(Species::Person, TilePos::new(30, 30, 2));
    let idx = w.agents.iter().position(|a| a.id == id).unwrap();
    w.agents[idx].needs.hunger = 1.0;
    w.agents[idx].needs.rest = 1.0;
    let tree = TilePos::new(44, 30, 2);
    w.plants.push(Plant::new(tree, PlantKind::Tree, 1.0, 1));
    post(&mut w, DesignationKind::Chop, tree, vec![], 25 * 20);
    for _ in 0..200 {
        w.step();
        if matches!(w.agents[idx].activity, Activity::Working { .. }) {
            break;
        }
    }
    assert!(
        matches!(w.agents[idx].activity, Activity::Working { .. }),
        "went to fell the tree"
    );
    // The tree is gone before they arrive.
    w.plants.retain(|p| p.pos != tree);
    w.run(20 * 60 * 2);
    assert!(
        !matches!(w.agents[idx].activity, Activity::Working { .. })
            || w.agents[idx].activity_ticks < 20 * 60,
        "did not stay stuck on a vanished tree"
    );
    assert!(
        w.reservations.held_by(id).is_empty() || !matches!(w.agents[idx].activity, Activity::Idle)
    );
}

#[test]
fn ledger_zero_drift_default_world() {
    let mut w = World::generate(5, GenParams::default());
    w.found_party(8);
    w.run(TICKS_PER_DAY * 3);
    let drift = ledger_ok(&w);
    assert!(drift.is_empty(), "ledger drift after three days: {drift:?}");
}
