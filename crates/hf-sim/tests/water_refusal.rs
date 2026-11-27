//! #453 (epic #477, part 4): knowledge is personal — a settler refuses salt and shuns foul
//! water they KNOW of, unless desperate; water never perceived is assumed fresh, and they
//! can be wrong (part 3's flux roll, ground truth at drink time, is the punishment). The
//! knowledge lives in the `tag` of a `Known::Water` belief: 0 fresh (the serde default, so
//! old saves read as fresh), 1 foul, 2 salt, 3 brackish (floods.rs `TAG_WATER_*`). Foul is
//! perceived by smell within `smell_radius` even without a sight line, or by sight of the
//! fouled tile; salt and brackish by sight of the water.

use hf_sim::floods::{self, WaterQuality};
use hf_sim::ids::TilePos;
use hf_sim::items::{Item, ItemKind, ItemPlace};
use hf_sim::mind::knowledge::Known;
use hf_sim::mind::Activity;
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

/// The middle of the fresh pond, pond B.
const FRESH_MIDDLE: TilePos = TilePos { x: 42, y: 42, z: 2 };

/// Flat ground, a pond at 20..24 × 20..24 (pond A) and a farther fresh pond at 40..44 ×
/// 40..44 (pond B). With `foul`, rotten meat east of pond A and one hour of the fouling
/// system: the shore beside the meat is foul. Returns the world and the fouled shore tile.
fn two_ponds(seed: u64, foul: bool) -> (World, TilePos) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(55, 55, 2);
    for y in 20..=24 {
        for x in 20..=24 {
            w.grid.set(TilePos::new(x, y, 2), water_tile());
        }
    }
    for y in 40..=44 {
        for x in 40..=44 {
            w.grid.set(TilePos::new(x, y, 2), water_tile());
        }
    }
    w.rebuild_caches();
    floods::survey(&mut w);
    let fouled = TilePos::new(24, 22, 2);
    if foul {
        let meat_id = w.alloc_item_id();
        w.items.push(Item {
            id: meat_id,
            kind: ItemKind::RawMeat,
            count: 1,
            place: ItemPlace::Ground(TilePos::new(25, 21, 2)),
            quality: 0.5,
            material: Material::MEAT,
            story: None,
            rot: 0.9,
        });
        w.meta.tick = 9 * TICKS_PER_HOUR + 300;
        water_quality::step(&mut w);
        assert_eq!(
            w.waters.foul.get(&fouled),
            Some(&FOUL_HOURS),
            "the fixture fouls the pond A shore beside the meat"
        );
    }
    w.meta.autoplan = false;
    w.ensure_nav();
    (w, fouled)
}

/// The founder, standing where the test puts them.
fn founder_at(w: &mut World, at: TilePos) -> usize {
    let ids = w.found_party(1);
    let i = w
        .agents
        .iter()
        .position(|a| a.id == ids[0])
        .expect("the founder");
    w.agents[i].pos = at;
    i
}

/// A day of looks: 20 consecutive ticks of perceive so every stripe looks, whatever the
/// world's tick.
fn look(w: &mut World) {
    for t in 0..20 {
        w.meta.tick = 9 * TICKS_PER_HOUR + t;
        hf_sim::systems::perceive::step(w);
    }
}

/// Thirsty, otherwise comfortable, idle: nothing competes with the drink goal.
fn thirsty(w: &mut World, i: usize, thirst: f32) {
    let a = &mut w.agents[i];
    a.needs.thirst = thirst;
    a.needs.hunger = 1.0;
    a.needs.rest = 1.0;
    a.needs.warmth = 1.0;
    a.activity = Activity::Idle;
}

/// The drink goal the decider just chose, if any.
fn drink_target(w: &World, i: usize) -> Option<TilePos> {
    match &w.agents[i].activity {
        Activity::Going { to, then } if matches!(**then, Activity::Drinking { .. }) => Some(*to),
        _ => None,
    }
}

/// Whether some water is remembered foul (one foul shore noted is the pond remembered
/// foul — perceive notes a tile per body, not every fouled tile).
fn knows_foul(w: &World, i: usize) -> bool {
    w.agents[i]
        .knowledge
        .iter_kind(|k| matches!(k, Known::Water(_)))
        .any(|(_, b)| b.tag == 1 /* floods::TAG_WATER_FOUL */)
}

#[test]
fn knowing_a_pond_is_foul_they_walk_past_it_to_fresh_water() {
    let (mut w, fouled) = two_ponds(4551, true);
    let i = founder_at(&mut w, TilePos::new(26, 22, 2));
    // This alternative is beyond the agent's sight at pond A. See it before returning.
    let original = w.agents[i].pos;
    w.agents[i].pos = TilePos::new(45, 42, 2);
    look(&mut w);
    w.agents[i].pos = original;
    look(&mut w);
    // The fouled shore was noted, tagged foul — that is the knowledge the choice turns on.
    assert!(knows_foul(&w, i), "the foul pond remembered foul");
    // Thirsty (0.6 pressing) but not desperate (0.7 is the parched mark).
    thirsty(&mut w, i, 0.4);
    hf_sim::systems::decide::step(&mut w);
    let to = drink_target(&w, i).expect("a goal of drinking");
    assert!(
        to.dist(fouled) > 4,
        "the foul pond shunned: going to {to:?}, foul at {fouled:?}"
    );
    assert!(
        to.dist(FRESH_MIDDLE) <= 4,
        "the farther fresh pond chosen: {to:?}"
    );
}

#[test]
fn parched_past_desperation_they_drink_the_foul_pond_they_know() {
    let (mut w, fouled) = two_ponds(4552, true);
    let i = founder_at(&mut w, TilePos::new(26, 22, 2));
    look(&mut w);
    assert!(knows_foul(&w, i), "the foul pond remembered foul");
    // thirst_p = 0.75, past the 0.7 parched mark: desperate takes what they know is foul.
    thirsty(&mut w, i, 0.25);
    hf_sim::systems::decide::step(&mut w);
    let to = drink_target(&w, i).expect("a goal of drinking");
    assert!(
        to.dist(fouled) <= 4,
        "the near foul pond taken when desperate: {to:?}"
    );
    assert!(
        to.dist(FRESH_MIDDLE) > 10,
        "not the walk to the far fresh pond: {to:?}"
    );
}

#[test]
fn one_who_never_perceived_the_foul_pond_walks_to_it_and_drinks() {
    let (mut w, fouled) = two_ponds(4553, true);
    // Never a look: no knowledge, so the foul pond is assumed fresh and, being near, taken.
    let i = founder_at(&mut w, TilePos::new(30, 22, 2));
    assert!(
        w.agents[i]
            .knowledge
            .iter_kind(|k| matches!(k, Known::Water(_)))
            .next()
            .is_none(),
        "they know of no water"
    );
    thirsty(&mut w, i, 0.4);
    hf_sim::systems::decide::step(&mut w);
    let to = drink_target(&w, i).expect("a goal of drinking");
    assert!(
        to.dist(fouled) <= 4,
        "the near foul pond taken by one who knows nothing of it: {to:?}"
    );
    // They were wrong, and they drink: walk them onto the bank the decider chose and let
    // the drinking run (part 3 decides whether the flux follows).
    w.agents[i].pos = to;
    w.agents[i].path.clear();
    for _ in 0..400 {
        if w.agents[i].needs.thirst == 1.0 {
            break;
        }
        hf_sim::systems::act::step(&mut w);
    }
    assert_eq!(
        w.agents[i].needs.thirst, 1.0,
        "they drank from the foul pond"
    );
}

#[test]
fn a_nose_knows_foul_water_without_a_sight_line() {
    let (mut w, _fouled) = two_ponds(4555, true);
    // A wall between the smeller and the fouled shore: the eye is shut, the nose is not.
    for y in 18..=26 {
        w.grid.set(
            TilePos::new(26, y, 2),
            Tile {
                material: Material::LIMESTONE,
                shape: Shape::Wall,
                flags: TileFlags(0),
            },
        );
    }
    w.rebuild_caches();
    let kind = hf_sim::species::by_key("murrow").expect("the murrow people");
    let id = w.spawn_agent(hf_sim::agents::Species::Person, TilePos::new(28, 22, 2));
    hf_sim::species::give(&mut w, id, kind, None);
    let i = w.agents.iter().position(|a| a.id == id).unwrap();
    assert_eq!(w.agents[i].species_def().smell_radius(), 6);
    look(&mut w);
    // Nothing of the water was seen through the wall ...
    assert!(
        w.agents[i]
            .knowledge
            .iter_kind(|k| matches!(k, Known::Water(_)))
            .all(
                |(_, b)| b.tag != 2 /* floods::TAG_WATER_SALT */
                && b.tag != 3 /* floods::TAG_WATER_BRACKISH */
            ),
        "no sight of it"
    );
    // ... but the foul shore was smelled, and noted foul.
    assert!(knows_foul(&w, i), "foul smelled without a sight line");
}

#[test]
fn a_known_salt_pond_is_never_chosen_even_when_nearer() {
    let (mut w, _fouled) = two_ponds(4554, false);
    // Mark pond A salt, as the survey marks the sea on a coast (part 1's model); the test
    // exercises the refusal, not the survey.
    let a_middle = TilePos::new(22, 22, 2);
    let b = w
        .waters
        .bodies
        .iter_mut()
        .find(|b| b.tiles.contains(&a_middle))
        .expect("pond A surveyed");
    b.quality = WaterQuality::Salt;
    let i = founder_at(&mut w, TilePos::new(26, 22, 2));
    // This alternative is beyond the agent's sight at pond A. See it before returning.
    let original = w.agents[i].pos;
    w.agents[i].pos = TilePos::new(45, 42, 2);
    look(&mut w);
    w.agents[i].pos = original;
    look(&mut w);
    // What the eye took of it, tagged salt.
    assert!(
        w.agents[i]
            .knowledge
            .iter_kind(|k| matches!(k, Known::Water(_)))
            .any(|(_, b)| b.tag == 2 /* floods::TAG_WATER_SALT */),
        "the salt pond noted by sight"
    );
    // Even parched past desperation, salt is never worth the walk.
    thirsty(&mut w, i, 0.25);
    hf_sim::systems::decide::step(&mut w);
    let to = drink_target(&w, i).expect("a goal of drinking");
    assert!(to.dist(a_middle) > 4, "the salt pond refused: {to:?}");
    assert!(
        to.dist(FRESH_MIDDLE) <= 4,
        "the farther fresh pond chosen: {to:?}"
    );
}

#[test]
fn a_thirsty_person_can_search_then_recall_an_obscured_pond() {
    let (mut w, _) = two_ponds(4563, false);
    let from = TilePos::new(30, 22, 2);
    let i = founder_at(&mut w, from);
    w.meta.tick = 9 * TICKS_PER_HOUR;
    for y in 15..=29 {
        w.grid
            .set(TilePos::new(26, y, 2), Tile::solid(Material::GRANITE));
    }
    w.rebuild_caches();
    w.ensure_nav();
    let source = TilePos::new(24, 22, 2);
    assert!(!hf_sim::mind::perception::line_of_sight(
        &w.grid, from, source
    ));
    thirsty(&mut w, i, 0.15);
    hf_sim::systems::decide::step(&mut w);
    assert!(
        matches!(&w.agents[i].activity,
        Activity::Going {to, then} if *to != from && matches!(**then, Activity::Loitering { .. })
            && hf_sim::mind::perception::line_of_sight(&w.grid, from, *to)),
        "a parched person without a source should set off to search: {:?}",
        w.agents[i].activity
    );
    let now = w.meta.tick;
    w.agents[i].knowledge.note(
        Known::Water(source),
        source,
        now,
        hf_sim::floods::TAG_WATER_FRESH,
    );
    thirsty(&mut w, i, 0.15);
    hf_sim::systems::decide::step(&mut w);
    let bank = drink_target(&w, i).expect("a remembered source merits a directed journey");
    assert!(hf_sim::water::reaches(bank, source));
    assert!(w.nav.as_ref().unwrap().reach(from, bank));
}
