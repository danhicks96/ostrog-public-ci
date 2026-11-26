//! Thoughts: weighted, dated feelings about what happened, feeding mood and stress. Each
//! kind has a weight and a life; a mind holds a bounded list. Nothing here is invented:
//! every thought is added by `systems::strain::observe` from a recorded event or by a
//! system from a state the person is in.

use serde::{Deserialize, Serialize};

use crate::ids::EventId;
use crate::time::TICKS_PER_DAY;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum ThoughtKind {
    AteWell,
    Hungry,
    SleptInBed,
    SleptRough,
    Hurt,
    Sick,
    Tended,
    Quarrel,
    GoodTalk,
    LostSomeone,
    SawDeath,
    NewChild,
    Wedding,
    NewClothes,
    SawBreak,
    PartyHome,
    MadeSomething,
    Cold,
    SawFire,
    /// A cherished or named tool broke or was lost.
    LostTreasure,
    /// Below ground with no light to see by.
    Dark,
    /// In the hollow by the portal stone, where something is wrong.
    Uneasy,
    /// Slept in the hollow and did not rest.
    SleptUneasy,
    /// Appended (C34): shared ale or wine at the hearth.
    SharedDrink,
    /// Given a ring at their pairing, or gave one.
    GivenARing,
    /// Appended (C35): made the finest thing of their life in a fey mood.
    Masterwork,
    /// A fey mood passed with the work undone.
    FeyLost,
    /// Appended (P4): out in heat above what their kind is comfortable in.
    Hot,
    /// Appended (P7): one of theirs was laid to rest by another people's rite.
    RiteBroken,
    /// Saw a living tree felled, which their people hold wrong.
    TreeFelled,
    /// Glad someone of another kind has joined them.
    NewKindWelcome,
    /// Uneasy that someone of another kind has joined them.
    NewKindUneasy,
    /// Appended (P11): saw one of the restless dead at dusk.
    GhostSeen,
    /// Burned by the sun, as the Thirst burns those it holds.
    Glare,
    /// Saw the dead eaten, which their people hold abhorrent.
    SawTheDeadEaten,
    /// Their limbs stiffening, as the stone of the underworld takes them.
    Stiffening,
    /// Appended (#338): saw the dead laid to rest as it should be.
    FittingRite,
    /// Appended (feelings): a strong feeling carried (mind/feelings.rs).
    Afraid,
    Grieving,
    Angry,
    Glad,
    Awed,
    Ashamed,
    /// Appended (#297): kept the season's observance with their people.
    KeptObservance,
    /// Appended (#297): the season's observance was let lapse.
    ObservanceLapsed,
    /// Appended (#322): carried down into the deep and brought back.
    BroughtBack,
    /// Appended (#392): thrown through the portal into a land they do not know. Its weight
    /// and life are the person's own (`systems::strain::arrival_shock`).
    Arrival,
    /// Appended (#297): their community's shrine was lost.
    ShrineLost,
    /// Appended (#274): slept in a house walled with fine stone (marble and its like).
    FineHouse,
    /// Appended (#297): a relic of their community was lost.
    RelicLost,
    /// Appended (#297): went to the shrine or a relic in hard times and prayed there.
    Prayed,
    /// Appended (#395): the rest of the party is unaccounted for since the portal; a
    /// background dread of the missing over the first days.
    Missing,
    /// Appended (#879): parched, same gate as Hungry (`needs.thirst < 0.2`).
    Thirsty,
    /// Appended (OST-1101): awake with the social need run low.
    Lonely,
}

impl ThoughtKind {
    /// (weight, days it lasts)
    pub fn shape(self) -> (f32, f32) {
        match self {
            ThoughtKind::AteWell => (0.15, 1.0),
            ThoughtKind::Hungry => (-0.25, 0.5),
            ThoughtKind::SleptInBed => (0.1, 1.0),
            ThoughtKind::SleptRough => (-0.2, 1.0),
            ThoughtKind::Hurt => (-0.3, 0.5),
            ThoughtKind::Sick => (-0.3, 0.5),
            ThoughtKind::Tended => (0.3, 2.0),
            ThoughtKind::Quarrel => (-0.3, 2.0),
            ThoughtKind::GoodTalk => (0.15, 1.0),
            ThoughtKind::LostSomeone => (-0.6, 7.0),
            ThoughtKind::SawDeath => (-0.2, 2.0),
            ThoughtKind::NewChild => (0.5, 5.0),
            ThoughtKind::Wedding => (0.4, 3.0),
            ThoughtKind::NewClothes => (0.2, 2.0),
            ThoughtKind::SawBreak => (-0.1, 2.0),
            ThoughtKind::Afraid => (-0.12, 0.2),
            ThoughtKind::Grieving => (-0.15, 0.2),
            ThoughtKind::Angry => (-0.1, 0.2),
            ThoughtKind::Glad => (0.15, 0.2),
            ThoughtKind::Awed => (0.1, 0.2),
            ThoughtKind::Ashamed => (-0.12, 0.2), // halved in #237
            ThoughtKind::PartyHome => (0.15, 2.0),
            ThoughtKind::MadeSomething => (0.15, 1.0),
            ThoughtKind::Cold => (-0.3, 0.5),
            ThoughtKind::SawFire => (-0.25, 2.0),
            ThoughtKind::LostTreasure => (-0.35, 4.0),
            ThoughtKind::Dark => (-0.2, 0.3),
            ThoughtKind::Uneasy => (-0.15, 0.3),
            ThoughtKind::SleptUneasy => (-0.3, 1.0),
            ThoughtKind::SharedDrink => (0.15, 1.0),
            ThoughtKind::GivenARing => (0.3, 10.0),
            ThoughtKind::Masterwork => (0.5, 15.0),
            ThoughtKind::FeyLost => (-0.4, 5.0),
            ThoughtKind::Hot => (-0.15, 0.3),
            ThoughtKind::RiteBroken => (-0.35, 4.0),
            ThoughtKind::FittingRite => (0.25, 3.0),
            ThoughtKind::TreeFelled => (-0.2, 1.0),
            ThoughtKind::NewKindWelcome => (0.15, 2.0),
            ThoughtKind::NewKindUneasy => (-0.15, 3.0),
            ThoughtKind::GhostSeen => (-0.35, 3.0),
            ThoughtKind::Glare => (-0.3, 0.5),
            ThoughtKind::SawTheDeadEaten => (-0.4, 5.0),
            ThoughtKind::Stiffening => (-0.3, 1.0),
            ThoughtKind::Arrival => (-0.35, 3.5),
            ThoughtKind::KeptObservance => (0.2, 3.0),
            ThoughtKind::ObservanceLapsed => (-0.2, 3.0),
            ThoughtKind::BroughtBack => (-0.3, 30.0),
            ThoughtKind::ShrineLost => (-0.35, 5.0),
            ThoughtKind::FineHouse => (0.15, 1.0),
            ThoughtKind::RelicLost => (-0.3, 5.0),
            ThoughtKind::Prayed => (0.2, 2.0),
            ThoughtKind::Missing => (-0.2, 4.0),
            ThoughtKind::Thirsty => (-0.3, 0.5),
            ThoughtKind::Lonely => (-0.15, 0.5),
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            ThoughtKind::AteWell => "ate well",
            ThoughtKind::Hungry => "hungry",
            ThoughtKind::SleptInBed => "slept in a bed",
            ThoughtKind::SleptRough => "slept on the ground",
            ThoughtKind::Hurt => "in pain",
            ThoughtKind::Sick => "feverish",
            ThoughtKind::Tended => "was looked after",
            ThoughtKind::Quarrel => "quarrelled",
            ThoughtKind::GoodTalk => "good company",
            ThoughtKind::LostSomeone => "grieving",
            ThoughtKind::SawDeath => "saw someone die",
            ThoughtKind::NewChild => "a new child",
            ThoughtKind::Wedding => "newly partnered",
            ThoughtKind::NewClothes => "new clothes",
            ThoughtKind::SawBreak => "saw someone break down",
            ThoughtKind::Afraid => "afraid",
            ThoughtKind::Grieving => "grieving",
            ThoughtKind::Angry => "angry",
            ThoughtKind::Glad => "glad",
            ThoughtKind::Awed => "in awe",
            ThoughtKind::Ashamed => "ashamed",
            ThoughtKind::PartyHome => "the party came home",
            ThoughtKind::MadeSomething => "made something",
            ThoughtKind::Cold => "cold to the bone",
            ThoughtKind::SawFire => "saw fire",
            ThoughtKind::LostTreasure => "lost something dear",
            ThoughtKind::Dark => "in the dark below",
            ThoughtKind::Uneasy => "uneasy by the stone",
            ThoughtKind::SleptUneasy => "slept badly by the stone",
            ThoughtKind::SharedDrink => "a drink at the hearth",
            ThoughtKind::GivenARing => "a ring, and what it means",
            ThoughtKind::Masterwork => "made the finest thing of their life",
            ThoughtKind::FeyLost => "the work that would not come",
            ThoughtKind::Hot => "the heat",
            ThoughtKind::RiteBroken => "the dead not given their rite",
            ThoughtKind::FittingRite => "the dead laid to rest as they should be",
            ThoughtKind::TreeFelled => "a living tree cut down",
            ThoughtKind::NewKindWelcome => "glad of a newcomer of another kind",
            ThoughtKind::NewKindUneasy => "uneasy about a newcomer of another kind",
            ThoughtKind::GhostSeen => "saw the restless dead",
            ThoughtKind::Glare => "the sun burns",
            ThoughtKind::SawTheDeadEaten => "saw the dead eaten",
            ThoughtKind::Stiffening => "limbs stiffening like stone",
            ThoughtKind::Arrival => "torn from the old world",
            ThoughtKind::KeptObservance => "kept the season's observance",
            ThoughtKind::ObservanceLapsed => "the season's observance let lapse",
            ThoughtKind::BroughtBack => "what was below, in the dark",
            ThoughtKind::ShrineLost => "the shrine lost",
            ThoughtKind::FineHouse => "slept under walls of fine stone",
            ThoughtKind::RelicLost => "a relic lost",
            ThoughtKind::Prayed => "prayed at the shrine in hard times",
            ThoughtKind::Missing => "the missing",
            ThoughtKind::Thirsty => "parched",
            ThoughtKind::Lonely => "lonely",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Thought {
    pub kind: ThoughtKind,
    pub weight: f32,
    pub since: u64,
    pub until: u64,
    pub source: Option<EventId>,
}

pub const THOUGHT_CAP: usize = 24;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Thoughts {
    pub list: Vec<Thought>,
}

impl Thoughts {
    /// Add a thought; a repeat of the same kind refreshes it instead of stacking (grief
    /// and quarrels stack, because each is about someone).
    pub fn add(&mut self, kind: ThoughtKind, now: u64, source: Option<EventId>) {
        let (weight, days) = kind.shape();
        let until = now + (days * TICKS_PER_DAY as f32) as u64;
        let stacks = matches!(
            kind,
            ThoughtKind::LostSomeone | ThoughtKind::Quarrel | ThoughtKind::SawDeath
        );
        if !stacks {
            if let Some(t) = self.list.iter_mut().find(|t| t.kind == kind) {
                t.until = until;
                t.since = now;
                t.source = source.or(t.source);
                return;
            }
        }
        if self.list.len() >= THOUGHT_CAP {
            // Drop the weakest remaining thought.
            if let Some((i, _)) = self
                .list
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.weight.abs().total_cmp(&b.1.weight.abs()))
            {
                self.list.remove(i);
            }
        }
        self.list.push(Thought {
            kind,
            weight,
            since: now,
            until,
            source,
        });
    }
    /// Add a thought whose weight and life are the person's own rather than the kind's
    /// (#392: how hard the arrival hits and how long it lasts is temperament).
    pub fn add_shaped(&mut self, kind: ThoughtKind, weight: f32, days: f32, now: u64) {
        self.list.retain(|t| t.kind != kind);
        if self.list.len() >= THOUGHT_CAP {
            if let Some((i, _)) = self
                .list
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.weight.abs().total_cmp(&b.1.weight.abs()))
            {
                self.list.remove(i);
            }
        }
        self.list.push(Thought {
            kind,
            weight,
            since: now,
            until: now + (days * TICKS_PER_DAY as f32) as u64,
            source: None,
        });
    }
    /// How much of a held thought's life is left, 0..1 (0 when not held).
    pub fn left(&self, kind: ThoughtKind, now: u64) -> f32 {
        self.list
            .iter()
            .find(|t| t.kind == kind)
            .map(|t| {
                let life = (t.until - t.since).max(1) as f32;
                (t.until.saturating_sub(now) as f32 / life).clamp(0.0, 1.0)
            })
            .unwrap_or(0.0)
    }
    pub fn prune(&mut self, now: u64) {
        self.list.retain(|t| t.until > now);
    }
    /// Sum of weights, each fading linearly over its life.
    pub fn sum(&self, now: u64) -> f32 {
        self.list
            .iter()
            .map(|t| {
                let life = (t.until - t.since).max(1) as f32;
                let left = t.until.saturating_sub(now) as f32 / life;
                t.weight * (0.4 + 0.6 * left)
            })
            .sum()
    }
    /// What the held thoughts of one kind add to `sum`.
    pub fn sum_of(&self, kind: ThoughtKind, now: u64) -> f32 {
        self.list
            .iter()
            .filter(|t| t.kind == kind)
            .map(|t| {
                let life = (t.until - t.since).max(1) as f32;
                let left = t.until.saturating_sub(now) as f32 / life;
                t.weight * (0.4 + 0.6 * left)
            })
            .sum()
    }
    /// The recorded event behind a thought they hold, if it has one.
    pub fn source_of(&self, kind: ThoughtKind) -> Option<EventId> {
        self.list
            .iter()
            .find(|t| t.kind == kind)
            .and_then(|t| t.source)
    }

    pub fn has(&self, kind: ThoughtKind) -> bool {
        self.list.iter().any(|t| t.kind == kind)
    }
}
