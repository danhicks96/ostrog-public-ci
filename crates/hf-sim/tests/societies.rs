//! #322: those angry at the same one meet apart; one who comes to like them gives it away.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::mind::feelings::FeelingKind as F;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

#[test]
fn a_society_forms_against_one_and_is_betrayed() {
    let mut w = World::generate(3221, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = 50 * hf_sim::time::TICKS_PER_YEAR;
    w.rebuild_caches();
    let ids = w.found_party(5);
    w.meta.autoplan = false;
    let target = ids[0];
    let members = &ids[1..5];
    let t = w.meta.tick;
    for m in members {
        let i = w.agents.iter().position(|a| a.id == *m).unwrap();
        w.agents[i].born = t as i64 - 30 * hf_sim::time::TICKS_PER_YEAR as i64;
        w.agents[i]
            .feelings
            .feel(F::Anger, Some(target), 0.9, t, None);
        for o in members {
            if o != m {
                w.agents[i].relate(*o, 0.8, 0.5, t);
            }
        }
    }
    let at20 = |d: u64| d * TICKS_PER_DAY + 20 * TICKS_PER_HOUR + 77;
    let base = t / TICKS_PER_DAY + 1;
    w.meta.tick = at20(base);
    hf_sim::societies::step(&mut w);
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::SocietyFormed { against, .. } if against == target)));
    assert_eq!(w.societies.len(), 1);
    // One of them comes round to the one they meet against.
    let traitor = w.societies[0].members[0];
    let ti = w.agents.iter().position(|a| a.id == traitor).unwrap();
    w.agents[ti].relate(target, 2.0, 0.5, t);
    let mut betrayed = false;
    for d in 1..40u64 {
        w.meta.tick = at20(base + d);
        w.events.pending.clear();
        hf_sim::societies::step(&mut w);
        if w.events
            .pending
            .iter()
            .any(|e| matches!(e.kind, EventKind::Betrayed { by, .. } if by == traitor))
        {
            betrayed = true;
            break;
        }
    }
    assert!(betrayed);
    let other = w
        .agents
        .iter()
        .find(|a| members.contains(&a.id) && a.id != traitor)
        .unwrap();
    assert!(other.opinion_of(traitor) < 0.5, "they turn on the betrayer");
}

/// #404: the only society of the M5 soaks was betrayed the evening it formed (seed 4). One
/// angry at the target who also likes them is not taken in, and nobody betrays in the pass that
/// forms the society.
#[test]
fn one_who_likes_the_target_is_not_taken_in_and_nobody_betrays_the_evening_it_forms() {
    let mut w = World::generate(3222, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = 50 * hf_sim::time::TICKS_PER_YEAR;
    w.rebuild_caches();
    let ids = w.found_party(6);
    w.meta.autoplan = false;
    let target = ids[0];
    let members = &ids[1..6];
    let t = w.meta.tick;
    for m in members {
        let i = w.agents.iter().position(|a| a.id == *m).unwrap();
        w.agents[i].born = t as i64 - 30 * hf_sim::time::TICKS_PER_YEAR as i64;
        w.agents[i]
            .feelings
            .feel(F::Anger, Some(target), 0.9, t, None);
        for o in members {
            if o != m {
                w.agents[i].relate(*o, 0.8, 0.5, t);
            }
        }
    }
    // Angry at the target and fond of them at once.
    let torn = members[0];
    let ti = w.agents.iter().position(|a| a.id == torn).unwrap();
    w.agents[ti].relate(target, 2.0, 0.5, t);
    assert!(w.agents[ti].opinion_of(target) > hf_sim::societies::LIKES);
    let day = t / TICKS_PER_DAY + 1;
    w.meta.tick = day * TICKS_PER_DAY + 20 * TICKS_PER_HOUR + 77;
    hf_sim::societies::step(&mut w);
    assert_eq!(w.societies.len(), 1, "the rest still meet");
    assert!(!w.societies[0].members.contains(&torn), "not taken in");
    // Every member comes round before the evening is out: still nobody gives it away tonight.
    for m in w.societies[0].members.clone() {
        let i = w.agents.iter().position(|a| a.id == m).unwrap();
        w.agents[i].relate(target, 2.0, 0.5, t);
    }
    hf_sim::societies::step(&mut w);
    assert!(!w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Betrayed { .. })));
    assert_eq!(w.societies.len(), 1);
}

/// #322 (round-17): one awake in the night who sees a society gathered apart suspects those
/// there, tells the one it meets against when they like them, and the talk goes round; a member
/// does not take it up against their own.
#[test]
fn one_who_sees_a_society_meet_suspects_them_and_the_talk_goes_round() {
    use hf_sim::events::Severity;
    let mut w = World::generate(3222, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = 50 * hf_sim::time::TICKS_PER_YEAR;
    w.rebuild_caches();
    let ids = w.found_party(7);
    w.meta.autoplan = false;
    let t = w.meta.tick;
    for id in &ids {
        let i = w.agents.iter().position(|a| a.id == *id).unwrap();
        w.agents[i].born = t as i64 - 30 * hf_sim::time::TICKS_PER_YEAR as i64;
    }
    let (target, members, seer, friend) = (ids[0], &ids[1..4], ids[4], ids[5]);
    let cid = w.communities[0].id;
    w.societies.push(hf_sim::societies::Society {
        community: cid,
        against: target,
        members: members.to_vec(),
        formed: t,
        aim: None,
        struck: false,
    });
    let spot = hf_sim::societies::meeting_place(&w, &w.societies[0]).unwrap();
    let idx = |w: &World, id| w.agents.iter().position(|a| a.id == id).unwrap();
    for (n, m) in members.iter().enumerate() {
        let i = idx(&w, *m);
        w.agents[i].pos = w.grid.surface(spot.x, spot.y + n as i32 - 1);
        w.agents[i].activity = hf_sim::mind::Activity::Idle;
    }
    // Awake, six tiles off, and on good terms with the one they meet against.
    let si = idx(&w, seer);
    w.agents[si].pos = w.grid.surface(spot.x - 6, spot.y);
    w.agents[si].activity = hf_sim::mind::Activity::Idle;
    w.agents[si].relate(target, 0.6, 0.5, t);
    w.agents[si].relate(friend, 0.6, 0.5, t);
    // Everyone else asleep at home, far off.
    for id in [target, friend, ids[6]] {
        let i = idx(&w, id);
        w.agents[i].pos = w.grid.surface(spot.x - 25, spot.y);
    }
    let day = t / TICKS_PER_DAY + 1;
    w.meta.tick = day * TICKS_PER_DAY + TICKS_PER_HOUR + 77;
    w.events.drain();
    hf_sim::societies::step(&mut w);
    let seen: Vec<_> = w
        .events
        .pending
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::MeetingSeen {
                by,
                against,
                members,
            } => Some((*by, *against, *members)),
            _ => None,
        })
        .collect();
    assert_eq!(seen, vec![(seer, target, 3)]);
    let sus = |w: &World, who, of| {
        w.agents[idx(w, who)]
            .suspects
            .iter()
            .find(|x| x.0 == of)
            .map_or(0.0, |x| x.1)
    };
    for m in members {
        assert!(sus(&w, seer, *m) >= 0.35);
        assert!(sus(&w, target, *m) >= 0.3, "told the one they meet against");
    }
    // Not seen twice for the same night's sight.
    w.events.drain();
    hf_sim::societies::step(&mut w);
    w.meta.tick = (day + 1) * TICKS_PER_DAY + TICKS_PER_HOUR + 77;
    hf_sim::societies::step(&mut w);
    assert!(!w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::MeetingSeen { .. })));
    // The talk goes round: the seer tells a friend, who comes to suspect a member.
    let tick = w.meta.tick;
    let talk = |w: &mut World, a, b| {
        let id = w.events.push(
            tick,
            None,
            Severity::Minor,
            EventKind::Talked { a, b, warmth: 0.5 },
        );
        let e = w
            .events
            .pending
            .iter()
            .find(|e| e.id == id)
            .unwrap()
            .clone();
        w.last_events = vec![e];
        hf_sim::social::observe(w);
    };
    talk(&mut w, seer, friend);
    assert!(members.iter().any(|m| sus(&w, friend, *m) > 0.0));
    // A member told of it does not take it up against their own.
    let mi = idx(&w, members[0]);
    w.agents[mi].relate(seer, 0.6, 0.5, tick);
    talk(&mut w, seer, members[0]);
    assert!(members[1..].iter().all(|m| sus(&w, members[0], *m) == 0.0));
}
