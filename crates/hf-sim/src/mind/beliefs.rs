//! Beliefs with sources (design-organic.md section 2).
//!
//! A belief is a stance on a topic (past.rs `Topic`: drink, violence, fire, hunger, strangers,
//! death, trust) with a confidence and the sources it came from: the chronicle events that
//! formed or moved it, and the people who told them. Beliefs weight choices; they never decide
//! them. `weight` is capped to 0.6..1.4 of a choice's utility, so a frightened person can still
//! be brave on a good day.
//!
//! Beliefs move. Talking spreads them, more from people one trusts and less to people already
//! sure. Outcomes confirm or shake them: someone who believed fire is fought and saw it work
//! holds that harder. A child starts with their parents' beliefs, held lightly.

use crate::ids::{AgentId, EventId};
pub use crate::past::Topic;
use serde::{Deserialize, Serialize};

pub const MAX_SOURCES: usize = 6;
/// The most a belief can move a choice: ×(1 ± CAP).
pub const CAP: f32 = 0.4;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Source {
    /// Something that happened (a chronicle event).
    Event(EventId),
    /// Someone who told them.
    Person(AgentId),
    /// What their people held when they came through (chosen at the start; the event records
    /// that choice).
    Ideology(EventId),
    /// What their kind leans toward (P7): the species' temperament, faintly held.
    Species(crate::species::SpeciesId),
    /// What their culture holds (P7): the realm culture's index.
    Culture(u32),
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Belief {
    pub topic: Topic,
    /// -1..1, meaning per `Topic`.
    pub stance: f32,
    /// 0..1: how sure.
    pub confidence: f32,
    /// Newest last, at most MAX_SOURCES.
    pub sources: Vec<Source>,
    pub since: u64,
}

#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Beliefs {
    pub list: Vec<Belief>,
}

impl Beliefs {
    pub fn get(&self, t: Topic) -> Option<&Belief> {
        self.list.iter().find(|b| b.topic == t)
    }

    /// Signed strength -1..1 (stance × confidence), 0 if they hold no belief.
    pub fn lean(&self, t: Topic) -> f32 {
        self.get(t).map(|b| b.stance * b.confidence).unwrap_or(0.0)
    }

    /// Multiplier on a choice that a positive stance on `t` favours (`dir` +1) or disfavours
    /// (`dir` -1): 1 ± CAP × lean, so always within 0.6..1.4.
    pub fn weight(&self, t: Topic, dir: f32) -> f32 {
        (1.0 + CAP * self.lean(t) * dir.signum()).clamp(1.0 - CAP, 1.0 + CAP)
    }

    fn add_source(b: &mut Belief, s: Source) {
        if b.sources.last() != Some(&s) {
            b.sources.push(s);
            if b.sources.len() > MAX_SOURCES {
                b.sources.remove(0);
            }
        }
    }

    /// Form or overwrite a belief outright (from a past answer or an ideology).
    pub fn hold(&mut self, t: Topic, stance: f32, confidence: f32, source: Source, now: u64) {
        let stance = stance.clamp(-1.0, 1.0);
        let confidence = confidence.clamp(0.0, 1.0);
        if let Some(b) = self.list.iter_mut().find(|b| b.topic == t) {
            b.stance = stance;
            b.confidence = confidence;
            Self::add_source(b, source);
        } else {
            self.list.push(Belief {
                topic: t,
                stance,
                confidence,
                sources: vec![source],
                since: now,
            });
            self.list.sort_by_key(|b| b.topic);
        }
    }

    /// Something happened that bears on `t`, pointing toward `toward` (-1..1) with `force`
    /// (0..1). A belief nudged the way it already leans grows surer; nudged against, it bends
    /// and loses some certainty. With no belief yet, one forms, weakly.
    pub fn experience(&mut self, t: Topic, toward: f32, force: f32, source: Source, now: u64) {
        let toward = toward.clamp(-1.0, 1.0);
        let force = force.clamp(0.0, 1.0);
        match self.list.iter_mut().find(|b| b.topic == t) {
            Some(b) => {
                let agrees = (b.stance >= 0.0) == (toward >= 0.0);
                b.stance = (b.stance + (toward - b.stance) * force * (1.0 - 0.5 * b.confidence))
                    .clamp(-1.0, 1.0);
                b.confidence = if agrees {
                    (b.confidence + 0.15 * force).min(1.0)
                } else {
                    (b.confidence - 0.2 * force).max(0.05)
                };
                Self::add_source(b, source);
            }
            None => self.hold(t, toward * force.sqrt(), 0.2 + 0.3 * force, source, now),
        }
    }

    /// Being told by someone: the listener moves toward the teller's stance by how much they
    /// trust them (`trust` 0..1) and how unsure they are themselves.
    pub fn hear(&mut self, t: Topic, teller_stance: f32, teller: AgentId, trust: f32, now: u64) {
        let trust = trust.clamp(0.0, 1.0);
        match self.list.iter_mut().find(|b| b.topic == t) {
            Some(b) => {
                let pull = 0.25 * trust * (1.0 - b.confidence);
                b.stance = (b.stance + (teller_stance - b.stance) * pull).clamp(-1.0, 1.0);
                if (b.stance >= 0.0) == (teller_stance >= 0.0) {
                    b.confidence = (b.confidence + 0.03 * trust).min(1.0);
                }
                Self::add_source(b, Source::Person(teller));
            }
            None if trust > 0.2 => self.hold(
                t,
                teller_stance * 0.6 * trust,
                0.15 + 0.2 * trust,
                Source::Person(teller),
                now,
            ),
            None => {}
        }
    }

    /// The belief they would most readily talk about (strongest held).
    pub fn strongest(&self) -> Option<&Belief> {
        self.list.iter().max_by(|a, b| {
            (a.stance.abs() * a.confidence)
                .partial_cmp(&(b.stance.abs() * b.confidence))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// What a child starts with: each parent's beliefs, held lightly, sourced to that parent.
    pub fn inherit(parents: &[(AgentId, &Beliefs)], now: u64) -> Beliefs {
        let mut out = Beliefs::default();
        for (who, b) in parents {
            for x in &b.list {
                out.hear(x.topic, x.stance, *who, 0.5 + 0.4 * x.confidence, now);
            }
        }
        for b in out.list.iter_mut() {
            b.confidence = b.confidence.min(0.35);
        }
        out
    }
}

/// How a belief reads in a sentence ("fire is something you fight").
pub fn phrase(t: Topic, stance: f32) -> &'static str {
    match (t, stance < 0.0) {
        (Topic::Drink, true) => "drink is ruin",
        (Topic::Drink, false) => "drink eases things",
        (Topic::Violence, true) => "you never raise a hand",
        (Topic::Violence, false) => "a fist settles things",
        (Topic::Fire, true) => "a fire is something you fight",
        (Topic::Fire, false) => "a fire is something you run from",
        (Topic::Hunger, true) => "in hunger you share",
        (Topic::Hunger, false) => "in hunger you keep what you have",
        (Topic::Strangers, true) => "strangers are welcome",
        (Topic::Strangers, false) => "strangers bring harm",
        (Topic::Death, true) => "grief is borne alone",
        (Topic::Death, false) => "the living must be held close",
        (Topic::Trust, true) => "people deserve forgiving",
        (Topic::Trust, false) => "trust is earned slowly",
        (Topic::Kill, true) => "killing outside a fight is abhorrent",
        (Topic::Kill, false) => "some deserve killing",
        (Topic::EatDead, true) => "eating the dead is abhorrent",
        (Topic::EatDead, false) => "the dead may feed the living",
        (Topic::Theft, true) => "taking what is another's is wrong",
        (Topic::Theft, false) => "what lies about is anyone's",
        (Topic::CutLivingTree, true) => "a living tree is not to be cut",
        (Topic::CutLivingTree, false) => "a tree is timber",
        (Topic::Burial, true) => "how the dead are kept matters little",
        (Topic::Burial, false) => "the dead must be given their rite",
        (Topic::OtherRaces, true) => "other kinds are best kept apart",
        (Topic::OtherRaces, false) => "any kind may live among us",
        (Topic::DeitySigns, true) => "signs are only weather",
        (Topic::DeitySigns, false) => "the signs must be heeded",
        (Topic::DeepDigging, true) => "the deep is not to be dug",
        (Topic::DeepDigging, false) => "the deep is there to be dug",
        (Topic::KeepingBeasts, true) => "beasts should run free",
        (Topic::KeepingBeasts, false) => "beasts are for keeping",
        (Topic::Tradition, true) => "the old ways bind no one",
        (Topic::Tradition, false) => "the old ways are to be kept",
        (Topic::Family, true) => "kin are no closer than anyone",
        (Topic::Family, false) => "kin come first",
        (Topic::Craftsmanship, true) => "a thing need only work",
        (Topic::Craftsmanship, false) => "a thing well made is worth the making",
        (Topic::Knowledge, true) => "some things are better not known",
        (Topic::Knowledge, false) => "what can be learned should be",
        (Topic::Commerce, true) => "trading is for the greedy",
        (Topic::Commerce, false) => "a fair trade helps both",
        (Topic::Nature, true) => "the wild is to be tamed",
        (Topic::Nature, false) => "the wild is to be left be",
        (Topic::Peace, true) => "peace is for the weak",
        (Topic::Peace, false) => "peace is worth keeping",
        (Topic::MartialProwess, true) => "fighting is no one's pride",
        (Topic::MartialProwess, false) => "skill at arms is to be honoured",
        (Topic::Merriment, true) => "merriment is idleness",
        (Topic::Merriment, false) => "a merry night is good for everyone",
        (Topic::Tranquility, true) => "noise is the sound of living",
        (Topic::Tranquility, false) => "quiet is precious",
        (Topic::Hospitality, true) => "a guest is a burden",
        (Topic::Hospitality, false) => "a guest is to be fed",
    }
}
