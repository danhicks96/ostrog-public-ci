//! Masterworks (C35): engravings that show a real event of the record, and fey moods that come
//! only from a cause in the person's own past and make the finest piece of their life.

use hf_sim::events::{Event, EventKind, Severity};
use hf_sim::ids::TilePos;
use hf_sim::items::{ItemKind as K, ItemPlace};
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::structures::{Structure, StructureKind};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::work::DesignationKind;
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material as M;
use hf_sim::World;

fn village(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    for i in 0..100 {
        w.plants.push(Plant::new(
            TilePos::new(4 + (i * 7) % 56, 4 + (i * 11) % 56, 2),
            PlantKind::BerryBush,
            1.0,
            i as u32,
        ));
    }
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.build_house(36, 24, 7, 6, 1, M::STONE_BLOCK, M::STONE_BLOCK);
    w.rebuild_caches();
    w.found_party(6);
    w.meta.autoplan = false;
    w.communities[0].house_sites.push(TilePos::new(36, 24, 2));
    w.ensure_nav();
    w
}

/// Push a major event through the pipeline's flush and the culture pass, as a real one goes.
fn happen(w: &mut World, kind: EventKind) -> hf_sim::ids::EventId {
    let tick = w.meta.tick;
    let id = w
        .events
        .push(tick, Some(TilePos::new(30, 30, 2)), Severity::Major, kind);
    hf_sim::systems::flush::step(w);
    hf_sim::systems::culture::observe(w);
    id
}

#[test]
fn an_engraving_shows_a_real_row() {
    let mut w = village(3501);
    let ids: Vec<_> = w
        .agents
        .iter()
        .filter(|a| a.community.is_some())
        .map(|a| a.id)
        .collect();
    let id = happen(
        &mut w,
        EventKind::Paired {
            a: ids[0],
            b: ids[1],
        },
    );
    let carver = ids[2];
    let ci = w.agents.iter().position(|a| a.id == carver).unwrap();
    for _ in 0..400 {
        w.agents[ci]
            .skills
            .practise(hf_sim::mind::Skill::Construction, 1.0);
    }
    let wall = hf_sim::engrave::wall_to_carve(&w, w.communities[0].home, 40).expect("a bare wall");
    let cid = w.communities[0].id;
    let mut d = hf_sim::systems::settlement::make(&mut w, DesignationKind::Engrave, wall, 9);
    d.community = Some(cid);
    d.owner = Some(carver);
    w.designations.push(d);
    for _ in 0..TICKS_PER_DAY {
        w.step();
        if !w.engravings.is_empty() {
            break;
        }
    }
    let e = w
        .engravings
        .get(&wall)
        .expect("the wall was carved")
        .clone();
    assert_eq!(e.event, id, "it shows the pairing that happened");
    assert_eq!(e.carver, carver);
    let (tick, kind) = w
        .culture_log
        .iter()
        .find(|(x, _, _)| *x == id)
        .map(|(_, t, k)| (*t, k.clone()))
        .unwrap();
    let back = hf_sim::view::narrate::line(
        &w,
        &Event {
            id,
            tick,
            at: None,
            severity: Severity::Major,
            kind,
        },
    );
    assert_eq!(e.line, back, "its words are the record's words");
    let note = hf_sim::engrave::note(&w, wall).unwrap();
    assert!(
        note.starts_with("Carved by ") && note.contains(back.trim_end_matches('.')),
        "{note}"
    );
}

#[test]
fn nothing_is_carved_that_did_not_happen_to_them() {
    let w = village(3502);
    let i = w.agents.iter().position(|a| a.community.is_some()).unwrap();
    assert!(
        hf_sim::engrave::choose(&w, i).is_none(),
        "an empty record gives nothing to carve"
    );
}

/// The opened world's party (they have recorded pasts), with a bench and marble at home.
fn party_with_marble(seed: u64) -> World {
    let mut w = World::opening_world(seed);
    let home = w.communities[0].home;
    let id = w.alloc_structure_id();
    let bench = TilePos::new(home.x + 2, home.y, home.z);
    w.structures.push(Structure {
        id,
        kind: StructureKind::Workbench,
        pos: bench,
        material: M::OAK,
        progress: 1.0,
        active: false,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
    let at = home.offset(1, 1, 0);
    let marble = w.create_item_of(
        K::StoneBlock,
        M::MARBLE,
        4,
        ItemPlace::Ground(at),
        0.6,
        "test",
    );
    // They have seen it: a mood takes only what its maker knows of.
    let tick = w.meta.tick;
    for a in w.agents.iter_mut().filter(|a| a.community.is_some()) {
        a.knowledge.note(
            hf_sim::mind::knowledge::Known::Item(marble),
            at,
            tick,
            K::StoneBlock as u16,
        );
    }
    w.ensure_nav();
    w
}

#[test]
fn a_mood_has_a_cause() {
    let mut w = party_with_marble(3503);
    // Someone of the party whose past holds a cause.
    let (i, past) = (0..w.agents.len())
        .filter(|&i| w.agents[i].community.is_some())
        .map(|i| (i, w.agents[i].past.clone()))
        .find(|(i, _)| hf_sim::systems::moods::take(&mut w.clone(), *i, 0))
        .expect("someone in the party has a recorded cause");
    assert!(hf_sim::systems::moods::take(&mut w, i, 0));
    let fey = w.agents[i].fey.clone().unwrap();
    assert!(
        past.contains(&fey.cause),
        "the cause is a row of their own past"
    );
    let who = w.agents[i].id;
    let mut piece = None;
    for _ in 0..TICKS_PER_DAY * 3 {
        w.step();
        piece = w
            .items
            .iter()
            .find(|it| {
                it.kind == K::Figurine && it.story.as_ref().is_some_and(|s| s.maker == Some(who))
            })
            .cloned();
        if piece.is_some() {
            break;
        }
    }
    let piece = piece.expect("the work was made");
    assert_eq!(
        piece.material,
        M::MARBLE,
        "of the finest stone in the store"
    );
    assert_eq!(piece.quality, 1.0);
    let s = piece.story.unwrap();
    assert!(s.name.is_some(), "it has a name");
    assert!(s.origin.contains("fey mood"), "{}", s.origin);
    assert!(w.agents[i].fey.is_none());
}

#[test]
fn no_cause_no_mood() {
    let mut w = village(3504);
    let i = w.agents.iter().position(|a| a.community.is_some()).unwrap();
    // Founders of a plain village have no recorded past and no kin dead here.
    assert!(!hf_sim::systems::moods::take(&mut w, i, 0));
    assert!(w.agents[i].fey.is_none());
}

/// Found by the C35 soaks: a mood began, its maker went to eat first, and the order was swept
/// away as a stale undertaking an hour later, so no mood ever ended in a piece. The work waits
/// while they eat and sleep; only the mood ends it.
#[test]
fn a_mood_waits_while_they_eat_and_sleep() {
    let mut w = party_with_marble(3505);
    // Evening: nobody works at night, so the work must wait out the night (the sweep took
    // waiting works an hour after they were set).
    let day = w.meta.tick / TICKS_PER_DAY;
    w.meta.tick = day * TICKS_PER_DAY + 22 * TICKS_PER_HOUR + TICKS_PER_HOUR / 2;
    let i = (0..w.agents.len())
        .filter(|&i| w.agents[i].community.is_some())
        .find(|&i| hf_sim::systems::moods::take(&mut w.clone(), i, 0))
        .expect("someone in the party has a recorded cause");
    assert!(hf_sim::systems::moods::take(&mut w, i, 0));
    let who = w.agents[i].id;
    // Hungry: they eat before they work.
    w.agents[i].needs.hunger = 0.05;
    let home = w.communities[0].home;
    w.create_item(
        K::Meal,
        3,
        ItemPlace::Ground(home.offset(-1, 1, 0)),
        0.8,
        "test",
    );
    let work = w.agents[i].fey.as_ref().unwrap().work;
    let mut made = false;
    for t in 0..TICKS_PER_DAY * 4 {
        w.step();
        if t == TICKS_PER_HOUR * 3 {
            assert!(
                w.designations.iter().any(|d| d.id == work),
                "the work is still waiting through the night"
            );
        }
        if w.items.iter().any(|it| {
            it.kind == K::Figurine && it.story.as_ref().is_some_and(|s| s.maker == Some(who))
        }) {
            made = true;
            break;
        }
    }
    assert!(made, "the work was made after they had eaten");
}

/// Weightier events are carved first: a death before a quarrel (the first soaks carved a
/// hundred walls of quarrels).
#[test]
fn a_death_is_carved_before_a_quarrel() {
    let mut w = village(3506);
    let ids: Vec<_> = w
        .agents
        .iter()
        .filter(|a| a.community.is_some())
        .map(|a| a.id)
        .collect();
    happen(
        &mut w,
        EventKind::Fought {
            a: ids[0],
            b: ids[1],
        },
    );
    let died = happen(
        &mut w,
        EventKind::Died {
            agent: ids[2],
            cause: "fever".into(),
        },
    );
    let i = w.agents.iter().position(|a| a.id == ids[3]).unwrap();
    let (event, _, line) = hf_sim::engrave::choose(&w, i).expect("something to carve");
    assert_eq!(event, died, "{line}");
}

/// A carving goes with its wall: none is left drawn on bare ground (seen in the C35 soaks).
#[test]
fn a_carving_goes_with_its_wall() {
    let mut w = village(3507);
    let ids: Vec<_> = w
        .agents
        .iter()
        .filter(|a| a.community.is_some())
        .map(|a| a.id)
        .collect();
    happen(
        &mut w,
        EventKind::Paired {
            a: ids[0],
            b: ids[1],
        },
    );
    let wall = hf_sim::engrave::wall_to_carve(&w, w.communities[0].home, 40).expect("a bare wall");
    let i = w.agents.iter().position(|a| a.id == ids[2]).unwrap();
    hf_sim::engrave::carve(&mut w, i, wall);
    assert!(w.engravings.contains_key(&wall));
    hf_sim::systems::nav::step(&mut w);
    assert!(
        w.engravings.contains_key(&wall),
        "it stays while the wall stands"
    );
    w.grid.set(
        wall,
        hf_sim::world::Tile {
            material: M::AIR,
            shape: hf_sim::world::Shape::Open,
            flags: hf_sim::world::TileFlags::NONE,
        },
    );
    hf_sim::systems::nav::step(&mut w);
    assert!(!w.engravings.contains_key(&wall), "gone with its wall");
}
