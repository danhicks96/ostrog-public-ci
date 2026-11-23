//! Read models behind the client fixes of C63 (QA round 2026-10-20): the dead left where they
//! fell (#185), the director's line in a viewer's words (#200), the lightning flash (#184) and
//! the inspect panel's list of what someone knows (#178).

use hf_sim::agents::Species;
use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::view::bodies::{Bodies, FALL_TICKS, LEFT_TICKS, POSE_FALL, POSE_LYING, TENDED_TICKS};
use hf_sim::view::{director, known, sound_of};
use hf_sim::World;

#[test]
fn the_dead_fall_lie_and_are_taken_away() {
    let mut w = World::opening_world(3);
    w.events.drain();
    let a = w
        .agents
        .iter()
        .find(|a| a.alive && a.species == Species::Person && a.community.is_some())
        .expect("someone in a community")
        .id;
    let t0 = w.meta.tick;
    hf_sim::systems::life::die(&mut w, a, "a test".into());
    let events = w.events.drain();
    assert!(events
        .iter()
        .any(|e| matches!(e.kind, EventKind::Died { agent, .. } if agent == a)));
    let tended = events
        .iter()
        .any(|e| matches!(e.kind, EventKind::LaidToRest { agent, .. } if agent == a));
    let mut b = Bodies::default();
    b.observe(&events);
    assert_eq!(b.pose(a, t0), Some(POSE_FALL), "falling at first");
    assert_eq!(b.pose(a, t0 + FALL_TICKS), Some(POSE_LYING), "then lying");
    let span = if tended { TENDED_TICKS } else { LEFT_TICKS };
    assert_eq!(b.pose(a, t0 + span - 1), Some(POSE_LYING), "still there");
    assert_eq!(b.pose(a, t0 + span), None, "taken away");
    b.prune(t0 + span);
    assert!(b.list.is_empty());
}

#[test]
fn a_body_nobody_tends_lies_longer() {
    let mut w = World::opening_world(3);
    let a = w.agents[0].id;
    let t0 = w.meta.tick;
    w.events.push(
        t0,
        Some(TilePos::new(1, 1, 0)),
        hf_sim::events::Severity::Major,
        EventKind::Died {
            agent: a,
            cause: "a test".into(),
        },
    );
    let mut b = Bodies::default();
    b.observe(&w.events.drain());
    assert_eq!(b.pose(a, t0 + TENDED_TICKS), Some(POSE_LYING));
    assert_eq!(b.pose(a, t0 + LEFT_TICKS), None);
}

#[test]
fn the_director_line_is_in_a_viewers_words() {
    assert_eq!(director::watching("identity_formed"), "the community");
    assert_eq!(director::watching("caravan_left"), "a caravan");
    for kind in [
        "identity_formed",
        "custom_adopted",
        "fire_started",
        "people at work",
        "settlement",
        "no_such_kind",
    ] {
        let l = director::watching(kind);
        assert!(!l.contains('_') && !l.is_empty(), "{kind} -> {l}");
    }
    // What the bridge sends: the suggestion's kind, put in words.
    let w = World::opening_world(3);
    let mut d = director::Director::default();
    let people = director::people(&w);
    let s = d.suggest(w.meta.tick, director::settlement_centre(&w), &people);
    assert!(!director::watching(s.kind).contains('_'));
}

#[test]
fn lightning_flashes_and_other_fires_flare() {
    let at = TilePos::new(3, 3, 0);
    let strike = EventKind::FireStarted {
        at,
        cause: "by lightning".into(),
    };
    let hearth = EventKind::FireStarted {
        at,
        cause: "from the hearth".into(),
    };
    assert_eq!(sound_of(&strike), Some("lightning"));
    assert_eq!(sound_of(&hearth), Some("flare"));
}

#[test]
fn what_someone_knows_reads_in_their_terms() {
    let mut w = World::opening_world(3);
    for _ in 0..600 {
        w.step();
    }
    let mut seen = 0;
    for a in w.agents.iter().filter(|a| a.alive) {
        for k in a.knowledge.map.keys() {
            let line = known::describe(&w, k);
            assert!(
                !line.contains("Id(") && !line.contains('(') && !line.is_empty(),
                "{line}"
            );
            seen += 1;
        }
    }
    assert!(seen > 0, "someone knows something");
}

/// #1481: rows of one kind and subject are one piece of news. 14 suspicions of one sleeper (by 9
/// people), 2 of another, and a death are three lines: 14, 2, and the death, which never folds;
/// each line keeps its first row's history id.
#[test]
fn fold_gathers_same_kind_and_subject() {
    use hf_sim::events::{Event, Severity};
    use hf_sim::ids::EventId;
    use hf_sim::time::TICKS_PER_DAY;
    use hf_sim::view::fold;
    let w = World::opening_world(1);
    let people: Vec<_> = w
        .agents
        .iter()
        .filter(|a| a.species == Species::Person)
        .map(|a| a.id)
        .collect();
    assert!(people.len() >= 3, "people to suspect and to suspect them");
    let (sleeper, other) = (people[0], people[1]);
    let row = |n: i64, tick: u64, kind: EventKind| {
        (
            n,
            tick,
            Event {
                id: EventId(n as u32),
                tick,
                at: Some(TilePos::new(10, 10, 3)),
                severity: Severity::Notable,
                kind,
            },
        )
    };
    let t0 = w.meta.tick;
    let mut rows = Vec::new();
    for i in 0..14u64 {
        let by = people[(i as usize % 9 + 2) % people.len()];
        rows.push(row(
            100 + i as i64,
            t0 + i * TICKS_PER_DAY,
            EventKind::Suspected {
                by,
                of: sleeper,
                because: "the sickness".into(),
            },
        ));
        if i == 3 || i == 9 {
            rows.push(row(
                200 + i as i64,
                t0 + i * TICKS_PER_DAY + 5,
                EventKind::Suspected {
                    by,
                    of: other,
                    because: "the sickness".into(),
                },
            ));
        }
    }
    rows.push(row(
        300,
        t0 + 20 * TICKS_PER_DAY,
        EventKind::Died {
            agent: people[2],
            cause: "a fever".into(),
        },
    ));
    let f = fold::fold(&rows, &w);
    assert_eq!(f.len(), 3, "{f:?}");
    assert_eq!((f[0].count, f[0].first_id), (14, 100));
    assert_eq!((f[1].count, f[1].first_id), (2, 203));
    assert_eq!(
        (f[2].count, f[2].first_id),
        (1, 300),
        "the death is its own line"
    );
    assert!(f[0].line.ends_with("(x 14, over 13 days)"), "{}", f[0].line);
    assert!(f[1].line.ends_with("(x 2, over 6 days)"), "{}", f[1].line);
    assert!(!f[2].line.contains("(x"), "{}", f[2].line);
    assert!(
        fold::key(&rows.last().unwrap().2).is_none(),
        "a death has no fold key"
    );
}

/// the panel's mood word and any sprite cue come from one rule.
#[test]
fn mood_band_follows_the_inspect_thresholds() {
    use hf_sim::view::why::{mood_band, mood_word};
    let mut w = World::opening_world(1);
    let i = w
        .agents
        .iter()
        .position(|a| a.species == Species::Person)
        .unwrap();
    // Exact edges: 0.3 is still content, 0.0 is low, -0.3 is miserable.
    for (m, band) in [
        (-0.5, 1),
        (-0.3, 1),
        (-0.1, 2),
        (0.0, 2),
        (0.2, 3),
        (0.3, 3),
        (0.5, 4),
    ] {
        w.agents[i].mood = m;
        assert_eq!(mood_band(&w.agents[i]), band, "mood {m}");
    }
    assert_eq!(mood_word(4), "cheerful");
    assert_eq!(mood_word(1), "miserable");
    w.agents[i].alive = false;
    assert_eq!(mood_band(&w.agents[i]), 0);
}

/// OST-1002: pulling the break threshold into one function did not change it.
#[test]
fn strain_threshold_is_unchanged() {
    let w = World::opening_world(1);
    for a in w.agents.iter().filter(|a| a.species == Species::Person) {
        assert_eq!(
            hf_sim::systems::strain::threshold(a),
            0.55 + a.genome.calm() * 0.35
        );
    }
}
