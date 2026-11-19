//! C59 (D28): the bone forest grows only above a deep cavern where the underworld's stone shows,
//! and its wood and stone are their own.

use hf_sim::plants::PlantKind;
use hf_sim::world::gen::biome::Biome;
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

#[test]
fn the_biome_arises_from_the_world() {
    let (mut with, mut without) = (0, 0);
    for seed in 0..16u64 {
        let w = World::generate(seed, GenParams::default());
        let under: Vec<_> = w.grid.caverns.iter().filter(|c| c.underworld).collect();
        let bone: Vec<(i32, i32)> = (0..w.grid.height)
            .flat_map(|y| (0..w.grid.width).map(move |x| (x, y)))
            .filter(|&(x, y)| w.grid.biome[w.grid.col(x, y)] == Biome::BoneForest)
            .collect();
        if under.is_empty() {
            assert!(
                bone.is_empty(),
                "seed {seed}: a bone forest with no underworld beneath"
            );
            without += 1;
            continue;
        }
        with += 1;
        assert!(
            !bone.is_empty(),
            "seed {seed}: the underworld's stone lies beneath and nothing grew"
        );
        for (x, y) in &bone {
            assert!(
                under
                    .iter()
                    .any(|c| (x - c.centre.x).abs() <= c.rx + 6
                        && (y - c.centre.y).abs() <= c.ry + 6),
                "seed {seed}: bone forest at {x},{y} away from any underworld cavern"
            );
        }
        // Its trees are bone trees, and under it the rock is milkstone.
        let trees: Vec<_> = w
            .plants
            .iter()
            .filter(|p| {
                bone.contains(&(p.pos.x, p.pos.y))
                    && p.kind.yields_wood()
                    && p.pos.z > w.grid.ground_z(p.pos.x, p.pos.y)
            })
            .collect();
        assert!(
            trees.iter().all(|p| p.kind == PlantKind::BoneTree),
            "seed {seed}: other trees in it"
        );
        let (x, y) = bone[bone.len() / 2];
        let gz = w.grid.ground_z(x, y);
        let below = (gz - 8..gz).any(|z| {
            w.grid.get(hf_sim::ids::TilePos::new(x, y, z)).material == Material::MILKSTONE
        });
        assert!(
            below,
            "seed {seed}: no milkstone under the bone forest at {x},{y}"
        );
    }
    assert!(
        with > 0 && without > 0,
        "some worlds have it and some do not: {with} with, {without} without"
    );
}

#[test]
fn bone_trees_give_bonewood() {
    assert_eq!(
        hf_sim::world::material::wood_of(PlantKind::BoneTree, 0),
        Material::BONEWOOD
    );
    let w = World::generate(1, GenParams::small());
    let m = w.grid.materials.get(Material::MILKSTONE);
    assert_eq!(
        m.yields,
        Some(Material::STONE_BLOCK),
        "milkstone cuts to blocks"
    );
    assert!(
        w.grid.materials.get(Material::BONEWOOD).density
            > w.grid.materials.get(Material::OAK).density
    );
}
