//! #393: the waters rise with the weather. Rain on a body's catchment raises it by what that
//! ground gathers for its size: a stream fed by the whole valley floor rises fast and goes over
//! its banks, a pond on a hilltop barely moves.

use hf_sim::floods::{self, Body};
use hf_sim::ids::TilePos;
use hf_sim::systems::weather::Sky;
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

/// The whole map flat at level 1 (people stand at 2); a stream two wide across it at y 30 and
/// 31; a hill of three levels at x and y 10 to 14 with a pond three across on its top.
fn valley(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(0, 0, 64, 64, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(40, 45, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    for y in 30..=31 {
        for x in 2..=61 {
            w.grid.set(TilePos::new(x, y, 2), water_tile());
        }
    }
    for y in 10..=14 {
        for x in 10..=14 {
            for z in 2..=4 {
                w.grid
                    .set(TilePos::new(x, y, z), Tile::solid(Material::SOIL));
            }
        }
    }
    for y in 11..=13 {
        for x in 11..=13 {
            w.grid.set(TilePos::new(x, y, 5), water_tile());
        }
    }
    w.rebuild_caches();
    floods::survey(&mut w);
    w
}

fn body_at(w: &World, p: TilePos) -> &Body {
    w.waters
        .bodies
        .iter()
        .find(|b| b.tiles.contains(&p))
        .expect("a surveyed body there")
}

fn weather(w: &mut World, sky: Sky, temp: f32) {
    let mut wx = w.weather;
    wx.sky = sky;
    wx.drizzle = false;
    wx.temperature_c = temp;
    w.force_weather = Some(wx);
}

fn hours(w: &mut World, n: u64) {
    for _ in 0..n * TICKS_PER_HOUR {
        w.step();
    }
}

fn flooded_from(w: &World, p: TilePos) -> usize {
    let i = w
        .waters
        .bodies
        .iter()
        .position(|b| b.tiles.contains(&p))
        .unwrap() as u16;
    w.waters.flood_of.values().filter(|b| **b == i).count()
}

#[test]
fn the_stream_gathers_more_than_the_hilltop_pond() {
    let w = valley(3931);
    let stream = body_at(&w, TilePos::new(30, 30, 2));
    let pond = body_at(&w, TilePos::new(12, 12, 5));
    assert_eq!(stream.kind, floods::WaterKind::Stream);
    assert_eq!(pond.kind, floods::WaterKind::Pond);
    assert!(
        stream.gathers() > 5.0 * pond.gathers(),
        "the valley floor drains to the stream: it gathers {:.1} times its size, the pond {:.1}",
        stream.gathers(),
        pond.gathers()
    );
}

#[test]
fn the_same_storm_raises_the_stream_more_than_the_pond() {
    let mut w = valley(3932);
    weather(&mut w, Sky::Storm, 20.0);
    hours(&mut w, 6);
    let s = TilePos::new(30, 30, 2);
    let p = TilePos::new(12, 12, 5);
    let (stream, pond) = (body_at(&w, s).stage, body_at(&w, p).stage);
    assert!(
        stream >= 1.0 && stream > 3.0 * pond,
        "six hours of storm: the stream stands {stream:.2} levels up, the pond {pond:.2}"
    );
    assert!(
        flooded_from(&w, s) > 20,
        "the stream is over its banks: {} tiles",
        flooded_from(&w, s)
    );
    assert!(
        flooded_from(&w, p) <= 4,
        "the pond barely moves: {} tiles",
        flooded_from(&w, p)
    );
    // A drizzle for the same hours raises the stream less than the storm did.
    let mut d = valley(3932);
    weather(&mut d, Sky::Rain, 20.0);
    let mut wx = d.force_weather.unwrap();
    wx.drizzle = true;
    d.force_weather = Some(wx);
    hours(&mut d, 6);
    let drizzled = body_at(&d, s).stage;
    assert!(
        drizzled < stream / 3.0,
        "a drizzle raised the stream {drizzled:.2}, the storm {stream:.2}"
    );
    // Dry weather after: the stream falls back and its flood drains.
    weather(&mut w, Sky::Clear, 20.0);
    hours(&mut w, 72);
    assert!(
        w.flooded.is_empty(),
        "three dry days later {} tiles are still under water",
        w.flooded.len()
    );
    assert!(w.grid.get(s).is_water(), "the stream itself is still there");
}

thread_local! {
    static SPRING: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Peak stage of the stream through `thaw` hours of spring after `snow` hours of snowfall.
fn spring_after(snow: u64, thaw: u64) -> (f32, usize, f32) {
    let mut w = valley(3933);
    let s = TilePos::new(30, 30, 2);
    weather(&mut w, Sky::Snow, -5.0);
    hours(&mut w, snow);
    let lying = w.waters.snow;
    assert!(
        body_at(&w, s).stage.abs() < 0.01,
        "snow lies; it does not run to the stream"
    );
    weather(&mut w, Sky::Clear, 10.0);
    let (mut peak, mut flood) = (0.0f32, 0usize);
    for _ in 0..thaw {
        for _ in 0..TICKS_PER_HOUR {
            w.step();
            for e in &w.last_events {
                if let hf_sim::events::EventKind::HighWater { spring, .. } = e.kind {
                    assert!(spring, "the thaw's flood is the spring flood");
                    SPRING.with(|c| c.set(c.get() + 1));
                }
            }
        }
        peak = peak.max(body_at(&w, s).stage);
        flood = flood.max(flooded_from(&w, s));
    }
    assert!(w.waters.snow <= 0.0, "the thaw took it all");
    (peak, flood, lying)
}

#[test]
fn a_heavy_winter_brings_a_spring_flood() {
    let (heavy, heavy_flood, lying) = spring_after(60, 48);
    assert!(
        lying >= 55.0,
        "sixty hours of snow lie on the ground: {lying}"
    );
    assert!(
        SPRING.with(|c| c.get()) >= 1,
        "the spring flood is on record"
    );
    let (light, light_flood, _) = spring_after(8, 48);
    assert!(
        heavy >= 1.5 && heavy_flood > 50,
        "after a heavy winter the thaw floods the stream: {heavy:.2} levels up, {heavy_flood} tiles"
    );
    assert!(
        light < heavy / 2.0 && light_flood < heavy_flood / 2,
        "a light winter's thaw: {light:.2} levels, {light_flood} tiles; a heavy one's {heavy:.2}"
    );
}

/// The valley with a floodplain south of the stream: x 20 to 40, y 33 to 38, a level lower
/// than the ground round it (people stand at 1 there), joined to the stream at y 32.
fn with_floodplain(seed: u64) -> World {
    let mut w = valley(seed);
    for y in 32..=38 {
        for x in 20..=40 {
            let mut t = Tile::AIR;
            t.flags = TileFlags::NONE;
            w.grid.set(TilePos::new(x, y, 1), t);
            w.grid
                .set(TilePos::new(x, y, 0), Tile::solid(Material::SOIL));
        }
    }
    w.rebuild_caches();
    floods::survey(&mut w);
    w
}

#[test]
fn a_spate_fills_the_floodplain_first_carries_off_what_lies_there_and_leaves_a_pond() {
    use hf_sim::events::EventKind;
    use hf_sim::items::ItemKind;
    let mut w = with_floodplain(3934);
    let s = TilePos::new(30, 30, 2);
    let plain = TilePos::new(30, 36, 1);
    let bank = TilePos::new(30, 28, 2);
    w.drop_item(ItemKind::Log, 3, plain);
    weather(&mut w, Sky::Storm, 20.0);
    let mut first: Option<Vec<TilePos>> = None;
    let mut carried = 0;
    let mut deep_plain = false;
    let mut high = 0;
    for _ in 0..14 {
        for _ in 0..TICKS_PER_HOUR {
            w.step();
            for e in &w.last_events {
                if let EventKind::CarriedOff { things, .. } = e.kind {
                    carried += things;
                }
                if let EventKind::HighWater { spring, .. } = e.kind {
                    assert!(!spring, "a storm's flood is not the spring flood");
                    high += 1;
                }
            }
        }
        if first.is_none() && !w.flooded.is_empty() {
            first = Some(w.flooded.iter().copied().collect());
        }
        deep_plain |= hf_sim::systems::hazards::under_water(&w, plain);
    }
    let first = first.expect("the storm put the stream over its banks");
    assert!(
        first.iter().all(|p| p.z == 1),
        "the low ground floods first: {first:?}"
    );
    assert!(
        body_at(&w, s).stage >= floods::SPATE,
        "fourteen hours of storm: the stream is in spate ({:.2})",
        body_at(&w, s).stage
    );
    assert!(
        w.grid.get(plain).is_water(),
        "the floodplain is under water"
    );
    assert!(
        deep_plain,
        "in spate the floodplain's water is over a person's head"
    );
    assert!(
        hf_sim::systems::hazards::under_water(&w, s),
        "the shallow stream cannot be forded in spate"
    );
    assert!(
        !w.items
            .iter()
            .any(|it| it.place == hf_sim::items::ItemPlace::Ground(plain)),
        "the logs on the floodplain were carried off"
    );
    assert_eq!(carried, 3, "on record");
    assert_eq!(high, 1, "the high water is on record once");
    assert!(w.grid.get(bank).is_water(), "and then the bank");
    // The storm passes: the bank drains, the hollow keeps a pond.
    weather(&mut w, Sky::Clear, 20.0);
    hours(&mut w, 72);
    assert!(
        !w.grid.get(bank).is_water(),
        "the bank level with the stream drained"
    );
    assert!(w.flooded.is_empty(), "{} still flooded", w.flooded.len());
    assert!(
        w.grid.get(plain).is_water() && w.waters.left.contains_key(&plain),
        "the floodplain, lower than the stream, keeps a pond"
    );
    assert!(
        !hf_sim::systems::hazards::under_water(&w, plain)
            && !hf_sim::systems::hazards::under_water(&w, s),
        "out of spate nothing is over a head"
    );
}

#[test]
fn a_creek_runs_dry_in_a_long_dry_spell_and_comes_back_after_rain() {
    use hf_sim::events::EventKind;
    let mut w = valley(3935);
    let s = TilePos::new(30, 30, 2);
    let mut said = Vec::new();
    let watch = |w: &mut World, n: u64, said: &mut Vec<&'static str>| {
        for _ in 0..n * TICKS_PER_HOUR {
            w.step();
            for e in &w.last_events {
                match e.kind {
                    EventKind::RanDry { .. } => said.push("dry"),
                    EventKind::RanAgain { .. } => said.push("again"),
                    _ => {}
                }
            }
        }
    };
    // Two days of summer heat: still running.
    weather(&mut w, Sky::Clear, 25.0);
    watch(&mut w, 48, &mut said);
    assert!(w.grid.get(s).is_water(), "two dry days do not dry a stream");
    // Five more: dry.
    watch(&mut w, 120, &mut said);
    assert!(
        !w.grid.get(s).is_water(),
        "a week without rain: the stream is dry ({:.2})",
        body_at(&w, s).stage
    );
    assert!(w.grid.walkable(s), "its bed can be walked");
    assert_eq!(said, ["dry"]);
    // A day of rain brings it back.
    weather(&mut w, Sky::Rain, 15.0);
    watch(&mut w, 12, &mut said);
    assert!(w.grid.get(s).is_water(), "the rain brought it back");
    assert_eq!(said, ["dry", "again"]);
}

#[test]
fn people_keep_homes_and_fields_off_ground_they_have_seen_flooded() {
    let mut w = valley(3936);
    let bank = TilePos::new(30, 28, 2);
    let far = TilePos::new(45, 50, 2);
    let none = std::collections::BTreeSet::new();
    assert!(hf_sim::grounds::open_ground(&w, bank, &none, true));
    weather(&mut w, Sky::Storm, 20.0);
    hours(&mut w, 8);
    assert!(w.grid.get(bank).is_water(), "the storm flooded the bank");
    weather(&mut w, Sky::Clear, 20.0);
    hours(&mut w, 72);
    assert!(!w.grid.get(bank).is_water(), "and it drained");
    assert!(
        !hf_sim::grounds::open_ground(&w, bank, &none, true),
        "no field or stockpile below the high-water line"
    );
    assert!(hf_sim::grounds::open_ground(&w, far, &none, true));
    let plan = hf_sim::houses::new_house(&w, bank, far, 3, 1).expect("a house somewhere");
    assert!(
        plan.floor.iter().all(|p| !w.waters.flooded_once(p.x, p.y)),
        "the new house stands above the high-water line"
    );
}

/// The stream in spate from one side of the map to the other, or with a way round its ends.
fn spate_across(w: &mut World, way_round: bool) {
    let xs = if way_round { 2..=61 } else { 0..=63 };
    for y in 30..=31 {
        for x in xs.clone() {
            let mut t = water_tile();
            t.flags = TileFlags(TileFlags::WATER | TileFlags::FULL);
            w.grid.set(TilePos::new(x, y, 2), t);
        }
    }
    w.rebuild_caches();
}

#[test]
fn nobody_sets_out_through_water_over_their_head() {
    // #418: on seed 2 people walked to bed and to meals through the stream in spate.
    let mut w = valley(3937);
    let ids = w.found_party(1);
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let (near, far) = (TilePos::new(30, 27, 2), TilePos::new(30, 34, 2));
    w.agents[i].pos = near;
    w.agents[i].from = near;
    spate_across(&mut w, true);
    assert!(
        w.path_agent_to(i, far),
        "there is a way round the stream's ends"
    );
    assert!(
        w.agents[i].path.iter().all(|p| !w.grid.over_head(*p)),
        "and it is taken rather than the spate"
    );
    spate_across(&mut w, false);
    assert!(
        !w.path_agent_to(i, far),
        "with no way round, nobody wades into water over their head"
    );
    // Someone already in it may go through it to get out.
    let mid = TilePos::new(30, 30, 2);
    w.agents[i].pos = mid;
    w.agents[i].from = mid;
    assert!(w.path_agent_to(i, far));
}

#[test]
fn people_on_the_floodplain_get_out_of_the_rising_waters_way() {
    use hf_sim::events::EventKind;
    let mut w = with_floodplain(3938);
    w.meta.autoplan = false;
    // A ramp up out of the hollow at its far corner: the way out is away from the water and
    // further than anyone caught under it could swim.
    w.grid.set(
        TilePos::new(40, 38, 1),
        Tile {
            material: Material::SOIL,
            shape: Shape::RampUp,
            flags: TileFlags::NONE,
        },
    );
    let ids = w.found_party(4);
    for (k, id) in ids.iter().enumerate() {
        let i = w.agents.iter().position(|a| a.id == *id).unwrap();
        let p = TilePos::new(21 + k as i32, 33, 1);
        w.agents[i].pos = p;
        w.agents[i].from = p;
        w.agents[i].path.clear();
        // At something that keeps them there all day, as a field is worked.
        w.agents[i].activity = hf_sim::mind::Activity::Loitering {
            ticks_left: 20 * TICKS_PER_HOUR as u32,
        };
    }
    w.rebuild_caches();
    weather(&mut w, Sky::Storm, 20.0);
    let mut fled = 0;
    let mut deep = false;
    for _ in 0..14 {
        for _ in 0..TICKS_PER_HOUR {
            w.step();
            for e in &w.last_events {
                if let EventKind::Chose { what, .. } = &e.kind {
                    fled += (what == "made for higher ground") as u32;
                }
            }
        }
        deep |= hf_sim::systems::hazards::under_water(&w, TilePos::new(30, 36, 1));
    }
    assert!(
        hf_sim::systems::hazards::under_water(&w, TilePos::new(21, 33, 1)),
        "where they stood went over a person's head"
    );
    assert!(deep);
    for id in &ids {
        let a = w.agents.iter().find(|a| a.id == *id).unwrap();
        assert!(a.alive, "{} {:?} at {}", a.name, a.death, a.pos);
        assert!(!hf_sim::systems::hazards::under_water(&w, a.pos));
    }
    assert!(fled >= 1, "the flight is on record");
}

#[test]
fn someone_caught_under_on_their_way_makes_for_the_nearest_dry_ground() {
    // #418: a man whose way to a meal went under kept walking on through the flood to it.
    let mut w = valley(3939);
    let ids = w.found_party(1);
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let mid = TilePos::new(30, 31, 2);
    w.agents[i].pos = mid;
    w.agents[i].from = mid;
    spate_across(&mut w, false);
    w.ensure_nav();
    // On their way across the rest of it, to a meal well beyond the far bank, with dry ground
    // a step behind them.
    let far = TilePos::new(30, 15, 2);
    assert!(w.path_agent_to(i, far));
    w.agents[i].activity = hf_sim::mind::Activity::Going {
        to: far,
        then: Box::new(hf_sim::mind::Activity::Idle),
    };
    for _ in 0..12 {
        w.step();
    }
    let to = match &w.agents[i].activity {
        hf_sim::mind::Activity::Going { to, .. } => *to,
        a => panic!("{a:?}"),
    };
    assert!(
        to.dist(mid) <= 2,
        "out by the nearest bank, not on to {far}: {to}"
    );
    assert!(w.agents[i].alive);
}

#[test]
fn the_tired_sleep_on_high_ground_when_the_flood_covers_their_beds() {
    // A tired person was offered every reachable bed, including those in the flood's
    // way, and `flee_step` kept turning them back, so a camp stayed up until it collapsed.
    use hf_sim::agents::Species;
    use hf_sim::events::EventKind;
    use hf_sim::mind::{Activity, Known};
    use hf_sim::structures::StructureKind;
    let mut w = valley(3940);
    w.meta.autoplan = false;
    w.meta.tick = TICKS_PER_HOUR * 20;
    // The stream in spate (from 18:00, so it has been up two hours), its banks 2 tiles from
    // the beds.
    let stream = TilePos::new(30, 30, 2);
    let mut ids = Vec::new();
    for k in 0..6 {
        let bed = TilePos::new(26 + 2 * k, 28, 2);
        w.place_structure(StructureKind::Bed, bed, Material::WOOD, false);
        let sid = w.structures.iter().find(|s| s.pos == bed).unwrap().id;
        let at = TilePos::new(26 + 2 * k, 24, 2);
        let id = w.spawn_agent(Species::Person, at);
        let i = w.agents.iter().position(|a| a.id == id).unwrap();
        let now = w.meta.tick;
        let a = &mut w.agents[i];
        a.activity = Activity::Idle;
        a.needs.rest = 0.2;
        a.needs.hunger = 1.0;
        a.needs.thirst = 1.0;
        a.needs.warmth = 1.0;
        a.needs.social = 1.0;
        a.knowledge
            .note(Known::Structure(sid), bed, now, StructureKind::Bed as u16);
        ids.push(id);
    }
    w.rebuild_caches();
    weather(&mut w, Sky::Clear, 20.0);
    let held = floods::SPATE + 0.3;
    let mut exhausted = 0;
    for _ in 0..3 * TICKS_PER_HOUR {
        let bi = w
            .waters
            .bodies
            .iter()
            .position(|b| b.tiles.contains(&stream))
            .unwrap();
        w.waters.bodies[bi].stage = held;
        w.step();
        for e in &w.last_events {
            exhausted += matches!(e.kind, EventKind::Exhausted { .. }) as u32;
        }
    }
    assert_eq!(exhausted, 0, "nobody collapsed");
    for id in &ids {
        let a = w.agents.iter().find(|a| a.id == *id).unwrap();
        assert!(a.activity.is_asleep(), "{} is {:?}", a.name, a.activity);
        assert!(
            !hf_sim::systems::hazards::in_floods_way(&w, a.pos),
            "{} sleeps in the flood's way at {}",
            a.name,
            a.pos
        );
    }
}
