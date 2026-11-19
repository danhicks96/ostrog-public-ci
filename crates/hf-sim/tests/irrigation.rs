//! #325: a crop beside water drinks: it grows faster, and a drought does not touch it.

use hf_sim::ids::TilePos;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

#[test]
fn a_watered_crop_outgrows_a_dry_one() {
    let mut w = World::generate(3251, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.rebuild_caches();
    let (wet, dry) = (w.grid.surface(20, 20), w.grid.surface(45, 45));
    // Water beside the one.
    let pool = w.grid.surface(21, 20);
    let mut t = w.grid.get(pool);
    t.flags = hf_sim::world::TileFlags(t.flags.0 | hf_sim::world::TileFlags::WATER);
    w.grid.set(pool, t);
    assert!(hf_sim::systems::plants::watered(&w, wet));
    assert!(!hf_sim::systems::plants::watered(&w, dry));
    for p in [wet, dry] {
        w.plants.push(Plant::new(p, PlantKind::Grain, 0.1, 0));
    }
    let mut wx = hf_sim::systems::weather::current(&w);
    wx.growth_factor = 0.8;
    w.force_weather = Some(wx);
    for t in 0..20_000u64 {
        w.meta.tick = t;
        w.weather = wx;
        hf_sim::systems::plants::step(&mut w);
    }
    let g = |p: TilePos| w.plants.iter().find(|x| x.pos == p).unwrap().growth;
    assert!(g(wet) > g(dry), "watered {} against dry {}", g(wet), g(dry));
}

// ---- #325 dug channels (channels.rs, option W0 of docs/design-fluids.md) ----

use hf_sim::ids::CommunityId;
use hf_sim::time::TICKS_PER_HOUR;

/// A flat meadow with a settled party, a pond at the west end and a field of twelve tiles at
/// the east, too far from the pond for the crops to drink from it.
fn dry_field(seed: u64) -> (World, CommunityId, Vec<TilePos>) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    let ids = w.found_party(5);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    w.ensure_nav();
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    // The pond: two tiles of shallow water.
    let mut pond = Vec::new();
    for dx in 0..2 {
        let p = w.grid.surface(home.x - 16 + dx, home.y + 6);
        let mut t = w.grid.get(p);
        t.flags = hf_sim::world::TileFlags(t.flags.0 | hf_sim::world::TileFlags::WATER);
        w.grid.set(p, t);
        pond.push(p);
    }
    w.waters.surveyed = true;
    w.waters.bodies.push(hf_sim::floods::Body {
        tiles: pond.clone(),
        surface: pond[0].z,
        ..Default::default()
    });
    // The field.
    let mut field = Vec::new();
    for dy in 0..3 {
        for dx in 0..4 {
            field.push(w.grid.surface(home.x + 7 + dx, home.y + 5 + dy));
        }
    }
    w.communities
        .iter_mut()
        .find(|c| c.id == cid)
        .unwrap()
        .field_tiles = field.clone();
    (w, cid, field)
}

#[test]
fn a_dug_channel_moistens_the_field_it_reaches_and_a_gap_stops_the_water() {
    let (mut w, cid, field) = dry_field(3252);
    let probe = field[5];
    assert!(
        !hf_sim::systems::plants::watered(&w, probe),
        "no water near the field yet"
    );
    assert!(hf_sim::channels::field_is_dry(&w, cid).is_some());

    let path =
        hf_sim::channels::route(&w, cid).expect("a downhill route from the pond to the field");
    assert!(
        path.len() >= 10 && path.len() <= hf_sim::channels::MAX_LEN,
        "len {}",
        path.len()
    );
    // It starts on a bank of the pond and never climbs.
    let pond = &w.waters.bodies[0].tiles;
    assert!(pond
        .iter()
        .any(|p| (p.x - path[0].x).abs() + (p.y - path[0].y).abs() == 1));
    assert!(path
        .windows(2)
        .all(|p| p[1].z <= p[0].z && p[0].z - p[1].z <= 1));
    // No field tile is dug up.
    assert!(path.iter().all(|p| !field.contains(p)));

    hf_sim::channels::begin(&mut w, cid, path.clone());
    assert!(
        !hf_sim::systems::plants::watered(&w, probe),
        "an ordered ditch is not a ditch"
    );
    // Cut all but the second tile: the water goes one tile, and stops at the gap.
    for p in path
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != 1)
        .map(|(_, p)| *p)
    {
        hf_sim::channels::dug(&mut w, p);
    }
    assert!(
        !hf_sim::systems::plants::watered(&w, probe),
        "a gap in the cut stops the water"
    );
    hf_sim::channels::dug(&mut w, path[1]);
    assert!(
        hf_sim::systems::plants::watered(&w, probe),
        "the whole cut waters the field"
    );
    // A control field of the same kind elsewhere stays dry.
    let far = w.grid.surface(55, 55);
    assert!(!hf_sim::systems::plants::watered(&w, far));
}

#[test]
fn no_water_in_reach_means_no_route() {
    let (mut w, cid, _) = dry_field(3253);
    w.waters.bodies.clear();
    assert!(hf_sim::channels::route(&w, cid).is_none());
}

#[test]
fn a_channel_survives_a_save_and_load() {
    let (mut w, cid, _) = dry_field(3254);
    let path = hf_sim::channels::route(&w, cid).unwrap();
    hf_sim::channels::begin(&mut w, cid, path.clone());
    hf_sim::channels::dug(&mut w, path[0]);
    let bytes = hf_sim::save::encode(&hf_sim::save::snapshot::Snapshot::capture(&w)).unwrap();
    let back = hf_sim::save::decode(&bytes).unwrap().restore();
    assert_eq!(back.channels, w.channels);
}

#[test]
fn people_with_a_dry_field_and_a_pond_dig_the_channel_themselves() {
    let (mut w, cid, _field) = dry_field(3255);
    w.meta.autoplan = true;
    let mut proposed = None;
    for day in 0..40u64 {
        for _ in 0..hf_sim::time::TICKS_PER_DAY {
            w.step();
        }
        if proposed.is_none()
            && w.projects
                .iter()
                .any(|p| p.kind == hf_sim::project::ProjectKind::Channel)
        {
            proposed = Some(day);
        }
        if w.channels.iter().any(|c| c.community == cid && c.done) {
            break;
        }
    }
    let c = w
        .channels
        .iter()
        .find(|c| c.community == cid && !c.tiles.is_empty());
    eprintln!(
        "proposed day {proposed:?}; channel {:?}",
        c.map(|c| (c.tiles.len(), c.reached(), c.done, c.gave_up))
    );
    let c = c.expect("someone proposed a channel");
    assert!(
        c.done,
        "the ditch was dug: {} of {} tiles",
        c.reached(),
        c.tiles.len()
    );
    let probe_dry = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .unwrap()
        .field_tiles[0];
    assert!(hf_sim::systems::plants::watered(&w, probe_dry) || c.reached() == c.tiles.len());
}

#[test]
fn a_field_that_already_drinks_digs_nothing() {
    let (mut w, cid, field) = dry_field(3256);
    // Water right beside the field: every tile drinks.
    let p = w.grid.surface(field[0].x - 1, field[0].y);
    let mut t = w.grid.get(p);
    t.flags = hf_sim::world::TileFlags(t.flags.0 | hf_sim::world::TileFlags::WATER);
    w.grid.set(p, t);
    assert!(
        field
            .iter()
            .filter(|f| hf_sim::systems::plants::watered(&w, **f))
            .count()
            * 2
            >= field.len()
    );
    assert!(hf_sim::channels::field_is_dry(&w, cid).is_none());
}
