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
