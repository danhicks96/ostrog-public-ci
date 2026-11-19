//! C57: a home is proposed by someone who slept on the ground, in the form their kind builds.

use hf_sim::ids::{CommunityId, StructureId, TilePos};
use hf_sim::mind::knowledge::Known;
use hf_sim::mind::{Activity, ThoughtKind};
use hf_sim::project::{HomeForm, ProjectKind};
use hf_sim::structures::{Structure, StructureKind};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::DesignationKind;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Tile};
use hf_sim::World;

fn valley(seed: u64, kind: Option<&str>) -> (World, CommunityId) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(14, 14, 2);
    w.meta.tick = TICKS_PER_HOUR * 7;
    w.rebuild_caches();
    let ids = w.found_party(6);
    if let Some(k) = kind {
        let sp = hf_sim::species::by_key(k).unwrap();
        for id in &ids {
            hf_sim::species::give(&mut w, *id, sp, None);
        }
    }
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
}

/// The day's proposals, then the planner's hour.
fn a_morning(w: &mut World) {
    let day = w.meta.tick / TICKS_PER_DAY;
    w.meta.tick = day * TICKS_PER_DAY + hf_sim::systems::projects::AT;
    hf_sim::systems::projects::step(w);
    for _ in 0..2 {
        w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
        hf_sim::systems::settlement::step(w);
    }
}

fn slept_rough(w: &mut World, cid: CommunityId) {
    let tick = w.meta.tick;
    for a in w.agents.iter_mut().filter(|a| a.community == Some(cid)) {
        a.thoughts.add(ThoughtKind::SleptRough, tick, None);
    }
}

fn walls(w: &World, cid: CommunityId) -> usize {
    w.designations
        .iter()
        .filter(|d| d.community == Some(cid) && matches!(d.kind, DesignationKind::BuildWall(_)))
        .count()
}

#[test]
fn nobody_on_the_ground_no_house() {
    // Six people and no beds, but nobody has yet slept on the ground: before C57 the planner
    // marked out a house at once by its own rule.
    let (mut w, cid) = valley(5701, None);
    a_morning(&mut w);
    assert_eq!(walls(&w, cid), 0, "no house without someone who wants it");
    assert!(!w
        .projects
        .iter()
        .any(|p| matches!(p.kind, ProjectKind::Home(_))));
}

#[test]
fn a_home_has_a_proposer_and_a_cause() {
    let (mut w, cid) = valley(5702, None);
    slept_rough(&mut w, cid);
    a_morning(&mut w);
    let p = w
        .projects
        .iter()
        .find(|p| matches!(p.kind, ProjectKind::Home(_)))
        .expect("someone proposes a home");
    assert_eq!(
        p.kind,
        ProjectKind::Home(HomeForm::House),
        "humans build houses"
    );
    assert!(p.because.contains("slept on the ground"), "{}", p.because);
    assert!(w
        .agents
        .iter()
        .any(|a| a.id == p.by && a.community == Some(cid)));
    assert!(walls(&w, cid) > 0, "the planner marks the house out");
    assert!(p.site.is_some());
}

#[test]
fn a_hollow_folk_digs_a_hollow() {
    let (mut w, cid) = valley(5703, Some("grisk"));
    // A hill of granite east of home, two levels high.
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    for z in home.z..=home.z + 1 {
        for y in home.y - 6..home.y + 6 {
            for x in home.x + 8..home.x + 16 {
                w.grid
                    .set(TilePos::new(x, y, z), Tile::solid(Material::GRANITE));
            }
        }
    }
    w.rebuild_caches();
    w.ensure_nav();
    slept_rough(&mut w, cid);
    a_morning(&mut w);
    let p = w
        .projects
        .iter()
        .find(|p| matches!(p.kind, ProjectKind::Home(_)))
        .expect("someone proposes a home");
    assert_eq!(p.kind, ProjectKind::Home(HomeForm::Hollow));
    let mines = w
        .designations
        .iter()
        .filter(|d| d.kind == DesignationKind::Mine)
        .count();
    assert!(
        mines >= 10,
        "the mouth and the room are marked to dig: {mines}"
    );
    assert_eq!(walls(&w, cid), 0, "no house walls");
    assert_eq!(
        w.communities
            .iter()
            .find(|c| c.id == cid)
            .unwrap()
            .hollows
            .len(),
        1
    );
}

#[test]
fn a_hollow_folk_without_rock_digs_down_and_says_why() {
    // C67: on flat ground with no rock face they dug no home and built a house instead.
    let (mut w, cid) = valley(5704, Some("grisk"));
    slept_rough(&mut w, cid);
    a_morning(&mut w);
    let p = w
        .projects
        .iter()
        .find(|p| matches!(p.kind, ProjectKind::Home(_)))
        .expect("someone proposes a home");
    assert_eq!(p.kind, ProjectKind::Home(HomeForm::Hollow));
    assert!(p.because.contains("dug down"), "{}", p.because);
    let h = w.communities.iter().find(|c| c.id == cid).unwrap().hollows[0].clone();
    assert!(h.dug_down);
    // Six of them, no beds: a room that sleeps six, as big as that needs and no bigger.
    let gap = hf_sim::houses::bed_gap(&w, cid);
    assert!(hf_sim::houses::room_sleeps(&h.room[1..], h.room[0], 6, gap));
    assert!(h.room.len() <= 1 + 4 * 6, "{} tiles", h.room.len());
    assert_eq!(
        h.room[0],
        h.mouth.offset(0, 0, -1),
        "the shaft goes straight down"
    );
    let stairs = w
        .designations
        .iter()
        .filter(|d| {
            d.kind == DesignationKind::DigStairs && (d.pos == h.mouth || d.pos == h.room[0])
        })
        .count();
    assert_eq!(stairs, 2, "the shaft is dug as stairs");
    assert_eq!(walls(&w, cid), 0, "no house walls");
    // Dug out, it is their home without a door.
    for p in std::iter::once(h.mouth).chain(h.room.iter().copied()) {
        w.grid.set(p, Tile::AIR);
    }
    assert!(hf_sim::systems::settlement::home_built(
        &w,
        cid,
        HomeForm::Hollow,
        h.mouth
    ));
}

#[test]
fn a_hollow_folk_with_no_ground_to_dig_builds_a_house_and_says_why() {
    let (mut w, cid) = valley(5704, Some("grisk"));
    // Nothing a room can be dug in: an open void under the whole valley floor.
    for y in 2..62 {
        for x in 2..62 {
            w.grid.set(TilePos::new(x, y, 0), Tile::AIR);
        }
    }
    w.rebuild_caches();
    w.ensure_nav();
    slept_rough(&mut w, cid);
    a_morning(&mut w);
    let p = w
        .projects
        .iter()
        .find(|p| matches!(p.kind, ProjectKind::Home(_)))
        .expect("someone proposes a home");
    assert_eq!(p.kind, ProjectKind::Home(HomeForm::House));
    assert!(p.because.contains("no rock face"), "{}", p.because);
}

#[test]
fn a_tent_folk_raise_tents() {
    let (mut w, cid) = valley(5705, Some("gnarrow"));
    slept_rough(&mut w, cid);
    a_morning(&mut w);
    let tents = w
        .designations
        .iter()
        .filter(|d| {
            matches!(
                d.kind,
                DesignationKind::BuildStructure(StructureKind::Tent, _)
            )
        })
        .count();
    assert!(tents >= 1, "tents are posted");
    assert_eq!(walls(&w, cid), 0, "no house walls");
}

#[test]
fn walls_lost_to_fire_are_built_again() {
    let (mut w, cid) = valley(5706, None);
    slept_rough(&mut w, cid);
    a_morning(&mut w);
    let origin = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .unwrap()
        .house_sites[0];
    // The walls stand, then a fire takes four of them and nothing is left ordered there.
    let plan = hf_sim::houses::plan(&w, origin);
    let ring = hf_sim::houses::walls(&plan.floor);
    for (q, _) in &ring {
        w.grid.set(*q, Tile::solid(Material::WOOD));
    }
    w.designations.retain(|d| {
        !matches!(
            d.kind,
            DesignationKind::BuildWall(_) | DesignationKind::BuildDoor
        )
    });
    let lost: Vec<TilePos> = ring
        .iter()
        .map(|(q, _)| *q)
        .filter(|q| !plan.doors.contains(q))
        .take(4)
        .collect();
    for q in &lost {
        w.grid.set(*q, Tile::AIR);
    }
    w.rebuild_caches();
    w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(&mut w);
    for q in &lost {
        assert!(
            w.designations
                .iter()
                .any(|d| d.pos == *q && matches!(d.kind, DesignationKind::BuildWall(_))),
            "the wall at {q:?} is ordered again"
        );
    }
}

#[test]
fn a_wooded_house_site_keeps_a_way_round_its_neighbours() {
    let (mut w, _) = valley(5707, None);
    // A standing house with its door to the south.
    let o = TilePos::new(30, 30, 2);
    for y in 0..6 {
        for x in 0..7 {
            if x == 0 || y == 0 || x == 6 || y == 5 {
                w.grid.set(o.offset(x, y, 0), Tile::solid(Material::WOOD));
            }
        }
    }
    w.rebuild_caches();
    w.ensure_nav();
    // Look for wooded ground right in front of its door.
    let site = hf_sim::systems::settlement::find_flat_area_with_trees(&w, o.offset(0, 6, 0), 7, 6)
        .expect("a site");
    for y in site.y - 1..site.y + 7 {
        for x in site.x - 1..site.x + 8 {
            let t = w.grid.get(TilePos::new(x, y, site.z));
            assert_ne!(
                t.shape,
                hf_sim::world::Shape::Wall,
                "a wall at {x},{y} touches the new site {site:?}"
            );
        }
    }
}

#[test]
fn a_hollow_left_undug_is_ordered_again_and_given_up_in_time() {
    // #163: a hollow's dropped orders were never posted again, and its home blocked every
    // later home for that community.
    let (mut w, cid) = valley(5708, Some("grisk"));
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    for z in home.z..=home.z + 1 {
        for y in home.y - 6..home.y + 6 {
            for x in home.x + 8..home.x + 16 {
                w.grid
                    .set(TilePos::new(x, y, z), Tile::solid(Material::GRANITE));
            }
        }
    }
    w.rebuild_caches();
    w.ensure_nav();
    slept_rough(&mut w, cid);
    a_morning(&mut w);
    let h = w.communities.iter().find(|c| c.id == cid).unwrap().hollows[0].clone();
    let first = w
        .projects
        .iter()
        .find(|p| matches!(p.kind, ProjectKind::Home(_)))
        .unwrap()
        .id;
    // The orders are dropped.
    w.designations.retain(|d| d.kind != DesignationKind::Mine);
    w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(&mut w);
    let mines = w
        .designations
        .iter()
        .filter(|d| {
            d.kind == DesignationKind::Mine && (d.pos == h.mouth || h.room.contains(&d.pos))
        })
        .count();
    assert_eq!(mines, 1 + h.room.len(), "every undug tile is ordered again");
    // Twenty days on and still undug: given up, and the home it was for ends.
    w.meta.tick += 21 * TICKS_PER_DAY;
    w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(&mut w);
    assert!(w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .unwrap()
        .hollows
        .is_empty());
    a_morning(&mut w);
    assert!(
        !w.projects.iter().any(|p| p.id == first && !p.done),
        "the home it was for has ended"
    );
}

/// #1081: a person's own lean-to is a sleeping place, and its own tile counts as roofed, so
/// knowing of one is enough to be secured for the night even standing in the open.
#[test]
fn a_lean_to_is_a_sleeping_place_with_a_roof_over_its_tile() {
    assert!(StructureKind::LeanTo.is_sleeping_place());

    let (mut w, cid) = valley(10810, None);
    let pos = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    w.structures.push(Structure {
        id: StructureId(9001),
        kind: StructureKind::LeanTo,
        pos,
        material: Material::WOOD,
        progress: 1.0,
        active: false,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
    let tick = w.meta.tick;
    let i = w
        .agents
        .iter()
        .position(|a| a.community == Some(cid))
        .unwrap();
    w.agents[i].pos = pos;
    w.agents[i].knowledge.note(
        Known::Structure(StructureId(9001)),
        pos,
        tick,
        StructureKind::LeanTo as u16,
    );
    // Night is near, and the open ground under them is no roof, yet they are secured.
    w.meta.tick = (tick / TICKS_PER_DAY) * TICKS_PER_DAY + TICKS_PER_DAY - 1;
    let secured = hf_sim::systems::decide::secured(&w, &w.agents[i], None);
    assert!(
        !secured.roofless,
        "a person who knows of their own finished lean-to is not roofless for the night"
    );
}

/// #1081 correction (Dan, 2026-11-14): the sleep choice hard-coded `Bed | Tent | Nest` and
/// missed the lean-to, so a tired person with a finished lean-to in sight never slept in it.
#[test]
fn a_lean_to_in_sight_is_slept_in() {
    let (mut w, cid) = valley(10811, None);
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    let lean_to_pos = home.offset(5, 0, 0);
    let lean_to_id = StructureId(9002);
    w.structures.push(Structure {
        id: lean_to_id,
        kind: StructureKind::LeanTo,
        pos: lean_to_pos,
        material: Material::WOOD,
        progress: 1.0,
        active: false,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
    w.rebuild_caches();
    w.ensure_nav();

    let i = w
        .agents
        .iter()
        .position(|a| a.community == Some(cid))
        .unwrap();
    let tick = w.meta.tick;
    {
        let a = &mut w.agents[i];
        a.knowledge.note(
            Known::Structure(lean_to_id),
            lean_to_pos,
            tick,
            StructureKind::LeanTo as u16,
        );
        // Tired, no bed known, night close, idle: ready to choose where to sleep.
        a.needs.rest = 0.05;
        a.needs.hunger = 1.0;
        a.needs.thirst = 1.0;
        a.activity = Activity::Idle;
        a.activity_ticks = 0;
    }
    w.meta.tick = (tick / TICKS_PER_DAY) * TICKS_PER_DAY + TICKS_PER_DAY - 1;

    hf_sim::systems::decide::step(&mut w);
    match &w.agents[i].activity {
        Activity::Going { then, .. } => assert!(
            matches!(**then, Activity::Sleeping { bed: Some(id), .. } if id == lean_to_id),
            "chooses to sleep in the lean-to, not {:?}",
            then
        ),
        Activity::Sleeping { bed: Some(id), .. } => assert_eq!(*id, lean_to_id),
        other => panic!("expected the lean-to to be chosen as the sleep goal, got {other:?}"),
    }

    // Run the world on until they have slept in it and pushed the event.
    let mut slept_in_bed = None;
    for _ in 0..TICKS_PER_DAY {
        w.step();
        for e in &w.last_events {
            if let hf_sim::events::EventKind::Slept { agent, in_bed, .. } = &e.kind {
                if *agent == w.agents[i].id {
                    slept_in_bed = Some(*in_bed);
                }
            }
        }
        if slept_in_bed.is_some() {
            break;
        }
    }
    assert_eq!(
        slept_in_bed,
        Some(true),
        "a tired person with a finished lean-to in sight sleeps in it, not on the ground"
    );
}
