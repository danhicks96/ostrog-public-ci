//! C71: the stockpile, the field, the cellar room and the quarry grown from need (the maintainer's rule
//! 1, D35), not set down as blocks.

use hf_sim::ids::TilePos;
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::structures::StructureKind;
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::work::DesignationKind;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Tile};
use hf_sim::World;

fn meadow(seed: u64, people: usize) -> (World, hf_sim::ids::CommunityId) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    // One level of soil, as before #267, so a test that paints the rock under it paints the
    // rock a shaft meets.
    w.grid.soil.clear();
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    let ids = w.found_party(people);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    w.meta.autoplan = true;
    w.ensure_nav();
    (w, cid)
}

fn orders(w: &World, f: impl Fn(&DesignationKind) -> bool) -> Vec<TilePos> {
    w.designations
        .iter()
        .filter(|d| !d.done && f(&d.kind))
        .map(|d| d.pos)
        .collect()
}

fn contiguous(tiles: &[TilePos]) -> bool {
    if tiles.is_empty() {
        return true;
    }
    let set: std::collections::BTreeSet<TilePos> = tiles.iter().copied().collect();
    let mut seen = std::collections::BTreeSet::new();
    let mut stack = vec![tiles[0]];
    while let Some(p) = stack.pop() {
        if !seen.insert(p) {
            continue;
        }
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let q = p.offset(dx, dy, 0);
            if set.contains(&q) && !seen.contains(&q) {
                stack.push(q);
            }
        }
    }
    seen.len() == set.len()
}

#[test]
fn a_patch_grows_from_its_seed_round_what_is_in_the_way() {
    let (mut w, _) = meadow(7101, 3);
    let seed = TilePos::new(40, 40, 2);
    // A wall just east of the seed.
    for dy in -3..=3 {
        w.grid
            .set(seed.offset(1, dy, 0), Tile::solid(Material::GRANITE));
    }
    w.rebuild_caches();
    let taken = std::collections::BTreeSet::new();
    let small = hf_sim::grounds::grow(&w, &[seed], 6, &taken, false, true);
    let big = hf_sim::grounds::grow(&w, &[seed], 20, &taken, false, true);
    assert_eq!(small.len(), 6);
    assert_eq!(big.len(), 20, "a bigger need, a bigger patch");
    assert!(contiguous(&big));
    assert!(big
        .iter()
        .all(|p| w.grid.get(*p).shape == hf_sim::world::Shape::Open));
    assert!(!big.contains(&seed.offset(1, 0, 0)), "not through the wall");
    let more = hf_sim::grounds::grow(&w, &big, 5, &taken, false, false);
    assert_eq!(more.len(), 5);
    assert!(
        more.iter().all(|p| !big.contains(p)),
        "grown beside the patch, not over it"
    );
}

#[test]
fn the_stockpile_is_as_big_as_what_lies_about() {
    // Before C71 every community got a 4 by 3 block.
    let count = |seed: u64, stacks: u32| {
        let (mut w, cid) = meadow(seed, 4);
        let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
        for k in 0..stacks {
            w.create_item(
                ItemKind::Log,
                1,
                ItemPlace::Ground(home.offset(6 + (k % 5) as i32, 6 + (k / 5) as i32, 0)),
                0.5,
                "test",
            );
        }
        hf_sim::systems::settlement::step(&mut w);
        let tiles = orders(&w, |k| {
            matches!(
                k,
                DesignationKind::BuildStructure(StructureKind::Stockpile, _)
            )
        });
        assert!(contiguous(&tiles), "one patch");
        tiles.len()
    };
    let few = count(7102, 0);
    let many = count(7102, 30);
    assert!(
        few >= 6 && few < many,
        "{few} tiles for nothing lying about, {many} for thirty stacks"
    );
    assert!(many <= 12);
}

#[test]
fn the_field_is_grown_to_the_mouths_and_sown() {
    let tiles_for = |people: usize| {
        let (mut w, cid) = meadow(7103, people);
        for a in w.agents.iter_mut() {
            a.needs.hunger = 1.0;
        }
        // The larder bare, so a field is wanted.
        w.items.retain(|it| !it.kind.is_food());
        hf_sim::systems::settlement::step(&mut w);
        let c = w.communities.iter().find(|c| c.id == cid).unwrap();
        assert!(contiguous(&c.field_tiles), "one field");
        assert!(c.field_site.is_some());
        let sown = orders(&w, |k| matches!(k, DesignationKind::Sow));
        assert!(
            sown.iter().all(|p| c.field_tiles.contains(p)),
            "sowing is on the field"
        );
        assert!(!sown.is_empty(), "the field is sown");
        c.field_tiles.len()
    };
    let small = tiles_for(3);
    let big = tiles_for(8);
    assert!(
        small >= 12 && small < big,
        "{small} tiles for three, {big} for eight"
    );
    assert_ne!(big, 24, "not a 6 by 4 plot");
}

#[test]
fn an_old_saves_plots_are_read_into_the_field() {
    let (mut w, cid) = meadow(7104, 4);
    let o = TilePos::new(40, 44, 2);
    {
        let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
        c.field_site = Some(o);
        c.extra_fields = vec![o.offset(7, 0, 0)];
    }
    hf_sim::systems::settlement::step(&mut w);
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    assert!(c.field_tiles.len() >= 48, "{}", c.field_tiles.len());
    assert!(c.field_tiles.contains(&o) && c.field_tiles.contains(&o.offset(12, 3, 0)));
}

#[test]
fn the_quarry_is_sunk_where_the_stone_is() {
    // Before C71 the shaft went at home + (9, -7) whatever lay under it.
    let (mut w, cid) = meadow(7105, 8);
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    // Soil everywhere under the ground but one patch of granite north-west of home.
    for y in 0..64 {
        for x in 0..64 {
            let p = TilePos::new(x, y, 0);
            let m = if (x - (home.x - 12)).abs() <= 2 && (y - (home.y - 10)).abs() <= 2 {
                Material::GRANITE
            } else {
                Material::SOIL
            };
            if w.grid.get(p).shape == hf_sim::world::Shape::Wall {
                w.grid.set(p, Tile::solid(m));
            }
        }
    }
    w.rebuild_caches();
    w.ensure_nav();
    for a in w.agents.iter_mut() {
        a.needs.hunger = 1.0;
    }
    for _ in 0..3 {
        hf_sim::systems::settlement::step(&mut w);
    }
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    if let Some(q) = c.quarry_site {
        assert!(
            (q.x - (home.x - 12)).abs() <= 2 && (q.y - (home.y - 10)).abs() <= 2,
            "the shaft {q:?} is over the granite, not at a set offset from {home:?}"
        );
    }
}

#[test]
fn the_quarrys_shaft_is_two_levels_and_no_more() {
    // #264: the shaft was dug a level deeper every pass, down toward the world's floor.
    let (mut w, cid) = meadow(26401, 8);
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    for y in 0..64 {
        for x in 0..64 {
            for z in -8..=0 {
                let p = TilePos::new(x, y, z);
                if w.grid.get(p).shape == hf_sim::world::Shape::Wall {
                    w.grid.set(p, Tile::solid(Material::GRANITE));
                }
            }
        }
    }
    w.rebuild_caches();
    w.ensure_nav();
    for a in w.agents.iter_mut() {
        a.needs.hunger = 1.0;
    }
    let mut lowest = i32::MAX;
    for _ in 0..40 {
        hf_sim::systems::settlement::step(&mut w);
        // Every shaft order dug at once, as the diggers would in time.
        let digs: Vec<TilePos> = w
            .designations
            .iter()
            .filter(|d| !d.done && matches!(d.kind, DesignationKind::DigStairs))
            .map(|d| d.pos)
            .collect();
        for p in digs {
            lowest = lowest.min(p.z);
            let mut t = w.grid.get(p);
            t.shape = hf_sim::world::Shape::StairsBoth;
            w.grid.set(p, t);
            w.designations.retain(|d| d.pos != p);
        }
        w.rebuild_caches();
    }
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    let site = c.quarry_site.expect("a quarry was begun");
    assert!(site.dist(home) >= 6);
    assert!(
        lowest >= site.z - 2,
        "the shaft stops two levels under the surface: lowest {lowest}, surface {}",
        site.z
    );
}
