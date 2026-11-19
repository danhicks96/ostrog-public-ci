//! QA round 2026-10-20, the medium issues (C62): one guard per fix, each failing without it.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::save::{decode, encode, snapshot::Snapshot};
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::world::gen::GenParams;
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

fn weather_lines(w: &mut World) -> Vec<String> {
    w.events
        .drain()
        .into_iter()
        .filter_map(|e| match e.kind {
            EventKind::Weather { kind } => Some(kind),
            _ => None,
        })
        .collect()
}

#[test]
fn weather_after_a_load_keeps_the_cold_lean() {
    // #160: a load rebuilt the sky without the deity's Cold lean, so the first tick after it
    // announced snow that a game never saved did not.
    use hf_sim::deity::{Answer, Lever, Scope, Trigger};
    use hf_sim::systems::weather::{self, Sky, Weather};
    let mut w = meadow(16001, 2);
    // A rainy hour, with the next hour in the same six-hour sky.
    let seed = w.meta.seed;
    let t = (w.meta.tick..w.meta.tick + 4000 * TICKS_PER_HOUR)
        .step_by(TICKS_PER_HOUR as usize)
        .find(|&t| {
            Weather::at(seed, t).sky == Sky::Rain && Weather::at(seed, t + 1).sky == Sky::Rain
        })
        .expect("a rainy hour within the year");
    w.meta.tick = t;
    w.deity_answers.push(Answer {
        trigger: Trigger::Hardship,
        lever: Lever::Cold,
        scope: Scope::Valley,
        strength: -60.0,
        from: 0,
        until: u64::MAX,
        event: hf_sim::ids::EventId(0),
    });
    weather::step(&mut w);
    assert_eq!(w.weather.sky, Sky::Snow, "rain below zero falls as snow");
    w.events.drain();
    let mut back = decode(&encode(&Snapshot::capture(&w)).unwrap())
        .unwrap()
        .restore();
    w.meta.tick += 1;
    back.meta.tick += 1;
    weather::step(&mut w);
    weather::step(&mut back);
    assert_eq!(back.weather.sky, Sky::Snow);
    assert_eq!(
        weather_lines(&mut back),
        weather_lines(&mut w),
        "the loaded game says what the unsaved one says"
    );
}

fn order(
    w: &mut World,
    kind: hf_sim::work::DesignationKind,
    pos: TilePos,
    reqs: Vec<hf_sim::work::Requirement>,
) -> hf_sim::work::DesignationId {
    let id = w.alloc_designation_id();
    w.designations.push(hf_sim::work::Designation {
        id,
        kind,
        pos,
        requirements: reqs,
        work_left: 200,
        work_total: 200,
        max_workers: 1,
        priority: 5,
        created: w.tick(),
        failures: 0,
        done: false,
        community: None,
        owner: None,
    });
    id
}

fn req(
    kind: hf_sim::items::ItemKind,
    needed: u32,
    delivered: u32,
    material: hf_sim::world::Material,
) -> hf_sim::work::Requirement {
    hf_sim::work::Requirement {
        kind,
        needed,
        delivered,
        material,
        want: None,
        quality: 0.0,
    }
}

#[test]
fn a_dropped_order_gives_back_what_was_delivered() {
    // #162: materials delivered to an order were consumed at delivery, and an order dropped
    // unfinished (failed too often, owner gone, a collapse, a mood passed) never gave them back.
    use hf_sim::items::{ItemKind, ItemPlace};
    use hf_sim::structures::StructureKind;
    use hf_sim::work::DesignationKind;
    use hf_sim::world::Material;
    let mut w = meadow(16201, 2);
    let site = TilePos::new(44, 40, 2);
    let id = order(
        &mut w,
        DesignationKind::BuildStructure(StructureKind::Kiln, Material::GRANITE),
        site,
        vec![
            req(ItemKind::StoneBlock, 4, 4, Material::GRANITE),
            req(ItemKind::Log, 3, 2, Material::OAK),
        ],
    );
    w.designations
        .iter_mut()
        .find(|d| d.id == id)
        .unwrap()
        .failures = 13;
    w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(&mut w);
    assert!(
        w.designations.iter().all(|d| d.id != id),
        "the order was dropped"
    );
    let at = |k: ItemKind, m: Material| -> u32 {
        w.items
            .iter()
            .filter(|it| it.kind == k && it.material == m && it.place == ItemPlace::Ground(site))
            .map(|it| it.count)
            .sum()
    };
    assert_eq!(
        at(ItemKind::StoneBlock, Material::GRANITE),
        4,
        "the blocks lie at the site"
    );
    assert_eq!(
        at(ItemKind::Log, Material::OAK),
        2,
        "the logs lie at the site"
    );
    assert_eq!(w.ledger.by_cause.get("+returned"), Some(&6));
}

#[test]
fn a_finished_order_gives_nothing_back() {
    // #162's other side: what went into a finished work stays in it.
    use hf_sim::items::{ItemKind, ItemPlace};
    use hf_sim::work::DesignationKind;
    use hf_sim::world::Material;
    let mut w = meadow(16202, 2);
    let site = TilePos::new(36, 30, 2);
    order(
        &mut w,
        DesignationKind::BuildWall(Material::OAK),
        site,
        vec![req(ItemKind::Log, 1, 1, Material::OAK)],
    );
    for a in w.agents.iter_mut() {
        a.needs.hunger = 1.0;
        a.needs.rest = 1.0;
    }
    w.run(TICKS_PER_HOUR * 8);
    assert_eq!(
        w.grid.get(site).shape,
        hf_sim::world::Shape::Wall,
        "the wall stands"
    );
    assert!(
        !w.items
            .iter()
            .any(|it| it.kind == ItemKind::Log && it.place == ItemPlace::Ground(site)),
        "no log comes back from a wall that stands"
    );
    assert_eq!(w.ledger.by_cause.get("+returned"), None);
}

fn trading_post(seed: u64) -> (World, hf_sim::ids::CommunityId, u32) {
    let w = meadow(seed, 10);
    let cid = w.communities[0].id;
    let site = w.overworld.sites[0].id;
    (w, cid, site)
}

fn at_hour_six_of_next_day(w: &mut World) {
    use hf_sim::time::TICKS_PER_DAY;
    w.meta.tick = (w.meta.tick / TICKS_PER_DAY + 1) * TICKS_PER_DAY + 6 * TICKS_PER_HOUR;
}

#[test]
fn a_caravan_carrying_meals_keeps_its_rations() {
    // #165: rations rode in the cargo as meals, and a party trading meals sold every meal it had.
    use hf_sim::expedition::{CaravanState, Purpose};
    use hf_sim::items::ItemKind;
    let (mut w, cid, site) = trading_post(16501);
    let home = w.communities[0].home;
    w.drop_item(ItemKind::Meal, 10, home.offset(1, 0, 0));
    w.drop_item(ItemKind::Meal, 10, home.offset(2, 0, 0));
    w.drop_item(ItemKind::Meal, 10, home.offset(3, 0, 0));
    w.drop_item(ItemKind::Meal, 10, home.offset(4, 0, 0));
    hf_sim::systems::expedition::form_caravan(
        &mut w,
        cid,
        Purpose::Trade {
            site,
            carry: ItemKind::Meal,
        },
    )
    .expect("a party forms");
    let rations = w.overworld.caravans[0].rations;
    assert!(rations > 0, "they took food for the road");
    w.overworld.caravans[0].state = CaravanState::AtSite { day: 0 };
    at_hour_six_of_next_day(&mut w);
    hf_sim::systems::expedition::step(&mut w);
    let c = &w.overworld.caravans[0];
    assert!(c
        .log
        .iter()
        .any(|l| l.contains("traded") || l.contains("nothing")));
    assert_eq!(
        c.rations, rations,
        "the road's food is not sold with the goods"
    );
}

#[test]
fn a_caravan_that_never_leaves_sets_its_cargo_down_as_it_was() {
    // #166: a caravan nobody reached the gathering for ended with its goods and rations gone.
    // #169: what comes back keeps what it is made of (oak, not plain "logs").
    use hf_sim::expedition::{CaravanState, Purpose};
    use hf_sim::items::{ItemKind, ItemPlace};
    use hf_sim::world::Material;
    let (mut w, cid, site) = trading_post(16601);
    let home = w.communities[0].home;
    w.items.retain(|it| it.kind != ItemKind::Log);
    w.create_item_of(
        ItemKind::Log,
        Material::OAK,
        20,
        ItemPlace::Ground(home.offset(1, 1, 0)),
        0.8,
        "test",
    );
    hf_sim::systems::expedition::form_caravan(
        &mut w,
        cid,
        Purpose::Trade {
            site,
            carry: ItemKind::Log,
        },
    )
    .expect("a party forms");
    let depart = w.overworld.caravans[0].depart_tile;
    let loaded = w.overworld.caravans[0].cargo[0].1;
    assert!(loaded > 0);
    // Nobody gets there: they are all kept far from the gathering, and the day runs out.
    let members = w.overworld.caravans[0].members.clone();
    for a in w.agents.iter_mut().filter(|a| members.contains(&a.id)) {
        a.pos = home;
        a.path.clear();
    }
    w.overworld.caravans[0].formed = 0;
    w.meta.tick = (w.meta.tick / 200 + 1) * 200 + 17 + 200 * 100;
    hf_sim::systems::expedition::step(&mut w);
    assert_eq!(w.overworld.caravans[0].state, CaravanState::Done);
    let back: Vec<_> = w
        .items
        .iter()
        .filter(|it| {
            it.kind == ItemKind::Log
                && matches!(it.place, ItemPlace::Ground(p) if p.dist(depart) <= 1)
        })
        .collect();
    assert_eq!(
        back.iter().map(|it| it.count).sum::<u32>(),
        loaded,
        "the logs are set down"
    );
    assert!(back.iter().all(|it| it.material == Material::OAK), "as oak");
    assert!(
        back.iter().all(|it| (it.quality - 0.8).abs() < 1e-3),
        "as good as they went"
    );
}

#[test]
fn a_party_lost_whole_marks_its_road() {
    // #167: a party lost whole had its member list emptied first, so the road counted nobody.
    use hf_sim::expedition::{CaravanState, Purpose};
    let (mut w, cid, site) = trading_post(16701);
    hf_sim::systems::expedition::form_caravan(&mut w, cid, Purpose::Scout { site })
        .expect("a party forms");
    let n = w.overworld.caravans[0].members.len();
    assert_eq!(hf_sim::trade::lost_on(&w, cid, site), 0);
    // Stranded with no food, day after day, until the road takes them.
    for _ in 0..200 {
        if w.overworld.caravans[0].state == CaravanState::Lost {
            break;
        }
        let c = &mut w.overworld.caravans[0];
        c.cargo.clear();
        c.rations = 0;
        c.state = CaravanState::Stranded {
            days: 3,
            then_return: false,
            back_day: 0,
        };
        at_hour_six_of_next_day(&mut w);
        hf_sim::systems::expedition::step(&mut w);
    }
    assert_eq!(w.overworld.caravans[0].state, CaravanState::Lost);
    assert_eq!(
        hf_sim::trade::lost_on(&w, cid, site),
        n,
        "every one of them counts"
    );
}

#[test]
fn a_storied_hide_is_never_loaded_for_trade() {
    // #169: loading took any stack of the kind, the White Hart's hide among them.
    use hf_sim::expedition::Purpose;
    use hf_sim::items::{ItemKind, ItemPlace};
    let (mut w, cid, site) = trading_post(16901);
    let home = w.communities[0].home;
    let hart = w.create_lone_item(
        ItemKind::Hide,
        1,
        ItemPlace::Ground(home.offset(1, 0, 0)),
        0.9,
        "test",
    );
    w.items.iter_mut().find(|it| it.id == hart).unwrap().story =
        Some(Box::new(hf_sim::legend::Story {
            maker_name: "the White Hart".into(),
            name: Some("the White Hart's hide".into()),
            ..Default::default()
        }));
    w.drop_item(ItemKind::Hide, 3, home.offset(2, 0, 0));
    hf_sim::systems::expedition::form_caravan(
        &mut w,
        cid,
        Purpose::Trade {
            site,
            carry: ItemKind::Hide,
        },
    )
    .expect("a party forms");
    assert_eq!(
        w.overworld.caravans[0].cargo[0].1, 3,
        "the plain hides only"
    );
    assert!(
        w.items.iter().any(|it| it.id == hart),
        "the trophy stays at home"
    );
}

fn storied(
    w: &mut World,
    kind: hf_sim::items::ItemKind,
    at: TilePos,
    name: &str,
) -> hf_sim::ids::ItemId {
    let id = w.create_lone_item(kind, 1, hf_sim::items::ItemPlace::Ground(at), 0.9, "test");
    w.items.iter_mut().find(|it| it.id == id).unwrap().story =
        Some(Box::new(hf_sim::legend::Story {
            maker_name: "the hunt".into(),
            name: Some(name.into()),
            ..Default::default()
        }));
    id
}

#[test]
fn a_storied_hide_does_not_rot() {
    // #170: the White Hart's hide rotted in twenty days in the open, and was "1 hides" in the
    // day's line. Dan, 2026-10-20: a thing with a story does not rot.
    use hf_sim::items::ItemKind;
    let mut w = meadow(17001, 1);
    let at = TilePos::new(50, 50, 2);
    let hart = storied(&mut w, ItemKind::Hide, at, "the White Hart's hide");
    w.drop_item(ItemKind::Hide, 4, at.offset(1, 0, 0));
    for it in w.items.iter_mut().filter(|it| it.kind == ItemKind::Hide) {
        it.rot = 0.999;
    }
    w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR + 37;
    hf_sim::systems::spoil::step(&mut w);
    assert!(w.items.iter().any(|it| it.id == hart), "the trophy keeps");
    let plain: u32 = w
        .items
        .iter()
        .filter(|it| it.kind == ItemKind::Hide && it.story.is_none())
        .map(|it| it.count)
        .sum();
    assert!(plain < 4, "a plain hide still rots: {plain}");
}

#[test]
fn a_storied_hide_lost_to_fire_is_named() {
    // #170: fire took storied things with no line naming them.
    use hf_sim::items::ItemKind;
    let mut w = meadow(17002, 1);
    w.meta.autoplan = false;
    // Nobody about to snatch it from the flames.
    w.agents.clear();
    let at = TilePos::new(50, 50, 2);
    let mut dry = w.weather;
    dry.sky = hf_sim::systems::weather::Sky::Clear;
    dry.temperature_c = 24.0;
    w.force_weather = Some(dry);
    // The grass under the hide carries a fire only in dry heat (C71, #258): the weather of
    // the moment, not only the forced one the next step will read.
    w.weather = dry;
    assert!(w.ignite(at, "in a test"), "it takes fire");
    // It burns out within the hour, before anyone could beat it out.
    for f in w.fires.iter_mut() {
        f.left = 50;
    }
    // Dropped where it burns (a fire pushes off what lies there as it catches).
    storied(&mut w, ItemKind::Clothes, at, "the Wedding Coat");
    let mut named = false;
    for _ in 0..TICKS_PER_HOUR * 12 {
        w.step();
        named |= w.last_events.iter().any(
            |e| matches!(&e.kind, EventKind::Burned { what, .. } if what.contains("Wedding Coat")),
        );
        if named {
            break;
        }
    }
    assert!(named, "its burning is named");
}

#[test]
fn hauling_onto_a_stack_respects_cap_and_quality() {
    // #171: a haul merged into any stack of its kind on the tile, past the kind's stack, at the
    // target's quality: ten fine meals onto ten crude made twenty crude.
    use hf_sim::items::{ItemKind, ItemPlace};
    let mut w = meadow(17101, 1);
    let who = w.agents[0].id;
    let at = TilePos::new(44, 44, 2);
    let lone = |w: &mut World, n: u32, q: f32, place: ItemPlace| {
        w.create_lone_item(ItemKind::Meal, n, place, q, "test")
    };
    lone(&mut w, 10, 0.3, ItemPlace::Ground(at));
    let fine = lone(&mut w, 10, 0.9, ItemPlace::Carried(who));
    let ix = w.items.iter().position(|it| it.id == fine).unwrap();
    hf_sim::systems::work::put_down(&mut w, ix, at);
    let mut stacks: Vec<(u32, f32)> = w
        .items
        .iter()
        .filter(|it| it.kind == ItemKind::Meal && it.place == ItemPlace::Ground(at))
        .map(|it| (it.count, it.quality))
        .collect();
    stacks.sort_by(|a, b| a.1.total_cmp(&b.1));
    assert_eq!(
        stacks,
        vec![(10, 0.3), (10, 0.9)],
        "two stacks of ten, each as good as it was"
    );
    // Five fine onto five crude: one stack, between the two.
    let at2 = at.offset(2, 0, 0);
    lone(&mut w, 5, 0.3, ItemPlace::Ground(at2));
    let fine = lone(&mut w, 5, 0.9, ItemPlace::Carried(who));
    let ix = w.items.iter().position(|it| it.id == fine).unwrap();
    hf_sim::systems::work::put_down(&mut w, ix, at2);
    let one: Vec<_> = w
        .items
        .iter()
        .filter(|it| it.kind == ItemKind::Meal && it.place == ItemPlace::Ground(at2))
        .collect();
    assert_eq!(one.len(), 1);
    assert_eq!(one[0].count, 10);
    assert!((one[0].quality - 0.6).abs() < 1e-4, "{}", one[0].quality);
}

#[test]
fn the_cook_cooks_the_planned_food_one_to_a_meal() {
    // #172: the cook cooked the first cookable thing in hand, not the food the plan fetched,
    // and took "two" (one, when one was carried). Dan, 2026-10-20: one raw thing is a meal.
    use hf_sim::agents::Species;
    use hf_sim::items::{ItemKind, ItemPlace};
    use hf_sim::structures::StructureKind;
    use hf_sim::work::DesignationKind;
    use hf_sim::world::Material;
    let mut w = World::generate(17201, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    w.ensure_nav();
    let hearth = TilePos::new(30, 26, 2);
    w.place_structure(StructureKind::Hearth, hearth, Material::STONE_BLOCK, true);
    let cook = w.spawn_agent(Species::Person, TilePos::new(30, 30, 2));
    w.create_item(ItemKind::Berries, 1, ItemPlace::Carried(cook), 0.5, "test");
    w.drop_item(ItemKind::RawMeat, 3, TilePos::new(32, 30, 2));
    order(&mut w, DesignationKind::Cook, hearth, vec![]);
    let mut meal = None;
    for _ in 0..TICKS_PER_HOUR * 6 {
        if let Some(a) = w.agents.iter_mut().find(|a| a.id == cook) {
            a.needs.hunger = 1.0;
        }
        w.step();
        if let Some(m) = w.last_events.iter().find_map(|e| match e.kind {
            EventKind::Cooked { item, .. } => Some(item),
            _ => None,
        }) {
            meal = Some(m);
            break;
        }
    }
    let meal = meal.expect("a meal was cooked");
    let m = w.items.iter().find(|it| it.id == meal).unwrap();
    assert_eq!(m.count, 1);
    assert_eq!(
        m.material,
        hf_sim::world::material::food_of(ItemKind::RawMeat),
        "of the meat it fetched"
    );
    let count = |k: ItemKind| -> u32 {
        w.items
            .iter()
            .filter(|it| it.kind == k)
            .map(|it| it.count)
            .sum()
    };
    assert_eq!(
        count(ItemKind::RawMeat),
        2,
        "one piece of meat went into it"
    );
    assert_eq!(
        count(ItemKind::Berries),
        1,
        "the berry in hand is still a berry"
    );
}

fn ring_and_coat(
    w: &mut World,
    on: hf_sim::ids::AgentId,
) -> (hf_sim::ids::ItemId, hf_sim::ids::ItemId) {
    use hf_sim::items::{ItemKind, ItemPlace};
    let at = TilePos::new(1, 1, 2);
    let ring = storied(w, ItemKind::Jewel, at, "the Garnet Ring");
    let coat = storied(w, ItemKind::Clothes, at, "the Wedding Coat");
    for id in [ring, coat] {
        w.items.iter_mut().find(|it| it.id == id).unwrap().place = ItemPlace::Held(on);
    }
    (ring, coat)
}

fn dies(w: &mut World, who: hf_sim::ids::AgentId) {
    hf_sim::systems::life::die(w, who, "old age".into());
    hf_sim::systems::flush::step(w);
    hf_sim::legend::step(w);
}

#[test]
fn a_ring_passes_to_kin_on_death() {
    // #168: rings and worn storied clothes stayed held by the dead for good.
    use hf_sim::items::ItemPlace;
    let mut w = meadow(16801, 3);
    let (a, b) = (w.agents[0].id, w.agents[1].id);
    w.agents[0].partner = Some(b);
    w.agents[1].partner = Some(a);
    let (ring, coat) = ring_and_coat(&mut w, b);
    dies(&mut w, b);
    let r = w.items.iter().find(|it| it.id == ring).unwrap();
    assert_eq!(r.place, ItemPlace::Held(a), "the ring goes to the widow");
    assert!(r.story.as_ref().unwrap().heirloom);
    let c = w.items.iter().find(|it| it.id == coat).unwrap();
    let apos = w.agents.iter().find(|x| x.id == a).unwrap().pos;
    assert_eq!(
        c.place,
        ItemPlace::Ground(apos),
        "the coat is set down beside them"
    );
}

#[test]
fn a_ring_with_no_kin_lies_where_they_fell() {
    use hf_sim::items::ItemPlace;
    let mut w = meadow(16802, 3);
    let b = w.agents[1].id;
    let (ring, _) = ring_and_coat(&mut w, b);
    let pos = w.agents[1].pos;
    dies(&mut w, b);
    let r = w.items.iter().find(|it| it.id == ring).unwrap();
    assert_eq!(r.place, ItemPlace::Ground(pos), "it lies where they fell");
}

/// A band of two camped at `camp`, at night, beside a valley founded at (30, 30).
fn camped_band(seed: u64, camp: TilePos) -> World {
    use hf_sim::agents::Species;
    use hf_sim::threat::{Band, BandState};
    let mut w = meadow(seed, 3);
    w.meta.autoplan = false;
    // Settlers fed and asleep in their beds of grass: the night is the band's.
    for a in w.agents.iter_mut() {
        a.needs.hunger = 1.0;
        a.needs.rest = 1.0;
    }
    let id = 1;
    let members: Vec<_> = (0..2)
        .map(|k| {
            let m = w.spawn_agent(Species::Person, camp.offset(k, 0, 0));
            w.agents.iter_mut().find(|a| a.id == m).unwrap().band = Some(id);
            m
        })
        .collect();
    let tick = w.meta.tick;
    w.bands.push(Band {
        id,
        site: w.overworld.sites[0].id,
        site_name: "Tollmoor".into(),
        members,
        size: 2,
        camp,
        state: BandState::Camped(tick),
        formed: tick,
        want: 12,
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
    // Eleven at night.
    w.meta.tick = (tick / hf_sim::time::TICKS_PER_DAY + 1) * hf_sim::time::TICKS_PER_DAY
        + 23 * TICKS_PER_HOUR;
    w
}

/// A room of stone walls round `p`, shut on every side.
fn walled_in(w: &mut World, p: TilePos) {
    use hf_sim::world::{Material, Tile};
    for dy in -2..=2i32 {
        for dx in -2..=2i32 {
            if dx.abs() == 2 || dy.abs() == 2 {
                w.grid
                    .set(p.offset(dx, dy, 0), Tile::solid(Material::GRANITE));
            }
        }
    }
    w.rebuild_caches();
    w.ensure_nav();
}

fn targets(w: &World) -> Vec<hf_sim::ids::ItemId> {
    w.bands
        .iter()
        .flat_map(|b| b.raiders.iter().map(|r| r.1))
        .collect()
}

#[test]
fn raiders_do_not_take_ordinary_food_they_never_saw() {
    // #173: raid targets were every food stack within thirty tiles of home, seen or not.
    use hf_sim::items::ItemKind;
    let camp = TilePos::new(8, 8, 2);
    let mut w = camped_band(17301, camp);
    // No food but what the test puts down.
    w.items.retain(|it| !it.kind.is_food());
    let hidden = TilePos::new(48, 48, 2);
    walled_in(&mut w, hidden);
    w.drop_item(ItemKind::Meal, 8, hidden);
    let at = |w: &World, p: TilePos| {
        w.items
            .iter()
            .find(|it| it.kind == ItemKind::Meal && it.place == hf_sim::items::ItemPlace::Ground(p))
            .unwrap()
            .id
    };
    let hid = at(&w, hidden);
    for _ in 0..TICKS_PER_HOUR * 3 {
        w.step();
        assert!(
            !targets(&w).contains(&hid),
            "they go for food they never saw"
        );
    }
    // What lies in their sight, they go for.
    w.drop_item(ItemKind::Meal, 8, camp.offset(4, 2, 0));
    let open = at(&w, camp.offset(4, 2, 0));
    let mut went = false;
    for _ in 0..TICKS_PER_HOUR * 2 {
        w.step();
        went |= targets(&w).contains(&open);
    }
    assert!(went, "food in their sight is food they go for");
}

#[test]
fn raiders_seek_a_renowned_piece_they_have_heard_of() {
    // #173, Dan's call: word of the valley's treasures may reach them, and they come for one.
    use hf_sim::items::ItemKind;
    let camp = TilePos::new(8, 8, 2);
    let mut w = camped_band(17302, camp);
    let home = w.communities[0].home;
    let crown = storied(
        &mut w,
        ItemKind::Jewel,
        home.offset(6, 6, 0),
        "the Antler Crown",
    );
    let plain = w.create_item(
        ItemKind::Jewel,
        1,
        hf_sim::items::ItemPlace::Ground(home.offset(7, 6, 0)),
        0.5,
        "test",
    );
    let heard = hf_sim::threat::heard_of(&w, home);
    assert!(
        heard.iter().any(|h| h.0 == crown),
        "a named thing is spoken of"
    );
    assert!(!heard.iter().any(|h| h.0 == plain), "a plain one is not");
    w.bands[0].heard = heard;
    let mut went = false;
    for _ in 0..TICKS_PER_HOUR * 2 {
        w.step();
        went |= targets(&w).contains(&crown);
    }
    assert!(went, "they come for the Antler Crown");
}

#[test]
fn a_treasure_moved_from_where_its_story_says_is_not_found() {
    // #173: they know a treasure by its telling, not by where it is now.
    use hf_sim::items::{ItemKind, ItemPlace};
    let camp = TilePos::new(8, 8, 2);
    let mut w = camped_band(17303, camp);
    let home = w.communities[0].home;
    let told = home.offset(-6, -6, 0);
    let crown = storied(&mut w, ItemKind::Jewel, told, "the Antler Crown");
    w.bands[0].heard = hf_sim::threat::heard_of(&w, home);
    // Taken indoors since the telling, out of their sight.
    let now = TilePos::new(48, 48, 2);
    walled_in(&mut w, now);
    w.items.iter_mut().find(|it| it.id == crown).unwrap().place = ItemPlace::Ground(now);
    for _ in 0..TICKS_PER_HOUR * 4 {
        w.step();
        if w.bands[0].heard.is_empty() {
            break;
        }
    }
    assert!(
        w.bands[0].heard.is_empty(),
        "they looked where it was said to be, and gave it up"
    );
    let it = w.items.iter().find(|it| it.id == crown).unwrap();
    assert_eq!(
        it.place,
        ItemPlace::Ground(now),
        "the crown is where it was put"
    );
}

#[test]
fn food_a_raider_splits_off_is_what_the_stack_was() {
    // #173: the part a raider took became plain food of middling quality, so food dropped by
    // a caught raider came back as something else.
    use hf_sim::items::{ItemKind, ItemPlace};
    use hf_sim::world::Material;
    let camp = TilePos::new(8, 8, 2);
    let mut w = camped_band(17304, camp);
    let stew = hf_sim::world::material::food_of(ItemKind::RawMeat);
    let id = w.create_item_of(
        ItemKind::Meal,
        stew,
        10,
        ItemPlace::Ground(camp.offset(4, 2, 0)),
        0.9,
        "test",
    );
    let mut part = None;
    for _ in 0..TICKS_PER_HOUR * 2 {
        w.step();
        part = w
            .items
            .iter()
            .find(|it| {
                it.kind == ItemKind::Meal
                    && it.id != id
                    && matches!(it.place, ItemPlace::Carried(_))
            })
            .map(|it| (it.material, it.quality));
        if part.is_some() {
            break;
        }
    }
    let (m, q) = part.expect("a raider took part of the stack");
    assert_eq!(m, stew);
    assert_ne!(m, Material::AIR);
    assert!((q - 0.9).abs() < 1e-4, "{q}");
}

/// The settler at agents[0], believing strangers bring harm and a fist settles things, idle so
/// decide will act on them this step.
fn hostile_settler(w: &mut World) -> hf_sim::ids::AgentId {
    use hf_sim::mind::beliefs::{Source, Topic};
    let a = &mut w.agents[0];
    let tick = w.meta.tick;
    a.beliefs.hold(
        Topic::Strangers,
        0.9,
        1.0,
        Source::Event(hf_sim::ids::EventId(0)),
        tick,
    );
    a.beliefs.hold(
        Topic::Violence,
        0.9,
        1.0,
        Source::Event(hf_sim::ids::EventId(0)),
        tick,
    );
    a.activity = hf_sim::mind::Activity::Idle;
    a.idle_ticks = 0;
    a.id
}

#[test]
fn pursuit_of_a_stranger_needs_personal_knowledge() {
    // #173 second half: the pursuit branch (decide.rs consider_bands) chose the nearest band
    // member by TRUE position, gated only by the band-level `sighted` flag set by ANYONE's
    // glance. A settler who never saw the stranger must not be offered a Confront.
    use hf_sim::mind::knowledge::Known;
    let camp = TilePos::new(8, 8, 2);
    let mut w = camped_band(17305, camp);
    w.bands[0].taken = 4;
    let me = w.agents[0].pos;
    let m = w.bands[0].members[0];
    // One of them sits thirty tiles off, out of sight and never seen.
    let far = me.offset(30, 2, 0);
    w.agents.iter_mut().find(|a| a.id == m).unwrap().pos = far;
    w.ensure_nav();
    assert!(
        w.agents[0].knowledge.get(Known::Agent(m)).is_none(),
        "the settler holds no knowledge of the stranger"
    );
    let s = hostile_settler(&mut w);
    hf_sim::systems::decide::step(&mut w);
    assert!(
        !w.bands[0].pursuits.iter().any(|p| p.0 == s),
        "they go after a stranger they never saw"
    );
    // Positive control: the same stranger stands ten tiles off in the open — in sight, so
    // the Confront IS offered.
    let near = me.offset(10, 0, 0);
    w.agents.iter_mut().find(|a| a.id == m).unwrap().pos = near;
    let s = hostile_settler(&mut w);
    hf_sim::systems::decide::step(&mut w);
    assert!(
        w.bands[0].pursuits.iter().any(|p| p.0 == s),
        "a stranger in the open is a stranger they go after"
    );
}

#[test]
fn pursuit_follows_what_is_known_of_the_stranger() {
    // #173 second half: a stale believed position is personal knowledge — the pursuit may
    // follow what they remember of the stranger, not only what is in sight now.
    use hf_sim::mind::knowledge::Known;
    let camp = TilePos::new(8, 8, 2);
    let mut w = camped_band(17306, camp);
    w.bands[0].taken = 4;
    let me = w.agents[0].pos;
    let m = w.bands[0].members[0];
    // They saw them at the camp once, long ago; the stranger has since moved far off.
    let tick = w.meta.tick;
    w.agents[0]
        .knowledge
        .note(Known::Agent(m), camp, tick - TICKS_PER_HOUR * 3, 0);
    let far = me.offset(30, 2, 0);
    w.agents.iter_mut().find(|a| a.id == m).unwrap().pos = far;
    w.ensure_nav();
    assert!(
        !hf_sim::mind::perception::can_see(&w.grid, me, far),
        "the stranger is out of sight"
    );
    let s = hostile_settler(&mut w);
    hf_sim::systems::decide::step(&mut w);
    assert!(
        w.bands[0].pursuits.iter().any(|p| p.0 == s),
        "a stranger they remember is a stranger they go after"
    );
}

#[test]
fn a_widow_is_free_to_pair_again_with_a_full_relationship_list() {
    // #175: the partner was freed only if they still had the dead in their relationships,
    // which a full list (64) can have evicted.
    let mut w = meadow(17501, 3);
    let (a, b) = (w.agents[0].id, w.agents[1].id);
    w.agents[0].partner = Some(b);
    w.agents[1].partner = Some(a);
    w.agents[0].relationships.remove(&b);
    hf_sim::systems::life::die(&mut w, b, "old age".into());
    assert_eq!(w.agents[0].partner, None, "free to pair again");
}

#[test]
fn a_partner_away_on_a_caravan_is_widowed() {
    // #175: someone away on a journey was never told.
    let mut w = meadow(17502, 3);
    let (a, b) = (w.agents[0].id, w.agents[1].id);
    w.agents[0].partner = Some(b);
    w.agents[1].partner = Some(a);
    let away = w.agents.remove(0);
    w.overworld.away.push(away);
    hf_sim::systems::life::die(&mut w, b, "old age".into());
    assert_eq!(w.overworld.away[0].partner, None);
}

#[test]
fn leaving_the_valley_ends_a_pairing() {
    // #175: one who left the valley stayed someone's partner for good.
    let mut w = meadow(17503, 3);
    let (a, b) = (w.agents[0].id, w.agents[1].id);
    w.agents[0].partner = Some(b);
    w.agents[1].partner = Some(a);
    hf_sim::systems::society::leave_valley(&mut w, 1);
    assert_eq!(w.agents[0].partner, None);
}
