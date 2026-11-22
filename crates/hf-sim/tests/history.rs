//! Phase 11 gates. `chronicle_only_real_events`: every line a viewer can read is produced
//! from a recorded real_history row's own fields, taught history always joins to a real
//! row, and every custom's origin is a recorded event. `history_view_reads_the_record`:
//! eras, major lines with detail, and custom origins come out of the record.

use std::collections::BTreeMap;

use hf_sim::chronicle::Chronicle;
use hf_sim::events::Severity;
use hf_sim::ids::TilePos;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::view::{history, narrate};
use hf_sim::world::gen::GenParams;
use hf_sim::{Session, World};

fn meadow(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    for i in 0..120 {
        w.plants.push(Plant::new(
            TilePos::new(4 + (i * 7) % 56, 4 + (i * 11) % 56, 2),
            PlantKind::BerryBush,
            1.0,
            i as u32,
        ));
    }
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    w
}

#[test]
fn chronicle_only_real_events() {
    let mut w = meadow(1101);
    w.found_party(12);
    let mut s = Session::new(w, Chronicle::in_memory().unwrap());
    // What the viewer would have been shown, keyed by event id: the feed text at the time.
    let mut shown: BTreeMap<u32, (u64, String)> = BTreeMap::new();
    let mut emitted = 0i64;
    for _ in 0..TICKS_PER_DAY * 6 {
        s.step().unwrap();
        for e in &s.world.last_events {
            if e.severity != Severity::Trace {
                emitted += 1;
            }
            if e.severity >= Severity::Minor {
                shown.insert(e.id.0, (e.tick, narrate::line(&s.world, e)));
            }
        }
    }
    // 1. Every non-trace event emitted is a row; nothing else is.
    assert_eq!(
        s.chronicle.count().unwrap(),
        emitted,
        "rows == emitted events"
    );
    // 2. Every shown line re-narrates identically from the stored row alone.
    let mut checked = 0;
    for (eid, (tick, text)) in &shown {
        let (id, row_tick, detail) = s
            .chronicle
            .row(hf_sim::ids::EventId(*eid))
            .unwrap()
            .unwrap_or_else(|| panic!("shown event {eid} has a row"));
        assert_eq!(row_tick, *tick);
        let e = history::event_from_row(id, row_tick, &detail).expect("row decodes");
        assert_eq!(
            &narrate::line(&s.world, &e),
            text,
            "line from row equals shown line"
        );
        checked += 1;
    }
    assert!(checked > 100, "enough lines audited: {checked}");
    // 3. Taught history never stands alone.
    assert_eq!(s.chronicle.orphan_taught().unwrap(), 0);
    // 4. Every custom's origin is a recorded event, and the summary and history views
    //    only read rows.
    for cu in &s.world.customs {
        assert!(
            s.chronicle.row(cu.origin).unwrap().is_some(),
            "custom origin recorded"
        );
    }
    let major = history::major(&s.world, &s.chronicle, 50).unwrap();
    for (id, _, line, _) in &major {
        assert!(*id > 0);
        assert!(!line.is_empty());
    }
}

#[test]
fn judgement_narration_marks_only_wrongful_verdicts() {
    let mut w = meadow(1103);
    let ids = w.found_party(2);
    w.agents[0].name = "Judge".into();
    w.agents[1].name = "Accused".into();
    let event = |wrongly| hf_sim::events::Event {
        id: hf_sim::ids::EventId(1),
        tick: w.meta.tick,
        at: None,
        severity: Severity::Notable,
        kind: hf_sim::events::EventKind::Judged {
            judge: ids[0],
            of: ids[1],
            what: "breaking the bed".into(),
            verdict: "pardoned".into(),
            wrongly,
        },
    };

    for (wrongly, expected) in [
        (
            false,
            "Judge judged Accused for breaking the bed: pardoned.",
        ),
        (
            true,
            "Judge wrongly judged Accused for breaking the bed: pardoned.",
        ),
    ] {
        let event = event(wrongly);
        assert_eq!(narrate::line(&w, &event), expected, "feed");
        let stored = serde_json::to_string(&event.kind).unwrap();
        let from_history = history::event_from_row(1, event.tick, &stored).unwrap();
        assert_eq!(narrate::line(&w, &from_history), expected, "history");
    }
}

#[test]
fn history_view_reads_the_record() {
    let mut w = meadow(1102);
    w.found_party(12);
    let mut s = Session::new(w, Chronicle::in_memory().unwrap());
    // Through a year end so era rows are written, at 50× life speed for births and deaths.
    s.world.meta.life_speed = 50.0;
    let start = s.world.tick();
    let year_end = (start / hf_sim::time::TICKS_PER_YEAR + 1) * hf_sim::time::TICKS_PER_YEAR;
    while s.world.tick() < year_end + TICKS_PER_DAY {
        s.step().unwrap();
    }
    let eras = history::eras(&s.world, &s.chronicle).unwrap();
    assert!(
        eras.len() >= 2,
        "two eras: {:?}",
        eras.iter().map(|e| e.year).collect::<Vec<_>>()
    );
    let first = &eras[0];
    assert!(
        // Meals are not kept as history (act.rs); talk is.
        first.counts.iter().any(|(k, n)| k == "talked" && *n > 20),
        "first era has talk: {:?}",
        first.counts
    );
    // era_summary rows for the first year exist and equal the live counts.
    let stored: i64 = s
        .chronicle
        .conn
        .query_row("SELECT COUNT(*) FROM era_summary", [], |r| r.get(0))
        .unwrap();
    assert_eq!(stored as usize, first.counts.len(), "one era row per kind");
    let origins = history::custom_origins(&s.world, &s.chronicle).unwrap();
    for o in &origins {
        assert!(
            o.origin_line.is_some(),
            "custom '{}' has its origin line",
            o.label
        );
    }
    let major = history::major(&s.world, &s.chronicle, 20).unwrap();
    assert!(!major.is_empty());
}

/// Issues #88, #92: a world with 150 years behind it. The valley's past is counted back from
/// the portal and every year of it says what happened; the colony's first year is the HUD's
/// Year 1; record lines from before the portal are dated before it, not on day 1.
#[test]
fn the_past_is_dated_from_the_portal() {
    let w = hf_sim::World::opening_world(7);
    let epoch = w.meta.epoch;
    let mut s = Session::new(w, Chronicle::in_memory().unwrap());
    s.write_pending().unwrap();
    let eras = history::eras(&s.world, &s.chronicle).unwrap();
    assert!(
        eras.windows(2).all(|p| p[0].year < p[1].year),
        "oldest first"
    );
    let past: Vec<_> = eras.iter().filter(|e| e.year < 0).collect();
    assert!(
        past.len() > 10,
        "years of the valley's past: {}",
        past.len()
    );
    let old = [
        "old_founded",
        "old_trouble",
        "old_died",
        "old_ended",
        "old_answered",
        "lived",
    ];
    for e in &past {
        assert!(
            e.label.ends_with("before the portal") || e.label == "The year the portal opened",
            "a past year is dated from the portal: {}",
            e.label
        );
        // The valley's own past (deep.rs) or the wider world's (realm/, M3).
        assert!(
            e.counts
                .iter()
                .any(|(k, _)| old.contains(&k.as_str()) || k.starts_with("realm_")),
            "{} says what happened: {:?}",
            e.label,
            e.counts
        );
    }
    assert!(
        past.iter().any(|e| e.label.starts_with("14")),
        "150 years deep"
    );
    let colony: Vec<_> = eras.iter().filter(|e| e.year >= 0).collect();
    assert_eq!(colony.len(), 1, "one colony year so far");
    assert_eq!(colony[0].label, "Year 1", "the HUD's first year");
    assert!(colony[0].counts.iter().any(|(k, _)| k == "arrived"));

    // The record: latest first by when it happened, the past below the arrival and dated
    // before the portal.
    let major = history::major(&s.world, &s.chronicle, 400).unwrap();
    assert!(major.windows(2).all(|p| p[0].1 >= p[1].1), "latest first");
    let first_past = major.iter().position(|l| l.1 < epoch).expect("the past");
    assert!(major[..first_past]
        .iter()
        .any(|l| l.2.ends_with("arrived.")));
    for (_, tick, line, _) in &major {
        let when = history::when(&s.world, *tick);
        if *tick < epoch {
            assert!(!when.starts_with('Y'), "{when}  {line}");
            assert!(when.contains("portal"), "{when}  {line}");
        } else {
            assert!(when.starts_with("Y1 day 1 "), "{when}  {line}");
        }
    }
    assert_eq!(
        history::when(&s.world, epoch + 7 * TICKS_PER_HOUR),
        "Y1 day 1 07:00"
    );

    // What the unfound lost went through is in the record but not in the view (#93).
    let lost = hf_sim::view::known::unfound(&s.world);
    let shown: i64 = eras.iter().flat_map(|e| e.counts.iter()).map(|c| c.1).sum();
    let hidden: i64 = s
        .chronicle
        .conn
        .query_row(
            &format!(
                "SELECT COUNT(DISTINCT history_id) FROM entity_ref WHERE entity_kind = 'agent' AND entity_id IN ({})",
                lost.iter()
                    .map(|a| a.0.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(hidden > 0, "seed 7 has one hurt coming through");
    assert_eq!(shown, s.chronicle.count().unwrap() - hidden);
}

/// #203 (QA 2026-10-20): History opened on "501 years before the portal: settlements founded
/// 8" and a row per year of the wider world before the colony's own record. The colony's years
/// come first and newest first; the wider world is a line per century, naming its great
/// events, not a count per year.
#[test]
fn history_opens_on_the_colony_and_names_the_world() {
    let mut w = hf_sim::World::opening_world(7);
    w.meta.tick = w.meta.epoch + 2 * hf_sim::time::TICKS_PER_YEAR;
    let mut s = Session::new(w, Chronicle::in_memory().unwrap());
    s.write_pending().unwrap();
    let eras = history::eras(&s.world, &s.chronicle).unwrap();
    let world_years = eras
        .iter()
        .filter(|e| e.counts.iter().any(|(k, _)| k.starts_with("realm_")))
        .count();
    let o = history::overview(&s.world, &s.chronicle).unwrap();
    assert!(!o.colony.is_empty(), "the colony's own year");
    assert!(
        o.colony.windows(2).all(|p| p[0].year > p[1].year),
        "newest first"
    );
    assert!(
        o.valley.windows(2).all(|p| p[0].year > p[1].year),
        "newest first"
    );
    assert!(o
        .valley
        .iter()
        .all(|e| e.counts.iter().all(|(k, _)| !k.starts_with("realm_"))));
    assert!(
        world_years > 20,
        "seed 7 has a wider world: {world_years} years"
    );
    let span = eras.first().map(|e| -e.year).unwrap_or(0);
    assert!(
        o.world.len() as i64 <= span / 100 + 1,
        "a line a century: {} lines for {span} years",
        o.world.len()
    );
    assert!(
        o.world.windows(2).all(|p| p[0].year > p[1].year),
        "newest first"
    );
    let named: usize = o.world.iter().map(|c| c.named.len()).sum();
    assert!(named > 0, "the world's great events by name");
    for c in &o.world {
        assert!(c.label.ends_with("years before the portal"), "{}", c.label);
        assert!(c.named.len() <= history::NAMED);
        for l in &c.named {
            assert!(!l.is_empty() && !l.contains("realm_"), "{l}");
        }
        let words = history::summary(&c.counts);
        assert!(!words.contains('_'), "{words}");
    }
}

/// OST-1000: one colony year's record, oldest first, folded, paged.
#[test]
fn year_record_pages_one_colony_year() {
    use hf_sim::events::{Event, EventKind};
    use hf_sim::ids::{AgentId, EventId};
    use hf_sim::time::TICKS_PER_YEAR;
    let w = hf_sim::World::opening_world(7);
    let epoch = w.meta.epoch;
    let mut c = Chronicle::in_memory().unwrap();
    let mut id = 0u32;
    let mut evs = Vec::new();
    let mut push = |tick: u64, kind: EventKind| {
        id += 1;
        evs.push(Event {
            id: EventId(id),
            tick,
            at: None,
            severity: Severity::Notable,
            kind,
        });
    };
    let died = |n: u32| EventKind::Died {
        agent: AgentId(1000 + n),
        cause: "age".into(),
    };
    for y in [0u64, 1, 2] {
        push(epoch + y * TICKS_PER_YEAR + 10, died(y as u32));
    }
    // Year 1: three more deaths and twenty suspicions of one sleeper (one line when folded).
    for n in 0..3u32 {
        push(epoch + TICKS_PER_YEAR + 100 + n as u64 * 10, died(10 + n));
    }
    for n in 0..20u64 {
        push(
            epoch + TICKS_PER_YEAR + 50 + n,
            EventKind::Suspected {
                by: AgentId(2000 + n as u32),
                of: AgentId(1),
                because: "x".into(),
            },
        );
    }
    c.record(&evs).unwrap();
    let (all, total) = history::year_record(&w, &c, 1, 0, 100).unwrap();
    assert_eq!(total, 5, "1 + 3 deaths and one folded suspicion line");
    assert_eq!(all.len(), 5);
    assert!(all.windows(2).all(|p| p[0].first_tick <= p[1].first_tick));
    assert!(all.iter().all(
        |f| f.first_tick >= epoch + TICKS_PER_YEAR && f.last_tick < epoch + 2 * TICKS_PER_YEAR
    ));
    assert_eq!(all.iter().filter(|f| f.count == 20).count(), 1, "folded");
    let (page, t2) = history::year_record(&w, &c, 1, 1, 2).unwrap();
    assert_eq!((page.len(), t2), (2, 5));
    assert_eq!(page[0], all[1]);
    assert_eq!(history::year_record(&w, &c, 1, 4, 10).unwrap().0.len(), 1);
    assert_eq!(history::year_record(&w, &c, 0, 0, 10).unwrap().1, 1);
    // Past the end of the year: no lines, the total unchanged; a negative year is empty.
    assert_eq!(
        history::year_record(&w, &c, 1, 5, 10).unwrap(),
        (Vec::new(), 5)
    );
    assert_eq!(
        history::year_record(&w, &c, -1, 0, 10).unwrap(),
        (Vec::new(), 0)
    );
}
