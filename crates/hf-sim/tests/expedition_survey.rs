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
fn only_reported_upper_faces_change_a_column_readout() {
    use hf_sim::{
        expedition::survey::Survey,
        view::block::inspect_with_survey,
        world::{Material, Tile, TileFlags},
    };
    let mut w = World::opening_world(7);
    let p = w.meta.founding_site.offset(3, 0, 0);
    let above = p.offset(0, 0, 1);
    let mut water = Tile::floor(Material::WATER);
    water.flags = TileFlags(TileFlags::WATER);
    w.grid.set(p, water);
    w.grid.set(above, Tile::AIR);
    let mut chart = Survey {
        tick: 100,
        ..Default::default()
    };
    chart.valley.insert(
        (p.x, p.y, p.z),
        (100, water.pack(), Material::GRANITE.0, -1),
    );
    let before = inspect_with_survey(&w, p, Some(&chart));
    w.grid.set(above, Tile::solid(Material::COAL));
    let covered = inspect_with_survey(&w, p, Some(&chart));
    assert_eq!(
        (before.surface_z, &before.temperature, &before.minerals),
        (covered.surface_z, &covered.temperature, &covered.minerals)
    );
    assert!(!covered.minerals.iter().any(|m| m.contains("coal")));
    w.grid.set(above, water);
    let flooded = inspect_with_survey(&w, p, Some(&chart));
    assert_eq!(flooded.water, before.water);
    assert!(flooded.water.contains("unknown"));
    chart.valley.insert(
        (above.x, above.y, above.z),
        (100, water.pack(), Material::WATER.0, -1),
    );
    assert_eq!(inspect_with_survey(&w, p, Some(&chart)).water_depth, 2);
    // A scout standing on the cap reports its material underfoot without revealing its contents.
    chart.valley.clear();
    chart.valley.insert(
        (above.x, above.y, above.z + 1),
        (101, Tile::AIR.pack(), Material::COAL.0, -1),
    );
    assert_eq!(chart.reported_height(p.x, p.y), Some(above.z));
    let mapped = chart.reported_heights(
        w.grid.width as usize,
        w.grid.height as usize,
        w.grid.z_min - 1,
    );
    assert_eq!(mapped[w.grid.col(p.x, p.y)], above.z);
    assert_eq!(chart.reported_tile(above).unwrap().material, Material::COAL);
    chart.valley.insert(
        (above.x, above.y, above.z),
        (102, Tile::AIR.pack(), Material::AIR.0, -1),
    );
    assert_eq!(chart.reported_height(p.x, p.y), None);
    assert_eq!(chart.reported_tile(above), Some(Tile::AIR));
}
