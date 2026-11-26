//! The catch-up card's read models (#1443).

use hf_sim::agents::Species;
use hf_sim::chronicle::Chronicle;
use hf_sim::events::{Event, EventKind, Severity};
use hf_sim::ids::{EventId, ItemId, TilePos};
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

/// OST-010: two deaths, a birth and many hauling rows read as "died 2, born 1"; the raw counts
/// still carry the hauling.
#[test]
fn summary_counts_read_as_words() {
    let w = World::opening_world(3);
    let a = w.agents[0].id;
    let mut evs = Vec::new();
    let mut push = |kind: EventKind, sev: Severity| {
        evs.push(Event {
            id: EventId(evs.len() as u32 + 1),
            tick: w.meta.epoch + 10,
            at: None,
            severity: sev,
            kind,
        });
    };
    for _ in 0..2 {
        push(
            EventKind::Died {
                agent: a,
                cause: "age".into(),
            },
            Severity::Major,
        );
    }
    push(
        EventKind::Born {
            agent: a,
            mother: a,
            father: None,
        },
        Severity::Major,
    );
    for _ in 0..40 {
        push(
            EventKind::Hauled {
                agent: a,
                item: ItemId(1),
                to: TilePos { x: 0, y: 0, z: 0 },
                load: None,
            },
            Severity::Minor,
        );
    }
    let mut c = Chronicle::in_memory().unwrap();
    c.record(&evs).unwrap();
    let sum = summary::since(&w, &c, 0, w.meta.epoch + 20, 12).unwrap();
    assert_eq!(sum.counts_in_words(), "died 2, born 1");
    assert!(
        sum.counts.iter().any(|(k, n)| k == "hauled" && *n == 40),
        "raw counts keep the hauling"
    );
}

/// OST-011: the Catch Up card reads the headline, then the named changes, then the folded
/// details; a row told by name is not repeated below, and the raw counts stay as they were.
#[test]
fn catch_up_card_orders_headline_changes_then_folded_details() {
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
    let mut rows = vec![
        (t0 + 2, notable, EventKind::Paired { a, b }),
        (
            t0 + 3,
            notable,
            EventKind::Died {
                agent: d,
                cause: "a fever".into(),
            },
        ),
    ];
    for i in 0..3 {
        rows.push((
            t0 + 10 + i,
            notable,
            EventKind::Suspected {
                by: a,
                of: b,
                because: "the sickness".into(),
            },
        ));
    }
    for (n, what) in [(1u32, "berries"), (2, "grain")] {
        rows.push((
            t0 + 20 + n as u64,
            Severity::Major,
            EventKind::Raided {
                band: n,
                what: what.into(),
                count: 2,
                open: false,
            },
        ));
    }
    let (c, _) = chronicle_fixture(rows);
    let to = t0 + 50;
    let raw = summary::since(&w, &c, t0, to, 12).unwrap();
    let card = summary::catch_up_card(&w, &c, t0, to, 10).unwrap();
    assert_eq!(card.headline, raw.counts_in_words());
    assert_eq!(card.changes.len(), 2, "the death and the pairing");
    assert_eq!(card.details.len(), 3, "folded suspicions and two raids");
    assert!(
        card.details.iter().any(|l| l.ends_with("(x3)")),
        "the folded line says how many: {:?}",
        card.details
    );
    let text = card.bbcode();
    let head = text.find(&card.headline).unwrap();
    let changes = text.find("What changed").unwrap();
    let details = text.find("Also noted").unwrap();
    assert!(head < changes && changes < details);
    assert!(text.find(&card.changes[0]).unwrap() < details);
    let after = summary::since(&w, &c, t0, to, 12).unwrap();
    assert_eq!(after.counts, raw.counts, "raw counts unchanged");
}

fn born_at(a: hf_sim::ids::AgentId, tick: u64) -> (u64, Severity, EventKind) {
    (
        tick,
        Severity::Major,
        EventKind::Born {
            agent: a,
            mother: a,
            father: None,
        },
    )
}

fn born_count(card: &summary::CatchUpCard) -> String {
    card.headline.clone()
}

/// OST-012: a day or season span covers its first tick, so a row on the midnight (or season
/// boundary) belongs to the new span and not to the previous one.
#[test]
fn catch_up_spans_put_the_boundary_row_in_the_new_span() {
    use hf_sim::time::{DAYS_PER_SEASON, TICKS_PER_DAY};
    use summary::{catch_up_span_bounds as bounds, CatchUpSpan::*};
    let w = World::opening_world(3);
    let a = w.agents[0].id;
    let epoch = w.meta.epoch;
    let card = |from: i64, to: i64, rows: Vec<(u64, Severity, EventKind)>| {
        let (c, _) = chronicle_fixture(rows);
        summary::catch_up_card_between(&w, &c, from, to, 10).unwrap()
    };

    // Midnight: rows just before, on and just after it, in a world three days old.
    let mid = epoch + 3 * TICKS_PER_DAY;
    let now = mid + 100;
    let rows = || vec![born_at(a, mid - 1), born_at(a, mid), born_at(a, mid + 1)];
    let (f, t) = bounds(epoch, now, CurrentDay).unwrap();
    assert_eq!(
        born_count(&card(f, t, rows())),
        "born 2",
        "the midnight row and after"
    );
    let (f, t) = bounds(epoch, now, PreviousDay).unwrap();
    assert_eq!(
        born_count(&card(f, t, rows())),
        "born 1",
        "only the row before midnight"
    );

    // A season boundary, five days in: the same three rows.
    let sb = epoch + DAYS_PER_SEASON * TICKS_PER_DAY;
    let now = sb + 10;
    let rows = || vec![born_at(a, sb - 1), born_at(a, sb), born_at(a, sb + 1)];
    let (f, t) = bounds(epoch, now, CurrentSeason).unwrap();
    assert_eq!(born_count(&card(f, t, rows())), "born 2");
    let (f, t) = bounds(epoch, now, PreviousSeason).unwrap();
    assert_eq!(born_count(&card(f, t, rows())), "born 1");
    // The current day at that boundary is the new day too.
    let (f, t) = bounds(epoch, now, CurrentDay).unwrap();
    assert_eq!(born_count(&card(f, t, rows())), "born 2");

    // Tick zero: a row at tick zero is in the current day and season; there is no previous one.
    let (f, t) = bounds(0, 0, CurrentDay).unwrap();
    assert_eq!((f, t), (-1, 0));
    assert_eq!(born_count(&card(f, t, vec![born_at(a, 0)])), "born 1");
    let (f, t) = bounds(0, 0, CurrentSeason).unwrap();
    assert_eq!(born_count(&card(f, t, vec![born_at(a, 0)])), "born 1");
    assert_eq!(bounds(0, 0, PreviousDay), None);
    assert_eq!(bounds(0, 0, PreviousSeason), None);
    // Inside the first day and the first season there is still no previous span.
    assert_eq!(bounds(epoch, epoch + 5, PreviousDay), None);
    assert_eq!(
        bounds(epoch, epoch + 2 * TICKS_PER_DAY, PreviousSeason),
        None
    );
    // Day one's previous day is day zero.
    let (f, t) = bounds(epoch, epoch + TICKS_PER_DAY + 7, PreviousDay).unwrap();
    assert_eq!(
        (f, t),
        (epoch as i64 - 1, (epoch + TICKS_PER_DAY) as i64 - 1)
    );

    // Out of the chronicle too: a span with no previous day gives no card.
    let (c, _) = chronicle_fixture(vec![]);
    assert!(summary::catch_up_card_span(&w, &c, PreviousDay, 10)
        .unwrap()
        .is_none());
    assert!(summary::catch_up_card_span(&w, &c, CurrentDay, 10)
        .unwrap()
        .is_some());
    assert_eq!(
        summary::CatchUpSpan::parse("previous_day"),
        Some(PreviousDay)
    );
    assert_eq!(summary::CatchUpSpan::parse("last_week"), None);
}

/// OST-012: only a named change or a notable detail is news; counts of routine work are not.
#[test]
fn catch_up_has_news_only_for_a_named_change_or_a_notable_line() {
    let w = World::opening_world(3);
    let a = w.agents[0].id;
    let t0 = w.meta.epoch + 10;
    let hauled = |n: u64| {
        (
            t0 + n,
            Severity::Minor,
            EventKind::Hauled {
                agent: a,
                item: ItemId(1),
                to: TilePos { x: 0, y: 0, z: 0 },
                load: None,
            },
        )
    };
    let (c, _) = chronicle_fixture((0..20).map(hauled).collect());
    let quiet = summary::catch_up_card_between(&w, &c, t0 as i64 - 1, t0 as i64 + 50, 10).unwrap();
    assert!(!quiet.has_news, "hauling alone is not news");
    let (c, _) = chronicle_fixture(vec![born_at(a, t0 + 1)]);
    let busy = summary::catch_up_card_between(&w, &c, t0 as i64 - 1, t0 as i64 + 50, 10).unwrap();
    assert!(busy.has_news, "a birth is");
}
