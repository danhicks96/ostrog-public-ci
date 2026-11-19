//! What the music answers to besides strain (#290 item 2, WP-08 slice 8.8): a death among the
//! people of the valley is mourned (the melody falls silent for a while), and a birth, a band of
//! strangers welcomed or a caravan come home is welcomed (a resting score comes back early).
//! Read-only; the bridge passes only events the valley knows of (`view::known::told`), so the
//! music never answers a death nobody at the landing has heard of.

use crate::agents::Species;
use crate::events::{Event, EventKind};
use crate::world_state::World;

/// The music's answer to this event, if any: "mourn" or "welcome".
pub fn moment_of(w: &World, e: &Event) -> Option<&'static str> {
    match &e.kind {
        // Animals die and are hunted all the time; only a person's death is mourned.
        EventKind::Died { agent, .. } => w
            .agents
            .iter()
            .find(|a| a.id == *agent)
            .filter(|a| a.species == Species::Person)
            .map(|_| "mourn"),
        EventKind::Born { .. } | EventKind::Welcomed { .. } | EventKind::CaravanReturned { .. } => {
            Some("welcome")
        }
        _ => None,
    }
}
