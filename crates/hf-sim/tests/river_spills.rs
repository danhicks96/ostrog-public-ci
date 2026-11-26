use hf_sim::ids::TilePos;
use hf_sim::setup::WorldSetup;
use hf_sim::world::gen::watershed::SpillEdge;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags, WorldGrid};
use hf_sim::World;

#[test]
fn a_generated_drop_keeps_its_direction_and_uses_exposed_surfaces() {
    let mut setup = WorldSetup::new(7);
    setup.history_years = 0;
    setup.realm_years = 200;
    setup.region = Some(28317);
    let mut w = World::from_setup(&setup);
    let map = w.realm_map.as_ref().unwrap();
    let next = map.down[28317];
    assert_eq!(next, 28508);
    assert_eq!(map.xy(next), (map.xy(28317).0 - 1, map.xy(28317).1 + 1));
    let outlet = hf_sim::floods::top_water(&w.grid, 0, w.grid.height - 1).unwrap();
    setup.region = Some(next);
    let downstream = World::from_setup(&setup);
    assert_eq!(downstream.meta.setup.as_ref().unwrap().region, Some(next));
    let inlet = hf_sim::floods::top_water(&downstream.grid, downstream.grid.width - 1, 0).unwrap();
    assert_eq!(outlet.z, inlet.z);
    let drop = SpillEdge {
        upper: TilePos::new(59, 76, 3),
        lower: TilePos::new(59, 77, 2),
    };
    assert!(w.active_river_spills().any(|edge| edge == drop));
    assert!(w
        .active_river_spills()
        .all(|edge| edge.upper.z > edge.lower.z
            && (edge.upper.x - edge.lower.x).abs() + (edge.upper.y - edge.lower.y).abs() == 1));

    // Per-tile temperatures let the lip freeze first, while the pool still holds liquid.
    let initial_temperature = w.weather.temperature_c;
    w.weather.temperature_c += hf_sim::water::ICE_C - w.temp_at(drop.upper) - 0.5;
    assert!(hf_sim::water::frozen_at(&w, drop.upper));
    assert!(!hf_sim::water::frozen_at(&w, drop.lower));
    assert!(!w.active_river_spills().any(|edge| edge == drop));
    w.weather.temperature_c -= 10.0;
    assert!(hf_sim::water::frozen_at(&w, drop.lower));
    assert!(!w.active_river_spills().any(|edge| edge == drop));
    w.weather.temperature_c = initial_temperature;
    assert!(w.active_river_spills().any(|edge| edge == drop));

    // Deepening the lower column never changes the reported surface.
    let water = Tile {
        material: Material::WATER,
        shape: Shape::Floor,
        flags: TileFlags(TileFlags::DEEP),
    };
    w.grid.set(drop.lower.offset(0, 0, -1), water);
    assert!(w.active_river_spills().any(|edge| edge == drop));

    // A spate submerges the lower lip and removes this face until the spate recedes.
    w.grid.set(drop.lower.offset(0, 0, 1), water);
    assert!(!w.active_river_spills().any(|edge| edge == drop));
    w.grid.set(drop.lower.offset(0, 0, 1), Tile::AIR);
    w.flooded.insert(drop.lower);
    assert!(!w.active_river_spills().any(|edge| edge == drop));
    w.flooded.remove(&drop.lower);
    assert!(w.active_river_spills().any(|edge| edge == drop));
    w.grid.set(drop.upper, Tile::AIR);
    assert!(!w.active_river_spills().any(|edge| edge == drop));
}

#[test]
fn old_grids_and_adjacent_lake_water_have_no_generated_spills() {
    let mut w = World::generate(2, GenParams::small());
    assert!(w.grid.river_spills.is_empty());
    let mut grid = WorldGrid::new(2, 1, -3, 3);
    grid.ground[0] = 1;
    for x in 0..2 {
        let water = Tile {
            material: Material::WATER,
            shape: Shape::Floor,
            flags: TileFlags(TileFlags::WATER),
        };
        grid.set(grid.surface(x, 0), water);
    }
    // Named-field saves before spill provenance have the same encoding as an empty list today.
    let bytes = rmp_serde::to_vec_named(&grid).unwrap();
    assert!(!bytes
        .windows(b"river_spills".len())
        .any(|w| w == b"river_spills"));
    w.grid = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(w.active_river_spills().count(), 0);
}
