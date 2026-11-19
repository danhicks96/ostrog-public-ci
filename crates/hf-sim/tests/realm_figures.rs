//! The named people of the world and the realms they rule (M3, C26).

#[path = "support/cpu.rs"]
mod cpu;

use std::collections::BTreeMap;

use hf_sim::realm::sites::{self, RealmEventKind};
use hf_sim::realm::{self, RealmParams};

fn world(
    seed: u64,
    years: i64,
) -> (
    hf_sim::realm::sites::Realm,
    Vec<hf_sim::realm::sites::RealmEvent>,
) {
    let m = realm::generate(seed, &RealmParams::default());
    sites::run(seed, &m, years)
}

#[test]
fn every_figure_begins_with_a_recorded_birth() {
    let (rl, events) = world(1, 400);
    let mut births: BTreeMap<u32, i64> = BTreeMap::new();
    for e in &events {
        if let RealmEventKind::Born { figure, site, .. } = &e.kind {
            assert!(births.insert(*figure, e.year).is_none(), "born twice");
            assert_eq!(rl.figures[*figure as usize].born_at, *site);
        }
    }
    assert_eq!(births.len(), rl.figures.len());
    for f in &rl.figures {
        assert_eq!(
            births[&f.id], f.born,
            "{} born in {} but recorded in {}",
            f.name, f.born, births[&f.id]
        );
        if let Some(p) = f.parent {
            let parent = &rl.figures[p as usize];
            assert!(
                parent.born + 18 <= f.born,
                "{}'s parent {} was a child",
                f.name,
                parent.name
            );
        }
    }
}

#[test]
fn every_death_has_a_year_and_a_cause() {
    let (rl, events) = world(2, 400);
    let mut deaths = 0;
    for f in rl.figures.iter().filter(|f| !f.alive()) {
        let (year, cause) = f.died.clone().unwrap();
        assert!(!cause.is_empty() && year >= f.born);
        assert!(events.iter().any(|e| e.year == year
            && matches!(&e.kind, RealmEventKind::Died { figure, cause: c } if *figure == f.id && *c == cause)));
        deaths += 1;
    }
    assert!(deaths > 100);
}

#[test]
fn rulers_live_at_their_seats_and_successors_are_grown() {
    let (rl, events) = world(3, 500);
    assert!(!rl.polities.is_empty(), "no realm rose in 500 years");
    for p in rl.polities.iter().filter(|p| p.ended.is_none()) {
        if let Some(f) = p.ruler {
            let fig = &rl.figures[f as usize];
            assert!(fig.alive(), "{} is ruled by the dead {}", p.name, fig.name);
            assert_eq!(
                fig.site, p.seat,
                "{} rules {} from elsewhere",
                fig.name, p.name
            );
        }
    }
    for e in &events {
        if let RealmEventKind::Succeeded { figure, .. } = e.kind {
            assert!(
                rl.figures[figure as usize].born + 16 <= e.year,
                "a child took a throne"
            );
        }
    }
}

#[test]
fn founders_were_grown_when_they_led() {
    let (rl, events) = world(4, 400);
    let mut led = 0;
    for e in &events {
        if let RealmEventKind::Led { figure, .. } = e.kind {
            let age = e.year - rl.figures[figure as usize].born;
            assert!((18..50).contains(&age), "a founder of {age}");
            led += 1;
        }
    }
    assert!(led > 10);
}

#[test]
fn a_thousand_years_bring_tens_of_thousands_of_figures() {
    // CPU time, not wall time: a busy machine is not a slow history (#582).
    let t = cpu::CpuTimer::start();
    let (rl, _) = world(1, 1000);
    assert!(rl.figures.len() >= 20_000, "{} figures", rl.figures.len());
    assert!(
        t.report("a thousand years with figures").as_secs_f32() < 30.0,
        "{:?} of CPU",
        t.elapsed()
    );
}

#[test]
fn the_realm_s_history_goes_into_the_chronicle_with_what_it_touched() {
    use hf_sim::chronicle::Chronicle;
    use hf_sim::events::Event;
    use hf_sim::ids::EventId;
    let (rl, events) = world(5, 200);
    let epoch = 300 * hf_sim::time::TICKS_PER_YEAR;
    let mut chron = Chronicle::in_memory().unwrap();
    let evs: Vec<Event> = hf_sim::realm::chronicle::to_events(&events, rl.year, epoch)
        .into_iter()
        .enumerate()
        .map(|(k, (tick, severity, kind))| Event {
            id: EventId(k as u32 + 1),
            tick,
            at: None,
            severity,
            kind,
        })
        .collect();
    // Older events come first, all before the arrival.
    assert!(evs.windows(2).all(|w| w[0].tick <= w[1].tick));
    assert!(evs.iter().all(|e| e.tick < epoch));
    let n = chron.record(&evs).unwrap();
    assert_eq!(n, evs.len());
    // Every reference resolves to something the realm holds.
    let mut st = chron
        .conn
        .prepare("SELECT entity_kind, entity_id FROM entity_ref")
        .unwrap();
    let rows: Vec<(String, i64)> = st
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    let mut kinds = std::collections::BTreeSet::new();
    for (kind, id) in &rows {
        let ok = match kind.as_str() {
            "figure" => (*id as usize) < rl.figures.len(),
            "site" => (*id as usize) < rl.sites.len(),
            "polity" => (*id as usize) < rl.polities.len(),
            "war" => (*id as usize) < rl.wars.len(),
            "culture" => (*id as usize) < rl.cultures.len(),
            "beast" => (*id as usize) < rl.beasts.len(),
            other => panic!("unexpected reference kind {other}"),
        };
        assert!(ok, "{kind} {id} does not exist");
        kinds.insert(kind.clone());
    }
    assert!(kinds.contains("figure") && kinds.contains("site"));
    // And each reads as a sentence naming real things.
    let w = {
        let mut w = hf_sim::World::generate(5, hf_sim::world::gen::GenParams::small());
        w.realm = rl.clone();
        w
    };
    let line = hf_sim::view::narrate::line(&w, &evs[0]);
    assert!(
        line.contains(&rl.sites[0].name) || line.contains(&rl.peoples[0].name),
        "{line}"
    );
}

#[test]
fn no_two_places_or_people_share_a_name() {
    let (rl, _) = world(1, 600);
    let mut seen = std::collections::BTreeSet::new();
    for n in rl
        .sites
        .iter()
        .map(|s| &s.name)
        .chain(rl.figures.iter().map(|f| &f.name))
    {
        assert!(seen.insert(n.clone()), "{n} is given twice");
    }
}
