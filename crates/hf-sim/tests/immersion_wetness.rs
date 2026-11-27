//! Immersion connects physical water exposure to the existing wetness and drying state.
use hf_sim::agents::Species;
use hf_sim::ids::TilePos;
use hf_sim::systems::{hazards, weather::Sky};
use hf_sim::world::{gen::GenParams, Material, TileFlags};
use hf_sim::World;

fn scene() -> World {
    let mut w = World::generate(1703, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.flooded.clear();
    w.waters = Default::default();
    w.meta.autoplan = false;
    w.weather.sky = Sky::Clear;
    w.weather.temperature_c = 12.0;
    w.force_weather = Some(w.weather);
    w.rebuild_caches();
    w
}

fn set_water(w: &mut World, p: TilePos, extra: u8) {
    let mut tile = w.grid.get(p);
    tile.flags = TileFlags(tile.flags.0 | TileFlags::WATER | extra);
    w.grid.set(p, tile);
}

#[test]
fn immersion_soaks_during_normal_world_steps_without_soaking_dry_or_shallow_people() {
    let mut w = scene();
    let submerged = TilePos::new(10, 10, 2);
    let shallow = TilePos::new(20, 20, 2);
    let dry = TilePos::new(30, 30, 2);
    set_water(&mut w, submerged, TileFlags::FULL);
    set_water(&mut w, shallow, 0);
    let ids = [submerged, shallow, dry].map(|p| w.spawn_agent(Species::Person, p));
    w.ensure_nav();
    assert!(hazards::under_water(&w, submerged));
    assert!(!hazards::under_water(&w, shallow));
    w.meta.tick = 4;
    w.step();
    let at = |id| w.agents.iter().find(|a| a.id == id).unwrap();
    assert_eq!(
        at(ids[0]).wet,
        1.0,
        "the submerged person was soaked before escape"
    );
    assert_eq!(at(ids[1]).wet, 0.0, "shallow water is not full immersion");
    assert_eq!(
        at(ids[2]).wet,
        0.0,
        "a dry person remains dry in clear weather"
    );
    let bytes = hf_sim::save::encode(&hf_sim::save::snapshot::Snapshot::capture(&w)).unwrap();
    let restored = hf_sim::save::decode(&bytes).unwrap().restore();
    let reloaded = restored.agents.iter().find(|a| a.id == ids[0]).unwrap();
    assert_eq!(reloaded.wet, 1.0, "immersion survives a save round trip");
}

#[test]
fn immersion_between_drowning_checks_still_soaks() {
    let mut w = scene();
    let submerged = TilePos::new(10, 10, 2);
    set_water(&mut w, submerged, TileFlags::FULL);
    let id = w.spawn_agent(Species::Person, submerged);
    w.ensure_nav();
    assert!(hazards::under_water(&w, submerged));
    w.meta.tick = 0;
    w.step();
    let a = w.agents.iter().find(|a| a.id == id).unwrap();
    assert_eq!(
        a.wet, 1.0,
        "a brief immersion should not wait for a drowning check"
    );
}

#[test]
fn a_flooded_shelter_does_not_dry_a_submerged_person() {
    let mut w = scene();
    w.build_house(40, 40, 7, 6, 1, Material::WOOD, Material::WOOD);
    w.rebuild_caches();
    let p = TilePos::new(43, 42, 2);
    assert!(hazards::indoors(&w, p), "fixture must have a roof");
    set_water(&mut w, p, TileFlags::FULL);
    let id = w.spawn_agent(Species::Person, p);
    hazards::cold_step(&mut w);
    let a = w.agents.iter().find(|a| a.id == id).unwrap();
    assert_eq!(
        a.wet, 1.0,
        "submersion must take precedence over drying under a roof"
    );
    let mut tile = w.grid.get(p);
    tile.flags = TileFlags(tile.flags.0 & !(TileFlags::WATER | TileFlags::FULL));
    w.grid.set(p, tile);
    assert!(!hazards::under_water(&w, p));
    for _ in 0..7 {
        w.meta.tick += hf_sim::time::TICKS_PER_HOUR;
        hazards::cold_step(&mut w);
    }
    let a = w.agents.iter().find(|a| a.id == id).unwrap();
    assert_eq!(
        a.wet, 0.0,
        "the existing roof drying resumes after the flood"
    );
}

#[test]
fn a_recently_submerged_person_chills_faster_after_reaching_dry_ground() {
    let mut w = scene();
    let submerged = TilePos::new(10, 10, 2);
    set_water(&mut w, submerged, TileFlags::FULL);
    let wet_id = w.spawn_agent(Species::Person, submerged);
    let dry_id = w.spawn_agent(Species::Person, TilePos::new(30, 30, 2));
    w.ensure_nav();
    w.meta.tick = 4;
    w.step();
    w.weather.sky = Sky::Clear;
    w.weather.temperature_c = 0.0;
    for (id, pos) in [
        (wet_id, TilePos::new(12, 12, 2)),
        (dry_id, TilePos::new(30, 30, 2)),
    ] {
        let a = w.agents.iter_mut().find(|a| a.id == id).unwrap();
        a.pos = pos;
        a.from = pos;
        a.clothes_wear = 1.0;
        a.furred = false;
        a.needs.warmth = 1.0;
    }
    let wet = w.agents.iter().find(|a| a.id == wet_id).unwrap();
    assert_eq!(wet.wet, 1.0, "wetness comes from actual immersion");
    for _ in 0..6 {
        w.meta.tick += hf_sim::time::TICKS_PER_HOUR;
        hazards::cold_step(&mut w);
    }
    let wet = w.agents.iter().find(|a| a.id == wet_id).unwrap();
    let dry = w.agents.iter().find(|a| a.id == dry_id).unwrap();
    let wet_loss = 1.0 - wet.needs.warmth;
    let dry_loss = 1.0 - dry.needs.warmth;
    assert!(dry_loss > 0.0, "the dry control must actually be cold");
    assert!(
        (1.7..=1.9).contains(&(wet_loss / dry_loss)),
        "wet loss {wet_loss}, dry loss {dry_loss}"
    );
}
