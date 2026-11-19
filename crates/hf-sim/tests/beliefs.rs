//! Beliefs with sources (design-organic.md section 2): they weight choices within a cap, they
//! come from somewhere, they spread by talk, and outcomes confirm or shake them.

use hf_sim::agents::Species;
use hf_sim::events::EventKind;
use hf_sim::ids::EventId;
use hf_sim::mind::beliefs::{Beliefs, Source, Topic, CAP};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

#[test]
fn belief_has_sources() {
    let mut held = 0;
    for seed in 0..12 {
        let w = World::opening_world(seed);
        let pending: Vec<_> = w.events.pending.clone();
        for a in w.agents.iter().filter(|a| a.species == Species::Person) {
            for b in &a.beliefs.list {
                held += 1;
                assert!(
                    !b.sources.is_empty(),
                    "{} believes {:?} from nowhere",
                    a.name,
                    b.topic
                );
                for s in &b.sources {
                    if let Source::Event(id) = s {
                        let e = pending
                            .iter()
                            .find(|e| e.id == *id)
                            .expect("a recorded event");
                        assert!(
                            matches!(e.kind, EventKind::Lived { .. }),
                            "a belief from their past points at their past"
                        );
                    }
                }
            }
        }
    }
    assert!(held > 30, "survivors arrive believing things: {held}");
}

#[test]
fn weight_stays_within_the_cap() {
    let mut b = Beliefs::default();
    for (k, s) in [-1.0f32, -0.3, 0.0, 0.4, 1.0].iter().enumerate() {
        b.hold(Topic::Fire, *s, 1.0, Source::Event(EventId(k as u32)), 0);
        for dir in [-1.0, 1.0] {
            let x = b.weight(Topic::Fire, dir);
            assert!((1.0 - CAP..=1.0 + CAP).contains(&x), "{x}");
        }
    }
    assert_eq!(
        Beliefs::default().weight(Topic::Hunger, 1.0),
        1.0,
        "no belief, no weight"
    );
}

#[test]
fn beliefs_spread_by_talk() {
    let mut w = World::generate(4, GenParams::small());
    let p = w.meta.founding_site;
    let a = w.spawn_agent(Species::Person, p);
    let b = w.spawn_agent(Species::Person, p);
    let (ia, ib) = (
        w.agents.iter().position(|x| x.id == a).unwrap(),
        w.agents.iter().position(|x| x.id == b).unwrap(),
    );
    w.agents[ia]
        .beliefs
        .hold(Topic::Fire, -0.9, 0.9, Source::Event(EventId(1)), 0);
    w.agents[ib].relate(a, 0.4, 0.6, 0);
    for t in 0..12 {
        hf_sim::systems::beliefs::share(&mut w, ia, ib, t);
    }
    let got = w.agents[ib]
        .beliefs
        .get(Topic::Fire)
        .expect("told often enough, they hold it");
    assert!(got.stance < 0.0, "toward the teller: {}", got.stance);
    assert!(
        got.sources.contains(&Source::Person(a)),
        "and they know who told them"
    );
    // A stranger they think ill of is not believed.
    let c = w.spawn_agent(Species::Person, p);
    let ic = w.agents.iter().position(|x| x.id == c).unwrap();
    w.agents[ic].relate(a, -0.9, 0.5, 0);
    hf_sim::systems::beliefs::share(&mut w, ia, ic, 20);
    assert!(w.agents[ic].beliefs.get(Topic::Fire).is_none());
}

#[test]
fn outcomes_confirm_and_shake() {
    let mut b = Beliefs::default();
    b.hold(Topic::Fire, -0.6, 0.5, Source::Event(EventId(1)), 0);
    b.experience(Topic::Fire, -1.0, 0.3, Source::Event(EventId(2)), 1);
    let sure = b.get(Topic::Fire).unwrap().confidence;
    assert!(sure > 0.5, "seeing it work makes them surer: {sure}");
    for k in 0..12 {
        b.experience(Topic::Fire, 1.0, 0.3, Source::Event(EventId(10 + k)), 2);
    }
    let f = b.get(Topic::Fire).unwrap();
    assert!(
        f.stance > 0.0,
        "enough burned houses turn them: {}",
        f.stance
    );
    assert!(f.sources.len() <= hf_sim::mind::beliefs::MAX_SOURCES);
}

#[test]
fn children_start_with_their_parents_lightly() {
    let mut w = World::generate(9, GenParams::small());
    let p = w.meta.founding_site;
    let m = w.spawn_agent(Species::Person, p);
    let f = w.spawn_agent(Species::Person, p);
    for (id, s) in [(m, -0.8f32), (f, -0.6)] {
        let a = w.agents.iter_mut().find(|x| x.id == id).unwrap();
        a.beliefs
            .hold(Topic::Strangers, s, 0.9, Source::Event(EventId(3)), 0);
    }
    let kid = w.give_birth(m, f).expect("a child");
    let k = w.agents.iter().find(|x| x.id == kid).unwrap();
    let b = k.beliefs.get(Topic::Strangers).expect("raised with it");
    assert!(b.stance < 0.0);
    assert!(b.confidence <= 0.35, "held lightly: {}", b.confidence);
    assert!(b
        .sources
        .iter()
        .any(|s| matches!(s, Source::Person(x) if *x == m || *x == f)));
}

#[test]
fn ideology_is_beliefs_not_rules() {
    let mut w = World::opening_world(3);
    let choice = EventId(999_999);
    w.seed_ideology(&[(Topic::Strangers, -0.8)], choice);
    let humans: Vec<usize> = w
        .agents
        .iter()
        .enumerate()
        .filter(|(_, a)| a.species == Species::Person && a.community.is_some())
        .map(|(k, _)| k)
        .collect();
    for &k in &humans {
        let b = w.agents[k].beliefs.get(Topic::Strangers).unwrap();
        assert!(b.sources.contains(&Source::Ideology(choice)));
        assert!(b.stance < 0.2, "the group's view is in them: {}", b.stance);
    }
    // And it can be lost: a run of bad experiences turns the most persuadable.
    let k = humans[0];
    for e in 0..15 {
        w.agents[k]
            .beliefs
            .experience(Topic::Strangers, 1.0, 0.3, Source::Event(EventId(e)), 10);
    }
    assert!(w.agents[k].beliefs.get(Topic::Strangers).unwrap().stance > 0.0);
}
