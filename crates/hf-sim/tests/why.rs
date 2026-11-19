//! #292: the inspect panel says what someone feels and why their mood leans as it does.

use hf_sim::ids::TilePos;
use hf_sim::mind::feelings::FeelingKind as F;
use hf_sim::mind::ThoughtKind;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

#[test]
fn what_they_feel_and_why() {
    let mut w = World::generate(2921, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(2);
    let (i, other) = (
        w.agents.iter().position(|a| a.id == ids[0]).unwrap(),
        ids[1],
    );
    let t = w.meta.tick;
    w.agents[i]
        .feelings
        .feel(F::Anger, Some(other), 0.8, t, None);
    w.agents[i].thoughts.add(ThoughtKind::Quarrel, t, None);
    w.agents[i].thoughts.add(ThoughtKind::SleptRough, t, None);
    let name = w
        .agents
        .iter()
        .find(|a| a.id == other)
        .unwrap()
        .name
        .clone();
    let a = &w.agents[i];
    assert_eq!(
        hf_sim::view::why::feeling(&w, a),
        format!("angry with {name}")
    );
    let why = hf_sim::view::why::because(&w, a);
    assert!(why.contains("slept on the ground"), "{why}");
}
