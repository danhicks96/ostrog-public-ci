//! #453 (epic #477, part 2): rot and the dead foul the water; flow and time clear it. A
//! water tile beside food well on its way to rotten (rot >= 0.6, systems/spoil.rs) or beside a
//! body whose rite has not yet been done (systems/rites.rs `rites_due`) is foul; a running
//! stream cleans itself in hours, standing water in about a day. An old save whose Waters
//! has no `foul` field loads with none (serde default).

use hf_sim::floods::{self, WaterKind};
use hf_sim::ids::{EventId, TilePos};
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

/// Flat ground at level 1 (water stands at 2).
fn flat(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(40, 40, 2);
    w
}

/// Step the system one game hour at `hour` (it runs at tick 300 of each hour, the same tick
/// of the hour as the waters' own step, hazards.rs `flood_step`).
fn at_hour(w: &mut World, hour: u64) {
    w.meta.tick = hour * TICKS_PER_HOUR + 300;
    water_quality::step(w);
}

fn body_at(w: &World, p: TilePos) -> &floods::Body {
    w.waters
        .bodies
        .iter()
        .find(|b| b.tiles.contains(&p))
        .expect("a surveyed body there")
}

#[test]
fn a_lesson_outlasts_the_forgotten_tag() {
    use hf_sim::mind::{Known, ObservationKind, OutcomeKind};
    let run = |threads| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                let mut w = flat(4560);
                for (x0, x1) in [(20, 24), (44, 48)] {
                    for y in 20..=24 {
                        for x in x0..=x1 {
                            w.grid.set(TilePos::new(x, y, 2), water_tile());
                        }
                    }
                }
                w.rebuild_caches();
                floods::survey(&mut w);
                w.ensure_nav();
                w.weather.temperature_c = 10.0;
                let who = w.found_party(1)[0];
                let i = w.agents.iter().position(|a| a.id == who).unwrap();
                let from = TilePos::new(26, 22, 2);
                w.agents[i].pos = from;
                let a = TilePos::new(20, 20, 2);
                let b = TilePos::new(44, 20, 2);
                let choose = |w: &World, thirst: f32| {
                    hf_sim::water::nearest_knowing(
                        w,
                        i,
                        from,
                        60,
                        &w.agents[i].knowledge,
                        thirst > 0.7,
                    )
                    .expect("a reachable pond")
                    .source
                };
                w.agents[i].needs.thirst = 0.45;
                let control = choose(&w, 0.55);
                assert!(std::ptr::eq(body_at(&w, control), body_at(&w, a)));
                let tick = w.meta.tick;
                w.agents[i].learning.learn(
                    ObservationKind::FoulWater,
                    OutcomeKind::Illness,
                    Some(a),
                    true,
                    tick,
                    None,
                );
                w.agents[i].learning.lessons[0].confidence = 0.6;
                w.agents[i]
                    .knowledge
                    .note(Known::Water(a), a, tick, floods::TAG_WATER_FOUL);
                let water: Vec<_> = w.agents[i]
                    .knowledge
                    .map
                    .keys()
                    .copied()
                    .filter(|k| matches!(k, Known::Water(_)))
                    .collect();
                for known in water {
                    w.agents[i].knowledge.forget(known);
                }
                for thirst in [0.55, 0.8] {
                    w.agents[i].needs.thirst = 1.0 - thirst;
                    let before = hf_sim::digest::digest(&w);
                    let chosen = choose(&w, thirst);
                    assert_eq!(
                        before,
                        hf_sim::digest::digest(&w),
                        "water selection is pure"
                    );
                    assert!(
            std::ptr::eq(body_at(&w, chosen), body_at(&w, b)),
            "the illness lesson outlasts the forgotten tag at thirst {thirst}: {chosen:?}"
        );
                }
                w.agents[i].needs.thirst = 0.05;
                let emergency = choose(&w, 0.95);
                assert!(
                    std::ptr::eq(body_at(&w, emergency), body_at(&w, a)),
                    "severe thirst can still overcome the lesson"
                );
                w.agents[i].learning.lessons.clear();
                w.agents[i].needs.thirst = 0.2;
                let control = choose(&w, 0.8);
                assert!(
                    std::ptr::eq(body_at(&w, control), body_at(&w, a)),
                    "one with no lesson still assumes the nearby water fresh"
                );
                hf_sim::digest::digest(&w)
            })
    };
    assert_eq!(run(1), run(4));
}

#[test]
fn a_large_learned_bad_pond_does_not_hide_farther_fresh_water() {
    use hf_sim::mind::{ObservationKind, OutcomeKind};
    let mut w = flat(4561);
    for (x0, x1, y0, y1) in [(8, 29, 8, 29), (50, 54, 17, 21)] {
        for y in y0..=y1 {
            for x in x0..=x1 {
                w.grid.set(TilePos::new(x, y, 2), water_tile());
            }
        }
    }
    w.rebuild_caches();
    floods::survey(&mut w);
    w.ensure_nav();
    w.weather.temperature_c = 10.0;
    let who = w.found_party(1)[0];
    let i = w.agents.iter().position(|a| a.id == who).unwrap();
    let from = TilePos::new(30, 19, 2);
    w.agents[i].pos = from;
    w.agents[i].needs.thirst = 0.45;
    let bad = TilePos::new(29, 19, 2);
    let tick = w.meta.tick;
    w.agents[i].learning.learn(
        ObservationKind::FoulWater,
        OutcomeKind::Illness,
        Some(bad),
        true,
        tick,
        None,
    );
    w.agents[i].learning.lessons[0].confidence = 0.6;
    assert!(
        hf_sim::water::spots(&w, from, 60)
            .iter()
            .filter(|s| std::ptr::eq(body_at(&w, s.source), body_at(&w, bad)))
            .count()
            > 48
    );
    let chosen = hf_sim::water::nearest_knowing(&w, i, from, 60, &w.agents[i].knowledge, false)
        .expect("the large bad pond must not exhaust the budget before farther fresh water");
    assert!(!std::ptr::eq(body_at(&w, chosen.source), body_at(&w, bad)));
    assert!(w.nav.as_ref().unwrap().reach(from, chosen.stand));
}

#[test]
fn rot_and_a_body_beside_water_foul_it_and_no_further() {
    let mut w = flat(4533);
    for y in 20..=24 {
        for x in 20..=24 {
            w.grid.set(TilePos::new(x, y, 2), water_tile());
        }
    }
    w.rebuild_caches();
    floods::survey(&mut w);
    // Meat well on its way to rotten, one tile east of the pond's edge.
    let meat_p = TilePos::new(25, 22, 2);
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
    // A dead party member lying on the pond's north shore, awaiting a rite (C68: the dead
    // lie where they fell until someone comes to lay them to rest).
    let ids = w.found_party(1);
    w.meta.autoplan = false;
    let ai = w
        .agents
        .iter()
        .position(|a| a.id == ids[0])
        .expect("the founder");
    let corpse_p = TilePos::new(22, 19, 2);
    w.agents[ai].pos = corpse_p;
    w.agents[ai].alive = false;
    let community = w.agents[ai].community.expect("in a community");
    let dead = w.agents[ai].id;
    w.rites_due.push(hf_sim::systems::rites::RiteDue {
        dead,
        community,
        since: w.meta.tick,
        death: EventId(0),
        doer: None,
        away: false,
    });

    at_hour(&mut w, 9);
    // The edge tiles beside the meat and the corpse are foul, at a day's fouling at most.
    let beside_meat = TilePos::new(24, 22, 2);
    let beside_body = TilePos::new(22, 20, 2);
    assert_eq!(w.waters.foul.get(&beside_meat), Some(&FOUL_HOURS));
    assert_eq!(w.waters.foul.get(&beside_body), Some(&FOUL_HOURS));
    // Water two plane-tiles or more from any source stays fresh.
    let two_away = TilePos::new(20, 20, 2);
    assert!(
        !w.waters.foul.contains_key(&two_away),
        "a tile Chebyshev 2 from both sources stays fresh"
    );
    assert!(
        w.waters.foul.keys().all(|p| w.grid.get(*p).is_water()),
        "only water tiles foul"
    );
}

#[test]
fn a_stream_clears_foul_water_in_hours_a_pond_in_about_a_day() {
    let mut w = flat(4534);
    // A stream: two tiles wide, sixteen long. A pond: five across.
    for y in 30..=31 {
        for x in 8..=23 {
            w.grid.set(TilePos::new(x, y, 2), water_tile());
        }
    }
    for y in 40..=44 {
        for x in 20..=24 {
            w.grid.set(TilePos::new(x, y, 2), water_tile());
        }
    }
    w.rebuild_caches();
    floods::survey(&mut w);
    let stream_tile = TilePos::new(15, 30, 2);
    let pond_tile = TilePos::new(22, 42, 2);
    assert_eq!(body_at(&w, stream_tile).kind, WaterKind::Stream);
    assert_eq!(body_at(&w, pond_tile).kind, WaterKind::Pond);
    // Dry, mild weather: nothing feeds the water, the foul clears by flow and time alone.
    let mut wx = hf_sim::systems::weather::current(&w);
    wx.temperature_c = 10.0;
    wx.sky = hf_sim::systems::weather::Sky::Clear;
    w.force_weather = Some(wx);
    w.weather = wx;
    // Fouling both as badly as it gets: a day on every tile of each body.
    for b in &w.waters.bodies {
        for t in &b.tiles {
            w.waters.foul.insert(*t, FOUL_HOURS);
        }
    }
    let stream: Vec<TilePos> = body_at(&w, stream_tile).tiles.clone();
    let pond: Vec<TilePos> = body_at(&w, pond_tile).tiles.clone();
    let mut stream_clear = None;
    let mut pond_clear = None;
    for hour in 1..=48 {
        at_hour(&mut w, hour);
        if stream_clear.is_none() && stream.iter().all(|p| !w.waters.foul.contains_key(p)) {
            stream_clear = Some(hour);
        }
        if pond_clear.is_none() && pond.iter().all(|p| !w.waters.foul.contains_key(p)) {
            pond_clear = Some(hour);
        }
        if stream_clear.is_some() && pond_clear.is_some() {
            break;
        }
    }
    let (sc, pc) = (
        stream_clear.expect("the stream cleared within two days"),
        pond_clear.expect("the pond cleared within two days"),
    );
    assert!(
        sc < pc,
        "the stream cleared in {sc} hours, the pond in {pc}"
    );
    assert!(sc <= 12, "a stream clears in hours, not a day: {sc}");
    assert!(pc >= 20, "standing water persists about a day: {pc}");
}

#[test]
fn waters_without_the_foul_field_load_with_none() {
    // The shape a save written before part 2 has: no `foul` field on Waters.
    let old = r#"{"surveyed": false, "bodies": [], "flood_of": {}}"#;
    let wa: floods::Waters = serde_json::from_str(old).expect("an old-format Waters loads");
    assert!(wa.foul.is_empty());
}
