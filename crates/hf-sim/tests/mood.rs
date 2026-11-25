//! #815: a long memory records the tick it was last renewed, not just when it was formed, so
//! mood can later weigh a memory kept fresh above one left to go stale.

use hf_sim::mind::memory::{MemKind, Memories, Record, Subject};

#[test]
fn a_reinforced_memory_records_when() {
    let mut m = Memories::default();
    m.record(Record {
        tick: 0,
        kind: MemKind::Meal,
        subject: Subject::None,
        valence: 0.6,
        salience: 0.8,
        event: None,
    });
    m.consolidate(100, false, false);
    let formed = m.long[0].formed;
    assert_eq!(formed, 100);
    assert_eq!(m.long[0].last, 100);

    m.record(Record {
        tick: 100,
        kind: MemKind::Meal,
        subject: Subject::None,
        valence: 0.6,
        salience: 0.8,
        event: None,
    });
    m.consolidate(500, false, false);
    assert_eq!(
        m.long[0].formed, formed,
        "formed does not move on reinforcement"
    );
    assert_eq!(m.long[0].last, 500, "last moves to when it was reinforced");
}

#[test]
fn an_old_memory_without_last_loads() {
    // Saved by an hf-sim built before #815: the field is absent from the map.
    let old = serde_json::json!({
        "kind": "Meal",
        "subject": "None",
        "valence": 0.5,
        "strength": 0.4,
        "formed": 12345,
        "reinforced": 2,
        "origins": [],
    });
    let mut out = Vec::new();
    let mut ser = rmp_serde::Serializer::new(&mut out).with_struct_map();
    serde::Serialize::serialize(&old, &mut ser).unwrap();
    let m: hf_sim::mind::memory::Memory = rmp_serde::from_slice(&out).unwrap();
    assert_eq!(m.formed, 12345);
    assert_eq!(m.last, 0, "absent from an old save, defaults to 0");
}

#[test]
fn mood_terms_sum_to_mood() {
    use hf_sim::agents::Species;
    use hf_sim::ids::TilePos;
    use hf_sim::world::gen::GenParams;
    use hf_sim::World;
    let mut w = World::generate(5101, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(3);
    let i = w
        .agents
        .iter()
        .position(|a| a.id == ids[0] && a.species == Species::Person)
        .unwrap();
    w.agents[i].needs.hunger = 0.1;
    w.agents[i].needs.thirst = 0.2;
    let stripe = hf_sim::systems::needs::STRIPE as usize;
    // Step until this person's stripe has run once.
    for _ in 0..stripe {
        hf_sim::systems::needs::step(&mut w);
        w.meta.tick += 1;
    }
    let a = &w.agents[i];
    // Nothing but the needs stripe ran, and a person's needs change only on their own stripe, so
    // the terms read now are the terms the stripe summed.
    let t = hf_sim::systems::needs::mood_terms(a, w.meta.tick);
    let sum: f32 = t.iter().sum();
    assert!(
        (sum.clamp(-1.0, 1.0) - a.mood).abs() < 1e-6,
        "terms {t:?} sum {sum} vs mood {}",
        a.mood
    );
}

fn needs_term(set: impl Fn(&mut hf_sim::mind::needs::Needs)) -> f32 {
    use hf_sim::ids::TilePos;
    use hf_sim::world::gen::GenParams;
    use hf_sim::World;
    let mut w = World::generate(5102, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(1);
    let a = w.agents.iter_mut().find(|a| a.id == ids[0]).unwrap();
    a.needs.hunger = 1.0;
    a.needs.rest = 1.0;
    a.needs.social = 1.0;
    a.needs.comfort = 1.0;
    a.needs.thirst = 1.0;
    a.needs.warmth = 1.0;
    set(&mut a.needs);
    hf_sim::systems::needs::mood_terms(a, w.meta.tick)[0]
}

/// OST-205: three comfortable needs used to hide a desperate one.
#[test]
fn a_starving_person_is_not_content() {
    let n = needs_term(|n| n.hunger = 0.0);
    assert!(n <= -0.3, "needs term {n} with hunger at zero");
}

#[test]
fn a_comfortable_person_is_only_mildly_lifted_by_needs() {
    let n = needs_term(|n| {
        n.hunger = 0.85;
        n.rest = 0.85;
        n.social = 0.85;
        n.comfort = 0.85;
        n.thirst = 0.85;
        n.warmth = 0.85;
    });
    assert!(
        (0.0..=0.15).contains(&n),
        "needs term {n} at 0.85 everywhere"
    );
}
