//! #725: water two levels deep beside the bank (a steep-sided river: the surveyed surface sits one
//! level above where a person stands) still gives a place to drink. Before, `water::spots` found
//! no stand beside the surface tile, so a thirsty band far from the water was never sent to it and
//! lived or died by wandering next to it by luck.

use hf_sim::ids::TilePos;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

fn water_floor() -> Tile {
    Tile {
        material: Material::WATER,
        shape: Shape::Floor,
        flags: TileFlags(TileFlags::WATER),
    }
}

#[test]
fn a_steep_sided_river_is_a_place_to_drink() {
    let mut w = World::generate(4, GenParams::default());
    w.flatten(0, 0, 128, 128, 1);
    w.plants.clear();
    for y in 70..72 {
        for x in 0..=127 {
            w.grid.set(TilePos::new(x, y, 2), water_floor());
            w.grid.set(TilePos::new(x, y, 3), water_floor());
        }
    }
    w.rebuild_caches();
    hf_sim::floods::survey(&mut w);
    let from = TilePos::new(111, 23, 2);
    let spots = hf_sim::water::spots(&w, from, 60);
    assert!(
        !spots.is_empty(),
        "no place to drink within 60 tiles of {from} beside a river 47 tiles away"
    );
    let first = &spots[0];
    assert!(
        hf_sim::water::reaches(first.stand, first.source),
        "stand {} does not reach {}",
        first.stand,
        first.source
    );
}
