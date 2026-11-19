//! The world goes on after the arrival, and the valley hears of it only from travellers
//! (M3, C29).

use hf_sim::events::EventKind;
use hf_sim::time::{TICKS_PER_HOUR, TICKS_PER_YEAR};
use hf_sim::view::known;

fn years_after(w: &mut hf_sim::World, years: u64) {
    let epoch = w.meta.epoch;
    let start = (w.meta.tick - epoch) / TICKS_PER_YEAR + 1;
    for k in start..start + years {
        w.meta.tick = epoch + k * TICKS_PER_YEAR + 6 * TICKS_PER_HOUR;
        hf_sim::realm::live::step(w);
    }
}

#[test]
fn the_world_goes_on_after_the_arrival_and_nobody_here_is_told() {
    let mut w = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(1));
    let arrived = w.realm.year;
    assert_eq!(w.realm.arrived, arrived);
    w.events.pending.clear();
    years_after(&mut w, 10);
    assert_eq!(w.realm.year, arrived + 10);
    let realm_events: Vec<_> = w
        .events
        .pending
        .iter()
        .filter(|e| matches!(e.kind, EventKind::Realm { .. }))
        .collect();
    assert!(
        realm_events.len() > 50,
        "{} events in ten years",
        realm_events.len()
    );
    for e in &realm_events {
        assert!(e.tick > w.meta.epoch);
        assert!(
            !known::told(&w, e),
            "the valley is told of something far off"
        );
    }
    // The neighbours are what their settlements have become.
    for s in w.overworld.sites.iter().filter(|s| s.realm_site.is_some()) {
        let rs = &w.realm.sites[s.realm_site.unwrap() as usize];
        if !rs.alive() {
            assert_eq!(s.people, 0);
        } else if s.kind == hf_sim::expedition::SiteKind::Village {
            assert_eq!(s.people, (rs.population as u32).clamp(8, 200));
        }
    }
}

/// #1322: the realm should hold who arrived, not who set out.
#[test]
fn a_valley_hamlet_counts_who_arrived() {
    let mut w = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(2));
    w.events.pending.clear();
    let species = w.agents.iter().find(|a| a.alive).unwrap().people;
    let before_sites = w.realm.sites.len();

    // A departure of 4, only 2 of whom reach the edge.
    let (pending, name) =
        hf_sim::realm::live::reserve_settle_from_valley(&mut w, species).expect("a destination");
    assert_eq!(
        w.realm.sites.len(),
        before_sites,
        "named and spotted, not founded, before anyone arrives"
    );
    hf_sim::realm::live::settle_arrived(&mut w, pending);
    hf_sim::realm::live::settle_arrived(&mut w, pending);
    let site = w
        .realm
        .sites
        .iter()
        .find(|s| s.name == name)
        .expect("founded on the first arrival");
    assert_eq!(site.population as u32, 2, "the realm holds who arrived");
    let founded = site.id;
    assert!(
        w.events.pending.iter().any(
            |e| matches!(&e.kind, EventKind::Realm { what } if matches!(what, hf_sim::realm::sites::RealmEventKind::Founded { site: s, parent: None } if *s == founded))
        ),
        "one Founded row, on the first arrival"
    );
    // Not the people's first home: this people's homeland site was founded already.
    let line = hf_sim::view::narrate::line(
        &w,
        &hf_sim::events::Event {
            id: hf_sim::ids::EventId(0),
            tick: w.meta.tick,
            at: None,
            severity: hf_sim::events::Severity::Notable,
            kind: EventKind::Realm {
                what: hf_sim::realm::sites::RealmEventKind::Founded {
                    site: founded,
                    parent: None,
                },
            },
        },
    );
    assert!(!line.contains("first home"), "{line}");
    assert!(line.contains("founded"), "{line}");

    // A second departure of 3, none of whom reach the edge: nothing is founded.
    w.events.pending.clear();
    let (pending2, _) = hf_sim::realm::live::reserve_settle_from_valley(&mut w, species)
        .expect("a second destination");
    let _ = pending2;
    assert_eq!(
        w.realm.sites.len(),
        before_sites + 1,
        "no site for a departure nobody survived"
    );
    assert!(
        !w.events
            .pending
            .iter()
            .any(|e| matches!(&e.kind, EventKind::Realm { what } if matches!(what, hf_sim::realm::sites::RealmEventKind::Founded { .. }))),
        "no Founded row for a departure nobody survived"
    );
}

#[test]
fn news_comes_with_someone_back_from_out_there() {
    let mut w = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(1));
    years_after(&mut w, 25);
    w.events.pending.clear();
    let teller = w
        .agents
        .iter()
        .find(|a| a.alive && a.community.is_some())
        .unwrap()
        .id;
    let mut heard = 0;
    for s in w
        .overworld
        .sites
        .clone()
        .iter()
        .filter(|s| s.realm_site.is_some())
    {
        hf_sim::realm::live::news(&mut w, s.id, teller);
        let news: Vec<_> = w
            .events
            .pending
            .iter()
            .filter(|e| matches!(e.kind, EventKind::NewsHeard { .. }))
            .cloned()
            .collect();
        for e in &news {
            assert!(known::told(&w, e), "news is told");
            let line = hf_sim::view::narrate::line(&w, e);
            assert!(line.contains(&s.name), "{line}");
            assert!(
                !line.contains("someone") && !line.contains("a place"),
                "{line}"
            );
        }
        heard += news.len();
        w.events.pending.clear();
        // Asked again at once, there is nothing new to tell.
        hf_sim::realm::live::news(&mut w, s.id, teller);
        assert!(!w
            .events
            .pending
            .iter()
            .any(|e| matches!(e.kind, EventKind::NewsHeard { .. })));
    }
    assert!(
        heard >= 1,
        "twenty-five years and no news near any neighbour"
    );
}
