//! The roof of the deep (underground.rs): a dug gallery holds only within its rock's span of a
//! wall or a prop; a cut past that can bring down the whole unheld part at once, and the
//! ground above sinks where the rock is thin. Miners who know shoring prop before cutting.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::mind::techniques::{Learned, Technique, Techniques};
use hf_sim::structures::StructureKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::underground;
use hf_sim::work::{Designation, DesignationKind};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

/// A flat meadow at ground z1, with a shaft at (40,30) down to z-2 and, at z-2, a dug room
/// x 42..42+w, y 28..33 opening off the shaft and roofed (z-1) with `roof`. The cut to make
/// is the next column's middle, (42+w, 30), widening the room.
fn mine(seed: u64, width: i32, roof: Material) -> World {
    shallow_mine(seed, width, roof, 1)
}

/// The same, with the ground at `ground` (0 puts the room two levels under the surface).
fn shallow_mine(seed: u64, width: i32, roof: Material, ground: i32) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, ground);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = 7 * TICKS_PER_DAY + TICKS_PER_HOUR * 8;
    let set = |w: &mut World, p: TilePos, shape: Shape, m: Material| {
        w.grid.set(
            p,
            Tile {
                material: m,
                shape,
                flags: TileFlags::NONE,
            },
        )
    };
    set(
        &mut w,
        TilePos::new(40, 30, ground + 1),
        Shape::StairsDown,
        Material::STONE_BLOCK,
    );
    for z in -1..=ground {
        set(
            &mut w,
            TilePos::new(40, 30, z),
            Shape::StairsBoth,
            Material::GRANITE,
        );
    }
    set(
        &mut w,
        TilePos::new(40, 30, -2),
        Shape::StairsUp,
        Material::GRANITE,
    );
    set(&mut w, TilePos::new(41, 30, -2), Shape::Open, Material::AIR);
    w.dug.insert(TilePos::new(41, 30, -2));
    for x in 42..42 + width {
        for y in 28..33 {
            let p = TilePos::new(x, y, -2);
            set(&mut w, p, Shape::Open, Material::AIR);
            w.dug.insert(p);
            set(&mut w, p.offset(0, 0, 1), Shape::Wall, roof);
        }
    }
    for y in 28..33 {
        set(&mut w, TilePos::new(42 + width, y, -1), Shape::Wall, roof);
    }
    w.rebuild_caches();
    w
}

#[test]
fn the_span_depends_on_the_rock() {
    // A room seven wide: its middle is three from any wall. Limestone spans two, basalt four.
    let cut = TilePos::new(49, 30, -2);
    let soft = mine(5, 7, Material::LIMESTONE);
    let hard = mine(5, 7, Material::BASALT);
    assert!(!underground::unheld_if_opened(&soft, cut).is_empty());
    assert!(underground::unheld_if_opened(&hard, cut).is_empty());
    // A narrow gallery stands in any rock.
    let narrow = mine(5, 3, Material::LIMESTONE);
    assert!(underground::unheld_if_opened(&narrow, TilePos::new(45, 30, -2)).is_empty());
}

#[test]
fn a_prop_holds_the_roof() {
    let cut = TilePos::new(49, 30, -2);
    let mut w = mine(5, 7, Material::LIMESTONE);
    let unheld = underground::unheld_if_opened(&w, cut);
    assert!(!unheld.is_empty());
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
    assert!(underground::unheld_if_opened(&w, cut).is_empty());
}

#[test]
fn a_cave_in_takes_everything_it_no_longer_holds() {
    let cut = TilePos::new(50, 30, -2);
    let mut w = shallow_mine(5, 8, Material::LIMESTONE, 0);
    let ids = w.found_party(1);
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    // Someone standing in the middle of the room.
    let mid = TilePos::new(46, 30, -2);
    w.agents[i].pos = mid;
    let unheld = underground::unheld_if_opened(&w, cut);
    assert!(
        unheld.len() >= 3,
        "a whole span, not one tile: {}",
        unheld.len()
    );
    assert!(unheld.contains(&mid));
    underground::collapse(&mut w, i, cut, &unheld);
    for p in &unheld {
        assert!(w.grid.get(*p).shape.is_solid(), "{p:?} filled");
        // Two below the surface: the ground above sank.
        assert_eq!(
            w.grid
                .get(TilePos::new(p.x, p.y, w.grid.ground_z(p.x, p.y)))
                .material,
            Material::GRAVEL
        );
    }
    let a = &w.agents[i];
    assert!(w.grid.walkable(a.pos), "thrown clear");
    assert!(a
        .body
        .wounds
        .iter()
        .any(|wd| wd.kind == hf_sim::body::WoundKind::Break));
    assert!(
        a.techniques.knows(Technique::Shoring),
        "learned the hard way"
    );
    let e = w
        .events
        .pending
        .iter()
        .find(|e| matches!(e.kind, EventKind::Collapse { .. }))
        .expect("recorded");
    if let EventKind::Collapse {
        tiles, sank, hurt, ..
    } = &e.kind
    {
        assert_eq!(*tiles as usize, unheld.len());
        assert!(*sank);
        assert_eq!(hurt, &vec![ids[0]]);
    }
}

/// Order the cut that widens an 8-wide room (x 42..50) by a column.
fn dig_wall(w: &mut World, cid: hf_sim::ids::CommunityId) {
    let id = w.alloc_designation_id();
    w.designations.push(Designation {
        id,
        kind: DesignationKind::Mine,
        pos: TilePos::new(50, 30, -2),
        requirements: vec![],
        work_left: 20 * 15,
        work_total: 20 * 15,
        max_workers: 1,
        priority: 9,
        created: w.tick(),
        failures: 0,
        done: false,
        community: Some(cid),
        owner: None,
    });
}

#[test]
fn a_miner_who_knows_props_first() {
    let mut w = mine(5, 8, Material::LIMESTONE);
    w.meta.autoplan = false;
    let ids = w.found_party(1);
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let cid = w.agents[i].community.unwrap();
    w.agents[i].techniques = Techniques::default();
    w.agents[i]
        .techniques
        .learn(Technique::Shoring, Learned::Past, 0.6, 0);
    for _ in 0..3 {
        w.create_item(
            ItemKind::Log,
            1,
            ItemPlace::Ground(TilePos::new(38, 30, 2)),
            0.6,
            "test",
        );
    }
    dig_wall(&mut w, cid);
    w.ensure_nav();
    let (mut propped, mut collapsed) = (false, false);
    for _ in 0..TICKS_PER_DAY {
        w.step();
        for e in &w.last_events {
            match e.kind {
                EventKind::Propped { .. } => propped = true,
                EventKind::Collapse { .. } => collapsed = true,
                _ => {}
            }
        }
    }
    assert!(propped, "stopped to prop the roof first");
    assert!(!collapsed, "and the roof held");
    assert!(
        w.structures
            .iter()
            .any(|s| s.kind == StructureKind::Support && s.pos.z == -2),
        "the prop was set"
    );
    assert!(
        !w.grid.get(TilePos::new(50, 30, -2)).shape.is_solid(),
        "and then the cut was made"
    );
}

#[test]
fn a_miner_who_does_not_brings_it_down_sometimes() {
    let mut collapses = 0;
    for seed in 0..8u64 {
        let mut w = mine(seed, 8, Material::LIMESTONE);
        w.meta.autoplan = false;
        let ids = w.found_party(1);
        let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
        let cid = w.agents[i].community.unwrap();
        w.agents[i].techniques = Techniques::default();
        dig_wall(&mut w, cid);
        w.ensure_nav();
        for _ in 0..TICKS_PER_HOUR * 8 {
            w.step();
            collapses += w
                .last_events
                .iter()
                .filter(|e| matches!(e.kind, EventKind::Collapse { .. }))
                .count();
        }
    }
    assert!(
        (1..8).contains(&collapses),
        "about a third of unpropped cuts bring the roof down: {collapses} of 8"
    );
}
