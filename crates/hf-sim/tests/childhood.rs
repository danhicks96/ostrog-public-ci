//! #296: a child learns by watching, and comes of age on record.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::mind::Skill;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR, TICKS_PER_YEAR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn party() -> (World, usize, usize) {
    let mut w = World::generate(2961, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = 30 * TICKS_PER_YEAR;
    w.rebuild_caches();
    let ids = w.found_party(2);
    w.meta.autoplan = false;
    let a = w.agents.iter().position(|x| x.id == ids[0]).unwrap();
    let c = w.agents.iter().position(|x| x.id == ids[1]).unwrap();
    (w, a, c)
}

#[test]
fn a_child_beside_someone_at_work_learns_its_skill() {
    let (mut w, adult, child) = party();
    let t = w.meta.tick;
    w.agents[child].born = t as i64 - 9 * TICKS_PER_YEAR as i64;
    w.agents[child].age_scale = 1.0;
    w.agents[child].activity = hf_sim::mind::Activity::Idle;
    let at = w.grid.surface(40, 40);
    w.agents[adult].pos = at;
    w.agents[child].pos = at.offset(1, 0, 0);
    // The adult at a mining order.
    let d = hf_sim::systems::settlement::make(&mut w, hf_sim::work::DesignationKind::Mine, at, 5);
    let did = d.id;
    w.designations.push(d);
    w.agents[adult].activity = hf_sim::mind::Activity::Working {
        plan: hf_sim::work::Plan {
            designation: did,
            steps: vec![],
            cursor: 0,
            label: "mining".into(),
        },
    };
    let before = w.agents[child].skills.level(Skill::Mining);
    for h in 1..200u64 {
        w.meta.tick = t + h * TICKS_PER_HOUR - t % TICKS_PER_HOUR + 222;
        hf_sim::systems::childhood::step(&mut w);
    }
    assert!(w.agents[child].skills.level(Skill::Mining) > before);
}

#[test]
fn coming_of_age_is_on_record() {
    let (mut w, _adult, child) = party();
    let t = w.meta.tick - w.meta.tick % TICKS_PER_DAY + 9 * TICKS_PER_HOUR + 222;
    w.meta.tick = t;
    w.agents[child].age_scale = 1.0;
    // Fifteen today.
    w.agents[child].born = t as i64 - 15 * TICKS_PER_YEAR as i64 + 1000;
    hf_sim::systems::childhood::step(&mut w);
    assert!(w
        .events
        .pending
        .iter()
        .all(|e| !matches!(e.kind, EventKind::CameOfAge { .. })));
    w.meta.tick = t + TICKS_PER_DAY;
    hf_sim::systems::childhood::step(&mut w);
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::CameOfAge { .. })));
}

#[test]
fn childhood_has_three_stages_scaled_to_each_kind() {
    use hf_sim::agents::Childhood::{Infant, Older, Small};
    let (mut w, _adult, c) = party();
    let t = w.meta.tick;
    w.agents[c].age_scale = 1.0;
    let at = |w: &mut World, years: f64| {
        w.agents[c].born = t as i64 - (years * TICKS_PER_YEAR as f64) as i64;
        w.agents[c].childhood(t)
    };
    w.agents[c].people = hf_sim::species::SpeciesId::HUMAN;
    assert_eq!(at(&mut w, 1.0), Some(Infant));
    assert_eq!(at(&mut w, 5.0), Some(Small));
    assert_eq!(at(&mut w, 10.0), Some(Older));
    assert_eq!(at(&mut w, 16.0), None);
    // Thresholds: 2 and 7 years exactly.
    assert_eq!(at(&mut w, 1.99), Some(Infant));
    assert_eq!(at(&mut w, 2.0), Some(Small));
    assert_eq!(at(&mut w, 6.99), Some(Small));
    assert_eq!(at(&mut w, 7.0), Some(Older));
    // A kind maturing at 30 scales them by two: infant under 4, small under 14.
    w.agents[c].people = hf_sim::species::by_key("seamfolk").unwrap();
    assert_eq!(at(&mut w, 3.0), Some(Infant));
    assert_eq!(at(&mut w, 10.0), Some(Small));
    assert_eq!(at(&mut w, 20.0), Some(Older));
    assert_eq!(at(&mut w, 31.0), None);
    // The helpers.
    w.agents[c].people = hf_sim::species::SpeciesId::HUMAN;
    at(&mut w, 1.0);
    assert!(!w.agents[c].past_infancy(t));
    assert!(!w.agents[c].old_enough_for(Small, t));
    at(&mut w, 5.0);
    assert!(w.agents[c].past_infancy(t));
    assert!(w.agents[c].old_enough_for(Small, t));
    assert!(!w.agents[c].old_enough_for(Older, t));
    at(&mut w, 20.0);
    assert!(w.agents[c].old_enough_for(Older, t));
}
