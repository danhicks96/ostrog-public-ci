//! #390: lakes and streams have depth. A lake shelves from one level at its shore to three or
//! more in its middle; a fen's water is one level over waterlogged ground.

use hf_sim::ids::TilePos;
use hf_sim::world::gen::biome::Biome;
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

/// The water in a column: its top tile and how many levels deep it is.
fn column(w: &World, x: i32, y: i32) -> Option<(i32, i32)> {
    let g = &w.grid;
    let bottom = g.ground_z(x, y) + 1;
    if !g.get(TilePos::new(x, y, bottom)).is_water() {
        return None;
    }
    let mut top = bottom;
    while g.get(TilePos::new(x, y, top + 1)).is_water() {
        top += 1;
    }
    Some((top, top - bottom + 1))
}

/// Chebyshev distance from a water column to the nearest dry one (the map's edge is dry).
fn to_shore(w: &World, x: i32, y: i32) -> i32 {
    let g = &w.grid;
    let dry = |x: i32, y: i32| {
        x < 0 || y < 0 || x >= g.width || y >= g.height || column(w, x, y).is_none()
    };
    (1..=6)
        .find(|r| (-r..=*r).any(|dy| (-r..=*r).any(|dx| dry(x + dx, y + dy))))
        .unwrap_or(6)
}

#[test]
fn a_lake_is_deep_in_the_middle_and_one_level_at_its_edge() {
    let (mut middles, mut deep_middles, mut deepest) = (0, 0, 0);
    for seed in 1..=8u64 {
        let w = World::generate(seed, GenParams::default());
        let g = &w.grid;
        for y in 0..g.height {
            for x in 0..g.width {
                let Some((_, depth)) = column(&w, x, y) else {
                    continue;
                };
                let d = to_shore(&w, x, y);
                if d == 1 {
                    assert_eq!(
                        depth, 1,
                        "seed {seed}: a shore column ({x},{y}) is one deep"
                    );
                }
                if g.biome_at(x, y) == Some(Biome::Fen) {
                    assert_eq!(depth, 1, "seed {seed}: fen water is shallow at ({x},{y})");
                    continue;
                }
                if d >= 3 {
                    middles += 1;
                    if depth >= 3 {
                        deep_middles += 1;
                    }
                }
                deepest = deepest.max(depth);
            }
        }
    }
    assert!(middles > 20, "lakes with a middle: {middles} columns");
    // Carving stops above a cave, so not every middle column reaches its full depth.
    assert!(
        deep_middles * 10 >= middles * 8,
        "{deep_middles} of {middles} middle columns are three or more deep"
    );
    assert!(deepest >= 3, "the deepest water is {deepest} levels");
}

#[test]
fn a_fen_is_shallow_water_over_waterlogged_ground() {
    let mut seen = 0;
    for seed in 1..=24u64 {
        let p = GenParams {
            landform: hf_sim::world::gen::Landform::Marsh,
            ..GenParams::small()
        };
        let w = World::generate(seed, p);
        let g = &w.grid;
        for y in 0..g.height {
            for x in 0..g.width {
                if g.biome_at(x, y) != Some(Biome::Fen) {
                    continue;
                }
                let Some((top, depth)) = column(&w, x, y) else {
                    continue;
                };
                assert_eq!(depth, 1);
                let under = g.get(TilePos::new(x, y, top - 1));
                assert_eq!(
                    under.material,
                    Material::WET_SOIL,
                    "waterlogged at ({x},{y})"
                );
                seen += 1;
            }
        }
    }
    assert!(seen > 0, "some fen water to look at");
}
