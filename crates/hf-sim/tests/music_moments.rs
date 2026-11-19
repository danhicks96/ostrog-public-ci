//! #290 item 2 (WP-08 slice 8.8): the music answers more than strain. A person's death is
//! mourned; an animal's is not. A birth, a band welcomed or a caravan home is welcomed.

use hf_sim::agents::Species;
use hf_sim::events::{Event, EventKind, Severity};
use hf_sim::ids::{EventId, TilePos};
use hf_sim::view::music::moment_of;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn ev(kind: EventKind) -> Event {
    Event {
        id: EventId(0),
        tick: 0,
        severity: Severity::Major,
        at: Some(TilePos::new(30, 30, 2)),
        kind,
    }
}

#[test]
fn a_persons_death_is_mourned_and_an_animals_is_not() {
    let mut w = World::generate(2921, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(2);
    let died = |agent| {
        ev(EventKind::Died {
            agent,
            cause: "of the cold".into(),
        })
    };
    assert_eq!(moment_of(&w, &died(ids[0])), Some("mourn"));
    // The same death, were they an animal, is no moment of the score.
    w.agents
        .iter_mut()
        .find(|a| a.id == ids[0])
        .unwrap()
        .species = Species::Deer;
    assert_eq!(moment_of(&w, &died(ids[0])), None);
    assert_eq!(
        moment_of(
            &w,
            &ev(EventKind::Born {
                agent: ids[1],
                mother: ids[0],
                father: None
            })
        ),
        Some("welcome")
    );
    assert_eq!(
        moment_of(
            &w,
            &ev(EventKind::Welcomed {
                agent: ids[1],
                band: 0
            })
        ),
        Some("welcome")
    );
    assert_eq!(
        moment_of(
            &w,
            &ev(EventKind::Arrived {
                agent: ids[1],
                community: None
            })
        ),
        None
    );
}
