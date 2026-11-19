//! #346: frost in the fields, hardening to the cold, and the even air below ground.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn cold_world() -> (World, usize) {
    let mut w = World::generate(3461, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(2);
    w.meta.autoplan = false;
    let mut wx = hf_sim::systems::weather::current(&w);
    wx.temperature_c = -12.0;
    w.weather = wx;
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    (w, i)
}

#[test]
fn a_killing_frost_takes_the_growing_crop() {
    let (mut w, _) = cold_world();
    let at = w.grid.surface(35, 35);
    w.communities[0].field_tiles.push(at);
    w.plants.push(hf_sim::plants::Plant {
        pos: at,
        kind: hf_sim::plants::PlantKind::Grain,
        growth: 0.5,
        variant: 0,
        fruit: 0.0,
        alive: true,
    });
    // An early frost: an autumn night by the valley's calendar.
    let day = (0..40u64)
        .find(|&d| {
            w.meta.tick = w.meta.epoch + d * TICKS_PER_DAY;
            w.calendar().season == hf_sim::time::Season::Autumn
        })
        .unwrap();
    w.meta.tick = w.meta.epoch + day * TICKS_PER_DAY + 4 * TICKS_PER_HOUR + 555;
    hf_sim::thermal::step(&mut w);
    assert!(!w.plants.last().unwrap().alive);
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Frosted { .. })));
}

/// #268: the lapse is the simulation's one temperature rule, not only a warmth-drain detail.
#[test]
fn high_ground_frosts_before_the_valley_floor() {
    let mut w = World::generate(2681, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(2);
    let _ = ids;
    w.meta.autoplan = false;
    // A plateau eight levels above the founding ground, inside the flattened field.
    w.flatten(38, 38, 8, 8, 9);
    w.rebuild_caches();
    // Above the killing frost at the datum, below it on the plateau.
    let mut wx = hf_sim::systems::weather::current(&w);
    wx.temperature_c = -3.0;
    w.weather = wx;
    let low = w.grid.surface(20, 20);
    let high = w.grid.surface(40, 40);
    assert!(low.z < high.z);
    assert!(
        w.temp_at(low) >= hf_sim::thermal::KILLING_FROST,
        "the valley floor is above the frost: {}",
        w.temp_at(low)
    );
    assert!(
        w.temp_at(high) < hf_sim::thermal::KILLING_FROST,
        "the plateau is under it: {}",
        w.temp_at(high)
    );
    for at in [low, high] {
        w.communities[0].field_tiles.push(at);
        w.plants.push(hf_sim::plants::Plant {
            pos: at,
            kind: hf_sim::plants::PlantKind::Grain,
            growth: 0.5,
            variant: 0,
            fruit: 0.0,
            alive: true,
        });
    }
    // An early frost: an autumn night by the valley's calendar.
    let day = (0..40u64)
        .find(|&d| {
            w.meta.tick = w.meta.epoch + d * TICKS_PER_DAY;
            w.calendar().season == hf_sim::time::Season::Autumn
        })
        .unwrap();
    w.meta.tick = w.meta.epoch + day * TICKS_PER_DAY + 4 * TICKS_PER_HOUR + 555;
    hf_sim::thermal::step(&mut w);
    assert!(
        w.plants.iter().find(|p| p.pos == low).unwrap().alive,
        "the valley floor's crop stands"
    );
    assert!(
        !w.plants.iter().find(|p| p.pos == high).unwrap().alive,
        "the plateau's crop is frosted"
    );
}

#[test]
fn a_dug_cellar_reads_warmer_than_the_winter_surface() {
    let (w, _) = cold_world();
    let surface = w.grid.surface(30, 30);
    let cellar = surface.offset(0, 0, -3);
    assert!(
        w.temp_at(cellar) > w.temp_at(surface),
        "three levels down reads warmer: {} vs {}",
        w.temp_at(cellar),
        w.temp_at(surface)
    );
    assert!(
        (w.temp_at(cellar) - w.temp_at(surface) - 3.0 * hf_sim::thermal::LAPSE_C_PER_LEVEL).abs()
            < 1e-4,
        "the difference is exactly three levels of lapse"
    );
}

#[test]
fn a_winter_out_in_the_cold_hardens_them() {
    let (mut w, i) = cold_world();
    w.agents[i].needs.warmth = 0.5;
    for d in 1..6u64 {
        w.meta.tick = d * TICKS_PER_DAY + 12 * TICKS_PER_HOUR + 555;
        hf_sim::thermal::step(&mut w);
    }
    assert!(w.agents[i].hardened > 1.0, "{}", w.agents[i].hardened);
}

/// #405: the valley's own weather brings the killing frost. On the first morning of spring or
/// the last of autumn, one time in six, the night keeps winter's cold and the growing crop
/// dies; on the other mornings of those seasons the coldest hour stays above the frost.
#[test]
fn a_late_frost_comes_from_the_weather_itself() {
    let (mut w, _) = cold_world();
    w.force_weather = None;
    let at = w.grid.surface(35, 35);
    w.communities[0].field_tiles.push(at);
    let mut frosts = 0;
    let mut shoulder_mornings = 0;
    for d in 0..200u64 {
        let tick = w.meta.epoch + d * TICKS_PER_DAY + 4 * TICKS_PER_HOUR + 555;
        w.meta.tick = tick;
        let cal = w.calendar();
        if !matches!(
            cal.season,
            hf_sim::time::Season::Spring | hf_sim::time::Season::Autumn
        ) {
            continue;
        }
        shoulder_mornings += 1;
        w.plants.retain(|p| p.pos != at);
        w.plants.push(hf_sim::plants::Plant {
            pos: at,
            kind: hf_sim::plants::PlantKind::Grain,
            growth: 0.5,
            variant: 0,
            fruit: 0.0,
            alive: true,
        });
        w.weather = hf_sim::systems::weather::current(&w);
        let edge = hf_sim::systems::weather::late_or_early_frost(
            w.meta.seed,
            &hf_sim::time::Calendar::from_tick(tick),
        );
        w.events.pending.clear();
        hf_sim::thermal::step(&mut w);
        let frosted = w
            .events
            .pending
            .iter()
            .any(|e| matches!(e.kind, EventKind::Frosted { .. }));
        assert_eq!(frosted, edge, "day {d}: {:.1} C", w.weather.temperature_c);
        assert_eq!(frosted, !w.plants.last().unwrap().alive);
        frosts += frosted as u32;
    }
    // Ten years: twenty turns of the year, about one in six of them a frost.
    assert_eq!(shoulder_mornings, 100);
    assert!((1..=8).contains(&frosts), "{frosts} frosts in 200 days");
}
