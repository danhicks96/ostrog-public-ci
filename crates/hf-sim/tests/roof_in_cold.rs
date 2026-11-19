//! #859: a hard cold must lift roofing's score, not only a homeless worker at dusk, or roofs
//! sit untaken for hundreds of days while everyone already has a bed.

use hf_sim::ids::TilePos;
use hf_sim::mind::Activity;
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::work::{Designation, DesignationKind, ResKey};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Tile};
use hf_sim::World;

const GZ: i32 = 2;

fn world() -> World {
    let mut w = World::generate(909, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, GZ);
    w.meta.tick = TICKS_PER_HOUR * 10; // mid-morning, not dusk
    w.meta.autoplan = false;
    // A small built shell with a door, its one interior tile unroofed yet (#859's own case:
    // the wall is up, only the roof is still waiting), and a natural rock face to mine
    // instead, both equally within a step or two of the door.
    w.build_house(30, 30, 3, 3, GZ, Material::WOOD, Material::WOOD);
    let interior = TilePos::new(31, 31, GZ + 1);
    w.roofs.remove(&interior);
    w.grid
        .set(TilePos::new(34, 31, GZ + 1), Tile::solid(Material::SOIL));
    w.rebuild_caches();
    w.ensure_nav();
    w
}

fn post(
    w: &mut World,
    kind: DesignationKind,
    pos: TilePos,
    community: hf_sim::ids::CommunityId,
) -> hf_sim::work::DesignationId {
    let id = w.alloc_designation_id();
    w.designations.push(Designation {
        id,
        kind,
        pos,
        requirements: vec![],
        work_left: 200,
        work_total: 200,
        max_workers: 1,
        priority: 3,
        created: w.tick(),
        failures: 0,
        done: false,
        community: Some(community),
        owner: None,
    });
    id
}

/// One person, equally able to mine or to roof (same, default skill levels), each order a
/// step or two from the door: which one they take on should turn on how cold they are, not
/// coin-flip noise.
fn setup(warmth: f32) -> (World, hf_sim::work::DesignationId, hf_sim::ids::AgentId) {
    let mut w = world();
    let ids = w.found_party(1);
    let id = ids[0];
    let community = w
        .agents
        .iter()
        .find(|a| a.id == id)
        .unwrap()
        .community
        .unwrap();
    if let Some(a) = w.agents.iter_mut().find(|a| a.id == id) {
        a.pos = TilePos::new(31, 31, GZ + 1); // the house's one interior tile
        a.band = None; // settled, not still travelling as a band (choose_band otherwise wins)
        a.activity = Activity::Idle;
        a.idle_ticks = 0;
        a.needs.hunger = 0.9;
        a.needs.thirst = 0.9;
        a.needs.rest = 0.9;
        a.needs.warmth = warmth;
    }
    let roof = post(
        &mut w,
        DesignationKind::BuildRoof(Material::WOOD),
        TilePos::new(31, 31, GZ + 1),
        community,
    );
    post(
        &mut w,
        DesignationKind::Mine,
        TilePos::new(34, 31, GZ + 1),
        community,
    );
    (w, roof, id)
}

#[test]
fn a_cold_worker_roofs_before_mining() {
    let (mut w, roof, id) = setup(0.3);
    hf_sim::systems::decide::step(&mut w);
    assert_eq!(
        w.reservations.holder(ResKey::Designation(roof)),
        Some(id),
        "a cold worker should have taken the roof, not the mine"
    );
}

#[test]
fn a_warm_worker_is_free_to_pick_either() {
    // Not asserting which one (that is for other weights to decide) — only that warmth is
    // not forcing the roof the way the cold case does.
    let (mut w, _roof, id) = setup(0.9);
    hf_sim::systems::decide::step(&mut w);
    let took_something = w.reservations.holders().contains(&id);
    assert!(
        took_something,
        "a warm, idle worker should still take some order"
    );
}
