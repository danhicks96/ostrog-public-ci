//! Issues #44, #52, #58: the art library is reachable by the world generator. Biomes choose the
//! ground and plant families by place, the world's character makes seeds differ, every
//! registered family is reachable over a set of seeds, the new plant kinds grow, a felled tree
//! leaves a stump, and the client has a sprite for every kind and family the simulation sends.

use std::collections::BTreeSet;

use hf_sim::agents::Species;
use hf_sim::ids::TilePos;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::{Designation, DesignationKind};
use hf_sim::world::gen::biome::{census, ground_sources, Biome, Character};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{MaterialTable, TileFlags};
use hf_sim::World;

const SEEDS: std::ops::RangeInclusive<u64> = 1..=24;

fn top(w: &World, x: i32, y: i32) -> hf_sim::world::Tile {
    w.grid.get(TilePos::new(x, y, w.grid.ground_z(x, y)))
}

fn is_water(w: &World, x: i32, y: i32) -> bool {
    w.grid
        .get(TilePos::new(x, y, w.grid.ground_z(x, y) + 1))
        .is_water()
}

#[test]
fn generation_is_deterministic_per_seed() {
    for seed in [3u64, 11] {
        let a = World::generate(seed, GenParams::small());
        let b = World::generate(seed, GenParams::small());
        assert_eq!(a.grid.biome, b.grid.biome, "seed {seed}: biomes");
        for y in 0..a.grid.height {
            for x in 0..a.grid.width {
                assert_eq!(
                    top(&a, x, y),
                    top(&b, x, y),
                    "seed {seed}: ground at {x},{y}"
                );
            }
        }
        let key = |w: &World| -> Vec<(TilePos, PlantKind, u32)> {
            w.plants
                .iter()
                .map(|p| (p.pos, p.kind, p.variant))
                .collect()
        };
        assert_eq!(key(&a), key(&b), "seed {seed}: plants");
        assert_eq!(a.meta.founding_site, b.meta.founding_site);
    }
    // A different seed is a different valley.
    let a = World::generate(3, GenParams::small());
    let b = World::generate(4, GenParams::small());
    assert_ne!(a.grid.biome, b.grid.biome);
}

/// #289 (DEC-8 item 4): the loose-rock pass converts only steep bare ground, keeps
/// flat and vegetated ground, and a seed's slopes read the same every run.
#[test]
fn scree_lies_on_steep_bare_ground() {
    use hf_sim::world::gen::scree;
    use hf_sim::world::{Tile, WorldGrid};
    let p = GenParams::small();
    let (mut converted, mut runs) = (0usize, 0usize);
    for seed in 1..=6u64 {
        let mut grid = WorldGrid::new(p.width, p.height, p.z_min, p.z_max);
        // Flat ground at 2; a two-column cliff at 5 beside it (a rise of 3 for the
        // columns on both sides); one vegetated column on the cliff top; one sandy
        // column on the cliff face.
        for y in 0..p.height {
            for x in 0..p.width {
                let gz = if (10..=11).contains(&x) { 5 } else { 2 };
                let c = grid.col(x, y);
                grid.ground[c] = gz;
                let top = TilePos::new(x, y, gz);
                match (x, y) {
                    // Vegetated: the pass must leave it, however steep.
                    (11, 20) => {
                        grid.set(top, Tile::solid(hf_sim::world::Material::MEADOW));
                    }
                    // Bare sand on the steep face: it lies loose like soil does.
                    (10, 30) => {
                        grid.set(top, Tile::solid(hf_sim::world::Material::SAND));
                    }
                    _ => {}
                }
            }
        }
        // A twin, to show the pass is a pure function of (seed, grid).
        let mut repeat = grid.clone();
        let before: Vec<_> = (0..p.height)
            .flat_map(|y| (0..p.width).map(move |x| (x, y)))
            .map(|(x, y)| grid.get(TilePos::new(x, y, grid.ground_z(x, y))).material)
            .collect();
        scree::apply(seed, &p, &mut grid);
        runs += 1;
        for (i, m) in before.iter().enumerate() {
            let (x, y) = ((i as i32) % p.width, (i as i32) / p.width);
            let gz = grid.ground_z(x, y);
            let now = grid.get(TilePos::new(x, y, gz)).material;
            let rise = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .map(|(dx, dy)| (grid.ground_z(x + dx, y + dy) - gz).abs())
                .max()
                .unwrap_or(0);
            if now != *m {
                converted += 1;
                assert!(
                    rise >= 2,
                    "seed {seed} ({x},{y}): loose rock only where the ground rises steeply (rise {rise})"
                );
                assert!(
                    matches!(
                        *m,
                        hf_sim::world::Material::SOIL | hf_sim::world::Material::SAND
                    ),
                    "seed {seed} ({x},{y}): only bare ground lies loose (was {m:?})"
                );
                assert!(
                    matches!(
                        now,
                        hf_sim::world::Material::SCREE | hf_sim::world::Material::GRAVEL
                    ),
                    "seed {seed} ({x},{y}): loose rock is scree or gravel (now {now:?})"
                );
            } else {
                // Unchanged is legal anywhere: steep bare ground below the patch
                // threshold keeps its cover by design, and flat ground was never a
                // candidate. (The vegetated column is checked explicitly below.)
            }
        }
        // The vegetated cliff top keeps its meadow, however steep.
        assert_eq!(
            grid.get(TilePos::new(11, 20, 5)).material,
            hf_sim::world::Material::MEADOW,
            "seed {seed}: the pass leaves vegetated ground"
        );
        // The same seed lies loose in the same places.
        let again: Vec<_> = (0..p.height)
            .flat_map(|y| (0..p.width).map(move |x| (x, y)))
            .map(|(x, y)| grid.get(TilePos::new(x, y, grid.ground_z(x, y))).material)
            .collect();
        scree::apply(seed, &p, &mut repeat);
        let repeat_top: Vec<_> = (0..p.height)
            .flat_map(|y| (0..p.width).map(move |x| (x, y)))
            .map(|(x, y)| {
                repeat
                    .get(TilePos::new(x, y, repeat.ground_z(x, y)))
                    .material
            })
            .collect();
        assert_eq!(again, repeat_top, "seed {seed}: the pass is deterministic");
    }
    assert!(runs > 0 && converted > 0, "the pass lies loose somewhere");
}

#[test]
fn ground_families_follow_biome() {
    for seed in 1..=6u64 {
        let w = World::generate(seed, GenParams::default());
        assert_eq!(w.grid.biome.len(), (w.grid.width * w.grid.height) as usize);
        for y in 0..w.grid.height {
            for x in 0..w.grid.width {
                if is_water(&w, x, y) {
                    continue;
                }
                let b = w.grid.biome_at(x, y).unwrap();
                let m = top(&w, x, y).material;
                assert!(
                    // Loose rock is a deposit, not a biome choice: #698 lays scree and
                    // gravel on knoll and ridge-edge tops of any biome whose bare top
                    // steps to its neighbours, so a column's top can read gravel where
                    // the biome's own list would never put it (scree.rs `apply`).
                    b.grounds().contains(&m)
                        || matches!(
                            m,
                            hf_sim::world::material::Material::SCREE
                                | hf_sim::world::material::Material::GRAVEL
                        ),
                    "seed {seed} ({x},{y}): {} is not a {} ground",
                    w.grid.materials.get(m).name,
                    b.label()
                );
            }
        }
        // Plants likewise come from their biome's list (or the placement rules that are not
        // per-biome: trees, reeds by water, vines at the foot of rises, cave mushrooms, and the
        // fungus forests of the caverns, which grow below whatever biome is above them).
        for p in &w.plants {
            if matches!(
                p.kind,
                PlantKind::Tree
                    | PlantKind::Reed
                    | PlantKind::Vine
                    | PlantKind::Mushroom
                    | PlantKind::FungusTree
                    | PlantKind::BoneTree
            ) {
                continue;
            }
            let b = w.grid.biome_at(p.pos.x, p.pos.y).unwrap();
            // The other trees come from the biome's tree list (#286).
            if p.kind.is_tree() {
                assert!(
                    b.trees().iter().any(|(k, _)| *k == p.kind),
                    "seed {seed}: {:?} in {}",
                    p.kind,
                    b.label()
                );
                continue;
            }
            assert!(
                b.flora().iter().any(|(k, _)| *k == p.kind),
                "seed {seed}: {:?} in {}",
                p.kind,
                b.label()
            );
        }
        // Trees on cold ground are pines (the client draws species variant % 3; 2 is pine).
        for p in w.plants.iter().filter(|p| p.kind == PlantKind::Tree) {
            if w.grid.biome_at(p.pos.x, p.pos.y) == Some(Biome::Snowfield) {
                assert_eq!(p.variant % 1000 % 3, 2, "snowfield trees are pines");
            }
        }
    }
}

#[test]
fn every_registered_family_is_reachable() {
    let mut grounds = BTreeSet::new();
    let mut plants = BTreeSet::new();
    let mut biomes = BTreeSet::new();
    let mut goats = 0;
    for seed in SEEDS {
        let mut w = World::generate(seed, GenParams::default());
        let c = census(&w.grid, &w.plants);
        for (f, n) in &c.grounds {
            if *n > 0 {
                grounds.insert(f.clone());
            }
        }
        for (k, n) in &c.plants {
            if *n > 0 {
                plants.insert(*k as usize);
            }
        }
        for (b, n) in &c.biomes {
            if *n > 0 {
                biomes.insert(*b);
            }
        }
        hf_sim::systems::animals::populate(&mut w);
        goats += w
            .agents
            .iter()
            .filter(|a| a.species == Species::Goat)
            .count();
    }
    for b in Biome::ALL {
        assert!(biomes.contains(&b), "no seed has {}", b.label());
    }
    for (fam, _) in ground_sources() {
        assert!(
            grounds.contains(&fam),
            "ground family {fam} never generated"
        );
    }
    for fam in ["water", "deep_water"] {
        assert!(grounds.contains(fam), "{fam} never generated");
    }
    for k in PlantKind::ALL {
        // Crops are sown by settlers, a stump is left by felling, and a thistle arrives on
        // worked fields over time (all tested below and in weeds.rs).
        if k.is_crop() || k == PlantKind::Stump || k.is_weed() {
            continue;
        }
        assert!(plants.contains(&(k as usize)), "{k:?} never generated");
    }
    assert!(goats > 0, "no goats on any stony upland");
    // Every surface family the materials table names is either chosen by the generator or is
    // something else on purpose (built floors, rock, ore, water, air).
    let table = MaterialTable::default();
    let chosen: BTreeSet<String> = ground_sources().into_iter().map(|(f, _)| f).collect();
    for d in &table.defs {
        if d.kind == hf_sim::world::material::MaterialKind::Soil {
            let reachable = chosen.contains(&d.family) || d.family == "soil" || d.family == "clay";
            assert!(reachable, "soil family {} has no biome", d.family);
        }
    }
}

#[test]
fn goats_live_only_on_stony_or_cold_ground() {
    for seed in SEEDS.take(8) {
        let mut w = World::generate(seed, GenParams::default());
        hf_sim::systems::animals::populate(&mut w);
        for a in w.agents.iter().filter(|a| a.species == Species::Goat) {
            assert!(matches!(
                w.grid.biome_at(a.pos.x, a.pos.y),
                Some(Biome::Stony | Biome::Fell)
            ));
        }
    }
}

fn share(w: &World, bs: &[Biome]) -> f32 {
    let c = census(&w.grid, &w.plants);
    let land: usize = c.biomes.iter().map(|(_, n)| n).sum();
    bs.iter().map(|b| c.biome(*b)).sum::<usize>() as f32 / land as f32
}

#[test]
fn character_changes_the_valley() {
    let with = |wetness, warmth, stoniness| {
        let p = GenParams {
            character: Some(Character {
                wetness,
                warmth,
                stoniness,
            }),
            ..GenParams::default()
        };
        World::generate(5, p)
    };
    let wet = with(1.0, 0.0, -1.0);
    let dry = with(-1.0, 0.0, -1.0);
    let stony = with(-0.3, 0.0, 1.0);
    let cold = with(0.0, -1.0, 0.0);
    assert!(share(&wet, &[Biome::Fen]) > 0.3, "wet valley is fen");
    assert!(
        share(&dry, &[Biome::Fen]) < 0.05,
        "dry valley has almost no fen"
    );
    assert!(share(&dry, &[Biome::Heath]) > 0.3, "dry valley is heath");
    assert!(
        share(&stony, &[Biome::Stony]) > 0.25,
        "stony valley is stony"
    );
    assert!(
        share(&cold, &[Biome::Fell, Biome::Snowfield]) > 0.4,
        "cold valley is fell and snow"
    );
    // A wet valley holds more water than a dry one with the same relief.
    let water = |w: &World| census(&w.grid, &w.plants).ground("water");
    assert!(water(&wet) > water(&dry));
    // Same seed, same relief: the character only changes what grows and lies on it.
    assert_eq!(wet.grid.ground, dry.grid.ground);
}

#[test]
fn seeds_differ_in_character() {
    let mut dominant = BTreeSet::new();
    for seed in SEEDS {
        let w = World::generate(seed, GenParams::small());
        let c = census(&w.grid, &w.plants);
        let top = c.biomes.iter().max_by_key(|(_, n)| *n).unwrap().0;
        dominant.insert(top);
    }
    assert!(
        dominant.len() >= 4,
        "only {} distinct dominant biomes over {} seeds: {:?}",
        dominant.len(),
        SEEDS.count(),
        dominant
    );
}

#[test]
fn deep_water_lies_only_in_lake_centres() {
    for seed in SEEDS {
        let w = World::generate(seed, GenParams::default());
        let g = &w.grid;
        for y in 0..g.height {
            for x in 0..g.width {
                let t = g.get(TilePos::new(x, y, g.ground_z(x, y) + 1));
                if !t.flags.has(TileFlags::DEEP) {
                    continue;
                }
                for dy in -2..=2 {
                    for dx in -2..=2 {
                        assert!(is_water(&w, x + dx, y + dy), "deep water at a shore");
                    }
                }
            }
        }
    }
}

#[test]
fn new_plant_kinds_grow_and_feed() {
    for k in PlantKind::ALL {
        let mut p = Plant::new(TilePos::new(0, 0, 0), k, 0.0, 0);
        let ticks = (k.days_to_mature() * TICKS_PER_DAY as f32) as u64 + 60;
        // Grown in a season it bears in (#286).
        use hf_sim::time::Season;
        let season = [Season::Spring, Season::Summer, Season::Autumn]
            .into_iter()
            .find(|s| k.bears_in(*s))
            .unwrap_or(Season::Summer);
        if !k.grows() {
            p.growth = 1.0;
            p.grow(TICKS_PER_DAY * 30, 1.0, season);
            assert_eq!(p.growth, 1.0);
            assert!(!k.yields_food());
            continue;
        }
        for _ in 0..ticks / 60 + 1 {
            p.grow(60, 1.0, season);
        }
        assert!(p.mature(), "{k:?} matured in its days_to_mature");
        if k.yields_food() {
            assert!(p.fruit > 0.3, "{k:?} bears food when grown");
        }
    }
    assert!(PlantKind::Herbs.yields_food() && PlantKind::RootCrops.yields_food());
    assert!(!PlantKind::Fern.yields_food() && PlantKind::Fern.grazed());
}

#[test]
fn felled_tree_leaves_a_stump() {
    let mut w = World::generate(35, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.meta.autoplan = false;
    w.rebuild_caches();
    let id = w.spawn_agent(Species::Person, TilePos::new(30, 30, 2));
    let idx = w.agents.iter().position(|a| a.id == id).unwrap();
    w.agents[idx].needs.hunger = 1.0;
    w.agents[idx].needs.rest = 1.0;
    let tree = TilePos::new(36, 30, 2);
    // variant 0: species oak, and (0 / 3) % 3 == 0 so the crown falls beside the stump
    w.plants.push(Plant::new(tree, PlantKind::Tree, 1.0, 0));
    let did = w.alloc_designation_id();
    w.designations.push(Designation {
        id: did,
        kind: DesignationKind::Chop,
        pos: tree,
        requirements: vec![],
        work_left: 25 * 20,
        work_total: 25 * 20,
        max_workers: 1,
        priority: 9,
        created: w.tick(),
        failures: 0,
        done: false,
        community: None,
        owner: None,
    });
    for _ in 0..(TICKS_PER_HOUR * 4) {
        w.step();
        if !w
            .plants
            .iter()
            .any(|p| p.pos == tree && p.kind == PlantKind::Tree)
        {
            break;
        }
    }
    assert!(
        w.plants
            .iter()
            .any(|p| p.pos == tree && p.kind == PlantKind::Stump && p.alive),
        "a stump stands where the tree was"
    );
    assert!(
        w.plants
            .iter()
            .any(|p| p.kind == PlantKind::FallenLog && p.pos.dist(tree) == 1),
        "the crown lies beside it"
    );
}

/// A worker sent off thirsty, with nothing else around to have noticed, comes back to the
/// order once watered (#433): the knowledge-settled-in gate used to hold the whole chop
/// order hostage to whether the worker happened to have sighted anything else.
#[test]
fn a_worker_sent_to_drink_comes_back_to_the_order() {
    let mut w = World::generate(35, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.meta.autoplan = false;
    w.rebuild_caches();
    let id = w.spawn_agent(Species::Person, TilePos::new(30, 30, 2));
    let idx = w.agents.iter().position(|a| a.id == id).unwrap();
    w.agents[idx].needs.hunger = 1.0;
    w.agents[idx].needs.rest = 1.0;
    // Thirsty from the start, and nothing else nearby to have learned of: the worst case for
    // the knowledge gate.
    w.agents[idx].needs.thirst = 0.2;
    let tree = TilePos::new(36, 30, 2);
    w.plants.push(Plant::new(tree, PlantKind::Tree, 1.0, 0));
    let did = w.alloc_designation_id();
    w.designations.push(Designation {
        id: did,
        kind: DesignationKind::Chop,
        pos: tree,
        requirements: vec![],
        work_left: 25 * 20,
        work_total: 25 * 20,
        max_workers: 1,
        priority: 9,
        created: w.tick(),
        failures: 0,
        done: false,
        community: None,
        owner: None,
    });
    for _ in 0..(TICKS_PER_HOUR * 6) {
        w.step();
        if !w
            .plants
            .iter()
            .any(|p| p.pos == tree && p.kind == PlantKind::Tree)
        {
            break;
        }
    }
    assert!(
        w.plants
            .iter()
            .any(|p| p.pos == tree && p.kind == PlantKind::Stump && p.alive),
        "a stump stands where the tree was, even sent off thirsty first"
    );
    let d = w.designations.iter().find(|d| d.id == did);
    assert!(
        d.is_none_or(|d| d.done || d.failures <= 12),
        "the order was not given up for having been left for a drink"
    );
}

/// The client maps enum indices to names; every name must have a sprite, and every material
/// family a terrain sheet.
#[test]
fn client_draws_every_kind_and_family() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../godot");
    let gd = std::fs::read_to_string(root.join("scripts/world_view.gd")).unwrap();
    let array = |name: &str| -> Vec<String> {
        let start = gd.find(&format!("const {name} := [")).unwrap();
        let body = &gd[start..];
        let body = &body[body.find('[').unwrap() + 1..body.find(']').unwrap()];
        body.split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };
    let kinds = array("PLANT_KIND");
    assert_eq!(
        kinds.len(),
        PlantKind::ALL.len(),
        "PLANT_KIND matches PlantKind"
    );
    let assets = root.join("assets");
    for name in &kinds {
        let files: Vec<String> = match name.as_str() {
            "tree" => vec![
                "oak_2.png".into(),
                "birch_2.png".into(),
                "pine_2.png".into(),
            ],
            "berry_bush" => vec!["berry_bush_fruit.png".into(), "berry_bush_bare.png".into()],
            // Field crops by four stages (#286: barley, oats, rye, peas, flax).
            "grain" | "root" | "barley" | "oats" | "rye" | "peas" | "flax" => {
                (0..4).map(|s| format!("{name}_{s}.png")).collect()
            }
            // The valley's other trees (#286): three stages, and a winter sheet each.
            "alder" | "ash" | "yew" | "willow" | "crab_apple" => (0..3)
                .flat_map(|s| [format!("{name}_{s}.png"), format!("{name}_{s}_winter.png")])
                .collect(),
            "reeds" => vec![
                "reeds.png".into(),
                "reed_0.png".into(),
                "reed_1.png".into(),
                "reed_2.png".into(),
            ],
            "herbs" | "fern" | "vine" | "root_crops" | "flowering_plant" | "hazel" | "bilberry"
            | "wild_strawberry" | "bramble" | "wild_garlic" | "chanterelle" | "cattail"
            | "death_cap" | "hemlock" | "nightshade" | "lily_of_the_valley" | "nettle" | "woad"
            | "madder" | "weld" | "yarrow" | "comfrey" | "thistle" => {
                (0..3).map(|s| format!("{name}_{s}.png")).collect()
            }
            "fallen_log" => vec!["log.png".into()],
            other => vec![format!("{other}.png")],
        };
        for f in files {
            assert!(
                assets.join("plants").join(&f).exists(),
                "{name}: plants/{f} missing"
            );
        }
    }
    for season in ["autumn", "winter"] {
        for f in [
            format!("shrub_{season}.png"),
            format!("berry_bush_bare_{season}.png"),
            format!("berry_bush_fruit_{season}.png"),
        ] {
            assert!(
                assets.join("plants").join(&f).exists(),
                "plants/{f} missing"
            );
        }
    }
    let animals = array("ANIMAL_NAME");
    assert_eq!(animals[Species::Goat as usize], "goat");
    assert!(assets.join("animals/goat.png").exists());
    let table = MaterialTable::default();
    let fstart = gd.find("const FAMILY := {").unwrap();
    let family_block = &gd[fstart..fstart + gd[fstart..].find('}').unwrap()];
    for d in &table.defs {
        // Metals, woods, fibres, hides and gems (C32) are what items are made of; they never
        // lie as tiles, so the client has no ground family for them.
        use hf_sim::world::material::MaterialKind as MK;
        if matches!(
            d.kind,
            MK::Metal | MK::Wood | MK::Fibre | MK::Hide | MK::Gem | MK::Glass | MK::Food
        ) {
            continue;
        }
        let entry = format!(" {}: \"", d.id.0);
        let at = family_block
            .find(&entry)
            .or_else(|| family_block.find(&format!("\t{}: \"", d.id.0)))
            .unwrap_or_else(|| panic!("FAMILY has no {}", d.id.0));
        let fam = &family_block[at + entry.len()..];
        let fam = &fam[..fam.find('"').unwrap()];
        let expect = match d.family.as_str() {
            "wood" => "wood_floor",
            "block" => "stone_floor",
            f => f,
        };
        assert_eq!(fam, expect, "FAMILY[{}]", d.id.0);
        if d.kind != hf_sim::world::material::MaterialKind::Air {
            assert!(
                assets.join(format!("terrain/{fam}.png")).exists(),
                "terrain/{fam}.png"
            );
        }
    }
    assert!(assets.join("terrain/deep_water.png").exists());
}

/// Every ground family the generator can lay must take part in edge blending: a place in the
/// client's BLEND_ORDER, the same order in tools/art/terrain.py (which cuts the pieces), and an
/// edge sheet. A family missing from any of them meets its neighbours with square tile edges.
#[test]
fn every_surface_family_blends() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let list = |src: &str, start: &str| -> Vec<String> {
        let at = src
            .find(start)
            .unwrap_or_else(|| panic!("{start} not found"));
        let body = &src[at..];
        let body = &body[body.find('[').unwrap() + 1..body.find(']').unwrap()];
        body.split(',')
            .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };
    let gd = std::fs::read_to_string(root.join("godot/scripts/world_view.gd")).unwrap();
    let py = std::fs::read_to_string(root.join("tools/art/terrain.py")).unwrap();
    let client = list(&gd, "const BLEND_ORDER := [");
    let art = list(&py, "\nBLEND_ORDER = [");
    assert_eq!(client[0], "water", "water is lowest in the client order");
    assert_eq!(
        &client[1..],
        &art[..],
        "world_view.gd BLEND_ORDER (after water) and terrain.py BLEND_ORDER differ"
    );
    let mut families: Vec<String> = ground_sources().into_iter().map(|(f, _)| f).collect();
    // Plus what the client shows on generated ground in some season or weather.
    families.extend(["frozen_soil", "wet_soil", "ash", "snow"].map(String::from));
    for fam in families {
        assert!(
            client.contains(&fam),
            "{fam} is generated but has no place in BLEND_ORDER (square edges)"
        );
        assert!(
            root.join(format!("godot/assets/terrain/edges/{fam}.png"))
                .exists(),
            "{fam} is generated but has no edge sheet"
        );
    }
}

/// Biome shares over land columns, Biome::ALL order: the whole valley, and within `r` tiles
/// (Euclidean) of a point.
fn biome_mix(w: &World, around: Option<(TilePos, i32)>) -> [f32; Biome::ALL.len()] {
    let mut n = [0usize; Biome::ALL.len()];
    for y in 0..w.grid.height {
        for x in 0..w.grid.width {
            if let Some((p, r)) = around {
                if (x - p.x).pow(2) + (y - p.y).pow(2) > r * r {
                    continue;
                }
            }
            if is_water(w, x, y) {
                continue;
            }
            n[w.grid.biome[w.grid.col(x, y)] as usize] += 1;
        }
    }
    let total = n.iter().sum::<usize>().max(1) as f32;
    n.map(|k| k as f32 / total)
}

/// How much of the valley's mix the founding site's surroundings share: the sum over biomes of
/// the smaller of the two shares (1 when the 15-tile circle is the valley in miniature).
fn founding_resemblance(w: &World) -> f32 {
    let valley = biome_mix(w, None);
    let local = biome_mix(w, Some((w.meta.founding_site, 15)));
    valley.iter().zip(local).map(|(a, b)| a.min(b)).sum()
}

/// The founding site shows the valley's character and is still a place to live. Measured over
/// seeds 1 to 24 with `founding_resemblance` (the share of the valley's biome mix that the
/// 15-tile circle around the site has in common with it): master at a7196a2 averaged 0.652
/// (12 of 24 sites in woodland, seed 24 as low as 0.341); with the typical-ground terms in
/// `flora::founding_site` the mean is 0.771 (3 in woodland, lowest 0.608). Survivability on
/// this branch: water within 6 tiles of every site, at least 106 of the 169 columns within 6
/// level with it, at least 26 food plants and 14 trees within 15 tiles (master: 106, 21, 31).
#[test]
fn founding_site_shows_the_valley() {
    let mut total = 0.0;
    for seed in SEEDS {
        let w = World::generate(seed, GenParams::default());
        let fs = w.meta.founding_site;
        let r = founding_resemblance(&w);
        total += r;
        let within = |p: &Plant| (p.pos.x - fs.x).pow(2) + (p.pos.y - fs.y).pow(2) <= 225;
        let food = w
            .plants
            .iter()
            .filter(|p| p.kind.yields_food() && within(p))
            .count();
        let trees = w
            .plants
            .iter()
            .filter(|p| p.kind == PlantKind::Tree && within(p))
            .count();
        let g = w.grid.ground_z(fs.x, fs.y);
        let mut water = i32::MAX;
        let mut flat = 0;
        for dy in -15..=15i32 {
            for dx in -15..=15i32 {
                let (x, y) = (fs.x + dx, fs.y + dy);
                if x < 0 || y < 0 || x >= w.grid.width || y >= w.grid.height {
                    continue;
                }
                if is_water(&w, x, y) {
                    water = water.min(dx.abs().max(dy.abs()));
                } else if dx.abs() <= 6 && dy.abs() <= 6 && w.grid.ground_z(x, y) == g {
                    flat += 1;
                }
            }
        }
        let biome = w.grid.biome_at(fs.x, fs.y);
        let what = format!(
            "seed {seed}: site {:?} in {:?}, resemblance {r:.3}, water {water}, flat {flat}, food {food}, trees {trees}",
            (fs.x, fs.y),
            biome.map(|b| b.label())
        );
        println!("{what}");
        assert!(
            !matches!(biome, Some(Biome::Fen | Biome::Fell | Biome::Snowfield)),
            "{what}: founded on excluded ground"
        );
        assert!(water <= 6, "{what}: no water within reach");
        assert!(flat >= 100, "{what}: too little level ground to build on");
        assert!(food >= 20, "{what}: too little food nearby");
        assert!(trees >= 10, "{what}: too little wood nearby");
    }
    let mean = total / SEEDS.count() as f32;
    println!("mean resemblance {mean:.3}");
    assert!(
        mean > 0.72,
        "founding sites resemble their valleys {mean:.3} on average (master 0.652)"
    );
}

/// #698: the pass fires on the ground real valleys have. The opening valleys step one level
/// at a time, so on each of seeds 1 to 4 the pass, given that valley's relief with every top
/// laid bare, must turn some knoll and ridge-edge tops loose (a few patches, not a rash),
/// never a top with no step on two sides, and the same way twice.
#[test]
fn scree_lies_on_the_knolls_of_generated_valleys() {
    use hf_sim::world::gen::scree;
    use hf_sim::world::{Material, Shape, Tile};
    for seed in 1..=4u64 {
        let w = World::opening_world(seed);
        let p = w.meta.params.clone();
        let mut grid = w.grid.clone();
        // Every top bare soil, so only the pass decides what lies loose.
        for y in 0..p.height {
            for x in 0..p.width {
                let top = TilePos::new(x, y, grid.ground_z(x, y));
                if grid.get(top).shape == Shape::Wall {
                    grid.set(top, Tile::solid(Material::SOIL));
                }
            }
        }
        let mut twin = grid.clone();
        scree::apply(seed, &p, &mut grid);
        scree::apply(seed, &p, &mut twin);
        let (mut eligible, mut loose) = (0usize, 0usize);
        for y in 1..p.height - 1 {
            for x in 1..p.width - 1 {
                let g = grid.ground_z(x, y);
                let m = grid.get(TilePos::new(x, y, g)).material;
                assert_eq!(
                    m,
                    twin.get(TilePos::new(x, y, g)).material,
                    "seed {seed}: not deterministic"
                );
                let sides = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .filter(|(dx, dy)| grid.ground_z(x + dx, y + dy) != g)
                    .count();
                let rise = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .map(|(dx, dy)| (grid.ground_z(x + dx, y + dy) - g).abs())
                    .max()
                    .unwrap_or(0);
                let is_loose = m == Material::SCREE || m == Material::GRAVEL;
                if sides >= 2 || rise >= 2 {
                    eligible += 1;
                    loose += usize::from(is_loose);
                } else {
                    assert!(
                        !is_loose,
                        "seed {seed}: loose rock at ({x},{y}), which steps on {sides} side(s)"
                    );
                }
            }
        }
        assert!(eligible > 0, "seed {seed}: no knolls at all");
        let share = loose as f32 / eligible as f32;
        assert!(
            loose > 0,
            "seed {seed}: the pass turned nothing loose on {eligible} knoll tops"
        );
        assert!(
            share < 0.35,
            "seed {seed}: {loose} of {eligible} knoll tops loose, a rash not patches"
        );
    }
}
