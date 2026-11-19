//! #268's roughness row (WP-05 slice 5.6): rough ground underfoot is slower going, so a path
//! goes round a scree slope where there is a way round, and crosses it where there is not.

use hf_sim::ids::TilePos;
use hf_sim::nav::{find_path, step_cost, AStarScratch, PathResult};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

fn meadow() -> World {
    let mut w = World::generate(2681, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.rebuild_caches();
    w
}

/// Lay `m` as the ground under the surface tile at (x, y).
fn ground(w: &mut World, x: i32, y: i32, m: Material) {
    let s = w.grid.surface(x, y);
    w.grid.set(
        s.offset(0, 0, -1),
        Tile {
            material: m,
            shape: Shape::Wall,
            flags: TileFlags::NONE,
        },
    );
}

#[test]
fn rough_ground_costs_more_and_a_worn_path_less() {
    let mut w = meadow();
    let at = |w: &World, x| step_cost(&w.grid, w.grid.surface(x, 30), false);
    let plain = at(&w, 10);
    for (x, m) in [
        (11, Material::SCREE),
        (12, Material::MUD),
        (13, Material::GRAVEL),
        (14, Material::SAND),
        (15, Material::PACKED_EARTH),
    ] {
        ground(&mut w, x, 30, m);
    }
    let (scree, mud, gravel, sand, path) =
        (at(&w, 11), at(&w, 12), at(&w, 13), at(&w, 14), at(&w, 15));
    assert!(
        scree > mud && mud > gravel && gravel > plain,
        "{scree} {mud} {gravel} {plain}"
    );
    assert_eq!(gravel, sand);
    assert!(path < plain, "a worn path is still the quick way");
}

#[test]
fn a_path_goes_round_a_scree_band_where_it_can_and_over_it_where_it_cannot() {
    let mut w = meadow();
    // A band of scree three wide at x = 29..31 across the whole flattened ground (y = 2..62):
    // no way round on the flat.
    for y in 2..62 {
        for x in 29..=31 {
            ground(&mut w, x, y, Material::SCREE);
        }
    }
    w.rebuild_caches();
    let (a, b) = (w.grid.surface(24, 22), w.grid.surface(36, 22));
    let mut scratch = AStarScratch::new(&w.grid);
    let on_scree = |w: &World, path: &[TilePos]| {
        path.iter()
            .filter(|p| w.grid.get(p.offset(0, 0, -1)).material == Material::SCREE)
            .count()
    };
    // No way round: the path crosses, scree and all.
    let PathResult::Found(over) = find_path(&w.grid, &mut scratch, a, b, 100_000) else {
        panic!("no path at all");
    };
    assert!(
        on_scree(&w, &over) > 0,
        "a long way round was taken over a short scramble"
    );
    // Now the band is only rows 20..23 long: the way round is a few steps and is taken.
    for y in 24..40 {
        for x in 29..=31 {
            ground(&mut w, x, y, Material::SOIL);
        }
    }
    w.rebuild_caches();
    let PathResult::Found(round) = find_path(&w.grid, &mut scratch, a, b, 100_000) else {
        panic!("no path at all");
    };
    assert_eq!(
        on_scree(&w, &round),
        0,
        "the path went over scree with a way round beside it"
    );
}
