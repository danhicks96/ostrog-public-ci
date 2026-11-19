//! #319: the first chaos cascade family, causally recorded (docs/design-emergence.md):
//! a flood drowns a stockpile, the drenched grain rots, the rot draws vermin, the vermin
//! bite a sleeper, the bite festers. Every step holds the previous step's event row — the
//! conditions.rs cause machinery, so the chronicle can trace the whole spiral (D9: history
//! is recorded, never invented). No step fires without the recorded row before it.

use hf_sim::body::WoundKind;
use hf_sim::events::{EventKind, Severity};
use hf_sim::ids::{EventId, TilePos};
use hf_sim::items::ItemKind;
use hf_sim::mind::Activity;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::world::TileFlags;
use hf_sim::World;

/// A flat meadow, one person asleep by a stockpile tile.
fn valley(seed: u64) -> (World, usize, TilePos) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    let store = TilePos::new(32, 30, 2);
    let who = w.spawn_agent(hf_sim::agents::Species::Person, store.offset(2, 0, 0));
    let i = w.agents.iter().position(|a| a.id == who).unwrap();
    w.agents[i].activity = Activity::Sleeping {
        bed: None,
        until: w.meta.tick + TICKS_PER_DAY,
    };
    (w, i, store)
}

/// Cover `at` with flood water, as floods.rs `rise` does, and record the flood's row.
fn flood(w: &mut World, at: TilePos) -> EventId {
    let mut t = w.grid.get(at);
    t.flags = t.flags.with(TileFlags::WATER);
    w.grid.set(at, t);
    w.flooded.insert(at);
    let tick = w.meta.tick;
    let ev = w.events.push(
        tick,
        Some(at),
        Severity::Major,
        EventKind::Flooded { tiles: 1 },
    );
    w.waters.flood_event = Some(ev);
    ev
}

/// Set the clock to night `d`, 21:53, and run the cascade step.
fn night(w: &mut World, d: u64) {
    w.meta.tick = d * TICKS_PER_DAY + 21 * TICKS_PER_HOUR + 53;
    hf_sim::cascade::step(w);
}

fn find(w: &World, f: impl Fn(&EventKind) -> bool) -> Option<EventId> {
    w.events.pending.iter().find(|e| f(&e.kind)).map(|e| e.id)
}

fn cause_of(w: &World, id: EventId) -> EventId {
    let e = w.events.pending.iter().find(|e| e.id == id).unwrap();
    match &e.kind {
        EventKind::Drenched { cause, .. }
        | EventKind::VerminDrawn { cause, .. }
        | EventKind::VerminBite { cause, .. }
        | EventKind::Festered { cause, .. } => *cause,
        _ => panic!("event {id:?} holds no cascade cause"),
    }
}

#[test]
fn a_flood_cascades_to_rot_vermin_bite_and_fester() {
    let (mut w, sleeper, store) = valley(7);
    w.drop_item(ItemKind::GrainSheaf, 40, store);
    let flood = flood(&mut w, store);
    // Night one: the grain under water rots, and the rot draws vermin.
    night(&mut w, 1);
    let drench = find(
        &w,
        |k| matches!(k, EventKind::Drenched { at, .. } if *at == store),
    )
    .expect("the drenched grain rotted with a recorded row");
    assert_eq!(cause_of(&w, drench), flood, "the rot holds the flood's row");
    let vermin = find(
        &w,
        |k| matches!(k, EventKind::VerminDrawn { at, .. } if *at == store),
    )
    .expect("the rot drew vermin");
    assert_eq!(
        cause_of(&w, vermin),
        drench,
        "the vermin hold the rot's row"
    );
    assert!(
        w.vermin.iter().any(|v| v.at == store),
        "an outbreak is tracked at the stockpile"
    );
    assert!(
        w.items
            .iter()
            .all(|it| it.kind != ItemKind::GrainSheaf || it.count < 40),
        "the flooded stack lost a share to the rot"
    );
    // The nights after: the vermin bite the sleeper, and the bite festers.
    let mut bite = None;
    for d in 2..14 {
        night(&mut w, d);
        if let Some(b) = find(&w, |k| matches!(k, EventKind::VerminBite { .. })) {
            bite = Some(b);
            break;
        }
    }
    let bite = bite.expect("the vermin bit the sleeper within a fortnight");
    assert_eq!(
        cause_of(&w, bite),
        vermin,
        "the bite holds the vermin's row"
    );
    let festered =
        find(&w, |k| matches!(k, EventKind::Festered { .. })).expect("the bite festered, recorded");
    assert_eq!(
        cause_of(&w, festered),
        bite,
        "the fester holds the bite's row"
    );
    let a = &w.agents[sleeper];
    assert!(
        a.body
            .wounds
            .iter()
            .any(|wd| wd.kind == WoundKind::Puncture && wd.infected),
        "the sleeper carries the festering bite"
    );
}

#[test]
fn no_recorded_flood_no_cascade() {
    // Flood water with no recorded flood row: nothing is taken by chance alone (D9).
    let (mut w, _i, store) = valley(11);
    w.drop_item(ItemKind::GrainSheaf, 40, store);
    let mut t = w.grid.get(store);
    t.flags = t.flags.with(TileFlags::WATER);
    w.grid.set(store, t);
    w.flooded.insert(store);
    assert!(w.waters.flood_event.is_none());
    night(&mut w, 1);
    assert!(
        find(&w, |k| matches!(k, EventKind::Drenched { .. })).is_none(),
        "no flood row, no recorded rot"
    );
    assert!(w.vermin.is_empty(), "no rot row, no vermin");
    assert!(
        w.items
            .iter()
            .any(|it| it.kind == ItemKind::GrainSheaf && it.count == 40),
        "the grain is untouched"
    );
}

#[test]
fn the_cascade_is_deterministic() {
    fn run() -> (EventId, EventId, EventId, EventId) {
        let (mut w, _sleeper, store) = valley(7);
        w.drop_item(ItemKind::GrainSheaf, 40, store);
        let flood = flood(&mut w, store);
        night(&mut w, 1);
        let drench = find(&w, |k| matches!(k, EventKind::Drenched { .. })).unwrap();
        let vermin = find(&w, |k| matches!(k, EventKind::VerminDrawn { .. })).unwrap();
        let mut bite = None;
        for d in 2..14 {
            night(&mut w, d);
            if let Some(b) = find(&w, |k| matches!(k, EventKind::VerminBite { .. })) {
                bite = Some(b);
                break;
            }
        }
        (flood, drench, vermin, bite.unwrap())
    }
    assert_eq!(run(), run(), "same seed, same cascade rows");
}
