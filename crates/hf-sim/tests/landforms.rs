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
