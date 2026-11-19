//! Issue #267: the deep differs by region (basalt, gabbro, gneiss under the slate), and coal
//! lies in flat seams through the sandstone, at one depth across a valley.

use hf_sim::world::gen::{strata, GenParams};
use hf_sim::world::Material;
use hf_sim::World;
use std::collections::BTreeSet;

#[test]
fn the_deep_differs_by_region_and_seams_lie_flat() {
    let p = GenParams::default();
    let (_, deep) = strata::breaks(&p);
    let mut deeps = BTreeSet::new();
    let mut seamed = 0;
    for seed in 1..7u64 {
        let w = World::generate(seed, p.clone());
        let g = &w.grid;
        let mut seam_zs = BTreeSet::new();
        for y in (0..p.height).step_by(4) {
            for x in (0..p.width).step_by(4) {
                let c = g.col(x, y);
                let m = g.rock_at(c, deep - 2);
                deeps.insert(m);
                if m == Material::GNEISS {
                    assert_eq!(g.strata[c].0, Material::SLATE, "gneiss lies under slate");
                }
                for z in deep..g.ground_z(x, y) {
                    if g.rock_at(c, z) == Material::COAL {
                        assert_eq!(g.bedrock[c], Material::SANDSTONE, "seams are in sandstone");
                        seam_zs.insert(z);
                    }
                }
            }
        }
        assert!(seam_zs.len() <= 1, "seed {seed}: seams at {seam_zs:?}");
        seamed += seam_zs.len();
    }
    assert!(seamed >= 2, "{seamed} worlds with a seam");
    assert!(deeps.contains(&Material::BASALT), "{deeps:?}");
    assert!(
        deeps.contains(&Material::GABBRO) && deeps.contains(&Material::GNEISS),
        "{deeps:?}"
    );
}

use hf_sim::ids::TilePos;
use hf_sim::world::gen::Landform;
use hf_sim::world::material::{gem_in, MaterialKind, MaterialTable};

/// #267: gold lies in the deep whatever the region's rock there, not only in basalt.
#[test]
fn gold_lies_in_the_deep_whatever_its_rock() {
    let p = GenParams::default();
    let (_, deep) = strata::breaks(&p);
    let mut hosts = BTreeSet::new();
    for seed in 1..=16u64 {
        let w = World::generate(seed, p.clone());
        let g = &w.grid;
        for z in g.z_min..=deep + 1 {
            for y in 0..g.height {
                for x in 0..g.width {
                    if g.get(TilePos::new(x, y, z)).material == Material::GOLD_ORE {
                        let c = g.col(x, y);
                        assert!(
                            z <= g.strata[c].2 as i32 + 1,
                            "seed {seed}: gold above the deep"
                        );
                        hosts.insert(g.rock_at(c, z));
                    }
                }
            }
        }
        // And none above it.
        for z in deep + 2..=g.z_max {
            for y in 0..g.height {
                for x in 0..g.width {
                    assert_ne!(g.get(TilePos::new(x, y, z)).material, Material::GOLD_ORE);
                }
            }
        }
    }
    assert!(
        hosts.iter().any(|m| *m != Material::BASALT),
        "gold in a deep of gabbro or gneiss somewhere: {hosts:?}"
    );
}

/// #267: which gem a pocket holds follows its rock: the gneiss mostly garnets, the gabbro more
/// sapphires than the basalt.
#[test]
fn gems_follow_the_rock() {
    let t = MaterialTable::default();
    let count = |host, gem| {
        (0..400)
            .filter(|x| gem_in(5, TilePos::new(*x, 7, -12), host) == gem)
            .count()
    };
    for host in [Material::BASALT, Material::GABBRO, Material::GNEISS] {
        assert_eq!(
            t.get(gem_in(5, TilePos::new(1, 1, -12), host)).kind,
            MaterialKind::Gem
        );
    }
    assert!(count(Material::GNEISS, Material::GARNET) > count(Material::BASALT, Material::GARNET));
    assert!(
        count(Material::GABBRO, Material::SAPPHIRE) > count(Material::BASALT, Material::SAPPHIRE)
    );
}

/// #267: the water lies in the porous sandstone and limestone, never in the granite, slate,
/// marble or the deep; and the same rock is wet or dry by where the layers fold, not by chance.
#[test]
fn aquifers_lie_in_the_porous_rock() {
    let mut wet_tiles = 0;
    for seed in 1..=4u64 {
        let mut w = World::generate(seed, GenParams::default());
        w.found_party(1);
        let g = &w.grid;
        for y in (0..g.height).step_by(3) {
            for x in (0..g.width).step_by(3) {
                let c = g.col(x, y);
                for z in g.z_min..g.ground_z(x, y) {
                    if hf_sim::aquifer::wet(&w, TilePos::new(x, y, z)) {
                        wet_tiles += 1;
                        let rock = g.rock_at(c, z);
                        assert!(
                            [Material::SANDSTONE, Material::LIMESTONE].contains(&rock),
                            "seed {seed}: water in {rock:?}"
                        );
                        assert!(z > g.strata[c].1 as i32, "above the middle stratum");
                    }
                }
            }
        }
    }
    assert!(wet_tiles > 50, "{wet_tiles} wet tiles");
}

/// #267 (and #268's soil depth): soil lies three deep on the level ground of a plain, one deep
/// on a slope, with the rock beneath.
#[test]
fn soil_is_deeper_on_level_low_ground() {
    let p = GenParams {
        landform: Landform::Plain,
        ..GenParams::default()
    };
    let w = World::generate(3, p);
    let g = &w.grid;
    // Counted, not tile by tile: rivers and ramps cut the ground after the soil is laid.
    let (mut level, mut level3, mut slope, mut slope1) = (0, 0, 0, 0);
    for y in 2..g.height - 2 {
        for x in 2..g.width - 2 {
            let z = g.ground_z(x, y);
            let flat = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .all(|(dx, dy)| g.ground_z(x + dx, y + dy) == z);
            let c = g.col(x, y);
            let d = g.soil_depth(c);
            let under = g.synthesized(TilePos::new(x, y, z - d)).material;
            assert_ne!(under, Material::SOIL, "({x},{y}) rock under the soil");
            if d > 1 {
                let soil = g.synthesized(TilePos::new(x, y, z - d + 1)).material;
                assert_eq!(soil, Material::SOIL, "({x},{y}) soil {d} deep");
            }
            if flat {
                level += 1;
                level3 += (d == 3) as u32;
            } else {
                slope += 1;
                slope1 += (d == 1) as u32;
            }
        }
    }
    assert!(
        level > 100 && level3 * 10 >= level * 9,
        "{level3} of {level} level at 3"
    );
    assert!(
        slope > 0 && slope1 * 10 >= slope * 8,
        "{slope1} of {slope} sloping at 1"
    );
}
