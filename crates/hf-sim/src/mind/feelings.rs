//! Feelings (docs/design-feelings.md, step 1): what is witnessed, felt by whom, and how it
//! fades. Six kinds, each about something; how strongly a person feels each is their
//! temperament (#281). Felt feelings feed thoughts, so mood and stress read them; contagion and
//! the lean on choices are later steps.

use crate::ids::{AgentId, EventId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FeelingKind {
    Fear,
    Grief,
    Anger,
    Joy,
    Awe,
    Shame,
}

pub const KINDS: [FeelingKind; 6] = [
    FeelingKind::Fear,
    FeelingKind::Grief,
    FeelingKind::Anger,
    FeelingKind::Joy,
    FeelingKind::Awe,
    FeelingKind::Shame,
];

impl FeelingKind {
    pub fn word(self) -> &'static str {
        match self {
            FeelingKind::Fear => "afraid",
            FeelingKind::Grief => "grieving",
            FeelingKind::Anger => "angry",
            FeelingKind::Joy => "glad",
            FeelingKind::Awe => "in awe",
            FeelingKind::Shame => "ashamed",
        }
    }
    /// Share lost an hour: anger burns out fast, grief slow.
    pub fn fade(self) -> f32 {
        match self {
            FeelingKind::Fear => 0.06,
            FeelingKind::Grief => 0.015,
            FeelingKind::Anger => 0.1,
            FeelingKind::Joy => 0.05,
            FeelingKind::Awe => 0.03,
            FeelingKind::Shame => 0.03,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Feeling {
    pub kind: FeelingKind,
    /// Who it is about, if a person.
    pub about: Option<AgentId>,
    /// 0..1.
    pub strength: f32,
    pub since: u64,
    pub source: Option<EventId>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Feelings {
    pub list: Vec<Feeling>,
}

impl Feelings {
    /// Feel `kind` about `about` this strongly; the same feeling about the same is topped up.
    pub fn feel(
        &mut self,
        kind: FeelingKind,
        about: Option<AgentId>,
        strength: f32,
        now: u64,
        source: Option<EventId>,
    ) {
        if strength < 0.02 {
            return;
        }
        if let Some(f) = self
            .list
            .iter_mut()
            .find(|f| f.kind == kind && f.about == about)
        {
            f.strength = (f.strength + strength * (1.0 - f.strength)).min(1.0);
            f.since = now;
            f.source = source.or(f.source);
            return;
        }
        self.list.push(Feeling {
            kind,
            about,
            strength: strength.min(1.0),
            since: now,
            source,
        });
        if self.list.len() > 8 {
            let k = self
                .list
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.strength.total_cmp(&b.1.strength))
                .map(|x| x.0)
                .unwrap();
            self.list.remove(k);
        }
    }
    /// An hour passes.
    pub fn fade_hour(&mut self) {
        for f in self.list.iter_mut() {
            f.strength *= 1.0 - f.kind.fade();
        }
        self.list.retain(|f| f.strength >= 0.05);
    }
    pub fn strongest(&self) -> Option<&Feeling> {
        self.list
            .iter()
            .max_by(|a, b| a.strength.total_cmp(&b.strength))
    }
    pub fn of(&self, kind: FeelingKind) -> f32 {
        self.list
            .iter()
            .filter(|f| f.kind == kind)
            .map(|f| f.strength)
            .fold(0.0, f32::max)
    }
}

/// How strongly `a` feels `kind`, 0.5 to 1.5: their temperament (#281). Drawn from who they
/// are (the genome's calm, their traits and their beliefs) and, for what is not otherwise
/// given, a lean of their own that does not change.
pub fn gain(a: &crate::agents::Agent, kind: FeelingKind) -> f32 {
    use crate::mind::beliefs::Topic;
    use crate::mind::Trait as T;
    let own = {
        let h = (a.id.0 as u64 ^ (kind as u64).wrapping_mul(0x9E37_79B9))
            .wrapping_mul(0xBF58_476D_1CE4_E5B9);
        ((h >> 40) % 1000) as f32 / 1000.0 * 0.6 - 0.3
    };
    let calm = a.genome.calm();
    let g = match kind {
        FeelingKind::Fear => {
            1.0 - 0.4 * (calm - 0.5) + if a.traits.has(T::Cautious) { 0.25 } else { 0.0 }
        }
        FeelingKind::Anger => 1.0 - 0.4 * (calm - 0.5) + 0.3 * a.beliefs.lean(Topic::Violence),
        FeelingKind::Grief => 1.0 + if a.traits.has(T::Sociable) { 0.2 } else { 0.0 },
        FeelingKind::Joy => 1.0 + if a.traits.has(T::Sociable) { 0.2 } else { 0.0 },
        FeelingKind::Awe => 1.0 + if a.traits.has(T::Curious) { 0.25 } else { 0.0 },
        FeelingKind::Shame => 1.0 - 0.2 * (calm - 0.5),
    };
    (g + own).clamp(0.5, 1.5)
}
