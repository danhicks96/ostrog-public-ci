//! The catch-up card's read models (#1443).

use hf_sim::agents::Species;
use hf_sim::chronicle::Chronicle;
use hf_sim::events::{Event, EventKind, Severity};
use hf_sim::ids::EventId;
use hf_sim::view::summary;
use hf_sim::World;

/// #1443: coming back, the life events lead by name: the death, then the pairing; thirty
/// suspicions and a wound are not changes.
#[test]
fn changes_name_who_died_and_paired_first() {
    let w = World::opening_world(1);
    let people: Vec<_> = w
        .agents
        .iter()
        .filter(|a| a.species == Species::Person)
        .map(|a| a.id)
        .collect();
    assert!(people.len() >= 3);
    let t0 = w.meta.tick;
    let mut n = 0u32;
    let mut ev = |tick: u64, kind: EventKind| {
        n += 1;
        Event {
            id: EventId(n),
            tick,
            at: None,
            severity: Severity::Notable,
            kind,
        }
    };
    let mut events = vec![ev(
        t0 + 10,
        EventKind::Paired {
            a: people[0],
            b: people[1],
        },
    )];
    for i in 0..30 {
        events.push(ev(
            t0 + 20 + i,
            EventKind::Suspected {
                by: people[0],
                of: people[1],
                because: "the sickness".into(),
            },
        ));
    }
    events.push(ev(
        t0 + 60,
        EventKind::Wounded {
            agent: people[1],
            part: "chest".into(),
            kind: "cut".into(),
            cause: "a wolf".into(),
        },
    ));
    events.push(ev(
        t0 + 80,
        EventKind::Died {
            agent: people[2],
            cause: "a fever".into(),
        },
    ));
    let mut c = Chronicle::in_memory().unwrap();
    c.record(&events).unwrap();
    let ch = summary::changes(&w, &c, t0, t0 + 100).unwrap();
    assert_eq!(ch.len(), 2, "death and pairing only");
    assert_eq!(
        ch[0].agent,
        Some(people[2]),
        "the death first, though later"
    );
    assert_eq!(ch[1].agent, Some(people[0]));
    assert!(ch[0].tick > ch[1].tick);
    assert!(ch.iter().all(|c| !c.line.is_empty()));
}

fn chronicle_fixture(rows: Vec<(u64, Severity, EventKind)>) -> (Chronicle, Vec<Event>) {
    let events: Vec<_> = rows
        .into_iter()
        .enumerate()
        .map(|(i, (tick, severity, kind))| Event {
            // Simulation event IDs are deliberately different from the new history row IDs.
            id: EventId(1_000 + i as u32),
            tick,
            at: None,
            severity,
            kind,
        })
        .collect();
    let mut c = Chronicle::in_memory().unwrap();
    c.record(&events).unwrap();
    (c, events)
}

#[test]
fn catch_up_folds_before_cap_without_changing_named_changes_or_raw_markers() {
    let w = World::opening_world(1);
    let people: Vec<_> = w
        .agents
        .iter()
        .filter(|a| a.species == Species::Person && !hf_sim::view::known::is_unfound(&w, a.id))
        .map(|a| a.id)
        .collect();
    let &[a, b, d, ..] = people.as_slice() else {
        panic!("expected three known people");
    };
    let t0 = w.meta.tick;
    let notable = Severity::Notable;
    let suspect = |because: &str| EventKind::Suspected {
        by: a,
        of: b,
        because: because.into(),
    };
    let mut rows = vec![
        (t0 + 2, notable, EventKind::Paired { a, b }),
        (
            t0 + 3,
            Severity::Minor,
            EventKind::Died {
                agent: d,
                cause: "a fever".into(),
            },
        ),
        (t0 + 5, notable, suspect("the first alarm")),
        (
            t0 + 6,
            notable,
            EventKind::Suspected {
                by: a,
                of: d,
                because: "a different subject".into(),
            },
        ),
    ];
    for tick in t0 + 10..=t0 + 12 {
        // Same narration does not make these raids a repeat key.
        rows.push((
            tick,
            Severity::Major,
            EventKind::Raided {
                band: 7,
                what: "berries".into(),
                count: 2,
                open: false,
            },
        ));
    }
    for i in 0..30 {
        rows.push((t0 + 60 + i, notable, suspect("the sickness")));
    }
    rows.push((t0 + 30, notable, suspect("the newest account")));
    let anchor = rows.len() as i64;
    rows.extend([
        (t0, notable, suspect("outside the lower bound")),
        (t0 + 90, Severity::Minor, suspect("below notable")),
        (t0 + 91, notable, suspect("outside the upper bound")),
    ]);
    let (c, events) = chronicle_fixture(rows);
    let to = t0 + 90; // The minor row at this tick counts, but is not a notable detail.
    let raw = summary::since(&w, &c, t0, to, 4).unwrap();
    let named = summary::changes(&w, &c, t0, to).unwrap();
    assert_eq!(named.len(), 2);
    assert_eq!(named[0].agent, Some(d));
    assert_eq!(named[1].agent, Some(a));

    let details = summary::catch_up_details(&w, &c, t0, to, 4).unwrap();
    let ids = |details: &[summary::CatchUpDetail]| {
        details.iter().map(|d| d.history_id).collect::<Vec<_>>()
    };
    assert_eq!(ids(&details), vec![anchor, 7, 6, 5]);
    let folded = &details[0];
    assert_eq!(folded.count, 32);
    assert_eq!(folded.anchor_tick, t0 + 30);
    assert_eq!(folded.earliest_tick, t0 + 5);
    assert_eq!(folded.latest_tick, t0 + 89);
    assert_eq!(
        folded.line,
        hf_sim::view::narrate::line(&w, &events[anchor as usize - 1])
    );
    assert!(details[1..]
        .iter()
        .all(|d| d.count == 1 && d.line == details[1].line));
    let all = summary::catch_up_details(&w, &c, t0, to, 10).unwrap();
    assert_eq!(ids(&all), vec![anchor, 7, 6, 5, 4, 1]);
    assert_eq!(all[4].count, 1);
    assert!(all[4].line.contains("a different subject"));
    assert!(summary::catch_up_details(&w, &c, t0, to, 0)
        .unwrap()
        .is_empty());
    assert_eq!(
        summary::catch_up_details(&w, &c, t0, t0 + 12, 10)
            .unwrap()
            .first()
            .map(|d| d.anchor_tick),
        Some(t0 + 12),
        "the upper-bound row is included"
    );

    let after = summary::since(&w, &c, t0, to, 4).unwrap();
    assert_eq!(after.counts, raw.counts);
    assert!(after.counts.contains(&("suspected".into(), 34)));
    assert_eq!(after.major, raw.major);
    assert_eq!(
        after.major.iter().map(|r| r.0).collect::<Vec<_>>(),
        vec![anchor, anchor - 1, anchor - 2, anchor - 3]
    );
    let named_after = summary::changes(&w, &c, t0, to).unwrap();
    let named_rows = |changes: Vec<summary::Change>| {
        changes
            .into_iter()
            .map(|ch| (ch.agent, ch.history_id, ch.tick, ch.line))
            .collect::<Vec<_>>()
    };
    assert_eq!(named_rows(named_after), named_rows(named));
}

#[test]
fn viewer_hidden_repeats_cannot_change_visible_groups() {
    let w = World::opening_world(1);
    let lost = hf_sim::view::known::unfound(&w)[0];
    let people: Vec<_> = w
        .agents
        .iter()
        .filter(|a| a.species == Species::Person && !hf_sim::view::known::is_unfound(&w, a.id))
        .map(|a| a.id)
        .collect();
    let t0 = w.meta.tick;
    let visible = EventKind::Suspected {
        by: people[0],
        of: people[1],
        because: "visible concern".into(),
    };
    // An unseen accuser of the same subject has the same fold key as the visible row.
    let hidden = EventKind::Suspected {
        by: lost,
        of: people[1],
        because: "private concern".into(),
    };
    let (c, events) = chronicle_fixture(vec![
        (t0 + 30, Severity::Notable, visible),
        (t0 + 40, Severity::Notable, hidden),
        (
            t0 + 50,
            Severity::Major,
            EventKind::Died {
                agent: lost,
                cause: "a fever".into(),
            },
        ),
    ]);
    assert_eq!(
        hf_sim::view::fold::key(&events[0]),
        hf_sim::view::fold::key(&events[1])
    );
    let details = summary::catch_up_details(&w, &c, t0, t0 + 50, 10).unwrap();
    assert_eq!(details.len(), 1);
    let detail = &details[0];
    assert_eq!(detail.history_id, 1);
    assert_eq!(detail.count, 1);
    assert_eq!(detail.earliest_tick, t0 + 30);
    assert_eq!(detail.latest_tick, t0 + 30);
    assert!(detail.line.contains("visible concern"));
}
