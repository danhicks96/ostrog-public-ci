//! C74: tissue layers and treatment that changes a wound's course (PLAN-FINISH M5).

use hf_sim::body::{Part, PartState, Tissue, Treatment, WoundKind};
use hf_sim::ids::TilePos;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn meadow(seed: u64) -> (World, usize) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    let ids = w.found_party(3);
    w.meta.autoplan = false;
    w.ensure_nav();
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    (w, i)
}

/// Hours of health alone, nobody tending, everyone fed.
fn hours(w: &mut World, n: u64) {
    for _ in 0..n {
        for a in w.agents.iter_mut() {
            a.needs.hunger = 1.0;
        }
        let t = w.meta.tick;
        w.meta.tick = (t / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR + 600;
        hf_sim::systems::health::step(w);
    }
}

#[test]
fn a_wound_has_a_depth() {
    assert_eq!(Tissue::of(WoundKind::Cut, 0.2, Part::ARM_L), Tissue::Skin);
    assert_eq!(Tissue::of(WoundKind::Cut, 0.5, Part::ARM_L), Tissue::Muscle);
    assert_eq!(Tissue::of(WoundKind::Break, 0.3, Part::LEG_R), Tissue::Bone);
    assert_eq!(
        Tissue::of(WoundKind::Puncture, 0.7, Part::TORSO),
        Tissue::Organ
    );
    assert_eq!(
        Tissue::of(WoundKind::Puncture, 0.7, Part::ARM_L),
        Tissue::Muscle
    );
}

#[test]
fn a_deep_wound_heals_slower_than_a_graze() {
    let (mut w, i) = meadow(7411);
    hf_sim::systems::health::wound(
        &mut w,
        i,
        Part::ARM_L,
        WoundKind::Bruise,
        0.3,
        0.0,
        "a test",
        None,
    );
    hf_sim::systems::health::wound(
        &mut w,
        i,
        Part::ARM_R,
        WoundKind::Break,
        0.3,
        0.0,
        "a test",
        None,
    );
    hours(&mut w, 18);
    let b = &w.agents[i].body;
    let graze = b
        .wounds
        .iter()
        .find(|x| x.part == Part::ARM_L)
        .map(|x| x.severity)
        .unwrap_or(0.0);
    let brk = b
        .wounds
        .iter()
        .find(|x| x.part == Part::ARM_R)
        .map(|x| x.severity)
        .unwrap();
    assert!(
        brk > graze + 0.05,
        "the break {brk} heals slower than the bruise {graze}"
    );
}

#[test]
fn bleeding_inside_does_not_stop_until_it_is_stitched() {
    let (mut w, i) = meadow(7412);
    hf_sim::systems::health::wound(
        &mut w,
        i,
        Part::TORSO,
        WoundKind::Puncture,
        0.7,
        0.4,
        "a test",
        None,
    );
    assert_eq!(w.agents[i].body.wounds[0].tissue, Tissue::Organ);
    hours(&mut w, 6);
    let bleeding = w.agents[i].body.wounds[0].bleeding;
    assert!(
        bleeding >= 0.4,
        "still bleeding inside after six hours: {bleeding}"
    );
    // Stitched, it stops.
    w.agents[i].body.wounds[0].treated = Some(Treatment::Stitched);
    hours(&mut w, 3);
    assert_eq!(w.agents[i].body.wounds[0].bleeding, 0.0);
}

#[test]
fn a_break_nobody_set_heals_crooked_and_a_splinted_one_straight() {
    for splint in [false, true] {
        let (mut w, i) = meadow(7413 + splint as u64);
        hf_sim::systems::health::wound(
            &mut w,
            i,
            Part::LEG_L,
            WoundKind::Break,
            0.2,
            0.0,
            "a test",
            None,
        );
        if splint {
            w.agents[i].body.wounds[0].treated = Some(Treatment::Splinted);
        }
        hours(&mut w, 24 * 8);
        let b = &w.agents[i].body;
        assert!(b.wounds.is_empty(), "healed in eight days");
        let crooked = b.parts.get(&Part::LEG_L) == Some(&PartState::Scarred);
        assert_eq!(crooked, !splint, "splinted {splint}: crooked {crooked}");
        if !splint {
            assert!(b.summary().contains("set crooked"), "{}", b.summary());
            assert!(b.work_factor() < 1.0);
        }
    }
}

#[test]
fn a_healer_treats_each_wound_as_it_needs() {
    assert_eq!(
        Treatment::for_wound(WoundKind::Break, Tissue::Bone),
        Treatment::Splinted
    );
    assert_eq!(
        Treatment::for_wound(WoundKind::Cut, Tissue::Muscle),
        Treatment::Stitched
    );
    assert_eq!(
        Treatment::for_wound(WoundKind::Puncture, Tissue::Organ),
        Treatment::Stitched
    );
    assert_eq!(
        Treatment::for_wound(WoundKind::Burn, Tissue::Skin),
        Treatment::Salved
    );
    assert_eq!(
        Treatment::for_wound(WoundKind::Bruise, Tissue::Skin),
        Treatment::Dressed
    );
}

#[test]
fn the_inspect_line_names_the_layer_and_the_treatment() {
    let (mut w, i) = meadow(7415);
    hf_sim::systems::health::wound(
        &mut w,
        i,
        Part::ARM_L,
        WoundKind::Cut,
        0.5,
        0.1,
        "a test",
        None,
    );
    w.agents[i].body.wounds[0].treated = Some(Treatment::Stitched);
    let s = w.agents[i].body.summary();
    assert!(s.contains("to the muscle") && s.contains("stitched"), "{s}");
    let _ = TICKS_PER_DAY;
}

/// #872: a wound used to be remembered only as "something bad here" — a `Hardship` record
/// about the place it happened, with no event behind it and nothing about who struck it. A
/// bite with a known attacker (the beast's own agent id) must leave the victim's short-term
/// record pointing at the real `Wounded` event, and a second record naming the attacker.
#[test]
fn a_wound_memory_points_to_its_event() {
    let (mut w, i) = meadow(7416);
    let beast = w.agents[i].id; // stand-in attacker id; only its identity matters here.
    hf_sim::systems::health::wound(
        &mut w,
        i,
        Part::ARM_R,
        WoundKind::Puncture,
        0.4,
        0.2,
        "bitten by a beast",
        Some(beast),
    );
    let eid = w
        .events
        .pending
        .last()
        .expect("the Wounded event was pushed")
        .id;
    let short = &w.agents[i].memories.short;
    let place_record = short
        .iter()
        .find(|r| {
            r.kind == hf_sim::mind::MemKind::Hardship
                && matches!(r.subject, hf_sim::mind::Subject::Place(_))
        })
        .expect("a hardship memory about the place");
    assert_eq!(
        place_record.event,
        Some(eid),
        "the place memory should point at the real event, not stand alone"
    );
    let attacker_record = short
        .iter()
        .find(|r| {
            r.kind == hf_sim::mind::MemKind::Hardship
                && r.subject == hf_sim::mind::Subject::Person(beast)
        })
        .expect("a second hardship memory naming the attacker");
    assert_eq!(attacker_record.event, Some(eid));
}
