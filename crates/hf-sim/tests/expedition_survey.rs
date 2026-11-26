use hf_sim::{
    expedition::{route_level as road, survey, Overworld, SiteKind},
    items::ItemKind,
    setup::WorldSetup,
    time::{Calendar, TICKS_PER_DAY},
    World,
};

#[test]
fn a_picked_patch_is_shared_on_the_return_trip() {
    let mut land = Overworld::generate(91);
    let site = land
        .sites
        .iter()
        .find(|s| s.kind == SiteKind::Forest)
        .unwrap()
        .clone();
    let seed = road::route_seed(91, &site);
    let summer = Calendar::from_tick(6 * TICKS_PER_DAY);
    let at = (0..500)
        .find(|&x| road::tile(seed, &site, x, road::path_y(seed, x) + 1).1 == 5)
        .unwrap();
    let amount = road::harvest(
        &mut land,
        seed,
        &site,
        i64::from(at) * road::TILE,
        summer,
        ItemKind::Berries,
        100,
    );
    assert!(amount > 0);
    assert_eq!(
        0,
        road::harvest(
            &mut land,
            seed,
            &site,
            i64::from(at) * road::TILE,
            summer,
            ItemKind::Berries,
            100
        )
    );
}

#[test]
fn an_unwitnessed_change_leaves_the_old_map_report() {
    let mut settings = WorldSetup::new(11);
    settings.history_years = 0;
    settings.realm_years = 10;
    let mut w = World::from_setup(&settings);
    let cid = w.communities[0].id;
    survey::observe(&mut w);
    let home = w.meta.setup.as_ref().unwrap().region.unwrap();
    let before = w.overworld.surveys[&cid].regions[&home].clone();
    let body = w
        .agents
        .iter()
        .find(|a| a.community == Some(cid))
        .unwrap()
        .pos;
    let key = (body.x, body.y, body.z);
    let reported = w.overworld.surveys[&cid].valley[&key];
    for a in &mut w.agents {
        a.community = None;
    }
    w.meta.tick += 200;
    w.grid.set(
        body,
        hf_sim::world::Tile::solid(hf_sim::world::Material::LIMESTONE),
    );
    survey::observe(&mut w);
    let memory = &w.overworld.surveys[&cid];
    assert_eq!(before.tick, memory.regions[&home].tick);
    assert_eq!(memory.valley[&key], reported);
    assert_ne!(reported.1, w.grid.get(body).pack());
    assert!(memory
        .valley
        .values()
        .all(|(tick, _, _, _)| *tick < memory.tick));
    assert_eq!(survey::footprint(4, 5, 3).count(), 37);
}

#[test]
fn a_colony_without_a_realm_still_observes_its_floor_and_keeps_the_seven_hidden() {
    let mut settings = WorldSetup::new(11);
    settings.history_years = 0;
    settings.realm_years = 0;
    let mut w = World::from_setup(&settings);
    let cid = w.communities[0].id;
    let lost = hf_sim::view::known::unfound(&w);
    assert_eq!(lost.len(), 7);
    survey::observe(&mut w);
    let memory = &w.overworld.surveys[&cid];
    assert!(!memory.valley.is_empty());
    assert!(memory.regions.is_empty());
    assert!(w.realm_map.is_none());
    // Even a living, sworn lost inhabitant cannot report their location before discovery.
    for a in &mut w.agents {
        a.community = lost.contains(&a.id).then_some(cid);
    }
    w.meta.tick += 200;
    survey::observe(&mut w);
    let memory = &w.overworld.surveys[&cid];
    assert!(memory.valley.values().all(|row| row.0 < memory.tick));
}

#[test]
fn old_overworld_bytes_survive_and_nonempty_watcher_records_round_trip() {
    // Preserve the original field order: the world digest hashes the named MessagePack bytes.
    #[derive(serde::Serialize)]
    struct LegacyOverworld {
        sites: Vec<hf_sim::expedition::Site>,
        caravans: Vec<hf_sim::expedition::Caravan>,
        next_caravan_id: u32,
        away: Vec<hf_sim::agents::Agent>,
    }
    let legacy = LegacyOverworld {
        sites: vec![],
        caravans: vec![],
        next_caravan_id: 0,
        away: vec![],
    };
    let bytes = rmp_serde::to_vec_named(&legacy).unwrap();
    let world: Overworld = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(rmp_serde::to_vec_named(&world).unwrap(), bytes);
    let colony = hf_sim::ids::CommunityId(7);
    let patch = (3, 4, 5);
    let floor = (9, 8, -1);
    for picked in [false, true] {
        for surveyed in [false, true] {
            let mut world = world.clone();
            if picked {
                world.picked.insert(patch, (200, 12));
            }
            if surveyed {
                let chart = world.surveys.entry(colony).or_default();
                chart.tick = 333;
                chart.valley.insert(floor, (310, 42, 2, -1));
            }
            let bytes = rmp_serde::to_vec_named(&world).unwrap();
            let fields: std::collections::BTreeMap<String, serde::de::IgnoredAny> =
                rmp_serde::from_slice(&bytes).unwrap();
            assert_eq!(fields.contains_key("picked"), picked);
            assert_eq!(fields.contains_key("surveys"), surveyed);
            let back: Overworld = rmp_serde::from_slice(&bytes).unwrap();
            assert_eq!(
                back.picked.get(&patch).copied(),
                picked.then_some((200, 12))
            );
            assert_eq!(
                back.surveys
                    .get(&colony)
                    .map(|s| (s.tick, s.valley[&floor])),
                surveyed.then_some((333, (310, 42, 2, -1)))
            );
        }
    }
    let mut explicit = serde_json::to_value(&legacy).unwrap();
    explicit["picked"] = serde_json::json!({});
    explicit["surveys"] = serde_json::json!({});
    let old_watcher: Overworld =
        rmp_serde::from_slice(&rmp_serde::to_vec_named(&explicit).unwrap()).unwrap();
    assert_eq!(rmp_serde::to_vec_named(&old_watcher).unwrap(), bytes);
}
