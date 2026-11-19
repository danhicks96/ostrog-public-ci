//! #587 (rule 7): a normal person sleeps through the night. One who lay down early stays abed
//! until the night ends rather than rising rested at 03:00 to loiter in the dark; hunger or
//! thirst still gets a sleeper up.

use hf_sim::agents::Species;
use hf_sim::mind::Activity;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

/// A flat meadow at `hour` on day 1, with one tired person lying down on it.
fn lying_down(seed: u64, hour: u64, rest: f32, hunger: f32) -> (World, usize) {
    let mut p = GenParams::small();
    p.width = 64;
    p.height = 64;
    let mut w = World::generate(seed, p);
    w.flatten(2, 2, 60, 60, 1);
    w.meta.tick = TICKS_PER_DAY + hour * TICKS_PER_HOUR;
    w.rebuild_caches();
    let at = w.grid.surface(30, 30);
    let id = w.spawn_agent(Species::Person, at);
    let i = w.agents.iter().position(|a| a.id == id).unwrap();
    let now = w.meta.tick;
    let a = &mut w.agents[i];
    a.needs.rest = rest;
    a.needs.hunger = hunger;
    a.needs.thirst = 1.0;
    a.needs.warmth = 1.0;
    a.needs.social = 1.0;
    a.activity = Activity::Sleeping {
        bed: None,
        until: now + 8 * TICKS_PER_HOUR,
    };
    a.activity_ticks = 0;
    (w, i)
}

fn hour(w: &World) -> u64 {
    (w.meta.tick % TICKS_PER_DAY) / TICKS_PER_HOUR
}

#[test]
fn one_who_lay_down_at_seven_sleeps_until_the_night_ends() {
    let (mut w, i) = lying_down(31, 19, 0.05, 1.0);
    // Through the small hours: 19:00 to 05:59 the next morning.
    while w.meta.tick < 2 * TICKS_PER_DAY + 6 * TICKS_PER_HOUR - 1 {
        w.step();
        assert!(
            w.agents[i].activity.is_asleep(),
            "up at {:02}:00 in the dark, rest {:.2}",
            hour(&w),
            w.agents[i].needs.rest
        );
    }
    // And up in the morning.
    let mut up = false;
    for _ in 0..3 * TICKS_PER_HOUR {
        w.step();
        if !w.agents[i].activity.is_asleep() {
            up = true;
            break;
        }
    }
    assert!(up, "still abed at {:02}:00", hour(&w));
}

#[test]
fn hunger_still_gets_a_rested_sleeper_up_in_the_night() {
    // Lay down at 19:00 already hungry: by the small hours, rested and hungry, they rise.
    let (mut w, i) = lying_down(32, 19, 0.05, 0.16);
    let mut up_at = None;
    while w.meta.tick < 2 * TICKS_PER_DAY + 6 * TICKS_PER_HOUR {
        w.step();
        if !w.agents[i].activity.is_asleep() {
            up_at = Some(hour(&w));
            break;
        }
    }
    let h = up_at.expect("a hungry sleeper rose before dawn");
    assert!(!(6..22).contains(&h), "rose at {h:02}:00");
}
