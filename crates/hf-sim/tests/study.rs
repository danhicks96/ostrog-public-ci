//! Study (issue #50, C13): records and tools the old hamlets left can teach a craft; the craft
//! changes the work; it spreads to those who watch it work.

use hf_sim::agents::Species;
use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::mind::techniques::{Learned, Technique};
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::work::DesignationKind;
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

#[test]
fn a_record_teaches_its_makers_craft() {
    let mut w = World::opening_world(11);
    w.meta.autoplan = false;
    let (rid, rpos, teaches) = w
        .items
        .iter()
        .find_map(|it| {
            let s = it.story.as_ref()?;
            if it.kind != ItemKind::Record {
                return None;
            }
            match it.place {
                ItemPlace::Ground(p) => Some((it.id, p, s.teaches?)),
                _ => None,
            }
        })
        .expect("the valley's past left a record");
    // Someone who does not know the craft, standing by it.
    let a = w.spawn_agent(Species::Person, rpos);
    let ia = w.agents.iter().position(|x| x.id == a).unwrap();
    w.agents[ia].community = w.communities.first().map(|c| c.id);
    w.agents[ia].techniques = Default::default();
    // Someone curious (the kind who goes and sits over a find).
    if !w.agents[ia].traits.has(hf_sim::mind::Trait::Curious) {
        w.agents[ia].traits.list.push(hf_sim::mind::Trait::Curious);
    }
    w.agents[ia].knowledge.note(
        hf_sim::mind::knowledge::Known::Item(rid),
        rpos,
        w.meta.tick,
        0,
    );
    w.ensure_nav();
    let mut learned = false;
    for _ in 0..8 * TICKS_PER_DAY {
        // Keep them fed and rested so leisure is free for it.
        let ia = w.agents.iter().position(|x| x.id == a).unwrap();
        w.agents[ia].needs.hunger = 1.0;
        w.agents[ia].needs.rest = 1.0;
        w.agents[ia].needs.social = 1.0;
        w.step();
        for e in &w.last_events {
            if let EventKind::Studied {
                agent,
                success,
                learned: l,
                ..
            } = &e.kind
            {
                // (They may study some other find first; the record's own craft is the test.)
                if *agent == a && *success && l == teaches.name() {
                    learned = true;
                }
            }
        }
        if learned {
            break;
        }
    }
    assert!(learned, "they studied it and worked it out");
    let ag = w.agents.iter().find(|x| x.id == a).unwrap();
    let k = ag
        .techniques
        .list
        .iter()
        .find(|k| k.technique == teaches)
        .expect("knows the craft");
    assert_eq!(k.learned, Learned::Studied);
    assert!(k.from.contains("left at"), "{}", k.from);
}

fn roof_quality(with: Option<Technique>) -> f32 {
    let mut w = World::generate(31, GenParams::small());
    w.agents.retain(|a| a.species == Species::Person);
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    let ids = w.found_party(3);
    for id in &ids {
        let a = w.agents.iter_mut().find(|a| a.id == *id).unwrap();
        a.techniques = Default::default();
        if let Some(t) = with {
            a.techniques.learn(t, Learned::Studied, 0.8, 0);
        }
    }
    // Walls of a long shed, roof tiles inside, planks and thatch to hand (C67: a roof is laid
    // of made things, a plank and a bundle of thatch a tile, not of raw logs).
    w.build_house(20, 20, 9, 5, 1, Material::WOOD, Material::WOOD);
    w.roofs.clear();
    w.drop_item(ItemKind::Plank, 30, TilePos::new(24, 27, 2));
    w.drop_item(ItemKind::Thatch, 30, TilePos::new(25, 27, 2));
    for x in 21..28 {
        for y in 21..24 {
            let d = hf_sim::systems::settlement::make(
                &mut w,
                DesignationKind::BuildRoof(Material::THATCH),
                TilePos::new(x, y, 2),
                9,
            );
            w.designations.push(d);
        }
    }
    w.ensure_nav();
    for _ in 0..2 * TICKS_PER_DAY {
        w.step();
    }
    assert!(w.roofs.len() >= 10, "{}", w.roofs.len());
    w.roofs.values().map(|r| r.quality).sum::<f32>() / w.roofs.len() as f32
}

#[test]
fn the_old_way_of_roofing_lays_roofs_that_bear_more() {
    let plain = roof_quality(None);
    let old = roof_quality(Some(Technique::Roofing));
    assert!(
        old > plain + 0.1,
        "with the craft {old:.2}, without {plain:.2}"
    );
}

#[test]
fn a_craft_spreads_to_those_who_watch_it_work() {
    let mut w = World::generate(32, GenParams::small());
    w.agents.retain(|a| a.species == Species::Person);
    let ids = w.found_party(6);
    let p = w.agents.iter().find(|a| a.id == ids[0]).unwrap().pos;
    for (k, id) in ids.iter().enumerate() {
        let a = w.agents.iter_mut().find(|a| a.id == *id).unwrap();
        a.techniques = Default::default();
        a.pos = TilePos::new(p.x + k as i32 % 3, p.y + k as i32 / 3, p.z);
        if k == 0 {
            a.techniques
                .learn(Technique::Hearthcraft, Learned::Studied, 0.6, 0);
        }
    }
    let i0 = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    for _ in 0..80 {
        hf_sim::systems::learn::witnessed(&mut w, i0, Technique::Hearthcraft);
        w.meta.tick += 1;
    }
    let learned: Vec<_> = w
        .agents
        .iter()
        .filter(|a| a.id != ids[0] && a.techniques.knows(Technique::Hearthcraft))
        .collect();
    assert!(!learned.is_empty(), "someone picked it up by watching");
    for a in &learned {
        let k = a
            .techniques
            .list
            .iter()
            .find(|k| k.technique == Technique::Hearthcraft)
            .unwrap();
        assert_eq!(k.learned, Learned::Watched);
        assert!(k.from.starts_with("from watching"), "{}", k.from);
    }
    let lines: Vec<String> = w
        .events
        .pending
        .iter()
        .map(|e| hf_sim::view::narrate::line(&w, e))
        .collect();
    assert!(lines
        .iter()
        .any(|l| l.contains("picked up old hearth cooking, watching")));
    // And the one who knows it got better with use.
    assert!(w.agents[i0].techniques.skill(Technique::Hearthcraft) > 0.6);
}

#[test]
fn in_an_ordinary_run_someone_goes_to_study() {
    let mut studied = 0;
    for seed in [7u64, 11, 19] {
        let mut w = World::opening_world(seed);
        w.meta.autoplan = true;
        for _ in 0..25 * TICKS_PER_DAY {
            w.step();
            studied += w
                .last_events
                .iter()
                .filter(|e| matches!(e.kind, EventKind::Studied { .. }))
                .count();
        }
    }
    assert!(studied >= 1, "{studied}");
}
