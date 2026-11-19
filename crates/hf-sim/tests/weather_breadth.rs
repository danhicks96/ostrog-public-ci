//! #279: fog and storms, each in its season.

use hf_sim::systems::weather::{Sky, Weather};
use hf_sim::time::{Calendar, Season, TICKS_PER_HOUR};

#[test]
fn fog_and_storms_come_in_their_seasons() {
    let (mut fog, mut storm) = (0, 0);
    for h in 0..(200 * 24u64) {
        let t = h * TICKS_PER_HOUR;
        let s = Calendar::from_tick(t).season;
        match Weather::at(5, t).sky {
            Sky::Fog => {
                fog += 1;
                assert!(matches!(s, Season::Spring | Season::Autumn), "fog in {s:?}");
            }
            Sky::Storm => {
                storm += 1;
                assert!(
                    matches!(s, Season::Summer | Season::Autumn),
                    "a storm in {s:?}"
                );
            }
            _ => {}
        }
    }
    assert!(fog > 0 && storm > 0, "fog {fog}, storms {storm}");
    assert!(Sky::Storm.wet() && Sky::Rain.wet() && !Sky::Fog.wet());
}

/// #289 (DEC-8 items 2+3): the weather ground overlay is the simulation's own
/// statement of the looks the client draws — snow lying stops fire, frozen and
/// wet earth swap as the client's rules say, and a roof keeps the snow off.
#[test]
fn the_weather_ground_overlay_matches_the_look() {
    use hf_sim::ids::TilePos;
    use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
    use hf_sim::world::gen::GenParams;
    use hf_sim::world::Material;
    use hf_sim::World;

    fn forced(temp: f32, sky: Sky) -> Weather {
        Weather {
            temperature_c: temp,
            sky,
            growth_factor: 0.0,
            daylight: 0.0,
            drizzle: false,
        }
    }
    let mut w = World::generate(2893, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let _ = w.found_party(2);
    w.meta.autoplan = false;
    let at = w.grid.surface(20, 20);
    let ground = at.offset(0, 0, -1);
    assert_eq!(
        w.grid.get(ground).material,
        Material::GRASS,
        "the fixture's cover is grass (what flatten lays)"
    );
    // Bare soil beside it, set by hand, for the wet/frozen swaps.
    let soil = w.grid.surface(22, 22).offset(0, 0, -1);
    let mut st = w.grid.get(soil);
    st.material = Material::SOIL;
    w.grid.set(soil, st);

    // A hot winter day (a heatwave under the snow line's logic): snow falling
    // lies on the cover and carries no fire...
    let winter = (0..400u64)
        .find(|&d| {
            w.meta.tick = w.meta.epoch + d * TICKS_PER_DAY;
            Calendar::from_tick(w.meta.tick).season == Season::Winter
        })
        .unwrap();
    w.meta.tick = w.meta.epoch + winter * TICKS_PER_DAY + 12 * TICKS_PER_HOUR;
    w.weather = forced(25.0, Sky::Snow);
    assert!(
        hf_sim::cover::snow_lies(&w, ground),
        "snow lies while it snows"
    );
    assert_eq!(
        hf_sim::systems::hazards::fuel_at(&w, at),
        0,
        "snow-covered ground carries no fire"
    );

    // ...the same day, clear: no snow lies and the hot dry grass carries fire as
    // it always did (the overlay changes nothing the look does not promise)...
    w.weather = forced(25.0, Sky::Clear);
    assert!(!hf_sim::cover::snow_lies(&w, ground));
    assert!(
        hf_sim::systems::hazards::fuel_at(&w, at) > 0,
        "clear hot grass carries fire as before"
    );

    // ...and a roof over the tile holds the snow off: it burns under cover.
    w.weather = forced(25.0, Sky::Snow);
    w.roofs.insert(
        at,
        hf_sim::building::Roof {
            material: Material::WOOD,
            quality: 1.0,
            built: 0,
            snow: 0.0,
        },
    );
    assert!(
        !hf_sim::cover::snow_lies(&w, ground),
        "a roof keeps the snow off"
    );
    assert!(
        hf_sim::systems::hazards::fuel_at(&w, at) > 0,
        "roofed grass carries fire as before"
    );
    w.roofs.clear();

    // Summer: rain wets the bare earth; hard frost freezes it; mud stays mud in rain.
    let summer = (0..400u64)
        .find(|&d| {
            w.meta.tick = w.meta.epoch + d * TICKS_PER_DAY;
            Calendar::from_tick(w.meta.tick).season == Season::Summer
        })
        .unwrap();
    w.meta.tick = w.meta.epoch + summer * TICKS_PER_DAY + 12 * TICKS_PER_HOUR;
    w.weather = forced(18.0, Sky::Rain);
    assert_eq!(
        hf_sim::cover::cover_at(&w, soil),
        Material::WET_SOIL,
        "rain wets the bare earth"
    );
    w.weather = forced(-3.0, Sky::Clear);
    assert_eq!(
        hf_sim::cover::cover_at(&w, soil),
        Material::FROZEN_SOIL,
        "hard frost freezes it"
    );
    w.weather = forced(18.0, Sky::Clear);
    assert_eq!(
        hf_sim::cover::cover_at(&w, soil),
        Material::SOIL,
        "the dry earth is itself again"
    );
    let mud = w.grid.surface(25, 25).offset(0, 0, -1);
    let mut mt = w.grid.get(mud);
    mt.material = Material::MUD;
    w.grid.set(mud, mt);
    w.weather = forced(18.0, Sky::Rain);
    assert_eq!(
        hf_sim::cover::cover_at(&w, mud),
        Material::MUD,
        "mud is already as wet as it gets"
    );
}
