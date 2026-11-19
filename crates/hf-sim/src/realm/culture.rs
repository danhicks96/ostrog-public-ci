//! Cultures (P7 of milestone MP, docs/design-peoples.md sections 5.1 and 6). Every people starts
//! with one culture; a daughter settlement founded far from the homeland grows its own. A
//! culture holds a stance on each act and a weight on each value, held by its people as
//! beliefs whose source is the culture, and a rite for its dead. Stances come from the
//! people's kind and convictions with a seeded lean of their own; a new culture drifts from its
//! parent. Every split is a row of the world's history.

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::past::Topic;
use crate::rng::{purpose, stream};

/// How a culture keeps its dead.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Rite {
    #[default]
    Bury,
    Burn,
    /// Laid out on high ground for the sky.
    Expose,
    /// Kept in a hall of the dead.
    Hall,
}

impl Rite {
    pub fn words(self) -> &'static str {
        match self {
            Rite::Bury => "put in the ground",
            Rite::Burn => "burned",
            Rite::Expose => "laid out under the sky",
            Rite::Hall => "kept in a hall of the dead",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Rite::Bury => "bury their dead",
            Rite::Burn => "burn their dead",
            Rite::Expose => "lay their dead out under the sky",
            Rite::Hall => "keep their dead in a hall",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Culture {
    pub id: u32,
    pub people: u32,
    /// "the Vashka of Tor Ennan".
    pub name: String,
    pub founded: i64,
    /// The culture it grew from, and the settlement where it did.
    pub from: Option<u32>,
    pub site: Option<u32>,
    /// A stance on every act and a weight on every value, -1..1.
    pub stances: Vec<(Topic, f32)>,
    pub rite: Rite,
    /// Against whom it holds a grudge, and since what row (P8).
    #[serde(default)]
    pub grudges: Vec<super::stories::Grudge>,
    /// The gods it named for what struck it (P8).
    #[serde(default)]
    pub gods: Vec<super::stories::God>,
}

impl Culture {
    pub fn stance(&self, t: Topic) -> f32 {
        self.stances
            .iter()
            .find(|s| s.0 == t)
            .map(|s| s.1)
            .unwrap_or(0.0)
    }
}

fn r2(v: f32) -> f32 {
    (v.clamp(-1.0, 1.0) * 100.0).round() / 100.0
}

/// A people's first culture: each act's stance and each value's weight from its kind's
/// leanings, its own convictions and a lean of its own; its rite drawn.
pub fn first(seed: u64, p: &super::peoples::People, kind: &crate::species::SpeciesDef) -> Culture {
    let mut r = stream(seed, &[purpose::REALM_PEOPLES, 3, p.id as u64]);
    let c = |t: Topic| p.conviction(t);
    let lean = |t: Topic| {
        kind.temperament
            .convictions
            .get(t.name())
            .copied()
            .unwrap_or(0.0)
    };
    let herding = matches!(p.living, super::peoples::Living::Herding);
    let hunting = matches!(p.living, super::peoples::Living::Hunting);
    let mut stances = Vec::new();
    for t in Topic::ACTS.into_iter().chain(Topic::VALUES) {
        let base = match t {
            Topic::Kill => -0.7 + 0.3 * c(Topic::Violence),
            Topic::EatDead => -0.75 + if c(Topic::Hunger) > 0.3 { 0.3 } else { 0.0 },
            Topic::Theft => -0.5,
            Topic::CutLivingTree => 0.0,
            Topic::Burial => 0.55,
            Topic::OtherRaces => -0.6 * c(Topic::Strangers),
            Topic::DeitySigns => 0.0,
            Topic::DeepDigging => 0.0,
            Topic::KeepingBeasts if herding => 0.5,
            Topic::Tradition => 0.2,
            Topic::Family => 0.3,
            Topic::Commerce => {
                if herding || hunting {
                    0.0
                } else {
                    0.2
                }
            }
            Topic::Nature if hunting => 0.3,
            Topic::Peace => -0.6 * c(Topic::Violence),
            Topic::MartialProwess => 0.6 * c(Topic::Violence),
            Topic::Merriment => 0.6 * c(Topic::Drink),
            Topic::Hospitality => -0.6 * c(Topic::Strangers),
            _ => 0.0,
        };
        let own: f32 = r.gen_range(-0.35..0.35);
        stances.push((t, r2(base + lean(t) + own)));
    }
    let rite = match r.gen_range(0..20) {
        0..=9 => Rite::Bury,
        10..=14 => Rite::Burn,
        15..=16 => Rite::Expose,
        _ => Rite::Hall,
    };
    Culture {
        id: p.id,
        people: p.id,
        name: format!("the {}", p.name),
        founded: 0,
        from: None,
        site: None,
        stances,
        rite,
        grudges: Vec::new(),
        gods: Vec::new(),
    }
}

/// A culture grown from `parent` at a far settlement: every stance drifts a little, and one in
/// five keeps its dead another way.
pub fn split(
    seed: u64,
    parent: &Culture,
    id: u32,
    site: u32,
    site_name: &str,
    year: i64,
    people_name: &str,
) -> Culture {
    let mut r = stream(seed, &[purpose::REALM_PEOPLES, 4, id as u64]);
    let stances = parent
        .stances
        .iter()
        .map(|(t, v)| (*t, r2(v + r.gen_range(-0.25..0.25))))
        .collect();
    let rite = if r.gen_bool(0.2) {
        [Rite::Bury, Rite::Burn, Rite::Expose, Rite::Hall][r.gen_range(0..4)]
    } else {
        parent.rite
    };
    Culture {
        id,
        people: parent.people,
        name: format!("the {people_name} of {site_name}"),
        founded: year,
        from: Some(parent.id),
        site: Some(site),
        stances,
        rite,
        grudges: Vec::new(),
        gods: Vec::new(),
    }
}

/// How far, in regions, a daughter settlement must lie from its people's homeland to grow a
/// culture of its own.
pub const SPLIT_DISTANCE: usize = 24;
