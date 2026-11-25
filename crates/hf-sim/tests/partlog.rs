//! Part histories (Body & Gear slice 0): one identity per part, a lost finger takes only its own
//! positions, an event leads back to its parts, and an old save loads with no history.
use hf_sim::ids::EventId;
use hf_sim::partlog::{Change, Entry, PartKey, PartLog, Region, Side, Spot, KEPT};

const L: Side = Side::Left;
const R: Side = Side::Right;

fn entry(part: PartKey, tick: u64, change: Change, ev: Option<u64>) -> Entry {
    Entry {
        part,
        tick,
        change,
        event: ev.map(|n| EventId(n as u32)),
        note: String::new(),
    }
}

#[test]
fn losing_a_left_ring_finger_leaves_the_right_ring_finger_and_the_hand() {
    let ring_l = PartKey::new(L, Region::Ring, Spot::Whole);
    let mut log = PartLog::default();
    log.record(entry(ring_l, 10, Change::Lost, Some(7)));
    assert!(
        log.missing(PartKey::new(L, Region::Ring, Spot::Base)),
        "its base goes with it"
    );
    assert!(log.missing(PartKey::new(L, Region::Ring, Spot::NailPlate)));
    assert!(!log.missing(PartKey::new(R, Region::Ring, Spot::Base)));
    assert!(!log.missing(PartKey::new(L, Region::Middle, Spot::Base)));
    assert!(!log.missing(PartKey::new(L, Region::Hand, Spot::Whole)));
}

#[test]
fn a_lost_fingertip_takes_the_nail_but_not_the_base() {
    let mut log = PartLog::default();
    log.record(entry(
        PartKey::new(L, Region::Ring, Spot::Tip),
        5,
        Change::Lost,
        Some(1),
    ));
    assert!(log.missing(PartKey::new(L, Region::Ring, Spot::NailPlate)));
    assert!(!log.missing(PartKey::new(L, Region::Ring, Spot::Base)));
}

#[test]
fn a_replacement_makes_the_part_present_again_and_history_stays() {
    let p = PartKey::new(L, Region::Hand, Spot::Whole);
    let mut log = PartLog::default();
    log.record(entry(p, 1, Change::Lost, Some(1)));
    log.record(entry(p, 9, Change::Replaced, Some(2)));
    assert!(!log.missing(p));
    assert_eq!(log.history(p, false).len(), 2);
}

#[test]
fn an_event_leads_back_to_every_part_it_touched_without_copies() {
    let mut log = PartLog::default();
    log.record(entry(
        PartKey::new(L, Region::Index, Spot::Base),
        3,
        Change::Wounded,
        Some(4),
    ));
    log.record(entry(
        PartKey::new(L, Region::Middle, Spot::Base),
        3,
        Change::Wounded,
        Some(4),
    ));
    assert_eq!(log.for_event(EventId(4)).len(), 2);
    assert!(log.for_event(EventId(5)).is_empty());
}

#[test]
fn history_below_a_hand_includes_its_digits() {
    let mut log = PartLog::default();
    log.record(entry(
        PartKey::new(L, Region::Thumb, Spot::NailPlate),
        2,
        Change::Wounded,
        None,
    ));
    let hand = PartKey::new(L, Region::Hand, Spot::Whole);
    assert_eq!(log.history(hand, true).len(), 1);
    assert!(log.history(hand, false).is_empty());
}

#[test]
fn the_log_is_bounded_and_keeps_significant_history_over_ordinary_marks() {
    let p = PartKey::new(L, Region::Palm, Spot::Whole);
    let mut log = PartLog::default();
    log.record(entry(p, 0, Change::Scarred, Some(1)));
    for t in 1..(KEPT as u64 + 20) {
        log.record(entry(p, t, Change::Marked, None));
    }
    let h = log.history(p, false);
    assert_eq!(h.len(), KEPT);
    assert_eq!(h[0].change, Change::Scarred, "the scar is kept");
}

#[test]
fn an_empty_log_is_empty_and_roundtrips() {
    let log = PartLog::default();
    assert!(log.is_empty());
    let s = serde_json::to_string(&log).unwrap();
    assert_eq!(serde_json::from_str::<PartLog>(&s).unwrap(), log);
}
