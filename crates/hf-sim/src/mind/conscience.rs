//! Judging an outcome afterwards: regret and rationalising (#568).
//!
//! After a quarrel a person does not yet know whether the other was in the wrong. If they later
//! see that the other was in fact hurt, sick or grieving, they judge what they did. Some regret
//! it: they think better of the other and are readier to help them. Others rationalise it: they
//! tell themselves the other had it coming, think a little worse of them and are slower to help.
//! Which way a person goes is their own (`regret_tendency`), not a rule for everyone.
//!
//! Every judgement is kept with the quarrel it judges (its event id) and what changed in the
//! person. The change to how they weigh helping that person is a multiplier between
//! `HELP_MIN` and `HELP_MAX` that fades back to 1 over `LEANING_DAYS`, so a judgement can
//! neither run away nor last for ever.

use serde::{Deserialize, Serialize};

use crate::ids::{AgentId, EventId};
use crate::time::TICKS_PER_DAY;

/// Judgements kept, newest last.
pub const JUDGEMENT_CAP: usize = 8;
/// People a person leans toward or away from helping at once.
pub const LEANING_CAP: usize = 8;
/// How long after a quarrel what is seen of the other can still be judged against it.
pub const JUDGE_WITHIN: u64 = TICKS_PER_DAY;
/// How long a leaning takes to fade back to nothing.
pub const LEANING_DAYS: u64 = 10;
/// The bounds of the helping multiplier, however many judgements stack up.
pub const HELP_MIN: f32 = 0.7;
pub const HELP_MAX: f32 = 1.4;
/// What one judgement does to the helping multiplier.
pub const REGRET_HELP: f32 = 1.3;
pub const RATIONALISE_HELP: f32 = 0.8;
/// What one judgement does to their opinion of the other.
pub const REGRET_OPINION: f32 = 0.12;
pub const RATIONALISE_OPINION: f32 = -0.05;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Stance {
    Regret,
    Rationalise,
}

/// What the other turned out to be when they were seen afterwards.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Found {
    Hurt,
    Sick,
    Grieving,
}

impl Found {
    pub fn words(self) -> &'static str {
        match self {
            Found::Hurt => "hurt",
            Found::Sick => "sick",
            Found::Grieving => "grieving",
        }
    }
}

/// One choice judged afterwards, linked to the event that recorded it.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Judgement {
    /// When the quarrel was.
    pub at: u64,
    /// The quarrel's event, the cause this judges.
    pub cause: Option<EventId>,
    pub other: AgentId,
    /// When it was judged, and how; `None` while still to be judged.
    pub judged: Option<(u64, Stance, Found)>,
    /// What it changed: their opinion of the other, and the helping multiplier after it.
    pub opinion: f32,
    pub help: f32,
}

/// A fading lean toward or away from helping one person.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Leaning {
    pub toward: AgentId,
    /// The multiplier when set, `HELP_MIN..=HELP_MAX`.
    pub help: f32,
    pub since: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Conscience {
    pub judgements: Vec<Judgement>,
    pub leanings: Vec<Leaning>,
}

impl Conscience {
    /// A quarrel to judge once they see how the other really was.
    pub fn quarrelled(&mut self, other: AgentId, cause: Option<EventId>, tick: u64) {
        if self
            .judgements
            .iter()
            .any(|j| j.other == other && j.judged.is_none())
        {
            return;
        }
        if self.judgements.len() >= JUDGEMENT_CAP {
            // The oldest judged one goes first; failing that the oldest.
            let k = self
                .judgements
                .iter()
                .position(|j| j.judged.is_some())
                .unwrap_or(0);
            self.judgements.remove(k);
        }
        self.judgements.push(Judgement {
            at: tick,
            cause,
            other,
            judged: None,
            opinion: 0.0,
            help: 1.0,
        });
    }

    /// The helping multiplier toward `who` now: the lean set, faded by its age.
    pub fn help_toward(&self, who: AgentId, now: u64) -> f32 {
        self.leanings
            .iter()
            .find(|l| l.toward == who)
            .map(|l| {
                let age =
                    now.saturating_sub(l.since) as f32 / (LEANING_DAYS * TICKS_PER_DAY) as f32;
                1.0 + (l.help - 1.0) * (1.0 - age).max(0.0)
            })
            .unwrap_or(1.0)
    }

    /// Judge the pending quarrel with `other`: set the lean from where it stands now, bounded.
    /// Returns the new multiplier.
    pub fn judge(
        &mut self,
        other: AgentId,
        stance: Stance,
        found: Found,
        opinion: f32,
        now: u64,
    ) -> f32 {
        let current = self.help_toward(other, now);
        let factor = match stance {
            Stance::Regret => REGRET_HELP,
            Stance::Rationalise => RATIONALISE_HELP,
        };
        let help = (current * factor).clamp(HELP_MIN, HELP_MAX);
        if let Some(l) = self.leanings.iter_mut().find(|l| l.toward == other) {
            l.help = help;
            l.since = now;
        } else {
            if self.leanings.len() >= LEANING_CAP {
                // The faintest goes.
                let k = self
                    .leanings
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        (a.help - 1.0)
                            .abs()
                            .total_cmp(&(b.help - 1.0).abs())
                            .then(a.since.cmp(&b.since))
                    })
                    .map(|(k, _)| k)
                    .unwrap_or(0);
                self.leanings.remove(k);
            }
            self.leanings.push(Leaning {
                toward: other,
                help,
                since: now,
            });
        }
        if let Some(j) = self
            .judgements
            .iter_mut()
            .find(|j| j.other == other && j.judged.is_none())
        {
            j.judged = Some((now, stance, found));
            j.opinion = opinion;
            j.help = help;
        }
        help
    }

    /// Drop what has lapsed: quarrels never judged within a day, leanings faded out.
    pub fn lapse(&mut self, now: u64) {
        self.judgements
            .retain(|j| j.judged.is_some() || now.saturating_sub(j.at) <= JUDGE_WITHIN);
        self.leanings
            .retain(|l| now.saturating_sub(l.since) < LEANING_DAYS * TICKS_PER_DAY);
    }

    pub fn pending(&self) -> impl Iterator<Item = &Judgement> {
        self.judgements.iter().filter(|j| j.judged.is_none())
    }
}

/// How far a person leans toward regret over rationalising, 0..1: warmth and caution toward
/// regret, self-reliance and the stress they carry toward rationalising (#568). Legacy minds
/// with no dimensional profile sit at the middle, nudged by their traits.
pub fn regret_tendency(a: &crate::agents::Agent) -> f32 {
    let base = match a.dispositions {
        Some(d) => {
            0.45 * d.warmth.clamp(0.0, 1.0)
                + 0.25 * d.caution.clamp(0.0, 1.0)
                + 0.30 * (1.0 - d.independence.clamp(0.0, 1.0))
        }
        None => 0.5,
    };
    (base - 0.2 * a.stress.clamp(0.0, 1.0)).clamp(0.0, 1.0)
}

/// Regret at or above the middle, rationalising below it.
pub fn stance_of(a: &crate::agents::Agent) -> Stance {
    if regret_tendency(a) >= 0.5 {
        Stance::Regret
    } else {
        Stance::Rationalise
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leanings_are_bounded_and_fade() {
        let mut c = Conscience::default();
        let b = AgentId(2);
        for k in 0..20 {
            c.quarrelled(b, None, k);
            c.judge(b, Stance::Regret, Found::Hurt, 0.1, k);
        }
        assert!((c.help_toward(b, 19) - HELP_MAX).abs() < 1e-6);
        for k in 0..20 {
            c.quarrelled(b, None, 100 + k);
            c.judge(b, Stance::Rationalise, Found::Hurt, -0.05, 100 + k);
        }
        assert!((c.help_toward(b, 119) - HELP_MIN).abs() < 1e-6);
        let half = 120 + LEANING_DAYS * TICKS_PER_DAY / 2;
        assert!((c.help_toward(b, half) - (1.0 + (HELP_MIN - 1.0) * 0.5)).abs() < 1e-3);
        c.lapse(120 + LEANING_DAYS * TICKS_PER_DAY);
        assert!(c.leanings.is_empty());
        assert_eq!(c.help_toward(b, 0), 1.0);
        assert!(c.judgements.len() <= JUDGEMENT_CAP);
    }

    #[test]
    fn an_unjudged_quarrel_lapses_after_a_day() {
        let mut c = Conscience::default();
        c.quarrelled(AgentId(3), None, 10);
        c.lapse(10 + JUDGE_WITHIN);
        assert_eq!(c.pending().count(), 1);
        c.lapse(11 + JUDGE_WITHIN);
        assert_eq!(c.pending().count(), 0);
    }
}
