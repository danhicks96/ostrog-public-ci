//! #453 (epic #477, part 3): foul water sickens — a drink from a fouled tile can bring the
//! flux. The roll is in the actual drinking action (`water::drink`): the source tile's foul
//! map entry is checked first (a salt or brackish tile can be fouled too; salt's own effects
//! are part 4). Wells and barrel stores stay fresh by definition: they are no tile of open
//! water, so `drink` finds no `source` and never rolls.

use hf_sim::body::IllnessKind;
use hf_sim::floods;
use hf_sim::ids::TilePos;
use hf_sim::items::{Item, ItemKind, ItemPlace};
use hf_sim::systems::water_quality::{self, FOUL_HOURS};
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

fn water_tile() -> Tile {
    Tile {
        material: Material::WATER,
        shape: Shape::Floor,
        flags: TileFlags(TileFlags::WATER),
    }
}

/// The part 2 fouled-pond fixture: flat ground, a pond, meat well rotten one tile east of its
/// shore, and one hour of the fouling system. Returns the world and a fouled tile at the
/// pond's edge.
fn fouled_pond(seed: u64) -> (World, TilePos) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(40, 40, 2);
    for y in 20..=24 {
        for x in 20..=24 {
            w.grid.set(TilePos::new(x, y, 2), water_tile());
        }
    }
    w.rebuild_caches();
    floods::survey(&mut w);
    let meat_p = TilePos::new(25, 21, 2);
    let meat_id = w.alloc_item_id();
    w.items.push(Item {
        id: meat_id,
        kind: ItemKind::RawMeat,
        count: 1,
        place: ItemPlace::Ground(meat_p),
        quality: 0.5,
        material: Material::MEAT,
        story: None,
        rot: 0.9,
    });
    w.meta.tick = 9 * TICKS_PER_HOUR + 300;
    water_quality::step(&mut w);
    // The pond tile beside the meat, on the shore where a drinker stands.
    let fouled = TilePos::new(24, 22, 2);
    assert_eq!(
        w.waters.foul.get(&fouled),
        Some(&FOUL_HOURS),
        "the fixture fouls the pond tile beside the meat"
    );
    (w, fouled)
}

/// A founder standing on the shore tile whose first open-water neighbour east is `fouled`.
fn drinker_beside(w: &mut World, fouled: TilePos) -> usize {
    w.weather.temperature_c = 10.0;
    let ids = w.found_party(1);
    let i = w
        .agents
        .iter()
        .position(|a| a.id == ids[0])
        .expect("the founder");
    w.agents[i].pos = fouled.offset(1, 0, 0);
    i
}

#[test]
fn a_drink_from_fouled_water_brings_the_flux() {
    // The seed is pinned so the drink roll lands under the chance (0.2, Sickness lever
    // aside); with the part 3 change the drink brings the flux. If an unrelated rng change
    // shifts this particular stream, pick another seed that pins a catching roll.
    let (mut w, fouled) = fouled_pond(4543);
    let i = drinker_beside(&mut w, fouled);
    assert!(
        hf_sim::water::drink(&mut w, i),
        "they drank from the fouled pond"
    );
    assert_eq!(
        w.agents[i].body.illness.map(|x| x.kind),
        Some(IllnessKind::Flux),
        "a foul drink brings the flux"
    );
}

#[test]
fn a_drink_from_fresh_water_brings_nothing() {
    let (mut w, _fouled) = fouled_pond(4536);
    // Across the pond from the rot: the first open-water neighbour west of this shore is a
    // fresh tile (nothing foul reaches it).
    w.weather.temperature_c = 10.0;
    let ids = w.found_party(1);
    let i = w
        .agents
        .iter()
        .position(|a| a.id == ids[0])
        .expect("the founder");
    w.agents[i].pos = TilePos::new(19, 24, 2);
    let fresh_tile = TilePos::new(20, 24, 2);
    assert!(!w.waters.foul.contains_key(&fresh_tile));
    assert!(hf_sim::water::drink(&mut w, i), "they drank from the pond");
    assert_eq!(w.agents[i].needs.thirst, 1.0, "the drink served");
    assert!(
        w.agents[i].body.illness.is_none(),
        "a fresh drink brings nothing"
    );
}

#[test]
fn an_immune_drinker_stays_healthy() {
    let (mut w, fouled) = fouled_pond(4543);
    let i = drinker_beside(&mut w, fouled);
    w.agents[i].immune.push(IllnessKind::Flux);
    assert!(
        hf_sim::water::drink(&mut w, i),
        "they drank from the fouled pond"
    );
    assert!(
        w.agents[i].body.illness.is_none(),
        "one who has had the flux and is guarded against it stays healthy"
    );
}
