//! Techniques: ways of doing something that a person knows or does not (design-organic.md
//! section 3). Nobody knows all of them. Survivors bring what their past taught them; anyone
//! can learn by watching someone succeed, or by trying something untried and seeing it work.
//! The settlement planner never sends anyone to a crisis: whoever is there responds with what
//! they know, weighted by what they believe, or does not.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Technique {
    /// Beat out a fire on open ground with whatever is to hand.
    BeatFlames,
    /// Carry water from the nearest source and throw it on the fire.
    CarryWater,
    /// Dress wounds and sickness with a poultice.
    Poultice,
    /// When food is short, range far beyond the usual ground to find it.
    ForageFar,
    // Old crafts of the valley (deep.rs), learned by studying what the hamlets left (C13).
    /// Setting a true edge: tools made by someone who knows it come out better.
    Smithing,
    /// Laying a roof that sheds snow: roofs they lay bear more.
    Roofing,
    /// Weaving close: their cloth and clothes are better.
    Weaving,
    /// The old herb lore: those they tend mend faster, their poultices are stronger.
    HerbLore,
    /// Old hearth cooking: their meals are better.
    Hearthcraft,
    /// Propping a dug gallery with timber before cutting further (underground.rs).
    Shoring,
    /// Understanding the thin place at the portal stone, from sitting by it (thin.rs).
    ThinLore,
    /// Appended (P11): lifting the Beast-hunger or the Thirst, a healers' tradition some
    /// peoples keep (conditions.rs).
    Unbinding,
    /// The crafts of the benches (C70, Dan's rule 3): a person knows them or does not, and
    /// only those who do work the bench. Appended.
    Carpentry,
    Masonry,
    Smelting,
    Potting,
    Brewing,
}

/// The crafts a bench's recipe may ask for (C70): the ones a community can lose.
pub const CRAFTS: [Technique; 8] = [
    Technique::Carpentry,
    Technique::Masonry,
    Technique::Smithing,
    Technique::Smelting,
    Technique::Weaving,
    Technique::Potting,
    Technique::Brewing,
    Technique::HerbLore,
];

impl Technique {
    pub fn name(self) -> &'static str {
        match self {
            Technique::BeatFlames => "beating out flames",
            Technique::CarryWater => "carrying water to a fire",
            Technique::Poultice => "making and laying a poultice",
            Technique::ForageFar => "foraging far afield",
            Technique::Smithing => "setting a true edge",
            Technique::Roofing => "laying a roof that sheds snow",
            Technique::Weaving => "weaving close",
            Technique::HerbLore => "the old herb lore",
            Technique::Hearthcraft => "old hearth cooking",
            Technique::Shoring => "propping a gallery before cutting further",
            Technique::ThinLore => "the ways of the thin place",
            Technique::Unbinding => "the unbinding of curses",
            Technique::Carpentry => "carpentry",
            Technique::Masonry => "dressing stone",
            Technique::Smelting => "smelting",
            Technique::Potting => "potting",
            Technique::Brewing => "brewing",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Learned {
    /// From their life before (or what any adult of their skills would know).
    Past,
    /// Saw someone do it and it worked.
    Watched,
    /// Tried it without knowing how, and it worked.
    Tried,
    /// Worked it out from something the old hamlets left (a record, a tool).
    Studied,
    /// At the bench, from one who knows it (C70).
    Taught,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Know {
    pub technique: Technique,
    /// 0..1: how well they do it. Grows with use.
    pub skill: f32,
    pub learned: Learned,
    pub wins: u16,
    pub losses: u16,
    pub since: u64,
    /// Where they learned it, in words ("from a record left at Hobhythe"), or "".
    #[serde(default)]
    pub from: String,
}

#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Techniques {
    pub list: Vec<Know>,
}

impl Techniques {
    pub fn knows(&self, t: Technique) -> bool {
        self.list.iter().any(|k| k.technique == t)
    }

    pub fn skill(&self, t: Technique) -> f32 {
        self.list
            .iter()
            .find(|k| k.technique == t)
            .map(|k| k.skill)
            .unwrap_or(0.0)
    }

    /// Learn a technique (or get better at one already known).
    pub fn learn(&mut self, t: Technique, how: Learned, skill: f32, now: u64) -> bool {
        if let Some(k) = self.list.iter_mut().find(|k| k.technique == t) {
            k.skill = k.skill.max(skill).min(1.0);
            return false;
        }
        self.list.push(Know {
            technique: t,
            skill: skill.clamp(0.0, 1.0),
            learned: how,
            wins: 0,
            losses: 0,
            since: now,
            from: String::new(),
        });
        self.list.sort_by_key(|k| k.technique);
        true
    }

    /// Learn it, and remember where from.
    pub fn learn_from(
        &mut self,
        t: Technique,
        how: Learned,
        skill: f32,
        now: u64,
        from: &str,
    ) -> bool {
        let new = self.learn(t, how, skill, now);
        if new {
            if let Some(k) = self.list.iter_mut().find(|k| k.technique == t) {
                k.from = from.to_string();
            }
        }
        new
    }

    /// The bonus a known technique gives (its skill), 0 if unknown.
    pub fn bonus(&self, t: Technique) -> f32 {
        self.skill(t)
    }

    /// How it went. A success teaches someone who did not know (they tried, it worked); a
    /// failure teaches nothing but costs a little confidence.
    pub fn outcome(&mut self, t: Technique, won: bool, now: u64) {
        if won && !self.knows(t) {
            self.learn(t, Learned::Tried, 0.2, now);
        }
        if let Some(k) = self.list.iter_mut().find(|k| k.technique == t) {
            if won {
                k.wins = k.wins.saturating_add(1);
                k.skill = (k.skill + 0.06 * (1.0 - k.skill)).min(1.0);
            } else {
                k.losses = k.losses.saturating_add(1);
                k.skill = (k.skill - 0.02).max(0.05);
            }
        }
    }

    /// What an adult of these skills and temperament would know from an ordinary life: those
    /// who have hauled water know how to carry it to a fire; the steady-handed and the hardy
    /// have beaten out a grass fire; cooks and herb-gatherers know a poultice; good foragers
    /// know to go far when food is short. Not everyone knows everything, and some know nothing.
    pub fn common(skills: &crate::mind::Skills, traits: &crate::mind::Traits, now: u64) -> Self {
        use crate::mind::Skill;
        use crate::mind::Trait;
        let mut t = Techniques::default();
        if skills.level(Skill::Hauling) >= 4.0 {
            t.learn(Technique::CarryWater, Learned::Past, 0.35, now);
        }
        // Those who have worked in a mine know to prop it.
        if skills.level(Skill::Mining) >= 5.0 {
            t.learn(Technique::Shoring, Learned::Past, 0.45, now);
        }
        if traits.has(Trait::Hearty) || traits.has(Trait::Industrious) {
            t.learn(Technique::BeatFlames, Learned::Past, 0.3, now);
        }
        if skills.level(Skill::Cooking) >= 5.0 || skills.level(Skill::Foraging) >= 7.0 {
            t.learn(Technique::Poultice, Learned::Past, 0.35, now);
        }
        if skills.level(Skill::Foraging) >= 5.0 {
            t.learn(Technique::ForageFar, Learned::Past, 0.4, now);
        }
        // The crafts of a trade (C70): a seasoned builder knows carpentry, a seasoned miner
        // how stone is dressed, a seasoned maker how cloth is woven.
        if skills.level(Skill::Construction) >= 5.0 {
            t.learn(Technique::Carpentry, Learned::Past, 0.35, now);
        }
        if skills.level(Skill::Mining) >= 5.0 {
            t.learn(Technique::Masonry, Learned::Past, 0.3, now);
        }
        if skills.level(Skill::Crafting) >= 6.0 {
            t.learn(Technique::Weaving, Learned::Past, 0.3, now);
        }
        t
    }
}
