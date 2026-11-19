//! Hidden arrival changes (#308, epic #508; docs/design-magic-psy-tech.md): some of those who came
//! through the portal carry something from it that nobody knows of, not even them.
//!
//! When one of the ten is made, a potential is drawn for them once, from the seed and their slot
//! (`draw`): about two in ten carry one — the rock-sense or, since 7.6, the ancient fragment.
//! It is
//! saved with them and shown nowhere. It wakes only at the stone: a carrier who has sat by the
//! woken portal stone four times (thin.rs `study`) comes back from it unsettled and tired, and
//! the record says no more than that. Woken, at a mining face whose cut would leave the roof
//! unheld (underground.rs `unheld_if_opened`, the test that brings a roof down), one who does
//! not know shoring stops and will not cut, and cannot say why; it tires them, and the place is
//! a bad one to them. The cut is left for someone else. One who knows shoring props first, as
//! anyone who knows it does, and the sense adds nothing.
//!
//! It costs and it fails (WP-07 7.4): every time it stops them it tires them, and it does not come
//! to one worn out (`FAILS_BELOW` rest). A tired carrier cuts the same face like anyone who does
//! not know shoring, with the same chance of bringing the roof down: the sense that held them back
//! in the morning is not there by evening. Sleep brings it back.

use serde::{Deserialize, Serialize};

use crate::events::{EventKind, Severity};
use crate::ids::TilePos;
use crate::mind::fragment::{Fragment, HeldFragment, Learned};
use crate::mind::{MemKind, Subject, Technique};
use crate::past::Topic;
use crate::rng::{purpose, stream};
use crate::world_state::World;
use rand::Rng;

/// The share of the arrivals who carry a potential (epic #508: two in ten).
pub const CARRY: f64 = 0.2;
/// Sittings by the woken stone that wake it.
pub const WAKES_AFTER: u32 = 4;
/// Below this rest the sense does not come (7.4): worn out, they feel nothing at the face.
pub const FAILS_BELOW: f32 = 0.5;
/// What one firing costs in rest.
pub const COST: f32 = 0.1;

/// What someone carries. Two so far (7.6 widened the catalogue with the fragment).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Potential {
    /// A dread at rock about to come down (an ancient resonance: a feeling, never a reason).
    RockSense,
    /// They wake from the stone with one bounded piece of the old world (a fragment,
    /// mind/fragment.rs; never a system, never a reason).
    AncientFragment,
}

/// What the carrier thinks happened the first time the sense stops them. This is deliberately
/// personal and uncertain: it is not a public diagnosis of the hidden potential.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Interpretation {
    /// Their own thin-place lore makes the stone feel like a warning from the old world.
    StoneRemembers,
    /// Their prior conviction that the deep should not be disturbed gives the event a name.
    DeepShouldNotBeDug,
    /// They have no relevant knowledge and keep only the frightening sensation.
    UnexplainedDread,
}

/// The first manifestation is an ambiguous event in the world and a private interpretation in
/// the person who experienced it. The event itself remains `WouldNotCut`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifestation {
    pub at: TilePos,
    pub interpretation: Interpretation,
}

/// A carrier's hidden state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Resonance {
    pub potential: Potential,
    /// Sittings by the woken stone so far.
    pub sat: u32,
    /// When it woke.
    pub woke: Option<u64>,
    /// Faces they would not cut.
    pub refused: u32,
    /// The first ambiguous manifestation, kept in this person's mind and nowhere public.
    #[serde(default)]
    pub first_manifestation: Option<Manifestation>,
}

/// The potential one of the ten carries, drawn once from the seed and their slot (and its
/// rerolls, so a rerolled slot is a different person); never drawn again.
pub fn draw(seed: u64, index: u8, rerolls: u32) -> Option<Resonance> {
    let mut r = stream(seed, &[purpose::RESONANCE, index as u64, rerolls as u64]);
    if !r.gen_bool(CARRY) {
        return None;
    }
    // 7.6: a third of the drawn carriers carry the fragment rather than the rock-sense.
    // Drawn after the carrier roll, so this changes nothing for who carries; saved
    // potentials are stored and never drawn again.
    let potential = if r.gen_bool(0.34) {
        Potential::AncientFragment
    } else {
        Potential::RockSense
    };
    Some(Resonance {
        potential,
        sat: 0,
        woke: None,
        refused: 0,
        first_manifestation: None,
    })
}

fn interpretation(a: &crate::agents::Agent) -> Interpretation {
    if a.techniques.knows(Technique::ThinLore) {
        Interpretation::StoneRemembers
    } else if a.beliefs.lean(Topic::DeepDigging) < -0.2 {
        Interpretation::DeepShouldNotBeDug
    } else {
        Interpretation::UnexplainedDread
    }
}

/// Agent `i` has sat an hour by the woken stone (thin.rs `study`).
pub fn sat_by_stone(w: &mut World, i: usize) {
    let tick = w.meta.tick;
    let Some(res) = w.agents[i].resonance.as_mut() else {
        return;
    };
    if res.woke.is_some() {
        return;
    }
    res.sat += 1;
    if res.sat < WAKES_AFTER {
        return;
    }
    res.woke = Some(tick);
    // The waking is quiet: they come back tired and unsettled, and nobody can say more.
    let a = &mut w.agents[i];
    a.needs.rest = (a.needs.rest - 0.3).max(0.0);
    let (agent, at) = (a.id, a.pos);
    let ev = w.events.push(
        tick,
        Some(at),
        Severity::Notable,
        EventKind::CameBackUnsettled { agent },
    );
    // 7.5: the quiet waking is still seen by those near the stone, and held the same way.
    // #778: near is not enough — a parallel gallery behind solid rock does not witness;
    // they must be able to see the spot (FIXLOG 2026-11-13).
    let witnesses = crate::systems::beliefs::near(w, at, crate::mind::witnessed::WAKING_RADIUS);
    for k in witnesses {
        if k == i || !crate::mind::perception::can_see(&w.grid, w.agents[k].pos, at) {
            continue;
        }
        crate::mind::witnessed::form(
            w,
            k,
            agent,
            at,
            crate::mind::witnessed::Saw::CameBackUnsettled,
            ev,
            0.35,
        );
    }
    // 7.6: a fragment potential wakes as exactly one bounded piece of the old world. The
    // draw is from the seed, the carrier and the woke tick, stored on them, never rerolled;
    // the world sees only the unsettled waking above. The very cautious keep it to themselves.
    if w.agents[i]
        .resonance
        .as_ref()
        .is_some_and(|r| r.potential == Potential::AncientFragment)
    {
        let agent = w.agents[i].id;
        let cautious = w.agents[i]
            .dispositions
            .map(|d| d.caution > 0.75)
            .unwrap_or(false);
        let mut r = stream(
            w.meta.seed,
            &[purpose::RESONANCE_FRAGMENT, agent.0 as u64, tick],
        );
        let what = match r.gen_range(0..4) {
            0 => Fragment::SawSymbol,
            1 => Fragment::RemembersProcedure,
            2 => Fragment::FeelsPlace,
            _ => Fragment::BelongsInMachine,
        };
        let mark = Some(r.gen_range(0..HeldFragment::span(what)));
        crate::mind::fragment::remember(
            &mut w.agents[i].fragments,
            HeldFragment {
                what,
                mark,
                origin: None,
                learned: Learned::Woke,
                tick,
                tells: !cautious,
            },
        );
    }
}

/// Agent `i` is about to cut the face at `pos`, which would leave `unheld` unheld. Whether the
/// rock-sense stops them; if it does, the dread is theirs and on record.
pub fn balks(w: &mut World, i: usize, pos: TilePos, unheld: &[TilePos]) -> bool {
    let tick = w.meta.tick;
    let personal_interpretation = interpretation(&w.agents[i]);
    if unheld.is_empty()
        || w.agents[i]
            .techniques
            .knows(crate::mind::Technique::Shoring)
    {
        return false;
    }
    let Some(res) = w.agents[i].resonance.as_mut() else {
        return false;
    };
    if res.woke.is_none() || res.potential != Potential::RockSense {
        return false;
    }
    // Worn out, they feel nothing (7.4): the cut goes ahead as anyone's would.
    if w.agents[i].needs.rest < FAILS_BELOW {
        return false;
    }
    let Some(res) = w.agents[i].resonance.as_mut() else {
        return false;
    };
    if res.first_manifestation.is_none() {
        res.first_manifestation = Some(Manifestation {
            at: pos,
            interpretation: personal_interpretation,
        });
    }
    res.refused += 1;
    let first = res.refused == 1;
    let a = &mut w.agents[i];
    a.needs.rest = (a.needs.rest - COST).max(0.0);
    let agent = a.id;
    let ev = w.events.push(
        tick,
        Some(pos),
        if first {
            Severity::Notable
        } else {
            Severity::Minor
        },
        EventKind::WouldNotCut { agent, at: pos },
    );
    w.agents[i].remember(
        tick,
        MemKind::Hardship,
        Subject::Place(pos),
        -0.5,
        0.6,
        Some(ev),
    );
    // 7.5: those present read the refusal their own way and hold it, told or withheld.
    // #778: present means able to see the face, not merely within radius (FIXLOG 2026-11-13).
    let witnesses = crate::systems::beliefs::near(w, pos, crate::mind::witnessed::FACE_RADIUS);
    for k in witnesses {
        if k == i || !crate::mind::perception::can_see(&w.grid, w.agents[k].pos, pos) {
            continue;
        }
        crate::mind::witnessed::form(
            w,
            k,
            agent,
            pos,
            crate::mind::witnessed::Saw::RefusedTheCut,
            ev,
            0.6,
        );
    }
    true
}
