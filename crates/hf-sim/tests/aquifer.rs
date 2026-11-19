//! #272: a cut into wet rock lets the water in; one into dry rock does not.

use hf_sim::events::{EventKind, Severity};
use hf_sim::ids::TilePos;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn cut(w: &mut World, at: TilePos) {
    // Open the cut, a floor under it.
    w.grid.set(at, hf_sim::world::Tile::AIR);
    let agent = w.agents[0].id;
    let tick = w.meta.tick;
    let id = w.events.push(
        tick,
        Some(at),
        Severity::Minor,
        EventKind::Mined {
            agent,
            at,
            found: None,
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

#[test]
fn wet_rock_floods_the_cut_and_dry_rock_does_not() {
    let mut w = World::generate(2721, GenParams::small());
    w.found_party(1);
    let (mut wet, mut dry) = (None, None);
    // #267: the water lies in a band of the porous rock, so look down each column.
    'find: for y in 5..60 {
        for x in 5..60 {
            for depth in 3..8 {
                let p = TilePos::new(x, y, w.grid.ground_z(x, y) - depth);
                if !w.grid.in_bounds(p) || w.grid.get(p).shape != hf_sim::world::Shape::Wall {
                    continue;
                }
                if hf_sim::aquifer::wet(&w, p) {
                    wet.get_or_insert(p);
                } else {
                    dry.get_or_insert(p);
                }
                if wet.is_some() && dry.is_some() {
                    break 'find;
                }
            }
        }
    }
    let (wet, dry) = (wet.expect("wet rock somewhere"), dry.unwrap());
    cut(&mut w, dry);
    assert!(!w.grid.get(dry).is_water());
    cut(&mut w, wet);
    assert!(w.grid.get(wet).is_water(), "the water came in");
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::StruckWater { .. })));
}

#[test]
fn water_let_in_runs_along_the_workings() {
    let mut w = World::generate(2722, GenParams::small());
    w.found_party(1);
    // A tunnel ten long through the rock five down, a floor under it.
    let (x0, y) = (20, 20);
    let z = w.grid.ground_z(x0, y) - 5;
    for dx in 0..10 {
        w.grid
            .set(TilePos::new(x0 + dx, y, z), hf_sim::world::Tile::AIR);
    }
    w.seeping.push((TilePos::new(x0, y, z), 8));
    for h in 1..20u64 {
        w.meta.tick = h * hf_sim::time::TICKS_PER_HOUR + 123;
        hf_sim::aquifer::flow(&mut w);
    }
    let wet = (0..10)
        .filter(|dx| w.grid.get(TilePos::new(x0 + dx, y, z)).is_water())
        .count();
    assert!(wet >= 5, "the water ran along the tunnel: {wet} tiles");
    assert!(w.seeping.is_empty(), "and its volume is spent");
}

/// #405: the valley digs one and two levels down (cellars, galleries, hollows into a hill).
/// Under the strata (#267) a basin's porous rock can lie two down: a cut there strikes water,
/// and one down is never wet.
#[test]
fn a_cut_two_down_can_strike_water_and_one_down_cannot() {
    // A small world with a basin whose wet rock lies two under the ground somewhere.
    let mut found = None;
    for seed in 2721..2761 {
        let w = World::generate(seed, GenParams::small());
        let g = &w.grid;
        let mut two = None;
        for y in 2..g.height - 2 {
            for x in 2..g.width - 2 {
                let gz = g.ground_z(x, y);
                assert!(!hf_sim::aquifer::wet(&w, TilePos::new(x, y, gz - 1)));
                let p = TilePos::new(x, y, gz - 2);
                if two.is_none()
                    && g.get(p).shape == hf_sim::world::Shape::Wall
                    && hf_sim::aquifer::wet(&w, p)
                {
                    two = Some(p);
                }
            }
        }
        if let Some(p) = two {
            found = Some((w, p));
            break;
        }
    }
    let (mut w, wet) = found.expect("a basin two down in some world");
    w.found_party(1);
    cut(&mut w, wet);
    assert!(w.grid.get(wet).is_water(), "the water came in");
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::StruckWater { .. })));
}

/// #405: only the first cut into wet rock strikes water. Digging on from that cut into the same
/// wet ground is wet, but no new strike on record and no new seep.
#[test]
fn digging_on_through_wet_rock_strikes_water_once() {
    let mut w = World::generate(2721, GenParams::small());
    w.found_party(1);
    // Two wet tiles of rock side by side, at whatever depth the basin's water lies (#267).
    let mut pair = None;
    'find: for y in 5..60 {
        for x in 5..60 {
            for depth in 2..9 {
                let a = TilePos::new(x, y, w.grid.ground_z(x, y) - depth);
                let b = TilePos::new(x + 1, y, a.z);
                let rock = |p: TilePos| w.grid.get(p).shape == hf_sim::world::Shape::Wall;
                if w.grid.in_bounds(a)
                    && rock(a)
                    && rock(b)
                    && hf_sim::aquifer::wet(&w, a)
                    && hf_sim::aquifer::wet(&w, b)
                {
                    pair = Some((a, b));
                    break 'find;
                }
            }
        }
    }
    let (a, b) = pair.expect("wet rock somewhere");
    let struck = |w: &World| {
        w.events
            .pending
            .iter()
            .filter(|e| matches!(e.kind, EventKind::StruckWater { .. }))
            .count()
    };
    w.dug.insert(a);
    cut(&mut w, a);
    assert_eq!(struck(&w), 1, "the first cut strikes water");
    let seeps = w.seeping.len();
    w.dug.insert(b);
    cut(&mut w, b);
    assert_eq!(
        struck(&w),
        1,
        "the next cut into the same wet rock is no new strike"
    );
    assert_eq!(w.seeping.len(), seeps);
    assert!(w.grid.get(b).is_water(), "but it is wet");
}
