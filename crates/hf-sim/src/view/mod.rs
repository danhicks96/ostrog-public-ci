//! Viewer-facing read models: templated narration (never invents), the camera director,
//! and the away summary. Transport-agnostic; the GDExtension and the CLI both use them.

pub mod alive;
pub mod away;
pub mod biography;
pub mod block;
pub mod bodies;
pub mod built;
pub mod camera_miss;
pub mod day_shape;
pub mod director;
pub mod experience;
pub mod fold;
pub mod history;
pub mod knowing;
pub mod known;
pub mod known_world;
pub mod legends;
pub mod look;
pub mod music;
pub mod narrate;
pub mod summary;
pub mod waiting;
pub mod why;

/// The sound the client plays for an event it has been told, if any: a fire put out, a fire
/// starting, and a strike of lightning, which also flashes the sky (#184, QA 2026-10-20, C63:
/// the grade shader's flash was never set).
pub fn sound_of(kind: &crate::events::EventKind) -> Option<&'static str> {
    use crate::events::EventKind;
    match kind {
        EventKind::Doused { .. } => Some("douse"),
        EventKind::FireStarted { cause, .. } if cause == "by lightning" => Some("lightning"),
        EventKind::FireStarted { .. } => Some("flare"),
        _ => None,
    }
}
