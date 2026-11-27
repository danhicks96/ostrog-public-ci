use hf_sim::events::EventKind;
use hf_sim::mind::Activity;
use hf_sim::structures::StructureKind;
use hf_sim::systems::culture::{hearth_of, step};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR, TICKS_PER_YEAR};
use hf_sim::world::Material;
use hf_sim::World;

fn circle() -> World {
    let mut w = World::opening_world(61);
    let community = w.communities[0].id;
    let home = w.communities[0].home;
    if hearth_of(&w, community).is_none() {
        w.place_structure(StructureKind::Hearth, home, Material::STONE_BLOCK, true);
    }
    let h = hearth_of(&w, community).unwrap();
    for a in &mut w.agents {
        a.pos = h.offset(30, 30, 0);
    }
    for (i, a) in w.agents.iter_mut().take(3).enumerate() {
        a.community = Some(community);
        a.band = None;
        a.pos = h.offset(i as i32 - 1, 0, 0);
        a.from = a.pos;
        a.activity = Activity::Idle;
        a.stories.clear();
        a.tales.clear();
        a.born = -30 * TICKS_PER_YEAR as i64;
    }
    w.meta.tick = 20 * TICKS_PER_DAY + 19 * TICKS_PER_HOUR + TICKS_PER_HOUR / 2;
    w.events.pending.clear();
    w
}

fn assert_excluded(mut w: World) {
    let passer = w.agents[2].id;
    let other = w.agents[1].id;
    step(&mut w);
    let (teller, listeners) = w
        .events
        .pending
        .iter()
        .find_map(|e| match &e.kind {
            EventKind::HearthTale {
                teller, listeners, ..
            } => Some((*teller, listeners)),
            _ => None,
        })
        .expect("the two stationary people can still share a story");
    assert_ne!(teller, passer);
    assert!(
        !listeners.contains(&passer),
        "a passer was counted as a listener"
    );
    assert!(
        teller == other || listeners.contains(&other),
        "the other stationary person participates"
    );
}

#[test]
fn a_person_going_elsewhere_is_not_a_hearth_listener() {
    let mut w = circle();
    let h = hearth_of(&w, w.communities[0].id).unwrap();
    w.agents[2].activity = Activity::Going {
        to: h.offset(0, -5, 0),
        then: Box::new(Activity::Idle),
    };
    // Even between strides, they remain committed to leaving.
    assert_excluded(w);
}

#[test]
fn a_person_wandering_past_is_not_a_hearth_listener() {
    let mut w = circle();
    let h = hearth_of(&w, w.communities[0].id).unwrap();
    w.agents[2].activity = Activity::Wandering {
        to: h.offset(0, -5, 0),
    };
    assert_excluded(w);
}

#[test]
fn someone_mid_stride_is_not_a_hearth_listener() {
    let mut w = circle();
    let h = hearth_of(&w, w.communities[0].id).unwrap();
    w.agents[2].from = h;
    w.agents[2].move_ticks = 2;
    w.agents[2].move_total = 4;
    assert_excluded(w);
}

#[test]
fn someone_eating_can_join_the_hearth_telling() {
    let mut w = circle();
    let eater = w.agents[2].id;
    w.agents[2].activity = Activity::Eating {
        item: w.items[0].id,
        ticks_left: 20,
    };
    step(&mut w);
    let (teller, listeners) = w
        .events
        .pending
        .iter()
        .find_map(|e| match &e.kind {
            EventKind::HearthTale {
                teller, listeners, ..
            } => Some((*teller, listeners)),
            _ => None,
        })
        .expect("people at the hearth can share a story");
    assert!(
        teller == eater || listeners.contains(&eater),
        "eating does not prevent participation"
    );
}
