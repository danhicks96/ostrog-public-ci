//! C67: a house is grown from what its household needs, a tile of floor at a time over the
//! ground there is, and walled round whatever floor was claimed; not set down from a template.

use std::collections::BTreeSet;

use hf_sim::houses::{layout, walls};
use hf_sim::ids::{CommunityId, TilePos};
use hf_sim::mind::ThoughtKind;
use hf_sim::structures::StructureKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::DesignationKind;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Tile};
use hf_sim::World;

fn valley(seed: u64, party: usize) -> (World, CommunityId) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(14, 14, 2);
    w.meta.tick = TICKS_PER_HOUR * 7;
    w.rebuild_caches();
    let ids = w.found_party(party);
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

fn a_morning(w: &mut World, cid: CommunityId) {
    let tick = w.meta.tick;
    for a in w.agents.iter_mut().filter(|a| a.community == Some(cid)) {
        a.thoughts.add(ThoughtKind::SleptRough, tick, None);
    }
    let day = w.meta.tick / TICKS_PER_DAY;
    w.meta.tick = day * TICKS_PER_DAY + hf_sim::systems::projects::AT;
    hf_sim::systems::projects::step(w);
    for _ in 0..2 {
        w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
        hf_sim::systems::settlement::step(w);
    }
}

fn wall_orders(w: &World) -> BTreeSet<TilePos> {
    w.designations
        .iter()
        .filter(|d| {
            matches!(
                d.kind,
                DesignationKind::BuildWall(_) | DesignationKind::BuildDoor
            )
        })
        .map(|d| d.pos)
        .collect()
}

/// The walls stand and the roof is on.
fn finish(w: &mut World, cid: CommunityId) {
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    let plan = hf_sim::houses::plan(w, *c.house_sites.last().unwrap());
    for (q, _) in walls(&plan.floor) {
        if w.grid.get(q).shape != hf_sim::world::Shape::Wall {
            w.grid.set(q, Tile::solid(Material::WOOD));
        }
    }
    for p in &plan.floor {
        w.roofs.insert(
            *p,
            hf_sim::building::Roof {
                material: Material::THATCH,
                quality: 0.6,
                built: 0,
                snow: 0.0,
            },
        );
    }
    w.designations.clear();
    w.rebuild_caches();
    w.ensure_nav();
}

fn next_hour(w: &mut World) {
    w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(w);
}

fn beds_ordered(w: &World, floor: &[TilePos]) -> usize {
    w.designations
        .iter()
        .filter(|d| {
            matches!(
                d.kind,
                DesignationKind::BuildStructure(StructureKind::Bed, _)
            ) && floor.contains(&d.pos)
        })
        .count()
}

#[test]
fn a_house_is_grown_to_its_household() {
    // Two people and eight: before C67 both got the same 7×6.
    let (mut small, sc) = valley(6701, 2);
    a_morning(&mut small, sc);
    let (mut big, bc) = valley(6702, 8);
    a_morning(&mut big, bc);
    let plan_of = |w: &World, cid: CommunityId| {
        let c = w.communities.iter().find(|c| c.id == cid).unwrap();
        assert_eq!(c.house_sites.len(), 1);
        hf_sim::houses::plan(w, c.house_sites[0])
    };
    let (ps, pb) = (plan_of(&small, sc), plan_of(&big, bc));
    let gap = 2;
    assert!(layout(&ps, gap).beds.len() >= 2);
    assert!(layout(&pb, gap).beds.len() >= 8);
    assert!(
        ps.floor.len() + 8 <= pb.floor.len(),
        "two get a smaller floor than eight: {} and {}",
        ps.floor.len(),
        pb.floor.len()
    );
    // The walls ordered are the line round the floor claimed, and one door, facing home.
    for (w, p) in [(&small, &ps), (&big, &pb)] {
        let ring: BTreeSet<TilePos> = walls(&p.floor).into_iter().map(|(q, _)| q).collect();
        assert_eq!(wall_orders(w), ring);
        assert_eq!(p.doors.len(), 1);
        assert!(ring.contains(&p.doors[0]));
    }
}

#[test]
fn a_big_folk_needs_more_floor() {
    let (w, _) = valley(6703, 2);
    let home = w.communities[0].home;
    let near = home.offset(-7, -5, 0);
    let most = hf_sim::houses::new_house(&w, near, home, 6, 2).unwrap();
    let big = hf_sim::houses::new_house(&w, near, home, 6, 3).unwrap();
    assert!(layout(&big, 3).beds.len() >= 6);
    assert!(
        big.floor.len() > most.floor.len(),
        "{} {}",
        big.floor.len(),
        most.floor.len()
    );
}

#[test]
fn the_floor_keeps_off_what_stands_in_the_way() {
    // A boulder where the house would go: the floor grows round it, with a way between.
    let (mut w, cid) = valley(6713, 6);
    let home = w.communities[0].home;
    let near = home.offset(-7, -5, 0);
    let first = hf_sim::houses::new_house(&w, near, home, 6, 2).unwrap();
    let rock = hf_sim::houses::heart(&first.floor).unwrap();
    w.grid.set(rock, Tile::solid(Material::GRANITE));
    w.rebuild_caches();
    w.ensure_nav();
    a_morning(&mut w, cid);
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    let plan = hf_sim::houses::plan(&w, c.house_sites[0]);
    for p in plan
        .floor
        .iter()
        .chain(walls(&plan.floor).iter().map(|(q, _)| q))
    {
        assert!(
            (p.x - rock.x).abs().max((p.y - rock.y).abs()) >= 2,
            "{p:?} by the rock at {rock:?}"
        );
    }
}

#[test]
fn a_household_that_outgrows_its_house_adds_to_it() {
    let (mut w, cid) = valley(6705, 3);
    a_morning(&mut w, cid);
    finish(&mut w, cid);
    let origin = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .unwrap()
        .house_sites[0];
    let before = hf_sim::houses::plan(&w, origin);
    let old_walls: BTreeSet<TilePos> = walls(&before.floor).into_iter().map(|(q, _)| q).collect();
    // Its beds are made; then five more come to live with them.
    for p in layout(&before, 2).beds {
        let id = w.alloc_structure_id();
        w.structures.push(hf_sim::structures::Structure {
            id,
            kind: StructureKind::Bed,
            pos: p,
            material: Material::WOOD,
            progress: 1.0,
            active: false,
            condition: 1.0,
            owner: None,
            quality: 0.5,
        });
    }
    let more = w.found_party(5);
    for id in more {
        let a = w.agents.iter_mut().find(|a| a.id == id).unwrap();
        a.community = Some(cid);
        w.communities
            .iter_mut()
            .find(|c| c.id == cid)
            .unwrap()
            .members
            .push(id);
    }
    // The house is seen to be roofed, its home is done, and they want more room.
    next_hour(&mut w);
    a_morning(&mut w, cid);
    a_morning(&mut w, cid);
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    assert_eq!(
        c.house_sites.len(),
        1,
        "no second house: the first is added to"
    );
    let after = hf_sim::houses::plan(&w, origin);
    assert!(after.floor.len() > before.floor.len());
    assert!(before.floor.iter().all(|p| after.floor.contains(p)));
    // A door through the old wall between the old floor and the new.
    let between: Vec<TilePos> = after
        .doors
        .iter()
        .copied()
        .filter(|d| old_walls.contains(d) && !before.doors.contains(d))
        .collect();
    assert_eq!(between.len(), 1, "{:?}", after.doors);
    assert!(w
        .designations
        .iter()
        .any(|d| d.kind == DesignationKind::BuildDoor && d.pos == between[0]));
    // Built, the new floor takes beds.
    finish(&mut w, cid);
    let new: Vec<TilePos> = after
        .floor
        .iter()
        .copied()
        .filter(|p| !before.floor.contains(p))
        .collect();
    next_hour(&mut w);
    assert!(beds_ordered(&w, &new) >= 1);
}

#[test]
fn a_lamp_by_the_door_even_when_the_flanks_are_taken() {
    // #255 part 2: rooms added against the door side stand on the two tiles flanking the
    // door step, and the lamp there was skipped for good. A lamp belongs by the door: the
    // planner designates the nearest other open tile instead.
    let (mut w, cid) = valley(6706, 6);
    a_morning(&mut w, cid);
    finish(&mut w, cid);
    // The village has kept up with its other building (a store, a bed each, its hearth), so
    // the house-furnishing step is not held back by open build orders elsewhere.
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    for i in 0..4 {
        w.place_structure(
            StructureKind::Stockpile,
            home.offset(2 + i, 0, 0),
            Material::SOIL,
            true,
        );
    }
    for i in 0..6 {
        w.place_structure(
            StructureKind::Bed,
            home.offset(4 + i, 2, 0),
            Material::WOOD,
            false,
        );
    }
    w.place_structure(
        StructureKind::Hearth,
        home.offset(-2, 2, 0),
        Material::STONE_BLOCK,
        true,
    );
    let origin = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .unwrap()
        .house_sites[0];
    let plan = hf_sim::houses::plan(&w, origin);
    let door = plan.doors[0];
    let (ox, oy) = [(0, 1), (0, -1), (1, 0), (-1, 0)]
        .into_iter()
        .find(|(dx, dy)| plan.floor.contains(&door.offset(-dx, -dy, 0)))
        .unwrap();
    let step = door.offset(ox, oy, 0);
    let (px, py) = (-oy, ox);
    let flanks = [step.offset(-px, -py, 0), step.offset(px, py, 0)];
    // Before anything stands in the way, this house would get its lamps (its flanks are
    // walkable open ground or the wall of the ring, as the grown plan leaves them).
    assert!(
        flanks
            .iter()
            .any(|p| w.grid.walkable(*p) && w.grid.get(*p).shape == hf_sim::world::Shape::Open),
        "a flank at {door:?} was a lamp spot before the rooms came"
    );
    // A room against the door side stands on both flanking tiles.
    for p in flanks {
        w.grid.set(p, Tile::solid(Material::GRANITE));
    }
    w.rebuild_caches();
    w.ensure_nav();
    next_hour(&mut w);
    let near: Vec<TilePos> = w
        .designations
        .iter()
        .filter(|d| {
            matches!(
                d.kind,
                DesignationKind::BuildStructure(StructureKind::Lamp, _)
            ) && d.pos.dist(door) <= 3
        })
        .map(|d| d.pos)
        .chain(
            w.structures
                .iter()
                .filter(|s| s.kind == StructureKind::Lamp && s.pos.dist(door) <= 3)
                .map(|s| s.pos),
        )
        .collect();
    assert!(!near.is_empty(), "no lamp within 3 of the door at {door}");
}
