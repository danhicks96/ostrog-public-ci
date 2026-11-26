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

#[test]
fn committed_drinker_rechecks_a_foul_shore_before_the_last_sip() {
    use hf_sim::mind::{Activity, Known};
    use hf_sim::{systems::act, water};
    let foul = TilePos::new(24, 22, 2);
    let clean = foul.offset(4, 0, 0);
    let bank = foul.offset(1, 0, 0);

    #[derive(Clone, Copy)]
    enum Alternative {
        Reachable,
        Unknown,
        Dried,
        CoolingDown,
        Sealed,
        WellHere,
    }
    use Alternative::*;
    for (recognized, alternative, thirst, refuses) in [
        (true, Reachable, 0.67, true),
        (false, Reachable, 0.67, false),
        (true, Unknown, 0.67, false),
        (true, Dried, 0.67, false),
        (true, CoolingDown, 0.67, false),
        (true, Sealed, 0.67, false),
        (true, WellHere, 0.67, false),
        (true, Reachable, 0.20, false),
    ] {
        let mut w = flat(4561);
        w.grid.set(foul, water_tile());
        if !matches!(alternative, Dried) {
            w.grid.set(clean, water_tile());
        }
        if matches!(alternative, Sealed) {
            for (dx, dy) in [(0, 1), (1, 0), (0, -1), (-1, 0)] {
                for dz in 0..=1 {
                    w.grid
                        .set(clean.offset(dx, dy, dz), Tile::solid(Material::GRANITE));
                }
            }
        }
        if matches!(alternative, WellHere) {
            w.place_structure(
                hf_sim::structures::StructureKind::Well,
                bank,
                Material::GRANITE,
                false,
            );
        }
        w.rebuild_caches();
        floods::survey(&mut w);
        w.ensure_nav();
        w.weather.temperature_c = 10.0;
        let who = w.found_party(1)[0];
        let i = w.agents.iter().position(|a| a.id == who).unwrap();
        w.waters.foul.insert(foul, FOUL_HOURS);
        let a = &mut w.agents[i];
        a.pos = bank;
        a.needs.thirst = thirst;
        a.activity = Activity::Drinking {
            ticks_left: 2,
            back: None,
        };
        if !matches!(alternative, Unknown) {
            a.knowledge.note(
                Known::Water(clean),
                clean,
                w.meta.tick,
                floods::TAG_WATER_FRESH,
            );
        }
        if matches!(alternative, CoolingDown) {
            let stand = water::spots(&w, bank, 60)
                .into_iter()
                .find(|s| s.source == clean)
                .unwrap()
                .stand;
            w.agents[i]
                .work_cooldowns
                .insert(water::dry_key(stand), w.meta.tick + 100);
        }
        act::step(&mut w);
        assert_eq!(w.agents[i].needs.thirst, thirst);
        if recognized {
            w.agents[i].knowledge.note(
                Known::Water(foul),
                foul,
                w.meta.tick,
                floods::TAG_WATER_FOUL,
            );
        }
        if refuses {
            let chosen =
                water::nearest_knowing(&w, i, bank, 60, &w.agents[i].knowledge, false).unwrap();
            assert_eq!(chosen.source, clean);
        }
        act::step(&mut w);
        assert_eq!(w.agents[i].needs.thirst, if refuses { thirst } else { 1.0 });
        assert!(!w.agents[i]
            .work_cooldowns
            .contains_key(&water::dry_key(bank)));
        if refuses {
            assert!(matches!(w.agents[i].activity, Activity::Idle));
            let reason = w.events.pending.iter().find_map(|e| match &e.kind {
                hf_sim::events::EventKind::PlanFailed { agent, reason, .. } if *agent == who => {
                    Some(reason)
                }
                _ => None,
            });
            assert!(reason.is_some_and(|r| r.contains("foul water")));
            let chosen =
                water::nearest_knowing(&w, i, bank, 60, &w.agents[i].knowledge, false).unwrap();
            w.agents[i].pos = chosen.stand;
            w.agents[i].activity = Activity::Drinking {
                ticks_left: 1,
                back: None,
            };
            act::step(&mut w);
            assert_eq!(w.agents[i].needs.thirst, 1.0);
        }
    }
}

#[test]
fn an_isolated_clean_pond_does_not_rule_out_reachable_foul_water() {
    use hf_sim::{mind::Known, water};
    let near = TilePos::new(23, 22, 2);
    let clean = TilePos::new(28, 22, 2);
    let from = near.offset(0, 1, 0);
    for closed in [false, true] {
        let mut w = flat(4561);
        w.grid.set(near, water_tile());
        w.grid.set(clean, water_tile());
        for x in -2_i32..=2 {
            for y in -2_i32..=2 {
                if x.abs().max(y.abs()) == 2 && (closed || (x, y) != (-2, 0)) {
                    for z in 0..=2 {
                        w.grid
                            .set(clean.offset(x, y, z), Tile::solid(Material::GRANITE));
                    }
                }
            }
        }
        w.rebuild_caches();
        floods::survey(&mut w);
        w.ensure_nav();
        w.weather.temperature_c = 10.0;
        let who = w.found_party(1)[0];
        let i = w.agents.iter().position(|a| a.id == who).unwrap();
        let clean_bank = water::spots(&w, from, 8)
            .into_iter()
            .find(|s| s.source == clean)
            .unwrap()
            .stand;
        assert_eq!(w.nav.as_ref().unwrap().reach(from, clean_bank), !closed);
        w.waters.foul.insert(near, FOUL_HOURS);
        let a = &mut w.agents[i];
        a.pos = from;
        a.needs.thirst = 0.45;
        a.knowledge.note(
            Known::Water(near),
            near,
            w.meta.tick,
            floods::TAG_WATER_FOUL,
        );
        a.knowledge.note(
            Known::Water(clean),
            clean,
            w.meta.tick,
            floods::TAG_WATER_FRESH,
        );
        let pick = water::nearest_knowing(&w, i, from, 8, &w.agents[i].knowledge, false);
        assert_eq!(
            pick.map(|s| s.source),
            Some(if closed { near } else { clean })
        );
        let desperate_pick =
            water::nearest_knowing(&w, i, from, 8, &w.agents[i].knowledge, true).unwrap();
        assert_eq!(desperate_pick.source, near);
        if closed {
            w.agents[i].knowledge.note(
                Known::Water(near),
                near,
                w.meta.tick,
                floods::TAG_WATER_SALT,
            );
            assert!(water::nearest_knowing(&w, i, from, 8, &w.agents[i].knowledge, true).is_none());
        }
    }
}
