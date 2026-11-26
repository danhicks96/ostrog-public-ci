//! Why it happened: from an event of the record back through the
//! causes the record holds. Several events name the event that caused them (a wound festered
//! because of a bite, the bite because vermin were drawn, a custom because of what it remembers);
//! the feed tells one step back, this page follows the chain. Read-only, and only through what the
//! valley may be told (`view::known`): an untold cause ends the chain, said as such.

use crate::chronicle::Chronicle;
use crate::events::{Event, EventKind};
use crate::ids::EventId;
use crate::world_state::World;

/// The event this one records as its cause, if it names one.
pub fn cause_of(k: &EventKind) -> Option<EventId> {
    use EventKind as K;
    match k {
        K::Possessed { cause, .. }
        | K::TrueDream { cause, .. }
        | K::Drenched { cause, .. }
        | K::VerminDrawn { cause, .. }
        | K::VerminBite { cause, .. }
        | K::Festered { cause, .. }
        | K::Weakened { cause, .. }
        | K::SickenedWeak { cause, .. }
        | K::RainSpoiled { cause, .. }
        | K::HealerLost { cause, .. }
        | K::WoundFestered { cause, .. } => Some(*cause),
        K::CustomFormed { origin, .. } | K::CustomAdopted { origin, .. } => Some(*origin),
        _ => None,
    }
}

/// The event of history row `history_id`, as recorded.
fn by_row(c: &Chronicle, history_id: i64) -> Option<Event> {
    let (tick, event_id, detail): (i64, i64, String) = c
        .conn
        .query_row(
            "SELECT tick, event_id, detail FROM real_history WHERE id = ?1",
            [history_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .ok()?;
    let mut e = crate::view::history::event_from_row(history_id, tick as u64, &detail)?;
    e.id = EventId(event_id as u32);
    Some(e)
}

/// Whether the event of history row `history_id` names a cause (for a "why?" link).
pub fn has_cause(c: &Chronicle, history_id: i64) -> bool {
    by_row(c, history_id).is_some_and(|e| cause_of(&e.kind).is_some())
}

/// Why a chain stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    /// The last event names no cause: the chain is whole.
    Root,
    /// The step limit was reached while a cause was still named.
    Limit,
    /// A cause is named but its row is not in the record.
    Missing,
    /// A cause is recorded but the valley may not be told of it.
    Untold,
}

/// The lines from the event of row `history_id` back through its recorded causes, as (tick,
/// line), at most `max` steps back, and why the chain stopped.
pub fn chain(w: &World, c: &Chronicle, history_id: i64, max: usize) -> (Vec<(u64, String)>, End) {
    let mut out = Vec::new();
    let Some(mut e) = by_row(c, history_id) else {
        return (out, End::Missing);
    };
    loop {
        out.push((e.tick, crate::view::narrate::line(w, &e)));
        let Some(cause) = cause_of(&e.kind) else {
            return (out, End::Root);
        };
        if out.len() > max {
            return (out, End::Limit);
        }
        let Ok(Some((_, tick, detail))) = c.row(cause) else {
            return (out, End::Missing);
        };
        let Some(mut next) = crate::view::history::event_from_row(0, tick, &detail) else {
            return (out, End::Missing);
        };
        next.id = cause;
        if !crate::view::known::told(w, &next) {
            return (out, End::Untold);
        }
        e = next;
    }
}

/// The Legends page `why:<history id>`.
pub fn page(w: &World, c: &Chronicle, history_id: i64) -> String {
    let (links, end) = chain(w, c, history_id, 6);
    let Some(((tick, first), rest)) = links.split_first() else {
        return "[b]Why it happened[/b]\nNothing is recorded here.".into();
    };
    let mut s = format!(
        "[b]Why it happened[/b]\n{}: {first}\n",
        crate::view::history::when(w, *tick)
    );
    for (tick, line) in rest {
        s += &format!(
            "  because, {}: {line}\n",
            crate::view::history::when(w, *tick)
        );
    }
    s += match (end, rest.is_empty()) {
        (End::Root, true) => "The record names no cause for it.\n",
        (End::Root, false) => "",
        (End::Limit, _) => "  and it goes back further than this page follows.\n",
        (End::Missing, _) => "  and the record of what caused that is missing.\n",
        (End::Untold, _) => "  and before that, nothing the colony knows.\n",
    };
    s
}
