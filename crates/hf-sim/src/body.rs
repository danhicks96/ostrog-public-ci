//! The body: parts that can be wounded, blood that can be lost, illness that can be
//! caught, ailments that come with age. Changed only by systems/health.rs and the places
//! that inflict harm (work accidents, hunts, cave-ins); read by decisions and the viewer.

use serde::{Deserialize, Serialize};

/// A part of the body: an id in the body plans' fixed vocabulary (plan.rs). Saves before P1
/// named the six human parts; they load as the same ids.
pub type Part = crate::plan::PartId;

/// What became of a part beyond a wound that heals (M5 fills these in: lost limbs,
/// prosthetics). A part with no entry is whole.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum PartState {
    Scarred,
    Lost,
    /// Replaced by a made thing: "an oak leg".
    Replaced(String),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum WoundKind {
    Cut,
    Bruise,
    Break,
    Burn,
    Frostbite,
    /// Appended (P11): two small wounds, from a bite.
    Puncture,
}

impl WoundKind {
    pub fn label(self) -> &'static str {
        match self {
            WoundKind::Cut => "cut",
            WoundKind::Bruise => "bruise",
            WoundKind::Break => "break",
            WoundKind::Burn => "burn",
            WoundKind::Frostbite => "frostbite",
            WoundKind::Puncture => "bite",
        }
    }
}

/// How deep a wound went (C74, PLAN-FINISH M5 "tissue layers"): what it takes to heal, and
/// what treatment changes its course.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub enum Tissue {
    #[default]
    Skin,
    Muscle,
    Bone,
    /// Into the body or the head: bleeds inside until it is stitched.
    Organ,
}

impl Tissue {
    pub fn label(self) -> &'static str {
        match self {
            Tissue::Skin => "skin",
            Tissue::Muscle => "to the muscle",
            Tissue::Bone => "to the bone",
            Tissue::Organ => "deep inside",
        }
    }
    /// How fast it heals against a graze.
    pub fn heal(self) -> f32 {
        match self {
            Tissue::Skin => 1.0,
            Tissue::Muscle => 0.6,
            Tissue::Bone => 0.35,
            Tissue::Organ => 0.4,
        }
    }
    /// The layer a wound of `kind` and `severity` on `part` reaches.
    pub fn of(kind: WoundKind, severity: f32, part: Part) -> Tissue {
        let core = part == Part::TORSO || part == Part::HEAD;
        match kind {
            WoundKind::Break => Tissue::Bone,
            WoundKind::Cut | WoundKind::Puncture if core && severity >= 0.6 => Tissue::Organ,
            _ if severity >= 0.35 => Tissue::Muscle,
            _ => Tissue::Skin,
        }
    }
}

/// What a healer did for a wound (C74).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Treatment {
    /// Washed and bound: a graze, a bruise.
    Dressed,
    /// Sewn shut: a deep cut or bite; stops the bleeding, inside too, and keeps it clean.
    Stitched,
    /// Set and splinted: a break heals straight.
    Splinted,
    /// A salve over a burn or frostbite.
    Salved,
}

impl Treatment {
    pub fn label(self) -> &'static str {
        match self {
            Treatment::Dressed => "dressed",
            Treatment::Stitched => "stitched",
            Treatment::Splinted => "splinted",
            Treatment::Salved => "salved",
        }
    }
    /// What a healer does for a wound of this kind and depth.
    pub fn for_wound(kind: WoundKind, tissue: Tissue) -> Treatment {
        match (kind, tissue) {
            (WoundKind::Break, _) => Treatment::Splinted,
            (WoundKind::Burn | WoundKind::Frostbite, _) => Treatment::Salved,
            (_, Tissue::Muscle | Tissue::Organ) => Treatment::Stitched,
            _ => Treatment::Dressed,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Wound {
    pub part: Part,
    pub kind: WoundKind,
    /// 0..1; heals toward 0.
    pub severity: f32,
    /// 0..1 blood lost per hour while it lasts; clots by itself except for breaks.
    pub bleeding: f32,
    pub infected: bool,
    pub since: u64,
    /// Who struck it, in a fight (P11: a killing has a killer).
    #[serde(default)]
    pub by: Option<crate::ids::AgentId>,
    /// How deep it went (C74). An old save's wounds are skin deep.
    #[serde(default)]
    pub tissue: Tissue,
    /// What a healer did for it, if anything (C74).
    #[serde(default)]
    pub treated: Option<Treatment>,
    /// Struck in a fight between people of the valley (#404): a death or a lost limb from it is
    /// a killing or a maiming at `by`'s hand. Not a blow struck in defence or by a stranger.
    #[serde(default)]
    pub in_anger: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Ailment {
    BadBack,
    WeakLungs,
    DimEyes,
}

impl Ailment {
    pub fn label(self) -> &'static str {
        match self {
            Ailment::BadBack => "a bad back",
            Ailment::WeakLungs => "weak lungs",
            Ailment::DimEyes => "dim eyes",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Illness {
    /// 0..1; death at 1.
    pub severity: f32,
    pub since: u64,
    /// Which sickness it is (#328); old saves' were a winter cough or a wound gone bad.
    #[serde(default)]
    pub kind: IllnessKind,
    /// A healer's answer to it: Some(true) the right one, Some(false) a wrong one (#328).
    #[serde(default)]
    pub treated: Option<bool>,
}

/// The valley's sicknesses (#328), each with its own course and its own cause in what people
/// did: a wound let go bad, crowded winter quarters, the dead left lying, the hunt.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IllnessKind {
    /// A wound gone bad: lasts while the wound is foul.
    WoundRot,
    /// Caught close in the cold months; passes in days; no lasting guard against it.
    #[default]
    WinterCough,
    /// Builds two days, then breaks or worsens; once had, never again.
    Fever,
    /// From the dead left lying near: the gut; only the right care turns it.
    Flux,
    /// Slow and lifelong, in the old and the long-hungry: never wholly lifts.
    Wasting,
    /// Caught from a beast's blood on the hunt: a fever's course; once had, never again.
    BeastFever,
    /// Appended (#286): a look-alike plant eaten in error. Worst in the first hours, then it
    /// passes if the eater lives.
    Poisoned,
}

impl IllnessKind {
    pub fn name(self) -> &'static str {
        match self {
            IllnessKind::WoundRot => "a wound gone bad",
            IllnessKind::WinterCough => "the winter cough",
            IllnessKind::Fever => "a fever",
            IllnessKind::Flux => "the flux",
            IllnessKind::Wasting => "the wasting",
            IllnessKind::BeastFever => "beast fever",
            IllnessKind::Poisoned => "a poisoning",
        }
    }
    /// Whether having it once guards against it after.
    pub fn leaves_immune(self) -> bool {
        matches!(self, IllnessKind::Fever | IllnessKind::BeastFever)
    }
    /// Severity per hour before rest and care: by how long it has run (hours).
    pub fn course(self, hours: u64, infected: bool) -> f32 {
        match self {
            IllnessKind::WoundRot => {
                if infected {
                    0.012
                } else {
                    -0.004
                }
            }
            IllnessKind::WinterCough => -0.004,
            IllnessKind::Fever | IllnessKind::BeastFever => {
                if hours < 48 {
                    0.006
                } else {
                    0.001
                }
            }
            IllnessKind::Flux => 0.003,
            IllnessKind::Wasting => 0.0,
            IllnessKind::Poisoned => {
                if hours < 8 {
                    0.008
                } else {
                    -0.006
                }
            }
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Body {
    pub wounds: Vec<Wound>,
    /// 1 = full; death below 0.3.
    pub blood: f32,
    pub illness: Option<Illness>,
    pub ailments: Vec<Ailment>,
    pub scars: u8,
    /// Parts that are no longer whole (P1; empty until M5's injuries).
    #[serde(default)]
    pub parts: std::collections::BTreeMap<Part, PartState>,
    /// A beast's body that is not a skeleton (#423): what it is and how much is left of it.
    /// None for every skeleton, and in every save before it.
    #[serde(default)]
    pub form: Option<Form>,
}

/// A body that is not a skeleton (#423): a swarm counted in members, a chain in segments, a
/// soft body in hundredths of its mass. It does not bleed or take wounds as limbs do: a blow
/// takes some of it away (`health::wound`), and below `least` it can no longer live as one.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Form {
    pub kind: crate::plan::Kind,
    /// How much of it there is now.
    pub units: u16,
    /// How much of it there was, whole.
    pub whole: u16,
}

impl Form {
    /// A whole body of `plan`'s kind, its size within the plan's count by `pick` (0..1, from the
    /// genome). None for a skeleton.
    pub fn whole_of(plan: &crate::plan::Plan, pick: f32) -> Option<Form> {
        plan.kind.counted()?;
        let [lo, hi] = plan.count;
        let n = lo + ((hi - lo) as f32 * pick.clamp(0.0, 1.0)).round() as u16;
        Some(Form {
            kind: plan.kind,
            units: n,
            whole: n,
        })
    }

    /// What is left of it, 0..1.
    pub fn share(self) -> f32 {
        self.units as f32 / self.whole.max(1) as f32
    }

    /// The least it lives on: a chain its head and two segments; a swarm or a soft body a fifth
    /// of itself.
    pub fn least(self) -> u16 {
        match self.kind {
            crate::plan::Kind::Segmented => 2,
            _ => self.whole.div_ceil(5).max(1),
        }
    }

    /// Too little is left to live as one creature.
    pub fn spent(self) -> bool {
        self.units < self.least()
    }

    /// How near it is to spent: 0 whole, 1 spent. What `combat::battered` reads for it.
    pub fn harm(self) -> f32 {
        let span = (self.whole + 1).saturating_sub(self.least()).max(1);
        (self.whole.saturating_sub(self.units) as f32 / span as f32).min(1.0)
    }

    /// The words for how it ends when spent: "scattered", "broken up", "cut to pieces".
    pub fn end(self) -> &'static str {
        match self.kind {
            crate::plan::Kind::Swarm => "scattered",
            crate::plan::Kind::Soft => "broken up",
            _ => "cut to pieces",
        }
    }

    /// For the inspect panel: "24 of its 40 left", "60% of its mass", "9 of 12 segments".
    pub fn label(self) -> String {
        match self.kind {
            crate::plan::Kind::Swarm => format!("{} of its {} left", self.units, self.whole),
            crate::plan::Kind::Soft => {
                format!("{}% of its mass", (self.share() * 100.0).round() as i32)
            }
            _ => format!("{} of {} segments", self.units, self.whole),
        }
    }
}

impl Default for Body {
    fn default() -> Self {
        Body {
            wounds: Vec::new(),
            blood: 1.0,
            illness: None,
            ailments: Vec::new(),
            scars: 0,
            parts: Default::default(),
            form: None,
        }
    }
}

impl Body {
    pub fn worst_wound(&self) -> f32 {
        self.wounds.iter().map(|w| w.severity).fold(0.0, f32::max)
    }
    /// One agreed number for how much hurts, 0..1 (#1344): every wound's severity weighed by
    /// its kind and depth, eased by treatment and tending, plus illness and lasting harm.
    pub fn pain(&self, now: u64, tended_until: u64) -> f32 {
        let kind_weight = |k: WoundKind| match k {
            WoundKind::Break => 1.2,
            WoundKind::Burn => 1.1,
            WoundKind::Cut | WoundKind::Puncture => 1.0,
            WoundKind::Frostbite => 0.8,
            WoundKind::Bruise => 0.6,
        };
        let depth_weight = |t: Tissue| match t {
            Tissue::Skin => 0.6,
            Tissue::Muscle => 1.0,
            Tissue::Bone => 1.3,
            Tissue::Organ => 1.4,
        };
        let mut p: f32 = self
            .wounds
            .iter()
            .map(|w| {
                let mut v = w.severity * kind_weight(w.kind) * depth_weight(w.tissue);
                if w.treated.is_some() {
                    v *= 0.6;
                }
                v
            })
            .sum();
        p += self.ill() * 0.5;
        p += self
            .parts
            .values()
            .filter(|s| matches!(s, PartState::Scarred))
            .count() as f32
            * 0.05;
        p += self
            .parts
            .values()
            .filter(|s| matches!(s, PartState::Lost))
            .count() as f32
            * 0.03;
        if self.ailments.contains(&Ailment::BadBack) {
            p += 0.1;
        }
        if tended_until > now {
            p *= 0.7;
        }
        p.clamp(0.0, 1.0)
    }
    pub fn ill(&self) -> f32 {
        self.illness.map(|i| i.severity).unwrap_or(0.0)
    }
    /// Hurt enough to show, to slow down, and to be excused from hard work.
    pub fn hurt(&self) -> bool {
        self.worst_wound() >= 0.2 || self.ill() >= 0.3 || self.blood < 0.7
    }
    /// Should be in a bed being looked after.
    pub fn needs_care(&self) -> bool {
        self.worst_wound() >= 0.35 || self.ill() >= 0.4 || self.wounds.iter().any(|w| w.infected)
    }
    /// Work speed factor from the body alone.
    pub fn work_factor(&self) -> f32 {
        let mut f = 1.0 - self.worst_wound() * 0.5 - self.ill() * 0.4;
        // A limb set crooked works less well for good (C74).
        let crooked = self
            .parts
            .values()
            .filter(|s| matches!(s, PartState::Scarred))
            .count();
        f -= 0.08 * crooked as f32;
        // A limb lost works not at all; one replaced by a made thing, part way (#284).
        for st in self.parts.values() {
            match st {
                PartState::Lost => f -= 0.25,
                PartState::Replaced(_) => f -= 0.1,
                PartState::Scarred => {}
            }
        }
        if self.ailments.contains(&Ailment::BadBack) {
            f *= 0.8;
        }
        f.max(0.3)
    }
    pub fn summary(&self) -> String {
        let mut parts: Vec<String> = self
            .wounds
            .iter()
            .map(|w| {
                // The layer and what was done for it (C74, M5's "wound layers on inspect").
                format!(
                    "{} {} ({}, {}%{}{})",
                    w.kind.label(),
                    w.part.label(),
                    w.tissue.label(),
                    (w.severity * 100.0).round() as i32,
                    w.treated
                        .map(|t| format!(", {}", t.label()))
                        .unwrap_or_default(),
                    if w.infected { ", infected" } else { "" }
                )
            })
            .collect();
        if let Some(i) = self.illness {
            parts.push(format!("fever {}%", (i.severity * 100.0).round() as i32));
        }
        if self.blood < 0.95 {
            parts.push(format!("blood {}%", (self.blood * 100.0).round() as i32));
        }
        for a in &self.ailments {
            parts.push(a.label().to_string());
        }
        for (p, st) in &self.parts {
            match st {
                PartState::Scarred => parts.push(format!("{} set crooked", p.label())),
                PartState::Lost => parts.push(format!("{} lost", p.label())),
                PartState::Replaced(what) => {
                    parts.push(format!("{} replaced by {what}", p.label()))
                }
            }
        }
        if let Some(f) = self.form {
            parts.push(f.label());
        }
        if self.scars > 0 {
            parts.push(format!(
                "{} scar{}",
                self.scars,
                if self.scars > 1 { "s" } else { "" }
            ));
        }
        if parts.is_empty()
            || self
                .form
                .is_some_and(|f| f.units == f.whole && parts.len() == 1)
        {
            "sound".to_string()
        } else {
            parts.join(", ")
        }
    }
}
