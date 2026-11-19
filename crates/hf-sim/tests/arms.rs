//! Chains and arms (C33): bars of every metal, bronze from copper and tin, recipes that take
//! only the material they name; spears and armour made from ore, taken up when strangers
//! threaten, worn down by fights, laid by when the danger passes.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::items::{ItemKind as K, ItemPlace};
use hf_sim::mind::beliefs::{Source, Topic};
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::structures::{Structure, StructureKind};
use hf_sim::systems::arms;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::{recipes, DesignationKind};
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material as M;
use hf_sim::World;

fn workshop(seed: u64) -> World {
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
    w.rebuild_caches();
    w.found_party(10);
    w.meta.autoplan = false;
    // A party with a smith among them (C70: the smelter and the anvil are worked only by
    // those who know the crafts).
    {
        use hf_sim::mind::techniques::{Learned, Technique};
        let tick = w.meta.tick;
        for a in w
            .agents
            .iter_mut()
            .filter(|a| a.community.is_some())
            .take(3)
        {
            a.techniques
                .learn(Technique::Smelting, Learned::Past, 0.5, tick);
            a.techniques
                .learn(Technique::Smithing, Learned::Past, 0.5, tick);
            a.techniques
                .learn(Technique::Carpentry, Learned::Past, 0.5, tick);
        }
    }
    for (kind, pos) in [
        (StructureKind::Smelter, TilePos::new(36, 28, 2)),
        (StructureKind::Anvil, TilePos::new(36, 31, 2)),
        (StructureKind::CarpenterBench, TilePos::new(24, 28, 2)),
    ] {
        let id = w.alloc_structure_id();
        w.structures.push(Structure {
            id,
            kind,
            pos,
            material: M::STONE_BLOCK,
            progress: 1.0,
            active: false,
            condition: 1.0,
            owner: None,
            quality: 0.5,
        });
    }
    w.items.clear();
    w.ensure_nav();
    w
}

fn order(w: &mut World, r: u8) {
    let at = match recipes::recipe(r).station {
        StructureKind::Smelter => TilePos::new(36, 28, 2),
        StructureKind::Anvil => TilePos::new(36, 31, 2),
        _ => TilePos::new(24, 28, 2),
    };
    let cid = w.communities[0].id;
    let mut d = hf_sim::systems::settlement::make(w, DesignationKind::Craft(r), at, 7);
    d.community = Some(cid);
    w.designations.push(d);
}

fn put(w: &mut World, k: K, m: M, n: u32, x: i32, y: i32) {
    w.create_item_of(
        k,
        m,
        n,
        ItemPlace::Ground(TilePos::new(x, y, 2)),
        0.6,
        "test",
    );
}

fn of(w: &World, k: K, m: M) -> u32 {
    w.items
        .iter()
        .filter(|it| it.kind == k && it.material == m)
        .map(|it| it.count)
        .sum()
}

fn run_until(w: &mut World, ticks: u64, done: impl Fn(&World) -> bool) {
    for _ in 0..ticks {
        w.step();
        if done(w) {
            break;
        }
    }
}

#[test]
fn a_tin_bar_takes_only_tin_ore() {
    let mut w = workshop(3301);
    put(&mut w, K::Ore, M::SILVER_ORE, 4, 33, 29);
    put(&mut w, K::Ore, M::TIN_ORE, 2, 33, 31);
    put(&mut w, K::Coal, M::COAL, 2, 34, 31);
    order(&mut w, recipes::TIN_BAR);
    run_until(&mut w, TICKS_PER_DAY * 2, |w| of(w, K::Bar, M::TIN) > 0);
    assert_eq!(of(&w, K::Bar, M::TIN), 1, "a tin bar was smelted");
    assert_eq!(
        of(&w, K::Ore, M::SILVER_ORE),
        4,
        "the silver ore was left alone"
    );
    assert_eq!(of(&w, K::Ore, M::TIN_ORE), 0);
}

#[test]
fn bronze_is_copper_and_tin() {
    let mut w = workshop(3302);
    put(&mut w, K::CopperBar, M::COPPER, 2, 33, 29);
    put(&mut w, K::Bar, M::SILVER, 2, 33, 30);
    put(&mut w, K::Bar, M::TIN, 1, 33, 31);
    put(&mut w, K::Coal, M::COAL, 1, 34, 31);
    order(&mut w, recipes::BRONZE);
    run_until(&mut w, TICKS_PER_DAY * 2, |w| of(w, K::Bar, M::BRONZE) > 0);
    assert_eq!(
        of(&w, K::Bar, M::BRONZE),
        3,
        "three bronze bars from two copper and one tin"
    );
    assert_eq!(of(&w, K::Bar, M::SILVER), 2, "silver is not tin");
    assert_eq!(of(&w, K::Bar, M::TIN), 0);
}

#[test]
fn ore_becomes_a_weapon_that_is_worn_and_used() {
    let mut w = workshop(3303);
    put(&mut w, K::IronOre, M::IRON_ORE, 2, 33, 30);
    put(&mut w, K::Coal, M::COAL, 1, 33, 31);
    put(&mut w, K::Log, M::OAK, 2, 27, 30);
    order(&mut w, recipes::IRON_BAR);
    order(&mut w, recipes::PLANK);
    run_until(&mut w, TICKS_PER_DAY * 3, |w| {
        of(w, K::IronBar, M::IRON) > 0 && of(w, K::Plank, M::OAK) > 0
    });
    order(&mut w, recipes::IRON_WEAPON);
    run_until(&mut w, TICKS_PER_DAY * 3, |w| {
        w.items.iter().any(|it| it.kind == K::Weapon)
    });
    let spear = w
        .items
        .iter()
        .find(|it| it.kind == K::Weapon)
        .expect("a spear was forged")
        .clone();
    assert_eq!(spear.material, M::IRON);
    assert!(spear.story.is_some(), "a spear has a story: who made it");
    // Strangers come near: one who would fight takes it up.
    for a in w.agents.iter_mut().filter(|a| a.community.is_some()) {
        a.beliefs.hold(
            Topic::Violence,
            0.6,
            0.9,
            Source::Event(hf_sim::ids::EventId(0)),
            0,
        );
    }
    let home = w.communities[0].home;
    arms::threatened(&mut w, home);
    let mut took = None;
    for _ in 0..TICKS_PER_HOUR * 3 {
        w.step();
        for e in &w.last_events {
            if let EventKind::TookUpArms { agent, what } = &e.kind {
                took = Some((*agent, what.clone()));
            }
        }
        if took.is_some() {
            break;
        }
    }
    let (who, what) = took.expect("someone took up the spear");
    assert!(what.contains("spear"), "{what}");
    let i = w.agents.iter().position(|a| a.id == who).unwrap();
    assert_eq!(w.agents[i].weapon, Some(spear.id));
    // It is used: blows struck wear it and its story keeps them.
    arms::struck_with(&mut w, i, false);
    let it = w.items.iter().find(|it| it.id == spear.id).unwrap();
    assert_eq!(it.place, ItemPlace::Held(who));
    let s = it.story.as_ref().unwrap();
    assert!(
        s.wear < 1.0 && s.fights == 1,
        "wear {} fights {}",
        s.wear,
        s.fights
    );
    assert!(
        s.deeds.iter().any(|d| d.1.contains("stranger")),
        "{:?}",
        s.deeds
    );
}

#[test]
fn harder_metal_better_arms_and_blows_wear_them_out() {
    let mut w = workshop(3304);
    let who = w.agents.iter().position(|a| a.community.is_some()).unwrap();
    let mut worth = Vec::new();
    for m in [M::COPPER, M::BRONZE, M::IRON] {
        let id = w.create_lone_item(
            K::Armour,
            1,
            ItemPlace::Ground(TilePos::new(30, 30, 2)),
            0.6,
            "test",
        );
        w.items.last_mut().unwrap().material = m;
        hf_sim::legend::ensure(&mut w, id, None, "test");
        w.agents[who].armour = Some(id);
        worth.push(arms::guard(&w, who).unwrap().1);
    }
    assert!(
        worth[0] < worth[1] && worth[1] < worth[2],
        "copper < bronze < iron: {worth:?}"
    );
    // Iron armour taking blow after blow: it holds a while, then is gone.
    let id = w.agents[who].armour.unwrap();
    let mut broke_at = None;
    for n in 1..=20 {
        arms::took_blow(&mut w, who);
        if !w.items.iter().any(|it| it.id == id) {
            broke_at = Some(n);
            break;
        }
    }
    let n = broke_at.expect("armour wears out under blows");
    assert!(n >= 10, "it held for {n} blows");
    assert_eq!(w.agents[who].armour, None);
}

#[test]
fn arms_are_laid_by_when_the_danger_has_passed() {
    let mut w = workshop(3305);
    for a in w.agents.iter_mut() {
        a.beliefs.hold(
            Topic::Violence,
            0.6,
            0.9,
            Source::Event(hf_sim::ids::EventId(0)),
            0,
        );
    }
    let home = w.communities[0].home;
    let id = w.create_lone_item(K::Weapon, 1, ItemPlace::Ground(home), 0.7, "test");
    w.items.last_mut().unwrap().material = M::BRONZE;
    arms::threatened(&mut w, home);
    w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR + 41;
    arms::step(&mut w);
    let holder = w
        .agents
        .iter()
        .position(|a| a.weapon == Some(id))
        .expect("taken up");
    // A season of quiet later.
    w.meta.tick += arms::LAY_BY_AFTER + TICKS_PER_DAY;
    w.meta.tick = (w.meta.tick / TICKS_PER_HOUR) * TICKS_PER_HOUR + 41;
    arms::step(&mut w);
    assert_eq!(w.agents[holder].weapon, None);
    let it = w.items.iter().find(|it| it.id == id).unwrap();
    assert_eq!(it.place, ItemPlace::Ground(home), "laid by at home");
}

#[test]
fn nobody_takes_up_arms_who_would_not_fight() {
    let mut w = workshop(3306);
    for a in w.agents.iter_mut() {
        a.beliefs.hold(
            Topic::Violence,
            -0.8,
            0.9,
            Source::Event(hf_sim::ids::EventId(0)),
            0,
        );
    }
    let home = w.communities[0].home;
    w.create_lone_item(K::Weapon, 1, ItemPlace::Ground(home), 0.7, "test");
    arms::threatened(&mut w, home);
    w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR + 41;
    arms::step(&mut w);
    assert!(w.agents.iter().all(|a| a.weapon.is_none()));
}

/// An opened world four days in with a hungry neighbour (as tests/threats.rs), its people
/// holding with force, and spears in the store.
fn raided(seed: u64) -> World {
    let mut w = World::opening_world(seed);
    w.meta.autoplan = true;
    w.meta.engine = hf_sim::engine::Engine::Relentless;
    for _ in 0..4 * TICKS_PER_DAY {
        w.step();
    }
    let s = w
        .overworld
        .sites
        .iter_mut()
        .find(|s| s.people > 0)
        .expect("an inhabited place beyond the valley");
    s.food = -(s.people as f32) * 4.0;
    s.hungry_days = 3;
    for a in w.agents.iter_mut().filter(|a| a.community.is_some()) {
        for t in [Topic::Strangers, Topic::Violence] {
            a.beliefs
                .hold(t, 0.9, 0.9, Source::Event(hf_sim::ids::EventId(0)), 0);
        }
    }
    let home = w.communities[0].home;
    for k in 0..4 {
        w.create_lone_item(
            K::Weapon,
            1,
            ItemPlace::Ground(home.offset(k, 1, 0)),
            0.7,
            "test",
        );
        w.items.last_mut().unwrap().material = M::IRON;
    }
    w
}

#[test]
fn spears_are_taken_up_against_a_band_and_used_on_it() {
    let (mut took, mut struck) = (0, 0);
    for seed in [3u64, 7, 11] {
        let mut w = raided(seed);
        let start = w.meta.tick;
        for _ in 0..12 * TICKS_PER_DAY {
            w.step();
            took += w
                .last_events
                .iter()
                .filter(|e| matches!(e.kind, EventKind::TookUpArms { .. }))
                .count();
            if w.bands.is_empty() && w.meta.tick - start > 10 * TICKS_PER_DAY {
                break;
            }
        }
        struck += w
            .items
            .iter()
            .filter(|it| it.kind == K::Weapon)
            .filter_map(|it| it.story.as_ref())
            .map(|s| s.fights)
            .sum::<u32>();
    }
    assert!(took >= 3, "spears taken up: {took}");
    assert!(struck >= 1, "blows struck with them: {struck}");
}

#[test]
fn built_things_say_what_they_are_made_of() {
    let mut w = workshop(3307);
    let p = TilePos::new(20, 20, 2);
    let id = w.alloc_structure_id();
    w.structures.push(Structure {
        id,
        kind: StructureKind::CarpenterBench,
        pos: p,
        material: M::OAK,
        progress: 1.0,
        active: false,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
    assert_eq!(
        hf_sim::structures::note(&w, p).as_deref(),
        Some("A carpenter bench of oak")
    );
    // The demo's plain-timber things say nothing.
    w.structures.last_mut().unwrap().material = M::WOOD;
    assert_eq!(hf_sim::structures::note(&w, p), None);
}
