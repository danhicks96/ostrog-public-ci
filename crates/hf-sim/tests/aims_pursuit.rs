//! OST-334: activity shares read hourly observations without changing people's choices.
use hf_sim::mind::Activity;
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::view::alive::{day_spread, AliveTally, DAY_SPREAD_MIN_HOURS};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

#[test]
fn day_spread_counts_hours_by_activity() {
    let mut world = World::generate(334, GenParams::small());
    let people = world.found_party(2);
    let mut tally = AliveTally::with_min_hours(24);
    for hour in 0..24 {
        world.meta.tick = hour * TICKS_PER_HOUR;
        for (person, sleep_hours) in people.iter().zip([6, 12]) {
            let a = world.agents.iter_mut().find(|a| a.id == *person).unwrap();
            a.activity = if hour < sleep_hours {
                Activity::Sleeping {
                    bed: None,
                    until: 0,
                }
            } else {
                Activity::Loitering { ticks_left: 1 }
            };
        }
        let before = hf_sim::digest::digest(&world);
        tally.observe_hour(&world);
        assert_eq!(before, hf_sim::digest::digest(&world));
    }
    let report = tally.report();
    assert_eq!(
        report.get("day_loitering_mean"),
        Some(62.5),
        "hourly activity shares must print"
    );
    assert_eq!(report.get("day_loitering_min"), Some(50.0));
    assert_eq!(report.get("day_loitering_max"), Some(75.0));
    assert_eq!(report.get("day_loitering_p90_p10"), Some(20.0));
    assert_eq!(report.get("day_sleeping_mean"), Some(37.5));
    assert_eq!(report.get("day_working_mean"), Some(0.0));
    assert_eq!(report.get("day_spread_n"), Some(2.0));
    assert_eq!(report.get("aim_lift_share"), Some(0.0));
    let hours: Vec<_> = tally.activity_hours().map(|(_, row)| row).collect();
    assert_eq!(hours[0][4], 6);
    assert_eq!(hours[1][4], 12);
    assert_eq!(day_spread(&hours, 24).activities[6].p90_p10, 20.0);
}

#[test]
fn day_spread_pools_people_and_excludes_short_lives() {
    let row = |loitering| {
        let mut hours = [0; 10];
        hours[4] = DAY_SPREAD_MIN_HOURS - loitering;
        hours[6] = loitering;
        hours
    };
    let mut short = row(0);
    short[4] -= 1;
    // Two runs with one and three people: every person has the same weight.
    let hours = [row(0), row(600), row(1200), row(2400), short, [0; 10]];
    let spread = day_spread(&hours, DAY_SPREAD_MIN_HOURS);
    assert_eq!(spread.people, 4);
    assert_eq!(spread.activities[6].mean, 43.75);
    assert!((spread.activities[6].p90_p10 - 77.5).abs() < 1e-9);
    assert_eq!(day_spread(&[], 0), Default::default());
    assert_eq!(day_spread(&[[0; 10]], 0), Default::default());
}

#[test]
fn day_spread_default_needs_one_hundred_observed_days() {
    let mut world = World::generate(334, GenParams::small());
    world.found_party(2);
    let mut tally = AliveTally::default();
    for hour in 0..DAY_SPREAD_MIN_HOURS {
        world.meta.tick = u64::from(hour) * TICKS_PER_HOUR;
        tally.observe_hour(&world);
        if hour == DAY_SPREAD_MIN_HOURS - 2 {
            assert_eq!(tally.report().get("day_spread_n"), Some(0.0));
        }
    }
    assert_eq!(tally.report().get("day_spread_n"), Some(2.0));
}
