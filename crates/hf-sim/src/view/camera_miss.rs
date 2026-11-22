//! The headless caller for the camera-miss measurement (OST-004): steps a world, feeds the camera
//! director the events a viewer would be told of, samples what it suggests at a fixed stride, and
//! hands both to `AliveTally::observe_major_events` and `AliveTally::observe_glance`.
//!
//! Read-only (D7): the director and the tally only read the world, use no rng and no wall clock,
//! so a world stepped through `measure` ends where one stepped plainly does (checked by digest in
//! `tests/director.rs`). This measures what the camera missed; it does not change how the camera
//! scores anything.

use crate::agents::Species;
use crate::events::{Event, EventKind};
use crate::view::alive::AliveTally;
use crate::view::director::{people, settlement_centre, Director};
use crate::view::known::told;
use crate::world_state::World;

/// Ticks between camera samples: 600, half a game hour, so an event gets two samples inside
/// its hour (`GLANCE_MAJOR_WINDOW` is 1,200 ticks).
pub const SAMPLE_STRIDE: u64 = 600;

/// Whether the director is shown this event: the filter the game client applies before it hands
/// events to the director. An event after the arrival that the viewer may be told of, and not the
/// death of a wild animal nobody keeps.
pub fn shown_to_director(w: &World, e: &Event) -> bool {
    if e.tick < w.meta.epoch || !told(w, e) {
        return false;
    }
    let EventKind::Died { agent, .. } = &e.kind else {
        return true;
    };
    !w.agents
        .iter()
        .find(|a| a.id == *agent)
        .is_some_and(|a| a.species != Species::Person && a.kept.is_none())
}

/// Step `w` for `ticks` ticks, sampling the director every `stride` ticks (a stride of 0 is
/// read as `SAMPLE_STRIDE`), and return the tally. Events still inside their hour when the run
/// ends are not counted: see `AliveTally::glance_major_pending`.
pub fn measure(w: &mut World, ticks: u64, stride: u64) -> AliveTally {
    let stride = if stride == 0 { SAMPLE_STRIDE } else { stride };
    let mut tally = AliveTally::default();
    let mut director = Director::default();
    for _ in 0..ticks {
        w.step();
        let tick = w.tick();
        let events: Vec<Event> = w
            .last_events
            .iter()
            .filter(|e| shown_to_director(w, e))
            .cloned()
            .collect();
        tally.observe_major_events(&events);
        director.observe(tick, &events);
        if tick.is_multiple_of(stride) {
            let s = director.suggest(tick, settlement_centre(w), &people(w));
            tally.observe_glance(tick, s.at);
        }
    }
    tally
}
