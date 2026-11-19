//! Ancient fragments (#508, WP-07 7.6): the second kind of potential surfaces one bounded
//! piece of the old world — a glyph without its meaning, a step without its purpose — never
//! a system and never a reason. The woken carrier holds it; it can be taught in talk
//! (systems/beliefs.rs `share`), where familiarity blurs which one it was and every fifth
//! retelling loses that altogether, or be withheld, and it is lost when its last carrier
//! dies. Copies of one fragment that now disagree are both kept — a contested record, with
//! no arbiter. Nothing here is ever written to the event log: the world sees the murmuring,
//! not the content.

use serde::{Deserialize, Serialize};

use crate::ids::AgentId;

/// Keep at most this many fragments per person; oldest drops.
pub const FRAGMENT_CAP: usize = 8;
/// Marks each kind carries; distortion walks inside these.
pub const SYMBOLS: u8 = 6;
pub const PROCEDURE_STEPS: u8 = 6;
pub const WRONG_PLACES: u8 = 8;
pub const WANTED_MATERIALS: u8 = 6;

/// One bounded fragment. Each kind stops short of a mechanism: what it is, never how or why.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Fragment {
    /// One glyph of the old world, meaningless alone.
    SawSymbol,
    /// One step of a rite or working, without its purpose.
    RemembersProcedure,
    /// A place reads wrong — dangerous, holy, hollow — with no reason given.
    FeelsPlace,
    /// A material wants to belong to a device they cannot conceive.
    BelongsInMachine,
}

/// How the fragment was come by.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Learned {
    /// Woken at the stone in the carrier who carries it.
    Woke,
    /// Taught in talk by someone who held it.
    Taught,
}

/// One held fragment. `origin` is its identity down the gossip chain: `None` on the woken
/// original (then the identity is its holder and the tick it woke), `Some((holder, tick))`
/// on every taught copy. The identity does not drift, so two copies of one fragment that
/// now say different things stand side by side — that is a contested record.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct HeldFragment {
    pub what: Fragment,
    /// Which one of the kind, drawn at the waking; `None` once retellings lose it.
    pub mark: Option<u8>,
    /// The woken original this copy descends from; `None` means this IS the original.
    pub origin: Option<(AgentId, u64)>,
    pub learned: Learned,
    pub tick: u64,
    /// Whether the holder will teach it in talk (`share`).
    pub tells: bool,
}

impl HeldFragment {
    /// The identity the copy chain dedupes on: its origin if it has one, else its holder
    /// and the tick it woke.
    pub fn identity(&self, holder: AgentId) -> (AgentId, u64) {
        self.origin.unwrap_or((holder, self.tick))
    }

    /// How many marks this kind carries (the drawn detail's range).
    pub fn span(what: Fragment) -> u8 {
        match what {
            Fragment::SawSymbol => SYMBOLS,
            Fragment::RemembersProcedure => PROCEDURE_STEPS,
            Fragment::FeelsPlace => WRONG_PLACES,
            Fragment::BelongsInMachine => WANTED_MATERIALS,
        }
    }
}

/// Keep a fragment, dropping the oldest past the cap.
pub fn remember(list: &mut Vec<HeldFragment>, x: HeldFragment) {
    if list.len() >= FRAGMENT_CAP {
        list.remove(0);
    }
    list.push(x);
}
