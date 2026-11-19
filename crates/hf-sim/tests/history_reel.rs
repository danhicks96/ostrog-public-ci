//! #561: the reel at a new game's start replays the world's history before the portal as the
//! simulation recorded it: every notable row before the arrival, year by year in order, and
//! reading it changes nothing.

use hf_sim::chronicle::Chronicle;
use hf_sim::setup::WorldSetup;
use hf_sim::{digest, Session, World};

fn new_game(seed: u64, realm_years: u64) -> Session {
    let mut setup = WorldSetup::new(seed);
    setup.realm_years = realm_years;
    let w = World::from_setup(&setup);
    let mut s = Session::new(w, Chronicle::in_memory().unwrap());
    s.write_pending().unwrap();
    s
}

#[test]
fn the_reel_is_the_recorded_history_year_by_year() {
    let s = new_game(7, 200);
    let w = &s.world;
    let reel = hf_sim::view::history::reel(w, &s.chronicle).unwrap();
    assert!(!reel.is_empty(), "two hundred years have events");
    // Years strictly rising, each within the realm's years, each with a line.
    for pair in reel.windows(2) {
        assert!(
            pair[0].0 < pair[1].0,
            "years in order: {} then {}",
            pair[0].0,
            pair[1].0
        );
    }
    for (year, lines) in &reel {
        assert!(
            *year >= 0 && *year <= w.realm.year,
            "year {year} of 0..={}",
            w.realm.year
        );
        assert!(!lines.is_empty());
    }
    // Every notable row before the arrival is in it, once.
    let rows: i64 = s
        .chronicle
        .conn
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM real_history WHERE tick < ?1 AND severity >= ?2{}",
                hf_sim::view::known::rows_told_sql(w)
            ),
            rusqlite::params![
                w.meta.epoch as i64,
                hf_sim::events::Severity::Notable as i64
            ],
            |r| r.get(0),
        )
        .unwrap();
    let lines: usize = reel.iter().map(|(_, l)| l.len()).sum();
    assert_eq!(lines as i64, rows);
    // The world's founding comes in the first years shown, its latest years last.
    assert!(
        reel[0].0 < w.realm.year / 2,
        "first year shown {}",
        reel[0].0
    );
    assert!(reel.last().unwrap().0 > w.realm.year / 2);
}

#[test]
fn reading_the_reel_changes_nothing() {
    let mut a = new_game(11, 200);
    let mut b = new_game(11, 200);
    let _ = hf_sim::view::history::reel(&a.world, &a.chronicle).unwrap();
    for _ in 0..200 {
        a.step().unwrap();
        b.step().unwrap();
    }
    assert_eq!(digest::digest(&a.world), digest::digest(&b.world));
}
