//! #809: a wild creature's death is Minor (seen) or Trace (unseen), never Major; a person's or
//! a kept animal's death stays Major.

use hf_sim::agents::Species;
use hf_sim::events::{EventKind, Severity};
use hf_sim::ids::TilePos;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn world() -> World {
    let mut w = World::generate(777, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    w
}

fn severity_of_death(w: &World, agent: hf_sim::ids::AgentId) -> Option<Severity> {
    w.events.pending.iter().find_map(|e| match e.kind {
        EventKind::Died { agent: a, .. } if a == agent => Some(e.severity),
        _ => None,
    })
}

#[test]
fn an_unwitnessed_wild_death_is_trace_a_witnessed_one_is_minor_a_person_is_major() {
    let mut w = world();
    let ids = w.found_party(1);
    let person = ids[0];
    // Move the person far from where the first wild creature will die, so it is unwitnessed.
    if let Some(a) = w.agents.iter_mut().find(|a| a.id == person) {
        a.pos = TilePos::new(5, 5, 2);
    }

    let far = w.spawn_agent(Species::Bat, TilePos::new(55, 55, 2));
    hf_sim::systems::life::die(&mut w, far, "engulfed by slime".into());
    assert_eq!(severity_of_death(&w, far), Some(Severity::Trace));

    let seen = w.spawn_agent(Species::Bat, TilePos::new(5, 6, 2));
    hf_sim::systems::life::die(&mut w, seen, "engulfed by slime".into());
    assert_eq!(severity_of_death(&w, seen), Some(Severity::Minor));

    hf_sim::systems::life::die(&mut w, person, "old age".into());
    assert_eq!(severity_of_death(&w, person), Some(Severity::Major));
}
