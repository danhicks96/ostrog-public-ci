//! #469: an exhausted person walks home to a bed they personally know, however far it is;
//! and the loser of a race for one bed re-decides (toward their next known free bed) instead
//! of sleeping on the ground beside the taken one. An unknown completed bed is a choice only
//! when the sleeper can see it now; knowledge stays personal and is never broadcast. The beds
//! of a person's own home are the exception, as its hearths are: known to all of it.

use hf_sim::agents::Species;
use hf_sim::ids::TilePos;
use hf_sim::mind::{Activity, Known};
use hf_sim::structures::StructureKind;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

/// A wide flat meadow at night on day 1 (hour 23, spring).
fn night_world(seed: u64) -> World {
    let mut p = GenParams::small();
    p.width = 96;
    p.height = 96;
    let mut w = World::generate(seed, p);
    w.flatten(2, 2, 92, 92, 1);
    w.meta.tick = TICKS_PER_DAY + 23 * TICKS_PER_HOUR;
    w.rebuild_caches();
    w
}

/// A bed on the meadow, remembered in one person's own knowledge.
fn bed(w: &mut World, x: i32, y: i32) -> TilePos {
    let p = TilePos::new(x, y, 2);
    w.place_structure(StructureKind::Bed, p, Material::WOOD, false);
    p
}

/// A exhausted, otherwise content person at `at` who personally knows `beds`.
fn tired_person(w: &mut World, at: TilePos, beds: &[TilePos]) -> usize {
    let id = w.spawn_agent(Species::Person, at);
    let i = w.agents.iter().position(|a| a.id == id).unwrap();
    let now = w.meta.tick;
    let a = &mut w.agents[i];
    a.activity = Activity::Idle;
    a.idle_ticks = 0;
    a.needs.rest = 0.1;
    a.needs.hunger = 1.0;
    a.needs.thirst = 1.0;
    a.needs.warmth = 1.0;
    a.needs.social = 1.0;
    for &p in beds {
        let sid = w
            .structures
            .iter()
            .find(|s| s.pos == p)
            .unwrap_or_else(|| panic!("no structure at {p:?}"))
            .id;
        a.knowledge
            .note(Known::Structure(sid), p, now, StructureKind::Bed as u16);
    }
    i
}

const QUARTER_DAY: u64 = TICKS_PER_DAY / 4;

fn assert_first_step_goes_to_bed(known: bool) {
    let mut w = night_world(9);
    let home = bed(&mut w, 10, 10);
    let bed_id = w.structures.iter().find(|s| s.pos == home).unwrap().id;
    let spot = TilePos::new(10, 12, 2);
    let known_beds = [home];
    let i = tired_person(&mut w, spot, if known { &known_beds[..] } else { &[] });
    assert_eq!(w.meta.tick, 56_400);
    assert_eq!(
        w.agents[i]
            .knowledge
            .get(Known::Structure(bed_id))
            .is_some(),
        known
    );

    w.step();

    assert!(
        matches!(&w.agents[i].activity, Activity::Going { then, .. } if matches!(**then, Activity::Sleeping { bed: Some(id), .. } if id == bed_id)),
        "first activity was {:?} (known = {known})",
        w.agents[i].activity
    );
}

#[test]
fn an_exhausted_person_uses_a_visible_bed_they_do_not_know() {
    assert_first_step_goes_to_bed(false);
}

#[test]
fn an_exhausted_person_uses_a_known_bed_on_the_same_off_stripe_step() {
    assert_first_step_goes_to_bed(true);
}

#[test]
fn an_exhausted_person_does_not_use_an_unfinished_visible_bed() {
    let mut w = night_world(10);
    let home = bed(&mut w, 10, 10);
    w.structures
        .iter_mut()
        .find(|s| s.pos == home)
        .unwrap()
        .progress = 0.5;
    let i = tired_person(&mut w, TilePos::new(10, 12, 2), &[]);

    w.step();

    assert!(
        matches!(w.agents[i].activity, Activity::Sleeping { bed: None, .. }),
        "first activity was {:?}",
        w.agents[i].activity
    );
}

#[test]
fn sleepy_decisions_index_remote_sleeping_places_once_a_tick() {
    let mut w = night_world(11);
    for y in 60..80 {
        for x in 60..80 {
            bed(&mut w, x, y);
        }
    }
    for _ in 0..hf_sim::systems::decide::DECISIONS_PER_TICK {
        tired_person(&mut w, TilePos::new(10, 12, 2), &[]);
    }
    let before = hf_sim::systems::decide::sleep_place_walks();

    w.step();

    let walks = hf_sim::systems::decide::sleep_place_walks() - before;
    assert_eq!(
        walks, 1,
        "24 decisions with 400 remote beds took {walks} full structure walks"
    );
    assert!(
        w.agents.iter().all(|a| {
            !matches!(a.activity, Activity::Sleeping { bed: Some(_), .. })
                && !matches!(&a.activity, Activity::Going { then, .. } if matches!(**then, Activity::Sleeping { bed: Some(_), .. }))
        }),
        "nobody chose a remote bed"
    );
}

#[test]
fn an_exhausted_person_walks_home_to_a_known_bed() {
    // The bed is farther than the ~40 tiles past which the ground used to outscore it
    // for an exhausted sleeper (needs.rest = 0.1, so rest_p = 0.9 > 0.8).
    for dist in [45, 60] {
        let mut w = night_world(7);
        let home = bed(&mut w, 10, 10);
        let spot = w.grid.surface(10 + dist, 10);
        let i = tired_person(&mut w, spot, &[home]);
        let mut reached = false;
        for _ in 0..QUARTER_DAY {
            w.step();
            match &w.agents[i].activity {
                Activity::Sleeping { bed: Some(_), .. } => {
                    reached = true;
                    break;
                }
                Activity::Sleeping { bed: None, .. } => {
                    panic!("slept on the ground {dist} tiles from a bed they know");
                }
                _ => {}
            }
        }
        assert!(
            reached,
            "an exhausted person never reached their known bed {dist} tiles away"
        );
    }
}

#[test]
fn the_loser_of_a_bed_race_redecides_instead_of_ground_sleeping() {
    let mut w = night_world(8);
    let near = bed(&mut w, 40, 40);
    let far = bed(&mut w, 56, 40);
    // Both know `near`; the one who also knows `far` is the one the race must not leave on
    // the ground.
    let pl = w.grid.surface(40, 41);
    let il = tired_person(&mut w, pl, &[near, far]);
    let pk = w.grid.surface(40, 41);
    let ik = tired_person(&mut w, pk, &[near]);
    let far_id = w.structures.iter().find(|s| s.pos == far).unwrap().id;
    let near_id = w.structures.iter().find(|s| s.pos == near).unwrap().id;
    // Two who picked the same bed both lie down in it, just arrived (what a simultaneous
    // pick, or a world saved and loaded mid-arrival, can leave). The act system keeps one
    // and must send the other to re-decide — not leave them on the ground beside the taken
    // bed.
    let until = w.meta.tick + 8 * TICKS_PER_HOUR;
    for i in [il, ik] {
        w.agents[i].activity = Activity::Sleeping {
            bed: Some(near_id),
            until,
        };
        w.agents[i].activity_ticks = 0;
    }
    let mut reached = false;
    for _ in 0..QUARTER_DAY {
        w.step();
        match &w.agents[il].activity {
            Activity::Sleeping { bed: Some(b), .. } if *b == far_id => {
                reached = true;
                break;
            }
            Activity::Sleeping { bed: None, .. } => {
                panic!("the race's loser slept on the ground beside the taken bed");
            }
            _ => {}
        }
    }
    assert!(
        reached,
        "the race's loser never re-decided toward their other known bed"
    );
    assert!(
        matches!(&w.agents[ik].activity, Activity::Sleeping { bed: Some(b), .. } if *b == near_id),
        "one of them keeps the contested bed"
    );
}

/// A tired person of a community whose home is at `home`, at `at`, who knows no bed.
fn tired_villager(w: &mut World, home: TilePos, at: TilePos) -> usize {
    let cid = w.found_community(home);
    let i = tired_person(w, at, &[]);
    let id = w.agents[i].id;
    w.agents[i].community = Some(cid);
    w.communities
        .iter_mut()
        .find(|c| c.id == cid)
        .unwrap()
        .members
        .push(id);
    i
}

fn first_choice_is_bed(w: &mut World, i: usize, bed_at: TilePos) -> bool {
    let bed_id = w.structures.iter().find(|s| s.pos == bed_at).unwrap().id;
    assert!(w.agents[i]
        .knowledge
        .get(Known::Structure(bed_id))
        .is_none());
    w.step();
    match &w.agents[i].activity {
        Activity::Going { then, .. } => {
            matches!(**then, Activity::Sleeping { bed: Some(id), .. } if id == bed_id)
        }
        Activity::Sleeping { bed: None, .. } => false,
        other => panic!("first activity was {other:?}"),
    }
}

#[test]
fn a_villager_who_has_forgotten_the_beds_of_home_still_goes_to_one() {
    // #469 (FIXLOG 2026-11-02): on master most rough nights were spent 8 to 15 tiles from a
    // free bed of home that had been pushed out of mind or never seen finished.
    let mut w = night_world(12);
    let bed_at = bed(&mut w, 20, 20);
    let i = tired_villager(&mut w, TilePos::new(22, 20, 2), TilePos::new(50, 20, 2));
    assert!(
        first_choice_is_bed(&mut w, i, bed_at),
        "slept on the ground 30 tiles from a free bed of home"
    );
}

#[test]
fn a_bed_far_from_home_is_not_known_without_seeing_it() {
    let mut w = night_world(13);
    let bed_at = bed(&mut w, 20, 20);
    // Home is 60 tiles from the bed: not one of home's beds.
    let i = tired_villager(&mut w, TilePos::new(80, 20, 2), TilePos::new(50, 20, 2));
    assert!(!first_choice_is_bed(&mut w, i, bed_at));
}

#[test]
fn a_stranger_does_not_know_a_village_bed_without_seeing_it() {
    let mut w = night_world(14);
    let bed_at = bed(&mut w, 20, 20);
    let i = tired_person(&mut w, TilePos::new(50, 20, 2), &[]);
    assert!(!first_choice_is_bed(&mut w, i, bed_at));
}
