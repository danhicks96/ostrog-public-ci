//! C58: digging in after a raid, and timber from the caverns when the trees are gone.

use hf_sim::ids::{CommunityId, EventId, TilePos};
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::mind::beliefs::{Source, Topic};
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::project::ProjectKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::DesignationKind;
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Tile, TileFlags};
use hf_sim::World;

fn valley(seed: u64, knoll: bool) -> (World, CommunityId, TilePos) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(14, 14, 2);
    w.meta.tick = TICKS_PER_HOUR * 7;
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
    if knoll {
        for z in home.z..=home.z + 1 {
            for y in home.y - 6..home.y + 6 {
                for x in home.x + 8..home.x + 16 {
                    w.grid
                        .set(TilePos::new(x, y, z), Tile::solid(Material::GRANITE));
                }
            }
        }
        w.rebuild_caches();
    }
    w.meta.autoplan = true;
    w.ensure_nav();
    (w, cid, home)
}

fn a_morning(w: &mut World) {
    let day = w.meta.tick / TICKS_PER_DAY;
    w.meta.tick = day * TICKS_PER_DAY + hf_sim::systems::projects::AT;
    hf_sim::systems::projects::step(w);
    for _ in 0..2 {
        w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
        hf_sim::systems::settlement::step(w);
    }
}

fn raiders_camped(w: &mut World, home: TilePos) {
    // The raid on record (#405: the shelter answers the community's recorded raid, as the wall
    // does, not a band that happens to be camped at the morning's planning).
    let tick = w.meta.tick;
    for c in w.communities.iter_mut().filter(|c| c.home == home) {
        c.hardships.push((
            tick,
            "strangers from Ashby took from the store".into(),
            EventId(0),
        ));
    }
    w.bands.push(hf_sim::threat::Band {
        id: 0,
        site: 0,
        site_name: "Ashby".into(),
        members: Vec::new(),
        size: 3,
        camp: home.offset(-20, 10, 0),
        state: hf_sim::threat::BandState::Camped(w.meta.tick),
        formed: w.meta.tick,
        want: 18,
        taken: 6,
        lost: 0,
        sighted: true,
        welcomed: false,
        pursuits: Vec::new(),
        raiders: Vec::new(),
        envoys: Vec::new(),
        eaten: 0,
        night: 0,
        night_taken: 0,
        remembered: false,
        seen: Vec::new(),
        heard: Vec::new(),
        host: false,
        captives: Vec::new(),
    });
}

fn hold(w: &mut World, t: Topic, lean: f32) {
    for a in w.agents.iter_mut() {
        a.beliefs.hold(t, lean, 0.9, Source::Event(EventId(0)), 0);
    }
}

fn shelter(w: &World) -> Option<&hf_sim::project::Project> {
    w.projects.iter().find(|p| p.kind == ProjectKind::Shelter)
}

#[test]
fn they_dig_in_after_a_raid() {
    let (mut w, cid, home) = valley(5801, true);
    hold(&mut w, Topic::Strangers, 0.6);
    hold(&mut w, Topic::Violence, -0.6);
    raiders_camped(&mut w, home);
    a_morning(&mut w);
    let p = shelter(&w).expect("a shelter is proposed");
    assert!(p.because.contains("strangers from Ashby"), "{}", p.because);
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    assert!(
        c.hollows.iter().any(|h| h.shelter),
        "a shelter is marked out in the rock"
    );
    // The mouth and a room for the stores, half a tile a person (C67).
    let h = c.hollows.iter().find(|h| h.shelter).unwrap();
    assert_eq!(
        h.room.len(),
        hf_sim::systems::settlement::shelter_room(c.members.len() as u32)
    );
    assert!(
        w.designations
            .iter()
            .filter(|d| d.kind == DesignationKind::Mine)
            .count()
            > h.room.len()
    );
}

#[test]
fn no_raid_no_shelter() {
    let (mut w, _, _) = valley(5802, true);
    hold(&mut w, Topic::Strangers, 0.6);
    a_morning(&mut w);
    assert!(shelter(&w).is_none(), "nobody digs in without a cause");
}

#[test]
fn one_who_holds_the_deep_is_not_to_be_dug_does_not_dig() {
    let (mut w, _, home) = valley(5803, true);
    hold(&mut w, Topic::Strangers, 0.6);
    hold(&mut w, Topic::DeepDigging, -0.8);
    raiders_camped(&mut w, home);
    a_morning(&mut w);
    assert!(shelter(&w).is_none());
}

#[test]
fn what_is_behind_the_shelter_door_is_kept_from_raiders() {
    let (mut w, cid, home) = valley(5804, true);
    let (mouth, room) =
        hf_sim::systems::settlement::find_hollow_site(&w, home, 30, &|r, _| r.len() >= 9).unwrap();
    for p in std::iter::once(mouth).chain(room.iter().copied()) {
        w.grid.set(p, Tile::AIR);
    }
    let mut door = Tile::solid(Material::WOOD);
    door.flags = TileFlags(TileFlags::DOOR);
    w.grid.set(mouth, door);
    w.communities
        .iter_mut()
        .find(|c| c.id == cid)
        .unwrap()
        .hollows
        .push(hf_sim::society::Hollow {
            mouth,
            room: room.clone(),
            shelter: true,
            begun: 0,
            dug_down: false,
        });
    assert!(hf_sim::systems::settlement::in_shelter(&w, room[4]));
    assert!(!hf_sim::systems::settlement::in_shelter(&w, home));
    // Without its door it keeps nothing.
    w.grid.set(mouth, Tile::AIR);
    assert!(!hf_sim::systems::settlement::in_shelter(&w, room[4]));
    let _ = ItemPlace::Ground(home);
}

#[test]
fn fungus_wood_when_the_trees_are_gone() {
    let (mut w, _, home) = valley(5805, false);
    // No trees above; a stand of grown fungus trees within reach.
    for k in 0..4 {
        w.plants.push(Plant::new(
            home.offset(20 + k * 2, 5, 0),
            PlantKind::FungusTree,
            1.0,
            k as u32,
        ));
    }
    // And no timber in hand.
    let logs: Vec<_> = w
        .items
        .iter()
        .filter(|i| i.kind == ItemKind::Log)
        .map(|i| (i.id, i.count))
        .collect();
    for (id, n) in logs {
        w.consume_item(id, n, "test");
    }
    w.rebuild_caches();
    w.ensure_nav();
    for _ in 0..3 {
        w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
        hf_sim::systems::settlement::step(&mut w);
    }
    let chops: Vec<_> = w
        .designations
        .iter()
        .filter(|d| d.kind == DesignationKind::Chop)
        .collect();
    assert!(!chops.is_empty(), "fungus trees are felled for timber");
    assert!(chops.iter().all(|d| w
        .plants
        .iter()
        .any(|p| p.pos == d.pos && p.kind == PlantKind::FungusTree)));
}

/// #405: in the openings' valleys no rock face lay near home and few held either belief, so no
/// shelter was ever proposed. On flat ground a careful one proposes it after the raid on
/// record, and it is dug down behind a door at the foot of the stair; what is behind the door
/// is kept from raiders, and the stair's foot is not.
#[test]
fn on_flat_ground_a_careful_one_digs_a_shelter_down_behind_a_door() {
    let (mut w, cid, home) = valley(5806, false);
    hold(&mut w, Topic::Strangers, 0.0);
    hold(&mut w, Topic::Violence, 0.0);
    let careful = w
        .agents
        .iter()
        .position(|a| a.community == Some(cid))
        .unwrap();
    w.agents[careful]
        .traits
        .list
        .push(hf_sim::mind::Trait::Cautious);
    raiders_camped(&mut w, home);
    a_morning(&mut w);
    let p = shelter(&w).expect("a shelter is proposed");
    assert!(p.because.contains("careful one"), "{}", p.because);
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    let h = c
        .hollows
        .iter()
        .find(|h| h.shelter)
        .expect("marked out")
        .clone();
    assert!(h.dug_down, "down into the ground, for want of a face");
    let door = h.door().unwrap();
    assert_eq!(door, h.room[1]);
    // Only the door's tile lies beside the stair's foot.
    let foot = h.room[0];
    for q in &h.room[2..] {
        assert!((q.x - foot.x).abs() + (q.y - foot.y).abs() > 1, "{q:?}");
    }
    // Dug: the shaft as stairs, the rest open; the planner orders the door and the stores.
    w.grid.set(h.mouth, Tile::AIR);
    w.grid.set(foot, Tile::AIR);
    for q in &h.room[1..] {
        w.grid.set(*q, Tile::AIR);
    }
    w.designations
        .retain(|d| d.pos != h.mouth && !h.room.contains(&d.pos));
    w.rebuild_caches();
    w.ensure_nav();
    w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
    hf_sim::systems::settlement::step(&mut w);
    assert!(w
        .designations
        .iter()
        .any(|d| d.kind == DesignationKind::BuildDoor && d.pos == door));
    assert!(!w
        .designations
        .iter()
        .any(|d| d.pos == foot && matches!(d.kind, DesignationKind::BuildStructure(..))));
    let mut wood = Tile::solid(Material::WOOD);
    wood.flags = TileFlags(TileFlags::DOOR);
    w.grid.set(door, wood);
    assert!(hf_sim::systems::settlement::in_shelter(&w, h.room[2]));
    assert!(!hf_sim::systems::settlement::in_shelter(&w, foot));
}

/// #405: a store under water is no store. Food on a stockpile that water came into (a cellar cut
/// into wet ground) is ordered hauled out to dry ground, as anything loose is.
#[test]
fn food_in_a_flooded_store_is_hauled_out() {
    let (mut w, _, home) = valley(5807, false);
    let dry = home.offset(4, 4, 0);
    let wet = home.offset(6, 4, 0);
    for p in [dry, wet] {
        w.place_structure(
            hf_sim::structures::StructureKind::Stockpile,
            p,
            Material::SOIL,
            true,
        );
    }
    let mut t = w.grid.get(wet);
    t.flags = TileFlags(t.flags.0 | TileFlags::WATER);
    w.grid.set(wet, t);
    // Nothing else lying loose to haul first.
    w.items.clear();
    w.drop_item(ItemKind::Meal, 3, wet);
    w.rebuild_caches();
    w.ensure_nav();
    for _ in 0..2 {
        w.meta.tick = (w.meta.tick / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR;
        hf_sim::systems::settlement::step(&mut w);
    }
    assert!(w
        .designations
        .iter()
        .any(|d| d.kind == DesignationKind::Haul && d.pos == wet));
}
