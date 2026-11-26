//! #352: not every landing is a valley.

use hf_sim::world::gen::{generate, GenParams, Landform};

fn mean_relief(p: &GenParams, seed: u64) -> (f32, usize) {
    let g = generate(seed, p);
    let n = (p.width * p.height) as usize;
    let mean = g.grid.ground.iter().map(|z| *z as f32).sum::<f32>() / n as f32;
    let water = (0..p.height)
        .flat_map(|y| (0..p.width).map(move |x| (x, y)))
        .filter(|(x, y)| {
            let z = g.grid.ground_z(*x, *y) + 1;
            g.grid.get(hf_sim::ids::TilePos::new(*x, *y, z)).is_water()
        })
        .count();
    (mean, water)
}

#[test]
fn the_lie_of_the_land_shapes_the_ground() {
    let base = GenParams::small();
    let at = |lf| GenParams {
        landform: lf,
        ..GenParams::small()
    };
    let (valley, vw) = mean_relief(&base, 11);
    let (plateau, _) = mean_relief(&at(Landform::Plateau), 11);
    let (marsh, mw) = mean_relief(&at(Landform::Marsh), 11);
    assert!(
        plateau > valley,
        "a plateau stands higher: {plateau} vs {valley}"
    );
    assert!(marsh < valley, "a marsh lies lower: {marsh} vs {valley}");
    assert!(mw >= vw, "and wetter: {mw} vs {vw} tiles of water");
    // Openings draw a mix.
    let kinds: std::collections::BTreeSet<String> = (0..60u64)
        .map(|s| format!("{:?}", Landform::of_seed(s)))
        .collect();
    assert!(kinds.len() >= 4, "{kinds:?}");
}

#[test]
fn every_landform_has_somewhere_to_land() {
    use std::collections::BTreeSet;
    let mut done = BTreeSet::new();
    for seed in 0..80u64 {
        let lf = Landform::of_seed(seed);
        if !done.insert(format!("{lf:?}")) {
            continue;
        }
        let w = hf_sim::World::opening_world(seed);
        let home = w.communities[0].home;
        assert!(
            w.grid.walkable(home),
            "{lf:?} (seed {seed}): a landing to stand on"
        );
        assert!(
            w.agents
                .iter()
                .filter(|a| a.alive && a.community.is_some())
                .count()
                >= 3,
            "{lf:?}: the party is there"
        );
    }
    assert!(done.len() >= 5, "{done:?}");
}

#[test]
fn an_inland_taiga_landing_is_not_a_seacoast() {
    use hf_sim::setup::{preview, WorldSetup};
    let mut setup = WorldSetup::new(7);
    setup.region = Some(16423);
    let world = hf_sim::World::from_setup(&setup);
    assert_ne!(world.meta.params.landform, Landform::Coast);
    assert!(world.meta.params.ocean.is_empty());
    let picture = preview(7, None, setup.region);
    assert_eq!(picture.region, 16423);
    assert_eq!(world.meta.founding_site, picture.founding);
}

#[test]
fn the_sea_appears_on_the_side_of_the_ocean() {
    use hf_sim::realm::{self, embark, RBiome, RealmParams};
    use hf_sim::setup::{preview_in, WorldSetup};
    let seed = 7;
    let map = realm::generate(seed, &RealmParams::default());
    let site = (0..map.biome.len())
        .find(|&i| {
            let (x, y) = map.xy(i);
            x > 0
                && x + 1 < map.size
                && y > 0
                && y + 1 < map.size
                && embark::habitable(&map, i)
                && map.biome[map.idx(x + 1, y)] == RBiome::Ocean
                && (y - 1..=y + 1).all(|yy| map.biome[map.idx(x - 1, yy)] != RBiome::Ocean)
        })
        .unwrap();
    let mut setup = WorldSetup::new(seed);
    setup.region = Some(site);
    let world = hf_sim::World::from_setup(&setup);
    assert_eq!(world.meta.params.landform, Landform::Coast);
    let picture = preview_in(seed, &map, None, Some(site));
    assert_eq!(picture.founding, world.meta.founding_site);
    let w = world.grid.width as usize;
    let h = world.grid.height as usize;
    let wet = |x| {
        (h / 4..3 * h / 4)
            .filter(|&y| picture.columns[y * w + x] == 255)
            .count()
    };
    assert_eq!(wet(w - 1), h / 2, "sea along the east edge");
    assert!(wet(0) < h / 2, "inland ground on the opposite edge");
}
