//! #567: the Why? lens. Selecting a person traces their decisions: the conditions checked, the
//! options weighed with named score terms, what was chosen and why, and what was put off. The
//! trace is a read model only: the simulation goes the same way traced or not.

use hf_sim::ids::{AgentId, TilePos};
use hf_sim::items::ItemKind;
use hf_sim::mind::knowledge::Known;
use hf_sim::mind::Activity;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::why::Mode;
use hf_sim::work::{Designation, DesignationId, DesignationKind};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

/// A standalone haul, cook or other order, posted directly (tests/dispositions.rs's `post`).
fn post(w: &mut World, kind: DesignationKind, pos: TilePos, priority: u8) -> DesignationId {
    let id = w.alloc_designation_id();
    w.designations.push(Designation {
        id,
        kind,
        pos,
        requirements: Vec::new(),
        work_left: 100,
        work_total: 100,
        max_workers: 1,
        priority,
        created: w.tick(),
        failures: 0,
        done: false,
        community: None,
        owner: None,
    });
    id
}

/// A flat meadow with a party of `n` at (20, 20) at ten in the morning on day 30, nothing
/// growing and no food about (the #415 camp of tests/normal_person.rs).
fn camp(seed: u64, n: usize) -> (World, Vec<AgentId>) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(20, 20, 2);
    w.meta.tick = TICKS_PER_DAY * 30 + TICKS_PER_HOUR * 10;
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

/// The first person lonely, knowing where the others are and where a berry bush stands.
fn lonely_by_a_bush(w: &mut World, ids: &[AgentId]) {
    let bush = TilePos::new(26, 22, 2);
    w.plants
        .push(Plant::new(bush, PlantKind::BerryBush, 1.0, 1));
    let i = idx(w, ids[0]);
    let t = w.meta.tick;
    w.agents[i].needs.social = 0.1;
    for o in &ids[1..] {
        let p = w.agents[idx(w, *o)].pos;
        w.agents[i].knowledge.note(Known::Agent(*o), p, t, 0);
    }
    w.agents[i]
        .knowledge
        .note(Known::FoodPlant(bush), bush, t, PlantKind::BerryBush as u16);
}

#[test]
fn the_lens_shows_the_options_their_terms_and_what_the_short_store_put_off() {
    let (mut w, ids) = camp(5671, 3);
    lonely_by_a_bush(&mut w, &ids);
    w.trace_why(Some(ids[0]));
    hf_sim::systems::decide::step(&mut w);
    let c = w
        .why
        .as_ref()
        .and_then(|t| t.latest())
        .expect("the traced person's cycle is written down")
        .clone();
    assert_eq!(c.mode, Mode::Deliberate);
    let chosen = c
        .options
        .iter()
        .find(|o| o.chosen)
        .expect("a chosen option");
    assert_eq!(chosen.cycle, "stock_the_store", "{c:#?}");
    assert_eq!(c.chosen, chosen.what);
    // The talk the person wanted is there with its terms: it scored well on its own, and the
    // secured gate held it to a twentieth.
    let talk = c
        .options
        .iter()
        .find(|o| o.cycle == "talk")
        .expect("talk was weighed");
    assert!(talk.rejected.is_none());
    assert!((talk.gate - hf_sim::systems::decide::LEISURE_UNSECURED).abs() < 1e-6);
    assert!(
        talk.base * talk.emotion > chosen.score,
        "{talk:?} vs {chosen:?}"
    );
    // #1760: talk's score now also carries `learned` and `temperament` (cheerfulness).
    assert!(
        (talk.score - talk.base * talk.emotion * talk.gate * talk.learned * talk.temperament).abs()
            < 1e-6
    );
    // The conditions carry their values; the store's shortness is the reason given.
    let store = c
        .conditions
        .iter()
        .find(|k| k.name == "days of food in store")
        .unwrap();
    assert!(!store.ok && store.value < 2.0, "{store:?}");
    assert!(
        c.because.iter().any(|b| b.contains("the store holds")),
        "{:?}",
        c.because
    );
    assert!(
        c.put_off
            .iter()
            .any(|p| p.starts_with(&talk.what) && p.contains("until food and shelter")),
        "{:?}",
        c.put_off
    );
}

#[test]
fn with_the_store_full_the_same_person_talks_and_nothing_is_put_off() {
    let (mut w, ids) = camp(5671, 3);
    let at = w.grid.surface(22, 18);
    w.drop_item(ItemKind::Meal, 90, at);
    lonely_by_a_bush(&mut w, &ids);
    w.trace_why(Some(ids[0]));
    hf_sim::systems::decide::step(&mut w);
    let c = w.why.as_ref().unwrap().latest().unwrap().clone();
    let chosen = c.options.iter().find(|o| o.chosen).unwrap();
    assert_eq!(chosen.cycle, "talk", "{c:#?}");
    assert_eq!(chosen.gate, 1.0);
    assert!(c.put_off.is_empty(), "{:?}", c.put_off);
}

#[test]
fn a_starving_talker_is_shown_as_interrupted() {
    let (mut w, ids) = camp(5672, 2);
    let i = idx(&w, ids[0]);
    w.agents[i].activity = Activity::Loitering { ticks_left: 400 };
    w.agents[i].activity_ticks = 400;
    w.agents[i].needs.hunger = 0.05;
    w.trace_why(Some(ids[0]));
    hf_sim::systems::decide::step(&mut w);
    let c = w.why.as_ref().unwrap().latest().unwrap();
    match &c.mode {
        Mode::Interrupted { by, .. } => assert_eq!(by, "starving"),
        m => panic!("{m:?}"),
    }
    assert!(c.because[0].starts_with("starving"), "{:?}", c.because);
}

#[test]
fn only_the_traced_person_is_written_down_and_choosing_another_starts_afresh() {
    let (mut w, ids) = camp(5673, 3);
    w.trace_why(Some(ids[1]));
    hf_sim::systems::decide::step(&mut w);
    assert!(w.why.as_ref().unwrap().cycles.len() == 1);
    w.trace_why(Some(ids[1]));
    assert_eq!(w.why.as_ref().unwrap().cycles.len(), 1, "same person kept");
    w.trace_why(Some(ids[2]));
    assert!(w.why.as_ref().unwrap().cycles.is_empty(), "a new person");
    w.trace_why(None);
    assert!(w.why.is_none());
}

#[test]
fn tracing_does_not_change_the_simulation() {
    let run = |traced: bool| {
        let (mut w, ids) = camp(5674, 4);
        let at = w.grid.surface(22, 18);
        w.drop_item(ItemKind::Meal, 40, at);
        lonely_by_a_bush(&mut w, &ids);
        if traced {
            w.trace_why(Some(ids[0]));
        }
        for _ in 0..(TICKS_PER_DAY * 2) {
            w.step();
        }
        let cycles = w.why.as_ref().map(|t| t.cycles.len()).unwrap_or(0);
        (hf_sim::digest::digest(&w), cycles)
    };
    let (plain, none) = run(false);
    let (traced, cycles) = run(true);
    assert_eq!(none, 0);
    assert!(cycles > 0, "the traced person decided in two days");
    assert_eq!(plain, traced, "tracing changed the simulation");
}

/// #885: the Why? lens has a slot for what mood did to a choice. Until mood steers (#886 on) the
/// factor is 1.0 on every option, so it changes no score and `explain` prints nothing for it.
#[test]
fn why_lists_a_spirits_factor_of_one() {
    let (mut w, ids) = camp(8851, 3);
    lonely_by_a_bush(&mut w, &ids);
    // #1758 gives the Stock option its own temperament term; keep this person legacy so every
    // option here still reads 1.0 for it.
    let i0 = idx(&w, ids[0]);
    w.agents[i0].dispositions = None;
    w.agents[i0].traits.list.clear();
    w.trace_why(Some(ids[0]));
    hf_sim::systems::decide::step(&mut w);
    let c = w
        .why
        .as_ref()
        .and_then(|t| t.latest())
        .expect("the traced person's cycle is written down")
        .clone();
    assert!(c.options.len() > 1, "{c:#?}");
    for o in &c.options {
        assert_eq!(o.spirits, 1.0, "{o:?}");
    }
    // The factor sits in the score: the product of the named terms is the score.
    let chosen = c
        .options
        .iter()
        .find(|o| o.chosen)
        .expect("a chosen option");
    assert!(
        (chosen.score
            - chosen.base * chosen.emotion * chosen.gate * chosen.learned * chosen.spirits)
            .abs()
            < 1e-5,
        "{chosen:?}"
    );
    // At 1.0 the lens says nothing of it.
    assert!(
        !c.because.iter().any(|b| b.contains("spirits")),
        "{:?}",
        c.because
    );
}

/// #906: the Why? lens has a slot for what a disposition did to a choice. For a legacy person
/// (no core profile, #1759's `independence`, #1760's `cheer_weight` and the rest all fall back to
/// neutral), the factor is 1.0 on every option, so it changes no score and `explain` prints
/// nothing for it.
#[test]
fn why_lists_a_temperament_factor_of_one() {
    let (mut w, ids) = camp(9061, 3);
    lonely_by_a_bush(&mut w, &ids);
    // No core profile and no relevant legacy trait keeps Loiter's own temperament term at
    // 1.0 too (#907), so this stays a test of the plumbing, not of that goal's particular term.
    // #1758 gives the Stock option its own temperament term; keep this person legacy so every
    // option here still reads 1.0 for it.
    let i0 = idx(&w, ids[0]);
    w.agents[i0].dispositions = None;
    w.agents[i0].traits.list.clear();
    w.trace_why(Some(ids[0]));
    hf_sim::systems::decide::step(&mut w);
    let c = w
        .why
        .as_ref()
        .and_then(|t| t.latest())
        .expect("the traced person's cycle is written down")
        .clone();
    assert!(c.options.len() > 1, "{c:#?}");
    for o in &c.options {
        assert_eq!(o.temperament, 1.0, "{o:?}");
    }
    // The factor sits in the score: the product of the named terms is the score.
    let chosen = c
        .options
        .iter()
        .find(|o| o.chosen)
        .expect("a chosen option");
    assert!(
        (chosen.score
            - chosen.base
                * chosen.emotion
                * chosen.gate
                * chosen.learned
                * chosen.spirits
                * chosen.temperament)
            .abs()
            < 1e-5,
        "{chosen:?}"
    );
    // At 1.0 the lens says nothing of it.
    assert!(
        !c.because.iter().any(|b| b.contains("temperament")),
        "{:?}",
        c.because
    );
}

/// A world with one agent facing a haul order, a cook order and a chop order (standing in for
/// "a build"), traced.
fn orderly_world(seed: u64, order: f32) -> hf_sim::why::Cycle {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(20, 20, 2);
    w.meta.tick = TICKS_PER_DAY * 30 + TICKS_PER_HOUR * 10;
    w.rebuild_caches();
    let ids = w.found_party(1);
    w.items.retain(|it| !it.kind.is_food());
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    w.agents[i].needs.hunger = 0.5;
    w.agents[i].needs.rest = 0.5;
    w.agents[i].needs.social = 0.5;
    w.agents[i].needs.warmth = 1.0;
    w.agents[i].activity = Activity::Idle;
    w.agents[i].idle_ticks = 0;
    let haul_at = TilePos::new(22, 20, 2);
    let cook_at = TilePos::new(22, 22, 2);
    let chop_at = TilePos::new(18, 20, 2);
    w.drop_item(ItemKind::Log, 1, haul_at);
    w.place_structure(
        hf_sim::structures::StructureKind::Stockpile,
        TilePos::new(23, 20, 2),
        hf_sim::world::Material::AIR,
        false,
    );
    w.plants.push(Plant::new(chop_at, PlantKind::Tree, 1.0, 1));
    post(&mut w, DesignationKind::Haul, haul_at, 5);
    post(&mut w, DesignationKind::Cook, cook_at, 5);
    post(&mut w, DesignationKind::Chop, chop_at, 5);
    let d = w.agents[i].dispositions.as_mut().unwrap();
    d.order = order;
    w.ensure_nav();
    w.trace_why(Some(ids[0]));
    hf_sim::systems::decide::step(&mut w);
    w.why
        .as_ref()
        .and_then(|t| t.latest())
        .expect("the traced person's cycle is written down")
        .clone()
}

/// A world with two adults, one lonely and knowing where a friend is, so `Talk` is weighed;
/// mood held at 0 so `current_dispositions`'s mood shift does not move cheerfulness off what it
/// is set to.
fn cheerful_world(seed: u64, cheerfulness: f32) -> hf_sim::why::Cycle {
    let (mut w, ids) = camp(seed, 2);
    let i0 = idx(&w, ids[0]);
    let friend = ids[1];
    let at = w.agents[idx(&w, friend)].pos;
    let t = w.meta.tick;
    w.agents[i0].needs.social = 0.1;
    w.agents[i0].knowledge.note(Known::Agent(friend), at, t, 0);
    w.agents[i0].mood = 0.0;
    let d = w.agents[i0].dispositions.as_mut().unwrap();
    d.cheerfulness = cheerfulness;
    w.trace_why(Some(ids[0]));
    hf_sim::systems::decide::step(&mut w);
    w.why
        .as_ref()
        .and_then(|t| t.latest())
        .expect("the traced person's cycle is written down")
        .clone()
}

/// #1761: the orderly weigh a haul or a cook order more; a build order (here, felling, which
/// is neither) is unaffected by the order weight. Since #1947, every work option also carries
/// Work's own weight, so the expectation is relative to the felling option (Work's weight alone).
#[test]
fn orderly_weigh_haul_and_cook_more() {
    let tidy = orderly_world(9171, 0.9);
    let messy = orderly_world(9171, 0.1);
    for (c, want) in [(&tidy, 1.16_f32), (&messy, 0.84_f32)] {
        let haul = c
            .options
            .iter()
            .find(|o| o.what == "work: hauling")
            .expect("a haul option");
        let cook = c
            .options
            .iter()
            .find(|o| o.what == "work: cooking")
            .expect("a cook option");
        let chop = c
            .options
            .iter()
            .find(|o| o.what == "work: felling")
            .expect("a chop option");
        let work_only = chop.temperament;
        assert!(
            (haul.temperament - work_only * want).abs() < 0.01,
            "{haul:?} against work {work_only}"
        );
        assert!(
            (cook.temperament - work_only * want).abs() < 0.01,
            "{cook:?} against work {work_only}"
        );
    }
}

/// #1760: a cheerful person is the one who stops for a word.
#[test]
fn cheerful_weigh_talk_more() {
    let cheerful = cheerful_world(9271, 0.9);
    let glum = cheerful_world(9271, 0.1);
    let talk_of = |c: &hf_sim::why::Cycle| {
        c.options
            .iter()
            .find(|o| o.cycle == "talk")
            .expect("a talk option")
            .temperament
    };
    let (hi, lo) = (talk_of(&cheerful), talk_of(&glum));
    assert!((0.85..=1.15).contains(&hi), "{hi}");
    assert!((0.85..=1.15).contains(&lo), "{lo}");
    assert!(hi - lo >= 0.2, "cheerful {hi} vs glum {lo}");
}
