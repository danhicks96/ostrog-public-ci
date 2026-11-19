//! #237: stress follows the whole of what a person carries. A contented person with one
//! grievance does not break down every few days; someone with nothing but trouble does.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::mind::thoughts::ThoughtKind as T;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn breaks_over(days: u64, good: &[T], bad: &[T]) -> usize {
    let mut w = World::generate(2371, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = 0;
    w.rebuild_caches();
    let ids = w.found_party(1);
    w.meta.autoplan = false;
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let mut n = 0;
    for h in 0..days * 24 {
        let tick = h * TICKS_PER_HOUR + 900;
        w.meta.tick = tick;
        if h % 24 == 0 {
            for k in good.iter().chain(bad) {
                w.agents[i].thoughts.add(*k, tick, None);
            }
        }
        w.agents[i].needs.hunger = 1.0;
        w.events.pending.clear();
        hf_sim::systems::strain::step(&mut w);
        n += w
            .events
            .pending
            .iter()
            .filter(|e| matches!(e.kind, EventKind::Broke { .. }))
            .count();
    }
    let _ = TICKS_PER_DAY;
    n
}

#[test]
fn a_contented_person_with_one_trouble_does_not_break() {
    let n = breaks_over(
        20,
        &[T::AteWell, T::SleptInBed, T::GoodTalk, T::MadeSomething],
        &[T::Quarrel],
    );
    // Before #237 (stress from the gross bad alone) this was 4 or more: a break every few days.
    assert!(n <= 1, "broke {n} times in twenty days of good days");
}

#[test]
fn someone_with_nothing_but_trouble_still_breaks() {
    let n = breaks_over(20, &[], &[T::SleptRough, T::Quarrel, T::Cold, T::Hungry]);
    assert!(n >= 1, "never broke");
}
