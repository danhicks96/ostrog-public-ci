//! A diagonal regional river has banks on both sides of the four-region junction.
use hf_sim::floods::{self, WaterKind};
use hf_sim::ids::TilePos;
use hf_sim::setup::WorldSetup;
use hf_sim::world::{Material, Tile, TileFlags};
use hf_sim::World;

fn site(region: usize) -> World {
    let mut s = WorldSetup::new(3);
    s.region = Some(region);
    s.history_years = 0;
    s.realm_years = 200;
    let mut w = World::from_setup(&s);
    assert_eq!(w.meta.setup.as_ref().unwrap().region, Some(region));
    floods::survey(&mut w);
    w
}

#[test]
fn both_sides_of_a_diagonal_have_a_wet_cross_section_at_the_shared_grade() {
    // Seed 3 drains southwest from 7752 into 7943. Its two neighboring
    // regions supply the northwestern and southeastern banks of the same river.
    let ne = site(7752);
    let sw = site(7943);
    for (region, southeast) in [(7751, false), (7944, true)] {
        let w = site(region);
        let corner = if southeast { 0 } else { 127 };
        let inward = if southeast { 1 } else { -1 };
        let at = TilePos::new(corner, corner, 1);
        assert_eq!(floods::body_at(&w, at).unwrap().kind, WaterKind::Stream);
        for i in 0..4 {
            for j in 0..4 - i {
                assert_eq!(
                    floods::top_water(&w.grid, corner + i * inward, corner + j * inward)
                        .unwrap()
                        .z,
                    1
                );
            }
            // These are the actual neighboring finished cells across both shared faces.
            let (a, b) = if southeast {
                ((i, 127), (127, i))
            } else {
                ((0, 127 - i), (127 - i, 0))
            };
            assert_eq!(floods::top_water(&ne.grid, a.0, a.1).unwrap().z, 1);
            assert_eq!(floods::top_water(&sw.grid, b.0, b.1).unwrap().z, 1);
        }
        assert!(!w
            .grid
            .river_spills
            .iter()
            .any(|e| e.upper == at || e.lower == at));
    }
}

#[test]
fn a_corner_never_promotes_a_separate_pond_or_a_larger_connected_lake() {
    let mut w = site(7944);
    let at = TilePos::new(0, 0, 1);
    let liquid = Tile {
        flags: TileFlags(TileFlags::WATER),
        ..Tile::floor(Material::WATER)
    };
    for y in 0..12 {
        for x in 0..12 {
            for z in 1..=4 {
                w.grid.set(TilePos::new(x, y, z), Tile::AIR);
            }
        }
    }
    for cell in [
        at,
        at.offset(1, 0, 0),
        at.offset(0, 1, 0),
        at.offset(8, 8, 0),
    ] {
        w.grid.set(cell, liquid);
    }
    floods::survey(&mut w);
    assert_eq!(floods::body_at(&w, at).unwrap().kind, WaterKind::Stream);
    assert_eq!(
        floods::body_at(&w, at.offset(8, 8, 0)).unwrap().kind,
        WaterKind::Pond
    );
    w.grid.set(
        at,
        Tile {
            flags: TileFlags(TileFlags::WATER),
            ..Tile::floor(Material::WOOD)
        },
    );
    floods::survey(&mut w);
    assert_eq!(floods::body_at(&w, at).unwrap().kind, WaterKind::Pond);
    for y in 0..8 {
        for x in 0..8 {
            w.grid.set(at.offset(x, y, 0), liquid);
        }
    }
    floods::survey(&mut w);
    assert_eq!(floods::body_at(&w, at).unwrap().kind, WaterKind::Lake);
}

#[test]
fn corner_water_keeps_its_kind_and_world_state_after_save_and_reload() {
    use hf_sim::save::{decode, encode, snapshot::Snapshot};
    let mut original = site(7944);
    let lip = TilePos::new(0, 0, 1);
    assert_eq!(
        floods::body_at(&original, lip).unwrap().kind,
        WaterKind::Stream
    );
    let bytes = encode(&Snapshot::capture(&original)).unwrap();
    let mut restored = decode(&bytes).unwrap().restore();
    assert_eq!(
        hf_sim::digest::digest(&original),
        hf_sim::digest::digest(&restored)
    );
    floods::survey(&mut restored);
    assert_eq!(
        floods::body_at(&restored, lip).unwrap().kind,
        WaterKind::Stream
    );
    for _ in 0..100 {
        original.step();
        restored.step();
    }
    assert_eq!(
        hf_sim::digest::digest(&original),
        hf_sim::digest::digest(&restored)
    );
}
#[test]
fn a_cardinal_crossing_and_an_inland_divide_keep_their_own_water() {
    // All seven columns of this southbound crossing matched on the original generator.
    let north = site(8134);
    let south = site(8326);
    let wet = |w: &World, x, y| floods::top_water(&w.grid, x, y).map(|p| p.z);
    let north_wet: Vec<_> = (0..128)
        .filter(|&x| wet(&north, x, 127).is_some())
        .collect();
    let south_wet: Vec<_> = (0..128).filter(|&x| wet(&south, x, 0).is_some()).collect();
    assert_eq!(north_wet, south_wet);
    assert_eq!(north_wet.len(), 7);
    for x in north_wet {
        assert_eq!(wet(&north, x, 127), wet(&south, x, 0));
    }
    let mut setup = WorldSetup::new(7);
    setup.region = Some(9368);
    setup.history_years = 0;
    setup.realm_years = 200;
    let dry = World::from_setup(&setup);
    assert!(
        dry.meta.params.drainage_divide
            && dry.meta.params.river.is_none()
            && dry.meta.params.river_corners.is_empty()
    );
    assert!((0..dry.grid.height).all(|y| (0..dry.grid.width).all(|x| wet(&dry, x, y).is_none())));
}
