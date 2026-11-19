use hf_sim::environment::worksite_clues;
use hf_sim::ids::TilePos;
use hf_sim::structures::StructureKind;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

fn water_tile() -> Tile {
    Tile {
        material: Material::WATER,
        shape: Shape::Open,
        flags: TileFlags(TileFlags::WATER),
    }
}

#[test]
fn worksite_clues_report_dampness_and_respect_the_existing_window() {
    let mut w = World::generate(161, GenParams::small());
    let worksite = TilePos::new(20, 20, w.grid.ground_z(20, 20) - 1);
    let water = worksite.offset(2, 0, 0);
    w.grid.set(worksite, Tile::solid(Material::GRANITE));
    w.grid.set(water, water_tile());

    assert!(worksite_clues(&w, worksite).damp);
    assert!(!worksite_clues(&w, worksite.offset(5, 0, 0)).damp);
}

fn rock_column(upper: Material) -> (World, TilePos) {
    let mut w = World::generate(5, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    let at = TilePos::new(20, 20, -3);
    let c = w.grid.col(at.x, at.y);
    w.grid.bedrock[c] = upper;
    w.grid.strata[c] = (Material::GRANITE, -4, -8);
    w.grid.deep[c] = (Material::BASALT, i8::MIN);
    w.grid.soil[c] = 1;
    w.grid.set(at, Tile::solid(upper));
    (w, at)
}

#[test]
fn worksite_clues_distinguish_water_bearing_limestone_from_dry_granite() {
    // Both columns have ground z1, one soil level and a basin break at z-4. At z-3 the
    // limestone is its water-bearing level; the otherwise identical granite holds no water.
    let (wet_world, wet) = rock_column(Material::LIMESTONE);
    let (dry_world, dry) = rock_column(Material::GRANITE);

    assert!(worksite_clues(&wet_world, wet).water_bearing);
    assert!(!worksite_clues(&dry_world, dry).water_bearing);
}

fn support_removed_gallery() -> (World, TilePos) {
    let mut w = World::generate(5, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    let cut = TilePos::new(49, 30, -2);
    for x in 42..49 {
        for y in 28..33 {
            let p = TilePos::new(x, y, -2);
            w.grid.set(p, Tile::AIR);
            w.dug.insert(p);
            w.grid
                .set(p.offset(0, 0, 1), Tile::solid(Material::LIMESTONE));
        }
    }
    w.structures.clear();
    w.rebuild_caches();
    (w, cut)
}

#[test]
fn worksite_clues_report_exact_support_removed_risk() {
    let (mut w, cut) = support_removed_gallery();
    // Opening the seven-by-five gallery's east wall leaves exactly x44..46 at y30 beyond
    // limestone's two-tile span. A completed prop at x45 holds all three tiles.
    assert_eq!(worksite_clues(&w, cut).unsupported_tiles, 3);

    let sid = w.alloc_structure_id();
    w.structures.push(hf_sim::structures::Structure {
        id: sid,
        kind: StructureKind::Support,
        pos: TilePos::new(45, 30, -2),
        material: Material::WOOD,
        progress: 1.0,
        active: false,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
    assert_eq!(worksite_clues(&w, cut).unsupported_tiles, 0);

    w.structures.clear();
    assert_eq!(worksite_clues(&w, cut).unsupported_tiles, 3);
}
