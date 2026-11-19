//! #292 item 6 (WP-08 slice 8.6): "why is nobody doing this?" A planned order's block line
//! names what stands in its way, from the same conditions the decision system applies.

use hf_sim::ids::TilePos;
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::mind::beliefs::{Source, Topic};
use hf_sim::work::{Designation, DesignationKind, Requirement, ResKey};
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

fn world() -> (World, Vec<hf_sim::ids::AgentId>) {
    let mut w = World::generate(2921, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(3);
    (w, ids)
}

fn order(w: &mut World, kind: DesignationKind, reqs: Vec<Requirement>) -> usize {
    let id = w.alloc_designation_id();
    let community = w.agents.iter().find_map(|a| a.community);
    w.designations.push(Designation {
        id,
        kind,
        pos: TilePos::new(34, 34, 2),
        requirements: reqs,
        work_left: 200,
        work_total: 200,
        max_workers: 1,
        priority: 3,
        created: w.tick(),
        failures: 0,
        done: false,
        community,
        owner: None,
    });
    w.designations.len() - 1
}

fn reason(w: &World, ix: usize) -> String {
    hf_sim::view::waiting::reason(w, &w.designations[ix])
}

#[test]
fn a_wall_with_no_stone_anywhere_says_so() {
    let (mut w, _) = world();
    w.items.retain(|it| {
        !(it.kind == ItemKind::StoneBlock && matches!(it.place, ItemPlace::Ground(_)))
    });
    let ix = order(
        &mut w,
        DesignationKind::BuildWall(Material::AIR),
        vec![Requirement {
            kind: ItemKind::StoneBlock,
            needed: 3,
            delivered: 0,
            material: Material::AIR,
            want: None,
            quality: 0.0,
        }],
    );
    let r = reason(&w, ix);
    assert_eq!(r, "waiting for 3 stone; none lies anywhere in the valley");
    // The block panel line carries it, not the debug name of the kind.
    let line = hf_sim::view::waiting::line(&w, &w.designations[ix]);
    assert_eq!(line, format!("Planned: wall ({r})"));
}

#[test]
fn a_single_missing_bed_frame_is_singular() {
    let (mut w, _) = world();
    w.items
        .retain(|it| !(it.kind == ItemKind::BedFrame && matches!(it.place, ItemPlace::Ground(_))));
    let ix = order(
        &mut w,
        DesignationKind::BuildStructure(hf_sim::structures::StructureKind::Bed, Material::AIR),
        vec![Requirement {
            kind: ItemKind::BedFrame,
            needed: 1,
            delivered: 0,
            material: Material::AIR,
            want: None,
            quality: 0.0,
        }],
    );
    let r = reason(&w, ix);
    assert_eq!(
        r,
        "waiting for 1 bed frame; none lies anywhere in the valley"
    );
}

#[test]
fn a_taken_order_names_who_holds_it_and_a_failed_one_is_set_aside() {
    let (mut w, ids) = world();
    let ix = order(&mut w, DesignationKind::Mine, vec![]);
    let did = w.designations[ix].id;
    let t = w.tick();
    assert!(w
        .reservations
        .acquire_all(ids[0], &[ResKey::Designation(did)], t + 1000));
    let name = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .name
        .clone();
    assert_eq!(reason(&w, ix), format!("{name} has taken it"));

    w.reservations.release_all(ids[0]);
    w.designations[ix].failures = 13;
    assert_eq!(reason(&w, ix), "set aside after 13 failed tries");
}

#[test]
fn nobody_can_when_every_hand_is_ruled_out() {
    let (mut w, ids) = world();
    let ix = order(&mut w, DesignationKind::Chop, vec![]);
    let t = w.tick();
    // Two will not fell a living tree; the third gave up on this one a moment ago.
    for &id in &ids[..2] {
        let a = w.agents.iter_mut().find(|a| a.id == id).unwrap();
        a.beliefs
            .hold(Topic::CutLivingTree, -1.0, 1.0, Source::Person(ids[2]), t);
        assert!(a.beliefs.lean(Topic::CutLivingTree) <= -0.4);
    }
    let did = w.designations[ix].id.0;
    let last = w.agents.iter_mut().find(|a| a.id == ids[2]).unwrap();
    last.work_cooldowns.insert(did, t + 10_000);
    assert_eq!(
        reason(&w, ix),
        "nobody can: 2 will not fell a living tree, 1 gave up on it lately"
    );
    // Lift the cooldown and the third could take it: the line says so instead.
    w.agents
        .iter_mut()
        .find(|a| a.id == ids[2])
        .unwrap()
        .work_cooldowns
        .clear();
    let r = reason(&w, ix);
    assert!(
        r == "1 could take it; other work weighs more for now" || r == "it waits for morning",
        "{r}"
    );
}

#[test]
fn the_waiting_list_names_old_untaken_orders_oldest_first() {
    let (mut w, ids) = world();
    // Far enough in that the orders can be backdated.
    w.meta.tick += 20_000;
    let now = w.tick();
    let a = order(&mut w, DesignationKind::Mine, vec![]);
    let b = order(&mut w, DesignationKind::Chop, vec![]);
    let c = order(&mut w, DesignationKind::Mine, vec![]);
    w.designations[a].created = now.saturating_sub(5_000);
    w.designations[b].created = now.saturating_sub(9_000);
    w.designations[c].created = now; // too fresh to list
                                     // One of the old ones is taken: not waiting.
    let held = w.designations[a].id;
    assert!(w
        .reservations
        .acquire_all(ids[0], &[ResKey::Designation(held)], now + 1000));
    let list = hf_sim::view::waiting::stuck(&w, 1_000, 10);
    assert_eq!(list.len(), 1, "{list:?}");
    assert_eq!(list[0].label, "felling");
    assert!(list[0].age >= 9_000);
    assert!(!list[0].reason.is_empty());
    // Released, it is listed too, after the older one.
    w.reservations.release_all(ids[0]);
    let list = hf_sim::view::waiting::stuck(&w, 1_000, 10);
    assert_eq!(
        list.iter().map(|s| s.label.as_str()).collect::<Vec<_>>(),
        vec!["felling", "mining"]
    );
    assert_eq!(
        hf_sim::view::waiting::stuck(&w, 1_000, 1).len(),
        1,
        "the limit holds"
    );
}
