//! Phase 14 gates: ore becomes a tool through the smelter and the anvil; a good tool makes
//! work faster than a crude one; worn-out tools and clothes are replaced from the store.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::structures::{Structure, StructureKind};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::DesignationKind;
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

fn meadow(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    for i in 0..100 {
        w.plants.push(Plant::new(
            TilePos::new(4 + (i * 7) % 56, 4 + (i * 11) % 56, 2),
            PlantKind::BerryBush,
            1.0,
            i as u32,
        ));
    }
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    w
}

fn place(w: &mut World, kind: StructureKind, pos: TilePos) {
    let id = w.alloc_structure_id();
    w.structures.push(Structure {
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

#[test]
fn recipe_chain_ore_to_tool() {
    let mut w = meadow(1401);
    w.found_party(10);
    w.meta.autoplan = false;
    let cid = w.communities[0].id;
    // Someone knows each craft in the chain (C70: an order nobody knows may be tried and
    // spoiled; that it was not here rested on who happened to be idle, and #237's change to
    // stress moved it).
    for r in [
        hf_sim::work::recipes::PLANK,
        hf_sim::work::recipes::IRON_BAR,
        hf_sim::work::recipes::IRON_TOOL,
    ] {
        if let Some(t) = hf_sim::work::recipes::craft_of(r) {
            for a in w.agents.iter_mut().filter(|a| a.community == Some(cid)) {
                a.techniques
                    .learn(t, hf_sim::mind::techniques::Learned::Past, 0.6, 0);
            }
        }
    }
    place(&mut w, StructureKind::Smelter, TilePos::new(36, 28, 2));
    place(&mut w, StructureKind::Anvil, TilePos::new(36, 31, 2));
    place(
        &mut w,
        StructureKind::CarpenterBench,
        TilePos::new(24, 28, 2),
    );
    w.drop_item(ItemKind::IronOre, 6, TilePos::new(33, 30, 2));
    w.drop_item(ItemKind::Coal, 3, TilePos::new(33, 31, 2));
    w.drop_item(ItemKind::Log, 4, TilePos::new(27, 30, 2));
    w.ensure_nav();
    // Orders as the planner would post them.
    let mut d = hf_sim::systems::settlement::make(
        &mut w,
        DesignationKind::Craft(hf_sim::work::recipes::PLANK),
        TilePos::new(24, 28, 2),
        6,
    );
    d.community = Some(cid);
    w.designations.push(d);
    let mut d = hf_sim::systems::settlement::make(
        &mut w,
        DesignationKind::Craft(hf_sim::work::recipes::IRON_BAR),
        TilePos::new(36, 28, 2),
        6,
    );
    d.community = Some(cid);
    w.designations.push(d);
    let mut made = Vec::new();
    for _ in 0..TICKS_PER_DAY * 3 {
        w.step();
        for e in &w.last_events {
            if let EventKind::Crafted { kind, tier, .. } = &e.kind {
                made.push((*kind, *tier));
            }
        }
        if made.iter().any(|(k, _)| *k == ItemKind::IronBar)
            && made.iter().any(|(k, _)| *k == ItemKind::Plank)
            && !w.designations.iter().any(|d| {
                d.kind == DesignationKind::Craft(hf_sim::work::recipes::IRON_TOOL) && !d.done
            })
        {
            let mut d = hf_sim::systems::settlement::make(
                &mut w,
                DesignationKind::Craft(hf_sim::work::recipes::IRON_TOOL),
                TilePos::new(36, 31, 2),
                6,
            );
            d.community = Some(cid);
            w.designations.push(d);
        }
        if made.iter().any(|(k, _)| *k == ItemKind::Tool) {
            break;
        }
    }
    assert!(
        made.iter().any(|(k, _)| *k == ItemKind::Plank),
        "planks were sawn: {made:?}"
    );
    assert!(
        made.iter().any(|(k, _)| *k == ItemKind::IronBar),
        "iron was smelted: {made:?}"
    );
    assert!(
        made.iter().any(|(k, _)| *k == ItemKind::Tool),
        "a tool was forged: {made:?}"
    );
    let tool = w
        .items
        .iter()
        .find(|it| it.kind == ItemKind::Tool && it.material == Material::IRON);
    assert!(tool.is_some(), "the tool is iron and on the ground");
    assert!(matches!(tool.unwrap().place, ItemPlace::Ground(_)));
}

#[test]
fn quality_affects_use() {
    // Two identical miners, one with a masterly tool, one with a crude one.
    let mut w = meadow(1402);
    let ids = w.found_party(2);
    w.meta.autoplan = false;
    w.ensure_nav();
    for (k, id) in ids.iter().enumerate() {
        let i = w.agents.iter().position(|a| a.id == *id).unwrap();
        w.agents[i].tool_quality = if k == 0 { 1.0 } else { 0.1 };
        w.agents[i].tool_wear = 1.0;
        w.agents[i].skills.states[hf_sim::mind::Skill::Mining as usize].level = 5.0;
    }
    // A rock to quarry per miner, each far from the other.
    let rocks = [TilePos::new(10, 10, 2), TilePos::new(50, 50, 2)];
    for (k, p) in rocks.iter().enumerate() {
        w.grid.set(
            *p,
            hf_sim::world::Tile {
                material: Material::GRANITE,
                shape: hf_sim::world::Shape::Wall,
                flags: hf_sim::world::TileFlags::NONE,
            },
        );
        let i = w.agents.iter().position(|a| a.id == ids[k]).unwrap();
        w.agents[i].pos = p.offset(1, 0, 0);
        w.agents[i].from = w.agents[i].pos;
        let mut d = hf_sim::systems::settlement::make(&mut w, DesignationKind::Mine, *p, 9);
        d.community = w.agents[i].community;
        w.designations.push(d);
    }
    w.rebuild_caches();
    w.ensure_nav();
    let mut done_at = [0u64, 0];
    for _ in 0..TICKS_PER_HOUR * 6 {
        w.step();
        for e in &w.last_events {
            if let EventKind::Mined { agent, .. } = &e.kind {
                let k = ids.iter().position(|x| x == agent).unwrap();
                if done_at[k] == 0 {
                    done_at[k] = w.tick();
                }
            }
        }
        if done_at.iter().all(|t| *t > 0) {
            break;
        }
    }
    assert!(
        done_at[0] > 0 && done_at[1] > 0,
        "both quarried: {done_at:?}"
    );
    assert!(
        done_at[0] < done_at[1],
        "the masterly tool finished first: {done_at:?}"
    );
}

#[test]
fn worn_out_replaced() {
    let mut w = meadow(1403);
    // One person, so nobody else can take the job (and the better tool) first.
    let ids = w.found_party(1);
    w.meta.autoplan = false;
    let home = w.meta.founding_site;
    w.drop_item(ItemKind::Clothes, 2, home.offset(2, 0, 0));
    w.drop_item(ItemKind::Tool, 1, home.offset(3, 0, 0));
    if let Some(it) = w.items.iter_mut().find(|it| it.kind == ItemKind::Tool) {
        it.quality = 0.8;
    }
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    w.agents[i].clothes_wear = 0.2;
    w.agents[i].tool_quality = 0.3;
    w.agents[i].tool_wear = 0.0001;
    // A rock next to them to work on, so the tool breaks and is replaced.
    let p = home.offset(-2, 0, 0);
    w.grid.set(
        p,
        hf_sim::world::Tile {
            material: Material::GRANITE,
            shape: hf_sim::world::Shape::Wall,
            flags: hf_sim::world::TileFlags::NONE,
        },
    );
    w.rebuild_caches();
    w.ensure_nav();
    let mut d = hf_sim::systems::settlement::make(&mut w, DesignationKind::Mine, p, 9);
    d.community = w.agents[i].community;
    w.designations.push(d);
    let (mut broke, mut dressed) = (false, false);
    for _ in 0..TICKS_PER_DAY * 2 {
        w.step();
        for e in &w.last_events {
            match &e.kind {
                EventKind::ToolBroke { agent, .. } if *agent == ids[0] => broke = true,
                EventKind::NewClothes { agent, .. } if *agent == ids[0] => dressed = true,
                _ => {}
            }
        }
    }
    assert!(broke, "the worn tool broke");
    assert!(dressed, "new clothes were put on");
    let a = w.agents.iter().find(|a| a.id == ids[0]).unwrap();
    assert!(a.clothes_wear > 0.9);
    assert_eq!(
        w.items
            .iter()
            .filter(|it| it.kind == ItemKind::Clothes)
            .map(|it| it.count)
            .sum::<u32>(),
        1
    );
}
