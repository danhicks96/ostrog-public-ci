//! #390: standing water beside a cut breaks into it. A shaft dug beside a pond floods to the
//! pond's level and the pond gives up as much; someone at the bottom has to get out or drowns;
//! the planner sites its shaft away from water; a cautious digger leaves damp stone.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

/// Flat ground at level 1 (people stand at 2), with a pond five across, two levels deep, at
/// x and y 20 to 24.
fn by_a_pond(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    for y in 20..=24 {
        for x in 20..=24 {
            for z in [1, 2] {
                w.grid.set(
                    TilePos::new(x, y, z),
                    Tile {
                        material: Material::WATER,
                        shape: Shape::Floor,
                        flags: TileFlags(TileFlags::WATER),
                    },
                );
            }
        }
    }
    w.rebuild_caches();
    w
}

fn pond_tiles(w: &World) -> usize {
    (20..=24)
        .flat_map(|y| (20..=24).flat_map(move |x| [1, 2].map(|z| TilePos::new(x, y, z))))
        .filter(|p| w.grid.get(*p).is_water())
        .count()
}

/// A stair shaft two levels down at (x, y), and a tunnel two long east from its foot.
fn dig_shaft(w: &mut World, x: i32, y: i32) {
    let stair = |shape| Tile {
        material: Material::GRANITE,
        shape,
        flags: TileFlags::NONE,
    };
    w.grid.set(TilePos::new(x, y, 2), stair(Shape::StairsDown));
    w.grid.set(TilePos::new(x, y, 1), stair(Shape::StairsBoth));
    w.grid.set(TilePos::new(x, y, 0), stair(Shape::StairsUp));
    for dx in 1..=2 {
        let mut t = Tile::AIR;
        t.flags = TileFlags::NONE;
        w.grid.set(TilePos::new(x + dx, y, 0), t);
    }
}

/// The cut at `at` is on record as just made, and the aquifer pass sees it.
fn cut(w: &mut World, at: TilePos) {
    let agent = w.agents[0].id;
    let tick = w.meta.tick;
    let id = w.events.push(
        tick,
        Some(at),
        hf_sim::events::Severity::Notable,
        EventKind::Mined {
            agent,
            at,
            found: Some("a way down".into()),
            rock: None,
        },
    );
    let e = w
        .events
        .pending
        .iter()
        .find(|e| e.id == id)
        .unwrap()
        .clone();
    w.last_events = vec![e];
    hf_sim::aquifer::observe(w);
}

fn broke_in(w: &World) -> Option<u32> {
    w.events.pending.iter().find_map(|e| match e.kind {
        EventKind::WaterBrokeIn { tiles, .. } => Some(tiles),
        _ => None,
    })
}

#[test]
fn a_shaft_beside_a_pond_floods_and_the_pond_gives_up_as_much() {
    let mut w = by_a_pond(3901);
    w.found_party(1);
    let before = pond_tiles(&w);
    // Right beside the pond's east shore: its foot is beside the pond's lower level.
    dig_shaft(&mut w, 25, 22);
    cut(&mut w, TilePos::new(25, 22, 0));
    let tiles = broke_in(&w).expect("the water broke in, on record");
    // The foot and the tunnel (level 0), the stair above it (level 1), and the stair's top at
    // the pond's own level.
    assert_eq!(tiles, 5);
    for p in [
        (25, 22, 0),
        (26, 22, 0),
        (27, 22, 0),
        (25, 22, 1),
        (25, 22, 2),
    ] {
        let p = TilePos::new(p.0, p.1, p.2);
        assert!(w.grid.get(p).is_water(), "{p:?} flooded");
    }
    assert!(w
        .grid
        .get(TilePos::new(27, 22, 0))
        .flags
        .has(TileFlags::FULL));
    assert!(!w
        .grid
        .get(TilePos::new(25, 22, 2))
        .flags
        .has(TileFlags::FULL));
    assert_eq!(
        pond_tiles(&w),
        before - 5,
        "the pond gave up what it let in"
    );
    // Nothing rose above the pond's level onto the dry ground.
    assert!(!w.grid.get(TilePos::new(26, 22, 2)).is_water());
    let line = hf_sim::view::narrate::line(
        &w,
        w.events
            .pending
            .iter()
            .find(|e| matches!(e.kind, EventKind::WaterBrokeIn { .. }))
            .unwrap(),
    );
    assert!(line.contains("water broke in"), "{line}");
}

#[test]
fn a_shaft_away_from_the_water_stays_dry() {
    let mut w = by_a_pond(3902);
    w.found_party(1);
    dig_shaft(&mut w, 28, 22);
    cut(&mut w, TilePos::new(28, 22, 0));
    assert!(broke_in(&w).is_none());
    assert!(!w.grid.get(TilePos::new(28, 22, 0)).is_water());
}

/// One person at the far end of the tunnel when the water comes in.
fn caught_below(seed: u64, way_out: bool) -> (World, usize) {
    let mut w = by_a_pond(seed);
    let ids = w.found_party(1);
    w.meta.autoplan = false;
    dig_shaft(&mut w, 25, 22);
    if !way_out {
        // The stair's foot falls in behind them: the tunnel is a pocket.
        w.grid
            .set(TilePos::new(26, 22, 0), Tile::solid(Material::GRANITE));
    }
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let bottom = TilePos::new(27, 22, 0);
    w.agents[i].pos = bottom;
    w.agents[i].from = bottom;
    w.agents[i].path.clear();
    w.rebuild_caches();
    w.ensure_nav();
    if way_out {
        cut(&mut w, TilePos::new(25, 22, 0));
    } else {
        // The pocket fills from the rock above it.
        let mut t = w.grid.get(bottom);
        t.flags = TileFlags(TileFlags::WATER | TileFlags::FULL);
        w.grid.set(bottom, t);
    }
    assert!(hf_sim::systems::hazards::under_water(&w, bottom));
    (w, i)
}

#[test]
fn someone_at_the_bottom_climbs_out_of_the_flood() {
    let (mut w, i) = caught_below(3903, true);
    for _ in 0..200 {
        w.step();
    }
    let a = &w.agents[i];
    assert!(a.alive, "they got out: {:?}", a.death);
    assert!(!hf_sim::systems::hazards::under_water(&w, a.pos));
}

#[test]
fn someone_with_no_way_out_drowns() {
    let (mut w, i) = caught_below(3904, false);
    for _ in 0..200 {
        w.step();
    }
    let a = &w.agents[i];
    assert!(!a.alive, "nobody holds their breath that long");
    assert!(a.death.as_deref().unwrap_or("").contains("drowned"));
}

#[test]
fn the_quarry_shaft_is_dug_on_dry_ground() {
    // Stone under all of it, and the pond nearer home than any dry ground: the shaft still goes
    // where the water is more than two tiles off.
    let mut w = by_a_pond(3905);
    w.meta.founding_site = TilePos::new(22, 30, 2);
    let ids = w.found_party(4);
    let cid = w.agents.iter().find(|a| a.id == ids[0]).unwrap().community;
    for _ in 0..(TICKS_PER_HOUR * 6) {
        w.step();
        let site = w
            .communities
            .iter()
            .find(|c| Some(c.id) == cid)
            .and_then(|c| c.quarry_site);
        if let Some(s) = site {
            assert!(
                !hf_sim::aquifer::near_water(&w, s, 2),
                "the shaft at {s:?} is within two of the pond"
            );
            return;
        }
    }
    panic!("no quarry was sited");
}

#[test]
fn a_cautious_digger_leaves_damp_stone() {
    use hf_sim::mind::Trait;
    let dig_beside_pond = |cautious: bool| {
        let mut w = by_a_pond(3906);
        let ids = w.found_party(1);
        w.meta.autoplan = false;
        let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
        if cautious {
            w.agents[i].traits.list.push(Trait::Cautious);
        } else {
            w.agents[i].traits.list.retain(|t| *t != Trait::Cautious);
        }
        // A boulder two tiles from the pond's shore, to be cut away.
        let at = TilePos::new(26, 22, 2);
        w.grid.set(at, Tile::solid(Material::GRANITE));
        assert!(hf_sim::aquifer::damp(&w, at));
        w.agents[i].pos = TilePos::new(27, 22, 2);
        w.agents[i].from = w.agents[i].pos;
        let mut d =
            hf_sim::systems::settlement::make(&mut w, hf_sim::work::DesignationKind::Mine, at, 9);
        d.community = w.agents[i].community;
        w.designations.push(d);
        w.rebuild_caches();
        w.ensure_nav();
        let mut left = false;
        for _ in 0..(TICKS_PER_HOUR * 6) {
            w.step();
            left |= w.last_events.iter().any(
                |e| matches!(&e.kind, EventKind::Chose { because, .. } if because.contains("damp")),
            );
        }
        let dug = w.grid.get(at).shape != Shape::Wall;
        (left, dug)
    };
    let (left, dug) = dig_beside_pond(true);
    assert!(
        left && !dug,
        "the cautious one left it: left {left}, dug {dug}"
    );
    let (left, dug) = dig_beside_pond(false);
    assert!(!left && dug, "anyone else dug on: left {left}, dug {dug}");
}

/// #405: water already in the workings is not a new break-in. A shaft beside the pond floods
/// once; digging on along the flooded working records no second break-in and takes no more
/// from the pond.
#[test]
fn digging_along_a_flooded_working_is_no_second_break_in() {
    let mut w = by_a_pond(3906);
    w.found_party(1);
    dig_shaft(&mut w, 25, 22);
    for p in [
        (25, 22, 2),
        (25, 22, 1),
        (25, 22, 0),
        (26, 22, 0),
        (27, 22, 0),
    ] {
        w.dug.insert(TilePos::new(p.0, p.1, p.2));
    }
    cut(&mut w, TilePos::new(25, 22, 0));
    assert!(broke_in(&w).is_some(), "the first cut lets the pond in");
    let pond = pond_tiles(&w);
    // On along the tunnel: a cut beside the flooded end.
    let next = TilePos::new(28, 22, 0);
    w.grid.set(next, Tile::AIR);
    w.dug.insert(next);
    w.events.pending.clear();
    cut(&mut w, next);
    assert_eq!(broke_in(&w), None, "no second break-in");
    assert_eq!(pond_tiles(&w), pond, "the pond gave up nothing more");
}
