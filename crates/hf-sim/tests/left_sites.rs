//! C72: a moved community's old site stands as a ruin and weathers; who comes on it wonders;
//! a community that moves carries what it can.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

fn big_valley(seed: u64) -> (World, hf_sim::ids::CommunityId) {
    let mut w = World::generate(seed, GenParams::default());
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
}

/// A standing house at the old home, and the hard times that move them.
fn ready_to_move(w: &mut World, cid: hf_sim::ids::CommunityId) -> TilePos {
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    let o = home.offset(-10, -8, 0);
    w.build_house(o.x, o.y, 7, 6, 1, Material::WOOD, Material::SOIL);
    let tick = w.meta.tick;
    {
        let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
        c.house_sites.push(o);
        c.house_stage.push(3);
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
    w.rebuild_caches();
    w.ensure_nav();
    home
}

#[test]
fn the_site_a_community_left_is_recorded_and_stands_as_a_ruin() {
    let (mut w, cid) = big_valley(7201);
    let home = ready_to_move(&mut w, cid);
    hf_sim::systems::moving::consider(&mut w, cid);
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    assert!(c.home.dist(home) >= 20, "they moved");
    assert_eq!(w.left_sites.len(), 1);
    let s = &w.left_sites[0];
    assert_eq!(s.home, home);
    assert_eq!(s.people, 8);
    assert!(s.walls.len() >= 20, "{} walls", s.walls.len());
    let ruins = hf_sim::deep::ruins(&w);
    assert!(
        s.walls.iter().all(|p| ruins.contains(&(*p, true))),
        "its walls are drawn as a ruin standing"
    );
}

#[test]
fn the_walls_nobody_keeps_fall_with_the_weather() {
    let (mut w, cid) = big_valley(7202);
    ready_to_move(&mut w, cid);
    hf_sim::systems::moving::consider(&mut w, cid);
    assert_eq!(w.left_sites.len(), 1);
    let walls = w.left_sites[0].walls.clone();
    for _ in 0..1500 {
        w.meta.tick += TICKS_PER_DAY;
        hf_sim::left_sites::decay(&mut w);
    }
    let standing = walls
        .iter()
        .filter(|p| w.grid.get(**p).shape == hf_sim::world::Shape::Wall)
        .count();
    assert!(
        standing < walls.len(),
        "after four years some of the timber walls have fallen: {standing} of {}",
        walls.len()
    );
    let ruins = hf_sim::deep::ruins(&w);
    assert!(
        ruins.iter().any(|(_, up)| !*up),
        "where a wall fell, its stones lie"
    );
}

#[test]
fn a_newcomer_among_the_left_houses_wonders_at_them() {
    let (mut w, cid) = big_valley(7203);
    let home = ready_to_move(&mut w, cid);
    hf_sim::systems::moving::consider(&mut w, cid);
    assert_eq!(w.left_sites.len(), 1);
    // A worn path leads away from the houses (#1816): what a stranger can actually read.
    let s = &w.left_sites[0];
    let out = s.home.offset(12, 0, 0);
    w.wear.insert((out.x, out.y), 90);
    // Someone who joined after the move, standing among the old houses.
    let id = w.spawn_agent(hf_sim::agents::Species::Person, home.offset(-7, -5, 0));
    w.join_community(id, cid);
    {
        let a = w.agents.iter_mut().find(|a| a.id == id).unwrap();
        a.came = w.meta.tick + 1;
        a.needs.rest = 1.0;
        a.traits = hf_sim::mind::Traits::default();
    }
    w.meta.tick += TICKS_PER_DAY;
    w.events.drain();
    hf_sim::left_sites::wonder(&mut w);
    let evs = w.events.drain();
    let wondered = evs
        .iter()
        .find(|e| matches!(&e.kind, EventKind::RuinWondered { agent, .. } if *agent == id))
        .expect("the newcomer wonders at the houses");
    let line = hf_sim::view::narrate::line(&w, wondered);
    assert!(line.contains("went to"), "{line}");
    // The tale says where they went, not the community's own name again (#358: "the last of
    // Eldergate went to Eldergate").
    let EventKind::RuinWondered { site, guess, .. } = &wondered.kind else {
        unreachable!()
    };
    assert!(
        !guess.ends_with(&format!("went to {site}")),
        "the tale names a place, not the site again: {guess}"
    );
    // One who left with the community does not: they know.
    assert!(!evs
        .iter()
        .any(|e| matches!(&e.kind, EventKind::RuinWondered { agent, .. } if *agent != id)));
}

#[test]
fn a_ruin_is_read_from_what_lies_there() {
    // #1816, D41: no more omniscience. A stranger reads graves as sickness, scorch as fire, a
    // worn path as the way out, and failing all of those, says nobody can tell.
    let (mut w, cid) = big_valley(7301);
    let home = ready_to_move(&mut w, cid);
    hf_sim::systems::moving::consider(&mut w, cid);
    assert_eq!(w.left_sites.len(), 1);
    let site_id = w.left_sites[0].id;
    let floor = w.left_sites[0].floors[0];

    let stranger_reads = |w: &mut World, site_id: u32| -> hf_sim::lore::Claim {
        let id = w.spawn_agent(hf_sim::agents::Species::Person, home.offset(-7, -5, 0));
        w.join_community(id, cid);
        {
            let a = w.agents.iter_mut().find(|a| a.id == id).unwrap();
            a.came = w.meta.tick + 1;
            a.needs.rest = 1.0;
            a.traits = hf_sim::mind::Traits::default();
        }
        w.meta.tick += TICKS_PER_DAY;
        hf_sim::left_sites::wonder(w);
        let a = w.agents.iter().find(|a| a.id == id).unwrap();
        a.tales
            .iter()
            .find(|t| t.site == site_id)
            .unwrap()
            .claim
            .clone()
    };

    // Three graves among the houses: read as sickness.
    for k in 0..3 {
        w.graves.push(hf_sim::accounts::Grave {
            pos: floor.offset(k, 0, 0),
            dead: hf_sim::ids::AgentId(900 + k as u32),
            death: hf_sim::ids::EventId(0),
            rite: hf_sim::ids::EventId(0),
            laid: w.meta.tick,
            words: None,
        });
    }
    assert_eq!(stranger_reads(&mut w, site_id), hf_sim::lore::Claim::Fever);
}

#[test]
fn a_moving_community_carries_food_and_sets_it_down_at_the_new_camp() {
    let (mut w, cid) = big_valley(7204);
    let home = ready_to_move(&mut w, cid);
    w.items.retain(|it| !it.kind.is_food());
    for k in 0..6 {
        w.create_item(
            ItemKind::GrainSheaf,
            9,
            ItemPlace::Ground(home.offset(2 + k, 2, 0)),
            0.6,
            "test",
        );
    }
    hf_sim::systems::moving::consider(&mut w, cid);
    let members: Vec<_> = w
        .agents
        .iter()
        .filter(|a| a.community == Some(cid))
        .map(|a| a.id)
        .collect();
    let carried: u32 = w
        .items
        .iter()
        .filter(|it| matches!(it.place, ItemPlace::Carried(m) if members.contains(&m)))
        .map(|it| it.count)
        .sum();
    assert!(carried >= 16, "a stack a head goes with them: {carried}");
    let evs = w.events.drain();
    assert!(evs.iter().any(
        |e| matches!(&e.kind, EventKind::Chose { what, .. } if what.contains("for the road"))
    ));
    // They walk; at the new camp the food is set down.
    let new_home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    for _ in 0..TICKS_PER_DAY {
        w.step();
    }
    let down = w
        .items
        .iter()
        .filter(|it| {
            it.kind.is_food() && matches!(it.place, ItemPlace::Ground(p) if p.dist(new_home) <= 8)
        })
        .map(|it| it.count)
        .sum::<u32>();
    // Some is eaten on the way and after; what matters is that it arrives (C73).
    assert!(down >= 4, "food set down at the new camp: {down}");
}

#[test]
fn a_baby_does_not_wonder_at_the_left_houses() {
    // #358: a child born two days after the move "wondered" at the old houses and kept the tale.
    let (mut w, cid) = big_valley(7203);
    let home = ready_to_move(&mut w, cid);
    hf_sim::systems::moving::consider(&mut w, cid);
    assert_eq!(w.left_sites.len(), 1);
    let id = w.spawn_agent(hf_sim::agents::Species::Person, home.offset(-7, -5, 0));
    w.join_community(id, cid);
    w.meta.tick += TICKS_PER_DAY;
    {
        let now = w.meta.tick;
        let a = w.agents.iter_mut().find(|a| a.id == id).unwrap();
        a.came = now;
        a.born = now as i64 - 2 * TICKS_PER_DAY as i64;
        a.needs.rest = 1.0;
        assert!(a.is_child(now));
    }
    w.events.drain();
    hf_sim::left_sites::wonder(&mut w);
    let evs = w.events.drain();
    assert!(
        !evs.iter()
            .any(|e| matches!(&e.kind, EventKind::RuinWondered { agent, .. } if *agent == id)),
        "a two-day-old does not wonder at ruins"
    );
}
