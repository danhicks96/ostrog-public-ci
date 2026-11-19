//! Phase 3 gates: inhabitants satisfy their needs from their own knowledge, and never target
//! what they have not seen.

use hf_sim::agents::Species;
use hf_sim::ids::TilePos;
use hf_sim::items::{Item, ItemKind, ItemPlace};
use hf_sim::mind::Activity;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::structures::StructureKind;
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

/// A meadow with berry bushes that regrow, beds in a hut, and 20 people.
fn meadow(seed: u64, people: usize) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(4, 4, 56, 56, 1);
    w.plants.clear();
    let z = 2;
    for i in 0..120 {
        let p = TilePos::new(6 + (i * 7) % 52, 6 + (i * 11) % 52, z);
        w.plants
            .push(Plant::new(p, PlantKind::BerryBush, 1.0, i as u32));
    }
    w.build_house(20, 20, 12, 8, 1, Material::WOOD, Material::WOOD);
    for i in 0..people.min(20) {
        w.place_structure(
            StructureKind::Bed,
            TilePos::new(22 + (i as i32 % 10), 22 + 2 * (i as i32 / 10), z),
            Material::WOOD,
            false,
        );
    }
    for i in 0..people {
        let p = TilePos::new(30 + (i as i32 % 8), 32 + (i as i32 / 8), z);
        w.spawn_agent(Species::Person, p);
    }
    w
}

#[test]
fn need_satisfaction_seeded() {
    for seed in [11u64, 12, 13] {
        let mut w = meadow(seed, 20);
        let mut starving_events = 0;
        let mut meals = 0;
        let mut sleeps = 0;
        let mut talks = 0;
        let mut min_hunger: f32 = 1.0;
        let days = 10;
        for _ in 0..(TICKS_PER_DAY * days) {
            w.step();
            for e in &w.last_events {
                match e.kind {
                    hf_sim::events::EventKind::Starving { .. } => starving_events += 1,
                    hf_sim::events::EventKind::Ate { .. } => meals += 1,
                    hf_sim::events::EventKind::Slept { .. } => sleeps += 1,
                    hf_sim::events::EventKind::Talked { .. } => talks += 1,
                    _ => {}
                }
            }
            for a in &w.agents {
                min_hunger = min_hunger.min(a.needs.hunger);
            }
        }
        // Regrowth keeps up: every plant regrows fruit through `grow` when growth >= 0.6.
        eprintln!("seed {seed}: meals {meals} sleeps {sleeps} talks {talks} starving {starving_events} min_hunger {min_hunger:.2}");
        assert_eq!(
            starving_events, 0,
            "seed {seed}: nobody starves with food in sight"
        );
        assert!(
            meals >= 20 * days as usize * 2,
            "seed {seed}: people eat regularly ({meals} meals)"
        );
        assert!(
            sleeps >= 20 * (days as usize - 1),
            "seed {seed}: people sleep nightly ({sleeps})"
        );
        assert!(talks > 20, "seed {seed}: people talk ({talks})");
        let switches: u32 = w.agents.iter().map(|a| a.switches).sum();
        let per_agent_per_hour = switches as f32 / 20.0 / (days as f32 * 24.0);
        assert!(
            per_agent_per_hour < 12.0,
            "no oscillation: {per_agent_per_hour:.1} activity changes per agent-hour"
        );
        let stalled = w
            .agents
            .iter()
            .filter(|a| a.activity_ticks > 20 * 60 * 10 && !a.activity.is_asleep())
            .count();
        assert_eq!(stalled, 0, "nobody stuck in one activity for 10 hours");
    }
}

#[test]
fn knowledge_only_targets() {
    // One hungry person; food exists but behind a wall out of sight. They must not go for it.
    let mut w = World::generate(21, GenParams::small());
    w.flatten(4, 4, 56, 56, 1);
    w.plants.clear();
    let z = 2;
    // Walled pantry at (40..46, 40..46) with no door: food inside.
    for i in 40..=46 {
        for (x, y) in [(i, 40), (i, 46), (40, i), (46, i)] {
            w.grid.set(
                TilePos::new(x, y, z),
                hf_sim::world::Tile::solid(Material::STONE_BLOCK),
            );
        }
    }
    let id = w.alloc_item_id();
    w.items.push(Item {
        id,
        kind: ItemKind::Meal,
        count: 10,
        place: ItemPlace::Ground(TilePos::new(43, 43, z)),
        quality: 0.9,
        material: hf_sim::world::Material::AIR,
        story: None,
        rot: 0.0,
    });
    let who = w.spawn_agent(Species::Person, TilePos::new(10, 10, z));
    let idx = w.agents.iter().position(|a| a.id == who).unwrap();
    w.agents[idx].needs.hunger = 0.1;
    w.run(TICKS_PER_DAY / 2);
    let a = &w.agents[idx];
    assert!(
        a.knowledge.get(hf_sim::mind::Known::Item(id)).is_none(),
        "never learned of the hidden meal"
    );
    assert!(
        !matches!(&a.activity, Activity::Going { then, .. } if matches!(**then, Activity::Eating { item, .. } if item == id))
    );
    assert_eq!(
        w.items.iter().find(|it| it.id == id).map(|it| it.count),
        Some(10),
        "hidden meal untouched"
    );
    // Now put a visible bush next to wherever they are: they find it and eat. (#433: thirst may
    // have taken them off the flat ground to water at the map's edge; they are set back on it.)
    w.agents[idx].pos = TilePos::new(20, 20, z);
    w.agents[idx].path.clear();
    w.agents[idx].activity = Activity::Idle;
    let here = w.agents[idx].pos;
    let mut spot = here.offset(2, 0, 0);
    if !w.grid.walkable(spot) {
        spot = here.offset(-2, 0, 0);
    }
    w.plants
        .push(Plant::new(spot, PlantKind::BerryBush, 1.0, 1));
    // They may have collapsed from searching all night; rested, they must notice and eat.
    // (#433: and with water drunk, or they would go to the water first, as anyone would.)
    w.agents[idx].needs.rest = 1.0;
    w.agents[idx].needs.thirst = 1.0;
    let before = w.agents[idx].needs.hunger;
    w.run(20 * 60 * 2);
    assert!(
        w.agents[idx].needs.hunger > before,
        "ate from the bush they could see"
    );
}

/// #1736: a grandmother glad of a birth comes to see the new mother within two days; a
/// stranger with no tie to her does not. Guards against `choose` weighing a new mother the
/// same as on any other day (the Talk gate alone needs `social_p > 0.3`, which a staged
/// grandmother at full social need would never clear).
#[test]
fn kin_come_to_see_the_new_child() {
    let mut w = World::generate(1, GenParams::small());
    w.flatten(4, 4, 56, 56, 1);
    w.plants.clear();
    let z = 2;
    for i in 0..60 {
        let p = TilePos::new(6 + (i * 7) % 52, 6 + (i * 11) % 52, z);
        w.plants
            .push(Plant::new(p, PlantKind::BerryBush, 1.0, i as u32));
    }
    // Mid-morning: well clear of #415's dusk gate, so an open-air camp with no known bed does
    // not also read as "unsecured" and crush every leisure goal's weight for the whole run.
    w.meta.tick = 9 * hf_sim::time::TICKS_PER_HOUR;
    let mother = w.spawn_agent(Species::Person, TilePos::new(30, 30, z));
    let grandmother = w.spawn_agent(Species::Person, TilePos::new(30, 50, z)); // 20 tiles off
    let stranger = w.spawn_agent(Species::Person, TilePos::new(32, 30, z));
    let mi = w.agents.iter().position(|a| a.id == mother).unwrap();
    let gi = w.agents.iter().position(|a| a.id == grandmother).unwrap();
    let si = w.agents.iter().position(|a| a.id == stranger).unwrap();
    // The birth, staged: `last_birth` is how `choose` finds a parent whose child was just
    // born (#1736).
    w.agents[mi].last_birth = w.meta.tick;
    // Kinship: the grandmother's child is the new mother.
    w.agents[gi].children.push(mother);
    // Already known to each other (a family knows where its own live, 20 tiles being well
    // past the ordinary sight range): both the grandmother and the stranger already hold a
    // `Known::Agent` of the mother, so the only difference between them is the tie checked
    // by the fix.
    let mother_pos = w.agents[mi].pos;
    let now = w.meta.tick;
    w.agents[gi].knowledge.note(
        hf_sim::mind::Known::Agent(mother),
        mother_pos,
        now,
        Species::Person as u16,
    );
    w.agents[si].knowledge.note(
        hf_sim::mind::Known::Agent(mother),
        mother_pos,
        now,
        Species::Person as u16,
    );
    // Neither is pining for company or hungry, so nothing but the birth draws them in.
    for i in [mi, gi, si] {
        w.agents[i].needs.social = 1.0;
        w.agents[i].needs.hunger = 0.9;
        w.agents[i].needs.rest = 1.0;
        w.agents[i].needs.thirst = 1.0;
        w.agents[i].needs.comfort = 1.0;
    }
    // A window short enough that the ordinary Talk gate (social_p > 0.3) has not opened on
    // its own: social decays fully over 24 hours (SOCIAL_HOURS), so a full social need stays
    // under that gate for about 7 hours. Five hours leaves margin and still sits well inside
    // the issue's two-day window, so only the birth-visit consideration can produce the talk.
    let mut talked_with_grandmother = false;
    let mut talked_with_stranger = false;
    for _ in 0..(5 * hf_sim::time::TICKS_PER_HOUR) {
        w.step();
        for e in &w.last_events {
            if let hf_sim::events::EventKind::Talked { a, b, .. } = e.kind {
                let pair = (a, b);
                if pair == (mother, grandmother) || pair == (grandmother, mother) {
                    talked_with_grandmother = true;
                }
                if pair == (mother, stranger) || pair == (stranger, mother) {
                    talked_with_stranger = true;
                }
            }
        }
        if talked_with_grandmother {
            break;
        }
    }
    assert!(
        talked_with_grandmother,
        "the grandmother never came to see the new baby within two days"
    );
    assert!(
        !talked_with_stranger,
        "a stranger with no tie to the mother should not have come"
    );
}
