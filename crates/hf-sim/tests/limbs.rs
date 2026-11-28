//! #284: a limb gone bad is taken to save its owner, and a made one fitted in its place.

use hf_sim::body::{Part, PartState, WoundKind};
use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::mind::techniques::Learned;
use hf_sim::mind::Technique;
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

#[test]
fn a_rotten_leg_is_taken_and_a_wooden_one_made() {
    let mut w = World::generate(2841, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 10;
    w.rebuild_caches();
    let ids = w.found_party(3);
    w.meta.autoplan = false;
    let idx: Vec<usize> = ids
        .iter()
        .map(|id| w.agents.iter().position(|a| a.id == *id).unwrap())
        .collect();
    let (p, healer, carpenter) = (idx[0], idx[1], idx[2]);
    for &k in &idx {
        w.agents[k].pos = w.grid.surface(30 + k as i32 % 3, 30);
        w.agents[k].activity = hf_sim::mind::Activity::Idle;
    }
    w.agents[healer]
        .techniques
        .learn(Technique::HerbLore, Learned::Past, 0.6, 0);
    w.agents[carpenter]
        .techniques
        .learn(Technique::Carpentry, Learned::Past, 0.6, 0);
    hf_sim::systems::health::wound(
        &mut w,
        p,
        Part::LEG_L,
        WoundKind::Cut,
        0.6,
        0.0,
        "a test",
        None,
    );
    w.agents[p].body.wounds.last_mut().unwrap().infected = true;
    w.agents[p].body.illness = Some(hf_sim::body::Illness {
        severity: 0.7,
        since: 0,
        kind: hf_sim::body::IllnessKind::WoundRot,
        treated: None,
    });
    let before = w.agents[p].body.work_factor();
    hf_sim::systems::health::limbs(&mut w);
    assert_eq!(
        w.agents[p].body.parts.get(&Part::LEG_L),
        Some(&PartState::Lost)
    );
    assert!(w.agents[p].body.ill() < 0.4, "the rot eased");
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::LimbTaken { .. })));
    // Wood to hand: the carpenter makes a leg.
    let at = w.agents[carpenter].pos;
    w.drop_item(hf_sim::items::ItemKind::Plank, 1, at);
    hf_sim::systems::health::limbs(&mut w);
    assert!(matches!(
        w.agents[p].body.parts.get(&Part::LEG_L),
        Some(PartState::Replaced(_))
    ));
    assert!(w.agents[p].body.summary().contains("wooden leg"));
    let _ = before;
}

/// #405: the limb is taken in the morning, when the valley is up, by a healer of the community
/// who comes from wherever they are working; not at half past five, when the healers slept.
#[test]
fn a_healer_comes_from_across_the_valley_in_the_morning() {
    let mut w = World::generate(2841, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 10;
    w.rebuild_caches();
    let ids = w.found_party(2);
    w.meta.autoplan = false;
    let (p, healer) = (
        w.agents.iter().position(|a| a.id == ids[0]).unwrap(),
        w.agents.iter().position(|a| a.id == ids[1]).unwrap(),
    );
    w.agents[p].pos = w.grid.surface(10, 30);
    w.agents[healer].pos = w.grid.surface(50, 30);
    for k in [p, healer] {
        w.agents[k].activity = hf_sim::mind::Activity::Idle;
    }
    w.agents[healer]
        .techniques
        .learn(Technique::HerbLore, Learned::Past, 0.6, 0);
    hf_sim::systems::health::wound(
        &mut w,
        p,
        Part::ARM_R,
        WoundKind::Cut,
        0.6,
        0.0,
        "a test",
        None,
    );
    w.agents[p].body.wounds.last_mut().unwrap().infected = true;
    w.agents[p].body.illness = Some(hf_sim::body::Illness {
        severity: 0.7,
        since: 0,
        kind: hf_sim::body::IllnessKind::WoundRot,
        treated: None,
    });
    let day = hf_sim::time::TICKS_PER_DAY;
    // Half past five: the daily pass, and no limb taken.
    w.meta.tick = day + 5 * TICKS_PER_HOUR + 600;
    hf_sim::systems::health::step(&mut w);
    assert_eq!(w.agents[p].body.parts.get(&Part::ARM_R), None);
    // Ten in the morning: forty tiles off, the healer comes.
    w.meta.tick = day + 10 * TICKS_PER_HOUR + 600;
    hf_sim::systems::health::step(&mut w);
    assert_eq!(
        w.agents[p].body.parts.get(&Part::ARM_R),
        Some(&PartState::Lost)
    );
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::LimbTaken { by, .. } if by == w.agents[healer].id)));
}

/// Body & Gear slice 0: a taken arm is kept as the hand's story, pointing at the chronicle event,
/// and the other hand has none.
#[test]
fn a_taken_arm_leaves_its_hands_history_linked_to_the_event() {
    use hf_sim::partlog::{Change, PartKey, Region, Side, Spot};
    let mut w = World::generate(2842, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 10;
    w.rebuild_caches();
    let ids = w.found_party(2);
    w.meta.autoplan = false;
    let idx: Vec<usize> = ids
        .iter()
        .map(|id| w.agents.iter().position(|a| a.id == *id).unwrap())
        .collect();
    let (p, healer) = (idx[0], idx[1]);
    for &k in &idx {
        w.agents[k].pos = w.grid.surface(30 + k as i32 % 3, 30);
        w.agents[k].activity = hf_sim::mind::Activity::Idle;
    }
    w.agents[healer]
        .techniques
        .learn(Technique::HerbLore, Learned::Past, 0.6, 0);
    hf_sim::systems::health::wound(
        &mut w,
        p,
        Part::ARM_L,
        WoundKind::Cut,
        0.6,
        0.0,
        "a test",
        None,
    );
    w.agents[p].body.wounds.last_mut().unwrap().infected = true;
    w.agents[p].body.illness = Some(hf_sim::body::Illness {
        severity: 0.7,
        since: 0,
        kind: hf_sim::body::IllnessKind::WoundRot,
        treated: None,
    });
    hf_sim::systems::health::limbs(&mut w);
    let left = PartKey::new(Side::Left, Region::Hand, Spot::Whole);
    let right = PartKey::new(Side::Right, Region::Hand, Spot::Whole);
    let h = w.agents[p].part_log.history(left, false);
    assert_eq!(h.len(), 1);
    assert_eq!(h[0].change, Change::Lost);
    let ev = h[0].event.expect("linked to an event");
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| e.id == ev && matches!(e.kind, EventKind::LimbTaken { .. })));
    assert!(w.agents[p].part_log.history(right, false).is_empty());
    assert!(w.agents[p].part_log.missing(left));
}
