//! Lived pasts: who each of the ten was before the portal (design-organic.md section 1).
//!
//! The ten came from one village. Some are siblings, sharing a household and what happened in
//! it; everyone shared what happened to the village (famine winters, fires, a raid, a fever).
//! Each person also has their own life: an apprenticeship, a love, a child, a betrayal.
//!
//! A formative event does not set a belief. It opens a question ("what is drink to me"), and
//! the answer comes from temperament, from whether someone showed them another way (a kind
//! grandparent, a teacher), and from chance. Two sons of a father who drank can answer
//! opposite ways, and here they do. Answers are recorded as events of their own.
//!
//! Everything is a pure function of (seed, slot, rerolls), like `opening::identity`, so the card
//! the viewer is shown and the person who steps through are the same life. The events are
//! written to the chronicle before the arrival (decision D18: the world's clock starts at the
//! epoch, so pre-arrival history has real ticks below it), which keeps the never-invents audit
//! whole: a past is history, recorded once, narrated from its rows.

use crate::mind::traits::Trait;
use crate::mind::{Skill, Traits};
use crate::rng::{purpose, stream};
use crate::time::{DAYS_PER_YEAR, TICKS_PER_DAY, TICKS_PER_YEAR};
use rand::Rng;
use serde::{Deserialize, Serialize};

pub const PARTY: usize = crate::opening::PARTY;

/// What a formative event makes someone answer for themselves.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Topic {
    /// Negative: rejected it. Positive: took after it.
    Drink,
    /// Negative: rejected violence. Positive: learned that force works.
    Violence,
    /// Negative: fire is something you fight. Positive: fire is something you flee.
    Fire,
    /// Negative: in hunger you share. Positive: in hunger you keep.
    Hunger,
    /// Negative: strangers are welcome. Positive: strangers are a danger.
    Strangers,
    /// Negative: grief hardened them. Positive: grief made them hold people closer.
    Death,
    /// Negative: they forgave. Positive: they trust nobody easily now.
    Trust,
    // Acts (P7, docs/design-peoples.md section 6.1): negative abhorrent, positive fitting or
    // required. Appended; never reorder.
    /// Killing someone outside a fight.
    Kill,
    /// Eating the flesh of a sentient dead.
    EatDead,
    /// Taking what another owns.
    Theft,
    /// Felling a living tree.
    CutLivingTree,
    /// How much it matters that the dead are given their people's rite.
    Burial,
    /// Living alongside people of another kind.
    OtherRaces,
    /// Heeding the signs of what watches.
    DeitySigns,
    /// Digging below the caverns.
    DeepDigging,
    /// Penning or riding beasts.
    KeepingBeasts,
    // Values (P7, section 6.2): negative scorned, positive prized.
    Tradition,
    Family,
    Craftsmanship,
    Knowledge,
    Commerce,
    Nature,
    Peace,
    MartialProwess,
    Merriment,
    Tranquility,
    Hospitality,
}

impl Topic {
    pub const ALL: [Topic; 7] = [
        Topic::Drink,
        Topic::Violence,
        Topic::Fire,
        Topic::Hunger,
        Topic::Strangers,
        Topic::Death,
        Topic::Trust,
    ];

    /// The acts a people holds a stance on (P7).
    pub const ACTS: [Topic; 9] = [
        Topic::Kill,
        Topic::EatDead,
        Topic::Theft,
        Topic::CutLivingTree,
        Topic::Burial,
        Topic::OtherRaces,
        Topic::DeitySigns,
        Topic::DeepDigging,
        Topic::KeepingBeasts,
    ];

    /// What a people prizes or scorns (P7).
    pub const VALUES: [Topic; 11] = [
        Topic::Tradition,
        Topic::Family,
        Topic::Craftsmanship,
        Topic::Knowledge,
        Topic::Commerce,
        Topic::Nature,
        Topic::Peace,
        Topic::MartialProwess,
        Topic::Merriment,
        Topic::Tranquility,
        Topic::Hospitality,
    ];

    /// Every topic: the seven convictions, the acts, the values.
    pub fn every() -> impl Iterator<Item = Topic> {
        Topic::ALL
            .into_iter()
            .chain(Topic::ACTS)
            .chain(Topic::VALUES)
    }

    pub fn from_name(s: &str) -> Option<Topic> {
        Topic::every().find(|t| t.name() == s)
    }

    pub fn name(self) -> &'static str {
        match self {
            Topic::Drink => "drink",
            Topic::Violence => "violence",
            Topic::Fire => "fire",
            Topic::Hunger => "hunger",
            Topic::Strangers => "strangers",
            Topic::Death => "death",
            Topic::Trust => "trust",
            Topic::Kill => "kill",
            Topic::EatDead => "eat_dead",
            Topic::Theft => "theft",
            Topic::CutLivingTree => "cut_living_tree",
            Topic::Burial => "burial",
            Topic::OtherRaces => "other_races",
            Topic::DeitySigns => "deity_signs",
            Topic::DeepDigging => "deep_digging",
            Topic::KeepingBeasts => "keeping_beasts",
            Topic::Tradition => "tradition",
            Topic::Family => "family",
            Topic::Craftsmanship => "craftsmanship",
            Topic::Knowledge => "knowledge",
            Topic::Commerce => "commerce",
            Topic::Nature => "nature",
            Topic::Peace => "peace",
            Topic::MartialProwess => "martial_prowess",
            Topic::Merriment => "merriment",
            Topic::Tranquility => "tranquility",
            Topic::Hospitality => "hospitality",
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Formative {
    ParentDrank {
        parent: String,
    },
    ParentViolent {
        parent: String,
    },
    ParentDied {
        parent: String,
        cause: String,
    },
    SiblingDied {
        sibling: String,
        cause: String,
    },
    KindElder {
        elder: String,
    },
    Famine,
    HouseFire,
    VillageFire,
    Raid,
    Fever,
    Flood,
    Apprenticed {
        craft: String,
        master: String,
    },
    Married {
        partner: String,
    },
    ChildBorn {
        child: String,
    },
    ChildDied {
        child: String,
    },
    Betrayed {
        by: String,
    },
    SavedFromWater {
        who: String,
    },
    LostInWoods,
    /// They helped beat out a fire and saw it work: how fire is fought, learned by doing.
    FoughtFire,
    /// The answer to a question an earlier event opened. `stance` is -100..100 (see Topic);
    /// `about` names the person or thing it was about, for the narration.
    Answered {
        topic: Topic,
        stance: i8,
        about: String,
    },
    /// Who they left on the other side of the portal.
    LeftBehind {
        who: String,
    },
}

impl Formative {
    /// Hard things leave their mark on stress at the start.
    pub fn is_trauma(&self) -> bool {
        matches!(
            self,
            Formative::ParentViolent { .. }
                | Formative::ParentDied { .. }
                | Formative::SiblingDied { .. }
                | Formative::ChildDied { .. }
                | Formative::HouseFire
                | Formative::Raid
                | Formative::Betrayed { .. }
        )
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PastEvent {
    /// Whole years before the arrival (at least 1, or 0 for the leaving itself).
    pub years_before: u32,
    /// Day within that year, 0..20.
    pub day: u8,
    /// Their age when it happened.
    pub age: u8,
    pub what: Formative,
}

#[derive(Clone, Debug, Default)]
pub struct PastLife {
    pub home: String,
    pub household: u8,
    pub age: u32,
    pub events: Vec<PastEvent>,
    pub answers: Vec<(Topic, f32)>,
    pub skill_bonus: Vec<(Skill, f32)>,
}

impl PastLife {
    pub fn stance(&self, t: Topic) -> Option<f32> {
        self.answers.iter().find(|(k, _)| *k == t).map(|(_, s)| *s)
    }
}

/// The tick a past event happened at, given the world's epoch (the arrival).
pub fn tick_of(epoch: u64, e: &PastEvent) -> u64 {
    if e.years_before == 0 {
        return epoch.saturating_sub(1); // at the portal, just before stepping through
    }
    let back = e.years_before as u64 * TICKS_PER_YEAR;
    let day = (e.day as u64).min(DAYS_PER_YEAR - 1) * TICKS_PER_DAY + TICKS_PER_DAY / 2;
    epoch.saturating_sub(back) + day.min(back.saturating_sub(1))
}

const SALT_HOUSE: u64 = 0x4005E;
const SALT_VILLAGE: u64 = 0x11A6E;
const SALT_LIFE: u64 = 0x11FE;

/// The village they all came from.
pub fn home_name(seed: u64) -> String {
    crate::names::world_name(seed ^ 0x0011_A6E0_0000_0001)
}

/// Household of each slot: neighbouring slots are siblings with some chance, up to three to a
/// house. Depends on the seed only, so rerolling someone keeps them in their family.
pub fn households(seed: u64) -> [u8; PARTY] {
    let mut r = stream(seed, &[purpose::PAST, SALT_HOUSE]);
    let mut out = [0u8; PARTY];
    let mut h = 0u8;
    let mut size = 1;
    for (i, slot) in out.iter_mut().enumerate() {
        if i > 0 {
            if size < 3 && r.gen_bool(0.35) {
                size += 1;
            } else {
                h += 1;
                size = 1;
            }
        }
        *slot = h;
    }
    out
}

/// A name for someone who never came through (a parent, a partner, a child).
fn other_name(seed: u64, a: u64, b: u64) -> String {
    let k = (a.wrapping_mul(0x9E37_79B9) ^ b.wrapping_mul(0x85EB_CA6B) ^ 0x5EED_0000) as u32;
    crate::names::person_name(seed ^ 0x0700_0000_0000_0000, k)
}

struct Household {
    base_age: i32,
    mother: String,
    father: String,
    /// Which parent drank (0 mother, 1 father) and from how many years before the arrival.
    drank: Option<(u8, u32)>,
    violent: Option<u8>,
    /// (parent, years before the arrival, cause)
    parent_died: Option<(u8, u32, &'static str)>,
    elder: Option<String>,
    sibling_died: Option<(String, u32, &'static str)>,
}

fn household(seed: u64, h: u8) -> Household {
    let mut r = stream(seed, &[purpose::PAST, SALT_HOUSE, h as u64]);
    let base_age = r.gen_range(19..40);
    let mother = other_name(seed, h as u64, 1);
    let father = other_name(seed, h as u64, 2);
    // Years before the arrival when the eldest was about six.
    let childhood = (base_age - 6).max(1) as u32;
    let drank = if r.gen_bool(0.3) {
        Some((r.gen_range(0..2u8), childhood + r.gen_range(0..4)))
    } else {
        None
    };
    let violent = if r.gen_bool(if drank.is_some() { 0.4 } else { 0.1 }) {
        Some(drank.map(|d| d.0).unwrap_or(r.gen_range(0..2u8)))
    } else {
        None
    };
    let parent_died = if r.gen_bool(0.3) {
        let causes = [
            "a winter fever",
            "a fall",
            "the cough",
            "a bad birth",
            "the river",
        ];
        Some((
            r.gen_range(0..2u8),
            r.gen_range(2..(base_age as u32 - 4).max(3)),
            causes[r.gen_range(0..causes.len())],
        ))
    } else {
        None
    };
    let elder = if r.gen_bool(0.4) {
        Some(other_name(seed, h as u64, 3))
    } else {
        None
    };
    let sibling_died = if r.gen_bool(0.15) {
        let causes = ["a fever", "the cold", "the river"];
        Some((
            other_name(seed, h as u64, 4),
            r.gen_range(childhood.saturating_sub(3).max(1)..childhood + 3),
            causes[r.gen_range(0..causes.len())],
        ))
    } else {
        None
    };
    Household {
        base_age,
        mother,
        father,
        drank,
        violent,
        parent_died,
        elder,
        sibling_died,
    }
}

/// What happened to the village, in years before the arrival.
fn village(seed: u64) -> Vec<(u32, Formative)> {
    let mut r = stream(seed, &[purpose::PAST, SALT_VILLAGE]);
    let mut out = Vec::new();
    let mut y = r.gen_range(2..9u32);
    while y < 70 {
        out.push((y, Formative::Famine));
        y += r.gen_range(7..16);
    }
    for _ in 0..r.gen_range(1..3) {
        out.push((r.gen_range(2..55), Formative::VillageFire));
    }
    for _ in 0..r.gen_range(0..3) {
        out.push((r.gen_range(2..60), Formative::Raid));
    }
    if r.gen_bool(0.6) {
        out.push((r.gen_range(3..50), Formative::Fever));
    }
    if r.gen_bool(0.5) {
        out.push((r.gen_range(2..45), Formative::Flood));
    }
    out.sort_by_key(|(y, _)| std::cmp::Reverse(*y));
    out
}

/// How old a slot's person is. Siblings are spread around their household's age.
pub fn age_of(seed: u64, index: u8, rerolls: u32) -> u32 {
    let hs = households(seed);
    let base = household(seed, hs[index as usize]).base_age;
    let mut r = stream(
        seed,
        &[purpose::PAST, SALT_LIFE, index as u64, rerolls as u64],
    );
    (base + r.gen_range(-5..=5)).clamp(16, 50) as u32
}

/// The epoch for a world opened with these rerolls: everyone's whole life fits before it.
pub fn epoch(seed: u64, rerolls: &[u32; PARTY]) -> u64 {
    let oldest = (0..PARTY as u8)
        .map(|i| age_of(seed, i, rerolls[i as usize]))
        .max()
        .unwrap_or(40);
    (oldest as u64 + 2) * TICKS_PER_YEAR
}

/// The answer someone gives to a question an event opened: -1..1, see `Topic`.
fn answer(topic: Topic, traits: &Traits, shown_otherwise: bool, r: &mut impl Rng) -> f32 {
    let mut lean: f32 = r.gen_range(-0.65..0.65);
    let has = |t: Trait| traits.has(t);
    match topic {
        Topic::Drink | Topic::Violence => {
            if shown_otherwise {
                lean -= 0.4;
            }
            if has(Trait::Industrious) {
                lean -= 0.15;
            }
            if has(Trait::Easygoing) {
                lean += 0.15;
            }
            if has(Trait::Cautious) {
                lean -= 0.1;
            }
        }
        Topic::Fire => {
            if has(Trait::Cautious) {
                lean += 0.3;
            }
            if has(Trait::Hearty) {
                lean -= 0.15;
            }
        }
        Topic::Hunger => {
            if has(Trait::Cautious) {
                lean += 0.2;
            }
            if has(Trait::Sociable) {
                lean -= 0.25;
            }
            if shown_otherwise {
                lean -= 0.2;
            }
        }
        Topic::Strangers => {
            if has(Trait::Sociable) {
                lean -= 0.3;
            }
            if has(Trait::Solitary) {
                lean += 0.25;
            }
            if has(Trait::Cautious) {
                lean += 0.2;
            }
        }
        Topic::Death => {
            if has(Trait::Sociable) {
                lean += 0.2;
            }
            if has(Trait::Solitary) {
                lean -= 0.2;
            }
        }
        Topic::Trust => {
            if has(Trait::Cautious) {
                lean += 0.25;
            }
            if has(Trait::Easygoing) {
                lean -= 0.2;
            }
        }
        // Acts and values (P7) are answered by a culture, not by one life's events.
        _ => {}
    }
    lean.clamp(-1.0, 1.0)
}

const CRAFTS: [(&str, Skill); 7] = [
    ("the carpenter", Skill::Construction),
    ("the smith", Skill::Crafting),
    ("the baker", Skill::Cooking),
    ("a farmer", Skill::Farming),
    ("the huntsman", Skill::Animals),
    ("the herb-woman", Skill::Foraging),
    ("the quarry", Skill::Mining),
];

/// One survivor's life before the portal.
pub fn life(seed: u64, index: u8, rerolls: u32, traits: &Traits) -> PastLife {
    let hs = households(seed);
    let hid = hs[index as usize];
    let house = household(seed, hid);
    let age = age_of(seed, index, rerolls) as i32;
    let mut r = stream(
        seed,
        &[purpose::PAST, SALT_LIFE, 0xE7, index as u64, rerolls as u64],
    );
    let mut ev: Vec<PastEvent> = Vec::new();
    let push = |ev: &mut Vec<PastEvent>, yb: i32, what: Formative, r: &mut dyn rand::RngCore| {
        if yb >= 1 && yb <= age {
            ev.push(PastEvent {
                years_before: yb as u32,
                day: (r.next_u32() % DAYS_PER_YEAR as u32) as u8,
                age: (age - yb) as u8,
                what,
            });
        }
    };
    let parent = |p: u8| {
        if p == 0 {
            house.mother.clone()
        } else {
            house.father.clone()
        }
    };
    let mut questions: Vec<(Topic, i32, String)> = Vec::new(); // (topic, years_before, about)

    // The household.
    if let Some(e) = &house.elder {
        let yb = (age - r.gen_range(4..9)).max(1);
        push(
            &mut ev,
            yb,
            Formative::KindElder { elder: e.clone() },
            &mut r,
        );
    }
    if let Some((p, yb)) = house.drank {
        let yb = (yb as i32).min(age - 5);
        push(
            &mut ev,
            yb,
            Formative::ParentDrank { parent: parent(p) },
            &mut r,
        );
        questions.push((Topic::Drink, yb, parent(p)));
    }
    if let Some(p) = house.violent {
        let yb = (age - r.gen_range(6..12)).max(1);
        push(
            &mut ev,
            yb,
            Formative::ParentViolent { parent: parent(p) },
            &mut r,
        );
        questions.push((Topic::Violence, yb, parent(p)));
    }
    if let Some((p, yb, cause)) = house.parent_died {
        push(
            &mut ev,
            yb as i32,
            Formative::ParentDied {
                parent: parent(p),
                cause: cause.into(),
            },
            &mut r,
        );
        questions.push((Topic::Death, yb as i32, parent(p)));
    }
    if let Some((sib, yb, cause)) = &house.sibling_died {
        push(
            &mut ev,
            *yb as i32,
            Formative::SiblingDied {
                sibling: sib.clone(),
                cause: (*cause).into(),
            },
            &mut r,
        );
        questions.push((Topic::Death, *yb as i32, sib.clone()));
    }
    // The village, as far as their memory reaches (from about four years old).
    let mut fires_seen = 0;
    for (yb, what) in village(seed) {
        let yb = yb as i32;
        if age - yb < 4 {
            continue;
        }
        match &what {
            Formative::Famine => questions.push((Topic::Hunger, yb, "the hungry winter".into())),
            Formative::Raid => questions.push((Topic::Strangers, yb, "the raiders".into())),
            Formative::VillageFire => {
                fires_seen += 1;
                // Some households burned; some of the young helped beat it out.
                if r.gen_bool(0.3) {
                    push(&mut ev, yb, Formative::HouseFire, &mut r);
                }
                if age - yb >= 9 && r.gen_bool(0.45) {
                    push(&mut ev, yb, Formative::FoughtFire, &mut r);
                }
                questions.push((Topic::Fire, yb, "the fire".into()));
            }
            _ => {}
        }
        push(&mut ev, yb, what, &mut r);
    }
    let _ = fires_seen;
    // Their own life.
    let mut skill_bonus = Vec::new();
    if age >= 15 && r.gen_bool(0.7) {
        let (craft, skill) = CRAFTS[r.gen_range(0..CRAFTS.len())];
        let at = r.gen_range(11..15).min(age - 1);
        push(
            &mut ev,
            age - at,
            Formative::Apprenticed {
                craft: craft.into(),
                master: other_name(seed, index as u64 + 100, rerolls as u64),
            },
            &mut r,
        );
        skill_bonus.push((skill, 2.5));
    }
    if r.gen_bool(0.12) {
        let at = r.gen_range(5..11).min(age - 1);
        push(&mut ev, age - at, Formative::LostInWoods, &mut r);
        skill_bonus.push((Skill::Foraging, 1.0));
    }
    if age >= 20 && r.gen_bool(0.5) {
        let at = r.gen_range(17..(age.min(32) + 1));
        let partner = other_name(seed, index as u64 + 200, rerolls as u64);
        push(
            &mut ev,
            age - at,
            Formative::Married {
                partner: partner.clone(),
            },
            &mut r,
        );
        let mut children = Vec::new();
        if age - at >= 2 && r.gen_bool(0.55) {
            let cat = at + r.gen_range(1..(age - at).clamp(2, 8));
            let child = other_name(seed, index as u64 + 300, rerolls as u64);
            push(
                &mut ev,
                age - cat,
                Formative::ChildBorn {
                    child: child.clone(),
                },
                &mut r,
            );
            if age - cat >= 2 && r.gen_bool(0.18) {
                let dyb = (age - cat) - r.gen_range(1..(age - cat).clamp(2, 5));
                push(
                    &mut ev,
                    dyb.max(1),
                    Formative::ChildDied {
                        child: child.clone(),
                    },
                    &mut r,
                );
                questions.push((Topic::Death, dyb.max(1), child.clone()));
            } else {
                children.push(child);
            }
        }
        // Those they could not bring through.
        let mut left = vec![partner];
        left.extend(children);
        for who in left {
            ev.push(PastEvent {
                years_before: 0,
                day: 0,
                age: age as u8,
                what: Formative::LeftBehind { who },
            });
        }
    }
    if age >= 16 && r.gen_bool(0.15) {
        let by = other_name(seed, index as u64 + 400, rerolls as u64);
        let yb = r.gen_range(1..(age - 14).max(2));
        push(&mut ev, yb, Formative::Betrayed { by: by.clone() }, &mut r);
        questions.push((Topic::Trust, yb, by));
    }
    if age >= 12 && r.gen_bool(0.08) {
        let who = other_name(seed, index as u64 + 500, rerolls as u64);
        let yb = r.gen_range(1..(age - 10).max(2));
        push(&mut ev, yb, Formative::SavedFromWater { who }, &mut r);
    }
    // Answers: each topic once, to its earliest question, a few years after.
    let shown_otherwise = house.elder.is_some();
    let fought = ev.iter().any(|e| e.what == Formative::FoughtFire);
    questions.sort_by_key(|q| std::cmp::Reverse(q.1));
    let mut answers: Vec<(Topic, f32)> = Vec::new();
    for (topic, yb, about) in questions {
        if answers.iter().any(|(t, _)| *t == topic) {
            continue;
        }
        let mut s = answer(topic, traits, shown_otherwise, &mut r);
        if topic == Topic::Fire && fought {
            s = (s - 0.6).max(-1.0);
        }
        answers.push((topic, s));
        // The answer comes a few years after the question, and not before twelve.
        let when = (yb - r.gen_range(1..4)).min(age - 12).max(1);
        push(
            &mut ev,
            when,
            Formative::Answered {
                topic,
                stance: (s * 100.0).round() as i8,
                about,
            },
            &mut r,
        );
    }
    ev.sort_by(|a, b| b.years_before.cmp(&a.years_before).then(a.day.cmp(&b.day)));
    PastLife {
        home: home_name(seed),
        household: hid,
        age: age as u32,
        events: ev,
        answers,
        skill_bonus,
    }
}

/// One line of a past, in the world's own words. `name` is whose life it is.
pub fn narrate(name: &str, age: u8, what: &Formative) -> String {
    use Formative::*;
    match what {
        ParentDrank { parent } => format!("{name}, aged {age}, watched {parent} drink the house into silence."),
        ParentViolent { parent } => format!("{name}, aged {age}, learned to read {parent}'s moods, and to stay out of reach."),
        ParentDied { parent, cause } => format!("{name} was {age} when {parent} died of {cause}."),
        SiblingDied { sibling, cause } => format!("{name} was {age} when {sibling} was taken by {cause}."),
        KindElder { elder } => format!("{name}, aged {age}, spent the long evenings at {elder}'s fire, being shown another way."),
        Famine => format!("{name} was {age} in the hungry winter, when the village ate its seed corn."),
        HouseFire => format!("{name} was {age} when the family's house burned."),
        VillageFire => format!("{name} was {age} when fire went through the village."),
        Raid => format!("{name} was {age} when raiders came over the hill."),
        Fever => format!("{name} was {age} the year the fever went house to house."),
        Flood => format!("{name} was {age} when the river came up over the fields."),
        Apprenticed { craft, master } => format!("{name}, aged {age}, was apprenticed to {craft}, {master}."),
        Married { partner } => format!("{name} married {partner} at {age}."),
        ChildBorn { child } => format!("{name} was {age} when {child} was born."),
        ChildDied { child } => format!("{name} was {age} when {child} died."),
        Betrayed { by } => format!("{name}, aged {age}, was betrayed by {by}."),
        SavedFromWater { who } => format!("{name}, aged {age}, pulled {who} out of the river."),
        LostInWoods => format!("{name} was lost in the woods for two nights at {age}, and came home."),
        FoughtFire => format!("{name}, aged {age}, stood in the line beating out the flames, and saw it work."),
        LeftBehind { who } => format!("{name} could not bring {who} through the portal."),
        Answered { topic, stance, about } => {
            let s = *stance as f32 / 100.0;
            let firm = if s.abs() > 0.5 { "" } else { "half-" };
            match (topic, s < 0.0) {
                (Topic::Drink, true) => format!("{name} {firm}swore, at {age}, never to drink as {about} had."),
                (Topic::Drink, false) => format!("{name} took to drink at {age}, as {about} had."),
                (Topic::Violence, true) => format!("{name} decided at {age} never to raise a hand the way {about} did."),
                (Topic::Violence, false) => format!("{name} learned from {about} that a fist settles things."),
                (Topic::Fire, true) => format!("After {about}, {name} {firm}believed a fire is something you fight."),
                (Topic::Fire, false) => format!("After {about}, {name} {firm}believed a fire is something you run from."),
                (Topic::Hunger, true) => format!("After {about}, {name} {firm}believed that in hunger you share."),
                (Topic::Hunger, false) => format!("After {about}, {name} {firm}believed that in hunger you keep what you have."),
                (Topic::Strangers, true) => format!("After {about}, {name} still opened the door to strangers."),
                (Topic::Strangers, false) => format!("After {about}, {name} {firm}believed strangers bring harm."),
                (Topic::Death, true) => format!("After losing {about}, {name} grew hard to reach."),
                (Topic::Death, false) => format!("After losing {about}, {name} held the living closer."),
                (Topic::Trust, true) => format!("{name} forgave {about}, in the end."),
                (Topic::Trust, false) => format!("After {about}, {name} {firm}stopped trusting easily."),
                (t, _) => format!(
                    "After {about}, {name} {firm}believed that {}.",
                    crate::mind::beliefs::phrase(*t, s)
                ),
            }
        }
    }
}
