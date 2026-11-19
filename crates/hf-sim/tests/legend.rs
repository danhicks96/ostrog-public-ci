//! Things with a life (issue #49): a tool has a maker, a date, a material and a story; it wears
//! and breaks with its history told; one made exceptionally is named; long use makes one
//! cherished, deeds give it a name and then a legend; a cherished tool passes to kin.

use hf_sim::agents::Species;
use hf_sim::events::EventKind;
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::work::DesignationKind;
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

#[path = "../examples/possessions_census.rs"]
mod possessions_census;

fn small(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.agents.retain(|a| a.species == Species::Person);
    w
}

/// A tool in agent `i`'s hand, made by agent `maker` at quality `q`.
fn held_tool(w: &mut World, i: usize, maker: usize, q: f32) -> hf_sim::ids::ItemId {
    let p = w.agents[i].pos;
    let id = w.create_item(ItemKind::Tool, 1, ItemPlace::Ground(p), q, "test");
    w.items.iter_mut().find(|it| it.id == id).unwrap().material = Material::IRON_ORE;
    hf_sim::legend::made(w, id, maker);
    hf_sim::legend::take_up(w, i, id);
    w.agents[i].tool_quality = q;
    id
}

fn events(w: &World) -> Vec<String> {
    w.events
        .pending
        .iter()
        .map(|e| hf_sim::view::narrate::line(w, e))
        .collect()
}

#[test]
fn a_tool_knows_who_made_it_when_and_of_what() {
    let mut w = small(1);
    let p = w.meta.founding_site;
    let a = w.spawn_agent(Species::Person, p);
    let b = w.spawn_agent(Species::Person, p);
    let (ia, ib) = (
        w.agents.iter().position(|x| x.id == a).unwrap(),
        w.agents.iter().position(|x| x.id == b).unwrap(),
    );
    let id = held_tool(&mut w, ib, ia, 0.5);
    let it = w.items.iter().find(|it| it.id == id).unwrap();
    assert_eq!(it.place, ItemPlace::Held(b));
    let s = it.story.as_ref().expect("a story");
    assert_eq!(s.maker, Some(a));
    assert_eq!(s.owners.last().map(|o| o.0), Some(b));
    let d = hf_sim::legend::describe(&w, id).unwrap();
    let maker_name = w.agents[ia].name.clone();
    assert!(d.contains("iron tool") && d.contains(&maker_name), "{d}");
    assert!(d.contains("of year 1") && d.contains("nearly new"), "{d}");
    // The tool the party brought has its own beginning.
    let w = World::opening_world(3);
    // (Not a relic from the valley's past: those say where they were left.)
    let t = w
        .items
        .iter()
        .find(|it| {
            it.kind == ItemKind::Tool && it.story.as_ref().is_some_and(|s| s.origin.is_empty())
        })
        .unwrap();
    assert_eq!(
        t.story.as_ref().unwrap().maker_name,
        "brought through the portal"
    );
}

#[test]
fn a_tool_wears_and_breaks_and_the_feed_tells_its_life() {
    let mut w = small(2);
    let p = w.meta.founding_site;
    let a = w.spawn_agent(Species::Person, p);
    let ia = w.agents.iter().position(|x| x.id == a).unwrap();
    let id = held_tool(&mut w, ia, ia, 0.5);
    for _ in 0..30 {
        hf_sim::legend::used(&mut w, ia, DesignationKind::BuildWall(Material::WOOD));
    }
    let mut fresh = true;
    let mut seen = Vec::new();
    for step in 0..2_000_000 {
        if !hf_sim::legend::wear(&mut w, ia, 1.0 / 50_000.0) {
            assert!(step > 1000, "it lasted");
            fresh = false;
            break;
        }
        if step % 10_000 == 0 {
            seen.push(hf_sim::legend::describe(&w, id).unwrap());
        }
    }
    assert!(!fresh, "it broke in the end");
    assert!(
        seen.iter().any(|d| d.contains("worn")),
        "wear shows on the way: {seen:?}"
    );
    assert!(
        !w.items.iter().any(|it| it.id == id),
        "a broken tool is gone"
    );
    assert_eq!(w.agents[ia].tool, None);
    let lines = events(&w);
    let broke = lines
        .iter()
        .find(|l| l.contains("broke"))
        .expect("a feed line");
    assert!(
        broke.contains("made it in") && broke.contains("walls of a house"),
        "{broke}"
    );
}

#[test]
fn a_fine_piece_is_named_when_it_is_made() {
    let mut w = small(3);
    let p = w.meta.founding_site;
    let a = w.spawn_agent(Species::Person, p);
    let ia = w.agents.iter().position(|x| x.id == a).unwrap();
    let plain = held_tool(&mut w, ia, ia, 0.6);
    assert!(w
        .items
        .iter()
        .find(|it| it.id == plain)
        .unwrap()
        .story
        .as_ref()
        .unwrap()
        .name
        .is_none());
    let fine = w.create_item(ItemKind::Hammer, 1, ItemPlace::Ground(p), 0.92, "test");
    hf_sim::legend::made(&mut w, fine, ia);
    let s = w
        .items
        .iter()
        .find(|it| it.id == fine)
        .unwrap()
        .story
        .clone()
        .unwrap();
    let name = s.name.expect("named");
    assert!(name.chars().next().unwrap().is_uppercase());
    assert!(events(&w)
        .iter()
        .any(|l| l.contains("so fine it was given a name") && l.contains(&name)));
}

#[test]
fn long_use_makes_a_tool_cherished_then_named_then_spoken_of() {
    let mut w = small(4);
    let p = w.meta.founding_site;
    let a = w.spawn_agent(Species::Person, p);
    let ia = w.agents.iter().position(|x| x.id == a).unwrap();
    let id = held_tool(&mut w, ia, ia, 0.6);
    let kinds = [
        DesignationKind::BuildWall(Material::WOOD),
        DesignationKind::BuildRoof(Material::THATCH),
        DesignationKind::Chop,
        DesignationKind::DigStairs,
        DesignationKind::Mine,
        DesignationKind::Craft(0),
    ];
    for k in 0..400 {
        hf_sim::legend::used(&mut w, ia, kinds[k % kinds.len()]);
    }
    let s = w
        .items
        .iter()
        .find(|it| it.id == id)
        .unwrap()
        .story
        .clone()
        .unwrap();
    assert!(s.cherished && s.name.is_some());
    assert!(!s.spoken_of, "not spoken of until it has been here a while");
    w.meta.tick += 11 * TICKS_PER_DAY;
    hf_sim::legend::used(&mut w, ia, DesignationKind::Chop);
    let s = w
        .items
        .iter()
        .find(|it| it.id == id)
        .unwrap()
        .story
        .clone()
        .unwrap();
    assert!(s.spoken_of, "deeds: {:?}", s.deeds);
    let lines = events(&w);
    assert!(lines.iter().any(|l| l.contains("has come to love")));
    assert!(lines.iter().any(|l| l.starts_with("They call")));
    assert!(
        lines.iter().any(|l| l.starts_with("People speak of")),
        "{lines:?}"
    );
    // A greater deed replaced the lesser of its kind.
    assert!(!s.deeds.iter().any(|d| d.1 == "raised the walls of a house"));
}

#[test]
fn a_cherished_tool_passes_to_kin() {
    let mut w = small(5);
    let p = w.meta.founding_site;
    let ids = w.found_party(4);
    let (parent, child) = (ids[0], ids[1]);
    let ip = w.agents.iter().position(|x| x.id == parent).unwrap();
    w.agents[ip].children.push(child);
    let id = held_tool(&mut w, ip, ip, 0.6);
    for _ in 0..(hf_sim::legend::CHERISH_USES + 5) {
        hf_sim::legend::used(&mut w, ip, DesignationKind::Chop);
    }
    // They die.
    w.agents[ip].alive = false;
    let tick = w.meta.tick;
    w.events.push(
        tick,
        Some(p),
        hf_sim::events::Severity::Major,
        EventKind::Died {
            agent: parent,
            cause: "in a test".into(),
        },
    );
    w.step();
    w.step();
    let it = w.items.iter().find(|it| it.id == id).unwrap();
    assert_eq!(it.place, ItemPlace::Held(child), "the child took it up");
    assert!(it.story.as_ref().unwrap().heirloom);
    let ic = w.agents.iter().position(|x| x.id == child).unwrap();
    assert_eq!(w.agents[ic].tool, Some(id));
    let d = hf_sim::legend::describe(&w, id).unwrap();
    assert!(d.contains("an heirloom"), "{d}");
}

/// #1120: an heirloom set down by its heir is seen on the ground, not with a living heir
/// holding it.
#[test]
fn possessions_census_sees_an_heirloom_set_down() {
    let mut w = small(9471);
    let p = w.meta.founding_site;
    let ids = w.found_party(4);
    let (parent, child) = (ids[0], ids[1]);
    let ip = w.agents.iter().position(|x| x.id == parent).unwrap();
    w.agents[ip].children.push(child);
    let id = held_tool(&mut w, ip, ip, 0.6);
    for _ in 0..(hf_sim::legend::CHERISH_USES + 5) {
        hf_sim::legend::used(&mut w, ip, DesignationKind::Chop);
    }
    w.agents[ip].alive = false;
    let tick = w.meta.tick;
    w.events.push(
        tick,
        Some(p),
        hf_sim::events::Severity::Major,
        EventKind::Died {
            agent: parent,
            cause: "in a test".into(),
        },
    );
    w.step();
    w.step();
    let it = w.items.iter().find(|it| it.id == id).unwrap();
    assert!(
        it.story.as_ref().unwrap().heirloom,
        "the child inherited it"
    );
    // The heir sets it down rather than keep it in hand.
    let pos = w.agents.iter().find(|a| a.id == child).unwrap().pos;
    w.items.iter_mut().find(|it| it.id == id).unwrap().place = ItemPlace::Ground(pos);
    // Only this one storied item is at issue here; other starting gear's stories would
    // otherwise be swept into the same counts.
    w.items.retain(|it| it.id == id || it.story.is_none());

    w.meta.tick = 0;
    let mut c = possessions_census::Census::default();
    c.sample(&w);
    let s = *c.samples.last().expect("a sample at tick 0");
    assert_eq!(s.heirloom, 1, "{s:?}");
    assert_eq!(s.ground, 1, "{s:?}");
    assert_eq!(s.heirloom_with_heir, 0, "set down, not held: {s:?}");
}

#[test]
fn an_heirloom_from_a_partner_stays_cherished_and_names_them() {
    // #1121: a dead partner's cherished blade stays cherished in the heir's hand instead of
    // resetting as if a stranger had picked it up.
    let mut w = small(6);
    let p = w.meta.founding_site;
    let ids = w.found_party(4);
    let (dead, heir) = (ids[0], ids[1]);
    let idd = w.agents.iter().position(|x| x.id == dead).unwrap();
    w.agents[idd].partner = Some(heir);
    let ih = w.agents.iter().position(|x| x.id == heir).unwrap();
    w.agents[ih].partner = Some(dead);
    let id = held_tool(&mut w, idd, idd, 0.6);
    for _ in 0..(hf_sim::legend::CHERISH_USES + 5) {
        hf_sim::legend::used(&mut w, idd, DesignationKind::Chop);
    }
    let dead_name = w.agents[idd].name.clone();
    // They die.
    w.agents[idd].alive = false;
    let tick = w.meta.tick;
    w.events.push(
        tick,
        Some(p),
        hf_sim::events::Severity::Major,
        EventKind::Died {
            agent: dead,
            cause: "in a test".into(),
        },
    );
    w.step();
    w.step();
    let it = w.items.iter().find(|it| it.id == id).unwrap();
    assert_eq!(it.place, ItemPlace::Held(heir), "the partner took it up");
    let s = it.story.as_ref().unwrap();
    assert!(s.heirloom, "marked an heirloom");
    assert!(
        s.cherished,
        "stayed cherished: an heirloom is dear for whose it was, not how long the heir has \
         used it"
    );
    assert_eq!(
        s.keepsake_of.as_ref().map(|(_, n)| n.as_str()),
        Some(dead_name.as_str())
    );
    let d = hf_sim::legend::describe(&w, id).unwrap();
    assert!(d.contains(&dead_name), "{d}");
    assert!(d.contains("cherished"), "{d}");
}

#[test]
fn an_ordinary_run_gives_tools_lives() {
    let mut kept_longest = 0;
    let mut made_here = 0;
    let mut with_deeds = 0;
    for seed in [3u64, 7] {
        let mut w = World::opening_world(seed);
        w.meta.autoplan = true;
        for _ in 0..20 * TICKS_PER_DAY {
            w.step();
        }
        kept_longest = w
            .items
            .iter()
            .filter_map(|it| it.story.as_ref())
            .map(|s| s.keeper_uses)
            .chain([kept_longest])
            .max()
            .unwrap_or(0);
        made_here += w
            .items
            .iter()
            .filter_map(|it| it.story.as_ref())
            .filter(|s| s.maker.is_some())
            .count();
        with_deeds += w
            .items
            .iter()
            .filter_map(|it| it.story.as_ref())
            .filter(|s| !s.deeds.is_empty())
            .count();
    }
    // A hundred uses by one keeper is a lot for twenty days in a village building one house at a
    // time: whether one tool across two seeds gets there turns on who happens to keep which tool
    // (#392's hurry in the first days moved seed 7's from 108 uses to 57 and seed 3's from 51 to
    // 69). Cherishing itself is `long_use_makes_a_tool_cherished_then_named_then_spoken_of`;
    // here, one tool well on the way, and tools remembered for work, is what an ordinary run
    // shows.
    assert!(
        kept_longest >= hf_sim::legend::CHERISH_USES / 2,
        "{kept_longest}"
    );
    assert!(with_deeds >= 1, "{with_deeds}");
    assert!(
        made_here >= 4,
        "tools made in the valley carry their maker: {made_here}"
    );
}

/// A haul order at `pos` (staged, as work.rs tests post them).
fn post_haul(w: &mut World, pos: hf_sim::ids::TilePos) -> hf_sim::work::DesignationId {
    let id = w.alloc_designation_id();
    w.designations.push(hf_sim::work::Designation {
        id,
        kind: DesignationKind::Haul,
        pos,
        requirements: Vec::new(),
        work_left: 0,
        work_total: 0,
        max_workers: 1,
        priority: 9,
        created: w.tick(),
        failures: 0,
        done: false,
        community: None,
        owner: None,
    });
    id
}

fn stockpile(w: &mut World, pos: hf_sim::ids::TilePos) {
    let id = w.alloc_structure_id();
    w.structures.push(hf_sim::structures::Structure {
        id,
        kind: hf_sim::structures::StructureKind::Stockpile,
        pos,
        material: Material::AIR,
        progress: 1.0,
        active: false,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
}

fn ledger_drift(w: &World) -> Vec<String> {
    let live = hf_sim::economy::live_counts(&w.items);
    w.ledger
        .expected()
        .iter()
        .filter(|(k, v)| live.get(*k).copied().unwrap_or(0) != **v)
        .map(|(k, v)| format!("{k}: live {:?} expected {v}", live.get(k)))
        .collect()
}

#[test]
fn a_storied_thing_never_vanishes_into_a_stack() {
    use hf_sim::mind::Activity;
    use hf_sim::work::{Plan, Step};
    // Open level ground, so every tile here is walkable and in reach.
    let mut w = small(1);
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = hf_sim::ids::TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let p = w.meta.founding_site;
    let a = w.spawn_agent(Species::Person, p);
    let i = w.agents.iter().position(|x| x.id == a).unwrap();
    // A trophy, and a stack of ordinary hides on the stockpile tile beside it.
    let trophy = w.create_lone_item(ItemKind::Hide, 1, ItemPlace::Ground(p), 0.9, "trophy");
    w.items.iter_mut().find(|it| it.id == trophy).unwrap().story =
        Some(Box::new(hf_sim::legend::Story {
            maker_name: "Ada".into(),
            name: Some("Greyjaw's pelt".into()),
            wear: 1.0,
            ..Default::default()
        }));
    let t = p.offset(1, 0, 0);
    let u = p.offset(4, 0, 0);
    stockpile(&mut w, t);
    stockpile(&mut w, u);
    let hides = w.create_item(ItemKind::Hide, 3, ItemPlace::Ground(t), 0.5, "hunt");
    // A hide made where the trophy lies does not join it either.
    let more = w.create_item(ItemKind::Hide, 2, ItemPlace::Ground(p), 0.5, "hunt");
    assert_ne!(more, trophy);
    w.consume_item(more, 2, "test");
    // The planner sends it to the empty tile, not onto the stack.
    w.ensure_nav();
    let d = post_haul(&mut w, p);
    let order = w.designations.last().unwrap().clone();
    let planned = hf_sim::work::planner::plan_for(&w, a, &order).expect("a plan");
    assert!(
        planned
            .plan
            .steps
            .iter()
            .any(|s| matches!(s, Step::DropAt { to, .. } if *to == u)),
        "{:?}",
        planned.plan.steps
    );
    // And carried onto the stack anyway (the tile filled while they walked), it is put down
    // beside it, still itself.
    w.agents[i].activity = Activity::Working {
        plan: Plan {
            designation: d,
            steps: vec![
                Step::PickUp {
                    item: trophy,
                    count: 1,
                },
                Step::DropAt {
                    to: t,
                    kind: ItemKind::Hide,
                    for_site: None,
                },
            ],
            cursor: 0,
            label: "hauling".into(),
        },
    };
    hf_sim::systems::work::run(&mut w, i);
    hf_sim::systems::work::run(&mut w, i);
    let it = w
        .items
        .iter()
        .find(|it| it.id == trophy)
        .expect("still there");
    assert_eq!(it.place, ItemPlace::Ground(t));
    assert_eq!(it.count, 1);
    assert_eq!(
        it.story.as_ref().and_then(|s| s.name.clone()).as_deref(),
        Some("Greyjaw's pelt")
    );
    let h = w.items.iter().find(|it| it.id == hides).unwrap();
    assert_eq!((h.count, h.story.is_none()), (3, true));
    assert!(ledger_drift(&w).is_empty(), "{:?}", ledger_drift(&w));
}

#[test]
fn storied_clothes_are_worn_not_used_up_and_their_end_is_told() {
    let mut w = small(1);
    let p = w.meta.founding_site;
    let a = w.spawn_agent(Species::Person, p);
    let i = w.agents.iter().position(|x| x.id == a).unwrap();
    let cloak = w.create_lone_item(ItemKind::Clothes, 1, ItemPlace::Ground(p), 0.7, "test");
    w.items.iter_mut().find(|it| it.id == cloak).unwrap().story =
        Some(Box::new(hf_sim::legend::Story {
            maker_name: "Bera, their partner in Ashford".into(),
            wear: 0.6,
            origin: "from Ashford, come through the thin place".into(),
            ..Default::default()
        }));
    // Their clothes are rags: at five in the morning they put on the set in the store.
    w.agents[i].clothes_wear = 0.26;
    let morning = |w: &mut World, day: u64| {
        w.meta.tick = day * TICKS_PER_DAY + 5 * hf_sim::time::TICKS_PER_HOUR;
        hf_sim::systems::life::step(w);
    };
    morning(&mut w, 1);
    let it = w
        .items
        .iter()
        .find(|it| it.id == cloak)
        .expect("worn, not used up");
    assert_eq!(it.place, ItemPlace::Held(a));
    let s = it.story.as_ref().unwrap();
    assert_eq!(s.owners.last().map(|o| o.0), Some(a));
    assert!(
        (w.agents[i].clothes_wear - 0.6).abs() < 1e-6,
        "as worn as it was"
    );
    // It wears as they do, and is kept as rags while there is nothing to replace it.
    w.agents[i].clothes_wear = 0.1;
    morning(&mut w, 2);
    let it = w
        .items
        .iter()
        .find(|it| it.id == cloak)
        .expect("kept as rags");
    assert!(it.story.as_ref().unwrap().wear < 0.1);
    // A plain set comes to the store: it replaces the cloak, and the feed says what it was.
    w.create_item(ItemKind::Clothes, 1, ItemPlace::Ground(p), 0.5, "craft");
    w.events.pending.clear();
    morning(&mut w, 3);
    assert!(w.items.iter().all(|it| it.id != cloak), "worn through");
    let told: Vec<String> = events(&w)
        .into_iter()
        .filter(|l| l.contains("new clothes"))
        .collect();
    assert!(
        told.iter()
            .any(|l| l.contains("worn through") && l.contains("Bera") && l.contains("Ashford")),
        "{told:?}"
    );
    assert_eq!(
        w.agents[i].clothes_wear, 1.0,
        "the plain set is used up as before"
    );
    assert!(w.ledger.by_cause.get("-worn out").copied() == Some(1));
    assert!(ledger_drift(&w).is_empty(), "{:?}", ledger_drift(&w));
}

#[test]
fn clothing_replacement_refuses_zero_wear_storied_set() {
    let mut w = small(4);
    let p = w.meta.founding_site;
    let a = w.spawn_agent(Species::Person, p);
    let i = w.agents.iter().position(|x| x.id == a).unwrap();
    let cloak = w.create_lone_item(ItemKind::Clothes, 1, ItemPlace::Ground(p), 0.9, "test");
    w.items.iter_mut().find(|it| it.id == cloak).unwrap().story =
        Some(Box::new(hf_sim::legend::Story {
            wear: 0.0,
            origin: "from Ashford, come through the thin place".into(),
            ..Default::default()
        }));
    w.agents[i].clothes_wear = 0.2;
    w.meta.tick = TICKS_PER_DAY + 5 * hf_sim::time::TICKS_PER_HOUR;

    hf_sim::systems::life::step(&mut w);

    assert!(w.agents[i].clothes_wear <= 0.2);
    assert!(
        !w.events.pending.iter().any(|e| matches!(
            &e.kind,
            EventKind::NewClothes { agent, .. } if *agent == a
        )),
        "a worn-out set cannot be announced as new clothes"
    );
    let cloak = w.items.iter().find(|it| it.id == cloak).unwrap();
    assert_eq!(cloak.place, ItemPlace::Ground(p));
    assert_eq!(cloak.story.as_ref().unwrap().wear, 0.0);
    assert!(cloak.story.as_ref().unwrap().owners.is_empty());
}

#[test]
fn clothing_replacement_prefers_best_usable_set() {
    let mut w = small(5);
    let p = w.meta.founding_site;
    let a = w.spawn_agent(Species::Person, p);
    let i = w.agents.iter().position(|x| x.id == a).unwrap();
    let rags = w.create_lone_item(ItemKind::Clothes, 1, ItemPlace::Ground(p), 1.0, "test");
    w.items.iter_mut().find(|it| it.id == rags).unwrap().story =
        Some(Box::new(hf_sim::legend::Story {
            wear: 0.2,
            ..Default::default()
        }));
    let storied = w.create_lone_item(ItemKind::Clothes, 1, ItemPlace::Ground(p), 0.6, "test");
    w.items
        .iter_mut()
        .find(|it| it.id == storied)
        .unwrap()
        .story = Some(Box::new(hf_sim::legend::Story {
        wear: 0.6,
        ..Default::default()
    }));
    let plain = w.create_item(ItemKind::Clothes, 1, ItemPlace::Ground(p), 0.7, "test");
    w.agents[i].clothes_wear = 0.2;
    w.meta.tick = TICKS_PER_DAY + 5 * hf_sim::time::TICKS_PER_HOUR;

    hf_sim::systems::life::step(&mut w);

    assert_eq!(w.agents[i].clothes_wear, 1.0);
    assert!(w.items.iter().all(|it| it.id != plain));
    for (id, wear) in [(rags, 0.2), (storied, 0.6)] {
        let it = w.items.iter().find(|it| it.id == id).unwrap();
        assert_eq!(it.place, ItemPlace::Ground(p));
        assert_eq!(it.story.as_ref().unwrap().wear, wear);
    }
    assert!(w.events.pending.iter().any(|e| matches!(
        &e.kind,
        EventKind::NewClothes { agent, .. } if *agent == a
    )));
}
