//! Food goes off (C31): in the open fastest, under a roof slower, in a barrel slowest, in frost
//! slowly; what rots leaves the ledger as rotted; fields are sown by what the store holds.

use hf_sim::economy::live_counts;
use hf_sim::ids::TilePos;
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::structures::{Structure, StructureKind};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

fn flat(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.items.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    w
}

fn count_at(w: &World, p: TilePos) -> u32 {
    w.items
        .iter()
        .filter(|it| it.place == ItemPlace::Ground(p))
        .map(|it| it.count)
        .sum()
}

fn hours(w: &mut World, n: u64, temp: f32) {
    for _ in 0..n {
        let next = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR + 37;
        w.meta.tick = next;
        w.weather.temperature_c = temp;
        w.weather.sky = hf_sim::systems::weather::Sky::Clear;
        hf_sim::systems::spoil::step(w);
    }
}

fn drift(w: &World) -> Vec<String> {
    let live = live_counts(&w.items);
    w.ledger
        .expected()
        .iter()
        .filter(|(k, v)| live.get(*k).copied().unwrap_or(0) != **v)
        .map(|(k, v)| format!("{k}: {v}"))
        .collect()
}

#[test]
fn food_in_the_open_rots_faster_than_under_a_roof_or_in_a_barrel() {
    let mut w = flat(7);
    w.build_house(40, 40, 7, 6, 1, Material::WOOD, Material::WOOD);
    let open = TilePos::new(10, 10, 2);
    let roofed = TilePos::new(43, 42, 2);
    let barrel = TilePos::new(20, 10, 2);
    let id = w.alloc_structure_id();
    w.structures.push(Structure {
        id,
        kind: StructureKind::Barrel,
        pos: barrel,
        material: Material::WOOD,
        progress: 1.0,
        active: false,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
    w.rebuild_caches();
    assert!(
        hf_sim::building::roofed(&w, roofed),
        "the test house has its roof"
    );
    for p in [open, roofed, barrel] {
        w.create_item(ItemKind::Berries, 40, ItemPlace::Ground(p), 0.6, "test");
    }
    hours(&mut w, 24 * 8, 15.0);
    let (o, r, b) = (
        count_at(&w, open),
        count_at(&w, roofed),
        count_at(&w, barrel),
    );
    assert!(o < 40, "berries in the open for eight days kept: {o}");
    assert!(r > o, "a roof keeps them longer: {r} against {o}");
    assert_eq!(b, 40, "a barrel keeps them past eight days");
    assert!(
        drift(&w).is_empty(),
        "the ledger knows what rotted: {:?}",
        drift(&w)
    );
}

#[test]
fn frost_keeps_food() {
    let mut w = flat(8);
    let (warm, cold) = (TilePos::new(10, 10, 2), TilePos::new(20, 20, 2));
    w.create_item(ItemKind::RawMeat, 20, ItemPlace::Ground(warm), 0.5, "test");
    let mut w2 = w.clone();
    hours(&mut w, 24 * 5, 15.0);
    w2.items.clear();
    w2.create_item(ItemKind::RawMeat, 20, ItemPlace::Ground(cold), 0.5, "test");
    hours(&mut w2, 24 * 5, -5.0);
    assert!(
        count_at(&w2, cold) > count_at(&w, warm),
        "frozen meat keeps"
    );
}

#[test]
fn what_does_not_spoil_does_not() {
    let mut w = flat(9);
    let p = TilePos::new(12, 12, 2);
    w.create_item(ItemKind::Log, 10, ItemPlace::Ground(p), 0.5, "test");
    w.create_item(
        ItemKind::IronOre,
        10,
        ItemPlace::Ground(p.offset(1, 0, 0)),
        0.5,
        "test",
    );
    hours(&mut w, 24 * 30, 15.0);
    assert_eq!(count_at(&w, p), 10);
    assert_eq!(count_at(&w, p.offset(1, 0, 0)), 10);
    let _ = TICKS_PER_DAY;
}

fn sown_plots(food: u32) -> usize {
    sown_plots_with(food, false)
}

/// As `sown_plots`, with the food in a second community's store when `neighbours` is true.
fn sown_plots_with(food: u32, neighbours: bool) -> usize {
    let mut w = flat(11);
    let ids = w.found_party(6);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    let plots = [
        home.offset(-20, 10, 0),
        home.offset(-12, 10, 0),
        home.offset(-4, 10, 0),
        home.offset(4, 10, 0),
    ];
    if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
        c.field_site = Some(plots[0]);
        c.extra_fields = plots[1..].to_vec();
    }
    let at = if neighbours {
        // Another community's hearth across the valley, with the grain beside it.
        let theirs = TilePos::new(58, 58, home.z);
        w.found_community(theirs);
        theirs.offset(-1, -1, 0)
    } else {
        home.offset(2, 2, 0)
    };
    if food > 0 {
        w.create_item(
            ItemKind::GrainSheaf,
            food,
            ItemPlace::Ground(at),
            0.6,
            "test",
        );
    }
    w.ensure_nav();
    for _ in 0..2 * TICKS_PER_HOUR {
        w.step();
    }
    // Plots being sown: an open sowing order on them, or grain already in the ground (C71:
    // six people sow a plot inside the two hours, so the orders alone miss it).
    let inside =
        |p: TilePos, o: &TilePos| p.x >= o.x && p.x < o.x + 6 && p.y >= o.y && p.y < o.y + 4;
    plots
        .iter()
        .filter(|o| {
            w.designations
                .iter()
                .any(|d| matches!(d.kind, hf_sim::work::DesignationKind::Sow) && inside(d.pos, o))
                || w.plants
                    .iter()
                    .any(|pl| pl.kind == hf_sim::plants::PlantKind::Grain && inside(pl.pos, o))
        })
        .count()
}

#[test]
fn fields_are_sown_by_what_the_store_holds() {
    let hungry = sown_plots(0);
    let full = sown_plots(6 * 60);
    assert!(hungry >= 2, "an empty store sows its plots: {hungry}");
    assert_eq!(full, 1, "a store with weeks of food sows one plot");
}

#[test]
fn a_neighbours_store_does_not_feed_us() {
    // The M4 review: after a split, a hungry community counted its neighbours' grain as its own
    // and sowed one plot through a famine (28 starved on seed 3).
    let hungry = sown_plots_with(6 * 60, true);
    assert!(
        hungry >= 2,
        "grain in another community's store sows our plots: {hungry}"
    );
}
