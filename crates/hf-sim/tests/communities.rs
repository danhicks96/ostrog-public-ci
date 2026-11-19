//! M4 review gates: a community plans with what it holds, not with what the whole valley holds.
//! After a split, each community builds its own hearth and beds and counts its own stores.

use hf_sim::ids::TilePos;
use hf_sim::structures::StructureKind;
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::work::DesignationKind;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

#[test]
fn a_community_that_split_off_builds_its_own_hearth() {
    let mut w = World::generate(4701, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(14, 14, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    let ids = w.found_party(6);
    let first = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    let home = w.communities.iter().find(|c| c.id == first).unwrap().home;
    // The first community has its hearth.
    let at = home.offset(-3, -3, 0);
    let id = w.alloc_structure_id();
    w.structures.push(hf_sim::structures::Structure {
        id,
        kind: StructureKind::Hearth,
        pos: at,
        material: hf_sim::world::Material::STONE_BLOCK,
        progress: 1.0,
        active: true,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
    // Three of them go off to found their own across the valley.
    let theirs = TilePos::new(50, 50, home.z);
    let second = w.found_community(theirs);
    for id in &ids[3..] {
        w.join_community(*id, second);
        if let Some(a) = w.agents.iter_mut().find(|a| a.id == *id) {
            a.pos = theirs;
        }
    }
    w.meta.autoplan = true;
    w.ensure_nav();
    for _ in 0..2 {
        w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
        hf_sim::systems::settlement::step(&mut w);
    }
    assert!(
        w.designations.iter().any(|d| d.community == Some(second)
            && matches!(
                d.kind,
                DesignationKind::BuildStructure(StructureKind::Hearth, _)
            )),
        "the new community plans its own hearth"
    );
    assert!(
        !w.designations.iter().any(|d| d.community == Some(first)
            && matches!(
                d.kind,
                DesignationKind::BuildStructure(StructureKind::Hearth, _)
            )),
        "the first already has one"
    );
}

#[test]
fn a_house_is_built_of_stone_only_from_stone_the_community_holds() {
    let mut w = World::generate(4701, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(14, 14, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    let ids = w.found_party(6);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    // Thirty blocks lie on the far side of the valley, beyond what this community holds.
    let far = TilePos::new(home.x + 46, home.y + 30, home.z);
    w.create_item_of(
        hf_sim::items::ItemKind::StoneBlock,
        hf_sim::world::Material::GRANITE,
        30,
        hf_sim::items::ItemPlace::Ground(far),
        0.6,
        "test",
    );
    w.meta.autoplan = true;
    w.ensure_nav();
    // Someone wants a house (C57): they slept on the ground.
    let tick = w.meta.tick;
    for a in w.agents.iter_mut() {
        a.thoughts
            .add(hf_sim::mind::ThoughtKind::SleptRough, tick, None);
    }
    let day = w.meta.tick / hf_sim::time::TICKS_PER_DAY;
    w.meta.tick = day * hf_sim::time::TICKS_PER_DAY + hf_sim::systems::projects::AT;
    hf_sim::systems::projects::step(&mut w);
    for _ in 0..2 {
        w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
        hf_sim::systems::settlement::step(&mut w);
    }
    let walls: Vec<_> = w
        .designations
        .iter()
        .filter_map(|d| match d.kind {
            DesignationKind::BuildWall(m) if d.community == Some(cid) => Some(m),
            _ => None,
        })
        .collect();
    assert!(!walls.is_empty(), "the community plans a house");
    assert!(
        walls.iter().all(|m| *m == hf_sim::world::Material::WOOD),
        "stone it does not hold does not make its house stone: {walls:?}"
    );
}

#[test]
fn a_workshop_is_built_on_the_communitys_own_ground() {
    // C60: a community planning from a house that stands nearer another community's home
    // put its workshop there, and wanted it again every day.
    let mut w = World::generate(6010, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(14, 14, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    let ids = w.found_party(10);
    let first = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    let home = w.communities.iter().find(|c| c.id == first).unwrap().home;
    // A neighbour's home across the valley, and this community's only house beside it.
    let other = TilePos::new(48, 48, home.z);
    w.found_community(other);
    let o = TilePos::new(44, 40, home.z);
    w.build_house(
        o.x,
        o.y,
        7,
        6,
        1,
        hf_sim::world::Material::WOOD,
        hf_sim::world::Material::WOOD,
    );
    w.communities
        .iter_mut()
        .find(|c| c.id == first)
        .unwrap()
        .house_sites
        .push(o);
    w.rebuild_caches();
    w.ensure_nav();
    let id = w.next_project_id;
    w.next_project_id += 1;
    w.projects.push(hf_sim::project::Project {
        id,
        by: ids[0],
        community: first,
        kind: hf_sim::project::ProjectKind::Workshop(StructureKind::CarpenterBench),
        because: "test".into(),
        source: None,
        proposed: w.meta.tick,
        work: None,
        done: false,
        site: None,
    });
    w.meta.autoplan = true;
    for _ in 0..2 {
        w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
        hf_sim::systems::settlement::step(&mut w);
    }
    for d in w.designations.iter().filter(|d| {
        matches!(
            d.kind,
            DesignationKind::BuildStructure(StructureKind::CarpenterBench, _)
        )
    }) {
        assert!(
            d.pos.dist(home) <= d.pos.dist(other),
            "a bench at {:?}, nearer the neighbour's home {other:?} than its own {home:?}",
            d.pos
        );
    }
}
