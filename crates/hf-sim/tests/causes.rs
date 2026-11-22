//! From an event of the record back through its recorded causes.

use hf_sim::chronicle::Chronicle;
use hf_sim::events::{Event, EventKind, Severity};
use hf_sim::ids::{EventId, TilePos};
use hf_sim::view::{causes, legends};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

#[test]
fn a_festered_wound_traces_back_through_the_bite_to_the_vermin() {
    let mut w = World::generate(2925, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(1);
    let at = TilePos::new(30, 31, 2);
    let t0 = w.meta.epoch + 100;
    let ev = |id: u32, tick: u64, kind: EventKind| Event {
        id: EventId(id),
        tick,
        at: Some(at),
        severity: Severity::Notable,
        kind,
    };
    let mut c = Chronicle::in_memory().unwrap();
    c.record(&[
        ev(1, t0, EventKind::WorldCreated { seed: 1 }),
        ev(
            2,
            t0 + 10,
            EventKind::VerminDrawn {
                at,
                cause: EventId(1),
            },
        ),
        ev(
            3,
            t0 + 20,
            EventKind::VerminBite {
                agent: ids[0],
                at,
                cause: EventId(2),
            },
        ),
        ev(
            4,
            t0 + 30,
            EventKind::Festered {
                agent: ids[0],
                cause: EventId(3),
            },
        ),
    ])
    .unwrap();
    let festered = c.history_id(EventId(4)).unwrap().unwrap();
    assert!(causes::has_cause(&c, festered));
    let root = c.history_id(EventId(1)).unwrap().unwrap();
    assert!(!causes::has_cause(&c, root), "a root event names no cause");
    let (links, whole) = causes::chain(&w, &c, festered, 6);
    assert_eq!(links.len(), 4, "{links:?}");
    assert_eq!(whole, causes::End::Root);
    // The page, through Legends: the event, then each cause in turn, oldest last.
    let page = legends::page(&w, &c, &format!("why:{festered}"));
    assert!(page.starts_with("[b]Why it happened[/b]"), "{page}");
    assert_eq!(page.matches("  because, ").count(), 3, "{page}");
    // A chain is bounded.
    let (short, end) = causes::chain(&w, &c, festered, 1);
    assert_eq!(short.len(), 2);
    assert_eq!(end, causes::End::Limit, "a cut chain is not called whole");
    // An event with no recorded cause says so.
    let lone = legends::page(&w, &c, &format!("why:{root}"));
    assert!(lone.contains("The record names no cause for it."), "{lone}");
}

#[test]
fn a_cause_missing_from_the_record_is_not_called_no_cause() {
    let mut w = World::generate(2925, GenParams::small());
    w.rebuild_caches();
    let at = TilePos::new(30, 31, 2);
    let t0 = w.meta.epoch + 100;
    let mut c = Chronicle::in_memory().unwrap();
    // The vermin event names cause 99, which was never recorded.
    c.record(&[Event {
        id: EventId(2),
        tick: t0,
        at: Some(at),
        severity: Severity::Notable,
        kind: EventKind::VerminDrawn {
            at,
            cause: EventId(99),
        },
    }])
    .unwrap();
    let row = c.history_id(EventId(2)).unwrap().unwrap();
    let (links, end) = causes::chain(&w, &c, row, 6);
    assert_eq!((links.len(), end), (1, causes::End::Missing));
    let page = legends::page(&w, &c, &format!("why:{row}"));
    assert!(!page.contains("names no cause"), "{page}");
    assert!(page.contains("is missing"), "{page}");
    // A row that does not exist at all.
    assert!(legends::page(&w, &c, "why:12345").contains("Nothing is recorded here."));
}

#[test]
fn a_cause_about_an_unfound_person_ends_the_chain_without_naming_them() {
    let w = World::opening_world(1);
    let lost = hf_sim::view::known::unfound(&w)[0];
    let seen = w
        .agents
        .iter()
        .find(|a| {
            a.species == hf_sim::agents::Species::Person
                && !hf_sim::view::known::is_unfound(&w, a.id)
        })
        .unwrap()
        .id;
    let at = TilePos::new(30, 31, 2);
    let t0 = w.meta.tick;
    let ev = |id: u32, tick: u64, kind: EventKind| Event {
        id: EventId(id),
        tick,
        at: Some(at),
        severity: Severity::Notable,
        kind,
    };
    let mut c = Chronicle::in_memory().unwrap();
    c.record(&[
        ev(
            2,
            t0,
            EventKind::VerminBite {
                agent: lost,
                at,
                cause: EventId(1),
            },
        ),
        ev(
            3,
            t0 + 5,
            EventKind::Festered {
                agent: seen,
                cause: EventId(2),
            },
        ),
    ])
    .unwrap();
    let row = c.history_id(EventId(3)).unwrap().unwrap();
    let (links, end) = causes::chain(&w, &c, row, 6);
    assert_eq!((links.len(), end), (1, causes::End::Untold), "{links:?}");
    let lost_name = &w.agents.iter().find(|a| a.id == lost).unwrap().name;
    let page = legends::page(&w, &c, &format!("why:{row}"));
    assert!(page.contains("nothing the valley knows"), "{page}");
    assert!(!page.contains(lost_name.as_str()), "{page}");
}
