//! #488: dimensional core dispositions and their temporary surface expression.

use hf_sim::body::{Part, Wound, WoundKind};
use hf_sim::genetics::Genome;
use hf_sim::ids::TilePos;
use hf_sim::items::ItemKind;
use hf_sim::mind::knowledge::Known;
use hf_sim::mind::{Activity, Dispositions, Trait};
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::save::{self, snapshot::Snapshot};
use hf_sim::structures::StructureKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::{Designation, DesignationId, DesignationKind};
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

fn person() -> (World, usize) {
    let mut w = World::generate(488, GenParams::small());
    let ids = w.found_party(1);
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let a = &mut w.agents[i];
    a.traits.list.clear();
    a.dispositions = Some(Dispositions {
        sociability: 0.8,
        industry: 0.8,
        curiosity: 0.75,
        caution: 0.2,
        warmth: 0.9,
        cheerfulness: 0.9,
        order: 0.7,
        independence: 0.6,
    });
    a.needs.rest = 1.0;
    a.mood = 0.2;
    a.stress = 0.0;
    (w, i)
}

fn labels(w: &World, i: usize) -> Vec<&'static str> {
    w.agents[i]
        .surface_traits()
        .into_iter()
        .map(|t| t.label)
        .collect()
}

#[test]
fn temporary_strain_changes_surface_and_recovers_without_rewriting_core() {
    let (mut w, i) = person();
    let core = w.agents[i].dispositions;
    let baseline = labels(&w, i);
    let baseline_talk = w.agents[i].talk_weight();
    assert!(baseline.contains(&"cheerful"));

    let a = &mut w.agents[i];
    a.needs.rest = 0.05;
    a.mood = -0.8;
    a.stress = 0.9;
    a.body.wounds.push(Wound {
        part: Part::TORSO,
        kind: WoundKind::Cut,
        severity: 0.8,
        bleeding: 0.0,
        infected: false,
        since: 0,
        by: None,
        tissue: Default::default(),
        treated: None,
        in_anger: false,
    });
    let strained = labels(&w, i);
    assert_ne!(strained, baseline);
    assert!(strained.contains(&"irritable"));
    assert!(w.agents[i].talk_weight() < baseline_talk);
    assert_eq!(w.agents[i].dispositions, core, "the core is not mood");

    let a = &mut w.agents[i];
    a.body.wounds.clear();
    a.needs.rest = 1.0;
    a.mood = 0.2;
    a.stress = 0.0;
    assert_eq!(labels(&w, i), baseline);
    assert_eq!(w.agents[i].talk_weight().to_bits(), baseline_talk.to_bits());
}

#[test]
fn relevant_axes_change_choice_weights_and_unrelated_axes_do_not() {
    let (mut w, i) = person();
    let base_talk = w.agents[i].talk_weight();
    let base_order = w.agents[i].dispositions.unwrap().order;
    w.agents[i].dispositions.as_mut().unwrap().order = 1.0 - base_order;
    assert_eq!(w.agents[i].talk_weight().to_bits(), base_talk.to_bits());

    w.agents[i].dispositions.as_mut().unwrap().sociability = 0.1;
    assert!(w.agents[i].talk_weight() < base_talk);
    w.agents[i].dispositions.as_mut().unwrap().sociability = 0.8;
    w.agents[i].dispositions.as_mut().unwrap().warmth = 0.1;
    assert!(w.agents[i].talk_weight() < base_talk);

    let base_work = w.agents[i].work_weight();
    w.agents[i].dispositions.as_mut().unwrap().cheerfulness = 0.1;
    assert_eq!(w.agents[i].work_weight().to_bits(), base_work.to_bits());
    w.agents[i].dispositions.as_mut().unwrap().industry = 0.1;
    assert!(w.agents[i].work_weight() < base_work);

    let low_order = w.agents[i].haul_fit();
    w.agents[i].dispositions.as_mut().unwrap().order = 1.0;
    assert!(w.agents[i].haul_fit() > low_order);
}

#[test]
fn circumstances_change_the_same_persons_work_and_wander_weights() {
    let (mut w, i) = person();
    let rested_work = w.agents[i].work_weight();
    let calm_wander = w.agents[i].wander_weight();
    let core = w.agents[i].dispositions;

    w.agents[i].needs.rest = 0.0;
    w.agents[i].stress = 1.0;
    assert!(w.agents[i].work_weight() < rested_work);
    assert!(w.agents[i].wander_weight() < calm_wander);
    assert_eq!(w.agents[i].dispositions, core);
}

#[test]
fn surface_is_salience_ordered_capped_and_legacy_saves_keep_their_traits() {
    let (mut w, i) = person();
    let a = &mut w.agents[i];
    a.dispositions = Some(Dispositions {
        sociability: 1.0,
        industry: 1.0,
        curiosity: 1.0,
        caution: 0.0,
        warmth: 1.0,
        cheerfulness: 1.0,
        order: 1.0,
        independence: 1.0,
    });
    a.stress = 1.0;
    a.needs.rest = 0.0;
    let surface = a.surface_traits();
    assert!(surface.len() <= 5);
    assert!(surface.windows(2).all(|p| p[0].salience >= p[1].salience));

    a.dispositions = None;
    a.traits.list = vec![
        Trait::Industrious,
        Trait::Sociable,
        Trait::Curious,
        Trait::Cautious,
        Trait::Hearty,
        Trait::Gourmet,
    ];
    assert_eq!(a.surface_traits().len(), 5);
    assert_eq!(a.talk_weight(), a.traits.talk_weight());
    assert_eq!(a.work_weight(), a.traits.work_weight());
}

#[test]
fn genome_profiles_are_deterministic_dimensional_and_bounded() {
    let mut genome = Genome::default();
    for (i, locus) in genome.loci.iter_mut().enumerate() {
        *locus = (i as u8).wrapping_mul(19).wrapping_add(7);
    }
    let first = genome.dispositions();
    let second = genome.dispositions();
    assert_eq!(first, second);
    for value in [
        first.sociability,
        first.industry,
        first.curiosity,
        first.caution,
        first.warmth,
        first.cheerfulness,
        first.order,
        first.independence,
    ] {
        assert!((0.0..=1.0).contains(&value));
    }
    assert_ne!(first.sociability, first.industry);
    assert_ne!(first.warmth, first.cheerfulness);
}

#[test]
fn named_save_roundtrip_preserves_core_not_its_temporary_expression() {
    let (mut w, i) = person();
    let core = w.agents[i].dispositions;
    w.agents[i].stress = 0.9;
    w.agents[i].needs.rest = 0.1;
    let encoded = rmp_serde::to_vec_named(&w.agents[i]).unwrap();
    let restored: hf_sim::agents::Agent = rmp_serde::from_slice(&encoded).unwrap();
    assert_eq!(restored.dispositions, core);
    assert_eq!(restored.stress, 0.9);
    assert_eq!(restored.needs.rest, 0.1);
}

fn choice_world(seed: u64, people: usize) -> (World, usize) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.items.clear();
    w.meta.founding_site = TilePos::new(20, 20, 2);
    w.meta.tick = TICKS_PER_DAY * 30 + TICKS_PER_HOUR * 10;
    w.rebuild_caches();
    let ids = w.found_party(people);
    w.meta.autoplan = false;
    for (k, id) in ids.iter().enumerate() {
        let i = w.agents.iter().position(|a| a.id == *id).unwrap();
        let a = &mut w.agents[i];
        a.pos = TilePos::new(20 + k as i32, 20, 2);
        a.from = a.pos;
        a.activity = Activity::Idle;
        a.idle_ticks = if k == 0 { 0 } else { 1 };
        a.needs.hunger = 0.95;
        a.needs.rest = 0.9;
        a.needs.social = 1.0;
        a.needs.warmth = 1.0;
        a.needs.thirst = 1.0;
        a.mood = 0.0;
        a.stress = 0.0;
        a.thoughts = Default::default();
        a.feelings = Default::default();
        a.knowledge.map.clear();
        a.traits.list.clear();
        a.dispositions = Some(Dispositions::default());
        for skill in &mut a.skills.states {
            skill.level = 0.0;
            skill.xp = 0.0;
        }
    }
    w.drop_item(
        ItemKind::Meal,
        (people * 30) as u32,
        TilePos::new(22, 18, 2),
    );
    w.ensure_nav();
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    (w, i)
}

fn choice_scenario<R>(
    seed: u64,
    people: usize,
    configure: impl FnOnce(&mut World, usize) -> R,
) -> (Activity, R) {
    let (mut w, i) = choice_world(seed, people);
    let extra = configure(&mut w, i);
    hf_sim::systems::decide::step(&mut w);
    (w.agents[i].activity.clone(), extra)
}

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

fn flip_unrelated(d: &mut Dispositions, axes: &[&str]) {
    fn flip(value: f32) -> f32 {
        if value == 0.5 {
            0.0
        } else {
            1.0 - value
        }
    }
    for axis in axes {
        match *axis {
            "sociability" => d.sociability = flip(d.sociability),
            "industry" => d.industry = flip(d.industry),
            "curiosity" => d.curiosity = flip(d.curiosity),
            "caution" => d.caution = flip(d.caution),
            "warmth" => d.warmth = flip(d.warmth),
            "cheerfulness" => d.cheerfulness = flip(d.cheerfulness),
            "order" => d.order = flip(d.order),
            "independence" => d.independence = flip(d.independence),
            _ => unreachable!(),
        }
    }
}

fn talk_choice(unrelated: &[&str], low: bool) -> Activity {
    choice_scenario(4881, 2, |w, i| {
        let other = w.agents[1 - i].id;
        let at = w.agents[1 - i].pos;
        let now = w.tick();
        w.agents[i].needs.social = 0.65;
        w.agents[i].knowledge.note(Known::Agent(other), at, now, 0);
        let d = w.agents[i].dispositions.as_mut().unwrap();
        d.sociability = if low { 0.0 } else { 1.0 };
        d.warmth = if low { 0.0 } else { 1.0 };
        d.curiosity = 0.8;
        flip_unrelated(d, unrelated);
    })
    .0
}

#[test]
fn social_axes_change_the_real_talk_choice_and_unrelated_axes_do_not() {
    let social = talk_choice(&[], false);
    assert!(
        matches!(&social, Activity::Going { then, .. } if matches!(**then, Activity::Talking { .. })),
        "{social:?}"
    );
    assert!(matches!(talk_choice(&[], true), Activity::Wandering { .. }));
    let unrelated = talk_choice(
        &[
            "industry",
            "curiosity",
            "caution",
            "cheerfulness",
            "order",
            "independence",
        ],
        false,
    );
    assert_eq!(unrelated, social);
}

fn wander_choice(unrelated: &[&str], low: bool) -> Activity {
    choice_scenario(4882, 1, |w, i| {
        let d = w.agents[i].dispositions.as_mut().unwrap();
        d.curiosity = if low { 0.0 } else { 1.0 };
        d.caution = if low { 1.0 } else { 0.0 };
        flip_unrelated(d, unrelated);
    })
    .0
}

#[test]
fn exploration_axes_change_the_real_wander_choice_and_unrelated_axes_do_not() {
    let curious = wander_choice(&[], false);
    assert!(matches!(curious, Activity::Wandering { .. }), "{curious:?}");
    assert!(matches!(
        wander_choice(&[], true),
        Activity::Loitering { .. }
    ));
    let unrelated = wander_choice(
        &[
            "sociability",
            "industry",
            "warmth",
            "cheerfulness",
            "order",
            "independence",
        ],
        false,
    );
    assert_eq!(unrelated, curious);
}

fn work_choice(unrelated: &[&str], low: bool) -> Activity {
    choice_scenario(4883, 1, |w, i| {
        let at = TilePos::new(40, 20, 2);
        w.plants.push(Plant::new(at, PlantKind::Tree, 1.0, 1));
        post(w, DesignationKind::Chop, at, 0);
        let d = w.agents[i].dispositions.as_mut().unwrap();
        d.industry = if low { 0.0 } else { 1.0 };
        d.curiosity = 0.2;
        flip_unrelated(d, unrelated);
    })
    .0
}

#[test]
fn industry_changes_the_real_work_choice_and_unrelated_axes_do_not() {
    let industrious = work_choice(&[], false);
    assert!(
        matches!(industrious, Activity::Working { .. }),
        "{industrious:?}"
    );
    // Fix-site (FIXLOG 2026-11-16 #907): low industry now also lifts Loiter's own weight
    // (1.5x at zero industry), so the laziest easily outweighs a low-curiosity Wander here.
    assert!(matches!(work_choice(&[], true), Activity::Loitering { .. }));
    let unrelated = work_choice(
        &[
            "sociability",
            "warmth",
            "cheerfulness",
            "order",
            "independence",
        ],
        false,
    );
    assert_eq!(unrelated, industrious);
}

fn haul_choice(unrelated: &[&str], orderly: bool) -> (Activity, DesignationId, DesignationId) {
    let (activity, (haul, chop)) = choice_scenario(4884, 1, |w, i| {
        let haul_at = TilePos::new(36, 20, 2);
        let chop_at = TilePos::new(4, 20, 2);
        w.drop_item(ItemKind::Log, 1, haul_at);
        w.place_structure(
            StructureKind::Stockpile,
            TilePos::new(37, 20, 2),
            Material::AIR,
            false,
        );
        w.plants.push(Plant::new(chop_at, PlantKind::Tree, 1.0, 2));
        let haul = post(w, DesignationKind::Haul, haul_at, 9);
        let chop = post(w, DesignationKind::Chop, chop_at, 9);
        let d = w.agents[i].dispositions.as_mut().unwrap();
        d.order = if orderly { 1.0 } else { 0.0 };
        flip_unrelated(d, unrelated);
        (haul, chop)
    });
    (activity, haul, chop)
}

#[test]
fn order_changes_the_real_haul_choice_and_unrelated_axes_do_not() {
    let (orderly, haul, chop) = haul_choice(&[], true);
    assert!(
        matches!(&orderly, Activity::Working { plan } if plan.designation == haul),
        "{orderly:?}"
    );
    let (unordered, _, _) = haul_choice(&[], false);
    assert!(
        matches!(&unordered, Activity::Working { plan } if plan.designation == chop),
        "{unordered:?}"
    );
    let (unrelated, _, _) = haul_choice(
        &[
            "sociability",
            "curiosity",
            "caution",
            "warmth",
            "cheerfulness",
            "independence",
        ],
        true,
    );
    assert_eq!(unrelated, orderly);
}

fn open_orders_completed(seed: u64, industry: f32) -> u32 {
    let (mut w, i) = choice_world(seed, 1);
    w.agents[i].dispositions.as_mut().unwrap().industry = industry;
    for k in 0..10u32 {
        let at = TilePos::new(21 + k as i32, 20, 2);
        w.plants.push(Plant::new(at, PlantKind::Tree, 1.0, 2));
        post(&mut w, DesignationKind::Chop, at, 0);
    }
    for _ in 0..(TICKS_PER_DAY * 2) {
        w.step();
    }
    w.designations.iter().filter(|d| d.done).count() as u32
}

#[test]
fn industrious_take_more_work() {
    let lazy = open_orders_completed(9081, 0.1);
    let industrious = open_orders_completed(9081, 0.9);
    assert!(
        industrious as f32 >= 2.0 * (lazy.max(1) as f32),
        "industrious {industrious}, lazy {lazy}"
    );
}

#[test]
fn industrious_stand_about_less() {
    let (mut w, i0) = choice_world(9071, 2);
    let i1 = w
        .agents
        .iter()
        .position(|a| a.id != w.agents[i0].id)
        .unwrap();
    w.agents[i0].dispositions.as_mut().unwrap().industry = 0.9;
    w.agents[i1].dispositions.as_mut().unwrap().industry = 0.1;
    // Open hauls: there is always something to work at, as there would be in a real camp.
    let haul_at = TilePos::new(21, 25, 2);
    w.drop_item(ItemKind::Log, 1, haul_at);
    w.place_structure(
        StructureKind::Stockpile,
        TilePos::new(22, 25, 2),
        Material::AIR,
        false,
    );
    post(&mut w, DesignationKind::Haul, haul_at, 5);
    w.drop_item(ItemKind::Log, 1, TilePos::new(23, 25, 2));
    post(&mut w, DesignationKind::Haul, TilePos::new(23, 25, 2), 5);
    let id0 = w.agents[i0].id;
    let id1 = w.agents[i1].id;
    let mut loiter = [0u32; 2];
    for _ in 0..TICKS_PER_DAY {
        w.step();
        let a0 = w.agents.iter().find(|a| a.id == id0).unwrap();
        let a1 = w.agents.iter().find(|a| a.id == id1).unwrap();
        if matches!(a0.activity, Activity::Loitering { .. }) {
            loiter[0] += 1;
        }
        if matches!(a1.activity, Activity::Loitering { .. }) {
            loiter[1] += 1;
        }
    }
    assert!(
        loiter[1] > 0,
        "the easygoing one loiters at all: {loiter:?}"
    );
    assert!(
        (loiter[0] as f32) <= 0.7 * (loiter[1] as f32),
        "the industrious one should stand about at most 0.7x as much: {loiter:?}"
    );
}

#[test]
fn real_v2_fixture_without_dispositions_decodes_with_legacy_behavior() {
    let fixture =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/saves/v2.hfsave");
    let (old, _chronicle) = save::read(&fixture).unwrap();
    assert!(!old.agents.is_empty());
    for a in &old.agents {
        assert_eq!(a.dispositions, None, "{} has no post-fixture field", a.name);
        assert_eq!(a.talk_weight(), a.traits.talk_weight());
        assert_eq!(a.work_weight(), a.traits.work_weight());
        assert_eq!(a.wander_weight(), a.traits.wander_weight());
        assert_eq!(a.social_rate(), a.traits.social_rate());
    }
}

#[test]
fn disposition_save_restore_matches_continuous_simulation() {
    let mut continuous = World::opening_world(488);
    continuous.run(600);
    assert!(continuous.agents.iter().any(|a| a.dispositions.is_some()));
    let bytes = save::encode(&Snapshot::capture(&continuous)).unwrap();
    let mut restored = save::decode(&bytes).unwrap().restore();
    assert_eq!(
        hf_sim::digest::digest(&restored),
        hf_sim::digest::digest(&continuous)
    );
    continuous.run(1_200);
    restored.run(1_200);
    assert_eq!(
        hf_sim::digest::digest(&restored),
        hf_sim::digest::digest(&continuous)
    );
}

/// A single-agent world at a hearth evening hour, needs secure, independence set on the
/// traced agent; `tests/communities.rs`'s hearth-placement pattern.
fn independence_world(seed: u64, independence: f32) -> hf_sim::why::Cycle {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(20, 20, 2);
    // Any hour in culture::HEARTH_HOURS (18..21).
    w.meta.tick = TICKS_PER_DAY * 30 + TICKS_PER_HOUR * 19;
    w.rebuild_caches();
    let ids = w.found_party(1);
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let cid = w.agents[i].community.unwrap();
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    let at = home.offset(-3, -3, 0);
    let id = w.alloc_structure_id();
    w.structures.push(hf_sim::structures::Structure {
        id,
        kind: StructureKind::Hearth,
        pos: at,
        material: Material::STONE_BLOCK,
        progress: 1.0,
        active: true,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
    w.agents[i].needs.hunger = 0.7;
    w.agents[i].needs.rest = 0.7;
    w.agents[i].needs.social = 0.7;
    w.agents[i].activity = Activity::Idle;
    w.agents[i].idle_ticks = 0;
    w.agents[i].dispositions.as_mut().unwrap().independence = independence;
    w.ensure_nav();
    w.trace_why(Some(ids[0]));
    hf_sim::systems::decide::step(&mut w);
    w.why
        .as_ref()
        .and_then(|t| t.latest())
        .expect("the traced person's cycle is written down")
        .clone()
}

/// A camp with the store short (under two days), one known fruiting bush roughly 8 tiles off,
/// traced; industry set on the traced agent.
fn stocking_world(seed: u64, industry: f32) -> hf_sim::why::Cycle {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(20, 20, 2);
    w.meta.tick = TICKS_PER_DAY * 30 + TICKS_PER_HOUR * 10;
    w.rebuild_caches();
    let ids = w.found_party(1);
    w.items.retain(|it| !it.kind.is_food());
    w.ensure_nav();
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    w.agents[i].needs.hunger = 0.95;
    w.agents[i].needs.rest = 1.0;
    w.agents[i].needs.social = 0.9;
    w.agents[i].needs.warmth = 1.0;
    w.agents[i].activity = Activity::Idle;
    w.agents[i].idle_ticks = 0;
    let bush = TilePos::new(26, 22, 2);
    w.plants
        .push(Plant::new(bush, PlantKind::BerryBush, 1.0, 1));
    let t = w.meta.tick;
    w.agents[i]
        .knowledge
        .note(Known::FoodPlant(bush), bush, t, PlantKind::BerryBush as u16);
    w.agents[i].dispositions.as_mut().unwrap().industry = industry;
    w.trace_why(Some(ids[0]));
    hf_sim::systems::decide::step(&mut w);
    w.why
        .as_ref()
        .and_then(|t| t.latest())
        .expect("the traced person's cycle is written down")
        .clone()
}

/// #1759: the self-reliant sit at the hearth less and wander more.
#[test]
fn independent_gather_less() {
    let independent = independence_world(9371, 0.9);
    let communal = independence_world(9371, 0.1);
    let gather_of = |c: &hf_sim::why::Cycle| {
        c.options
            .iter()
            .find(|o| o.cycle == "hearth")
            .expect("a hearth option")
            .temperament
    };
    let wander_of = |c: &hf_sim::why::Cycle| {
        c.options
            .iter()
            .find(|o| o.cycle == "wander")
            .expect("a wander option")
            .temperament
    };
    let (hi, lo) = (gather_of(&independent), gather_of(&communal));
    assert!(hi < lo, "independent {hi} vs communal {lo}");
    let (hi, lo) = (wander_of(&independent), wander_of(&communal));
    assert!(hi > lo, "independent {hi} vs communal {lo}");
}

/// #1758: when the stores run short, the hard workers are the first out with a basket.
#[test]
fn industrious_weigh_stocking_more() {
    let industrious = stocking_world(9471, 0.9);
    let idle = stocking_world(9471, 0.1);
    let stock_of = |c: &hf_sim::why::Cycle| {
        c.options
            .iter()
            .find(|o| o.cycle == "stock_the_store")
            .expect("a stock option")
            .temperament
    };
    let (hi, lo) = (stock_of(&industrious), stock_of(&idle));
    assert!((hi - 1.32).abs() < 0.01, "{hi}");
    assert!((lo - 0.68).abs() < 0.01, "{lo}");
}
