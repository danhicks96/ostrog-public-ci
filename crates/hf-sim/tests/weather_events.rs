//! Issue 25: rain and snow starting or stopping reach the feed and chronicle as events.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::systems::hazards::cold_step;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

#[test]
fn wet_weather_turning_is_an_event() {
    let mut w = World::generate(5, GenParams::small());
    let mut kinds = Vec::new();
    for _ in 0..TICKS_PER_DAY * 20 {
        w.step();
        for e in &w.last_events {
            if let EventKind::Weather { kind } = &e.kind {
                kinds.push(kind.clone());
            }
        }
    }
    assert!(
        kinds.iter().any(|k| k == "rain"),
        "a year has rain in it: {kinds:?}"
    );
    assert!(
        kinds.iter().any(|k| k.ends_with("stopped")),
        "and it stops: {kinds:?}"
    );
    assert!(kinds.len() < 200, "not every sky flicker: {}", kinds.len());
}

/// #1262: wet clothes and fur strip heat faster than a dry cold does.
#[test]
fn soaked_in_the_cold_chills_faster() {
    let mut w = World::generate(1262, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(2);
    w.meta.autoplan = false;
    let mut wx = hf_sim::systems::weather::current(&w);
    wx.temperature_c = 0.0;
    w.weather = wx;

    let dry = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let wet = w.agents.iter().position(|a| a.id == ids[1]).unwrap();
    // Clothed, not furred, more than one tile apart so neither counts as huddled.
    w.agents[dry].clothes_wear = 1.0;
    w.agents[dry].furred = false;
    w.agents[dry].pos = TilePos::new(30, 30, 2);
    w.agents[wet].clothes_wear = 1.0;
    w.agents[wet].furred = false;
    w.agents[wet].wet = 1.0;
    w.agents[wet].pos = TilePos::new(40, 40, 2);

    let dry_before = w.agents[dry].needs.warmth;
    let wet_before = w.agents[wet].needs.warmth;

    for _ in 0..6 {
        w.meta.tick += TICKS_PER_HOUR;
        cold_step(&mut w);
    }

    let dry_loss = dry_before - w.agents[dry].needs.warmth;
    let wet_loss = wet_before - w.agents[wet].needs.warmth;

    assert!(
        dry_loss > 0.0,
        "the dry one should still be chilling: {dry_loss}"
    );
    let ratio = wet_loss / dry_loss;
    assert!(
        (1.7..=1.9).contains(&ratio),
        "wet should chill 1.7x-1.9x the dry rate, got {ratio} (wet {wet_loss}, dry {dry_loss})"
    );
    assert!(
        wet_loss - dry_loss >= 0.07,
        "wet should lose at least 0.07 more warmth than dry: wet {wet_loss}, dry {dry_loss}"
    );
}
