//! #1453: the biography keeps the rows that make a life and drops the routine.

use hf_sim::chronicle::Chronicle;
use hf_sim::events::{Event, EventKind, Severity};
use hf_sim::ids::EventId;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

#[test]
fn biography_keeps_life_events_drops_routine() {
    let mut w = World::generate(5, GenParams::small());
    let ids = w.found_party(2);
    let (a, b) = (ids[0], ids[1]);
    let mut c = Chronicle::in_memory().unwrap();
    let mut n = 0u32;
    let mut ev = |tick: u64, severity: Severity, kind: EventKind| {
        n += 1;
        Event {
            id: EventId(n),
            tick,
            at: None,
            severity,
            kind,
        }
    };
    let mut evs = vec![ev(100, Severity::Notable, EventKind::Paired { a, b })];
    for i in 0..50 {
        evs.push(ev(
            110 + i,
            Severity::Minor,
            EventKind::Talked { a, b, warmth: 0.5 },
        ));
    }
    evs.push(ev(
        200,
        Severity::Notable,
        EventKind::ItemNamed {
            agent: a,
            name: "Dawnmantle".into(),
            what: "cloak".into(),
            because: "fey work".into(),
        },
    ));
    for t in [300, 310] {
        evs.push(ev(
            t,
            Severity::Minor,
            EventKind::RoleTaken {
                agent: a,
                role: "smith".into(),
            },
        ));
    }
    evs.push(ev(
        400,
        Severity::Major,
        EventKind::Died {
            agent: a,
            cause: "old age".into(),
        },
    ));
    c.record(&evs).unwrap();

    let bio = hf_sim::view::biography::biography(&w, &c, a).unwrap();
    let ticks: Vec<u64> = bio.iter().map(|l| l.tick).collect();
    assert_eq!(ticks, vec![100, 200, 300, 400], "{bio:#?}");
    for l in &bio {
        let n: i64 = c
            .conn
            .query_row(
                "SELECT COUNT(*) FROM real_history WHERE id = ?1",
                [l.history_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1, "line {l:?} must be a recorded row");
    }
}
