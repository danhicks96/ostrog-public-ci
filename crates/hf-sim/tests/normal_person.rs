//! #415, Dan's rule 7 (docs/design-rules.md, D56): what would a normal person do here? Nobody
//! wanders, loiters, talks or sits about while their food, tomorrow's food or tonight's shelter
//! is not secured; a secure community still talks and wanders.

use hf_sim::ids::{AgentId, TilePos};
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::mind::knowledge::Known;
use hf_sim::mind::Activity;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

/// A flat meadow with a party of `n` at (20, 20), at `hour` on day 30, nothing growing, no
/// planner, and every stack of food taken away.
fn camp(seed: u64, n: usize, hour: u64) -> (World, Vec<AgentId>) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(20, 20, 2);
    w.meta.tick = TICKS_PER_DAY * 30 + TICKS_PER_HOUR * hour;
    w.rebuild_caches();
    let ids = w.found_party(n);
    w.items.retain(|it| !it.kind.is_food());
    w.ensure_nav();
    for a in w.agents.iter_mut() {
        a.needs.hunger = 0.95;
        a.needs.rest = 0.9;
        a.needs.social = 0.9;
        a.needs.warmth = 1.0;
        a.activity = Activity::Idle;
        a.idle_ticks = 0;
    }
    (w, ids)
}

fn idx(w: &World, id: AgentId) -> usize {
    w.agents.iter().position(|a| a.id == id).unwrap()
}

/// Ten days of meals for everyone, by the camp.
fn stock(w: &mut World, n: usize) {
    let at = w.grid.surface(22, 18);
    w.drop_item(ItemKind::Meal, (n * 30) as u32, at);
}

/// The first person lonely and knowing the others are about.
fn lonely(w: &mut World, ids: &[AgentId]) {
    let i = idx(w, ids[0]);
    let t = w.meta.tick;
    w.agents[i].needs.social = 0.1;
    for o in &ids[1..] {
        let p = w.agents[idx(w, *o)].pos;
        w.agents[i].knowledge.note(Known::Agent(*o), p, t, 0);
    }
}

fn decide(w: &mut World) {
    hf_sim::systems::decide::step(w);
}

fn going_to(a: &hf_sim::agents::Agent) -> String {
    match &a.activity {
        Activity::Going { then, .. } => format!("going to {}", then.name()),
        other => other.name().to_string(),
    }
}

#[test]
fn a_starving_wanderer_who_knows_of_food_goes_to_it() {
    // Wandering was left alone when starving, "already the search for food"; one who knows
    // where food is is not searching.
    let (mut w, ids) = camp(4151, 1, 10);
    let at = w.grid.surface(26, 20);
    w.drop_item(ItemKind::Meal, 4, at);
    let food = w.items.last().unwrap().id;
    let i = idx(&w, ids[0]);
    let t = w.meta.tick;
    w.agents[i]
        .knowledge
        .note(Known::Item(food), at, t, ItemKind::Meal as u16);
    let far = w.grid.surface(50, 50);
    assert!(w.path_agent_to(i, far));
    w.agents[i].activity = Activity::Wandering { to: far };
    w.agents[i].activity_ticks = 400;
    w.agents[i].needs.hunger = 0.05;
    decide(&mut w);
    assert!(
        matches!(&w.agents[i].activity, Activity::Going { to, then } if *to == at && matches!(**then, Activity::Eating { .. })),
        "{}",
        going_to(&w.agents[i])
    );
}

#[test]
fn stores_under_two_days_turn_a_talker_into_a_forager() {
    let run = |stocked: bool| {
        let (mut w, ids) = camp(4152, 3, 10);
        if stocked {
            stock(&mut w, 3);
        }
        let bush = TilePos::new(26, 22, 2);
        w.plants
            .push(Plant::new(bush, PlantKind::BerryBush, 1.0, 1));
        lonely(&mut w, &ids);
        let i = idx(&w, ids[0]);
        let t = w.meta.tick;
        w.agents[i]
            .knowledge
            .note(Known::FoodPlant(bush), bush, t, PlantKind::BerryBush as u16);
        decide(&mut w);
        (w.agents[i].activity.clone(), going_to(&w.agents[i]))
    };
    let (short, said) = run(false);
    assert!(
        matches!(&short, Activity::Going { then, .. } if matches!(**then, Activity::Foraging { .. })),
        "stores empty, not hungry yet, lonely: {said}"
    );
    // The same person with ten days in store talks (a secure community still talks).
    let (fed, said) = run(true);
    assert!(
        matches!(&fed, Activity::Going { then, .. } if matches!(**then, Activity::Talking { .. })),
        "stores full: {said}"
    );
}

#[test]
fn gathering_for_the_store_leaves_the_food_for_the_store() {
    let (mut w, ids) = camp(4153, 1, 10);
    let i = idx(&w, ids[0]);
    let bush = w.agents[i].pos.offset(1, 0, 0);
    w.plants
        .push(Plant::new(bush, PlantKind::BerryBush, 1.0, 1));
    w.agents[i].activity = Activity::Foraging {
        plant: bush,
        ticks_left: 1,
    };
    let fed = w.agents[i].needs.hunger;
    w.step();
    let berries = w
        .items
        .iter()
        .find(|it| it.kind == ItemKind::Berries)
        .expect("gathered");
    assert!(
        matches!(berries.place, ItemPlace::Ground(_)),
        "{:?}",
        berries.place
    );
    assert!(!matches!(w.agents[i].activity, Activity::Eating { .. }));
    assert!(w.agents[i].needs.hunger <= fed);
}

#[test]
fn dusk_with_no_bed_sends_people_under_the_roof() {
    let run = |dusk: bool| {
        let (mut w, ids) = camp(4154, 3, if dusk { 20 } else { 14 });
        stock(&mut w, 3);
        w.build_house(
            12,
            10,
            7,
            6,
            1,
            hf_sim::world::Material::WOOD,
            hf_sim::world::Material::SOIL,
        );
        w.rebuild_caches();
        w.ensure_nav();
        let cid = w.agents[idx(&w, ids[0])].community.unwrap();
        w.communities
            .iter_mut()
            .find(|c| c.id == cid)
            .unwrap()
            .house_sites
            .push(TilePos::new(12, 10, 2));
        lonely(&mut w, &ids);
        let i = idx(&w, ids[0]);
        decide(&mut w);
        let indoors = match &w.agents[i].activity {
            Activity::Going { to, .. } => hf_sim::systems::hazards::indoors(&w, *to),
            _ => false,
        };
        (indoors, going_to(&w.agents[i]))
    };
    let (indoors, said) = run(true);
    assert!(indoors, "at dusk, no bed known: {said}");
    let (indoors, said) = run(false);
    assert!(!indoors, "by day they talk: {said}");
}

#[test]
fn one_far_from_camp_with_nothing_secured_walks_back() {
    let (mut w, ids) = camp(4155, 3, 10);
    let i = idx(&w, ids[0]);
    let cid = w.agents[i].community.unwrap();
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    w.agents[i].pos = w.grid.surface(54, 54);
    w.ensure_nav();
    decide(&mut w);
    assert!(
        matches!(&w.agents[i].activity, Activity::Going { to, .. } if to.dist(home) <= 12),
        "{} at {:?}, home {:?}",
        going_to(&w.agents[i]),
        w.agents[i].pos,
        home
    );
}

#[test]
fn hungry_with_no_food_known_searches_where_they_ate_before() {
    let (mut w, ids) = camp(4156, 1, 10);
    let i = idx(&w, ids[0]);
    let once = w.grid.surface(28, 26);
    let t = w.meta.tick;
    w.agents[i].remember(
        t - TICKS_PER_DAY,
        hf_sim::mind::MemKind::Meal,
        hf_sim::mind::Subject::Place(once),
        0.3,
        0.3,
        None,
    );
    w.agents[i].needs.hunger = 0.35;
    decide(&mut w);
    assert!(
        matches!(&w.agents[i].activity, Activity::Going { to, .. } if *to == once),
        "{}",
        going_to(&w.agents[i])
    );
}

#[test]
fn a_secure_community_still_talks_and_wanders() {
    let (mut w, ids) = camp(4157, 6, 8);
    stock(&mut w, 6);
    for a in w.agents.iter_mut() {
        a.needs.social = 0.5;
    }
    let (mut talk, mut wander) = (0u32, 0u32);
    for _ in 0..12 * TICKS_PER_HOUR {
        w.step();
        // Talk is counted as the talks held, not as who is seen talking or setting out to
        // at each quarter hour (#553, FIXLOG 2026-11-01): a talk lasts six minutes, so the
        // sample mostly caught the walks to a talk, and those that came to nothing counted.
        talk += w
            .last_events
            .iter()
            .filter(|e| matches!(e.kind, hf_sim::events::EventKind::Talked { .. }))
            .count() as u32;
        if w.meta.tick.is_multiple_of(TICKS_PER_HOUR / 4) {
            for id in &ids {
                if let Activity::Wandering { .. } = &w.agents[idx(&w, *id)].activity {
                    wander += 1;
                }
            }
        }
    }
    assert!(talk >= 4 && wander >= 4, "talk {talk}, wander {wander}");
}

#[test]
fn nobody_dives_into_a_flooded_cellar_for_food() {
    // The moved-camp census (#415): the stores of a flooded cellar drew the starving down
    // again and again, and the water turned them back each time, until eight starved.
    use hf_sim::world::TileFlags;
    let (mut w, ids) = camp(4158, 1, 10);
    let i = idx(&w, ids[0]);
    let me = w.agents[i].pos;
    let wet = me.offset(3, 0, 0);
    let dry = me.offset(9, 0, 0);
    w.drop_item(ItemKind::Meal, 4, wet);
    let drowned = w.items.last().unwrap().id;
    w.drop_item(ItemKind::Root, 4, dry);
    let root = w.items.last().unwrap().id;
    let mut t = w.grid.get(wet);
    t.flags = TileFlags(TileFlags::WATER | TileFlags::FULL);
    w.grid.set(wet, t);
    w.rebuild_caches();
    w.ensure_nav();
    assert!(hf_sim::systems::hazards::under_water(&w, wet));
    let now = w.meta.tick;
    w.agents[i]
        .knowledge
        .note(Known::Item(drowned), wet, now, ItemKind::Meal as u16);
    w.agents[i]
        .knowledge
        .note(Known::Item(root), dry, now, ItemKind::Root as u16);
    w.agents[i].needs.hunger = 0.1;
    decide(&mut w);
    assert!(
        matches!(&w.agents[i].activity, Activity::Going { to, then } if *to == dry && matches!(**then, Activity::Eating { item, .. } if item == root)),
        "{}",
        going_to(&w.agents[i])
    );
}

#[test]
fn a_hungry_band_with_nothing_known_growing_looks_round_its_camp() {
    use hf_sim::agents::Species;
    use hf_sim::threat::{Band, BandState};
    let (mut w, _) = camp(4159, 1, 10);
    let at = w.grid.surface(40, 40);
    let m = w.spawn_agent(Species::Person, at);
    let i = idx(&w, m);
    w.agents[i].band = Some(1);
    w.agents[i].needs.hunger = 0.2;
    w.agents[i].needs.rest = 0.9;
    let tick = w.meta.tick;
    w.bands.push(Band {
        id: 1,
        site: w.overworld.sites[0].id,
        site_name: "Tollmoor".into(),
        members: vec![m],
        size: 1,
        camp: at,
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
    w.ensure_nav();
    decide(&mut w);
    assert!(
        matches!(&w.agents[i].activity, Activity::Going { to, .. } if to.dist(at) <= 12 && *to != at),
        "{}",
        going_to(&w.agents[i])
    );
}

#[test]
fn other_systems_can_ask_whether_someone_is_secured() {
    // For gatherings called outside the decision (an observance, #415): the same test.
    let (mut w, ids) = camp(4160, 2, 11);
    let i = idx(&w, ids[0]);
    assert!(!hf_sim::systems::decide::is_secured(&w, i), "stores empty");
    stock(&mut w, 2);
    assert!(
        hf_sim::systems::decide::is_secured(&w, i),
        "fed and stocked"
    );
    w.agents[i].needs.hunger = 0.3;
    assert!(!hf_sim::systems::decide::is_secured(&w, i), "hungry");
}

#[test]
fn a_community_short_of_food_does_not_gather_for_its_observance() {
    // An hour before noon on a season's first day those free walk to the shrine or home
    // (observances.rs `gather`); not while tomorrow's food is not secured.
    let run = |stocked: bool| {
        let (mut w, ids) = camp(4161, 4, 10);
        if stocked {
            stock(&mut w, 4);
        }
        let home = w.communities[0].home;
        for (k, id) in ids.iter().enumerate() {
            let i = idx(&w, *id);
            w.agents[i].pos = w.grid.surface(home.x + 5 + k as i32, home.y);
        }
        w.ensure_nav();
        w.meta.tick = w.meta.epoch + 5 * TICKS_PER_DAY + 11 * TICKS_PER_HOUR;
        hf_sim::observances::step(&mut w);
        w.agents
            .iter()
            .filter(|a| matches!(&a.activity, Activity::Going { to, .. } if to.dist(home) <= 3))
            .count()
    };
    assert_eq!(run(false), 0, "short of food, nobody goes");
    assert_eq!(run(true), 4, "stocked, all four go");
}

#[test]
fn nobody_walks_back_to_camp_through_water() {
    // Safety first (FIXLOG 2026-10-28): on seed 1 the walk back to camp crossed a spate and
    // eight drowned in it.
    use hf_sim::world::TileFlags;
    let (mut w, ids) = camp(4162, 3, 10);
    let i = idx(&w, ids[0]);
    let home = w.communities[0].home;
    w.agents[i].pos = w.grid.surface(54, 54);
    // A band of rising flood water from edge to edge between them and camp.
    w.waters.bodies.push(hf_sim::floods::Body {
        stage: 10.0,
        high: true,
        ..Default::default()
    });
    let body = (w.waters.bodies.len() - 1) as u16;
    for y in 2..62 {
        for x in 38..41 {
            let p = w.grid.surface(x, y);
            let mut t = w.grid.get(p);
            t.flags = TileFlags(TileFlags::WATER);
            w.grid.set(p, t);
            w.waters.flood_of.insert(p, body);
            w.flooded.insert(p);
        }
    }
    w.rebuild_caches();
    w.ensure_nav();
    decide(&mut w);
    let a = &w.agents[i];
    assert!(
        !matches!(&a.activity, Activity::Going { to, .. } if to.dist(home) <= 12),
        "{} with path {:?}",
        going_to(a),
        a.path.len()
    );
    assert!(!a.path.iter().any(|p| hf_sim::systems::decide::wet(&w, *p)));
}

#[test]
fn a_flooded_roof_is_no_shelter() {
    use hf_sim::world::TileFlags;
    let (mut w, ids) = camp(4163, 1, 20);
    w.build_house(
        12,
        10,
        7,
        6,
        1,
        hf_sim::world::Material::WOOD,
        hf_sim::world::Material::SOIL,
    );
    w.rebuild_caches();
    let i = idx(&w, ids[0]);
    let inside = TilePos::new(15, 12, 2);
    w.agents[i].pos = inside;
    let a = &w.agents[i];
    assert!(!hf_sim::systems::decide::secured(&w, a, Some(10.0)).roofless);
    let mut t = w.grid.get(inside);
    t.flags = TileFlags(TileFlags::WATER);
    w.grid.set(inside, t);
    let a = &w.agents[i];
    assert!(hf_sim::systems::decide::secured(&w, a, Some(10.0)).roofless);
}

#[test]
fn tired_at_dusk_with_no_bed_they_lie_down_at_camp() {
    let run = |temp: f32| {
        let (mut w, ids) = camp(4164, 2, 20);
        stock(&mut w, 2);
        w.weather.temperature_c = temp;
        let i = idx(&w, ids[0]);
        w.agents[i].needs.rest = 0.3;
        decide(&mut w);
        (w.agents[i].activity.is_asleep(), going_to(&w.agents[i]))
    };
    let (asleep, said) = run(14.0);
    assert!(asleep, "a mild evening: {said}");
    // In the cold they do not lie down out of doors before night (seed 4 froze five so).
    let (asleep, said) = run(-6.0);
    assert!(!asleep, "a freezing evening: {said}");
}

#[test]
fn a_shallow_stream_in_its_banks_does_not_keep_the_cold_from_the_hearth() {
    // Seed 4: with every water tile refused, four turned back from the walk to the hearth across
    // a stream, again and again, and froze.
    use hf_sim::world::TileFlags;
    let (mut w, ids) = camp(4165, 1, 1);
    stock(&mut w, 1);
    let i = idx(&w, ids[0]);
    let hearth = w.grid.surface(20, 24);
    w.place_structure(
        hf_sim::structures::StructureKind::Hearth,
        hearth,
        hf_sim::world::Material::STONE_BLOCK,
        true,
    );
    let sid = w.structures.last().unwrap().id;
    for y in 2..62 {
        let p = w.grid.surface(30, y);
        let mut t = w.grid.get(p);
        t.flags = TileFlags(TileFlags::WATER);
        w.grid.set(p, t);
    }
    w.agents[i].pos = w.grid.surface(40, 24);
    w.rebuild_caches();
    w.ensure_nav();
    let now = w.meta.tick;
    w.agents[i].knowledge.note(
        Known::Structure(sid),
        hearth,
        now,
        hf_sim::structures::StructureKind::Hearth as u16,
    );
    w.weather.temperature_c = -8.0;
    w.agents[i].needs.warmth = 0.2;
    decide(&mut w);
    assert!(
        matches!(&w.agents[i].activity, Activity::Going { to, .. } if to.dist(hearth) <= 1),
        "{}",
        going_to(&w.agents[i])
    );
}
