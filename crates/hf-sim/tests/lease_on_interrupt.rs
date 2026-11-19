//! #743: a cook taken off the work by something other than the work's own finish or abort (a
//! rite, an observance, a threat) kept the hearth leased for the rest of the plan's lease (two
//! game hours), so in a hungry valley the hearth was always held by someone not cooking, and
//! nobody cooked a meal all day (seed 2, day 400). Whoever is no longer at the work lets go.

use hf_sim::ids::TilePos;
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::mind::Activity;
use hf_sim::structures::StructureKind;
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::work::{Designation, DesignationKind, ResKey};
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

#[test]
fn a_cook_taken_off_the_work_lets_go_of_the_hearth() {
    let mut w = World::generate(7431, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 9;
    w.rebuild_caches();
    let ids = w.found_party(2);
    let hp = w.grid.surface(33, 30);
    w.place_structure(StructureKind::Hearth, hp, Material::STONE_BLOCK, false);
    let hearth = w
        .structures
        .iter()
        .find(|s| s.kind == StructureKind::Hearth)
        .unwrap()
        .id;
    let raw_at = w.grid.surface(31, 31);
    w.create_item(ItemKind::Berries, 6, ItemPlace::Ground(raw_at), 0.6, "test");
    w.ensure_nav();
    let did = w.alloc_designation_id();
    let tick = w.tick();
    let community = w.agents.iter().find(|a| a.id == ids[0]).unwrap().community;
    w.designations.push(Designation {
        id: did,
        kind: DesignationKind::Cook,
        pos: hp,
        requirements: vec![],
        work_left: 900,
        work_total: 900,
        max_workers: 1,
        priority: 7,
        created: tick,
        failures: 0,
        done: false,
        community,
        owner: None,
    });
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    // They know the berries, plan the cooking and take its leases, as decide does.
    let raw = w
        .items
        .iter()
        .find(|it| it.kind == ItemKind::Berries)
        .unwrap()
        .id;
    w.agents[i].knowledge.note(
        hf_sim::mind::knowledge::Known::Item(raw),
        raw_at,
        tick,
        ItemKind::Berries as u16,
    );
    let d = w.designations.iter().find(|d| d.id == did).unwrap().clone();
    let p = hf_sim::work::planner::plan_for(&w, ids[0], &d).expect("fixture: the cooking plans");
    assert!(w
        .reservations
        .acquire_all(ids[0], &p.keys, tick + p.estimate as u64 + 600));
    w.agents[i].activity = Activity::Working { plan: p.plan };
    w.agents[i].activity_ticks = 1000;
    assert_eq!(w.reservations.holder(ResKey::Station(hearth)), Some(ids[0]));
    // Taken off the work as a rite or an alarm takes someone: the activity replaced, no abort.
    w.agents[i].activity = Activity::Loitering { ticks_left: 400 };
    for _ in 0..hf_sim::systems::work::LEASE_SWEEP + 1 {
        w.step();
    }
    assert_ne!(
        w.reservations.holder(ResKey::Station(hearth)),
        Some(ids[0]),
        "the cook taken off the work still holds the hearth, so nobody else can cook"
    );
    // Someone still at the work keeps theirs: plan and lease again, step a sweep, still held.
    let d = w.designations.iter().find(|d| d.id == did).unwrap().clone();
    if let Some(p) = hf_sim::work::planner::plan_for(&w, ids[0], &d) {
        let t = w.tick();
        assert!(w
            .reservations
            .acquire_all(ids[0], &p.keys, t + p.estimate as u64 + 600));
        w.agents[i].activity = Activity::Working { plan: p.plan };
        w.agents[i].needs.hunger = 1.0;
        for _ in 0..hf_sim::systems::work::LEASE_SWEEP + 1 {
            w.step();
            if !matches!(w.agents[i].activity, Activity::Working { .. }) {
                break;
            }
        }
        if matches!(w.agents[i].activity, Activity::Working { .. }) {
            assert_eq!(
                w.reservations.holder(ResKey::Station(hearth)),
                Some(ids[0]),
                "a cook still at work lost the hearth"
            );
        }
    }
}
