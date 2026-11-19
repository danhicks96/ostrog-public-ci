//! A saved game resumed mid-hour goes on exactly as the game it was saved from (the review of
//! C38-C44: state the save left out, the light list below ground, made a resumed game see
//! differently for up to an hour).

use hf_sim::save::{decode, encode, snapshot::Snapshot};
use hf_sim::time::TICKS_PER_HOUR;

#[test]
fn a_game_resumed_mid_hour_goes_on_the_same() {
    let mut a = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(11));
    a.meta.autoplan = true;
    for _ in 0..(3 * TICKS_PER_HOUR + TICKS_PER_HOUR / 2) {
        a.step();
    }
    let mut b = decode(&encode(&Snapshot::capture(&a)).unwrap())
        .unwrap()
        .restore();
    for _ in 0..(2 * TICKS_PER_HOUR) {
        a.step();
        b.step();
    }
    assert_eq!(hf_sim::digest::digest(&a), hf_sim::digest::digest(&b));
}

#[test]
fn the_lights_below_ground_are_kept_in_the_save() {
    let mut w = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(12));
    w.lights = vec![(hf_sim::ids::TilePos::new(20, 20, -2), 4.5, true)];
    let back = decode(&encode(&Snapshot::capture(&w)).unwrap())
        .unwrap()
        .restore();
    assert_eq!(back.lights, w.lights);
}

/// #557: an opening saved to disk at day 10 (the soak's own save, chronicle and all) and
/// resumed is the same world at day 30 as the game run on without stopping. The short test
/// above catches state left out of the save within hours; this one, what drifts over weeks.
/// (Day 30 to 90 took eight minutes a seed; the soaks' resume check, tools/soak/run4.sh,
/// covers the long runs.)
fn resumed_at_day_10_equals_continuous_at_day_30(seed: u64) {
    use hf_sim::time::TICKS_PER_DAY;
    let world = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(seed));
    let mut a = hf_sim::Session::new(world, hf_sim::chronicle::Chronicle::working().unwrap());
    a.write_pending().unwrap();
    a.run(10 * TICKS_PER_DAY).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let slot = dir.path().join("day10.hfsave");
    hf_sim::save::write(&a.world, &mut a.chronicle, &slot).unwrap();
    let (world, chronicle) = hf_sim::save::read(&slot).unwrap();
    let mut b = hf_sim::Session::new(world, chronicle);
    assert_eq!(
        hf_sim::digest::digest(&a.world),
        hf_sim::digest::digest(&b.world),
        "seed {seed}: the save reads back as it was written"
    );
    a.run(20 * TICKS_PER_DAY).unwrap();
    b.run(20 * TICKS_PER_DAY).unwrap();
    assert_eq!(
        hf_sim::digest::digest(&a.world),
        hf_sim::digest::digest(&b.world),
        "seed {seed}: resumed at day 10, the game differs at day 30"
    );
}

#[test]
fn seed_1_resumed_at_day_10_is_the_same_at_day_30() {
    resumed_at_day_10_equals_continuous_at_day_30(1);
}

#[test]
fn seed_3_resumed_at_day_10_is_the_same_at_day_30() {
    resumed_at_day_10_equals_continuous_at_day_30(3);
}
