//! #433: people need water. Thirst runs down faster than hunger, faster in heat and work; a
//! day and a half parched kills; people go to fresh water and drink; flood water, water in the
//! workings and ice do not serve, a well does, and in a hard frost a hearth melts snow.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::structures::StructureKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
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

/// Flat ground at level 1 (people stand at 2), a pond five across at x and y 20 to 24, and
/// `n` people at (30, 30).
fn by_a_pond(seed: u64, n: usize) -> (World, Vec<usize>) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 9;
    for y in 20..=24 {
        for x in 20..=24 {
            w.grid.set(TilePos::new(x, y, 2), water_tile());
        }
    }
    w.rebuild_caches();
    hf_sim::floods::survey(&mut w);
    let ids = w.found_party(n);
    w.meta.autoplan = false;
    w.ensure_nav();
    let idx = ids
        .iter()
        .map(|id| w.agents.iter().position(|a| a.id == *id).unwrap())
        .collect();
    (w, idx)
}

fn mild(w: &mut World, c: f32) {
    let mut wx = hf_sim::systems::weather::current(w);
    wx.temperature_c = c;
    wx.sky = hf_sim::systems::weather::Sky::Clear;
    w.force_weather = Some(wx);
    w.weather = wx;
}

#[test]
fn a_parched_person_is_miserable_and_strained() {
    // #879: thirst and warmth never reached mood, so someone dying of thirst read as content
    // (the probe's seed 6 day 101: six died of thirst with the colony's mood at 0.98-1.00).
    let (mut w, idx) = by_a_pond(1791, 2);
    // Keep the pair away from the pond and from food, so nothing else moves their needs much
    // over the short run below.
    for &i in &idx {
        w.agents[i].needs.hunger = 1.0;
        w.agents[i].needs.rest = 1.0;
        w.agents[i].needs.social = 1.0;
        w.agents[i].needs.comfort = 1.0;
        w.agents[i].needs.warmth = 1.0;
    }
    w.agents[idx[0]].needs.thirst = 1.0;
    w.agents[idx[1]].needs.thirst = 0.05;
    // Drive only needs and strain, bypassing decide/movement so nobody walks to the pond and
    // drinks before the comparison is made.
    for _ in 0..TICKS_PER_HOUR {
        w.meta.tick += 1;
        hf_sim::systems::needs::step(&mut w);
        hf_sim::systems::strain::step(&mut w);
        w.agents[idx[0]].needs.thirst = 1.0;
        w.agents[idx[1]].needs.thirst = 0.05;
    }
    let (hydrated, parched) = (w.agents[idx[0]].mood, w.agents[idx[1]].mood);
    assert!(
        parched < hydrated - 0.2,
        "parched reads far more miserable than hydrated: parched {parched}, hydrated {hydrated}"
    );
    assert!(
        w.agents[idx[1]]
            .thoughts
            .has(hf_sim::mind::ThoughtKind::Thirsty),
        "parched carries the thought"
    );
}

#[test]
fn thirst_runs_down_in_hours_and_faster_in_heat() {
    let mut n = hf_sim::mind::Needs::default();
    n.dry(4 * TICKS_PER_HOUR as u32, 1.0, false);
    assert!(n.thirst < 0.05, "four waking hours: {}", n.thirst);
    let mut s = hf_sim::mind::Needs::default();
    s.dry(8 * TICKS_PER_HOUR as u32, 1.0, true);
    assert!(s.thirst > 0.45, "a night's sleep costs half: {}", s.thirst);
    let mut h = hf_sim::mind::Needs::default();
    h.dry(TICKS_PER_HOUR as u32, 2.0, false);
    assert!(
        h.thirst < 0.55,
        "an hour in great heat costs half: {}",
        h.thirst
    );
}

#[test]
fn a_thirsty_person_goes_to_the_pond_and_drinks() {
    let (mut w, p) = by_a_pond(4331, 1);
    mild(&mut w, 15.0);
    w.agents[p[0]].needs.thirst = 0.25;
    w.agents[p[0]].needs.hunger = 1.0;
    w.agents[p[0]].activity = hf_sim::mind::Activity::Idle;
    let mut drank_at = None;
    for _ in 0..TICKS_PER_HOUR {
        w.step();
        if matches!(
            w.agents[p[0]].activity,
            hf_sim::mind::Activity::Drinking { .. }
        ) {
            drank_at = Some(w.agents[p[0]].pos);
        }
    }
    // Drank to the full, an hour's thirst ago at most.
    assert!(
        w.agents[p[0]].needs.thirst > 0.7,
        "{}",
        w.agents[p[0]].needs.thirst
    );
    let at = drank_at.expect("they drank");
    assert!(
        at.dist(TilePos::new(22, 22, 2)) <= 3,
        "at the pond's edge: {at:?}"
    );
}

#[test]
fn a_day_and_a_half_parched_kills() {
    let (mut w, p) = by_a_pond(4332, 1);
    let id = w.agents[p[0]].id;
    w.agents[p[0]].needs.thirst = 0.0;
    let start = w.meta.tick - w.meta.tick % 20;
    let mut parched = false;
    for k in 1..=(2 * TICKS_PER_DAY / 20) {
        w.meta.tick = start + k * 20;
        w.agents[p[0]].needs.thirst = 0.0;
        hf_sim::systems::life::step(&mut w);
        if !w.agents.iter().any(|a| a.id == id && a.alive) {
            parched = true;
            break;
        }
    }
    assert!(parched, "dead of thirst within two days");
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(&e.kind, EventKind::Died { cause, .. } if cause == "thirst")));
}

#[test]
fn foul_water_and_ice_do_not_serve_a_well_and_a_winter_hearth_do() {
    let (mut w, p) = by_a_pond(4333, 1);
    mild(&mut w, 15.0);
    let shore = TilePos::new(25, 22, 2);
    assert!(hf_sim::water::can_drink_at(&w, shore), "beside the pond");
    // Water standing in the workings is not for drinking.
    let pit = TilePos::new(40, 40, 2);
    w.grid.set(pit, water_tile());
    w.dug.insert(pit);
    assert!(!hf_sim::water::can_drink_at(&w, TilePos::new(41, 40, 2)));
    // Nor flood water over the land.
    let flood = TilePos::new(45, 40, 2);
    w.grid.set(flood, water_tile());
    w.flooded.insert(flood);
    assert!(!hf_sim::water::can_drink_at(&w, TilePos::new(46, 40, 2)));
    // A hard frost ices the pond; a well still serves, and a hearth melts snow.
    mild(&mut w, -8.0);
    assert!(!hf_sim::water::can_drink_at(&w, shore), "iced over");
    let well = TilePos::new(35, 35, 2);
    w.place_structure(StructureKind::Well, well, Material::STONE_BLOCK, true);
    assert!(hf_sim::water::can_drink_at(&w, TilePos::new(36, 35, 2)));
    let hearth = TilePos::new(50, 50, 2);
    w.place_structure(StructureKind::Hearth, hearth, Material::STONE_BLOCK, true);
    assert!(hf_sim::water::can_drink_at(&w, TilePos::new(51, 50, 2)));
    mild(&mut w, 15.0);
    assert!(
        !hf_sim::water::can_drink_at(&w, TilePos::new(51, 50, 2)),
        "a hearth is no water in summer"
    );
    let _ = p;
}

/// #433: a household whose water is more than a few steps from home keeps a day's water in a
/// barrel by home, carried a bucket at a time, and drinks from it.
#[test]
fn a_household_far_from_water_keeps_a_store_and_drinks_from_it() {
    let mut w = World::generate(4337, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(40, 40, 2);
    w.meta.tick = TICKS_PER_HOUR * 9;
    for y in 20..=24 {
        for x in 20..=24 {
            w.grid.set(TilePos::new(x, y, 2), water_tile());
        }
    }
    w.rebuild_caches();
    hf_sim::floods::survey(&mut w);
    let ids = w.found_party(3);
    mild(&mut w, 15.0);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    let barrel = home.offset(2, 2, 0);
    w.place_structure(StructureKind::Barrel, barrel, Material::WOOD, true);
    w.drop_item(hf_sim::items::ItemKind::Bucket, 1, home.offset(1, 0, 0));
    w.meta.autoplan = true;
    w.ensure_nav();
    for a in w.agents.iter_mut() {
        a.needs.thirst = 1.0;
        a.needs.hunger = 1.0;
    }
    let mut filled = 0;
    for _ in 0..8 * TICKS_PER_HOUR {
        w.step();
        filled = filled.max(w.communities.iter().find(|c| c.id == cid).unwrap().water);
    }
    assert!(
        filled >= hf_sim::water::VESSEL,
        "water carried home: {filled}"
    );
    // Beside the barrel, a thirsty one drinks from the store.
    let before = w.communities.iter().find(|c| c.id == cid).unwrap().water;
    if before > 0 {
        let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
        w.agents[i].pos = barrel.offset(1, 0, 0);
        w.agents[i].needs.thirst = 0.1;
        assert!(hf_sim::water::drink(&mut w, i));
        assert_eq!(
            w.communities.iter().find(|c| c.id == cid).unwrap().water,
            before - 1
        );
    }
}

/// #433: in a drought the pond's edge dries and what was its middle is the water's edge; they
/// drink there.
#[test]
fn when_the_shore_dries_they_drink_at_the_waters_new_edge() {
    let (mut w, p) = by_a_pond(4338, 1);
    mild(&mut w, 15.0);
    for y in 20..=24 {
        for x in 20..=24 {
            if x == 20 || x == 24 || y == 20 || y == 24 {
                w.grid.set(TilePos::new(x, y, 2), Tile::AIR);
            }
        }
    }
    w.rebuild_caches();
    w.ensure_nav();
    let me = w.agents[p[0]].pos;
    let s = hf_sim::water::nearest(&w, me, 60).expect("water still");
    assert!(
        (21..=23).contains(&s.source.x) && (21..=23).contains(&s.source.y),
        "{s:?}"
    );
    assert!(hf_sim::water::can_drink_at(&w, s.stand));
}

/// #415's order, #433: thirsty and hungry alike, with food in hand and the pond a walk off,
/// they go to drink first.
#[test]
fn thirsty_and_hungry_they_drink_first() {
    let (mut w, p) = by_a_pond(4339, 1);
    mild(&mut w, 15.0);
    let i = p[0];
    let me = w.agents[i].id;
    w.agents[i].needs.thirst = 0.3;
    w.agents[i].needs.hunger = 0.3;
    w.agents[i].activity = hf_sim::mind::Activity::Idle;
    w.agents[i].idle_ticks = 0;
    let id = w.alloc_item_id();
    w.items.push(hf_sim::items::Item {
        id,
        kind: hf_sim::items::ItemKind::Meal,
        count: 2,
        place: hf_sim::items::ItemPlace::Carried(me),
        quality: 0.6,
        material: Material::AIR,
        story: None,
        rot: 0.0,
    });
    let a = &w.agents[i];
    let s = hf_sim::systems::decide::secured(&w, a, None);
    assert!(s.thirsty && s.hungry);
    for _ in 0..40 {
        w.step();
        if !matches!(w.agents[i].activity, hf_sim::mind::Activity::Idle) {
            break;
        }
    }
    assert!(
        matches!(&w.agents[i].activity, hf_sim::mind::Activity::Going { then, .. }
            if matches!(**then, hf_sim::mind::Activity::Drinking { .. })),
        "{:?}",
        w.agents[i].activity
    );
}

/// #433: an hour at empty is on record and remembered; a lapse that ends in a drink within the
/// hour leaves nothing.
#[test]
fn parched_is_an_hour_at_empty_not_a_lapse() {
    let (mut w, p) = by_a_pond(4340, 1);
    let i = p[0];
    let parched = |w: &World| {
        w.events
            .pending
            .iter()
            .filter(|e| matches!(e.kind, EventKind::Parched { .. }))
            .count()
    };
    let start = w.meta.tick - w.meta.tick % 20;
    // Half an hour at empty, then a drink.
    for k in 1..=(TICKS_PER_HOUR / 2 / 20) {
        w.meta.tick = start + k * 20;
        w.agents[i].needs.thirst = 0.0;
        hf_sim::systems::life::step(&mut w);
    }
    w.agents[i].needs.thirst = 1.0;
    w.meta.tick += 20;
    hf_sim::systems::life::step(&mut w);
    assert_eq!(parched(&w), 0, "a lapse leaves nothing");
    // An hour and more at empty: once on record.
    let start = w.meta.tick;
    for k in 1..=(2 * TICKS_PER_HOUR / 20) {
        w.meta.tick = start + k * 20;
        w.agents[i].needs.thirst = 0.0;
        hf_sim::systems::life::step(&mut w);
    }
    assert_eq!(parched(&w), 1);
}

/// #433: nobody goes down into a flood to drink. A pond in spate is drunk from only where the
/// bank stands above any flood yet seen; with every bank under the high-water line there is
/// nowhere to drink from it.
#[test]
fn no_drinking_at_water_in_spate_below_the_high_water_line() {
    let (mut w, p) = by_a_pond(4341, 1);
    mild(&mut w, 15.0);
    let me = w.agents[p[0]].pos;
    let pond = |w: &World| -> Vec<hf_sim::water::Spot> {
        hf_sim::water::spots(w, me, 60)
            .into_iter()
            .filter(|s| (20..=24).contains(&s.source.x) && (20..=24).contains(&s.source.y))
            .collect()
    };
    assert!(!pond(&w).is_empty());
    for b in w.waters.bodies.iter_mut() {
        b.high = true;
    }
    for y in 15..=29 {
        for x in 15..=29 {
            w.waters.high_water.insert((x, y));
        }
    }
    assert!(
        pond(&w).is_empty(),
        "every bank is under the high-water line"
    );
    // One bank above it: they may drink there.
    for y in 20..=24 {
        w.waters.high_water.remove(&(25, y));
    }
    let s = pond(&w);
    assert!(!s.is_empty() && s.iter().all(|s| s.stand.x == 25), "{s:?}");
}

/// #433, #418: with flood water lying by the nearest bank, they drink at a bank clear of it
/// rather than not at all.
#[test]
fn a_bank_clear_of_the_flood_serves() {
    let (mut w, p) = by_a_pond(4344, 1);
    mild(&mut w, 15.0);
    let me = w.agents[p[0]].pos;
    let first = hf_sim::water::nearest(&w, me, 60).expect("water");
    // The pond risen, its flood water over the ground by that bank.
    let k = w
        .waters
        .bodies
        .iter()
        .position(|b| b.tiles.contains(&first.source))
        .unwrap();
    w.waters.bodies[k].stage = 2.0;
    for dy in -1..=1 {
        for dx in -1..=1 {
            let q = first.stand.offset(dx, dy, 0);
            if !w.grid.get(q).is_water() {
                w.flooded.insert(q);
                w.waters.flood_of.insert(q, k as u16);
            }
        }
    }
    assert!(hf_sim::systems::hazards::in_floods_way(&w, first.stand));
    let s = hf_sim::water::nearest(&w, me, 60).expect("another bank");
    assert!(
        !hf_sim::systems::hazards::in_floods_way(&w, s.stand),
        "{s:?}"
    );
}

/// #433: one who leaves a job to go and drink comes back to it after: a thirsty worker by a
/// tree to fell walks to the pond, drinks, comes back and fells it.
#[test]
fn a_worker_who_leaves_to_drink_comes_back_and_finishes() {
    let (mut w, p) = by_a_pond(4345, 1);
    mild(&mut w, 15.0);
    let i = p[0];
    // They have visited the pond before starting this job out of sight of it.
    let water = TilePos::new(24, 24, 2);
    w.agents[i].knowledge.note(
        hf_sim::mind::Known::Water(water),
        water,
        w.meta.tick,
        hf_sim::floods::TAG_WATER_FRESH,
    );
    let tree = TilePos::new(46, 45, 2);
    w.plants.push(hf_sim::plants::Plant::new(
        tree,
        hf_sim::plants::PlantKind::Tree,
        1.0,
        0,
    ));
    w.agents[i].pos = TilePos::new(45, 45, 2);
    w.agents[i].path.clear();
    w.agents[i].activity = hf_sim::mind::Activity::Idle;
    w.agents[i].needs.hunger = 1.0;
    w.agents[i].needs.rest = 1.0;
    w.agents[i].needs.thirst = 0.2;
    let did = w.alloc_designation_id();
    w.designations.push(hf_sim::work::Designation {
        id: did,
        kind: hf_sim::work::DesignationKind::Chop,
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
    w.rebuild_caches();
    w.ensure_nav();
    let mut drank = false;
    let mut felled = false;
    for _ in 0..(TICKS_PER_HOUR * 4) {
        w.step();
        drank |= matches!(
            w.agents[i].activity,
            hf_sim::mind::Activity::Drinking { .. }
        );
        if !w
            .plants
            .iter()
            .any(|p| p.pos == tree && p.kind == hf_sim::plants::PlantKind::Tree)
        {
            felled = true;
            break;
        }
    }
    assert!(drank, "they went to drink first");
    assert!(felled, "and came back and felled the tree");
}

/// #433: a walk off for water is a walk back after it: they return to where they were.
#[test]
fn after_a_walk_to_drink_they_come_back() {
    let (mut w, p) = by_a_pond(4346, 1);
    mild(&mut w, 15.0);
    let i = p[0];
    // They have visited the pond before starting this job out of sight of it.
    let water = TilePos::new(24, 24, 2);
    w.agents[i].knowledge.note(
        hf_sim::mind::Known::Water(water),
        water,
        w.meta.tick,
        hf_sim::floods::TAG_WATER_FRESH,
    );
    let start = TilePos::new(45, 45, 2);
    w.agents[i].pos = start;
    w.agents[i].path.clear();
    w.agents[i].activity = hf_sim::mind::Activity::Idle;
    w.agents[i].needs.hunger = 1.0;
    w.agents[i].needs.rest = 1.0;
    w.agents[i].needs.thirst = 0.2;
    let mut drank_at = None;
    let mut back = false;
    for _ in 0..(TICKS_PER_HOUR * 2) {
        w.step();
        if matches!(
            w.agents[i].activity,
            hf_sim::mind::Activity::Drinking { .. }
        ) {
            drank_at = Some(w.agents[i].pos);
        }
        if drank_at.is_some() && w.agents[i].pos.dist(start) <= 3 {
            back = true;
            break;
        }
    }
    let at = drank_at.expect("they drank");
    assert!(at.dist(start) > 10, "a walk off: {at:?}");
    assert!(back, "and came back to where they were");
    // And they know where the water is.
    assert!(w.agents[i]
        .knowledge
        .map
        .keys()
        .any(|k| matches!(k, hf_sim::mind::knowledge::Known::Water(_))));
}

/// #433: one waiting at a caravan's gathering place still goes to drink when thirsty.
#[test]
fn waiting_for_a_caravan_they_still_drink() {
    let (mut w, p) = by_a_pond(4348, 2);
    mild(&mut w, 15.0);
    let (i, other) = (p[0], p[1]);
    let me = w.agents[i].pos;
    let cid = w.agents[i].community.unwrap();
    let members = vec![w.agents[i].id, w.agents[other].id];
    w.overworld.caravans.push(hf_sim::expedition::Caravan {
        id: 7,
        community: cid,
        members,
        purpose: hf_sim::expedition::Purpose::Scout { site: 0 },
        cargo: Vec::new(),
        materials: Default::default(),
        qualities: Default::default(),
        rations: 0,
        lost: 0,
        state: hf_sim::expedition::CaravanState::Forming,
        formed: w.tick(),
        departed: 0,
        finished: 0,
        depart_tile: me,
        log: Vec::new(),
    });
    // The other is still asleep far off, so the caravan waits for them.
    w.agents[other].pos = TilePos::new(55, 55, 2);
    w.agents[other].activity = hf_sim::mind::Activity::Sleeping {
        bed: None,
        until: w.tick() + 10 * TICKS_PER_HOUR,
    };
    w.agents[other].caravan = Some(7);
    w.agents[i].caravan = Some(7);
    w.agents[i].needs.hunger = 1.0;
    w.agents[i].needs.thirst = 0.1;
    w.agents[i].activity = hf_sim::mind::Activity::Loitering { ticks_left: 10_000 };
    w.agents[i].activity_ticks = 1_000;
    let id = w.agents[i].id;
    let mut drank = false;
    for _ in 0..TICKS_PER_HOUR {
        w.step();
        let a = w
            .agents
            .iter()
            .find(|a| a.id == id)
            .expect("still in the valley");
        if a.needs.thirst > 0.9 {
            drank = true;
            break;
        }
    }
    assert!(drank, "they went and drank");
}

fn wall_tile() -> Tile {
    Tile {
        material: Material::STONE_BLOCK,
        shape: Shape::Wall,
        flags: TileFlags::default(),
    }
}

/// A household by its barrel, the pond 18 tiles off (#608's town, seed 3 day 390): flat ground
/// at level 1, home at (40, 40, 2), the barrel at (42, 42, 2) holding 30 drinks, a wall built
/// on the barrel's east side with a ramp up onto it, and `n` people.
fn by_a_walled_barrel(n: usize) -> (World, Vec<usize>, TilePos, hf_sim::ids::CommunityId) {
    let mut w = World::generate(4337, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(40, 40, 2);
    w.meta.tick = TICKS_PER_HOUR * 9;
    for y in 20..=24 {
        for x in 20..=24 {
            w.grid.set(TilePos::new(x, y, 2), water_tile());
        }
    }
    w.rebuild_caches();
    hf_sim::floods::survey(&mut w);
    let ids = w.found_party(n);
    w.meta.autoplan = false;
    mild(&mut w, 15.0);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    let barrel = home.offset(2, 2, 0);
    w.place_structure(StructureKind::Barrel, barrel, Material::WOOD, true);
    w.communities
        .iter_mut()
        .find(|c| c.id == cid)
        .unwrap()
        .water = 30;
    w.grid.set(barrel.offset(1, 0, 0), wall_tile());
    w.grid.set(
        barrel.offset(2, 0, 0),
        Tile {
            material: Material::SOIL,
            shape: Shape::RampUp,
            flags: TileFlags::NONE,
        },
    );
    w.rebuild_caches();
    w.ensure_nav();
    let idx = ids
        .iter()
        .map(|id| w.agents.iter().position(|a| a.id == *id).unwrap())
        .collect();
    (w, idx, barrel, cid)
}

/// #608: a wall built beside the barrel. The place offered to drink at was the top of the wall,
/// one level up, and the drink there needed `dist <= 1`, which a level up never is: everyone
/// who went for water stood on the wall until they died of thirst beside a full barrel.
#[test]
fn a_wall_beside_the_barrel_does_not_keep_them_from_it() {
    let (mut w, p, barrel, cid) = by_a_walled_barrel(3);
    let top = barrel.offset(1, 0, 1);
    let me = w.agents[p[0]].pos;
    let offered = hf_sim::water::spots(&w, me, 60);
    let at_barrel = offered
        .iter()
        .find(|s| s.source == barrel)
        .expect("the barrel is offered");
    assert_eq!(at_barrel.stand, top, "the top of the wall is the stand");
    assert!(
        w.nav.as_ref().unwrap().reach(me, top),
        "a way up onto the wall"
    );
    for &i in &p {
        w.agents[i].needs.thirst = 0.15;
        w.agents[i].needs.hunger = 1.0;
        w.agents[i].activity = hf_sim::mind::Activity::Idle;
    }
    let mut on_top = 0;
    for _ in 0..3 * TICKS_PER_HOUR {
        w.step();
        if p.iter().any(|&i| w.agents[i].pos == top) {
            on_top += 1;
        }
    }
    for &i in &p {
        assert!(
            w.agents[i].needs.thirst > 0.6,
            "{} drank: thirst {} at {}",
            w.agents[i].name,
            w.agents[i].needs.thirst,
            w.agents[i].pos
        );
    }
    let left = w.communities.iter().find(|c| c.id == cid).unwrap().water;
    assert!(left < 30, "drunk from the barrel: {left} left");
    assert!(on_top > 0, "they went up on the wall to drink");
}

/// #608, the whole class: every place `spots` offers to drink, a person standing there drinks.
/// Shores, a well, a winter hearth and a barrel, each with a wall and a floor beside it so a
/// stand a level up is offered where one can be.
#[test]
fn every_place_offered_to_drink_gives_a_drink() {
    let (mut w, p, barrel, _) = by_a_walled_barrel(1);
    let i = p[0];
    let well = TilePos::new(30, 50, 2);
    w.place_structure(StructureKind::Well, well, Material::STONE_BLOCK, true);
    w.grid.set(well.offset(0, 1, 0), wall_tile());
    let hearth = TilePos::new(50, 30, 2);
    w.place_structure(StructureKind::Hearth, hearth, Material::STONE_BLOCK, true);
    w.grid.set(hearth.offset(0, -1, 0), wall_tile());
    // A raised bank on one side of the pond, a level above the rest of its shore.
    for y in 20..=24 {
        w.grid.set(TilePos::new(25, y, 2), wall_tile());
    }
    w.rebuild_caches();
    hf_sim::floods::survey(&mut w);
    w.ensure_nav();
    let mut checked = 0;
    for c in [15.0, -8.0] {
        mild(&mut w, c);
        for from in [barrel, well, hearth, TilePos::new(22, 26, 2)] {
            for s in hf_sim::water::spots(&w, from, 200) {
                assert!(hf_sim::water::reaches(s.stand, s.source), "{s:?} at {c} C");
                let mut probe = w.clone();
                probe.agents[i].pos = s.stand;
                probe.agents[i].needs.thirst = 0.1;
                assert!(
                    hf_sim::water::drink(&mut probe, i),
                    "no drink standing at {} for {} at {c} C",
                    s.stand,
                    s.source
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 20, "{checked} places checked");
}

/// #608: a drink that fails where they stand sends them to other water, not back to the same
/// place again and again.
#[test]
fn a_stand_where_no_drink_was_had_is_passed_over_for_other_water() {
    let (mut w, p) = by_a_pond(6081, 1);
    mild(&mut w, 15.0);
    let i = p[0];
    let me = w.agents[i].pos;
    let k = w.agents[i].knowledge.clone();
    let first = hf_sim::water::nearest_knowing(&w, i, me, 60, &k, false).expect("the pond");
    // Sent to drink somewhere dry: the drink fails and the place is noted.
    let dry = TilePos::new(40, 40, 2);
    w.agents[i].pos = dry;
    w.agents[i].activity = hf_sim::mind::Activity::Drinking {
        ticks_left: 1,
        back: None,
    };
    hf_sim::systems::act::step(&mut w);
    assert!(w.agents[i]
        .work_cooldowns
        .contains_key(&hf_sim::water::dry_key(dry)));
    // A failed drink at the pond's nearest stand: the next water offered is another stand.
    w.agents[i].pos = me;
    let until = w.meta.tick + hf_sim::water::DRY_STAND_TICKS;
    w.agents[i]
        .work_cooldowns
        .insert(hf_sim::water::dry_key(first.stand), until);
    let next = hf_sim::water::nearest_knowing(&w, i, me, 60, &k, false).expect("other water");
    assert_ne!(next.stand, first.stand);
    w.meta.tick = until;
    let again = hf_sim::water::nearest_knowing(&w, i, me, 60, &k, false).unwrap();
    assert_eq!(again.stand, first.stand, "and back to it after a while");
}

/// OST-551: the needs term reads all six needs, so one parched or freezing person with every
/// other need full is miserable on that term alone.
#[test]
fn one_empty_thirst_or_warmth_is_miserable_on_the_needs_term() {
    let w = World::opening_world(1791);
    let mut a = w.agents[0].clone();
    a.memories = Default::default();
    a.thoughts = Default::default();
    for (thirst, warmth) in [(0.05, 1.0), (1.0, 0.05)] {
        a.needs.hunger = 1.0;
        a.needs.rest = 1.0;
        a.needs.social = 1.0;
        a.needs.comfort = 1.0;
        a.needs.thirst = thirst;
        a.needs.warmth = warmth;
        let [needs, ..] = hf_sim::systems::needs::mood_terms(&a, w.meta.tick);
        assert!(
            needs <= -0.3,
            "thirst {thirst}, warmth {warmth}: needs term {needs}"
        );
    }
}

#[test]
fn a_hidden_pond_does_not_conceal_a_known_household_water_store() {
    use hf_sim::mind::Known;
    let (mut w, people) = by_a_pond(4564, 1);
    let i = people[0];
    let at = TilePos::new(30, 22, 2);
    let barrel = at.offset(4, 0, 0);
    w.agents[i].pos = at;
    w.meta.tick = 9 * TICKS_PER_HOUR;
    mild(&mut w, 15.0);
    let cid = w.agents[i].community.unwrap();
    let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
    c.home = barrel;
    c.water = 3;
    w.place_structure(StructureKind::Barrel, barrel, Material::WOOD, true);
    for y in 15..=29 {
        w.grid
            .set(TilePos::new(26, y, 2), Tile::solid(Material::GRANITE));
    }
    w.ensure_nav();
    let choose =
        |w: &World| hf_sim::water::nearest_knowing(w, i, at, 60, &w.agents[i].knowledge, false);
    assert_eq!(choose(&w).unwrap().source, barrel);
    w.grid
        .set(at.offset(1, 0, 0), Tile::solid(Material::GRANITE));
    w.ensure_nav();
    assert!(
        choose(&w).is_none(),
        "neither hidden pond nor hidden barrel was learned"
    );
    let id = w.structures.iter().find(|s| s.pos == barrel).unwrap().id;
    w.agents[i].knowledge.note(
        Known::Structure(id),
        barrel,
        w.meta.tick,
        StructureKind::Barrel as u16,
    );
    assert_eq!(choose(&w).unwrap().source, barrel);

    // A fetch job has the barrel's location, not the location of its water supply.
    let fetch = hf_sim::work::Designation {
        id: w.alloc_designation_id(),
        kind: hf_sim::work::DesignationKind::FetchWater,
        pos: barrel,
        requirements: vec![],
        work_left: 10,
        work_total: 10,
        max_workers: 1,
        priority: 5,
        created: w.meta.tick,
        failures: 0,
        done: false,
        community: Some(cid),
        owner: None,
    };
    let actor = w.agents[i].id;
    assert!(
        hf_sim::work::planner::plan_for(&w, actor, &fetch).is_none(),
        "cannot refill the barrel from itself or from the unlocated pond"
    );
    let well = at.offset(-6, 8, 0);
    w.place_structure(StructureKind::Well, well, Material::STONE_BLOCK, true);
    assert!(hf_sim::work::planner::plan_for(&w, actor, &fetch).is_none());
    let id = w.structures.iter().find(|s| s.pos == well).unwrap().id;
    w.agents[i].knowledge.note(
        Known::Structure(id),
        well,
        w.meta.tick,
        StructureKind::Well as u16,
    );
    let plan = hf_sim::work::planner::plan_for(&w, actor, &fetch)
        .unwrap()
        .plan;
    assert!(
        matches!(plan.steps.first(),Some(hf_sim::work::Step::GoTo{to,..}) if hf_sim::water::reaches(*to,well))
    );
}
