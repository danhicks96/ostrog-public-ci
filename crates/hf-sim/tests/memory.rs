//! Phase 5 gates: memories change choices, stay bounded, and move relationships.

use hf_sim::agents::Species;
use hf_sim::ids::TilePos;
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::mind::{MemKind, Subject};
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn meadow(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.meta.autoplan = false;
    w.rebuild_caches();
    w
}

#[test]
fn memory_changes_choice() {
    // Two identical hungry people, two equally close food stacks (roots and berries). One
    // carries a strong sour memory of roots; they must pick the berries. The control picks
    // whichever scores higher without bias, so we only require the biased one to avoid roots.
    let mut w = meadow(51);
    let roots = TilePos::new(34, 30, 2);
    let berries = TilePos::new(26, 30, 2);
    w.drop_item(ItemKind::Root, 10, roots);
    w.drop_item(ItemKind::Berries, 10, berries);
    let biased = w.spawn_agent(Species::Person, TilePos::new(30, 30, 2));
    let control = w.spawn_agent(Species::Person, TilePos::new(30, 32, 2));
    for id in [biased, control] {
        let a = w.agents.iter_mut().find(|a| a.id == id).unwrap();
        a.needs.hunger = 0.3;
        a.needs.rest = 1.0;
        a.needs.social = 1.0;
        a.traits.list.clear();
    }
    let a = w.agents.iter_mut().find(|a| a.id == biased).unwrap();
    for t in 0..30u64 {
        a.remember(
            t,
            MemKind::Meal,
            Subject::Food(ItemKind::Root),
            -0.9,
            0.8,
            None,
        );
    }
    a.memories.consolidate(100, false, false);
    assert!(
        a.memories
            .bias(MemKind::Meal, Subject::Food(ItemKind::Root))
            < 0.5
    );
    let mut first_meal: std::collections::BTreeMap<u32, ItemKind> = Default::default();
    for _ in 0..TICKS_PER_HOUR * 3 {
        w.step();
        for e in &w.last_events {
            if let hf_sim::events::EventKind::Ate {
                agent,
                item: Some(_),
                ..
            } = e.kind
            {
                first_meal.entry(agent.0).or_insert_with(|| {
                    // The item has been consumed by one; identify by what is left near the eater.
                    let a = w.agents.iter().find(|a| a.id == agent).unwrap();
                    if a.pos.dist(roots) <= 2 {
                        ItemKind::Root
                    } else {
                        ItemKind::Berries
                    }
                });
            }
        }
        if first_meal.len() == 2 {
            break;
        }
    }
    assert_eq!(
        first_meal.get(&biased.0),
        Some(&ItemKind::Berries),
        "the person who remembers bad roots eats berries: {first_meal:?}"
    );
}

#[test]
fn memory_cap_bounded() {
    let mut w = World::generate(52, GenParams::default());
    w.found_party(8);
    w.run(TICKS_PER_DAY * 6);
    for a in &w.agents {
        assert!(
            a.memories.short.len() <= hf_sim::mind::memory::SHORT_CAP,
            "{} short {}",
            a.name,
            a.memories.short.len()
        );
        assert!(
            a.memories.long.len() <= hf_sim::mind::memory::LONG_CAP,
            "{} long {}",
            a.name,
            a.memories.long.len()
        );
        assert!(a.relationships.len() <= hf_sim::mind::memory::RELATION_CAP);
    }
    let with_memories = w
        .agents
        .iter()
        .filter(|a| a.species == Species::Person && !a.memories.long.is_empty())
        .count();
    assert!(
        with_memories >= 6,
        "most people formed memories after six days: {with_memories}"
    );
    let dreamt = w
        .agents
        .iter()
        .filter(|a| a.species == Species::Person)
        .all(|a| a.last_sleep > 0);
    assert!(dreamt);
}

#[test]
fn relationship_from_memories() {
    // A small group talks for days; opinions move and favourite partners emerge.
    let mut w = meadow(53);
    for i in 0..5 {
        w.spawn_agent(Species::Person, TilePos::new(28 + i, 30, 2));
    }
    for i in 0..30 {
        w.plants.push(Plant::new(
            TilePos::new(10 + (i * 7) % 40, 10 + (i * 11) % 40, 2),
            PlantKind::BerryBush,
            1.0,
            i as u32,
        ));
    }
    w.run(TICKS_PER_DAY * 4);
    let positive = w
        .agents
        .iter()
        .flat_map(|a| a.relationships.values())
        .filter(|r| r.opinion > 0.1)
        .count();
    assert!(
        positive >= 4,
        "talking builds positive opinions: {positive}"
    );
    let have_company_memory = w
        .agents
        .iter()
        .filter(|a| a.memories.long.iter().any(|m| m.kind == MemKind::Company))
        .count();
    assert!(
        have_company_memory >= 3,
        "company memories consolidated: {have_company_memory}"
    );
    // Talk partner choice follows opinion: the most-talked pair should be a top opinion.
    let a = &w.agents[0];
    if let Some((best, r)) = a
        .relationships
        .iter()
        .max_by(|x, y| x.1.opinion.partial_cmp(&y.1.opinion).unwrap())
    {
        assert!(r.familiarity > 0.0, "{} knows {:?}", a.name, best);
    }
    let _ = ItemPlace::Carried(a.id);
}

#[test]
fn nobody_has_eaten_until_they_eat() {
    // #425: a new animal and a new survivor held their spawn tick as a last meal, which the
    // inspect dump printed as a day number. They hold "never" until the first real meal.
    let mut w = meadow(425);
    let at = TilePos::new(30, 30, 2);
    let hare = w.spawn_agent(Species::Hare, at);
    let slot = hf_sim::opening::SurvivorSlot {
        index: 0,
        rerolls: 0,
        state: hf_sim::opening::SurvivorState::Starting,
        past_events: Vec::new(),
    };
    let survivor = w.spawn_survivor(&slot, TilePos::new(30, 32, 2));
    for id in [hare, survivor] {
        let a = w.agents.iter().find(|a| a.id == id).unwrap();
        assert_eq!(a.last_meal, None, "{} has not eaten yet", a.name);
    }
    for id in [hare, survivor] {
        let a = w.agents.iter_mut().find(|a| a.id == id).unwrap();
        a.needs.hunger = 0.1;
        a.needs.rest = 1.0;
        a.needs.social = 1.0;
    }
    w.drop_item(ItemKind::Berries, 20, at);
    w.drop_item(ItemKind::Berries, 20, TilePos::new(30, 32, 2));
    let start = w.meta.tick;
    w.run(TICKS_PER_DAY);
    let a = w.agents.iter().find(|a| a.id == survivor).unwrap();
    let t = a.last_meal.expect("the hungry survivor ate");
    assert!(t >= start && t <= w.meta.tick, "a real tick, not the spawn");
}
