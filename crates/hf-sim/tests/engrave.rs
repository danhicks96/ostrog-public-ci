//! #1395: the last smith dies and the craft dies with them. When a person is the only one
//! left who knows a craft their own community uses, and is old enough it could die with them
//! tonight, they carve the way of it instead of an event (D37).

use hf_sim::ids::TilePos;
use hf_sim::mind::techniques::{Learned, Technique};
use hf_sim::mind::Skill;
use hf_sim::structures::StructureKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR, TICKS_PER_YEAR};
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material as M;
use hf_sim::World;

fn village(seed: u64, people: usize) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.build_house(36, 24, 7, 6, 1, M::STONE_BLOCK, M::STONE_BLOCK);
    w.rebuild_caches();
    let ids = w.found_party(people);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    for a in w.agents.iter_mut() {
        a.techniques
            .list
            .retain(|k| !hf_sim::mind::techniques::CRAFTS.contains(&k.technique));
    }
    w.meta.autoplan = true;
    w.communities[0].house_sites.push(TilePos::new(36, 24, 2));
    w.place_structure(
        StructureKind::Smelter,
        TilePos::new(33, 30, 2),
        M::STONE_BLOCK,
        true,
    );
    let _ = cid;
    w.ensure_nav();
    w
}

/// Agent `i` is a seasoned, old smelter: knows the craft and is past their ailing years.
fn make_old_smelter(w: &mut World, i: usize) {
    let now = w.meta.tick;
    w.agents[i]
        .techniques
        .learn(Technique::Smelting, Learned::Past, 0.6, now);
    w.agents[i].born = -((60 * TICKS_PER_YEAR) as i64);
    w.agents[i].skills.practise(Skill::Construction, 1.0);
    for _ in 0..400 {
        w.agents[i].skills.practise(Skill::Construction, 1.0);
    }
}

#[test]
fn the_last_smith_carves_the_way_of_smelting() {
    let mut w = village(9101, 4);
    let i = 0;
    make_old_smelter(&mut w, i);
    assert_eq!(
        hf_sim::engrave::craft_to_carve(&w, i),
        Some(Technique::Smelting),
        "the only old smelter should prefer to carve the craft"
    );
    let wall = hf_sim::engrave::wall_to_carve(&w, w.communities[0].home, 40)
        .expect("a bare wall stands to carve");
    hf_sim::engrave::carve(&mut w, i, wall);
    let e = w
        .engravings
        .get(&wall)
        .expect("the wall now carries a carving");
    assert_eq!(e.craft, Some(Technique::Smelting), "{e:?}");
    assert!(
        e.line.contains("the way of") && e.line.contains("smelting"),
        "{}",
        e.line
    );
}

#[test]
fn the_planner_proposes_the_craft_carving_within_five_days() {
    // A fresh world, so the one wall this house has is still bare for the planner to find.
    let mut w = village(9103, 4);
    let i = 0;
    make_old_smelter(&mut w, i);
    let day = w.meta.tick / TICKS_PER_DAY + 1;
    w.meta.tick = day * TICKS_PER_DAY + hf_sim::systems::projects::AT;
    hf_sim::systems::projects::step(&mut w);
    let p = w
        .projects
        .iter()
        .find(|p| p.kind == hf_sim::project::ProjectKind::Carve)
        .expect("a carving proposed within five days");
    assert_eq!(p.by, w.agents[i].id);
    assert!(
        p.because.starts_with("is the last who knows it"),
        "{}",
        p.because
    );
}

#[test]
fn with_two_smelters_nobody_carves_the_craft() {
    let mut w = village(9102, 4);
    make_old_smelter(&mut w, 0);
    make_old_smelter(&mut w, 1);
    assert_eq!(
        hf_sim::engrave::craft_to_carve(&w, 0),
        None,
        "two knowers: the craft is not about to die"
    );
    assert_eq!(hf_sim::engrave::craft_to_carve(&w, 1), None);
}

#[test]
fn an_old_saves_engravings_load_with_craft_none() {
    // The field is new; an engraving recorded before it existed carries no craft tag.
    let old = r#"{
        "event": 3,
        "tick": 100,
        "line": "strangers were seen on the ridge",
        "carver": 1,
        "carver_name": "Galn",
        "carved": 120,
        "quality": 0.5
    }"#;
    let e: hf_sim::engrave::Engraving = serde_json::from_str(old).expect("old shape still loads");
    assert_eq!(e.craft, None);
}
