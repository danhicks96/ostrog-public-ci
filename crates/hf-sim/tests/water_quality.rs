//! #453 (epic #477, part 1): water carries quality. On a coast the body on the map's low
//! edge is the sea and is salt; a fresh body whose shore stands within two tiles of the sea
//! is brackish; everywhere else surface water is fresh. An old save whose bodies have no
//! quality field loads as fresh (serde default).

use hf_sim::floods::{self, Body, WaterQuality};
use hf_sim::ids::TilePos;
use hf_sim::world::gen::{GenParams, Landform};
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

fn water_tile() -> Tile {
    Tile {
        material: Material::WATER,
        shape: Shape::Floor,
        flags: TileFlags(TileFlags::WATER),
    }
}

fn body_at(w: &World, p: TilePos) -> &Body {
    w.waters
        .bodies
        .iter()
        .find(|b| b.tiles.contains(&p))
        .expect("a surveyed body there")
}

/// A coast world, flattened: a strip of sea along the west edge, a pond two tiles from it
/// (a lagoon), and a pond far inland.
fn coast(seed: u64) -> World {
    let mut w = World::generate(
        seed,
        GenParams {
            landform: Landform::Coast,
            ..GenParams::small()
        },
    );
    w.flatten(0, 0, 64, 64, 1);
    w.plants.clear();
    for y in 0..64 {
        w.grid.set(TilePos::new(0, y, 2), water_tile());
    }
    for y in 20..=22 {
        for x in 2..=4 {
            w.grid.set(TilePos::new(x, y, 2), water_tile());
        }
    }
    for y in 40..=42 {
        for x in 30..=32 {
            w.grid.set(TilePos::new(x, y, 2), water_tile());
        }
    }
    w.rebuild_caches();
    floods::survey(&mut w);
    w
}

#[test]
fn survey_marks_the_coast_sea_salt_and_an_adjoining_pond_brackish() {
    let w = coast(4531);
    let sea = body_at(&w, TilePos::new(0, 10, 2));
    assert_eq!(
        sea.quality,
        WaterQuality::Salt,
        "the edge-hugging sea is salt"
    );
    let lagoon = body_at(&w, TilePos::new(3, 21, 2));
    assert_eq!(
        lagoon.quality,
        WaterQuality::Brackish,
        "a pond within two tiles of the sea is brackish"
    );
    let inland = body_at(&w, TilePos::new(31, 41, 2));
    assert_eq!(
        inland.quality,
        WaterQuality::Fresh,
        "a pond far from the sea stays fresh"
    );
    // The lookup agrees for any surface water tile.
    assert_eq!(
        floods::quality(&w, TilePos::new(0, 10, 2)),
        Some(WaterQuality::Salt)
    );
    assert_eq!(
        floods::quality(&w, TilePos::new(3, 21, 2)),
        Some(WaterQuality::Brackish)
    );
    assert_eq!(
        floods::quality(&w, TilePos::new(31, 41, 2)),
        Some(WaterQuality::Fresh)
    );
}

#[test]
fn valley_water_is_fresh_even_at_the_map_edge() {
    let mut w = World::generate(4532, GenParams::small());
    w.flatten(0, 0, 64, 64, 1);
    w.plants.clear();
    // A stream running edge to edge, and a pond on it: touching the edge is what a coast
    // stream fed from beyond does, and on a valley that water is still fresh.
    for y in 30..=31 {
        for x in 0..=63 {
            w.grid.set(TilePos::new(x, y, 2), water_tile());
        }
    }
    for y in 40..=42 {
        for x in 20..=22 {
            w.grid.set(TilePos::new(x, y, 2), water_tile());
        }
    }
    w.rebuild_caches();
    floods::survey(&mut w);
    assert!(!w.waters.bodies.is_empty());
    for b in &w.waters.bodies {
        assert_eq!(
            b.quality,
            WaterQuality::Fresh,
            "valley water is fresh (body of {} tiles)",
            b.tiles.len()
        );
    }
    assert_eq!(
        floods::quality(&w, TilePos::new(0, 30, 2)),
        Some(WaterQuality::Fresh)
    );
}

#[test]
fn a_body_without_the_quality_field_loads_as_fresh() {
    // The shape a save written before #453 has: no `quality` field on Body.
    let old = r#"{
        "kind": "Lake",
        "surface": 2,
        "tiles": [{"x": 1, "y": 2, "z": 2}],
        "shore": [{"x": 1, "y": 2, "z": 2}],
        "catchment": 5,
        "stage": 0.25,
        "high": false,
        "dried": []
    }"#;
    let b: Body = serde_json::from_str(old).expect("an old-format body loads");
    assert_eq!(b.quality, WaterQuality::Fresh);
}
