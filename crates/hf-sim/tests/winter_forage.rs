//! #467: a settler weighs the cold they are in against the walk. Out foraging or stocking
//! in a hard cold, warmth drains wherever the temperature sits below their comfort, but
//! master's Warm goal only engaged under 2 C and only an asleep freezer re-decided: a
//! committed forager walked the long trip to the end and froze on it (seed 2's winters
//! killed 34 of 48).

use hf_sim::ids::TilePos;
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::mind::knowledge::Known;
use hf_sim::mind::Activity;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::structures::StructureKind;
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

/// A flat meadow, one clothed person at camp (20, 20) on day 30 mid-morning, a known lit
/// hearth four tiles off, a known berry bush 40 tiles away, the food stores cleared, and a
/// hard -10 C cold. The tick starts on the hour-boundary cadence `cold_step` uses
/// (`tick % TICKS_PER_HOUR == 300`, hazards.rs).
fn cold_camp(seed: u64) -> (World, usize, TilePos, TilePos) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(20, 20, 2);
    w.meta.tick = TICKS_PER_DAY * 30 + 10 * hf_sim::time::TICKS_PER_HOUR + 300;
    w.rebuild_caches();
    let ids = w.found_party(1);
    w.items.retain(|it| {
        !matches!(
            it.kind,
            ItemKind::Meal | ItemKind::Berries | ItemKind::Root | ItemKind::GrainSheaf
        )
    });
    w.ensure_nav();
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    w.agents[i].clothes_wear = 1.0;
    w.agents[i].needs.hunger = 0.4;
    w.agents[i].needs.rest = 0.9;
    w.agents[i].needs.social = 0.9;
    // A lit hearth they know of, as the founders would.
    let hearth = w.grid.surface(20, 24);
    w.place_structure(
        StructureKind::Hearth,
        hearth,
        hf_sim::world::Material::STONE_BLOCK,
        true,
    );
    let sid = w.structures.last().unwrap().id;
    let now = w.meta.tick;
    w.agents[i].knowledge.note(
        Known::Structure(sid),
        hearth,
        now,
        StructureKind::Hearth as u16,
    );
    // The far bush: 40 tiles of open ground from camp.
    let bush = TilePos::new(60, 20, 2);
    w.plants
        .push(Plant::new(bush, PlantKind::BerryBush, 1.0, 1));
    w.agents[i].knowledge.note(
        Known::FoodPlant(bush),
        bush,
        now,
        PlantKind::BerryBush as u16,
    );
    let mut wx = hf_sim::systems::weather::current(&w);
    wx.temperature_c = -10.0;
    w.weather = wx;
    (w, i, hearth, bush)
}

fn going_to(a: &hf_sim::agents::Agent) -> String {
    match &a.activity {
        Activity::Going { then, .. } => format!("going to {}", then.name()),
        other => other.name().to_string(),
    }
}

#[test]
fn a_forager_turns_back_from_a_long_trip_in_hard_cold() {
    // Already walking out to the far bush, the cold biting: a normal person turns for the
    // hearth before frostbite, not after. On master the walk was never re-thought awake, so
    // warmth reaches frostbite (0.1) on the way.
    let (mut w, i, hearth, bush) = cold_camp(4671);
    w.agents[i].needs.warmth = 0.5;
    // Ten tiles out, past the hearth's warmth, mid-way to the bush: the cold is draining
    // them here (on master it drains them all the way: frostbite at warmth 0.1).
    w.agents[i].pos = w.grid.surface(30, 20);
    w.agents[i].activity = Activity::Going {
        to: bush,
        then: Box::new(Activity::Foraging {
            plant: bush,
            ticks_left: 200,
        }),
    };
    w.agents[i].activity_ticks = 500;
    let mut turned = false;
    for _ in 0..TICKS_PER_DAY {
        w.meta.tick += 1;
        if w.meta.tick % hf_sim::time::TICKS_PER_HOUR == 300 {
            hf_sim::systems::hazards::cold_step(&mut w);
        }
        hf_sim::systems::decide::step(&mut w);
        assert!(
            w.agents[i].needs.warmth > 0.1,
            "froze on the trip: warmth {:.2}, {}",
            w.agents[i].needs.warmth,
            going_to(&w.agents[i])
        );
        if matches!(&w.agents[i].activity, Activity::Going { to, .. } if to.dist(hearth) <= 2) {
            turned = true;
            break;
        }
    }
    assert!(
        turned,
        "never turned back to the hearth: {}",
        going_to(&w.agents[i])
    );
}

#[test]
fn half_warmed_in_hard_cold_they_warm_instead_of_setting_out() {
    // Idle at camp, warmth already half gone in a -10 C cold: they warm at the hearth
    // rather than set out on a 40-tile food trip. On master the Warm gate sat under 2 C
    // (open here) but only under warmth 0.4 (shut here), so they walked out instead.
    let (mut w, i, hearth, _) = cold_camp(4672);
    w.agents[i].needs.warmth = 0.45;
    hf_sim::systems::decide::step(&mut w);
    assert!(
        matches!(&w.agents[i].activity, Activity::Going { to, .. } if to.dist(hearth) <= 2),
        "{}",
        going_to(&w.agents[i])
    );
}

#[test]
fn a_freezing_settler_finishes_the_meal_in_hand() {
    // Eating takes 300 ticks, longer than the freezing rethink's 200. At hunger 0.07 the
    // carried meal outranks Warm, so interrupting the meal itself restarts it forever.
    let (mut w, i, hearth, _) = cold_camp(4673);
    w.agents[i].needs.hunger = 0.07;
    w.agents[i].needs.warmth = 0.39;
    w.agents[i].needs.thirst = 0.9;
    w.agents[i].activity = Activity::Idle;
    w.agents[i].idle_ticks = 0;
    let agent = w.agents[i].id;
    let meal = w.create_item(ItemKind::Meal, 1, ItemPlace::Carried(agent), 0.5, "test");
    for _ in 0..301 {
        hf_sim::systems::decide::step(&mut w);
        hf_sim::systems::act::step(&mut w);
    }
    assert!(
        !w.items.iter().any(|it| it.id == meal),
        "meal was not consumed: {}",
        going_to(&w.agents[i])
    );
    assert!(w.agents[i].needs.hunger > 0.07);

    // Once the meal and its idle delay are over, the cold interruption still sends them
    // to the known lit hearth.
    for _ in 0..11 {
        hf_sim::systems::decide::step(&mut w);
        hf_sim::systems::act::step(&mut w);
    }
    assert!(
        matches!(&w.agents[i].activity, Activity::Going { to, .. } if to.dist(hearth) <= 2),
        "{}",
        going_to(&w.agents[i])
    );
}
