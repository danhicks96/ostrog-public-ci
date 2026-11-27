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

#[test]
fn one_upper_bank_scout_can_watch_only_the_exposed_water_face() {
    use hf_sim::expedition::survey::observe;
    let mut setup = WorldSetup::new(7);
    setup.region = Some(28317);
    setup.history_years = 0;
    setup.realm_years = 200;
    let mut w = World::from_setup(&setup);
    let c = w.communities[0].id;
    let scout = w
        .agents
        .iter()
        .position(|a| {
            a.community == Some(c) && a.alive && !hf_sim::view::known::is_unfound(&w, a.id)
        })
        .unwrap();
    let home = w.agents[scout].pos;
    let shore = TilePos::new(56, 75, 3);
    let edge = SpillEdge {
        upper: TilePos::new(59, 76, 3),
        lower: TilePos::new(59, 77, 2),
    };
    let lower_key = (edge.lower.x, edge.lower.y, edge.lower.z);
    let seen = |w: &World| w.overworld.surveys[&c].sees_spill(edge, 3);
    w.agents[scout].pos = shore;
    let person = w.agents[scout].id;
    let foreign = w.found_community(shore);
    w.join_community(person, foreign);
    observe(&mut w);
    assert!(!seen(&w));
    assert!(w.overworld.surveys[&foreign].sees_spill(edge, 3));
    w.join_community(person, c);
    let previous = w.survivor_pool[0].state;
    w.survivor_pool[0].state = hf_sim::opening::SurvivorState::Lost {
        agent: person,
        found: false,
    };
    observe(&mut w);
    assert!(!seen(&w));
    w.survivor_pool[0].state = previous;
    observe(&mut w);
    assert!(seen(&w));
    assert!(!w.overworld.surveys[&c].sees_spill(edge, 2));
    assert!(!w.overworld.surveys[&c].valley.contains_key(&lower_key));
    assert!(!w.overworld.surveys[&c].sees_spill(edge, 1));

    // The last report survives an unseen change above the crest. A fresh look at
    // the exposed face correctly records that it has stopped moving.
    let cap = edge.upper.offset(0, 0, 1);
    let original = w.grid.get(cap);
    w.grid.set(cap, w.grid.get(edge.upper));
    assert!(!w.active_river_spills().any(|e| e == edge));
    assert!(w.overworld.surveys[&c].bank_spills.contains(&edge));
    observe(&mut w);
    assert!(!w.overworld.surveys[&c].bank_spills.contains(&edge));
    w.grid.set(cap, original);
    observe(&mut w);
    assert!(w.overworld.surveys[&c].bank_spills.contains(&edge));

    // An exposed moving face disappears even between surveys when its lip ices over.
    // The pool, one level lower, is still liquid at this temperature.
    let warm = w.weather.temperature_c;
    w.weather.temperature_c += hf_sim::water::ICE_C - w.temp_at(edge.upper) - 0.5;
    assert!(hf_sim::water::frozen_at(&w, edge.upper));
    assert!(!hf_sim::water::frozen_at(&w, edge.lower));
    assert!(!w.active_river_spills().any(|e| e == edge));
    w.weather.temperature_c -= 10.0;
    assert!(hf_sim::water::frozen_at(&w, edge.lower));
    observe(&mut w);
    assert!(!seen(&w));
    w.weather.temperature_c = warm;
    observe(&mut w);
    assert!(w.active_river_spills().any(|e| e == edge) && seen(&w));
    assert!(!w.overworld.surveys[&c].sees_spill(edge, 2));

    // A wall in the horizontal part of the over-bank ray blocks the pool.
    let beside = TilePos::new(57, 75, 3);
    let saved = w.grid.get(beside);
    w.grid.set(beside, Tile::solid(Material::GRANITE));
    observe(&mut w);
    assert!(!seen(&w));
    w.grid.set(beside, saved);
    observe(&mut w);
    assert!(seen(&w));

    // Even with light at both endpoints, a deck or roof directly over the pool blocks it.
    let cover = edge.lower.offset(0, 0, 1);
    let saved = w.grid.get(cover);
    w.grid.set(cover, Tile::floor(Material::WOOD));
    observe(&mut w);
    assert!(!seen(&w));
    w.grid.set(cover, saved);
    w.roofs.insert(
        edge.lower,
        hf_sim::building::Roof {
            material: Material::WOOD,
            quality: 1.0,
            built: 0,
            snow: 0.0,
        },
    );
    observe(&mut w);
    assert!(!seen(&w));
    w.roofs.remove(&edge.lower);
    observe(&mut w);
    assert!(seen(&w));

    // Enclosing the observer under rock cannot add a surface report from underground.
    let overhead = shore.offset(0, 0, 1);
    let saved = w.grid.get(overhead);
    w.grid.set(overhead, Tile::solid(Material::GRANITE));
    observe(&mut w);
    assert!(!seen(&w));
    w.grid.set(overhead, saved);

    // A diagonal sightline touches this upper tile's corner on its way to the pool.
    w.agents[scout].pos = TilePos::new(60, 82, 3);
    observe(&mut w);
    assert!(seen(&w));
    let corner = TilePos::new(59, 80, 3);
    let saved = w.grid.get(corner);
    w.grid.set(corner, Tile::solid(Material::GRANITE));
    observe(&mut w);
    assert!(!seen(&w));
    w.grid.set(corner, saved);

    // The closer upstream angle looks through the crest, which hides the plunge pool.
    w.agents[scout].pos = TilePos::new(58, 74, 3);
    observe(&mut w);
    assert!(!seen(&w));

    // A returning scout leaves no live report of the water or its occupants.
    w.agents[scout].pos = home;
    w.meta.tick += 1;
    observe(&mut w);
    assert!(!seen(&w));
    assert!(!w.overworld.surveys[&c].valley.contains_key(&lower_key));
}

#[test]
fn separate_witnesses_can_still_reveal_the_fall_on_both_floors() {
    let edge = SpillEdge {
        upper: TilePos::new(8, 8, 3),
        lower: TilePos::new(8, 9, 2),
    };
    let mut survey = hf_sim::expedition::survey::Survey {
        tick: 100,
        ..Default::default()
    };
    survey.valley.insert((8, 8, 3), (100, 0, 0, -1));
    assert!(!survey.sees_spill(edge, 3));
    survey.valley.insert((8, 9, 2), (100, 0, 0, -1));
    assert!(survey.sees_spill(edge, 2));
    assert!(survey.sees_spill(edge, 3));
    assert!(!survey.sees_spill(edge, 4));
    survey.tick += 1;
    assert!(!survey.sees_spill(edge, 3));
}
