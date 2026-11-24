//! OST-130: the "first days" graces count from the latest party to settle the valley, not from
//! the first portal, so a party arriving long after the first still gets its quiet first days.

use hf_sim::events::EventKind;
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::World;

/// An opened world four days in, with one inhabited place beyond the valley gone hungry.
fn hungry_world(seed: u64) -> World {
    let mut w = World::opening_world(seed);
    w.meta.autoplan = true;
    w.meta.engine = hf_sim::engine::Engine::Relentless;
    for _ in 0..4 * TICKS_PER_DAY {
        w.step();
    }
    let s = w
        .overworld
        .sites
        .iter_mut()
        .find(|s| s.people > 0)
        .expect("an inhabited place beyond the valley");
    s.food = -(s.people as f32) * 4.0;
    s.hungry_days = 3;
    w
}

fn band_sets_out_within(w: &mut World, days: u64) -> bool {
    for _ in 0..days * TICKS_PER_DAY {
        w.step();
        if w.last_events
            .iter()
            .any(|e| matches!(e.kind, EventKind::BandSetOut { .. }))
        {
            return true;
        }
    }
    false
}

#[test]
fn grace_counts_from_the_latest_party() {
    // The first party has been here four days: a hungry place sends people within six.
    let mut w = hungry_world(3);
    assert_eq!(w.since_party(), w.since_arrival(), "no later party yet");
    assert!(band_sets_out_within(&mut w, 6), "the control sends a band");

    // A new party has just arrived: nobody out there knows of the valley for four days.
    let mut w = hungry_world(3);
    let year = w.calendar().year;
    w.meta.arrived = w.meta.tick;
    assert_eq!(w.since_party(), 0);
    assert!(
        w.since_arrival() >= 4 * TICKS_PER_DAY,
        "the first portal is older"
    );
    assert!(
        !band_sets_out_within(&mut w, 3),
        "no band within the new party's first days"
    );
    assert_eq!(
        w.calendar().year,
        year,
        "years still count from the first portal"
    );
}
