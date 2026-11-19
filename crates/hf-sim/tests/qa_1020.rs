//! QA round 2026-10-20 (#158 to #177): one guard per fix, each failing without it.

use hf_sim::ids::{AgentId, TilePos};
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::save::{decode, encode, snapshot::Snapshot};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::{Designation, DesignationKind, Requirement};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile};
use hf_sim::World;

fn meadow(seed: u64, people: usize) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    w.found_party(people);
    w.ensure_nav();
    w
}

#[test]
fn new_clothes_for_a_high_id_do_not_overflow() {
    // #161: the colour was a u8 add and overflowed for ids from 245 (debug and test builds).
    let mut w = meadow(16101, 2);
    let i = w.agents.iter().position(|a| a.community.is_some()).unwrap();
    w.agents[i].id = AgentId(250);
    w.agents[i].clothes_wear = 0.1;
    let home = w.communities[0].home;
    w.create_item(
        ItemKind::Clothes,
        2,
        ItemPlace::Ground(home.offset(1, 0, 0)),
        0.95,
        "test",
    );
    w.meta.tick = (w.meta.tick / TICKS_PER_DAY + 1) * TICKS_PER_DAY + TICKS_PER_HOUR * 5;
    hf_sim::systems::life::step(&mut w);
    assert!(
        w.agents[i].clothes_wear > 0.25,
        "they put on the new clothes"
    );
}

#[test]
fn a_house_open_to_the_sky_stays_open_after_a_load() {
    // #159: a save with no roof at the moment got every walled house roofed on load.
    let mut w = meadow(15901, 3);
    let o = TilePos::new(36, 24, 2);
    w.build_house(o.x, o.y, 7, 6, 1, Material::WOOD, Material::WOOD);
    w.roofs.clear();
    w.communities[0].house_sites.push(o);
    let back = decode(&encode(&Snapshot::capture(&w)).unwrap())
        .unwrap()
        .restore();
    assert!(
        back.roofs.is_empty(),
        "no roof appears from nowhere: {}",
        back.roofs.len()
    );
}

#[test]
fn a_floor_over_an_open_tile_buries_nothing() {
    // #174: finishing a floor turned the tile below into rock, whatever was there.
    let mut w = meadow(17401, 1);
    let pos = TilePos::new(40, 40, 2);
    let below = pos.offset(0, 0, -1);
    w.grid.set(below, Tile::AIR);
    w.rebuild_caches();
    let id = w.alloc_designation_id();
    w.designations.push(Designation {
        id,
        kind: DesignationKind::BuildFloor(Material::WOOD),
        pos,
        requirements: vec![Requirement {
            kind: ItemKind::Log,
            needed: 1,
            delivered: 1,
            material: Material::OAK,
            want: None,
            quality: 0.0,
        }],
        work_left: 20,
        work_total: 20,
        max_workers: 1,
        priority: 9,
        created: w.tick(),
        failures: 0,
        done: false,
        community: None,
        owner: None,
    });
    let _ = id;
    w.meta.autoplan = false;
    for a in w.agents.iter_mut() {
        a.needs.hunger = 1.0;
        a.needs.rest = 1.0;
    }
    w.run(TICKS_PER_HOUR * 6);
    assert!(
        w.designations
            .iter()
            .any(|d| d.kind == DesignationKind::BuildFloor(Material::WOOD) && !d.done),
        "without walls below to support it, the upper-floor order waits"
    );
    assert_ne!(
        w.grid.get(below).shape,
        Shape::Wall,
        "the open tile below stays open"
    );
}

#[test]
fn a_game_saved_on_the_tick_of_a_death_goes_on_as_it_would_have() {
    // #158: the stages that act on a death (the tool let go, an heirloom) read the tick's
    // events, which a save did not keep.
    let mut w = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(15801));
    for _ in 0..TICKS_PER_HOUR {
        w.step();
    }
    // Someone holding a tool.
    let id = w
        .agents
        .iter()
        .find(|a| a.alive && a.community.is_some())
        .unwrap()
        .id;
    let tool = w.create_item(ItemKind::Tool, 1, ItemPlace::Held(id), 0.6, "test");
    w.agents.iter_mut().find(|a| a.id == id).unwrap().tool = Some(tool);
    // They die on this tick, and the tick's events are flushed, as at the end of a step.
    hf_sim::systems::life::die(&mut w, id, "starvation".into());
    hf_sim::systems::flush::step(&mut w);
    let mut continuous = w.clone();
    let mut resumed = decode(&encode(&Snapshot::capture(&w)).unwrap())
        .unwrap()
        .restore();
    continuous.step();
    resumed.step();
    let held = |w: &World| {
        w.items
            .iter()
            .filter(|it| it.place == ItemPlace::Held(id))
            .count()
    };
    let tool = |w: &World| w.agents.iter().find(|a| a.id == id).unwrap().tool;
    assert_eq!(
        tool(&resumed),
        tool(&continuous),
        "the tool as it would have been"
    );
    assert_eq!(
        held(&resumed),
        held(&continuous),
        "what they held as it would have been"
    );
}

#[test]
fn a_column_off_the_map_reads_as_its_edge() {
    // #177.1: neighbours of an edge tile read another row, or past the end.
    let w = meadow(17701, 1);
    let (wd, ht) = (w.grid.width, w.grid.height);
    assert_eq!(w.grid.ground_z(wd, 5), w.grid.ground_z(wd - 1, 5));
    assert_eq!(w.grid.ground_z(-1, ht), w.grid.ground_z(0, ht - 1));
}

#[test]
fn every_band_that_has_gone_is_removed_in_one_step() {
    // #177.3: removing one band shifted the next into its place and returned, skipping it.
    let mut w = meadow(17703, 4);
    for k in 0..2u32 {
        w.bands.push(hf_sim::threat::Band {
            id: k,
            site: 0,
            site_name: "Ashby".into(),
            members: Vec::new(),
            size: 3,
            camp: TilePos::new(50, 10 + k as i32 * 5, 2),
            state: hf_sim::threat::BandState::Camped(w.meta.tick),
            formed: w.meta.tick,
            want: 18,
            taken: 0,
            lost: 0,
            sighted: true,
            welcomed: false,
            pursuits: Vec::new(),
            raiders: Vec::new(),
            envoys: Vec::new(),
            eaten: 0,
            night: 0,
            night_taken: 0,
            remembered: false,
            seen: Vec::new(),
            heard: Vec::new(),
            host: false,
            captives: Vec::new(),
        });
    }
    let step = hf_sim::threat::BAND_STEP;
    w.meta.tick = (w.meta.tick / step + 1) * step + 13;
    hf_sim::threat::step(&mut w);
    assert!(w.bands.is_empty(), "{} bands left", w.bands.len());
}

#[test]
fn food_carried_in_hand_rots() {
    // #177.4: only food on the ground rotted.
    let mut w = meadow(17704, 1);
    let who = w.agents.iter().find(|a| a.community.is_some()).unwrap().id;
    let meat = w.create_item(ItemKind::RawMeat, 4, ItemPlace::Carried(who), 0.5, "test");
    w.items.iter_mut().find(|it| it.id == meat).unwrap().rot = 0.999;
    w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR + 37;
    hf_sim::systems::spoil::step(&mut w);
    let left: u32 = w
        .items
        .iter()
        .filter(|it| it.id == meat)
        .map(|it| it.count)
        .sum();
    assert!(left < 4, "some of it rotted in hand: {left} of 4 left");
}

#[test]
fn an_ore_face_is_no_quarry_for_blocks() {
    // #177.8: the face search for stone took ore faces too.
    let mut w = meadow(17708, 2);
    let home = w.communities[0].home;
    for y in home.y..home.y + 3 {
        w.grid.set(
            TilePos::new(home.x + 6, y, home.z),
            Tile::solid(Material::IRON_ORE),
        );
    }
    w.rebuild_caches();
    w.ensure_nav();
    assert!(
        hf_sim::systems::settlement::nearest_rock_face(&w, home, 10).is_none(),
        "an iron face is not a stone face"
    );
}
