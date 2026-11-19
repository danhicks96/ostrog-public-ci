//! E16 social knowledge: practical warnings are selective, sourced, and fallible.

use hf_sim::agents::Species;
use hf_sim::ids::{EventId, TilePos};
use hf_sim::mind::{Evidence, ProblemKind};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn setup() -> (World, usize, usize, TilePos) {
    let mut w = World::generate(119, GenParams::small());
    let p = w.meta.founding_site;
    let teller = w.spawn_agent(Species::Person, p);
    let listener = w.spawn_agent(Species::Person, p.offset(1, 0, 0));
    let ti = w.agents.iter().position(|a| a.id == teller).unwrap();
    let li = w.agents.iter().position(|a| a.id == listener).unwrap();
    (w, ti, li, p)
}

#[test]
fn trusted_people_share_personal_warnings_with_source() {
    let (mut w, ti, li, p) = setup();
    let teller = w.agents[ti].id;
    let listener = w.agents[li].id;
    w.agents[ti].relate(listener, 0.8, 0.8, 0);
    w.agents[li].relate(teller, 0.8, 0.8, 0);
    w.agents[ti].problems.observe(
        ProblemKind::Flooding,
        p,
        Evidence::Direct,
        0.9,
        1,
        Some(EventId(51)),
    );

    hf_sim::systems::beliefs::share(&mut w, ti, li, 2);

    let received = w.agents[li]
        .problems
        .strongest_open()
        .expect("the warning was received")
        .1;
    assert_eq!(received.evidence, Evidence::ToldBy(teller));
    assert_eq!(received.pos, p);
    assert_eq!(received.trigger, Some(EventId(51)));
}

#[test]
fn rivalry_distorts_without_revealing_deceptive_intent() {
    let (mut w, ti, li, p) = setup();
    let teller = w.agents[ti].id;
    let listener = w.agents[li].id;
    w.agents[ti].relate(listener, -0.8, 0.8, 0);
    w.agents[li].relate(teller, -0.8, 0.8, 0);
    w.agents[ti].problems.observe(
        ProblemKind::WaterHazard,
        p,
        Evidence::Direct,
        0.9,
        1,
        Some(EventId(52)),
    );

    hf_sim::systems::beliefs::share(&mut w, ti, li, 2);

    let received = w.agents[li].problems.strongest_open().unwrap().1;
    assert_eq!(received.evidence, Evidence::ToldBy(teller));
    assert_ne!(received.pos, p, "the rival gave a misleading location");
}

#[test]
fn distortion_at_the_right_edge_chooses_a_different_valid_tile() {
    let (mut w, ti, li, _) = setup();
    let teller = w.agents[ti].id;
    let listener = w.agents[li].id;
    let p = w.grid.surface(
        w.grid.width - 2,
        w.meta.founding_site.y.clamp(1, w.grid.height - 2),
    );
    w.agents[ti].relate(listener, -0.8, 0.8, 0);
    w.agents[li].relate(teller, -0.8, 0.8, 0);
    w.agents[ti].problems.observe(
        ProblemKind::WaterHazard,
        p,
        Evidence::Direct,
        0.9,
        1,
        Some(EventId(54)),
    );

    hf_sim::systems::beliefs::share(&mut w, ti, li, 2);

    let received = w.agents[li].problems.strongest_open().unwrap().1;
    assert_eq!(received.evidence, Evidence::ToldBy(teller));
    assert_ne!(
        received.pos, p,
        "the edge warning must not name its source tile"
    );
    assert!((1..w.grid.width - 1).contains(&received.pos.x));
    assert!((1..w.grid.height - 1).contains(&received.pos.y));
}

#[test]
fn neutral_people_can_withhold_without_creating_knowledge() {
    let (mut w, ti, li, p) = setup();
    let listener = w.agents[li].id;
    w.agents[ti].relate(listener, -0.4, 0.8, 0);
    w.agents[ti].problems.observe(
        ProblemKind::FoodSpoilage,
        p,
        Evidence::Direct,
        0.7,
        1,
        Some(EventId(53)),
    );

    hf_sim::systems::beliefs::share(&mut w, ti, li, 2);

    assert!(w.agents[li].problems.records.is_empty());
}
