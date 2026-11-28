use hf_sim::{
    ids::TilePos,
    mind::{Activity, Known},
    save::{self, snapshot::Snapshot},
    work::{Designation, DesignationKind, Step},
    world::{gen::GenParams, Material, Shape, Tile, TileFlags},
    World,
};
fn setup() -> (World, usize, TilePos, Tile) {
    let mut w = World::generate(4337, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.autoplan = false;
    let source = TilePos::new(20, 20, 2);
    let start = TilePos::new(35, 20, 2);
    let barrel = TilePos::new(37, 20, 2);
    let dry = w.grid.get(source);
    w.grid.set(
        source,
        Tile {
            material: Material::WATER,
            shape: Shape::Floor,
            flags: TileFlags(TileFlags::WATER),
        },
    );
    w.rebuild_caches();
    hf_sim::floods::survey(&mut w);
    let id = w.found_party(1)[0];
    let i = w.agents.iter().position(|a| a.id == id).unwrap();
    w.agents[i].pos = start;
    w.agents[i].from = start;
    w.agents[i].knowledge.map.clear();
    w.agents[i]
        .knowledge
        .note(Known::Water(source), source, w.meta.tick, 0);
    let cid = w.agents[i].community.unwrap();
    w.ensure_nav();
    let d = Designation {
        id: w.alloc_designation_id(),
        kind: DesignationKind::FetchWater,
        pos: barrel,
        requirements: vec![],
        work_left: 1,
        work_total: 1,
        max_workers: 1,
        priority: 5,
        created: w.meta.tick,
        failures: 0,
        done: false,
        community: Some(cid),
        owner: None,
    };
    let plan = hf_sim::work::planner::plan_for(&w, id, &d)
        .expect("known wet source plans")
        .plan;
    assert!(
        matches!(plan.steps.first(),Some(Step::GoTo{to,..}) if hf_sim::water::reaches(*to,source))
    );
    assert!(matches!(plan.steps.get(1), Some(Step::FillWater{source:s}) if *s==source));
    w.designations.push(d);
    w.agents[i].activity = Activity::Working { plan };
    (w, i, source, dry)
}
fn stored(w: &World, i: usize) -> u32 {
    let cid = w.agents[i].community.unwrap();
    w.communities.iter().find(|c| c.id == cid).unwrap().water
}
fn isolate_water(w: &mut World, source: TilePos) {
    // Main's global query can otherwise find generated water beyond this fixture.
    w.waters.bodies.retain(|b| b.tiles.contains(&source));
    for body in &mut w.waters.bodies {
        body.tiles.retain(|p| *p == source);
    }
}
fn advance(w: &mut World, n: usize) {
    for _ in 0..n {
        w.step();
    }
}
#[test]
fn dry_after_planning_aborts_without_credit() {
    let (mut w, i, source, dry) = setup();
    isolate_water(&mut w, source);
    w.grid.set(source, dry);
    advance(&mut w, 10000);
    assert_eq!(stored(&w, i), 0);
    assert!(!w.designations[0].done);
}
#[test]
fn wet_water_is_collected_and_delivered() {
    let (mut w, i, _, _) = setup();
    advance(&mut w, 10000);
    assert_eq!(stored(&w, i), hf_sim::water::VESSEL);
    assert!(w.designations[0].done);
}
#[test]
fn fill_cannot_happen_far_from_source() {
    let (mut w, i, _, _) = setup();
    if let Activity::Working { plan } = &mut w.agents[i].activity {
        plan.cursor = 1;
    } else {
        panic!()
    }
    hf_sim::systems::work::run(&mut w, i);
    assert_eq!(stored(&w, i), 0);
    assert!(!matches!(w.agents[i].activity, Activity::Working { .. }));
}
#[test]
fn legacy_fetch_plan_replans_without_creating_water() {
    let (mut w, i, _, _) = setup();
    let site = w.designations[0].pos;
    w.agents[i].pos = site;
    if let Activity::Working { plan } = &mut w.agents[i].activity {
        plan.steps.remove(1);
        plan.cursor = 2;
    } else {
        panic!()
    }
    let mut w = save::decode(&save::encode(&Snapshot::capture(&w)).unwrap())
        .unwrap()
        .restore();
    hf_sim::systems::work::run(&mut w, i);
    assert_eq!(stored(&w, i), 0);
    assert!(!w.designations[0].done);
}
#[test]
fn saving_at_fill_replays_identically() {
    let (mut w, i, _, _) = setup();
    for _ in 0..10000 {
        if matches!(&w.agents[i].activity,Activity::Working{plan} if matches!(plan.current(),Some(Step::FillWater{..})))
        {
            break;
        }
        w.step();
    }
    assert!(
        matches!(&w.agents[i].activity,Activity::Working{plan} if matches!(plan.current(),Some(Step::FillWater{..})))
    );
    let mut loaded = save::decode(&save::encode(&Snapshot::capture(&w)).unwrap())
        .unwrap()
        .restore();
    advance(&mut w, 2000);
    advance(&mut loaded, 2000);
    assert_eq!(stored(&w, i), hf_sim::water::VESSEL);
    assert_eq!(hf_sim::digest::digest(&w), hf_sim::digest::digest(&loaded));
}
#[test]
fn water_already_filled_survives_source_drying_and_save() {
    let (mut w, i, source, dry) = setup();
    for _ in 0..10000 {
        if matches!(&w.agents[i].activity,Activity::Working{plan} if plan.cursor==2) {
            break;
        }
        w.step();
    }
    assert!(matches!(&w.agents[i].activity,Activity::Working{plan} if plan.cursor==2));
    w.grid.set(source, dry);
    let mut loaded = save::decode(&save::encode(&Snapshot::capture(&w)).unwrap())
        .unwrap()
        .restore();
    advance(&mut w, 2000);
    advance(&mut loaded, 2000);
    assert_eq!(stored(&w, i), hf_sim::water::VESSEL);
    assert_eq!(hf_sim::digest::digest(&w), hf_sim::digest::digest(&loaded));
}
#[test]
fn legacy_retirement_releases_leases_and_order_can_be_redone() {
    use hf_sim::work::ResKey;
    let (mut w, i, _, _) = setup();
    let who = w.agents[i].id;
    let site = w.designations[0].pos;
    let did = w.designations[0].id;
    let key = ResKey::Designation(did);
    assert!(w.reservations.acquire_all(who, &[key], w.tick() + 1000));
    w.agents[i].pos = site;
    if let Activity::Working { plan } = &mut w.agents[i].activity {
        plan.steps.remove(1);
        plan.cursor = 2;
    } else {
        panic!()
    }
    hf_sim::systems::work::run(&mut w, i);
    assert!(w.reservations.holder(key).is_none());
    assert_eq!(w.designations[0].failures, 0);
    assert!(!w.designations[0].done);
    w.step();
    let d = w.designations.iter().find(|d| d.id == did).unwrap().clone();
    let planned = hf_sim::work::planner::plan_for(&w, who, &d).expect("order can be replanned");
    assert!(w.reservations.acquire_all(
        who,
        &planned.keys,
        w.tick() + planned.estimate as u64 + 600
    ));
    w.agents[i].activity = Activity::Working { plan: planned.plan };
    advance(&mut w, 10000);
    assert_eq!(stored(&w, i), hf_sim::water::VESSEL);
    assert!(w.designations.iter().find(|d| d.id == did).unwrap().done);
}
#[test]
fn interruption_after_fill_does_not_credit_twice() {
    let (mut w, i, _, _) = setup();
    let who = w.agents[i].id;
    for _ in 0..10000 {
        if matches!(&w.agents[i].activity,Activity::Working{plan} if plan.cursor==2) {
            break;
        }
        w.step();
    }
    assert!(matches!(&w.agents[i].activity,Activity::Working{plan} if plan.cursor==2));
    w.agents[i].activity = Activity::Idle;
    w.step();
    let d = w.designations[0].clone();
    let p = hf_sim::work::planner::plan_for(&w, who, &d).expect("interrupted order replans");
    w.agents[i].activity = Activity::Working { plan: p.plan };
    advance(&mut w, 10000);
    assert_eq!(stored(&w, i), hf_sim::water::VESSEL);
    assert!(w.designations[0].done);
}
fn stand_for_fill(w: &mut World, i: usize) {
    let stand = match &w.agents[i].activity {
        Activity::Working { plan } => match plan.steps[0] {
            Step::GoTo { to, .. } => to,
            _ => panic!(),
        },
        _ => panic!(),
    };
    w.agents[i].pos = stand;
    if let Activity::Working { plan } = &mut w.agents[i].activity {
        plan.cursor = 1;
    }
}
#[test]
fn a_source_freezing_at_its_height_cannot_be_filled() {
    let (mut w, i, source, _) = setup();
    w.meta.founding_site.z = source.z;
    w.weather.temperature_c = 0.0;
    assert!(!hf_sim::water::frozen(&w));
    assert!(!hf_sim::water::frozen_at(&w, source));
    stand_for_fill(&mut w, i);
    w.weather.temperature_c = -1.0;
    assert!(!hf_sim::water::frozen(&w));
    assert!(hf_sim::water::frozen_at(&w, source));
    hf_sim::systems::work::run(&mut w, i);
    assert!(!matches!(w.agents[i].activity, Activity::Working { .. }));
    assert_eq!(stored(&w, i), 0);
}
#[test]
fn a_rising_flood_invalidates_the_stand_before_filling() {
    let (mut w, i, source, _) = setup();
    stand_for_fill(&mut w, i);
    let stand = w.agents[i].pos;
    let body = w
        .waters
        .bodies
        .iter_mut()
        .find(|b| b.tiles.contains(&source))
        .unwrap();
    body.high = true;
    w.waters.high_water.insert((stand.x, stand.y));
    hf_sim::systems::work::run(&mut w, i);
    assert!(!matches!(w.agents[i].activity, Activity::Working { .. }));
    assert_eq!(stored(&w, i), 0);
}
#[test]
fn wells_and_frozen_hearths_remain_valid_sources() {
    use hf_sim::structures::StructureKind;
    for kind in [StructureKind::Well, StructureKind::Hearth] {
        let (mut w, i, source, dry) = setup();
        w.grid.set(source, dry);
        if kind == StructureKind::Hearth {
            w.weather.temperature_c = -10.0;
        }
        w.place_structure(kind, source, Material::STONE_BLOCK, true);
        let s = w
            .structures
            .iter()
            .find(|s| s.kind == kind && s.pos == source)
            .unwrap()
            .id;
        w.agents[i].knowledge.map.clear();
        w.agents[i]
            .knowledge
            .note(Known::Structure(s), source, w.meta.tick, kind as u16);
        let d = w.designations[0].clone();
        let who = w.agents[i].id;
        let p = hf_sim::work::planner::plan_for(&w, who, &d).expect("structure source is planned");
        assert!(matches!(p.plan.steps.get(1),Some(Step::FillWater{source:s}) if *s==source));
        w.agents[i].activity = Activity::Working { plan: p.plan };
        stand_for_fill(&mut w, i);
        hf_sim::systems::work::run(&mut w, i);
        assert!(
            matches!(&w.agents[i].activity,Activity::Working{plan} if plan.cursor==2),
            "{kind:?}"
        );
    }
}
#[test]
fn elevated_surface_uses_its_accessible_lower_water() {
    let (mut w, i, source, _) = setup();
    let top = source.offset(0, 0, 1);
    let water = w.grid.get(source);
    w.grid.set(top, water);
    let wall = Tile {
        material: Material::LIMESTONE,
        shape: Shape::Wall,
        flags: TileFlags::NONE,
    };
    for (dx, dy) in [(0, 1), (1, 0), (0, -1), (-1, 0)] {
        w.grid.set(top.offset(dx, dy, 0), wall);
        w.grid.set(top.offset(dx, dy, 1), wall);
    }
    let body = w
        .waters
        .bodies
        .iter_mut()
        .find(|b| b.tiles.contains(&source))
        .unwrap();
    body.tiles.clear();
    body.tiles.push(top);
    w.rebuild_caches();
    w.ensure_nav();
    let d = w.designations[0].clone();
    let who = w.agents[i].id;
    let p = hf_sim::work::planner::plan_for(&w, who, &d).expect("lower water is reachable");
    assert!(matches!(p.plan.steps.get(1),Some(Step::FillWater{source:s}) if *s==source));
    w.agents[i].activity = Activity::Working { plan: p.plan };
    stand_for_fill(&mut w, i);
    hf_sim::systems::work::run(&mut w, i);
    assert!(matches!(&w.agents[i].activity,Activity::Working{plan} if plan.cursor==2));
}
#[test]
fn refill_ignores_nearby_barrel_and_finds_farther_water() {
    use hf_sim::structures::StructureKind;
    let (mut w, i, source, _) = setup();
    let barrel = w.designations[0].pos;
    let cid = w.agents[i].community.unwrap();
    let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
    c.home = barrel.offset(-2, 0, 0);
    c.water = 10;
    w.place_structure(StructureKind::Barrel, barrel, Material::WOOD, true);
    assert_eq!(
        hf_sim::water::nearest(&w, barrel, 40).unwrap().source,
        barrel,
        "ordinary drinking still sees the store"
    );
    w.agents[i].knowledge.map.clear();
    let d = w.designations[0].clone();
    let p = hf_sim::work::planner::plan_for(&w, w.agents[i].id, &d)
        .expect("global source remains available");
    assert!(matches!(p.plan.steps.get(1),Some(Step::FillWater{source:s}) if *s==source));
}
#[test]
fn refill_with_only_a_barrel_has_no_plan() {
    use hf_sim::structures::StructureKind;
    let (mut w, i, source, dry) = setup();
    isolate_water(&mut w, source);
    let barrel = w.designations[0].pos;
    let cid = w.agents[i].community.unwrap();
    let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
    c.home = barrel.offset(-2, 0, 0);
    c.water = 10;
    w.place_structure(StructureKind::Barrel, barrel, Material::WOOD, true);
    w.grid.set(source, dry);
    assert_eq!(
        hf_sim::water::nearest(&w, barrel, 40).unwrap().source,
        barrel
    );
    assert!(hf_sim::work::planner::plan_for(&w, w.agents[i].id, &w.designations[0]).is_none());
}
#[test]
fn refill_can_replan_to_an_alternate_after_the_first_source_drains() {
    let (mut w, i, source, dry) = setup();
    isolate_water(&mut w, source);
    let other = source.offset(0, 8, 0);
    let tile = w.grid.get(source);
    w.grid.set(other, tile);
    w.waters.bodies[0].tiles.push(other);
    w.grid.set(source, dry);
    w.ensure_nav();
    let p = hf_sim::work::planner::plan_for(&w, w.agents[i].id, &w.designations[0])
        .expect("alternate water is available");
    assert!(matches!(p.plan.steps.get(1),Some(Step::FillWater{source:s}) if *s==other));
}
