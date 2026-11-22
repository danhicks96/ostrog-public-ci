//! OST-004: the camera-miss measurement. Which Major events that name a person did the camera
//! never show, by kind, read from `AliveTally::observe_major_events` and `observe_glance` and
//! driven headlessly by `view::camera_miss::measure`.

use hf_sim::agents::Species;
use hf_sim::digest::digest;
use hf_sim::events::{Event, EventKind, Severity};
use hf_sim::ids::{AgentId, EventId, TilePos};
use hf_sim::view::alive::AliveTally;
use hf_sim::view::camera_miss::{measure, SAMPLE_STRIDE};
use hf_sim::view::director::{self, Director};
use hf_sim::World;

fn died(tick: u64, at: Option<TilePos>, severity: Severity) -> Event {
    Event {
        id: EventId(0),
        tick,
        at,
        severity,
        kind: EventKind::Died {
            agent: AgentId(1),
            cause: "fever".into(),
        },
    }
}

fn counts(t: &AliveTally, kind: &str) -> (u32, u32) {
    let c = t.glance_major_counts();
    let row = c.iter().find(|r| r.0 == kind).unwrap();
    (row.1, row.2)
}

/// The staged world: the director held on the settlement (no people, no events fed to it, so its
/// dwell holds), sampled every 100 ticks for one game hour and a little over.
fn sampled(event_at: TilePos) -> AliveTally {
    let w = World::opening_world(3);
    let settlement = director::settlement_centre(&w);
    let mut tally = AliveTally::default();
    let mut dir = Director::default();
    tally.observe_major_events(&[died(
        10_000,
        Some(settlement.offset(event_at.x, event_at.y, event_at.z)),
        Severity::Major,
    )]);
    for t in (10_000..=11_300).step_by(100) {
        let s = dir.suggest(t, settlement, &[]);
        assert_eq!(
            s.at, settlement,
            "t={t}: the director stays on the settlement"
        );
        tally.observe_glance(t, s.at);
    }
    tally
}

#[test]
fn a_death_far_from_the_frame_counts_as_missed() {
    let far = sampled(TilePos::new(40, 0, 0));
    assert_eq!(
        counts(&far, "died"),
        (1, 1),
        "a death 40 tiles off is missed"
    );
    let r = far.report();
    assert_eq!(r.get("glance_missed_major"), Some(1.0));
    assert_eq!(r.get("glance_missed_major_died"), Some(1.0));
    assert_eq!(r.get("glance_major_n"), Some(1.0));
}

#[test]
fn a_death_inside_the_frame_counts_as_shown() {
    let near = sampled(TilePos::new(3, 0, 0));
    assert_eq!(counts(&near, "died"), (0, 1));
    assert_eq!(near.report().get("glance_missed_major"), Some(0.0));
}

/// The timing guard. A sample taken after the hour is up can no longer show the event, even when
/// the camera is exactly on it; a sample at exactly the hour still can.
#[test]
fn a_sample_after_the_hour_does_not_show_the_event() {
    let at = TilePos::new(50, 50, 0);
    let elsewhere = TilePos::new(500, 500, 0);

    // First look on the event arrives 1,201 ticks late: missed, not shown.
    let mut late = AliveTally::default();
    late.observe_major_events(&[died(10_000, Some(at), Severity::Major)]);
    late.observe_glance(10_600, elsewhere);
    late.observe_glance(11_201, at);
    assert_eq!(
        counts(&late, "died"),
        (1, 1),
        "a sample past the deadline shows nothing"
    );

    // The same look exactly at the deadline counts.
    let mut on_time = AliveTally::default();
    on_time.observe_major_events(&[died(10_000, Some(at), Severity::Major)]);
    on_time.observe_glance(10_600, elsewhere);
    on_time.observe_glance(11_200, at);
    assert_eq!(
        on_time.glance_major_pending(),
        1,
        "still inside its hour at 1,200"
    );
    on_time.observe_glance(11_201, elsewhere);
    assert_eq!(
        counts(&on_time, "died"),
        (0, 1),
        "a sample at the deadline shows it"
    );
}

#[test]
fn only_major_events_that_name_a_person_with_a_place_on_the_level_can_be_shown() {
    let at = TilePos::new(5, 5, 0);
    let mut t = AliveTally::default();
    t.observe_major_events(&[
        died(100, None, Severity::Major),     // no place: always missed
        died(100, Some(at), Severity::Minor), // not Major: ignored
        died(100, Some(TilePos::new(5, 5, 3)), Severity::Major), // another level
        died(100, Some(at), Severity::Major), // on the camera: shown
        Event {
            id: EventId(1),
            tick: 100,
            at: Some(at),
            severity: Severity::Major,
            kind: EventKind::Doused {
                agent: AgentId(1),
                at,
            },
        }, // not a named kind: ignored
    ]);
    assert_eq!(t.glance_major_pending(), 3);
    t.observe_glance(200, at);
    t.observe_glance(1_301, at);
    assert_eq!(
        counts(&t, "died"),
        (2, 3),
        "no place and another level are missed"
    );
    assert_eq!(t.glance_major_pending(), 0);
}

/// An empty report keeps every key at 0.0 and a kind with nothing watched reads 0.0, not NaN.
#[test]
fn nothing_settled_reads_zero() {
    let r = AliveTally::default().report();
    for k in [
        "glance_major_n",
        "glance_missed_major",
        "glance_missed_major_born",
    ] {
        assert_eq!(r.get(k), Some(0.0), "{k}");
    }
}

/// The headless caller: a person's death injected into a running world is watched, settles
/// after its hour, and is counted by kind. Stepping through `measure` leaves the world exactly
/// where plain stepping does.
#[test]
fn the_headless_caller_counts_a_death_and_changes_nothing() {
    const TICKS: u64 = 2 * SAMPLE_STRIDE + 1_800;
    let stage = || {
        let mut w = World::opening_world(3);
        let person = w
            .agents
            .iter()
            .find(|a| a.alive && a.species == Species::Person)
            .map(|a| a.id)
            .expect("the opening world has people");
        let tick = w.meta.tick;
        let at = director::settlement_centre(&w).offset(40, 0, 0);
        w.events.push(
            tick,
            Some(at),
            Severity::Major,
            EventKind::Died {
                agent: person,
                cause: "fever".into(),
            },
        );
        w
    };

    let mut measured = stage();
    let tally = measure(&mut measured, TICKS, SAMPLE_STRIDE);
    let mut plain = stage();
    plain.run(TICKS);
    assert_eq!(
        digest(&measured),
        digest(&plain),
        "measuring does not change the simulation"
    );

    let (missed, all) = counts(&tally, "died");
    assert!(
        all >= 1,
        "the injected death settled into the counts: {all}"
    );
    assert!(missed <= all);
    let r = tally.report();
    assert_eq!(
        r.get("glance_major_n"),
        Some(f64::from(
            tally.glance_major_counts().iter().map(|c| c.2).sum::<u32>()
        ))
    );
}
