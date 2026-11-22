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

#[test]
fn rain_wets_those_out_in_it_and_a_roof_dries_them() {
    use hf_sim::ids::TilePos;
    use hf_sim::mind::Activity;
    use hf_sim::systems::weather::Sky;
    use hf_sim::time::TICKS_PER_HOUR;
    use hf_sim::world::Material;
    let mut w = World::generate(1703, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    let ids = w.found_party(3);
    w.meta.autoplan = false;
    w.build_house(40, 40, 7, 6, 1, Material::WOOD, Material::WOOD);
    w.rebuild_caches();
    w.ensure_nav();
    let at = |w: &World, id| w.agents.iter().position(|a| a.id == id).unwrap();
    let (i_out, i_in, i_fire) = (at(&w, ids[0]), at(&w, ids[1]), at(&w, ids[2]));
    let p_out = TilePos::new(10, 10, 2);
    let p_in = TilePos::new(43, 42, 2);
    let mut rain = w.weather;
    rain.sky = Sky::Rain;
    rain.drizzle = false;
    rain.temperature_c = 12.0;
    w.force_weather = Some(rain);
    let hold = |w: &mut World, spots: &[(usize, TilePos)]| {
        for &(j, p) in spots {
            let a = &mut w.agents[j];
            a.activity = Activity::Loitering { ticks_left: 100 };
            a.pos = p;
            a.from = p;
            a.path.clear();
            a.needs.hunger = 0.9;
        }
    };
    for _ in 0..TICKS_PER_HOUR * 2 {
        hold(&mut w, &[(i_out, p_out), (i_in, p_in)]);
        w.step();
    }
    assert!(
        w.agents[i_out].wet >= 0.6,
        "soaked: {}",
        w.agents[i_out].wet
    );
    assert_eq!(w.agents[i_in].wet, 0.0, "a roof keeps the rain off");
    // The rain stops and the soaked one sits beside a lit hearth: dry within 2 hours. (Standing
    // out in the rain beside a hearth still wets, as the issue words it: outdoors while wet.)
    rain.sky = Sky::Clear;
    w.force_weather = Some(rain);
    w.agents[i_fire].wet = w.agents[i_out].wet;
    let near = TilePos::new(20, 20, 2);
    let id = w.alloc_structure_id();
    w.structures.push(hf_sim::structures::Structure {
        id,
        kind: hf_sim::structures::StructureKind::Hearth,
        pos: near,
        material: Material::STONE_BLOCK,
        progress: 1.0,
        active: true,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
    w.rebuild_caches();
    for _ in 0..TICKS_PER_HOUR * 2 {
        hold(&mut w, &[(i_fire, near)]);
        w.step();
    }
    assert_eq!(w.agents[i_fire].wet, 0.0, "dried by the hearth");
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
