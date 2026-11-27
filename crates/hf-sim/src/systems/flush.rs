//! Drain this tick's events. The chronicle writer and the client both read `last_events`.

use crate::world_state::World;

pub fn step(w: &mut World) {
    let drained = w.events.drain();
    if let Some(trace) = &mut w.why {
        trace.observe_failures(&drained);
    }
    w.last_events = drained;
}
