//! What a person was told, by whom, and what they made of it (#487).
//!
//! Know-how passes person to person in talk: a crisis technique explained, a look-alike plant
//! warned of. Whether it passes depends on both sides. The teller must know it well enough to
//! explain it and be willing: someone they think ill of, or are at feud with, is not told unless
//! they are unusually warm. The listener must believe the teller: trust is what they think of the
//! teller and how well they know them, and a self-reliant listener needs more of it before they
//! take another's word. A told technique can also be misheard: they believe they know it, but
//! their hands do not. Every telling, taken, refused or misheard, is kept with its teller, so
//! the record shows where a person's knowledge came from and what they would not take.

use serde::{Deserialize, Serialize};

use crate::ids::AgentId;
use crate::mind::techniques::Technique;
use crate::plants::PlantKind;

/// Tellings kept per person, newest last.
pub const TOLD_CAP: usize = 12;
/// A refused telling is not offered again by the same teller for this long.
pub const RETELL_DAYS: u64 = 5;
/// How well a teller must know a technique to explain it.
pub const EXPLAIN_SKILL: f32 = 0.3;
/// The skill a technique learned by being told starts at; watching gives 0.15 (learn.rs).
pub const TOLD_SKILL: f32 = 0.1;
/// The skill a misheard technique starts at: they think they know it.
pub const MISHEARD_SKILL: f32 = 0.03;
/// The chance a told technique is misheard.
pub const MISHEAR: f32 = 0.1;
/// The trust a listener must place in the teller to take their word.
pub const BELIEVE: f32 = 0.45;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum What {
    Technique(Technique),
    LookAlike(PlantKind),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Took {
    Taken,
    /// They did not believe the teller.
    Refused,
    /// They took it, but wrongly.
    Misheard,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Told {
    pub what: What,
    pub from: AgentId,
    pub tick: u64,
    /// The trust they placed in the teller when told.
    pub trust: f32,
    pub took: Took,
}

impl What {
    pub fn words(self) -> String {
        match self {
            What::Technique(t) => t.name().to_string(),
            What::LookAlike(k) => format!("which {} is the bad one", k.label()),
        }
    }
}

/// The trust `listener` places in `teller`'s word, 0..: opinion and familiarity raise it, a
/// self-reliant bent asks more of it (the threshold is `BELIEVE`).
pub fn trust(listener: &crate::agents::Agent, teller: AgentId) -> f32 {
    let (opinion, fam) = listener
        .relationships
        .get(&teller)
        .map(|r| (r.opinion, r.familiarity))
        .unwrap_or((0.0, 0.0));
    let independence = listener
        .dispositions
        .map(|d| d.independence.clamp(0.0, 1.0))
        .unwrap_or(0.5);
    ((0.5 + 0.6 * opinion + 0.3 * fam) * (1.2 - 0.6 * independence)).max(0.0)
}

/// Whether `teller` is willing to tell `listener` anything: not one they think ill of, unless
/// they are unusually warm. `feud` is whether their families are at feud.
pub fn willing(teller: &crate::agents::Agent, listener: AgentId, feud: bool) -> bool {
    let warm = teller.dispositions.map(|d| d.warmth).unwrap_or(0.5) >= 0.75;
    let cold = teller.opinion_of(listener) < -0.3 || feud;
    !cold || warm
}

/// Keep a telling, dropping the oldest past the cap.
pub fn remember(told: &mut Vec<Told>, t: Told) {
    if told.len() >= TOLD_CAP {
        told.remove(0);
    }
    told.push(t);
}

/// Whether `from` told `what` lately and was refused: not offered again yet.
pub fn refused_lately(told: &[Told], what: What, from: AgentId, now: u64) -> bool {
    told.iter().any(|t| {
        t.what == what
            && t.from == from
            && t.took == Took::Refused
            && now.saturating_sub(t.tick) < RETELL_DAYS * crate::time::TICKS_PER_DAY
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_record_is_bounded() {
        let mut v = Vec::new();
        for k in 0..20 {
            remember(
                &mut v,
                Told {
                    what: What::Technique(Technique::ForageFar),
                    from: AgentId(1),
                    tick: k,
                    trust: 0.5,
                    took: Took::Refused,
                },
            );
        }
        assert_eq!(v.len(), TOLD_CAP);
        assert_eq!(v[0].tick, 8);
        assert!(refused_lately(
            &v,
            What::Technique(Technique::ForageFar),
            AgentId(1),
            20
        ));
        assert!(!refused_lately(
            &v,
            What::Technique(Technique::ForageFar),
            AgentId(2),
            20
        ));
    }
}
