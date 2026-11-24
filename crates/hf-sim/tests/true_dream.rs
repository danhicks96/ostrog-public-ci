//! #308: the curse bag grows — the true-dream. The thin place marks one who sleeps within
//! its reach while it stands woke; every third night they cry out in their sleep, of
//! something true to the record, and their community comes to suspect them of dealing with
//! the hollow. The suspicion has the cry for its source row, and it is wrong: the mark is
//! the thin place's, and they are its victim, not its agent (P11: a suspicion can be wrong).

use hf_sim::conditions::ConditionKind as C;
use hf_sim::events::EventKind;
use hf_sim::ids::AgentId;
use hf_sim::mind::{Activity, Subject};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::World;

/// Set the clock to `h`:07 on day `d` and run the conditions step once.
fn at(w: &mut World, d: u64, h: u64) {
    w.meta.tick = d * TICKS_PER_DAY + h * TICKS_PER_HOUR + 7;
    hf_sim::conditions::step(w);
}

fn pending(w: &World, f: impl Fn(&EventKind) -> bool) -> usize {
    w.events.pending.iter().filter(|e| f(&e.kind)).count()
}

/// Wake the thin place and put `who` to sleep within its reach, with two of their community
/// sleeping near, so there is someone to hear the cry.
fn hollow_sleeper(w: &mut World, who: AgentId) -> usize {
    let s = hf_sim::thin::stone(w).expect("the arrival places the stone");
    let tick = w.meta.tick;
    w.thin.woke = Some(tick);
    w.thin.strength = 1.0;
    let i = w.agents.iter().position(|a| a.id == who).unwrap();
    w.agents[i].pos = s.offset(4, 0, 0);
    w.agents[i].activity = Activity::Sleeping {
        bed: None,
        until: tick + TICKS_PER_DAY,
    };
    let cid = w.agents[i].community;
    let mut n = 0;
    for a in w.agents.iter_mut() {
        if a.id != who && a.community == cid && n < 2 {
            a.pos = s.offset(4 + n + 1, 0, 0);
            a.activity = Activity::Sleeping {
                bed: None,
                until: tick + TICKS_PER_DAY,
            };
            n += 1;
        }
    }
    i
}

fn keep_sleeping(w: &mut World, i: usize) {
    let until = w.meta.tick + TICKS_PER_DAY;
    w.agents[i].activity = Activity::Sleeping { bed: None, until };
}

#[test]
fn the_true_dream_marks_a_hollow_sleeper() {
    let mut w = World::opening_world(61);
    let cid = w.communities[0].id;
    let who = w
        .agents
        .iter()
        .find(|a| a.community == Some(cid))
        .unwrap()
        .id;
    let i = hollow_sleeper(&mut w, who);
    let mut marked = false;
    for d in 1..60 {
        at(&mut w, d, 1);
        if hf_sim::conditions::has(&w, i, C::TrueDream) {
            marked = true;
            break;
        }
        keep_sleeping(&mut w, i);
    }
    assert!(
        marked,
        "sleeping in the hollow marked them, within two months"
    );
    let c = w.agents[i]
        .conditions
        .iter()
        .find(|c| c.kind == C::TrueDream)
        .unwrap();
    // The condition holds the row that gave it: their own cursing, recorded.
    assert!(matches!(
        w.events
            .pending
            .iter()
            .find(|e| e.id == c.cause)
            .map(|e| &e.kind),
        Some(EventKind::Cursed { agent, condition, .. })
            if *agent == who && condition == "the true-dream"
    ));
}

#[test]
fn the_true_dream_is_suspected_and_the_suspicion_is_wrong() {
    let mut w = World::opening_world(61);
    let cid = w.communities[0].id;
    let who = w
        .agents
        .iter()
        .find(|a| a.community == Some(cid))
        .unwrap()
        .id;
    let i = hollow_sleeper(&mut w, who);
    // Until marked.
    for d in 1..60 {
        at(&mut w, d, 1);
        if hf_sim::conditions::has(&w, i, C::TrueDream) {
            break;
        }
        keep_sleeping(&mut w, i);
    }
    assert!(hf_sim::conditions::has(&w, i, C::TrueDream));
    let cause = w.agents[i]
        .conditions
        .iter()
        .find(|c| c.kind == C::TrueDream)
        .unwrap()
        .cause;
    // Three nights on: the cry, and a suspicion.
    let mut dream = None;
    for d in 60..72 {
        at(&mut w, d, 1);
        keep_sleeping(&mut w, i);
        dream = w
            .events
            .pending
            .iter()
            .find(|e| matches!(&e.kind, EventKind::TrueDream { agent, .. } if *agent == who))
            .map(|e| e.id);
        if dream.is_some() {
            break;
        }
    }
    let dream = dream.expect("they cried out in their sleep");
    // The cry is true to the record: it holds their curse's row.
    let EventKind::TrueDream {
        cause: dream_cause, ..
    } = &w
        .events
        .pending
        .iter()
        .find(|e| e.id == dream)
        .unwrap()
        .kind
    else {
        unreachable!()
    };
    assert_eq!(*dream_cause, cause);
    // Their community suspected them, with the cry as the suspicion's source row.
    assert!(
        pending(
            &w,
            |k| matches!(k, EventKind::Suspected { of, .. } if *of == who)
        ) > 0,
        "their community suspected them"
    );
    assert!(
        w.agents.iter().any(|a| a.memories.short.iter().any(|r| {
            r.subject == Subject::Person(who) && r.valence < 0.0 && r.event == Some(dream)
        })),
        "the suspicion's memory holds the cry's row as its source"
    );
    // And the suspicion is wrong: what they are blamed for is the curse's doing. The
    // people cite the cry; the record knows the curse's row, and they are its victim.
    let blamed = w
        .events
        .pending
        .iter()
        .find(|e| matches!(&e.kind, EventKind::Suspected { of, .. } if *of == who))
        .unwrap();
    let EventKind::Suspected { because, .. } = &blamed.kind else {
        unreachable!()
    };
    assert!(
        because.contains("no one told them"),
        "the suspicion names the pattern: {because}"
    );
}

#[test]
fn the_unbinding_lifts_the_true_dream_from_one_suspected() {
    let mut w = World::opening_world(61);
    let cid = w.communities[0].id;
    let who = w
        .agents
        .iter()
        .find(|a| a.community == Some(cid))
        .unwrap()
        .id;
    let i = hollow_sleeper(&mut w, who);
    // Until marked, and until a dream has made one of the community suspect them.
    for d in 1..72 {
        at(&mut w, d, 1);
        keep_sleeping(&mut w, i);
        if hf_sim::conditions::has(&w, i, C::TrueDream)
            && pending(
                &w,
                |k| matches!(k, EventKind::Suspected { of, .. } if *of == who),
            ) > 0
        {
            break;
        }
    }
    assert!(hf_sim::conditions::has(&w, i, C::TrueDream));
    // A healer of their community who has come to suspect them (the dream made one).
    let healer = w
        .agents
        .iter()
        .find(|a| {
            a.alive
                && a.id != who
                && a.community == Some(cid)
                && a.memories
                    .short
                    .iter()
                    .any(|r| r.subject == Subject::Person(who) && r.valence < 0.0)
        })
        .map(|a| a.id)
        .expect("a community member suspects them");
    let h = w.agents.iter().position(|a| a.id == healer).unwrap();
    w.agents[h].techniques.learn(
        hf_sim::mind::Technique::Unbinding,
        hf_sim::mind::techniques::Learned::Past,
        0.5,
        w.meta.tick,
    );
    // The unbinding runs in the morning pass.
    at(&mut w, 75, 6);
    assert!(
        !hf_sim::conditions::has(&w, i, C::TrueDream),
        "the healer lifted the true-dream from the one they suspected"
    );
    assert!(
        pending(
            &w,
            |k| matches!(k, EventKind::Unbound { agent, condition, .. }
            if *agent == who && condition == "the true-dream")
        ) > 0,
        "the lifting is recorded"
    );
}

#[test]
fn a_suspicion_is_not_repeated_without_new_evidence() {
    let mut w = World::opening_world(61);
    let cid = w.communities[0].id;
    let who = w
        .agents
        .iter()
        .find(|a| a.community == Some(cid))
        .unwrap()
        .id;
    let i = hollow_sleeper(&mut w, who);
    let mut d = 1;
    while !hf_sim::conditions::has(&w, i, C::TrueDream) {
        at(&mut w, d, 1);
        keep_sleeping(&mut w, i);
        d += 1;
        assert!(d < 200, "never marked");
    }
    w.events.pending.clear();
    for n in 0..15 {
        at(&mut w, d + n, 1);
        keep_sleeping(&mut w, i);
    }
    let by_first = |w: &World| {
        let mut by: Vec<AgentId> = w
            .events
            .pending
            .iter()
            .filter_map(|e| match &e.kind {
                EventKind::Suspected { by, of, .. } if *of == who => Some(*by),
                _ => None,
            })
            .collect();
        by.sort();
        by.dedup();
        by.len()
    };
    let total = pending(
        &w,
        |k| matches!(k, EventKind::Suspected { of, .. } if *of == who),
    );
    assert_eq!(by_first(&w), 1, "one neighbour suspects");
    assert_eq!(total, 1, "and only once over fifteen nights");
}
