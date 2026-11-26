//! OST-1101: an awake person with the social need run low carries a Lonely thought, which
//! clears after the need recovers; a sleeper does not carry it.

use hf_sim::ids::TilePos;
use hf_sim::mind::{Activity, ThoughtKind};
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn pair() -> (World, usize, usize) {
    let mut w = World::generate(1101, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 9;
    let ids = w.found_party(2);
    w.meta.autoplan = false;
    let pos = |id| w.agents.iter().position(|a| a.id == id).unwrap();
    let (a, b) = (pos(ids[0]), pos(ids[1]));
    (w, a, b)
}

/// Strain runs once an hour at a fixed offset: move to the next such tick, `hours` later.
fn to_strain_tick(w: &mut World, hours: u64) {
    let h = TICKS_PER_HOUR;
    w.meta.tick = w.meta.tick - w.meta.tick % h + h + 900 + hours * h;
}

#[test]
fn low_social_is_lonely_when_awake_and_clears_on_recovery() {
    let (mut w, a, b) = pair();
    w.agents[a].needs.social = 0.05;
    w.agents[b].needs.social = 1.0;
    to_strain_tick(&mut w, 0);
    hf_sim::systems::strain::step(&mut w);
    assert!(
        w.agents[a].thoughts.has(ThoughtKind::Lonely),
        "lonely when starved of company"
    );
    assert!(
        !w.agents[b].thoughts.has(ThoughtKind::Lonely),
        "content person is not"
    );
    w.agents[a].needs.social = 1.0;
    to_strain_tick(&mut w, 24);
    hf_sim::systems::strain::step(&mut w);
    assert!(
        !w.agents[a].thoughts.has(ThoughtKind::Lonely),
        "clears after recovery"
    );
}

#[test]
fn a_sleeper_is_not_lonely() {
    let (mut w, a, _) = pair();
    w.agents[a].needs.social = 0.05;
    w.agents[a].activity = Activity::Sleeping {
        bed: None,
        until: u64::MAX,
    };
    to_strain_tick(&mut w, 0);
    hf_sim::systems::strain::step(&mut w);
    assert!(!w.agents[a].thoughts.has(ThoughtKind::Lonely));
}
