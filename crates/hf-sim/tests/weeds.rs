//! #286: thistle, the weed of worked ground. A worked field picks it up over time; it crowds
//! the crops beside it; a mature one pressing a crop is ordered pulled.

use hf_sim::ids::TilePos;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::work::DesignationKind;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

/// Flat ground, no plants, no communities' fields: two wheat crops, one with a thistle beside
/// it. After the same ticks the crowded one has grown less.
#[test]
fn a_thistle_crowds_the_crop_beside_it() {
    let mut w = World::generate(2861, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    for c in w.communities.iter_mut() {
        c.field_tiles.clear();
    }
    w.rebuild_caches();
    let (crowded, free) = (w.grid.surface(20, 20), w.grid.surface(45, 45));
    w.plants.push(Plant::new(crowded, PlantKind::Grain, 0.1, 0));
    w.plants.push(Plant::new(
        w.grid.surface(21, 20),
        PlantKind::Thistle,
        0.5,
        0,
    ));
    w.plants.push(Plant::new(free, PlantKind::Grain, 0.1, 0));
    let mut wx = hf_sim::systems::weather::current(&w);
    wx.growth_factor = 0.8;
    w.force_weather = Some(wx);
    for t in 0..20_000u64 {
        w.meta.tick = t;
        w.weather = wx;
        hf_sim::systems::plants::step(&mut w);
    }
    let g = |p: TilePos| w.plants.iter().find(|x| x.pos == p).unwrap().growth;
    assert!(
        g(crowded) < g(free),
        "crowded {} against free {}",
        g(crowded),
        g(free)
    );
}

/// A community's worked field gains thistles over days, and only there: every thistle in the
/// world stands on a field tile of that community. The plants system is stepped on its own so
/// the planner does not sow the tiles bare behind the test's back.
#[test]
fn a_worked_field_picks_up_thistles_over_time() {
    let mut w = World::opening_world(2862);
    let fs = w.meta.founding_site;
    let mut field = Vec::new();
    for dy in 0..4 {
        for dx in 0..6 {
            field.push(w.grid.surface(fs.x + 10 + dx, fs.y + 10 + dy));
        }
    }
    for p in &field {
        w.plants.retain(|q| q.pos != *p);
    }
    for c in w.communities.iter_mut() {
        c.field_tiles = field.clone();
    }
    w.rebuild_caches();
    let mut wx = hf_sim::systems::weather::current(&w);
    wx.growth_factor = 0.8;
    w.force_weather = Some(wx);
    for t in 0..6 * 28_800u64 {
        w.meta.tick = t;
        w.weather = wx;
        hf_sim::systems::plants::step(&mut w);
    }
    let weeds: Vec<TilePos> = w
        .plants
        .iter()
        .filter(|p| p.alive && p.kind == PlantKind::Thistle)
        .map(|p| p.pos)
        .collect();
    assert!(!weeds.is_empty(), "a worked field picks up thistles");
    for p in &weeds {
        assert!(
            field.contains(p),
            "a thistle only on worked ground, not at {p:?}"
        );
    }
}

/// End to end in a living valley: a mature thistle on a field tile that presses a neighbouring
/// crop is ordered pulled, and the pulling happens.
#[test]
fn a_mature_thistle_pressing_a_crop_is_pulled() {
    let mut w = World::opening_world(2863);
    let weed_at = w
        .grid
        .surface(w.meta.founding_site.x + 14, w.meta.founding_site.y + 14);
    let crop_at = w
        .grid
        .surface(w.meta.founding_site.x + 15, w.meta.founding_site.y + 14);
    w.communities[0].field_tiles = vec![weed_at, crop_at];
    w.plants.retain(|p| p.pos != weed_at && p.pos != crop_at);
    w.plants
        .push(Plant::new(weed_at, PlantKind::Thistle, 1.0, 0));
    w.plants.push(Plant::new(crop_at, PlantKind::Grain, 0.2, 0));
    w.rebuild_caches();
    let mut ordered = false;
    for _ in 0..8 * 28_800u64 {
        w.step();
        if w.designations
            .iter()
            .any(|d| !d.done && matches!(d.kind, DesignationKind::Uproot) && d.pos == weed_at)
        {
            ordered = true;
        }
        if ordered && !w.plants.iter().any(|p| p.alive && p.pos == weed_at) {
            return;
        }
    }
    assert!(ordered, "the planner orders the pressing weed pulled");
    panic!("the weed was ordered pulled but never was");
}

/// The community id the first test world uses: kept here so a reader finds how the weed
/// arrival keys its stream (systems/plants.rs, `purpose::PLANTS`, tag 0x7715).
#[test]
fn weed_arrival_is_deterministic() {
    let run = || {
        let mut w = World::opening_world(2862);
        let fs = w.meta.founding_site;
        let mut field = Vec::new();
        for dy in 0..4 {
            for dx in 0..6 {
                field.push(w.grid.surface(fs.x + 10 + dx, fs.y + 10 + dy));
            }
        }
        for p in &field {
            w.plants.retain(|q| q.pos != *p);
        }
        for c in w.communities.iter_mut() {
            c.field_tiles = field.clone();
        }
        w.rebuild_caches();
        let mut wx = hf_sim::systems::weather::current(&w);
        wx.growth_factor = 0.8;
        w.force_weather = Some(wx);
        for t in 0..3 * 28_800u64 {
            w.meta.tick = t;
            w.weather = wx;
            hf_sim::systems::plants::step(&mut w);
        }
        w.plants
            .iter()
            .filter(|p| p.alive && p.kind == PlantKind::Thistle)
            .map(|p| p.pos)
            .collect::<Vec<_>>()
    };
    assert_eq!(run(), run(), "the same field picks up the same thistles");
}
