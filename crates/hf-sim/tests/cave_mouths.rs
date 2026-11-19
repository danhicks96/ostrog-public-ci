//! Cave mouths (#482): a rare opening in a hillside that leads into a real cave, walked by the
//! game's own rules, cut only through rock and roofed, the same for the same seed, and absent
//! where the world has no caves.

use hf_sim::ids::TilePos;
use hf_sim::nav::{find_path, AStarScratch, Nav, PathResult};
use hf_sim::project::HomeForm;
use hf_sim::species;
use hf_sim::world::gen::{
    generate,
    mouths::{self, Mouth},
    GenParams,
};
use hf_sim::world::{Material, Shape, Tile, TileFlags, WorldGrid};

const SEEDS: std::ops::RangeInclusive<u64> = 1..=10;
const SCENARIO_FOOT: TilePos = TilePos::new(11, 16, 1);

fn cave_room(x: i32, y: i32, width: i32, height: i32) -> Vec<TilePos> {
    (y..y + height)
        .flat_map(|y| (x..x + width).map(move |x| TilePos::new(x, y, -2)))
        .collect()
}

/// One hillside face, one straight possible adit, and a hand-cut cave at its end. Water marks
/// every other column unavailable to the mouth search; it is scenario fencing, not hydrology.
fn mouth_scenario(cave: &[TilePos]) -> (WorldGrid, Vec<bool>, GenParams) {
    let p = GenParams {
        width: 24,
        height: 24,
        ..GenParams::default()
    };
    let mut g = WorldGrid::new(p.width, p.height, -12, 8);
    for y in 2..22 {
        for x in 12..21 {
            let c = g.col(x, y);
            g.ground[c] = 4;
        }
    }
    let columns = (p.width * p.height) as usize;
    g.strata = vec![(Material::MARBLE, 0, -2); columns];
    g.deep = vec![(Material::GABBRO, i8::MIN); columns];
    for &at in cave {
        g.set(at, Tile::AIR);
    }
    let mut water = vec![true; columns];
    for x in 11..=16 {
        water[g.col(x, 16)] = false;
    }
    (g, water, p)
}

fn apply_scenario(g: &mut WorldGrid, water: &[bool], p: &GenParams) -> Vec<Mouth> {
    mouths::apply(1, p, g, water)
}

#[test]
fn constructed_mouth_requires_a_connected_cave_room_of_eight_tiles() {
    let room = cave_room(17, 15, 3, 3);
    let (mut g, water, p) = mouth_scenario(&room);
    let mouths = apply_scenario(&mut g, &water, &p);
    assert_eq!(mouths.len(), 1, "the nine-tile room accepts the mouth");
    let m = &mouths[0];
    assert!(room.contains(&m.cave));
    assert_eq!(walk(&g, m.foot, m.cave).last(), Some(&m.cave));
    assert_eq!(walk(&g, m.cave, m.foot).last(), Some(&m.foot));

    let seven = cave_room(17, 10, 1, 7);
    let (mut g, water, p) = mouth_scenario(&seven);
    assert!(
        apply_scenario(&mut g, &water, &p).is_empty(),
        "seven connected floors are not a cave room"
    );

    let mut isolated_with_room_elsewhere = vec![TilePos::new(17, 16, -2)];
    isolated_with_room_elsewhere.extend(cave_room(18, 2, 4, 2));
    let (mut g, water, p) = mouth_scenario(&isolated_with_room_elsewhere);
    assert!(
        apply_scenario(&mut g, &water, &p).is_empty(),
        "an isolated floor does not borrow volume from a disconnected room"
    );
}

#[test]
fn constructed_mouth_preserves_the_ramp_approach_and_bidirectional_traversal() {
    let (mut g, water, p) = mouth_scenario(&cave_room(17, 15, 3, 3));
    let upper_approach = TilePos::new(11, 15, 2);
    g.set(upper_approach.offset(0, 0, -1), Tile::solid(Material::SOIL));
    g.set(
        SCENARIO_FOOT,
        Tile {
            material: Material::SOIL,
            shape: Shape::RampUp,
            flags: TileFlags::NONE,
        },
    );
    assert_eq!(
        walk(&g, upper_approach, SCENARIO_FOOT).last(),
        Some(&SCENARIO_FOOT)
    );

    let mouths = apply_scenario(&mut g, &water, &p);
    assert_eq!(mouths.len(), 1);
    let m = &mouths[0];
    assert_eq!(m.foot, SCENARIO_FOOT);
    assert_eq!(g.get(SCENARIO_FOOT).shape, Shape::RampUp);
    assert_eq!(g.get(SCENARIO_FOOT).material, Material::SOIL);
    assert_eq!(walk(&g, upper_approach, m.cave).last(), Some(&m.cave));
    assert_eq!(
        walk(&g, m.cave, upper_approach).last(),
        Some(&upper_approach)
    );
}

#[test]
fn constructed_mouth_requires_an_existing_rock_brow() {
    let (mut g, water, p) = mouth_scenario(&cave_room(17, 15, 3, 3));
    let open_brow = TilePos::new(12, 16, 1);
    g.set(open_brow, Tile::AIR);
    let before = g.clone();

    assert!(apply_scenario(&mut g, &water, &p).is_empty());
    assert_eq!(g.ground, before.ground);
    assert_eq!(g.get(open_brow), before.get(open_brow));
}

#[test]
fn constructed_mouth_keeps_the_rock_material_at_each_actual_depth() {
    let (mut g, water, p) = mouth_scenario(&cave_room(17, 15, 3, 3));
    let before = g.clone();
    let mouths = apply_scenario(&mut g, &water, &p);
    assert_eq!(mouths.len(), 1);
    let m = &mouths[0];
    let brow = m.entrance.offset(0, 0, 1);
    let sill = m.entrance.offset(0, 0, -1);
    assert_eq!(brow, TilePos::new(12, 16, 1));
    assert_eq!(before.get(brow).shape, Shape::Wall);
    assert_eq!(g.get(brow), before.get(brow));
    assert_eq!(g.get(sill).material, before.get(sill).material);
    assert_eq!(g.ground, before.ground);

    let mut ramps = 0;
    for x in 12..=16 {
        for z in -2..=1 {
            let at = TilePos::new(x, 16, z);
            if g.get(at).shape == Shape::RampUp {
                ramps += 1;
                assert_eq!(
                    g.get(at).material,
                    before.get(at).material,
                    "ramp at {at} must keep its removed wall's material"
                );
            }
        }
    }
    assert!(
        ramps >= 3,
        "the constructed adit descends through three strata"
    );
}

#[test]
fn constructed_mouth_never_adds_a_brow_or_raises_the_surface() {
    let (mut g, water, p) = mouth_scenario(&cave_room(17, 15, 3, 3));
    let surface = TilePos::new(12, 16, 1);
    let open_above = TilePos::new(12, 16, 2);
    g.set(open_above, Tile::AIR);
    let before = g.clone();

    let mouths = apply_scenario(&mut g, &water, &p);
    assert_eq!(mouths.len(), 1);
    let m = &mouths[0];
    assert_eq!(m.entrance, TilePos::new(12, 16, 0));
    assert_eq!(g.ground, before.ground);
    assert_eq!(g.get(surface), before.get(surface), "the brow is untouched");
    assert_eq!(g.get(open_above), Tile::AIR, "no surface hump was added");
    assert_eq!(walk(&g, m.foot, m.cave).last(), Some(&m.cave));
    assert_eq!(walk(&g, m.cave, m.foot).last(), Some(&m.foot));
}

fn walk(g: &WorldGrid, a: TilePos, b: TilePos) -> Vec<TilePos> {
    let mut s = AStarScratch::new(g);
    match find_path(g, &mut s, a, b, 200_000) {
        PathResult::Found(p) => p,
        other => panic!("no way from {a} to {b}: {other:?}"),
    }
}

/// Every tile of the passage, from the entrance to the last step before the cave, found by
/// walking in from the foot: the tiles under ground that are not the cave itself.
fn passage(g: &WorldGrid, m: &Mouth) -> Vec<TilePos> {
    walk(g, m.foot, m.cave)
        .into_iter()
        .filter(|p| g.ground_z(p.x, p.y) > p.z && *p != m.cave)
        .collect()
}

#[test]
fn a_mouth_is_walked_in_and_out_to_a_real_cave() {
    let mut mouths = 0;
    for seed in SEEDS {
        let g = generate(seed, &GenParams::default());
        let nav = Nav::build(&g.grid);
        for m in &g.cave_mouths {
            mouths += 1;
            let grid = &g.grid;
            // The cave end is a floor well below the ground; the foot is open ground.
            assert!(
                grid.walkable(m.cave),
                "seed {seed}: the cave end can be stood on"
            );
            assert!(
                m.cave.z <= -2,
                "seed {seed}: it ends in a cave, not near the surface"
            );
            assert!(
                grid.ground_z(m.cave.x, m.cave.y) - m.cave.z >= 3,
                "seed {seed}: with rock above the cave"
            );
            assert!(grid.walkable(m.foot));
            assert!(
                nav.reach(m.foot, m.cave),
                "seed {seed}: reachable in the region graph"
            );
            // In, and out again.
            let inward = walk(grid, m.foot, m.cave);
            let outward = walk(grid, m.cave, m.foot);
            assert_eq!(inward[0], m.foot);
            assert_eq!(
                inward[1], m.entrance,
                "seed {seed}: the first step in from the foot is the entrance"
            );
            assert_eq!(*inward.last().unwrap(), m.cave);
            assert_eq!(*outward.last().unwrap(), m.foot);
            for w in inward.windows(2) {
                assert!(
                    (w[0].x - w[1].x).abs() <= 1
                        && (w[0].y - w[1].y).abs() <= 1
                        && (w[0].z - w[1].z).abs() <= 1
                        && w[0].dist(w[1]) <= 5,
                    "no teleports: {} -> {}",
                    w[0],
                    w[1]
                );
            }
        }
    }
    assert!(mouths >= 8, "mouths are findable: {mouths} in ten valleys");
}

#[test]
fn a_mouth_is_cut_through_rock_and_roofed() {
    for seed in SEEDS {
        let g = generate(seed, &GenParams::default());
        for m in &g.cave_mouths {
            let grid = &g.grid;
            let cells = passage(grid, m);
            assert!(!cells.is_empty(), "seed {seed}: there is a passage");
            assert_eq!(
                cells[0], m.entrance,
                "seed {seed}: the way in starts at the entrance"
            );
            for c in &cells {
                let t = grid.get(*c);
                assert!(
                    matches!(t.shape, Shape::Open | Shape::RampUp),
                    "seed {seed}: {c} is cut floor or a step down, not {:?}",
                    t.shape
                );
                // Rock under every level stretch.
                if t.shape == Shape::Open {
                    assert_eq!(
                        grid.get(c.offset(0, 0, -1)).shape,
                        Shape::Wall,
                        "seed {seed}: {c} has a floor"
                    );
                }
                // Rock over every cell of the passage, the entrance included.
                assert_eq!(
                    grid.get(c.offset(0, 0, 1)).shape,
                    Shape::Wall,
                    "seed {seed}: {c} has a roof"
                );
            }
        }
    }
}

#[test]
fn most_of_the_hillside_stays_closed() {
    // Rare: at most three to a valley, none in some valleys, and far from every face.
    let (mut none, mut most) = (0, 0);
    for seed in SEEDS {
        let g = generate(seed, &GenParams::default());
        let n = g.cave_mouths.len();
        most = most.max(n);
        if n == 0 {
            none += 1;
        }
        // Apart from one another.
        for (i, a) in g.cave_mouths.iter().enumerate() {
            for b in &g.cave_mouths[i + 1..] {
                assert!(
                    a.foot.dist(b.foot) >= 24,
                    "seed {seed}: two mouths side by side"
                );
            }
        }
    }
    assert!(most <= 3, "no more than three to a valley, got {most}");
    assert!(none >= 1, "some valleys have no mouth");
    // And an ordinary house-building people still builds houses on the surface, not hollows.
    let human = species::library().iter().find(|s| s.id == "human").unwrap();
    assert_eq!(HomeForm::of(&human.habitat.builds), HomeForm::House);
}

#[test]
fn the_same_seed_cuts_the_same_mouths() {
    for seed in [1, 2, 11] {
        let a = generate(seed, &GenParams::default());
        let b = generate(seed, &GenParams::default());
        assert_eq!(a.cave_mouths, b.cave_mouths, "seed {seed}");
        for m in &a.cave_mouths {
            assert_eq!(a.grid.get(m.entrance), b.grid.get(m.entrance));
        }
    }
}

#[test]
fn no_caves_no_mouths() {
    // The no-entrance control: a world without caves has nothing to open into.
    let p = GenParams {
        cave_density: 0.0,
        ..GenParams::default()
    };
    for seed in SEEDS {
        assert!(generate(seed, &p).cave_mouths.is_empty(), "seed {seed}");
    }
}
