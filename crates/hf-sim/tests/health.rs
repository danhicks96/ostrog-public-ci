//! Phase 15 gates: a wound heals with bed rest; an untreated infection can kill while a
//! tended one does not; winter sickness spreads between people sleeping close and fades.

use hf_sim::body::{Illness, Part, WoundKind};
use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::items::ItemKind;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

/// A 9Ã—7 wooden house at (24,24) with six beds, as the planner would build it.
fn house_with_beds(w: &mut World) {
    w.build_house(
        24,
        24,
        9,
        7,
        1,
        hf_sim::world::Material::WOOD,
        hf_sim::world::Material::WOOD,
    );
    for k in 0..6 {
        let p = TilePos::new(26 + (k % 3) * 2, 26 + (k / 3) * 2, 2);
        let id = w.alloc_structure_id();
        w.structures.push(hf_sim::structures::Structure {
            id,
            kind: hf_sim::structures::StructureKind::Bed,
            pos: p,
            material: hf_sim::world::Material::WOOD,
            progress: 1.0,
            active: false,
            condition: 1.0,
            owner: None,
            quality: 0.5,
        });
    }
    w.rebuild_caches();
}

fn meadow(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    for i in 0..100 {
        w.plants.push(Plant::new(
            TilePos::new(4 + (i * 7) % 56, 4 + (i * 11) % 56, 2),
            PlantKind::BerryBush,
            1.0,
            i as u32,
        ));
    }
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    // A well by the house (#433): people need water, and in winter the open water is ice.
    w.place_structure(
        hf_sim::structures::StructureKind::Well,
        TilePos::new(36, 33, 2),
        hf_sim::world::Material::STONE_BLOCK,
        true,
    );
    w.rebuild_caches();
    w
}

#[test]
fn wound_heals_with_rest() {
    let mut w = meadow(1501);
    let ids = w.found_party(6);
    house_with_beds(&mut w);
    w.ensure_nav();
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    // They know their house's beds (#454: until then the test relied on the role handed out by
    // skill walking this person past the house before the first night; design rule 8 lets
    // nobody sleep in a bed they have not seen).
    let tick = w.meta.tick;
    let beds: Vec<_> = w
        .structures
        .iter()
        .filter(|s| s.kind == hf_sim::structures::StructureKind::Bed)
        .map(|s| (s.id, s.pos))
        .collect();
    for (id, pos) in beds {
        w.agents[i].knowledge.note(
            hf_sim::mind::knowledge::Known::Structure(id),
            pos,
            tick,
            hf_sim::structures::StructureKind::Bed as u16,
        );
    }
    hf_sim::systems::health::wound(
        &mut w,
        i,
        Part::ARM_L,
        WoundKind::Bruise,
        0.5,
        0.0,
        "in a test",
        None,
    );
    assert!(w.agents[i].body.hurt());
    let mut healed = false;
    let mut slept_in_bed = 0;
    for _ in 0..TICKS_PER_DAY * 6 {
        w.step();
        if matches!(
            w.agents[i].activity,
            hf_sim::mind::Activity::Sleeping { bed: Some(_), .. }
        ) {
            slept_in_bed += 1;
        }
        if w.last_events
            .iter()
            .any(|e| matches!(&e.kind, EventKind::Healed { agent, .. } if *agent == ids[0]))
        {
            healed = true;
            break;
        }
    }
    assert!(healed, "the bruise healed within six days");
    assert!(slept_in_bed > 0, "they rested in a bed");
    // The bruise is gone (#274: stone work takes longer by its hardness now, and in this run
    // the builder later cut an arm at the building site, a wound of its own).
    assert!(
        !w.agents[i]
            .body
            .wounds
            .iter()
            .any(|x| x.kind == WoundKind::Bruise && x.part == Part::ARM_L),
        "{:?}",
        w.agents[i].body.wounds
    );
    assert!(w.agents[i].alive);
}

#[test]
fn infection_can_kill() {
    // Two worlds from one seed: a deep cut that turns bad with nobody to tend it, and the
    // same cut with a healer and poultices at hand.
    let outcome = |tend: bool| -> (bool, bool) {
        let mut w = meadow(1502);
        let ids = w.found_party(6);
        house_with_beds(&mut w);
        w.meta.autoplan = tend;
        if tend {
            w.drop_item(ItemKind::Poultice, 6, TilePos::new(36, 30, 2));
            // Tending is something a person knows how to do, not an order: one of them does.
            let h = w.agents.iter().position(|a| a.id == ids[1]).unwrap();
            w.agents[h].techniques.learn(
                hf_sim::mind::Technique::Poultice,
                hf_sim::mind::techniques::Learned::Past,
                0.6,
                0,
            );
        }
        if !tend {
            // Nobody at all who knows healing (#284: one who does takes a rotting limb to save
            // its owner).
            for a in w.agents.iter_mut() {
                a.techniques
                    .list
                    .retain(|k| k.technique != hf_sim::mind::Technique::HerbLore);
                a.skills.states[hf_sim::mind::Skill::Cooking as usize].level = 0.0;
            }
        }
        w.ensure_nav();
        let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
        hf_sim::systems::health::wound(
            &mut w,
            i,
            Part::LEG_R,
            WoundKind::Cut,
            0.7,
            0.0,
            "in a test",
            None,
        );
        w.agents[i].body.wounds[0].infected = true;
        w.agents[i].body.illness = Some(Illness {
            severity: 0.5,
            since: w.tick(),
            // A wound gone bad (#328): it grows while the wound is foul.
            kind: hf_sim::body::IllnessKind::WoundRot,
            treated: None,
        });
        let mut tended = false;
        for _ in 0..TICKS_PER_DAY * 12 {
            w.step();
            if w.last_events
                .iter()
                .any(|e| matches!(&e.kind, EventKind::Tended { patient, .. } if *patient == ids[0]))
            {
                tended = true;
            }
            if !w.agents[i].alive {
                break;
            }
        }
        let a = &w.agents[i];
        // OST-205: a tended, miserable patient may now set out from the valley instead of
        // staying (no death is recorded then); what matters here is that the fever did not kill.
        (
            a.alive || a.death.is_none(),
            tended || a.death.as_deref() == Some("fever"),
        )
    };
    let (alive_alone, _) = outcome(false);
    let (alive_tended, was_tended) = outcome(true);
    assert!(!alive_alone, "untreated, the infection killed");
    assert!(was_tended, "a healer brought a poultice");
    assert!(alive_tended, "tended, they lived");
}

#[test]
fn illness_spreads_then_fades() {
    let mut w = meadow(1503);
    let ids = w.found_party(10);
    house_with_beds(&mut w);
    w.meta.autoplan = false;
    // Winter, day 16, and food enough that nobody is weak from hunger.
    w.meta.tick = 15 * TICKS_PER_DAY + TICKS_PER_HOUR * 8;
    w.drop_item(ItemKind::Meal, 60, TilePos::new(36, 30, 2));
    w.rebuild_caches();
    w.ensure_nav();
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    w.agents[i].body.illness = Some(Illness {
        severity: 0.8,
        since: w.tick(),
        kind: Default::default(),
        treated: None,
    });
    let mut caught = std::collections::BTreeSet::new();
    // Someone can walk out of the valley alive. Their retained row stops updating then.
    let mut departed = std::collections::BTreeSet::new();
    let mut note_departures = |w: &World| {
        for e in &w.last_events {
            if let EventKind::Left { agent } = e.kind {
                departed.insert(agent);
            }
        }
    };
    for _ in 0..TICKS_PER_DAY * 5 {
        w.step();
        note_departures(&w);
        for e in &w.last_events {
            if let EventKind::FellIll { agent, .. } = &e.kind {
                caught.insert(*agent);
            }
        }
    }
    assert!(
        caught.len() >= 2,
        "the fever passed to others in the house: {caught:?}"
    );
    // Leave winter behind and let it run its course.
    w.meta.tick = 21 * TICKS_PER_DAY;
    w.rebuild_caches();
    for _ in 0..TICKS_PER_DAY * 12 {
        w.step();
        note_departures(&w);
    }
    let survived = |a: &hf_sim::agents::Agent| {
        ids.contains(&a.id) && (a.alive || (departed.contains(&a.id) && a.death.is_none()))
    };
    let sick = w
        .agents
        .iter()
        // The winter fever, that is. Twelve days of ordinary life can bring a fresh infection
        // from a new wound (a hunt went wrong on one seed once threats and goats arrived);
        // that is a different illness, and infection_can_kill covers it.
        .filter(|a| (a.alive || survived(a)) && a.body.illness.is_some())
        .filter(|a| !a.body.wounds.iter().any(|wd| wd.infected))
        // Fix-site (FIXLOG 2026-11-01 #488 beast fever): a hunter bitten in these twelve days
        // catches beast fever, a separate illness with its own slow course, whose wound has
        // healed by then. Only the house's winter fever counts here.
        .filter(|a| {
            a.body.illness.is_some_and(|i| {
                matches!(
                    i.kind,
                    hf_sim::body::IllnessKind::WinterCough | hf_sim::body::IllnessKind::Fever
                )
            })
        })
        .count();
    assert_eq!(sick, 0, "everyone recovered from the fever");
    let alive = w.agents.iter().filter(|a| survived(a)).count();
    assert!(
        alive >= 8,
        "a winter fever thins, it does not erase: {alive}"
    );
}
