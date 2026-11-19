//! #278: spells over the valley: begin, grip for days, lift; one at a time, by season.

use hf_sim::events::EventKind;
use hf_sim::spells::{self, Spell, SpellKind};
use hf_sim::time::{Calendar, Season, TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn world() -> World {
    let mut w = World::generate(2781, GenParams::small());
    w.found_party(6);
    w
}

#[test]
fn spells_begin_and_lift_one_at_a_time_as_the_season_allows() {
    let mut w = world();
    let (mut began, mut lifted) = (0, 0);
    for d in 0..600u64 {
        w.meta.tick = d * TICKS_PER_DAY + 6 * TICKS_PER_HOUR + 11;
        w.events.pending.clear();
        spells::step(&mut w);
        assert!(w.spells.len() <= 1, "one at a time");
        for e in &w.events.pending {
            match &e.kind {
                EventKind::SpellBegan { .. } => {
                    began += 1;
                    let s = Calendar::from_tick(w.meta.tick).season;
                    let k = w.spells[0].kind;
                    let out_of_season = (k == SpellKind::HeatWave && s != Season::Summer)
                        || (k == SpellKind::ColdSnap && s != Season::Winter);
                    assert!(!out_of_season, "{k:?} in {s:?}");
                }
                EventKind::SpellLifted { days, .. } => {
                    lifted += 1;
                    assert!((3..=7).contains(days));
                }
                _ => {}
            }
        }
    }
    assert!(began >= 5, "spells began {began} times in 600 days");
    assert!(lifted + 1 >= began, "and lifted");
}

#[test]
fn a_heat_wave_and_a_drought_lean_the_weather() {
    let mut w = world();
    // A summer noon.
    let t = (0..TICKS_PER_DAY * 20)
        .step_by(TICKS_PER_DAY as usize)
        .find(|&t| Calendar::from_tick(t).season == Season::Summer)
        .unwrap()
        + 12 * TICKS_PER_HOUR;
    w.meta.tick = t;
    let plain = hf_sim::systems::weather::current(&w);
    w.spells.push(Spell {
        kind: SpellKind::HeatWave,
        began: t - 1,
        until: t + TICKS_PER_DAY,
    });
    let hot = hf_sim::systems::weather::current(&w);
    assert!(hot.temperature_c >= plain.temperature_c + 8.9);
    w.spells[0].kind = SpellKind::Drought;
    let dry = hf_sim::systems::weather::current(&w);
    assert_ne!(dry.sky, hf_sim::systems::weather::Sky::Rain);
    assert!(dry.growth_factor < plain.growth_factor || plain.growth_factor == 0.0);
}
