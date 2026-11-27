//! A real sequence of daily notices can produce hardship, plenty and hardship
//! again before the first reprieve's cooldown ends.
use hf_sim::deity::{self, Lever, Trigger};
use hf_sim::engine::Engine;
use hf_sim::items::ItemKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::World;

fn observe_day(w: &mut World, hungry: bool) {
    for a in &mut w.agents {
        a.needs.hunger = if hungry { 0.05 } else { 1.0 };
        a.needs.rest = 1.0;
        a.needs.comfort = 1.0;
        a.body.illness = None;
    }
    w.meta.tick += TICKS_PER_DAY;
    deity::step(w);
}

#[test]
fn repeated_hardship_ends_new_pressure_without_renewing_the_reprieve() {
    let mut w = World::opening_world(7);
    w.meta.engine = Engine::Relentless;
    w.meta.tick = (w.meta.tick / TICKS_PER_DAY + 1) * TICKS_PER_DAY + 4 * TICKS_PER_HOUR + 11;
    w.drop_item(ItemKind::Meal, 900, w.communities[0].home);
    observe_day(&mut w, true);
    observe_day(&mut w, true);
    let respite = w
        .deity_answers
        .iter()
        .find(|a| a.trigger == Trigger::Hardship && a.lever == Lever::Sickness)
        .unwrap()
        .clone();
    for _ in 0..3 {
        observe_day(&mut w, false);
    }
    let pressure = w
        .deity_answers
        .iter()
        .find(|a| a.trigger == Trigger::Plenty && a.lever == Lever::Harvest)
        .unwrap()
        .clone();
    observe_day(&mut w, true);
    assert_eq!(w.deity_watch.hard_days, 1);
    assert!(
        w.deity_answers
            .iter()
            .find(|a| a.event == pressure.event)
            .unwrap()
            .until
            > w.meta.tick
    );
    observe_day(&mut w, true);
    assert_eq!(w.deity_watch.hard_days, 2);
    assert!(w.meta.tick < respite.until + 3 * TICKS_PER_DAY);
    assert!(w.meta.tick < pressure.until);
    assert_eq!(
        w.deity_answers
            .iter()
            .find(|a| a.event == pressure.event)
            .unwrap()
            .until,
        w.meta.tick
    );
    assert!(w
        .deity_answers
        .iter()
        .filter(|a| a.trigger == Trigger::Plenty)
        .all(|a| a.until <= w.meta.tick));
    let aids: Vec<_> = w
        .deity_answers
        .iter()
        .filter(|a| a.trigger == Trigger::Hardship && a.lever == Lever::Sickness)
        .collect();
    assert_eq!(aids.len(), 1);
    assert_eq!(
        (aids[0].event, aids[0].until),
        (respite.event, respite.until)
    );
}
