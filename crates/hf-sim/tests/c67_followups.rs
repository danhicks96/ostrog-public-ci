//! C69: the C67 follow-ups the tester filed (#239 to #243).

use hf_sim::ids::TilePos;
use hf_sim::structures::StructureKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn meadow(seed: u64, people: usize) -> (World, hf_sim::ids::CommunityId) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    let ids = w.found_party(people);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    w.ensure_nav();
    (w, cid)
}

#[test]
fn a_house_is_not_grown_round_a_hearth() {
    // #241: a split-off's house enclosed the community's hearth; sparks burned it for good.
    let (mut w, cid) = meadow(24101, 6);
    let home = w.communities[0].home;
    // A hearth a little way from home, as the settlement step places one.
    let hearth = home.offset(-4, -4, 0);
    w.place_structure(
        StructureKind::Hearth,
        hearth,
        hf_sim::world::Material::STONE_BLOCK,
        true,
    );
    let gap = hf_sim::houses::bed_gap(&w, cid);
    let grown = hf_sim::houses::new_house(&w, home.offset(-3, -3, 0), home, 6, gap)
        .expect("a house is grown");
    for p in &grown.floor {
        assert!(
            p.dist(hearth) > 2,
            "the floor keeps two off the hearth (#257): {p:?}"
        );
    }
    for (q, _) in hf_sim::houses::walls(&grown.floor) {
        assert!(
            q.dist(hearth) > 2,
            "the wall keeps two off the hearth (#257): {q:?}"
        );
    }
}

#[test]
fn a_workshop_with_no_place_is_given_up_once_for_a_season() {
    // #242: a loom was proposed every five days for sixty, never built.
    use hf_sim::events::EventKind;
    use hf_sim::project::ProjectKind;
    let (mut w, cid) = meadow(24201, 10);
    w.meta.autoplan = true;
    // Nowhere on their own ground: another community's home a few tiles off claims it all.
    let other = w.found_community(w.communities[0].home.offset(3, 0, 0));
    let _ = other;
    let by = w.communities[0].members[0];
    let id = w.next_project_id;
    w.next_project_id += 1;
    let tick = w.meta.tick;
    w.projects.push(hf_sim::project::Project {
        id,
        by,
        community: cid,
        kind: ProjectKind::Workshop(StructureKind::Loom),
        because: "test".into(),
        source: None,
        proposed: tick,
        work: None,
        done: false,
        site: None,
    });
    // Walls everywhere flat ground could be: no spot at all.
    for y in 2..60 {
        for x in 2..60 {
            let p = TilePos::new(x, y, 2);
            if p.dist(w.communities[0].home) > 2 {
                w.grid.set(
                    p,
                    hf_sim::world::Tile::solid(hf_sim::world::Material::GRANITE),
                );
            }
        }
    }
    w.rebuild_caches();
    w.ensure_nav();
    hf_sim::systems::settlement::step(&mut w);
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    assert!(
        c.no_site.iter().any(|(k, _)| *k == StructureKind::Loom),
        "the loom is remembered as having no place"
    );
    let gave_up = w.events.drain().iter().filter(|e| matches!(&e.kind, EventKind::Chose { what, .. } if what.contains("gave up on the loom"))).count();
    assert_eq!(gave_up, 1, "said once");
    assert!(w.projects.iter().all(|p| p.done), "the project is closed");
}

#[test]
fn an_orphaned_workshop_order_is_adopted_instead_of_reproposed() {
    use hf_sim::project::ProjectKind;
    use hf_sim::work::DesignationKind;
    use hf_sim::world::Material;

    let (mut w, cid) = meadow(24202, 10);
    w.meta.autoplan = true;
    let by = w.communities[0].members[0];
    let tick = w.meta.tick;
    let project_id = w.next_project_id;
    w.next_project_id += 1;
    w.projects.push(hf_sim::project::Project {
        id: project_id,
        by,
        community: cid,
        kind: ProjectKind::Workshop(StructureKind::CarpenterBench),
        because: "the old order survived".into(),
        source: None,
        proposed: tick,
        work: None,
        done: false,
        site: None,
    });

    w.meta.planning_for = Some(cid);
    let orphan_pos = w.communities[0].home.offset(3, 0, 0);
    let orphan = hf_sim::systems::settlement::make(
        &mut w,
        DesignationKind::BuildStructure(StructureKind::CarpenterBench, Material::WOOD),
        orphan_pos,
        5,
    );
    let orphan_id = orphan.id;
    w.designations.push(orphan);
    w.rebuild_caches();
    w.ensure_nav();

    hf_sim::systems::settlement::step(&mut w);

    let project = w.projects.iter().find(|p| p.id == project_id).unwrap();
    assert_eq!(
        project.work,
        Some(orphan_id),
        "the old workshop order is adopted"
    );
    assert!(!project.done, "an adopted order must not time out");
    assert_eq!(
        w.projects
            .iter()
            .filter(|p| p.community == cid
                && p.kind == ProjectKind::Workshop(StructureKind::CarpenterBench))
            .count(),
        1,
        "an orphaned order must not produce a duplicate proposal"
    );
}

#[test]
fn a_community_does_not_move_back_to_the_camp_it_left() {
    // #243: moved back to the camp left twenty days before.
    let (mut w, cid) = {
        let mut w = World::generate(24301, GenParams::default());
        w.flatten(2, 2, 120, 120, 1);
        w.plants.clear();
        w.meta.founding_site = TilePos::new(60, 60, 2);
        w.meta.tick = TICKS_PER_DAY * 30 + TICKS_PER_HOUR * 8;
        w.rebuild_caches();
        let ids = w.found_party(8);
        let cid = w
            .agents
            .iter()
            .find(|a| a.id == ids[0])
            .unwrap()
            .community
            .unwrap();
        w.meta.autoplan = true;
        w.ensure_nav();
        (w, cid)
    };
    let tick = w.meta.tick;
    let old = TilePos::new(90, 60, 2);
    {
        let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
        c.old_camps.push((old, tick - 20 * TICKS_PER_DAY));
        for (k, what) in [
            "Ana died of starvation",
            "strangers took from the store",
            "the rock came down",
            "Bren died of the cold",
        ]
        .iter()
        .enumerate()
        {
            c.hardships.push((
                tick - (4 - k as u64) * TICKS_PER_DAY,
                what.to_string(),
                hf_sim::ids::EventId(k as u32 + 1),
            ));
        }
    }
    let ids: Vec<_> = w
        .agents
        .iter()
        .filter(|a| a.community == Some(cid))
        .map(|a| a.id)
        .collect();
    for a in w.agents.iter_mut().filter(|a| a.community == Some(cid)) {
        for o in &ids {
            if *o != a.id {
                a.relationships.entry(*o).or_default().opinion = 0.6;
            }
        }
    }
    hf_sim::systems::moving::consider(&mut w, cid);
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    assert!(c.home.dist(old) > 20, "not back to {old:?}: {:?}", c.home);
}

#[test]
fn a_return_is_judged_against_the_receivers_own_need() {
    // #243, then #260: 115 trades in 115 brought nothing back when the return asked the town
    // that lacks to hold more a head than the hamlet that gives. Now: what B holds beyond
    // four days of its own use, in a kind A lacks.
    assert!(
        hf_sim::systems::society::returns(200, 30, 10, 10),
        "the town has 200 grain for 30, the hamlet lacks it: some goes back"
    );
    assert!(
        !hf_sim::systems::society::returns(100, 30, 10, 10),
        "not within the town's own four days"
    );
    assert!(
        !hf_sim::systems::society::returns(200, 30, 600, 10),
        "not a kind the hamlet has plenty of"
    );
}

#[test]
fn every_house_is_furnished_not_only_the_first() {
    // #239: step 6b ran for `house_sites.first()` only; later houses stood on bare grass.
    use hf_sim::work::DesignationKind;
    use hf_sim::world::Material;
    let (mut w, cid) = meadow(23901, 6);
    w.meta.autoplan = true;
    // Two houses: the first unfinished (a wall missing), the second standing and roofed.
    w.build_house(10, 10, 7, 6, 1, Material::WOOD, Material::SOIL);
    w.build_house(40, 10, 7, 6, 1, Material::WOOD, Material::SOIL);
    w.grid
        .set(TilePos::new(13, 10, 2), hf_sim::world::Tile::AIR);
    for origin in [TilePos::new(10, 10, 2), TilePos::new(40, 10, 2)] {
        for p in hf_sim::systems::settlement::house_interior(&w, origin) {
            w.roofs.insert(
                p,
                hf_sim::building::Roof {
                    material: Material::WOOD,
                    quality: 0.6,
                    built: 0,
                    snow: 0.0,
                },
            );
        }
    }
    {
        let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
        c.house_sites = vec![TilePos::new(10, 10, 2), TilePos::new(40, 10, 2)];
        c.house_stage = vec![2, 3];
    }
    w.rebuild_caches();
    w.ensure_nav();
    hf_sim::systems::settlement::step(&mut w);
    let second: Vec<TilePos> =
        hf_sim::systems::settlement::house_interior(&w, TilePos::new(40, 10, 2));
    let furnished = w.designations.iter().any(|d| {
        second.contains(&d.pos)
            && matches!(
                d.kind,
                DesignationKind::BuildFloor(_) | DesignationKind::BuildStructure(..)
            )
    });
    assert!(furnished, "the second house gets its floor and fittings");
}

#[test]
fn a_community_that_moves_leaves_its_orders_behind() {
    // C69 (found in the seed 3 soak): the old camp's open wall orders held the "walls posted"
    // gate, so a community of eighty-five built no house for five hundred days.
    use hf_sim::work::DesignationKind;
    use hf_sim::world::Material;
    let mut w = World::generate(24302, GenParams::default());
    w.flatten(2, 2, 120, 120, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(60, 60, 2);
    w.meta.tick = TICKS_PER_DAY * 30 + TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    let ids = w.found_party(8);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    w.meta.autoplan = true;
    w.ensure_nav();
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    // Their own wall orders by the old home, one with planks delivered.
    for k in 0..3 {
        let mut d = hf_sim::systems::settlement::make(
            &mut w,
            DesignationKind::BuildWall(Material::WOOD),
            home.offset(-6 + k, -5, 0),
            5,
        );
        d.community = Some(cid);
        if k == 0 {
            d.requirements = vec![hf_sim::work::Requirement {
                kind: hf_sim::items::ItemKind::Plank,
                needed: 2,
                delivered: 2,
                material: Material::OAK,
                want: None,
                quality: 0.7,
            }];
        }
        w.designations.push(d);
    }
    let tick = w.meta.tick;
    {
        let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
        for (k, what) in [
            "Ana died of starvation",
            "strangers took from the store",
            "the rock came down",
            "Bren died of the cold",
        ]
        .iter()
        .enumerate()
        {
            c.hardships.push((
                tick - (4 - k as u64) * TICKS_PER_DAY,
                what.to_string(),
                hf_sim::ids::EventId(k as u32 + 1),
            ));
        }
    }
    let members: Vec<_> = w
        .agents
        .iter()
        .filter(|a| a.community == Some(cid))
        .map(|a| a.id)
        .collect();
    for a in w.agents.iter_mut().filter(|a| a.community == Some(cid)) {
        for o in &members {
            if *o != a.id {
                a.relationships.entry(*o).or_default().opinion = 0.6;
            }
        }
    }
    hf_sim::systems::moving::consider(&mut w, cid);
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    assert!(c.home.dist(home) >= 20, "they moved");
    assert!(
        !w.designations
            .iter()
            .any(|d| !d.done && matches!(d.kind, DesignationKind::BuildWall(_))),
        "the old site's wall orders are dropped"
    );
    assert!(
        w.items
            .iter()
            .any(|it| it.kind == hf_sim::items::ItemKind::Plank
                && it.count == 2
                && matches!(it.place, hf_sim::items::ItemPlace::Ground(p) if p.dist(home) <= 10)),
        "what was delivered lies on the ground there"
    );
}

#[test]
fn an_added_room_never_takes_the_step_outside_a_door() {
    // #253: a room grown over the door's step turned the door into one between two closed
    // rooms, and whoever was inside when the last wall went up stayed in for a year.
    use hf_sim::world::Material;
    let (mut w, cid) = meadow(25301, 8);
    let home = w.communities[0].home;
    // A standing house with its door on the east wall, home to the east of it.
    let o = TilePos::new(18, 28, 2);
    w.build_house(o.x, o.y, 7, 6, 1, Material::WOOD, Material::SOIL);
    let door = TilePos::new(o.x + 6, o.y + 2, 2);
    {
        let mut t = hf_sim::world::Tile::solid(Material::WOOD);
        t.flags = hf_sim::world::TileFlags(hf_sim::world::TileFlags::DOOR);
        w.grid.set(door, t);
    }
    {
        let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
        c.house_sites = vec![o];
        c.house_stage = vec![3];
    }
    let mut plan = hf_sim::houses::plan(&w, o);
    plan.doors = vec![door];
    w.communities[0].house_plans = vec![plan];
    w.rebuild_caches();
    w.ensure_nav();
    let gap = hf_sim::houses::bed_gap(&w, cid);
    let step = door.offset(1, 0, 0);
    for sleepers in [2u32, 4, 6] {
        if let Some((floor, _)) = hf_sim::houses::annex(&w, o, home, sleepers, gap) {
            assert!(!floor.contains(&step), "the room took the door's step");
            let ring = hf_sim::houses::walls(&floor);
            assert!(
                !ring.iter().any(|(q, _)| *q == step),
                "the room's wall took the door's step"
            );
        }
    }
}

#[test]
fn newcomers_arrive_where_the_camp_can_be_reached() {
    // #254: an arrival on a rim with no way to the camp stood there until they froze.
    use hf_sim::world::{Material, Tile};
    let (mut w, cid) = meadow(25401, 4);
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    // A wall across the valley at y = 6 shuts the north strip off from the camp.
    let width = w.grid.width;
    for x in 0..width {
        for z in 1..5 {
            w.grid
                .set(TilePos::new(x, 6, z), Tile::solid(Material::GRANITE));
        }
    }
    w.rebuild_caches();
    w.ensure_nav();
    let shut = w.grid.surface(30, 2);
    assert!(
        !w.nav.as_ref().unwrap().reach(shut, home),
        "the strip is shut off"
    );
    let got = hf_sim::systems::society::arrival_tile(&w, shut, 0, home).expect("some way in");
    assert!(
        w.nav.as_ref().unwrap().reach(got, home),
        "from {got:?} the camp can be reached"
    );
    assert!(got.y > 6, "not on the shut strip: {got:?}");
    let open = w.grid.surface(30, 50);
    assert_eq!(
        hf_sim::systems::society::arrival_tile(&w, open, 2, home),
        Some(open),
        "a tile with a way in is kept"
    );
}

#[test]
fn grass_carries_a_fire_only_in_dry_heat() {
    // #258: a lightning fire crossed forty tiles of summer ground overnight and burned three
    // houses; bare grass carried a fire in any weather.
    use hf_sim::systems::weather::Sky;
    let (mut w, _) = meadow(25801, 2);
    let p = TilePos::new(45, 45, 2);
    assert!(w.grid.get(p.offset(0, 0, -1)).material.is_vegetated());
    let mut cool = w.weather;
    cool.temperature_c = 12.0;
    cool.sky = Sky::Clear;
    w.weather = cool;
    assert_eq!(
        hf_sim::systems::hazards::fuel_at(&w, p),
        0,
        "cool grass is no fuel"
    );
    let mut hot = cool;
    hot.temperature_c = 28.0;
    w.weather = hot;
    assert!(
        hf_sim::systems::hazards::fuel_at(&w, p) > 0,
        "dry heat: grass burns"
    );
    hot.sky = Sky::Rain;
    w.weather = hot;
    assert_eq!(
        hf_sim::systems::hazards::fuel_at(&w, p),
        0,
        "not in the rain"
    );
}

#[test]
fn a_roof_begun_in_thatch_goes_on_in_planks_when_there_is_no_thatch() {
    // #257: an annex waited sixty-eight days for thatch no reeds within reach would give.
    use hf_sim::world::Material;
    let (mut w, cid) = meadow(25702, 6);
    w.meta.autoplan = true;
    let o = TilePos::new(20, 20, 2);
    w.build_house(o.x, o.y, 7, 6, 1, Material::WOOD, Material::SOIL);
    let mut plan = hf_sim::houses::plan(&w, o);
    for p in plan.floor.clone() {
        w.roofs.insert(
            p,
            hf_sim::building::Roof {
                material: Material::THATCH,
                quality: 0.6,
                built: 0,
                snow: 0.0,
            },
        );
    }
    // A room added east of the house, its walls up, no roof.
    let annex: Vec<TilePos> = (0..3)
        .flat_map(|dy| (0..2).map(move |dx| o.offset(7 + dx, 1 + dy, 0)))
        .collect();
    for (q, _) in hf_sim::houses::walls(&annex) {
        if !plan.floor.contains(&q) && w.grid.get(q).shape != hf_sim::world::Shape::Wall {
            w.grid.set(q, hf_sim::world::Tile::solid(Material::WOOD));
        }
    }
    plan.floor.extend(annex.iter().copied());
    plan.doors.push(o.offset(6, 2, 0));
    {
        let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
        c.house_sites = vec![o];
        c.house_stage = vec![3];
        c.house_plans = vec![plan];
    }
    w.plants.clear();
    w.items.retain(|it| {
        !matches!(
            it.kind,
            hf_sim::items::ItemKind::Thatch | hf_sim::items::ItemKind::Fibre
        )
    });
    w.rebuild_caches();
    w.ensure_nav();
    hf_sim::systems::settlement::step(&mut w);
    let roofs: Vec<Material> = w
        .designations
        .iter()
        .filter_map(|d| match d.kind {
            hf_sim::work::DesignationKind::BuildRoof(m) if annex.contains(&d.pos) => Some(m),
            _ => None,
        })
        .collect();
    assert!(!roofs.is_empty(), "the annex's roof is ordered");
    assert!(
        roofs.iter().all(|m| *m != Material::THATCH),
        "in planks, not thatch there is none of: {roofs:?}"
    );
}
