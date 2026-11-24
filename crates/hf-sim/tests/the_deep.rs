//! The deep (Phase 18): three strata of named rock by depth (marble in the middle under
//! limestone since C32), ore that follows the rock (coal
//! shallow, gems only in the deep basalt), and named caverns with fungus forests and pools.

use hf_sim::ids::TilePos;
use hf_sim::plants::PlantKind;
use hf_sim::world::gen::{strata, GenParams};
use hf_sim::world::{Material, Shape};
use hf_sim::World;

fn world(seed: u64) -> World {
    World::generate(seed, GenParams::default())
}

#[test]
fn rock_changes_with_depth() {
    let p = GenParams::default();
    let (top, deep) = strata::breaks(&p);
    for seed in [1u64, 5, 9] {
        let w = world(seed);
        let g = &w.grid;
        let mut seen = [false; 3];
        for (x, y) in [(20, 20), (64, 64), (100, 30), (40, 110)] {
            let c = g.col(x, y);
            for z in p.z_min..g.ground_z(x, y) {
                let m = g.rock_at(c, z);
                let band =
                    if [Material::LIMESTONE, Material::SANDSTONE, Material::COAL].contains(&m) {
                        0
                    } else if [Material::GRANITE, Material::SLATE, Material::MARBLE].contains(&m) {
                        // Marble (C32) is the middle stratum baked, and only under limestone.
                        if m == Material::MARBLE {
                            assert_eq!(g.rock_at(c, g.ground_z(x, y) - 1), Material::LIMESTONE);
                        }
                        1
                    } else {
                        // #267: the deep is basalt, gabbro or gneiss by region.
                        assert!(
                            [Material::BASALT, Material::GABBRO, Material::GNEISS].contains(&m),
                            "seed {seed} ({x},{y},{z}): {m:?}"
                        );
                        2
                    };
                seen[band] = true;
                // The breaks lie within a step of the same height everywhere.
                match band {
                    0 => assert!(z >= top - 1),
                    1 => assert!(z <= top + 1 && z >= deep - 1),
                    _ => assert!(z <= deep + 1),
                }
            }
        }
        assert_eq!(seen, [true; 3], "seed {seed}: all three strata");
    }
}

#[test]
fn middle_stratum_reads_a_sequence_as_a_shaft_digs() {
    // #267: the middle stratum splits into sub-bands alternating granite and slate, so
    // digging down crosses a sequence of named rock rather than one rock to the deep.
    let p = GenParams::default();
    let (top, deep) = strata::breaks(&p);
    for seed in 1..7u64 {
        let w = world(seed);
        let g = &w.grid;
        let (mut banded, mut both_rocks) = (0usize, false);
        for y in (0..g.height).step_by(7) {
            for x in (0..g.width).step_by(7) {
                let c = g.col(x, y);
                let Some(bands) = g.bands.get(c) else {
                    continue;
                };
                if bands.is_empty() {
                    continue;
                }
                banded += 1;
                // Every band's top lies strictly inside the middle stratum, deepest first,
                // each band a different rock from the one above it.
                for (i, &(rock, btop)) in bands.iter().enumerate() {
                    let btop = btop as i32;
                    assert!(
                        btop > deep && btop < top,
                        "seed {seed} ({x},{y}): band top z{btop} inside the middle stratum z{deep}..z{top}"
                    );
                    if i == 0 {
                        assert_ne!(
                            rock, g.strata[c].0,
                            "seed {seed} ({x},{y}): the first sub-band is the other hard rock"
                        );
                    } else {
                        assert_ne!(
                            rock,
                            bands[i - 1].0,
                            "seed {seed} ({x},{y}): sub-bands alternate"
                        );
                        assert!(
                            btop > bands[i - 1].1 as i32,
                            "seed {seed} ({x},{y}): ordered"
                        );
                    }
                }
                // The rock below a band's top is the band's rock, not the shallow one.
                let (rock, btop) = *bands.last().unwrap();
                assert_eq!(g.rock_at(c, btop as i32), rock, "seed {seed} ({x},{y})");
                let (deepest, dbottom) = bands[0];
                assert_eq!(
                    g.rock_at(c, dbottom as i32 - 1),
                    deepest,
                    "seed {seed} ({x},{y}): the deepest band reaches the deep break"
                );
                // Somewhere a shaft crosses both hard rocks inside the middle stratum.
                let rocks: std::collections::BTreeSet<_> =
                    (deep..=top).map(|z| g.rock_at(c, z)).collect();
                if rocks.contains(&Material::GRANITE) && rocks.contains(&Material::SLATE) {
                    both_rocks = true;
                }
            }
        }
        assert!(banded > 0, "seed {seed}: sub-bands somewhere in the valley");
        assert!(both_rocks, "seed {seed}: a shaft crosses granite and slate");
    }
}

#[test]
fn bands_are_deterministic_and_absent_in_old_grids() {
    // The same seed makes the same sub-bands, and a world loaded from before them (no
    // `bands` rows) reads the single middle rock it was generated with.
    for seed in [1u64, 5, 9] {
        let a = world(seed);
        let b = world(seed);
        assert_eq!(a.grid.bands, b.grid.bands, "seed {seed}: same sub-bands");
    }
    let mut w = world(1);
    w.grid.bands = Vec::new();
    let g = &w.grid;
    let c = g.col(64, 64);
    let (_, top, deep) = g.strata[c];
    // Strictly inside the middle stratum: the deep break's own level is the deep rock.
    for z in (deep as i32 + 1)..=top as i32 {
        assert_eq!(
            g.rock_at(c, z),
            g.strata[c].0,
            "without sub-bands the middle stratum is one rock (z{z})"
        );
    }
}

#[test]
fn ore_follows_the_rock() {
    let p = GenParams::default();
    let (top, deep) = strata::breaks(&p);
    for seed in 1..7u64 {
        let w = world(seed);
        let g = &w.grid;
        let (mut gems, mut coal) = (0, 0);
        for z in g.z_min..=g.z_max {
            for y in 0..g.height {
                for x in 0..g.width {
                    let m = g.get(TilePos::new(x, y, z)).material;
                    if m == Material::GEM_ROCK {
                        gems += 1;
                        assert!(z <= deep + 1, "gems lie only in the deep: z {z}");
                    } else if m == Material::COAL {
                        coal += 1;
                        assert!(z >= top - 3, "coal lies in the upper rock: z {z}");
                    }
                }
            }
        }
        assert!(gems >= 8, "seed {seed}: {gems} gem tiles");
        let _ = coal;
    }
}

#[test]
fn caverns_with_forests_and_pools() {
    let mut pools = 0;
    for seed in 1..7u64 {
        let w = world(seed);
        let g = &w.grid;
        assert!(g.caverns.len() >= 2, "seed {seed}");
        let mut names: Vec<&String> = g.caverns.iter().map(|c| &c.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(
            names.len(),
            g.caverns.len(),
            "every cavern has its own name"
        );
        for c in &g.caverns {
            pools += c.pool as u32;
            let mut floor = 0;
            for y in c.centre.y - c.ry..=c.centre.y + c.ry {
                for x in c.centre.x - c.rx..=c.centre.x + c.rx {
                    let p = TilePos::new(x, y, c.centre.z);
                    if c.contains(p) && g.walkable(p) {
                        floor += 1;
                    }
                    if c.contains(p.offset(0, 0, 1)) {
                        assert_eq!(
                            g.get(p.offset(0, 0, 1)).shape,
                            Shape::Open,
                            "two levels high"
                        );
                    }
                }
            }
            assert!(floor >= 60, "{} has {floor} floor tiles", c.name);
            let forest = w
                .plants
                .iter()
                .filter(|p| p.kind == PlantKind::FungusTree && c.contains(p.pos))
                .count();
            assert!(forest >= 8, "{} has {forest} fungus trees", c.name);
        }
        // Fungus trees grow only in the caverns.
        for p in w.plants.iter().filter(|p| p.kind == PlantKind::FungusTree) {
            assert!(g.caverns.iter().any(|c| c.contains(p.pos)));
        }
    }
    assert!(pools >= 3, "some caverns hold a pool: {pools}");
}

use hf_sim::agents::Species;
use hf_sim::events::EventKind;
use hf_sim::time::TICKS_PER_HOUR;

#[test]
fn creatures_of_the_deep() {
    for seed in 1..6u64 {
        let w = World::opening_world(seed);
        for c in &w.grid.caverns {
            assert!(
                w.agents
                    .iter()
                    .any(|a| a.species == Species::Bat && c.contains(a.pos)),
                "seed {seed}: bats roost in {}",
                c.name
            );
        }
        let crawlers: Vec<_> = w
            .agents
            .iter()
            .filter(|a| a.species == Species::Crawler)
            .collect();
        assert!(crawlers.len() <= 1, "seed {seed}");
        if let Some(c) = crawlers.first() {
            let deepest = w
                .grid
                .caverns
                .iter()
                .filter(|c| c.deep)
                .min_by_key(|c| (c.centre.z, c.centre.x, c.centre.y))
                .unwrap();
            assert!(
                deepest.contains(c.pos),
                "the crawler keeps the deepest deep cavern"
            );
        }
        let burrowers: Vec<_> = w
            .agents
            .iter()
            .filter(|a| a.species == Species::Burrower)
            .collect();
        assert_eq!(burrowers.len(), 1, "seed {seed}");
        assert!(hf_sim::underground::below(&w, burrowers[0].pos));
    }
}

use hf_sim::world::gen::caverns::Cavern;
use hf_sim::world::{Tile, TileFlags};

fn constructed_cavern(
    name: &str,
    centre: TilePos,
    pool: bool,
    deep: bool,
    underworld: bool,
) -> Cavern {
    Cavern {
        name: name.into(),
        centre,
        rx: 4,
        ry: 4,
        pool,
        deep,
        opened: false,
        underworld,
        breach: None,
        sealed: false,
        gate_of: None,
        keeper: None,
    }
}

fn constructed_cavern_world(caverns: Vec<Cavern>) -> World {
    let mut w = World::generate(19, GenParams::small());
    w.grid.caverns = caverns;
    for c in w.grid.caverns.clone() {
        for y in c.centre.y - c.ry..=c.centre.y + c.ry {
            for x in c.centre.x - c.rx..=c.centre.x + c.rx {
                let p = TilePos::new(x, y, c.centre.z);
                if c.contains(p) {
                    w.grid
                        .set(p.offset(0, 0, -1), Tile::solid(Material::GRANITE));
                    w.grid.set(p, Tile::AIR);
                }
            }
        }
    }
    w
}

fn fauna_in(w: &World, c: &Cavern, species: Species) -> usize {
    w.agents
        .iter()
        .filter(|a| a.alive && a.species == species && c.contains(a.pos))
        .count()
}

#[test]
fn constructed_caverns_scale_bats_with_pool_and_depth() {
    let plain = constructed_cavern("plain", TilePos::new(12, 12, -2), false, false, false);
    let pool = constructed_cavern("pool", TilePos::new(40, 12, -2), true, false, false);
    let deep = constructed_cavern("deep", TilePos::new(12, 40, -5), false, true, false);
    let both = constructed_cavern("both", TilePos::new(40, 40, -6), true, true, true);
    let mut w = constructed_cavern_world(vec![
        plain.clone(),
        pool.clone(),
        deep.clone(),
        both.clone(),
    ]);

    // Pools remove most of the eligible floor, but leave more dry tiles than the requested roost.
    for c in [&pool, &both] {
        let mut dry = 0;
        for y in c.centre.y - c.ry..=c.centre.y + c.ry {
            for x in c.centre.x - c.rx..=c.centre.x + c.rx {
                let p = TilePos::new(x, y, c.centre.z);
                if c.contains(p) {
                    dry += 1;
                    if dry > 8 {
                        w.grid.set(
                            p,
                            Tile {
                                flags: TileFlags(TileFlags::WATER),
                                ..Tile::AIR
                            },
                        );
                    }
                }
            }
        }
    }

    hf_sim::systems::animals::populate(&mut w);
    for (c, expected) in [(&plain, 4), (&pool, 6), (&deep, 5), (&both, 7)] {
        assert_eq!(fauna_in(&w, c, Species::Bat), expected, "{}", c.name);
        for bat in w
            .agents
            .iter()
            .filter(|a| a.alive && a.species == Species::Bat && c.contains(a.pos))
        {
            assert!(w.grid.walkable(bat.pos) && !w.grid.get(bat.pos).is_water());
        }
    }
}

#[test]
fn constructed_caverns_put_the_crawler_in_the_deepest_deep_habitat() {
    let shallow = constructed_cavern(
        "lowest but shallow",
        TilePos::new(12, 12, -7),
        false,
        false,
        false,
    );
    let deep = constructed_cavern("deep", TilePos::new(40, 12, -4), false, true, false);
    let deepest = constructed_cavern("deepest deep", TilePos::new(28, 40, -6), false, true, false);
    let mut w = constructed_cavern_world(vec![shallow.clone(), deep.clone(), deepest.clone()]);

    hf_sim::systems::animals::populate(&mut w);
    assert_eq!(fauna_in(&w, &shallow, Species::Crawler), 0);
    assert_eq!(fauna_in(&w, &deep, Species::Crawler), 0);
    assert_eq!(fauna_in(&w, &deepest, Species::Crawler), 1);
    assert_eq!(
        w.agents
            .iter()
            .filter(|a| a.alive && a.species == Species::Crawler)
            .count(),
        1
    );
}

#[test]
fn constructed_caverns_prefer_the_underworld_for_the_burrower() {
    let underworld = constructed_cavern("underworld", TilePos::new(12, 12, -3), false, true, true);
    let deeper = constructed_cavern(
        "deeper ordinary cave",
        TilePos::new(40, 40, -6),
        false,
        true,
        false,
    );
    let mut w = constructed_cavern_world(vec![deeper.clone(), underworld.clone()]);

    hf_sim::systems::animals::populate(&mut w);
    assert_eq!(fauna_in(&w, &underworld, Species::Burrower), 1);
    assert_eq!(fauna_in(&w, &deeper, Species::Burrower), 0);
}

#[test]
fn malformed_world_without_caverns_uses_the_burrower_rock_fallback() {
    let mut w = constructed_cavern_world(Vec::new());
    hf_sim::systems::animals::populate(&mut w);

    assert_eq!(
        w.agents
            .iter()
            .filter(|a| a.species == Species::Bat)
            .count(),
        0
    );
    assert_eq!(
        w.agents
            .iter()
            .filter(|a| a.species == Species::Crawler)
            .count(),
        0
    );
    let burrower = w
        .agents
        .iter()
        .find(|a| a.species == Species::Burrower)
        .expect("rock fallback preserves the burrower");
    assert!(hf_sim::underground::below(&w, burrower.pos));
    assert!(w.grid.walkable(burrower.pos));
}

#[test]
fn cavern_without_dry_floor_skips_cavern_fauna_and_uses_the_rock_fallback() {
    let c = constructed_cavern("flooded", TilePos::new(28, 28, -5), true, true, true);
    let mut w = constructed_cavern_world(vec![c.clone()]);
    for y in c.centre.y - c.ry..=c.centre.y + c.ry {
        for x in c.centre.x - c.rx..=c.centre.x + c.rx {
            let p = TilePos::new(x, y, c.centre.z);
            if c.contains(p) {
                w.grid.set(
                    p,
                    Tile {
                        flags: TileFlags(TileFlags::WATER | TileFlags::DEEP),
                        ..Tile::AIR
                    },
                );
            }
        }
    }

    hf_sim::systems::animals::populate(&mut w);
    assert_eq!(fauna_in(&w, &c, Species::Bat), 0);
    assert_eq!(fauna_in(&w, &c, Species::Crawler), 0);
    assert_eq!(fauna_in(&w, &c, Species::Burrower), 0);
    let burrower = w
        .agents
        .iter()
        .find(|a| a.species == Species::Burrower)
        .expect("a flooded cavern does not remove the species");
    assert!(hf_sim::underground::below(&w, burrower.pos));
    assert!(w.grid.walkable(burrower.pos) && !w.grid.get(burrower.pos).is_water());
}

/// Someone alone in the crawler's cavern, a few tiles from it, holding that force answers
/// force; with `lamp`, a lit lamp beside them. Returns the crawler's attacks over six hours.
fn alone_with_the_crawler(seed: u64, lamp: bool) -> Option<Vec<bool>> {
    let mut w = World::opening_world(seed);
    let c = w.agents.iter().find(|a| a.species == Species::Crawler)?.pos;
    let at = (2..6)
        .flat_map(|d| [(d, 0), (-d, 0), (0, d), (0, -d)])
        .map(|(dx, dy)| c.offset(dx, dy, 0))
        .find(|p| w.grid.walkable(*p) && !w.grid.get(*p).is_water())?;
    let who = w.spawn_agent(Species::Person, at);
    if let Some(a) = w.agents.iter_mut().find(|a| a.id == who) {
        a.beliefs.hold(
            hf_sim::mind::beliefs::Topic::Violence,
            0.7,
            0.9,
            hf_sim::mind::beliefs::Source::Event(hf_sim::ids::EventId(0)),
            0,
        );
    }
    if lamp {
        let sid = w.alloc_structure_id();
        w.structures.push(hf_sim::structures::Structure {
            id: sid,
            kind: hf_sim::structures::StructureKind::Lamp,
            pos: at.offset(1, 0, 0),
            material: Material::WOOD,
            progress: 1.0,
            active: true,
            condition: 1.0,
            owner: None,
            quality: 0.5,
        });
    }
    w.ensure_nav();
    hf_sim::underground::rebuild_lights(&mut w);
    let crawler = w.agents.iter().find(|a| a.species == Species::Crawler)?.id;
    let mut attacks = Vec::new();
    for _ in 0..6 * TICKS_PER_HOUR {
        w.step();
        for e in &w.last_events {
            if let EventKind::Attacked {
                animal, driven_off, ..
            } = e.kind
            {
                if animal == crawler {
                    attacks.push(driven_off);
                }
            }
        }
        // Keep them where they were put: the test is about the crawler, not their wandering.
        if let Some(a) = w.agents.iter_mut().find(|a| a.id == who) {
            if a.pos.dist(at) > 3 {
                a.pos = at;
                a.path.clear();
            }
        }
    }
    Some(attacks)
}

#[test]
fn the_crawler_hunts_in_the_dark_and_is_fought_off() {
    let (mut dark, mut fought, mut lit, mut tried) = (0, 0, 0, 0);
    for seed in 1..10u64 {
        let Some(a) = alone_with_the_crawler(seed, false) else {
            continue;
        };
        tried += 1;
        dark += a.len();
        fought += a.iter().filter(|d| **d).count();
        lit += alone_with_the_crawler(seed, true).unwrap_or_default().len();
    }
    assert!(tried >= 4, "{tried} valleys had a crawler to test");
    assert!(dark >= 2, "it went for people in the dark: {dark}");
    assert!(fought >= 1, "and someone stood and drove it off");
    assert_eq!(lit, 0, "it never went for anyone a lamp reached");
}

#[test]
fn cave_fauna_variants_keep_their_habitats() {
    for seed in 1..6u64 {
        let w = World::opening_world(seed);
        let crawler_cavern = w
            .grid
            .caverns
            .iter()
            .filter(|c| c.deep)
            .min_by_key(|c| (c.centre.z, c.centre.x, c.centre.y))
            .map(|c| (c.centre.x, c.centre.y, c.centre.z));
        let bears: Vec<_> = w
            .agents
            .iter()
            .filter(|a| a.species == Species::BlindCaveBear)
            .collect();
        assert!(bears.len() <= 1, "seed {seed}: at most one blind bear");
        if let Some(b) = bears.first() {
            let home = w
                .grid
                .caverns
                .iter()
                .find(|c| c.contains(b.pos))
                .expect("seed {seed}: the blind bear is in a cavern");
            assert!(home.deep, "seed {seed}: the blind bear keeps a deep cavern");
            assert_ne!(
                crawler_cavern,
                Some((home.centre.x, home.centre.y, home.centre.z)),
                "seed {seed}: not the crawler's own cavern"
            );
        }
        let crocs: Vec<_> = w
            .agents
            .iter()
            .filter(|a| a.species == Species::CaveCroc)
            .collect();
        assert!(crocs.len() <= 1, "seed {seed}: at most one cave crocodile");
        if let Some(c) = crocs.first() {
            let home = w
                .grid
                .caverns
                .iter()
                .find(|cv| cv.contains(c.pos))
                .expect("seed {seed}: the cave crocodile is in a cavern");
            assert!(
                home.pool,
                "seed {seed}: the cave crocodile keeps a pool cavern"
            );
            let beside = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .any(|&(dx, dy)| w.grid.get(c.pos.offset(dx, dy, 0)).is_water());
            assert!(beside, "seed {seed}: it lies up at the water's edge");
        }
    }
}

/// Someone beside the blind bear of their valley, a few tiles off in its dark cavern; with
/// `lamp`, a lit lamp beside them. Returns the bear's attacks over six hours.
fn alone_with_the_blind_bear(seed: u64, lamp: bool) -> Option<Vec<bool>> {
    let mut w = World::opening_world(seed);
    let c = w
        .agents
        .iter()
        .find(|a| a.species == Species::BlindCaveBear)?
        .pos;
    let at = (1..=3)
        .flat_map(|d| [(d, 0), (-d, 0), (0, d), (0, -d)])
        .map(|(dx, dy)| c.offset(dx, dy, 0))
        .find(|p| w.grid.walkable(*p) && !w.grid.get(*p).is_water())?;
    let who = w.spawn_agent(Species::Person, at);
    if lamp {
        let sid = w.alloc_structure_id();
        w.structures.push(hf_sim::structures::Structure {
            id: sid,
            kind: hf_sim::structures::StructureKind::Lamp,
            pos: at.offset(1, 0, 0),
            material: Material::WOOD,
            progress: 1.0,
            active: true,
            condition: 1.0,
            owner: None,
            quality: 0.5,
        });
    }
    w.ensure_nav();
    hf_sim::underground::rebuild_lights(&mut w);
    let bear = w
        .agents
        .iter()
        .find(|a| a.species == Species::BlindCaveBear)?
        .id;
    let mut attacks = Vec::new();
    for _ in 0..6 * TICKS_PER_HOUR {
        w.step();
        for e in &w.last_events {
            if let EventKind::Attacked {
                animal, driven_off, ..
            } = e.kind
            {
                if animal == bear {
                    attacks.push(driven_off);
                }
            }
        }
        if let Some(a) = w.agents.iter_mut().find(|a| a.id == who) {
            if a.pos.dist(at) > 3 {
                a.pos = at;
                a.path.clear();
            }
        }
    }
    Some(attacks)
}

#[test]
fn the_blind_bear_strikes_only_in_the_dark() {
    let (mut dark, mut lit, mut tried) = (0, 0, 0);
    for seed in 1..10u64 {
        let Some(a) = alone_with_the_blind_bear(seed, false) else {
            continue;
        };
        tried += 1;
        dark += a.len();
        lit += alone_with_the_blind_bear(seed, true)
            .unwrap_or_default()
            .len();
    }
    assert!(tried >= 2, "{tried} valleys had a blind bear to test");
    assert!(
        dark >= 1,
        "it mauled someone who came at it in the dark: {dark}"
    );
    assert_eq!(lit, 0, "it never struck where a lamp reached");
}

/// Someone within reach of the valley's cave crocodile; `away` puts them where no tile
/// beside them is water (not come to drink). Returns the crocodile's attacks over six hours.
fn beside_the_cave_croc(seed: u64, away: bool) -> Option<Vec<bool>> {
    let mut w = World::opening_world(seed);
    let c = w
        .agents
        .iter()
        .find(|a| a.species == Species::CaveCroc)?
        .pos;
    let beside_water = |w: &World, p: TilePos| {
        [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .iter()
            .any(|&(dx, dy)| w.grid.get(p.offset(dx, dy, 0)).is_water())
    };
    let at = (-2..=2)
        .flat_map(|dx| (-2..=2).map(move |dy| (dx, dy)))
        .filter(|&(dx, dy)| dx != 0 || dy != 0)
        .map(|(dx, dy)| c.offset(dx, dy, 0))
        .find(|p| {
            w.grid.walkable(*p) && !w.grid.get(*p).is_water() && beside_water(&w, *p) != away
        })?;
    let who = w.spawn_agent(Species::Person, at);
    w.ensure_nav();
    hf_sim::underground::rebuild_lights(&mut w);
    let croc = w.agents.iter().find(|a| a.species == Species::CaveCroc)?.id;
    let mut attacks = Vec::new();
    for _ in 0..6 * TICKS_PER_HOUR {
        w.step();
        for e in &w.last_events {
            if let EventKind::Attacked {
                animal, driven_off, ..
            } = e.kind
            {
                if animal == croc {
                    attacks.push(driven_off);
                }
            }
        }
        // Hold them exactly where they were put: within reach, the seize turns on whether
        // water is beside them, not on where they drifted to.
        if let Some(a) = w.agents.iter_mut().find(|a| a.id == who) {
            if a.pos != at {
                a.pos = at;
                a.path.clear();
            }
        }
    }
    Some(attacks)
}

#[test]
fn the_cave_croc_seizes_at_the_waters_edge() {
    let (mut edge, mut dry, mut tried) = (0, 0, 0);
    for seed in 1..10u64 {
        let Some(a) = beside_the_cave_croc(seed, false) else {
            continue;
        };
        tried += 1;
        edge += a.len();
        dry += beside_the_cave_croc(seed, true).unwrap_or_default().len();
    }
    assert!(tried >= 2, "{tried} valleys had a cave crocodile to test");
    assert!(
        edge >= 1,
        "it seized someone who came down to the pool: {edge}"
    );
    assert_eq!(dry, 0, "it never seized anyone away from the water");
}

#[test]
fn the_dark_below_is_felt_and_a_lamp_lifts_it() {
    let mut w = world(4);
    // Mid-morning, so nobody is asleep.
    w.meta.tick = 10 * TICKS_PER_HOUR;
    w.rebuild_caches();
    // A dug room in the rock, two levels down from the founding site.
    let fs = w.meta.founding_site;
    let z = w.grid.ground_z(fs.x, fs.y) - 2;
    for y in fs.y..fs.y + 3 {
        for x in fs.x..fs.x + 3 {
            let p = TilePos::new(x, y, z);
            w.grid.set(p, hf_sim::world::Tile::AIR);
            w.dug.insert(p);
        }
    }
    let at = TilePos::new(fs.x + 1, fs.y + 1, z);
    let who = w.spawn_agent(Species::Person, at);
    w.ensure_nav();
    hf_sim::underground::rebuild_lights(&mut w);
    assert!(hf_sim::underground::dark(&w, at));
    for _ in 0..400 {
        w.step();
    }
    let a = w.agents.iter().find(|a| a.id == who).unwrap();
    assert!(a
        .thoughts
        .list
        .iter()
        .any(|t| t.kind == hf_sim::mind::ThoughtKind::Dark));
    let sid = w.alloc_structure_id();
    w.structures.push(hf_sim::structures::Structure {
        id: sid,
        kind: hf_sim::structures::StructureKind::Lamp,
        pos: TilePos::new(fs.x, fs.y, z),
        material: Material::WOOD,
        progress: 1.0,
        active: true,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
    hf_sim::underground::rebuild_lights(&mut w);
    assert!(!hf_sim::underground::dark(&w, at), "the lamp reaches them");
}

/// A straight gallery `len` tiles long, eight levels under the founding site, walled, floored
/// and roofed in granite so nothing but the gallery is open. Returns its tiles, west to east.
fn gallery(w: &mut World, len: i32) -> Vec<TilePos> {
    let fs = w.meta.founding_site;
    let z = w.grid.ground_z(fs.x, fs.y) - 8;
    for dz in -1..=1 {
        for y in fs.y - 2..=fs.y + 2 {
            for x in fs.x - 2..fs.x + len + 2 {
                let p = TilePos::new(x, y, z + dz);
                w.grid.set(p, hf_sim::world::Tile::solid(Material::GRANITE));
                w.dug.remove(&p);
            }
        }
    }
    let tiles: Vec<TilePos> = (0..len).map(|d| TilePos::new(fs.x + d, fs.y, z)).collect();
    for &p in &tiles {
        w.grid.set(p, hf_sim::world::Tile::AIR);
        w.dug.insert(p);
    }
    tiles
}

#[test]
fn a_collapse_hurts_each_person_once_and_throws_them_clear() {
    let mut w = world(4);
    let g = gallery(&mut w, 6);
    // The roof comes down over the four tiles east of the first: someone stands at the west
    // end of the fall, open ground behind them.
    let fall: Vec<TilePos> = g[1..5].to_vec();
    let who = w.spawn_agent(Species::Person, fall[0]);
    let i = w.agents.iter().position(|a| a.id == who).unwrap();
    w.events.pending.clear();
    hf_sim::underground::collapse(&mut w, i, fall[0], &fall);

    let a = &w.agents[i];
    assert!(a.alive);
    assert_eq!(a.body.wounds.len(), 1, "hurt once");
    assert!(!fall.contains(&a.pos), "not left inside the rubble");
    assert!(w.grid.walkable(a.pos), "on open ground: {}", a.pos);
    assert_eq!(a.pos, g[0], "the nearest open ground beside the fall");
    let wounded = w
        .events
        .pending
        .iter()
        .filter(|e| matches!(&e.kind, EventKind::Wounded { agent, .. } if *agent == who))
        .count();
    assert_eq!(wounded, 1);
    let named: Vec<_> = w
        .events
        .pending
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Collapse { hurt, tiles, .. } => Some((hurt.clone(), *tiles)),
            _ => None,
        })
        .collect();
    assert_eq!(
        named,
        vec![(vec![who], 4)],
        "one collapse, naming them once"
    );
    for t in &fall {
        assert!(w.grid.get(*t).shape.is_solid(), "{t} is rubble");
    }
}

#[test]
fn a_collapse_with_no_open_ground_near_buries_whoever_is_under_it() {
    let mut w = world(4);
    let g = gallery(&mut w, 9);
    // The whole gallery comes down; someone in its middle is four tiles from either end.
    let who = w.spawn_agent(Species::Person, g[4]);
    let i = w.agents.iter().position(|a| a.id == who).unwrap();
    w.events.pending.clear();
    hf_sim::underground::collapse(&mut w, i, g[4], &g);
    let a = &w.agents[i];
    assert!(!a.alive);
    assert_eq!(a.death.as_deref(), Some("buried under the fallen rock"));
    let died = w
        .events
        .pending
        .iter()
        .filter(|e| matches!(&e.kind, EventKind::Died { agent, .. } if *agent == who))
        .count();
    assert_eq!(died, 1);
}

#[test]
fn the_burrower_turns_back_from_gem_rock() {
    let mut w = World::opening_world(3);
    let i = w
        .agents
        .iter()
        .position(|a| a.species == Species::Burrower)
        .unwrap();
    let at = w.agents[i].pos;
    // Gem-bearing basalt on every side it could dig.
    let sides: Vec<TilePos> = [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .iter()
        .map(|(dx, dy)| at.offset(*dx, *dy, 0))
        .collect();
    for &p in &sides {
        w.grid
            .set(p, hf_sim::world::Tile::solid(Material::GEM_ROCK));
    }
    for k in 1..=12u64 {
        w.meta.tick = k * 2 * TICKS_PER_HOUR;
        hf_sim::underground::step(&mut w);
        assert_eq!(w.agents[i].pos, at, "it stays out of the gem rock");
    }
    for p in sides {
        assert_eq!(w.grid.get(p).material, Material::GEM_ROCK);
    }
}

// ---- A party up hurt from below carries a wound (OST-1311) ----

/// A world where a hurt party of three has just come up: the first two hurt, the third not.
fn hurt_party() -> (World, Vec<hf_sim::ids::AgentId>, String) {
    use hf_sim::taken::{Outcome, Stage, Taken};
    let mut w = world(4);
    // A bare world has no community: the party still comes up at the way down.
    let stand = w
        .agents
        .first()
        .map_or_else(|| TilePos::new(10, 10, 0), |a| a.pos);
    let child = w.spawn_agent(Species::Person, stand);
    let party: Vec<_> = (0..3)
        .map(|_| w.spawn_agent(Species::Person, stand))
        .collect();
    // Everyone is below: out of the valley and into the away list.
    for id in party.iter().chain([&child]) {
        let i = w.agents.iter().position(|a| a.id == *id).unwrap();
        let a = w.agents.remove(i);
        w.overworld.away.push(a);
    }
    let name = w
        .overworld
        .away
        .iter()
        .find(|a| a.id == child)
        .unwrap()
        .name
        .clone();
    let now = w.meta.tick;
    w.gone_below.push(Taken {
        child,
        community: hf_sim::ids::CommunityId(0),
        site: "the deep".into(),
        cavern: 0,
        taken: 1,
        party: party.clone(),
        stand,
        since: now,
        back_at: now,
        stage: Stage::Down,
        opener: None,
        accused: Vec::new(),
        going_back: false,
    });
    w.events.pending.clear();
    hf_sim::taken::settle(&mut w, 0, Outcome::Hurt);
    (w, party, name)
}

#[test]
fn hurt_below_is_a_wound() {
    let (w, party, name) = hurt_party();
    let wounds = |id| {
        w.agents
            .iter()
            .find(|a| a.id == id)
            .map(|a| a.body.wounds.len())
            .unwrap()
    };
    assert_eq!(wounds(party[0]), 1, "the first has one wound");
    assert_eq!(wounds(party[1]), 1, "the second has one wound");
    assert_eq!(wounds(party[2]), 0, "the third is not");
    let cause = format!("hurt below, going after {name}");
    let named = w
        .events
        .pending
        .iter()
        .filter(|e| matches!(&e.kind, EventKind::Wounded { cause: c, .. } if *c == cause))
        .count();
    assert_eq!(named, 2, "two wounds, each naming the child");
}

#[test]
fn the_same_seed_gives_the_same_wounds_below() {
    let wounds = |w: &World, id| {
        w.agents
            .iter()
            .find(|a| a.id == id)
            .map(|a| format!("{:?}", a.body.wounds))
            .unwrap()
    };
    let (a, pa, _) = hurt_party();
    let (b, pb, _) = hurt_party();
    for k in 0..3 {
        assert_eq!(wounds(&a, pa[k]), wounds(&b, pb[k]));
    }
}
