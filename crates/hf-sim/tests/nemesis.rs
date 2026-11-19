//! Rivals: a stranger who fought someone of the valley and lived comes back for them.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn pending(w: &World, f: impl Fn(&EventKind) -> bool) -> usize {
    w.events.pending.iter().filter(|e| f(&e.kind)).count()
}

#[test]
fn a_rival_is_made_comes_back_meets_their_foe_and_falls() {
    let mut w = World::generate(4401, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 12;
    w.rebuild_caches();
    let ids = w.found_party(3);
    w.meta.autoplan = false;
    w.ensure_nav();
    let si = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let foe = w.agents[si].id;
    let at = w.grid.surface(45, 45);
    w.agents[si].pos = at;
    // A stranger from place 7 fights them and lives.
    let raider = w.spawn_agent(hf_sim::agents::Species::Person, w.grid.surface(46, 45));
    let ri = w.agents.iter().position(|a| a.id == raider).unwrap();
    w.agents[ri].name = "Brask Holloway".into();
    hf_sim::nemesis::fought(&mut w, si, ri, 7);
    assert_eq!(w.rivals.len(), 1);
    assert_eq!(pending(&w, |k| matches!(k, EventKind::RivalMade { .. })), 1);
    // Gone home.
    w.agents[ri].alive = false;
    // Their people send others; the rival comes with them as the first.
    let newcomer = w.spawn_agent(hf_sim::agents::Species::Person, w.grid.surface(46, 45));
    hf_sim::nemesis::joins_band(&mut w, 7, &[newcomer]);
    let ni = w.agents.iter().position(|a| a.id == newcomer).unwrap();
    assert_eq!(w.agents[ni].name, "Brask Holloway");
    assert_eq!(
        pending(&w, |k| matches!(k, EventKind::RivalReturned { .. })),
        1
    );
    // They find their foe out of doors and have it out, a day on.
    w.meta.tick += TICKS_PER_DAY + 1;
    hf_sim::nemesis::seek(&mut w, &[newcomer], 7);
    assert_eq!(w.rivals[0].meetings, 2);
    assert_eq!(pending(&w, |k| matches!(k, EventKind::RivalMet { .. })), 1);
    assert!(!w.rivals[0].title.is_empty(), "a name earned");
    // And when they fall, it is on record.
    w.events.pending.clear();
    hf_sim::systems::life::die(&mut w, newcomer, "killed".into());
    w.last_events = w.events.pending.clone();
    hf_sim::nemesis::observe(&mut w);
    assert!(!w.rivals[0].alive);
    assert_eq!(pending(&w, |k| matches!(k, EventKind::RivalFell { .. })), 1);
    let _ = foe;
}
