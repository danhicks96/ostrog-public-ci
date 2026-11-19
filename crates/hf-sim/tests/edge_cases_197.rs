//! Edge cases worth provoking (#197, WP-08 slice 8.7): each test drives one scenario from the
//! issue's list and pins the behaviour it says is wanted.

use hf_sim::ids::TilePos;
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::save::{self, snapshot::Snapshot};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::{Designation, DesignationKind};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Shape, Tile, TileFlags};
use hf_sim::World;

fn meadow(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    w
}

fn set(w: &mut World, p: TilePos, shape: Shape, m: Material) {
    w.grid.set(
        p,
        Tile {
            material: m,
            shape,
            flags: TileFlags::NONE,
        },
    );
}

/// Case 8: the last open tile beside someone is walled in. The wall goes up on the tile they
/// stand on while rock or wall closes every neighbour: they must be taken out of the wall, with
/// what lay there, not left standing inside it.
#[test]
fn someone_in_a_walled_in_pocket_is_not_left_inside_the_wall() {
    let mut w = meadow(1971);
    let ids = w.found_party(1);
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let here = w.agents[i].pos;
    // A pocket: all eight neighbours at this level are wall.
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx != 0 || dy != 0 {
                set(
                    &mut w,
                    here.offset(dx, dy, 0),
                    Shape::Wall,
                    Material::STONE_BLOCK,
                );
            }
        }
    }
    let crate_at = w.create_item(ItemKind::Log, 1, ItemPlace::Ground(here), 0.6, "test");
    // The wall is finished on their own tile.
    set(&mut w, here, Shape::Wall, Material::STONE_BLOCK);
    w.rebuild_caches();
    hf_sim::systems::work::displace_from(&mut w, here);
    let a = &w.agents[i];
    assert_ne!(a.pos, here, "still standing inside the wall");
    assert!(
        w.grid.walkable(a.pos),
        "taken onto a tile nobody can stand on: {:?}",
        a.pos
    );
    let item = w.items.iter().find(|it| it.id == crate_at).unwrap();
    assert_ne!(
        item.place,
        ItemPlace::Ground(here),
        "an item was left inside the wall"
    );
}

/// Case 1: save on the tick of a death, then load. The restored game goes on exactly as the
/// continuous one does (the heirloom, the carrion and the rite all hang on that tick's events).
#[test]
fn a_save_made_on_the_tick_of_a_death_goes_on_as_the_continuous_game_does() {
    let mut continuous = World::opening_world(1972);
    continuous.run(600);
    let victim = continuous
        .agents
        .iter()
        .find(|a| a.alive && a.species == hf_sim::agents::Species::Person)
        .map(|a| a.id)
        .expect("a person");
    hf_sim::systems::life::die(&mut continuous, victim, "an edge-case test".into());
    continuous.step();
    let bytes = save::encode(&Snapshot::capture(&continuous)).unwrap();
    let mut restored = save::decode(&bytes).unwrap().restore();
    assert_eq!(
        hf_sim::digest::digest(&restored),
        hf_sim::digest::digest(&continuous),
        "differs straight after the load"
    );
    continuous.run(2_400);
    restored.run(2_400);
    assert_eq!(
        hf_sim::digest::digest(&restored),
        hf_sim::digest::digest(&continuous),
        "differs after going on"
    );
}

/// Case 10: mining a face on the grid's own edge (column 0, then the last column and the first
/// and last row) must not panic or leave the order stuck forever.
#[test]
fn mining_a_face_on_the_edge_of_the_grid_is_safe() {
    for (ex, ey, sx, sy) in [
        (0, 30, 1, 30),
        (63, 30, 62, 30),
        (30, 0, 30, 1),
        (30, 63, 30, 62),
    ] {
        let mut w = meadow(1973);
        let ids = w.found_party(1);
        let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
        let cid = w.agents[i].community.unwrap();
        w.meta.autoplan = false;
        let (wx, wy) = (w.grid.width - 1, w.grid.height - 1);
        let (ex, ey, sx, sy) = (
            if ex == 63 { wx } else { ex },
            if ey == 63 { wy } else { ey },
            if sx == 62 { wx - 1 } else { sx },
            if sy == 62 { wy - 1 } else { sy },
        );
        let face = TilePos::new(ex, ey, -1);
        let stand = TilePos::new(sx, sy, -1);
        set(&mut w, face, Shape::Wall, Material::GRANITE);
        set(&mut w, stand, Shape::Open, Material::AIR);
        w.dug.insert(stand);
        set(
            &mut w,
            stand.offset(0, 0, -1),
            Shape::Wall,
            Material::GRANITE,
        );
        w.agents[i].pos = stand;
        w.agents[i].from = stand;
        w.rebuild_caches();
        w.ensure_nav();
        let id = w.alloc_designation_id();
        w.designations.push(Designation {
            id,
            kind: DesignationKind::Mine,
            pos: face,
            requirements: vec![],
            work_left: 20 * 15,
            work_total: 20 * 15,
            max_workers: 1,
            priority: 9,
            created: w.tick(),
            failures: 0,
            done: false,
            community: Some(cid),
            owner: None,
        });
        for _ in 0..600 {
            w.step();
        }
        let d = w.designations.iter().find(|d| d.id == id);
        assert!(
            w.grid.get(face).shape != Shape::Wall && d.is_none_or(|d| d.done),
            "edge face ({ex},{ey}) was not cut in 600 ticks: {d:?}"
        );
    }
}

/// Case 2: save after a house's walls are up but before any roof tile, then load. The roof layer
/// round-trips exactly (still empty): a restored game must not hand the frame a free roof, and it
/// goes on as the continuous game does.
#[test]
fn a_save_made_between_walls_and_roof_comes_back_unroofed() {
    let mut w = meadow(1974);
    let _ids = w.found_party(3);
    w.meta.autoplan = false;
    // The ring of walls, no floor and no roof.
    let (x0, y0, z) = (40, 40, 2);
    for y in y0..y0 + 6 {
        for x in x0..x0 + 7 {
            if x == x0 || y == y0 || x == x0 + 6 || y == y0 + 5 {
                set(&mut w, TilePos::new(x, y, z), Shape::Wall, Material::WOOD);
            }
        }
    }
    w.rebuild_caches();
    assert!(w.roofs.is_empty());
    let bytes = save::encode(&Snapshot::capture(&w)).unwrap();
    let mut back = save::decode(&bytes).unwrap().restore();
    assert!(
        back.roofs.is_empty(),
        "a roof appeared on load: {}",
        back.roofs.len()
    );
    assert_eq!(hf_sim::digest::digest(&back), hf_sim::digest::digest(&w));
    for _ in 0..600 {
        w.step();
        back.step();
    }
    assert_eq!(hf_sim::digest::digest(&back), hf_sim::digest::digest(&w));
}

/// Case 6: a partner away on a caravan when the other dies. They must not come home still paired
/// to the dead.
#[test]
fn a_partner_away_when_the_other_dies_comes_home_unpaired() {
    let mut w = meadow(1976);
    let ids = w.found_party(4);
    let (a, b) = (ids[0], ids[1]);
    for (x, y) in [(a, b), (b, a)] {
        w.agents.iter_mut().find(|p| p.id == x).unwrap().partner = Some(y);
    }
    // One of them sets out: off the map and off the roll of those present.
    let ix = w.agents.iter().position(|p| p.id == b).unwrap();
    let away = w.agents.remove(ix);
    w.overworld.away.push(away);
    hf_sim::systems::life::die(&mut w, a, "an edge-case test".into());
    let b_after = w
        .overworld
        .away
        .iter()
        .find(|p| p.id == b)
        .expect("still away");
    assert_eq!(
        b_after.partner, None,
        "the one away is still paired to the dead"
    );
}

/// Case 5: a thing with a story does not rot (the White Hart's hide left out for weeks, or sent as
/// trade goods, once rotted in twenty days). An ordinary stack of the same kind does.
#[test]
fn a_thing_with_a_story_is_not_eaten_by_rot_and_its_twin_without_one_is() {
    let mut w = meadow(1975);
    let _ids = w.found_party(1);
    w.meta.autoplan = false;
    let spot = TilePos::new(50, 50, 2);
    let kept = w.create_item(ItemKind::Hide, 8, ItemPlace::Ground(spot), 0.6, "test");
    let plain = w.create_item(
        ItemKind::RawMeat,
        8,
        ItemPlace::Ground(spot.offset(3, 0, 0)),
        0.6,
        "test",
    );
    // As the hunt gives it (creatures.rs): a trophy's story is set directly on the hide.
    if let Some(it) = w.items.iter_mut().find(|it| it.id == kept) {
        it.story = Some(Box::new(hf_sim::legend::Story {
            maker_name: "taken from the White Hart, brought down by a hunter".into(),
            made: 0,
            wear: 1.0,
            ..Default::default()
        }));
    }
    let has_story = |id| {
        w.items
            .iter()
            .find(|it| it.id == id)
            .map(|it| it.story.is_some())
    };
    assert_eq!(has_story(kept), Some(true), "the trophy has no story");
    assert_eq!(has_story(plain), Some(false));
    // Forty days of hours, stepping only the spoilage rule at its own hour.
    for day in 0..40u64 {
        for hour in 0..24u64 {
            w.meta.tick = (10 + day) * TICKS_PER_DAY + hour * TICKS_PER_HOUR + 37;
            hf_sim::systems::spoil::step(&mut w);
        }
    }
    let count = |id| {
        w.items
            .iter()
            .find(|it| it.id == id)
            .map(|it| it.count)
            .unwrap_or(0)
    };
    assert_eq!(count(kept), 8, "the stack with a story rotted");
    assert!(
        count(plain) < 8,
        "the control stack never rotted: {}",
        count(plain)
    );
}

/// Case 3: a caravan is forming when something (a fire, a raid) pulls everyone away, so nobody comes
/// to the gathering. The journey is given up and what was loaded for it is set down at home, not
/// lost with it.
#[test]
fn a_caravan_nobody_comes_to_is_given_up_and_its_cargo_comes_back() {
    use hf_sim::expedition::{CaravanState, Purpose};
    let mut w = meadow(1977);
    w.found_party(10);
    let cid = w.communities[0].id;
    w.drop_item(ItemKind::Log, 40, TilePos::new(32, 30, 2));
    w.ensure_nav();
    let logs = |w: &World| -> u32 {
        w.items
            .iter()
            .filter(|it| it.kind == ItemKind::Log)
            .map(|it| it.count)
            .sum()
    };
    let site = w
        .overworld
        .sites
        .iter()
        .find(|s| s.wants.contains(&ItemKind::Log))
        .expect("a site that wants logs")
        .clone();
    let before = logs(&w);
    let id = hf_sim::systems::expedition::form_caravan(
        &mut w,
        cid,
        Purpose::Trade {
            site: site.id,
            carry: ItemKind::Log,
        },
    )
    .expect("caravan forms");
    assert!(
        w.overworld.caravans[0]
            .cargo
            .iter()
            .any(|(k, n)| *k == ItemKind::Log && *n > 0),
        "nothing was loaded"
    );
    let loaded = logs(&w);
    assert!(
        loaded < before,
        "formation did not take the logs out of the world ({loaded} of {before})"
    );
    // The gathering time has passed and nobody stands at the departure tile.
    let depart = w.overworld.caravans[0].depart_tile;
    for m in w.overworld.caravans[0].members.clone() {
        let i = w.agents.iter().position(|a| a.id == m).unwrap();
        let far = TilePos::new(
            (depart.x + 20).min(w.grid.width - 3),
            (depart.y + 20).min(w.grid.height - 3),
            2,
        );
        w.agents[i].pos = far;
        w.agents[i].from = far;
    }
    w.overworld.caravans[0].formed = 0;
    w.meta.tick = 3 * TICKS_PER_DAY;
    for _ in 0..30 {
        w.step();
        if w.overworld.caravans[0].state != CaravanState::Forming {
            break;
        }
    }
    let c = &w.overworld.caravans[0];
    assert_eq!(c.id, id);
    assert_eq!(
        c.state,
        CaravanState::Done,
        "the journey was not given up: {:?}",
        c.state
    );
    assert!(c.log.iter().any(|l| l.contains("given up")), "{:?}", c.log);
    assert!(
        logs(&w) >= before,
        "the cargo vanished with the caravan: {} of {before}",
        logs(&w)
    );
}

/// Case 13: a house wall gap over ground dug out beneath it. The issue feared such a gap could
/// never be built, so `roofs_and_stages` would repost it hourly, the house would stay "marked
/// out", and `all_roofed` would block every new house. It does not hold on master: a wall needs
/// no support beneath it (work.rs finishes a BuildWall as it stands), so the gap is reposted,
/// built again within a day, and the house comes back to roofed. Pinned so it stays that way.
#[test]
fn a_wall_gap_over_a_dug_out_hole_is_built_again_and_the_house_comes_back() {
    let mut w = meadow(1973);
    let ids = w.found_party(2);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    w.meta.autoplan = true;
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    let o = home.offset(-9, -7, 0);
    w.build_house(o.x, o.y, 7, 6, 1, Material::WOOD, Material::SOIL);
    {
        let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
        c.house_sites.push(o);
        c.house_stage.push(3);
    }
    // One wall of the house, not a door, knocked out and the ground beneath it dug away deep.
    let plan = hf_sim::houses::plan(&w, o);
    let (gap, _) = hf_sim::houses::walls(&plan.floor)
        .into_iter()
        .find(|(q, _)| !plan.doors.contains(q) && w.grid.get(*q).shape == Shape::Wall)
        .unwrap();
    for dz in 0..=4 {
        set(&mut w, gap.offset(0, 0, -dz), Shape::Open, Material::AIR);
    }
    w.rebuild_caches();
    w.ensure_nav();
    for _ in 0..3 * TICKS_PER_DAY {
        w.step();
    }
    assert_eq!(
        w.grid.get(gap).shape,
        Shape::Wall,
        "the gap was never built again"
    );
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    assert!(
        c.house_stage.iter().all(|s| *s == 0 || *s == 3),
        "the house did not come back to roofed, so it still blocks new houses: {:?}",
        c.house_stage
    );
}

/// Case 4: a roof falls in on a shelter being dug, refilling it. The record of a shelter
/// (`Hollow { shelter: true }`) is what stops `shelter()` from proposing another, so one left
/// standing for a hollow that will never be dug would mean no shelter ever again. Since #163 a
/// hollow still not dug after twenty days is given up, and one never opened is dropped, which
/// frees the community to propose a shelter again. Pinned here.
#[test]
fn a_shelter_refilled_before_it_was_opened_is_given_up_so_another_can_be_dug() {
    let mut w = meadow(1976);
    let ids = w.found_party(3);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    w.meta.autoplan = true;
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    // A shelter marked out in the rock beside home, its mouth and room all fallen in (solid).
    let mouth = home.offset(6, 0, 0);
    let room: Vec<TilePos> = (1..4).map(|k| mouth.offset(k, 0, 0)).collect();
    for p in std::iter::once(mouth).chain(room.iter().copied()) {
        set(&mut w, p, Shape::Wall, Material::GRANITE);
    }
    let begun = w.meta.tick;
    w.communities
        .iter_mut()
        .find(|c| c.id == cid)
        .unwrap()
        .hollows
        .push(hf_sim::society::Hollow {
            mouth,
            room: room.clone(),
            shelter: true,
            begun,
            dug_down: false,
        });
    w.rebuild_caches();
    // Nineteen days on it is still waited for (the planner's hour, no orders standing).
    w.meta.tick = begun + 19 * TICKS_PER_DAY;
    w.meta.tick -= w.meta.tick % TICKS_PER_HOUR;
    w.designations.clear();
    hf_sim::systems::settlement::step(&mut w);
    assert!(
        w.communities
            .iter()
            .find(|c| c.id == cid)
            .unwrap()
            .hollows
            .iter()
            .any(|h| h.shelter && h.mouth == mouth),
        "given up before its twenty days"
    );
    // Twenty-one days on, at the planner's hour.
    w.meta.tick = begun + 21 * TICKS_PER_DAY;
    w.meta.tick -= w.meta.tick % TICKS_PER_HOUR;
    // Keep anyone from digging it out meanwhile: no orders stand for it.
    w.designations.clear();
    hf_sim::systems::settlement::step(&mut w);
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    assert!(
        !c.hollows.iter().any(|h| h.shelter && h.mouth == mouth),
        "a shelter never opened is still on record after twenty days, so none can be proposed again"
    );
}

/// Case 7: the founding community dies out while a split-off lives on. Bands go to `home()`
/// (threat.rs), the first community not dissolved; `moving::endings` dissolves one with nobody
/// left by the next daily round, so the bands turn to the split-off. Pinned here. And the
/// neighbour of case 14: a community whose people are all away on the road has not died out.
#[test]
fn a_community_dies_out_when_nobody_is_left_and_not_while_they_are_away() {
    let mut w = meadow(1975);
    let first = w.found_party(2);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == first[0])
        .unwrap()
        .community
        .unwrap();
    w.meta.tick += 2 * TICKS_PER_DAY;
    // Everyone of it away on the road, on journeys of their own: not ended.
    let back = w.meta.tick + 5 * TICKS_PER_DAY;
    for id in &first {
        let ix = w.agents.iter().position(|a| a.id == *id).unwrap();
        let mut a = w.agents.remove(ix);
        a.journey = Some((back, "Tollmoor".into(), "to see it".into()));
        w.overworld.away.push(a);
    }
    hf_sim::systems::moving::endings(&mut w);
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    assert!(
        !c.dissolved,
        "a community whose people are all on the road was recorded as died out"
    );
    // Dead on the road, all of them: now it has ended, so the bands' home moves on.
    for a in w.overworld.away.iter_mut() {
        a.alive = false;
    }
    hf_sim::systems::moving::endings(&mut w);
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    assert!(
        c.dissolved,
        "a community with nobody left alive is still standing"
    );
}

/// Case 14: the proposer of a project leaves on a caravan. `close()` (systems/projects.rs)
/// took anyone not in `w.agents` for gone, and someone on the road is kept in `overworld.away`
/// until they come back, so their project ended the day they set off.
#[test]
fn a_project_is_not_ended_because_its_proposer_is_away_on_the_road() {
    let mut w = meadow(1974);
    let ids = w.found_party(3);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    let by = ids[0];
    let id = w.next_project_id;
    w.next_project_id += 1;
    let tick = w.meta.tick;
    w.projects.push(hf_sim::project::Project {
        id,
        by,
        community: cid,
        // A brew ends only by age (PROJECT_DAYS), so within a day nothing but its proposer's
        // absence could close it.
        kind: hf_sim::project::ProjectKind::Brew,
        because: "test".into(),
        source: None,
        proposed: tick,
        work: None,
        done: false,
        site: None,
    });
    // They set off on the road, a journey of their own: off the map, kept whole in
    // overworld.away.
    let ix = w.agents.iter().position(|a| a.id == by).unwrap();
    let mut a = w.agents.remove(ix);
    a.journey = Some((
        w.meta.tick + 5 * TICKS_PER_DAY,
        "Tollmoor".into(),
        "to see it".into(),
    ));
    w.overworld.away.push(a);
    w.rebuild_caches();
    w.meta.autoplan = true;
    // The day's project round (systems/projects.rs runs it once a day at its hour).
    let round = |w: &mut World| {
        let day = hf_sim::time::TICKS_PER_DAY;
        let at = hf_sim::systems::projects::AT;
        w.meta.tick = (w.meta.tick / day + 1) * day + at;
        hf_sim::systems::projects::step(w);
    };
    round(&mut w);
    let p = w.projects.iter().find(|p| p.id == id).unwrap();
    assert!(
        !p.done,
        "the project ended because its proposer was on the road"
    );
    // Away but not on the road (carried off, no journey, no caravan) is gone, as a captive
    // or a child gone below for good is.
    w.overworld.away[0].journey = None;
    round(&mut w);
    assert!(
        w.projects.iter().find(|p| p.id == id).unwrap().done,
        "a proposer away for good still held the project open"
    );
    w.projects.iter_mut().find(|p| p.id == id).unwrap().done = false;
    w.overworld.away[0].journey = Some((
        w.meta.tick + 5 * TICKS_PER_DAY,
        "Tollmoor".into(),
        "to see it".into(),
    ));
    // Dead on the road is gone.
    w.overworld.away[0].alive = false;
    round(&mut w);
    let p = w.projects.iter().find(|p| p.id == id).unwrap();
    assert!(
        p.done,
        "a proposer who died away still holds the project open"
    );
}
/// Case 12: a band leaving for home whose edge tile cannot be reached. `go()` failed without a
/// word, the members stood where they were, the band was never removed, and `busy` kept its
/// site from ever sending another. Walled into pockets, they must still leave the valley.
#[test]
fn a_leaving_band_that_cannot_reach_the_edge_still_goes() {
    let mut w = World::opening_world(3);
    w.meta.autoplan = true;
    w.meta.engine = hf_sim::engine::Engine::Relentless;
    for _ in 0..4 * TICKS_PER_DAY {
        w.step();
    }
    let s = w.overworld.sites.iter_mut().find(|s| s.people > 0).unwrap();
    s.food = -(s.people as f32) * 4.0;
    s.hungry_days = 3;
    let mut tries = 0;
    while w.bands.is_empty() {
        w.step();
        tries += 1;
        assert!(tries < 8 * TICKS_PER_DAY, "no band came");
    }
    let (band, site) = (w.bands[0].id, w.bands[0].site);
    w.bands[0].state = hf_sim::threat::BandState::Leaving;
    // Every member shut into a pocket of wall of their own, in the middle of the valley.
    let members = w.bands[0].members.clone();
    let (cx, cy) = (w.grid.width / 2, w.grid.height / 2);
    for (k, m) in members.iter().enumerate() {
        let Some(i) = w.agents.iter().position(|a| a.id == *m && a.alive) else {
            continue;
        };
        let here = w.grid.surface(cx + 4 * k as i32, cy);
        w.agents[i].pos = here;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx != 0 || dy != 0 {
                    set(
                        &mut w,
                        here.offset(dx, dy, 0),
                        Shape::Wall,
                        Material::STONE_BLOCK,
                    );
                }
            }
        }
    }
    w.rebuild_caches();
    for _ in 0..TICKS_PER_DAY {
        w.step();
        if w.bands.iter().all(|b| b.id != band) {
            break;
        }
    }
    assert!(
        w.bands.iter().all(|b| b.id != band),
        "the band is still in the valley a day after it set off for home"
    );
    assert!(
        w.bands.iter().all(|b| b.site != site),
        "its site is still busy, so it can never send another band"
    );
}

/// Case 11: a night raid does not go forever unopposed. Sightings and the
/// Confront/Welcome answers need someone awake, and a raider moving among the
/// houses gives each nearby sleeper a 25% stream-seeded wake roll per tick
/// (threat.rs `raids`), so someone must wake within a bounded window.
#[test]
fn a_night_raid_eventually_wakes_someone() {
    let mut w = World::opening_world(19711);
    let home = w
        .grid
        .surface(w.meta.founding_site.x, w.meta.founding_site.y);
    // Everyone asleep at home on one level.
    for a in w.agents.iter_mut() {
        a.pos = home;
        a.activity = hf_sim::mind::Activity::Sleeping {
            bed: None,
            until: 9_999_999,
        };
        a.activity_ticks = 0;
    }
    // One band camped on the doorstep, a raider standing among the sleepers.
    let r = w.spawn_agent(hf_sim::agents::Species::Person, home);
    w.bands.push(hf_sim::threat::Band {
        id: 91,
        site: 0,
        site_name: "the test band".to_string(),
        members: vec![r],
        size: 1,
        camp: home,
        state: hf_sim::threat::BandState::Camped(0),
        formed: 0,
        want: 6,
        taken: 0,
        lost: 0,
        sighted: false,
        welcomed: false,
        pursuits: vec![],
        raiders: vec![(r, hf_sim::ids::ItemId(0))],
        envoys: vec![],
        eaten: 0,
        night: 0,
        night_taken: 0,
        remembered: false,
        seen: vec![],
        captives: vec![],
        heard: vec![],
        host: false,
    });
    // Deep night for the whole bounded run.
    let night_start = 22 * hf_sim::time::TICKS_PER_HOUR;
    w.meta.tick = night_start;
    let mut woke = false;
    for _it in 0..hf_sim::time::TICKS_PER_HOUR {
        hf_sim::threat::step(&mut w);
        if w.agents
            .iter()
            .any(|a| a.alive && a.community.is_some() && !a.activity.is_asleep())
        {
            woke = true;
            break;
        }
        w.meta.tick += 1;
    }
    assert!(
        woke,
        "nobody woke through a night of raiding among the sleepers"
    );
}
