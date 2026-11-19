//! Materials (C32): a table with properties, kept current in old saves; every source gives its
//! material (the rock its blocks, the tree its logs, the deep its gems); stacks keep materials
//! apart; crafts carry them through; the valley holds marble, tin and silver; icons exist.

use hf_sim::ids::TilePos;
use hf_sim::items::ItemPlace;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::structures::StructureKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::{Designation, DesignationKind};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

fn meadow(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    for i in 0..200 {
        let p = TilePos::new(3 + (i * 7) % 58, 3 + (i * 13 + i / 58) % 58, 2);
        if !w.plants.iter().any(|q| q.pos == p) {
            w.plants
                .push(Plant::new(p, PlantKind::BerryBush, 1.0, i as u32));
        }
    }
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    w
}

fn open(p: TilePos, w: &mut World) {
    w.grid.set(
        p,
        Tile {
            material: Material::AIR,
            shape: Shape::Open,
            flags: TileFlags::NONE,
        },
    );
}

/// A shaft at (40,30) from z2 down to z0, a natural cave two tiles east at z0 (with
/// mushrooms, as worldgen makes them), one wall tile between, and an order to mine it. With
/// `cavern`, the cave is one of the named great chambers.
fn shaft_to_cave(cavern: bool) -> (World, hf_sim::ids::CommunityId) {
    let mut w = meadow(111);
    w.meta.autoplan = false;
    let ids = w.found_party(4);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    w.grid.set(
        TilePos::new(40, 30, 2),
        Tile {
            material: Material::STONE_BLOCK,
            shape: Shape::StairsDown,
            flags: TileFlags::NONE,
        },
    );
    w.grid.set(
        TilePos::new(40, 30, 1),
        Tile {
            material: Material::GRANITE,
            shape: Shape::StairsBoth,
            flags: TileFlags::NONE,
        },
    );
    w.grid.set(
        TilePos::new(40, 30, 0),
        Tile {
            material: Material::GRANITE,
            shape: Shape::StairsUp,
            flags: TileFlags::NONE,
        },
    );
    open(TilePos::new(41, 30, 0), &mut w);
    for x in 43..48 {
        for y in 28..33 {
            open(TilePos::new(x, y, 0), &mut w);
        }
    }
    w.plants.push(Plant::new(
        TilePos::new(44, 30, 0),
        PlantKind::Mushroom,
        1.0,
        1,
    ));
    // Order: mine the wall at (42,30,0).
    let id = w.alloc_designation_id();
    w.designations.push(Designation {
        id,
        kind: DesignationKind::Mine,
        pos: TilePos::new(42, 30, 0),
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
    if cavern {
        w.grid.caverns.push(hf_sim::world::gen::caverns::Cavern {
            name: "the Echoing Hall".into(),
            centre: TilePos::new(45, 30, 0),
            rx: 2,
            ry: 2,
            pool: true,
            deep: false,
            opened: false,
            underworld: false,
            breach: None,
            sealed: false,
            gate_of: None,
            keeper: None,
        });
    }
    (w, cid)
}

use hf_sim::items::{stack_name, ItemKind as K};
use hf_sim::world::material::{gem_at, hide_of, wood_of, MaterialKind};
use hf_sim::world::MaterialTable;

fn palette() -> Vec<String> {
    let text = include_str!("../../../godot/assets/palette/ostrog.json");
    text.split("\"hex\": \"")
        .skip(1)
        .map(|s| s[..7].to_string())
        .collect()
}

#[test]
fn every_material_has_properties_and_a_palette_colour() {
    let t = MaterialTable::default();
    let pal = palette();
    for (i, d) in t.defs.iter().enumerate() {
        assert_eq!(d.id.0 as usize, i, "{} sits at its id", d.name);
        if d.id == Material::AIR {
            continue;
        }
        assert!(d.density > 0.0, "{} has a density", d.name);
        assert!(
            pal.contains(&d.colour),
            "{} colour {} is on the palette",
            d.name,
            d.colour
        );
        if d.kind == MaterialKind::Metal {
            assert!(d.melts.is_some(), "{} melts", d.name);
        }
        if let Some(m) = d.becomes {
            assert_eq!(
                t.get(m).kind,
                MaterialKind::Metal,
                "{} becomes a metal",
                d.name
            );
        }
    }
    for ore in [
        Material::IRON_ORE,
        Material::COPPER_ORE,
        Material::TIN_ORE,
        Material::SILVER_ORE,
        Material::GOLD_ORE,
    ] {
        assert!(t.get(ore).becomes.is_some(), "{} smelts", t.get(ore).name);
    }
    assert!(t.get(Material::GOLD).value > t.get(Material::IRON).value);
    assert!(t.get(Material::BRONZE).hardness > t.get(Material::COPPER).hardness);
}

#[test]
fn an_old_save_gets_the_new_materials() {
    let mut w = meadow(3201);
    // A world saved before C32: 28 materials and no properties.
    w.grid.materials.defs.truncate(28);
    for d in w.grid.materials.defs.iter_mut() {
        d.density = 0.0;
        d.colour.clear();
    }
    let back = hf_sim::save::snapshot::Snapshot::capture(&w).restore();
    let t = &back.grid.materials;
    assert_eq!(t.defs.len(), MaterialTable::default().defs.len());
    assert_eq!(t.get(Material::GRANITE).density, 2700.0);
    assert_eq!(t.get(Material::GARNET).name, "garnet");
    // A material the code no longer names the same is left as it was saved.
    let mut odd = MaterialTable::default();
    odd.defs[5].name = "chalk".into();
    odd.defs[5].density = 1.0;
    odd.refresh();
    assert_eq!(odd.defs[5].name, "chalk");
    assert_eq!(odd.defs[5].density, 1.0);
}

#[test]
fn the_rock_gives_blocks_of_itself() {
    let (mut w, _) = shaft_to_cave(false);
    w.grid.set(
        TilePos::new(42, 30, 0),
        Tile {
            material: Material::MARBLE,
            shape: Shape::Wall,
            flags: TileFlags::NONE,
        },
    );
    for _ in 0..TICKS_PER_HOUR * 6 {
        w.step();
        if w.grid.walkable(TilePos::new(42, 30, 0)) {
            break;
        }
    }
    assert!(
        w.grid.walkable(TilePos::new(42, 30, 0)),
        "the marble was mined"
    );
    let blocks: Vec<_> = w
        .items
        .iter()
        // C67: mining yields rough stone of the rock's material; a bench dresses it to blocks.
        .filter(|it| it.kind == K::RoughStone)
        .map(|it| it.material)
        .collect();
    assert!(
        blocks.contains(&Material::MARBLE),
        "marble rough stone came out: {blocks:?}"
    );
}

#[test]
fn felled_trees_give_their_wood() {
    let mut w = meadow(3202);
    w.meta.autoplan = false;
    let ids = w.found_party(3);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    // Variant 1 is a birch (the client draws `variant % 3`: oak, birch, pine).
    let at = TilePos::new(34, 30, 2);
    w.items.retain(|it| it.kind != K::Log);
    w.plants.retain(|p| p.pos != at);
    w.plants.push(Plant::new(at, PlantKind::Tree, 1.0, 4));
    w.rebuild_caches();
    let mut d = hf_sim::systems::settlement::make(&mut w, DesignationKind::Chop, at, 9);
    d.community = Some(cid);
    w.designations.push(d);
    for _ in 0..TICKS_PER_HOUR * 8 {
        w.step();
        if w.items.iter().any(|it| it.kind == K::Log) {
            break;
        }
    }
    let logs: Vec<_> = w
        .items
        .iter()
        .filter(|it| it.kind == K::Log)
        .map(|it| it.material)
        .collect();
    assert_eq!(
        logs,
        vec![Material::BIRCH],
        "the felled birch gave birch logs"
    );
    assert_eq!(wood_of(PlantKind::Tree, 0), Material::OAK);
    assert_eq!(wood_of(PlantKind::Tree, 2), Material::PINE);
    assert_eq!(wood_of(PlantKind::FungusTree, 7), Material::FUNGUS_WOOD);
}

#[test]
fn stacks_of_different_materials_stay_apart() {
    let mut w = meadow(3203);
    w.items.clear();
    let p = TilePos::new(10, 10, 2);
    let a = w.create_item_of(K::Log, Material::OAK, 3, ItemPlace::Ground(p), 0.6, "test");
    let b = w.create_item_of(
        K::Log,
        Material::BIRCH,
        3,
        ItemPlace::Ground(p),
        0.6,
        "test",
    );
    let c = w.create_item_of(K::Log, Material::OAK, 2, ItemPlace::Ground(p), 0.6, "test");
    assert_ne!(a, b, "birch does not go into the oak stack");
    assert_eq!(a, c, "oak goes into the oak stack");
    assert_eq!(w.items.len(), 2);
    let note = hf_sim::items::stack_note(&w, p);
    // Nobody has been told of them yet, unless the viewer sees everything.
    if let Some(n) = note {
        assert!(
            n.contains("5 oak logs") && n.contains("3 birch logs"),
            "{n}"
        );
    }
}

#[test]
fn stacks_are_named_by_their_material() {
    let t = MaterialTable::default();
    assert_eq!(stack_name(&t, K::Log, Material::OAK), "oak logs");
    assert_eq!(
        stack_name(&t, K::StoneBlock, Material::GRANITE),
        "granite blocks"
    );
    assert_eq!(stack_name(&t, K::Gem, Material::GARNET), "garnets");
    assert_eq!(stack_name(&t, K::Ore, Material::TIN_ORE), "tin ore");
    assert_eq!(stack_name(&t, K::IronOre, Material::IRON_ORE), "iron ore");
    assert_eq!(stack_name(&t, K::Hide, Material::WOLF_PELT), "wolf pelts");
    assert_eq!(stack_name(&t, K::Meal, Material::AIR), "meals");
}

#[test]
fn planks_keep_their_wood_and_bars_their_metal() {
    let mut w = meadow(3204);
    w.found_party(10);
    // A carpenter and a smith among them (C70: the bench and the smelter are worked only by
    // those who know the crafts).
    {
        use hf_sim::mind::techniques::{Learned, Technique};
        let tick = w.meta.tick;
        for a in w
            .agents
            .iter_mut()
            .filter(|a| a.community.is_some())
            .take(3)
        {
            a.techniques
                .learn(Technique::Carpentry, Learned::Past, 0.5, tick);
            a.techniques
                .learn(Technique::Smelting, Learned::Past, 0.5, tick);
            a.techniques
                .learn(Technique::Smithing, Learned::Past, 0.5, tick);
        }
    }
    w.meta.autoplan = false;
    let cid = w.communities[0].id;
    for (kind, pos) in [
        (StructureKind::Smelter, TilePos::new(36, 28, 2)),
        (StructureKind::CarpenterBench, TilePos::new(24, 28, 2)),
    ] {
        let id = w.alloc_structure_id();
        w.structures.push(hf_sim::structures::Structure {
            id,
            kind,
            pos,
            material: Material::STONE_BLOCK,
            progress: 1.0,
            active: false,
            condition: 1.0,
            owner: None,
            quality: 0.5,
        });
    }
    w.items.clear();
    let g = |x, y| ItemPlace::Ground(TilePos::new(x, y, 2));
    w.create_item_of(K::IronOre, Material::IRON_ORE, 6, g(33, 30), 0.6, "test");
    w.create_item_of(K::Coal, Material::COAL, 3, g(33, 31), 0.6, "test");
    w.create_item_of(K::Log, Material::PINE, 4, g(27, 30), 0.6, "test");
    w.ensure_nav();
    for (r, at) in [
        (hf_sim::work::recipes::PLANK, TilePos::new(24, 28, 2)),
        (hf_sim::work::recipes::IRON_BAR, TilePos::new(36, 28, 2)),
    ] {
        let mut d = hf_sim::systems::settlement::make(&mut w, DesignationKind::Craft(r), at, 6);
        d.community = Some(cid);
        w.designations.push(d);
    }
    for _ in 0..TICKS_PER_DAY * 3 {
        w.step();
        if w.items.iter().any(|it| it.kind == K::Plank)
            && w.items.iter().any(|it| it.kind == K::IronBar)
        {
            break;
        }
    }
    let mat = |k| {
        w.items
            .iter()
            .filter(|it| it.kind == k)
            .map(|it| it.material)
            .collect::<Vec<_>>()
    };
    assert!(!mat(K::Plank).is_empty() && mat(K::Plank).iter().all(|m| *m == Material::PINE));
    assert!(!mat(K::IronBar).is_empty() && mat(K::IronBar).iter().all(|m| *m == Material::IRON));
}

#[test]
fn valleys_hold_marble_tin_and_silver() {
    let (mut marble, mut tin, mut silver, mut gold) = (0, 0, 0, 0);
    for seed in 1..=6u64 {
        let w = World::generate(seed, GenParams::default());
        let (mut m, mut t, mut s, mut g) = (0, 0, 0, 0);
        for z in w.grid.z_min..=w.grid.z_max {
            for y in 0..w.grid.height {
                for x in 0..w.grid.width {
                    match w.grid.get(TilePos::new(x, y, z)).material {
                        Material::MARBLE => m += 1,
                        Material::TIN_ORE => t += 1,
                        Material::SILVER_ORE => s += 1,
                        Material::GOLD_ORE => g += 1,
                        _ => {}
                    }
                }
            }
        }
        marble += (m > 0) as u32;
        tin += (t > 0) as u32;
        silver += (s > 0) as u32;
        gold += (g > 0) as u32;
    }
    assert!(marble >= 3, "marble in {marble} of 6 valleys");
    assert!(tin >= 5, "tin in {tin} of 6 valleys");
    assert!(silver >= 4, "silver in {silver} of 6 valleys");
    assert!(gold <= 5, "gold is rare: {gold} of 6 valleys");
}

#[test]
fn gems_and_hides_have_their_kinds() {
    let t = MaterialTable::default();
    let mut seen = std::collections::BTreeSet::new();
    for x in 0..40 {
        let g = gem_at(9, TilePos::new(x, 3, -10));
        assert_eq!(t.get(g).kind, MaterialKind::Gem);
        assert_eq!(
            g,
            gem_at(9, TilePos::new(x, 3, -10)),
            "the same pocket, the same gem"
        );
        seen.insert(g);
    }
    assert!(seen.len() >= 3, "several kinds of gem: {seen:?}");
    assert_eq!(
        hide_of(hf_sim::agents::Species::Wolf),
        Some(Material::WOLF_PELT)
    );
    assert_eq!(hide_of(hf_sim::agents::Species::Crow), None);
}

#[test]
fn every_material_variant_has_an_icon() {
    let t = MaterialTable::default();
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../godot/assets/items32/");
    let mut missing = Vec::new();
    for d in &t.defs {
        let kind = match d.kind {
            MaterialKind::Wood if d.id != Material::OAK => "log",
            MaterialKind::Stone if d.id != Material::GRANITE => "stone_block",
            MaterialKind::Hide if d.id != Material::DEER_HIDE => "hide",
            MaterialKind::Gem => "gem",
            MaterialKind::Ore if d.becomes.is_some() && t.get(d.becomes.unwrap()).value > 6.0 => {
                "ore"
            }
            _ => continue,
        };
        let f = format!("{dir}{kind}_{}.png", hf_sim::items::slug(&d.name));
        if !std::path::Path::new(&f).exists() {
            missing.push(f);
        }
    }
    assert!(missing.is_empty(), "icons missing: {missing:?}");
}
