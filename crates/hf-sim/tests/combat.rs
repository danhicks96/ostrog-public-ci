//! C75: combat and militias: nerve, standing together, raiders who break, the beast from the
//! breach, and the Beast-hunger's shape.

use hf_sim::agents::Species;
use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::mind::beliefs::{Source, Topic};
use hf_sim::systems::combat;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;
use rand::SeedableRng;

fn meadow(seed: u64, n: usize) -> (World, Vec<usize>) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 12;
    w.rebuild_caches();
    let ids = w.found_party(n);
    w.meta.autoplan = false;
    w.ensure_nav();
    let idx = ids
        .iter()
        .map(|id| w.agents.iter().position(|a| a.id == *id).unwrap())
        .collect();
    (w, idx)
}

/// Lean someone toward settling things with a fist (+1) or away from it (-1).
fn lean(w: &mut World, i: usize, toward: f32) {
    let me = w.agents[i].id;
    for _ in 0..20 {
        w.agents[i]
            .beliefs
            .experience(Topic::Violence, toward, 1.0, Source::Person(me), 0);
    }
}

fn pending(w: &World, f: impl Fn(&EventKind) -> bool) -> usize {
    w.events.pending.iter().filter(|e| f(&e.kind)).count()
}

#[test]
fn nerve_comes_from_company_and_goes_with_pain() {
    let (mut w, p) = meadow(7501, 4);
    let (a, others) = (p[0], &p[1..]);
    let home = TilePos::new(30, 30, 2);
    w.agents[a].pos = w.grid.surface(home.x, home.y);
    for &k in others {
        let at = w.agents[a].pos.offset(2, 0, 0);
        w.agents[k].pos = w.grid.surface(at.x, at.y);
    }
    let together = combat::nerve(&w, a);
    for &k in others {
        let far = w.grid.surface(55, 55);
        w.agents[k].pos = far;
    }
    let alone = combat::nerve(&w, a);
    assert!(together > alone, "company steadies: {together} vs {alone}");
    hf_sim::systems::health::wound(
        &mut w,
        a,
        hf_sim::body::Part::TORSO,
        hf_sim::body::WoundKind::Cut,
        0.6,
        0.0,
        "a test",
        None,
    );
    let hurt = combat::nerve(&w, a);
    assert!(hurt < alone, "pain takes it: {hurt} vs {alone}");
    assert!(hurt < 0.25, "alone and badly hurt, they run: {hurt}");
}

#[test]
fn neighbours_stand_together_and_fell_a_beast() {
    let (mut w, p) = meadow(7502, 4);
    let at = w.grid.surface(30, 30);
    for (n, &k) in p.iter().enumerate() {
        w.agents[k].pos = w.grid.surface(30 + n as i32, 31);
        w.agents[k].activity = hf_sim::mind::Activity::Idle;
        lean(&mut w, k, 1.0);
    }
    let id = w.spawn_agent(Species::Wolf, at);
    let b = w.agents.iter().position(|a| a.id == id).unwrap();
    w.agents[b].name = "Greyjaw".into();
    let mut r = rand_chacha::ChaCha8Rng::seed_from_u64(1);
    let mut felled = false;
    for _ in 0..12 {
        let (stood, f) = combat::stand_against(&mut w, b, p[0], &mut r);
        assert!(!stood.is_empty(), "the bold stand");
        if f {
            felled = true;
            break;
        }
    }
    assert!(felled, "a beast hurt enough is felled");
    assert!(!w.agents[b].alive);
}

#[test]
fn the_fearful_run_and_do_not_stand() {
    let (mut w, p) = meadow(7503, 3);
    // Out in the fields, well away from home.
    let at = w.grid.surface(45, 45);
    for (n, &k) in p.iter().enumerate() {
        w.agents[k].pos = w.grid.surface(46 + n as i32, 45);
        w.agents[k].activity = hf_sim::mind::Activity::Idle;
        lean(&mut w, k, -1.0);
        hf_sim::systems::health::wound(
            &mut w,
            k,
            hf_sim::body::Part::ARM_L,
            hf_sim::body::WoundKind::Bruise,
            0.3,
            0.0,
            "a test",
            None,
        );
    }
    let id = w.spawn_agent(Species::Bear, at);
    let b = w.agents.iter().position(|a| a.id == id).unwrap();
    let mut r = rand_chacha::ChaCha8Rng::seed_from_u64(2);
    let (stood, felled) = combat::stand_against(&mut w, b, p[0], &mut r);
    assert!(stood.is_empty(), "nobody with the stomach for it");
    assert!(!felled);
    assert!(
        matches!(
            w.agents[p[1]].activity,
            hf_sim::mind::Activity::Wandering { .. }
        ),
        "a frightened neighbour runs for home"
    );
}

/// Hour by hour: the way walled or not, and the stir below (#455, stirring.rs).
fn hours(w: &mut World, n: u64) {
    w.meta.tick -= w.meta.tick % TICKS_PER_HOUR;
    for _ in 0..n {
        w.meta.tick += TICKS_PER_HOUR;
        hf_sim::underground::rise(w);
        hf_sim::stirring::step(w);
    }
}

#[test]
fn the_beast_of_the_deep_comes_up_through_the_breach() {
    let mut w = World::from_setup(&hf_sim::setup::WorldSetup::new(52));
    let region = w.meta.setup.as_ref().and_then(|s| s.region).unwrap();
    let b = w
        .realm
        .beasts
        .iter()
        .position(|b| b.layer == "deep_caverns")
        .unwrap();
    w.realm.beasts[b].region = region;
    let name = w.realm.beasts[b].name.clone();
    let c = w.grid.caverns.iter().find(|c| c.deep).unwrap().clone();
    let i = w
        .agents
        .iter()
        .position(|a| a.alive && a.community.is_some())
        .unwrap();
    w.agents[i].pos = c.centre;
    hf_sim::underground::breaches(&mut w);
    assert!(w.creatures.risen.is_empty(), "#273: not at once");
    // #455: it comes up when the noise of the one standing in the cavern has stirred it, not on
    // a day fixed at the breach.
    hours(&mut w, 48);
    assert_eq!(w.creatures.risen.len(), 1, "it came");
    let id = w.creatures.risen[0];
    let beast = w.agents.iter().find(|a| a.id == id).unwrap();
    assert_eq!(beast.name, name, "by the name the realm knows it");
    assert!(c.contains(beast.pos));
    assert_eq!(pending(&w, |k| matches!(k, EventKind::Rose { .. })), 1);
}

#[test]
fn under_the_full_moon_the_cursed_take_a_beasts_shape_and_hunt() {
    let (mut w, p) = meadow(7505, 3);
    let (cursed, prey) = (p[0], p[1]);
    hf_sim::conditions::take(
        &mut w,
        cursed,
        hf_sim::conditions::ConditionKind::BeastHunger,
        "for a test",
        None,
    );
    for c in w.agents[cursed].conditions.iter_mut() {
        c.stage = 1;
    }
    let full = (0..TICKS_PER_DAY * 20)
        .map(|h| h * TICKS_PER_HOUR)
        .find(|&t| {
            hf_sim::time::Calendar::full_moon(t) && (t % TICKS_PER_DAY) / TICKS_PER_HOUR == 1
        })
        .unwrap();
    w.meta.tick = full + 7;
    hf_sim::conditions::step(&mut w);
    let who = w.agents[cursed].id;
    assert!(
        w.creatures.shifted.iter().any(|x| x.0 == who),
        "in the beast's shape"
    );
    assert_eq!(pending(&w, |k| matches!(k, EventKind::Shifted { .. })), 1);
    // Their neighbour is out alone beside them, and the hunger finds them.
    let at = w.agents[cursed].pos;
    w.agents[prey].pos = at.offset(1, 0, 0);
    w.agents[p[2]].pos = w.grid.surface(55, 55);
    let before = w.agents[prey].body.wounds.len();
    hf_sim::systems::combat::shifted(&mut w);
    assert!(
        w.agents[prey].body.wounds.len() > before,
        "torn in the night"
    );
    assert!(
        !w.creatures.shifted.iter().any(|x| x.0 == who),
        "one attack a night"
    );
}

#[test]
fn a_band_that_has_lost_half_its_number_runs_for_home() {
    let mut w = World::opening_world(3);
    w.meta.autoplan = true;
    w.meta.engine = hf_sim::engine::Engine::Relentless;
    for _ in 0..4 * TICKS_PER_DAY {
        w.step();
    }
    let s = w.overworld.sites.iter_mut().find(|s| s.people > 0).unwrap();
    s.food = -(s.people as f32) * 4.0;
    s.hungry_days = 3;
    let mut routed = 0;
    for _ in 0..8 * TICKS_PER_DAY {
        w.step();
        // Once one is out, count half of it hurt and gone.
        for b in w.bands.iter_mut() {
            if b.lost == 0 && b.size >= 2 {
                b.lost = b.size.div_ceil(2);
            }
        }
        routed += w
            .last_events
            .iter()
            .filter(|e| matches!(e.kind, EventKind::Routed { .. }))
            .count();
        if routed > 0 {
            break;
        }
    }
    assert_eq!(routed, 1, "the band broke and is on record once");
    assert!(w
        .bands
        .iter()
        .all(|b| b.lost == 0 || matches!(b.state, hf_sim::threat::BandState::Leaving)));
}

#[test]
fn one_bitten_alone_still_fights_back() {
    let (mut w, p) = meadow(7506, 2);
    let a = p[0];
    w.agents[a].pos = w.grid.surface(45, 45);
    w.agents[a].activity = hf_sim::mind::Activity::Idle;
    w.agents[p[1]].pos = w.grid.surface(10, 10);
    lean(&mut w, a, 0.5);
    let id = w.spawn_agent(Species::Wolf, w.grid.surface(46, 45));
    let b = w.agents.iter().position(|x| x.id == id).unwrap();
    // The bite lands first.
    hf_sim::systems::health::wound(
        &mut w,
        a,
        hf_sim::body::Part::LEG_L,
        hf_sim::body::WoundKind::Cut,
        0.5,
        0.15,
        "a test",
        None,
    );
    assert!(w.agents[a].body.needs_care());
    let mut r = rand_chacha::ChaCha8Rng::seed_from_u64(6);
    let (stood, _) = combat::stand_against(&mut w, b, a, &mut r);
    assert_eq!(stood.len(), 1, "they turn on it, hurt as they are");
}

#[test]
fn a_wolf_going_for_someone_alone_meets_them_standing() {
    let mut stood = 0;
    for seed in 0..10u64 {
        let (mut w, p) = meadow(7600 + seed, 2);
        let a = p[0];
        w.agents[a].pos = w.grid.surface(45, 45);
        w.agents[a].activity = hf_sim::mind::Activity::Idle;
        w.agents[p[1]].pos = w.grid.surface(10, 10);
        let id = w.spawn_agent(Species::Wolf, w.grid.surface(46, 45));
        let b = w.agents.iter().position(|x| x.id == id).unwrap();
        let mut r = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
        hf_sim::systems::animals::strike(&mut w, b, a, &mut r);
        stood += pending(&w, |k| matches!(k, EventKind::StoodAgainst { .. }));
    }
    // Judged before the teeth: an ordinary person, unhurt, turns on it.
    assert!(stood >= 8, "stood {stood} of 10");
}

#[test]
fn a_breach_walled_in_time_bars_the_beast_and_the_cavern_is_described() {
    let mut w = World::from_setup(&hf_sim::setup::WorldSetup::new(52));
    let region = w.meta.setup.as_ref().and_then(|s| s.region).unwrap();
    let b = w
        .realm
        .beasts
        .iter()
        .position(|b| b.layer == "deep_caverns")
        .unwrap();
    w.realm.beasts[b].region = region;
    let k = w.grid.caverns.iter().position(|c| c.deep).unwrap();
    let c = w.grid.caverns[k].clone();
    let i = w
        .agents
        .iter()
        .position(|a| a.alive && a.community.is_some())
        .unwrap();
    w.agents[i].pos = c.centre;
    w.agents[i].from = c.centre.offset(c.rx + 2, 0, 0);
    hf_sim::underground::breaches(&mut w);
    assert_eq!(
        pending(&w, |k| matches!(k, EventKind::CavernFound { .. })),
        1,
        "what was found is told"
    );
    let entry = w.grid.caverns[k].breach.expect("the way in is known");
    assert!(
        w.designations.iter().any(
            |d| d.pos == entry && matches!(d.kind, hf_sim::work::DesignationKind::BuildWall(_))
        ),
        "the valley sets about walling it"
    );
    // Walled before the beast comes.
    w.grid.set(
        entry,
        hf_sim::world::Tile::solid(hf_sim::world::Material::STONE_BLOCK),
    );
    hours(&mut w, 48);
    assert!(w.grid.caverns[k].sealed);
    assert!(w.creatures.risen.is_empty(), "barred");
    assert_eq!(pending(&w, |k| matches!(k, EventKind::Barred { .. })), 1);
}

/// #402 (M5 review 2026-10-26: one person took 592 blows from a host and lived through them):
/// blow after blow lands deeper on what is already hurt, so someone beaten again and again
/// bleeds inside and, untended, dies of it.
#[test]
fn someone_beaten_again_and_again_is_in_mortal_danger() {
    let (mut w, p) = meadow(7511, 2);
    let (a, b) = (p[0], p[1]);
    let mut r = rand::rngs::StdRng::seed_from_u64(402);
    for _ in 0..40 {
        combat::blow(&mut w, a, b, &mut r, "a test");
    }
    let body = &w.agents[b].body;
    assert!(
        body.wounds
            .iter()
            .any(|wd| wd.tissue == hf_sim::body::Tissue::Organ && wd.bleeding > 0.0),
        "a wound reached deep inside and bleeds: {}",
        body.summary()
    );
    // Untended, a few days of it.
    let start = w.meta.tick - w.meta.tick % TICKS_PER_HOUR;
    let id = w.agents[b].id;
    for h in 1..=96 {
        w.meta.tick = start + h * TICKS_PER_HOUR + 600;
        hf_sim::systems::health::step(&mut w);
        if !w.agents.iter().any(|x| x.id == id && x.alive) {
            break;
        }
    }
    let alive = w.agents.iter().any(|x| x.id == id && x.alive);
    assert!(!alive, "bled out within four days untended");
}

/// #405: `shifted` never fires in an opening because openings are Aun's valleys, and only Hask
/// curses a killer with the Beast-hunger. Under Hask a killing (here a wolf felled, as the
/// valley's fights record it: "killed by" the one who struck) brings the curse the next
/// morning, and from the second full moon after it the killer takes the beast's shape; under
/// Aun the same killing brings neither.
#[test]
fn under_hask_a_killing_brings_the_beasts_shape_two_full_moons_on() {
    for (engine, curses) in [
        (hf_sim::engine::Engine::Relentless, true),
        (hf_sim::engine::Engine::Steady, false),
    ] {
        let (mut w, p) = meadow(7506, 3);
        w.meta.engine = engine;
        let killer = p[0];
        let by = w.agents[killer].id;
        let near = w.agents[killer].pos.offset(2, 0, 0);
        let wolf = w.spawn_agent(Species::Wolf, near);
        let dead = w.agents.iter().position(|a| a.id == wolf).unwrap();
        hf_sim::systems::health::wound(
            &mut w,
            dead,
            hf_sim::body::Part::TORSO,
            hf_sim::body::WoundKind::Cut,
            0.9,
            0.5,
            "fighting off a beast",
            Some(by),
        );
        let (dead_id, name) = (w.agents[dead].id, w.agents[killer].name.clone());
        hf_sim::systems::life::die(&mut w, dead_id, format!("killed by {name}"));
        w.last_events = w.events.pending.clone();
        hf_sim::deity::step(&mut w);
        // The deity's morning.
        w.meta.tick = 2 * TICKS_PER_DAY + 4 * TICKS_PER_HOUR + 11;
        hf_sim::deity::step(&mut w);
        let cursed = w.agents[killer]
            .conditions
            .iter()
            .any(|c| c.kind == hf_sim::conditions::ConditionKind::BeastHunger);
        assert_eq!(cursed, curses, "{engine:?}");
        // One in the morning of each of the next thirty days: two full moons pass.
        let mut shifted = 0;
        for d in 3..33 {
            w.meta.tick = d * TICKS_PER_DAY + TICKS_PER_HOUR + 7;
            w.events.pending.clear();
            hf_sim::conditions::step(&mut w);
            shifted += pending(
                &w,
                |k| matches!(k, EventKind::Shifted { agent } if *agent == by),
            );
        }
        assert_eq!(shifted > 0, curses, "{engine:?}: {shifted}");
    }
}
