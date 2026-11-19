//! #439: someone cut off below is dug down to and tended. On seed 4 three people fell through a
//! burned floor into a closed cave seven levels down, lived there four weeks and died of a fever
//! nobody could come down to tend.

use hf_sim::body::{Illness, IllnessKind};
use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Tile, TileFlags};
use hf_sim::World;

/// A party on flat ground (they stand at 2) with one of them, sick, in a closed cave at -3:
/// open tiles x and y 30 to 34 with rock all round and above.
fn one_below(seed: u64) -> (World, usize, usize) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(24, 24, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    for y in 30..=34 {
        for x in 30..=34 {
            for z in -3..=-2 {
                w.grid.set(TilePos::new(x, y, z), Tile::AIR);
            }
        }
    }
    w.rebuild_caches();
    let ids = w.found_party(4);
    w.meta.autoplan = false;
    let idx: Vec<usize> = ids
        .iter()
        .map(|id| w.agents.iter().position(|a| a.id == *id).unwrap())
        .collect();
    let (sick, kin) = (idx[0], idx[1]);
    let (si, ki) = (w.agents[sick].id, w.agents[kin].id);
    w.agents[sick].partner = Some(ki);
    w.agents[kin].partner = Some(si);
    let down = TilePos::new(32, 32, -3);
    w.agents[sick].pos = down;
    w.agents[sick].from = down;
    w.agents[sick].path.clear();
    let t = w.meta.tick;
    w.agents[sick].body.illness = Some(Illness {
        severity: 0.5,
        since: t,
        kind: IllnessKind::Fever,
        treated: None,
    });
    w.ensure_nav();
    (w, sick, kin)
}

#[test]
fn their_people_dig_down_to_someone_cut_off_below_and_tend_them() {
    let (mut w, sick, _) = one_below(4391);
    let home = w.communities[0].home;
    assert!(
        !w.nav.as_ref().unwrap().reach(w.agents[sick].pos, home),
        "the cave is closed"
    );
    let si = w.agents[sick].id;
    let mut tended = false;
    for _ in 0..24 * 6 {
        for _ in 0..TICKS_PER_HOUR {
            for a in w.agents.iter_mut() {
                a.needs.hunger = a.needs.hunger.max(0.6);
            }
            w.step();
            tended |= w
                .last_events
                .iter()
                .any(|e| matches!(e.kind, EventKind::Tended { patient, .. } if patient == si));
        }
        if tended {
            break;
        }
    }
    let a = &w.agents[sick];
    assert!(a.alive, "{:?}", a.death);
    assert!(
        w.nav.as_ref().unwrap().reach(a.pos, home),
        "a way was dug down to them"
    );
    assert!(tended, "and they were tended");
}

#[test]
fn someone_left_on_a_ledge_a_storey_up_climbs_down() {
    // #447: on seed 3 a builder finishing a wall was put off its tile onto a ledge a level above
    // the ground round it, which the wall had been the only way on and off, and starved there.
    let mut w = World::generate(4471, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(24, 24, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    // A ledge three across standing at 3, with no way up or down.
    for y in 40..=42 {
        for x in 40..=42 {
            w.grid.set(
                TilePos::new(x, y, 2),
                Tile::solid(hf_sim::world::Material::SOIL),
            );
        }
    }
    w.rebuild_caches();
    let ids = w.found_party(2);
    w.meta.autoplan = false;
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let up = TilePos::new(41, 41, 3);
    w.agents[i].pos = up;
    w.agents[i].from = up;
    w.agents[i].path.clear();
    w.ensure_nav();
    let home = w.communities[0].home;
    assert!(
        !w.nav.as_ref().unwrap().reach(up, home),
        "the ledge is cut off"
    );
    let mut climbed = false;
    for _ in 0..2 * TICKS_PER_HOUR {
        w.step();
        climbed |= w
            .last_events
            .iter()
            .any(|e| matches!(&e.kind, EventKind::Chose { what, .. } if what == "climbed down"));
    }
    let a = &w.agents[i];
    assert!(climbed, "on record");
    assert_eq!(a.pos.z, 2, "down off the ledge");
    assert!(w.nav.as_ref().unwrap().reach(a.pos, home));
}

#[test]
fn whole_community_on_a_ledge_climbs_down_without_anyone_home() {
    let mut w = World::generate(5231, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(24, 24, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    for y in 40..=42 {
        for x in 40..=42 {
            w.grid.set(
                TilePos::new(x, y, 2),
                Tile::solid(hf_sim::world::Material::SOIL),
            );
        }
    }
    w.rebuild_caches();
    let ids = w.found_party(2);
    w.meta.autoplan = false;
    let people: Vec<usize> = ids
        .iter()
        .map(|id| w.agents.iter().position(|a| a.id == *id).unwrap())
        .collect();
    let up = TilePos::new(41, 41, 3);
    for &i in &people {
        w.agents[i].pos = up;
        w.agents[i].from = up;
        w.agents[i].path.clear();
    }
    w.ensure_nav();
    let home = w.communities[0].home;
    assert!(people
        .iter()
        .all(|&i| { !w.nav.as_ref().unwrap().reach(w.agents[i].pos, home) }));
    let mut climbed = 0;
    for _ in 0..2 * TICKS_PER_HOUR {
        w.step();
        climbed += w
            .last_events
            .iter()
            .filter(|e| matches!(&e.kind, EventKind::Chose { what, .. } if what == "climbed down"))
            .count();
    }
    assert_eq!(climbed, 2, "both founders climbed down");
    for &i in &people {
        let a = &w.agents[i];
        assert_eq!(a.pos.z, 2, "down off the ledge");
        assert!(w.nav.as_ref().unwrap().reach(a.pos, home));
    }
}

#[test]
fn whole_community_climbs_while_short_walks_cross_the_rescue_tick() {
    let mut w = World::generate(5232, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(24, 24, 2);
    w.meta.tick = TICKS_PER_HOUR * 8 + 899;
    for x in 40..=41 {
        w.grid.set(
            TilePos::new(x, 40, 2),
            Tile::solid(hf_sim::world::Material::SOIL),
        );
    }
    w.rebuild_caches();
    let ids = w.found_party(25);
    w.meta.autoplan = false;
    let people: Vec<usize> = ids
        .iter()
        .map(|id| w.agents.iter().position(|a| a.id == *id).unwrap())
        .collect();
    for (n, &i) in people.iter().enumerate() {
        let up = TilePos::new(40 + n as i32 % 2, 40, 3);
        let other = TilePos::new(41 - n as i32 % 2, 40, 3);
        w.agents[i].pos = up;
        w.agents[i].from = other;
        w.agents[i].path = vec![other];
        w.agents[i].move_total = 4;
        w.agents[i].move_ticks = 1;
        w.agents[i].activity = hf_sim::mind::Activity::Wandering { to: other };
    }
    w.ensure_nav();
    let home = w.communities[0].home;
    assert!(people.iter().all(|&i| {
        let a = &w.agents[i];
        a.is_moving() && !w.nav.as_ref().unwrap().reach(a.pos, home)
    }));
    w.step();
    let climbed: std::collections::BTreeSet<_> = w
        .last_events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Chose { agent, what, .. } if what == "climbed down" => Some(*agent),
            _ => None,
        })
        .collect();
    assert_eq!(
        climbed.len(),
        people.len(),
        "every distinct person climbed down"
    );
    for &i in &people {
        assert_eq!(w.agents[i].pos.z, 2, "down off the ledge");
        assert!(w.nav.as_ref().unwrap().reach(w.agents[i].pos, home));
    }
}

#[test]
fn a_builder_walling_the_way_off_a_ledge_steps_off_on_the_home_side() {
    // #447: the builder stood on the tile it walled, and when it stood was put off on the ledge
    // side, the first open tile in a fixed order.
    use hf_sim::world::{Material, Shape};
    let mut w = World::generate(4472, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(10, 10, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    let raise = |w: &mut World, x: i32, y: i32| {
        w.grid
            .set(TilePos::new(x, y, 2), Tile::solid(Material::SOIL));
    };
    // A plateau standing at 3, with a ramp up from the ground on its west side...
    for y in 35..=47 {
        for x in 30..=42 {
            raise(&mut w, x, y);
        }
    }
    w.grid.set(
        TilePos::new(29, 41, 2),
        Tile {
            material: Material::SOIL,
            shape: Shape::RampUp,
            flags: TileFlags::NONE,
        },
    );
    // ...joined by one tile to a ledge east of it.
    raise(&mut w, 43, 41);
    for y in 40..=42 {
        for x in 44..=46 {
            raise(&mut w, x, y);
        }
    }
    w.rebuild_caches();
    let ids = w.found_party(1);
    w.meta.autoplan = false;
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let way = TilePos::new(43, 41, 3);
    w.agents[i].pos = way;
    w.agents[i].from = way;
    w.ensure_nav();
    let home = w.communities[0].home;
    assert!(
        w.nav.as_ref().unwrap().reach(way, home),
        "the ledge is joined to home, by this tile"
    );
    hf_sim::systems::work::displace_from(&mut w, way);
    let at = w.agents[i].pos;
    assert_eq!(
        at,
        TilePos::new(42, 41, 3),
        "stepped off onto the plateau, the home side"
    );
}
