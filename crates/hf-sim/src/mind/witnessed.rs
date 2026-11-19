//! Witnessed accounts (#508, WP-07 7.5): what a person saw of another's manifestation, what
//! they made of it, and whether they will say. A witness forms their own explanation from
//! their own knowledge and beliefs — never the hidden truth of the potential. The account
//! can be taught on in talk (systems/beliefs.rs `share`), misread a little each time it
//! travels, or withheld entirely.

use serde::{Deserialize, Serialize};

use crate::agents::Agent;
use crate::ids::{AgentId, EventId, TilePos};
use crate::mind::beliefs::Source;
use crate::mind::feelings::{self, FeelingKind};
use crate::mind::techniques::Technique;
use crate::past::Topic;
use crate::world_state::World;

/// Accounts kept per person, newest last.
pub const WITNESSED_CAP: usize = 8;
/// Who is close enough to read a refusal at a face.
pub const FACE_RADIUS: i32 = 8;
/// The unsettled waking is quieter: fewer notice, and they hold it less surely.
pub const WAKING_RADIUS: i32 = 6;

/// What the witness saw. Bounded by observation: the event, never its cause.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Saw {
    /// A carrier would not cut a face (EventKind::WouldNotCut).
    RefusedTheCut,
    /// A carrier came back from the stone unsettled (EventKind::CameBackUnsettled).
    CameBackUnsettled,
}

/// The witness's own explanation, drawn from their beliefs and knowledge — the analogue of
/// the carrier's private `resonance::Interpretation`, one step removed from the event.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum WitnessRead {
    /// Their thin-place lore makes the moment read as the stone remembering.
    StoneRemembers,
    /// Their conviction that the deep should not be disturbed gives it a name.
    DeepWarns,
    /// They have no frame for it and keep only the unease.
    UnexplainedDread,
}

/// One witnessed account: who it was about, where, what was seen, what the witness makes of
/// it, and whether they will speak of it. `who` is the account's identity in the gossip chain
/// — teaching keeps the carrier's id, so the record shows where the story came from.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Witnessed {
    pub who: AgentId,
    pub at: TilePos,
    pub saw: Saw,
    pub read: WitnessRead,
    pub tick: u64,
    /// Whether this account may travel in talk (`share`). Withheld accounts stay out of the
    /// gossip chain (set at formation; a retelling sets it true on the listener's copy).
    pub tells: bool,
}

impl Witnessed {
    /// The account's identity for dedupe: the same carrier, the same kind of sighting.
    pub fn same_account(&self, other: &Witnessed) -> bool {
        self.who == other.who && self.saw == other.saw
    }
}

/// Keep an account, dropping the oldest past the cap.
pub fn remember(list: &mut Vec<Witnessed>, x: Witnessed) {
    if list.len() >= WITNESSED_CAP {
        list.remove(0);
    }
    list.push(x);
}

/// The explanation this witness will form, from what they already hold.
pub fn read_for(a: &Agent) -> WitnessRead {
    if a.techniques.knows(Technique::ThinLore) {
        WitnessRead::StoneRemembers
    } else if a.beliefs.lean(Topic::DeepDigging) < -0.2 {
        WitnessRead::DeepWarns
    } else {
        WitnessRead::UnexplainedDread
    }
}

/// Agent `k` has seen `carrier`'s manifestation (`saw`) at `at`. Form their account:
/// their own read of it, a feeling toward the carrier (awe if they hold the signs heeded,
/// fear otherwise), a nudge to their sign belief, and a move in their opinion. `strength`
/// is the base of the feeling (the waking passes a smaller one than a refusal).
///
/// Whether they will tell it is decided here: one who thinks well of the carrier protects
/// them, and one whose nature is too cautious to speak of it keeps it to themselves.
/// (FIXLOG 2026-11-15, #1834: the sign was flipped; the calmest witnesses were silenced
/// instead of the most cautious ones.)
pub fn form(
    w: &mut World,
    k: usize,
    carrier: AgentId,
    at: TilePos,
    saw: Saw,
    ev: EventId,
    strength: f32,
) {
    let tick = w.meta.tick;
    let read = read_for(&w.agents[k]);
    let protective = w.agents[k].opinion_of(carrier) > 0.5;
    let shaken = w.agents[k]
        .dispositions
        .map(|d| d.caution > 0.75)
        .unwrap_or(false);
    let tells = !(protective || shaken);
    let awe = w.agents[k].beliefs.lean(Topic::DeitySigns) > 0.2;
    let kind = if awe {
        FeelingKind::Awe
    } else {
        FeelingKind::Fear
    };
    let a = &mut w.agents[k];
    remember(
        &mut a.witnessed,
        Witnessed {
            who: carrier,
            at,
            saw,
            read,
            tick,
            tells,
        },
    );
    let felt = (strength * feelings::gain(a, kind)).min(1.0);
    a.feelings.feel(kind, Some(carrier), felt, tick, Some(ev));
    // Awe reads the moment as a sign heeded; fear rationalises it as only weather.
    let (toward, force) = if awe { (1.0, 0.1) } else { (-1.0, 0.08) };
    a.beliefs
        .experience(Topic::DeitySigns, toward, force, Source::Event(ev), tick);
    let delta = if awe { 0.1 } else { -0.12 };
    a.relate(carrier, delta, 0.05, tick);
}
