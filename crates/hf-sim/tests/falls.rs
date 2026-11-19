//! #324: a drop is not flat ground. Someone with the floor gone from under them falls to what
//! lies below and is hurt by the height. #444: the harm grows steeply with the height (a level
//! is a storey), deep water softens it, and nobody falls through rock.

use hf_sim::agents::Species;
use hf_sim::ids::TilePos;
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

fn ground(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    w
}

#[test]
fn the_floor_dug_away_under_someone_drops_them_and_hurts_them() {
    let mut w = ground(3241);
    let ids = w.found_party(1);
    w.meta.autoplan = false;
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let at = w.agents[i].pos;
    w.agents[i].path.clear();
    w.agents[i].move_total = 0;
    w.agents[i].move_ticks = 0;
    // Two levels of ground under them dug out.
    w.grid.set(at.offset(0, 0, -1), Tile::AIR);
    w.grid.set(at.offset(0, 0, -2), Tile::AIR);
    assert!(w.grid.walkable(at.offset(0, 0, -2)));
    let before = w.agents[i].body.wounds.len();
    hf_sim::systems::movement::step(&mut w);
    let a = &w.agents[i];
    assert_eq!(a.pos, at.offset(0, 0, -2), "down to the floor below");
    assert!(
        a.body.wounds.len() > before || a.death.as_deref() == Some("fell"),
        "hurt by the fall"
    );
}

/// Forty people at once, each over a shaft `levels` deep (with `water` levels of deep water at
/// its foot): how many the fall kills, and the worst wound among the living.
fn drop(seed: u64, levels: i32, water: i32) -> (usize, Vec<hf_sim::body::Wound>) {
    let mut w = ground(seed);
    w.meta.autoplan = false;
    let mut idx = Vec::new();
    for k in 0..40 {
        let (x, y) = (6 + (k % 10) * 5, 6 + (k / 10) * 5);
        for z in (2 - levels)..=1 {
            w.grid.set(TilePos::new(x, y, z), Tile::AIR);
        }
        for z in (2 - levels)..(2 - levels + water) {
            w.grid.set(
                TilePos::new(x, y, z),
                Tile {
                    material: Material::WATER,
                    shape: Shape::Open,
                    flags: TileFlags(TileFlags::WATER | TileFlags::DEEP),
                },
            );
        }
        let id = w.spawn_agent(Species::Person, TilePos::new(x, y, 2));
        idx.push(w.agents.iter().position(|a| a.id == id).unwrap());
    }
    w.rebuild_caches();
    hf_sim::systems::movement::step(&mut w);
    let dead = idx
        .iter()
        .filter(|i| w.agents[**i].death.as_deref() == Some("fell"))
        .count();
    let wounds = idx
        .iter()
        .filter(|i| w.agents[**i].alive)
        .flat_map(|i| w.agents[*i].body.wounds.clone())
        .collect();
    (dead, wounds)
}

#[test]
fn a_fall_hurts_steeply_by_the_height() {
    use hf_sim::body::WoundKind;
    let (dead1, w1) = drop(4441, 1, 0);
    assert_eq!(dead1, 0, "a storey kills nobody");
    assert!(
        w1.iter().all(|wd| wd.severity < 0.5),
        "a storey bruises or sprains: {w1:?}"
    );
    let (dead2, w2) = drop(4442, 2, 0);
    assert!(dead2 <= 6, "two storeys kill few: {dead2} of 40");
    assert!(
        w2.iter().filter(|wd| wd.kind == WoundKind::Break).count() >= 15,
        "two storeys break bones"
    );
    let (dead3, _) = drop(4443, 3, 0);
    assert!(
        (3..=20).contains(&dead3),
        "three storeys kill some: {dead3} of 40"
    );
    let (dead4, _) = drop(4444, 4, 0);
    assert!(dead4 >= 16, "four storeys kill most: {dead4} of 40");
    let (dead7, _) = drop(4447, 7, 0);
    assert!(dead7 >= 34, "seven storeys kill nearly all: {dead7} of 40");
}

#[test]
fn deep_water_softens_a_fall() {
    let (dead, wounds) = drop(4448, 3, 2);
    assert_eq!(dead, 0, "into deep water from three storeys");
    assert!(wounds.is_empty(), "unhurt: {wounds:?}");
    let (dead7, _) = drop(4449, 7, 2);
    let (dry7, _) = drop(4449, 7, 0);
    assert!(
        dead7 < dry7,
        "from seven, water still saves some: {dead7} against {dry7}"
    );
}

#[test]
fn nobody_falls_through_rock() {
    // #444: on seed 4 a grass fire caught the tile three people stood on; the fall code took the
    // unwalkable tile for a missing floor and dropped them through six levels of soil and slate
    // into a closed cave.
    let mut w = ground(4450);
    w.meta.autoplan = false;
    let at = TilePos::new(30, 30, 2);
    for y in 28..=32 {
        for x in 28..=32 {
            w.grid.set(TilePos::new(x, y, -3), Tile::AIR);
        }
    }
    let id = w.spawn_agent(Species::Person, at);
    let i = w.agents.iter().position(|a| a.id == id).unwrap();
    let mut t = w.grid.get(at);
    t.flags = t.flags.with(TileFlags::FIRE);
    w.grid.set(at, t);
    w.rebuild_caches();
    assert!(w.grid.walkable(at.offset(0, 0, -5)), "a cave under them");
    hf_sim::systems::movement::step(&mut w);
    let a = &w.agents[i];
    assert!(a.alive);
    assert_eq!(
        a.pos.z, at.z,
        "stepped off the fire, not down through the rock"
    );
    assert_ne!(a.pos, at);
}
