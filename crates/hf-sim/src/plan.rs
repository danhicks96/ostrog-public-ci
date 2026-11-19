//! Body plans (P1 of milestone MP, docs/design-peoples.md section 2.1). A plan is the skeleton
//! that the art rig and the body simulation share: which parts a body has and what each does.
//! Plans are data (`data/peoples/plans/*.toml`, built into the binary so every world reads the
//! same ones). Part ids are one fixed vocabulary across all plans, so a wound or a lost limb
//! saves as a small number and reads the same in every plan that has that part.
//!
//! Most plans are skeletons: a head, a chest and limbs. Some beasts are built otherwise (#423,
//! `Kind`): a soft mass with no fixed parts, a swarm of small bodies acting as one, or a chain
//! of segments. Those plans are beasts' only; a people's body is always a skeleton.

use std::sync::OnceLock;

use rand::Rng;
use serde::{Deserialize, Serialize};

/// A part of a body, from the fixed vocabulary below.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PartId(pub u8);

/// The vocabulary: (id, name in plan files, words for it). Never reorder; only append.
const PARTS: [(&str, &str); 17] = [
    ("head", "head"),
    ("torso", "chest"),
    ("arm_l", "left arm"),
    ("arm_r", "right arm"),
    ("leg_l", "left leg"),
    ("leg_r", "right leg"),
    ("tail", "tail"),
    ("wing_l", "left wing"),
    ("wing_r", "right wing"),
    ("lower_arm_l", "lower left arm"),
    ("lower_arm_r", "lower right arm"),
    ("foreleg_l", "left foreleg"),
    ("foreleg_r", "right foreleg"),
    ("coil", "coils"),
    // Appended (#423): the parts of bodies that are not skeletons.
    ("mass", "body"),
    ("member", "members"),
    ("segment", "segments"),
];

impl PartId {
    pub const HEAD: PartId = PartId(0);
    pub const TORSO: PartId = PartId(1);
    pub const ARM_L: PartId = PartId(2);
    pub const ARM_R: PartId = PartId(3);
    pub const LEG_L: PartId = PartId(4);
    pub const LEG_R: PartId = PartId(5);
    /// The whole of a soft body (#423).
    pub const MASS: PartId = PartId(14);
    /// The small bodies of a swarm (#423).
    pub const MEMBER: PartId = PartId(15);
    /// The links of a segmented chain (#423).
    pub const SEGMENT: PartId = PartId(16);

    pub fn label(self) -> &'static str {
        PARTS.get(self.0 as usize).map(|p| p.1).unwrap_or("body")
    }

    pub fn key(self) -> &'static str {
        PARTS.get(self.0 as usize).map(|p| p.0).unwrap_or("?")
    }

    pub fn by_key(k: &str) -> Option<PartId> {
        PARTS.iter().position(|p| p.0 == k).map(|i| PartId(i as u8))
    }
}

impl Serialize for PartId {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(self.0)
    }
}

/// Saves before P1 named the six human parts (`"LeftLeg"`); newer ones store the id.
impl<'de> Deserialize<'de> for PartId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = PartId;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a part id or an old part name")
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<PartId, E> {
                u8::try_from(v)
                    .map(PartId)
                    .map_err(|_| E::custom("part id out of range"))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<PartId, E> {
                self.visit_u64(v as u64)
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<PartId, E> {
                Ok(match v {
                    "Head" => PartId::HEAD,
                    "Torso" => PartId::TORSO,
                    "LeftArm" => PartId::ARM_L,
                    "RightArm" => PartId::ARM_R,
                    "LeftLeg" => PartId::LEG_L,
                    "RightLeg" => PartId::LEG_R,
                    other => PartId::by_key(other).ok_or_else(|| E::custom("unknown part name"))?,
                })
            }
        }
        d.deserialize_any(V)
    }
}

/// What a part does for its body.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Sense,
    Vital,
    Manipulate,
    Locomote,
    Fly,
    Swim,
    Balance,
}

/// How a body is built (#423). Saved with a body that is not a skeleton (body.rs `Form`), so
/// never reorder; only append. Radial, translucent and crystalline bodies are to follow.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Kind {
    /// A head, a chest and limbs: every people's body and the valley's beasts.
    #[default]
    Skeletal,
    /// A slime or blob: one mass with no fixed parts, which can be split.
    Soft,
    /// Many small bodies acting as one creature: a wound takes members away.
    Swarm,
    /// A chain of segments behind a head: losing a segment shortens it.
    Segmented,
}

impl Kind {
    fn by_name(s: &str) -> Option<Kind> {
        Some(match s {
            "skeletal" => Kind::Skeletal,
            "soft" => Kind::Soft,
            "swarm" => Kind::Swarm,
            "segmented" => Kind::Segmented,
            _ => return None,
        })
    }

    /// The part a body of this kind is counted in: the swarm's members, the chain's segments,
    /// the soft body's mass. None for a skeleton.
    pub fn counted(self) -> Option<PartId> {
        match self {
            Kind::Skeletal => None,
            Kind::Soft => Some(PartId::MASS),
            Kind::Swarm => Some(PartId::MEMBER),
            Kind::Segmented => Some(PartId::SEGMENT),
        }
    }
}

impl Kind {
    /// The built-in plan of this kind (its file is named for it), for a body that is not a
    /// skeleton; the human one for a skeleton.
    pub fn plan(self) -> &'static Plan {
        let id = match self {
            Kind::Skeletal => return biped(),
            Kind::Soft => "soft",
            Kind::Swarm => "swarm",
            Kind::Segmented => "segmented",
        };
        plan(id).unwrap_or_else(biped)
    }
}

/// Whether this creature's body can do `a` (#423): only a body that is not a skeleton has
/// abilities beyond its limbs.
pub fn can(a: &crate::agents::Agent, ab: Ability) -> bool {
    a.body.form.is_some_and(|f| f.kind.plan().can(ab))
}

/// The body plan of a kind of the valley's beasts that is not a skeleton (#423); None for the
/// rest, whose bodies are told by their species alone. Creatures are added here with their
/// species.
pub fn beast_plan(s: crate::agents::Species) -> Option<&'static Plan> {
    use crate::agents::Species;
    plan(match s {
        Species::Slime => "soft",
        Species::Rats => "swarm",
        Species::Centipede => "segmented",
        _ => return None,
    })
}

/// How much there is of a whole soft body: it is counted in hundredths of its mass (#423).
pub const SOFT_WHOLE: u16 = 100;

/// What a part lets its body do beyond walking (#423). Abilities are properties of parts in the
/// plan files, so a body has them by what it is made of, not by a rule written for its species
/// (#337). Each is wired into the movement, combat or health code the valley already has.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ability {
    /// Goes up or down a level at a rock face where there is no way for a walker
    /// (`systems::animals::get_past`).
    Climb,
    /// A cutting blow that takes enough of it away makes a second body of what it took
    /// (`systems::health`, a soft body's wound).
    Split,
    /// Takes a much smaller creature into itself and feeds on it (`systems::animals::engulf`).
    Engulf,
    /// Gets through a built wall of wood, fence or palisade, by the gaps in it
    /// (`systems::animals::get_past`).
    Squeeze,
    /// Grows back what it lost while it is fed (`systems::health`, the hourly pass).
    Regrow,
}

#[derive(Clone, Debug)]
pub struct PartDef {
    pub id: PartId,
    pub roles: Vec<Role>,
    pub wound_weight: u32,
    pub optional: bool,
    /// "digitigrade", "webbed", or "".
    pub form: String,
    /// What the part lets its body do (#423); none for a limb.
    pub abilities: Vec<Ability>,
}

#[derive(Clone, Debug)]
pub struct Plan {
    pub id: String,
    /// Sprite cell in pixels.
    pub cell: u32,
    /// Footprint in tiles on a side.
    pub tiles: u32,
    pub parts: Vec<PartDef>,
    /// How the body is built (#423); a skeleton unless the file says otherwise.
    pub kind: Kind,
    /// How many members (a swarm) or segments (a chain) a grown one has, least and most; a soft
    /// body is `SOFT_WHOLE` hundredths of itself. [0, 0] for a skeleton.
    pub count: [u16; 2],
}

/// Which parts a harm can land on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PartFilter {
    Any,
    Role(Role),
    /// An extremity the cold takes first: a manipulating or walking part.
    Extremity,
}

impl Plan {
    /// Whether a people may have this body (#423: the new kinds are beasts only).
    pub fn for_people(&self) -> bool {
        self.kind == Kind::Skeletal
    }

    pub fn has(&self, p: PartId) -> bool {
        self.parts.iter().any(|d| d.id == p)
    }

    /// Whether some part of this body gives it `a` (#423).
    pub fn can(&self, a: Ability) -> bool {
        self.parts.iter().any(|d| d.abilities.contains(&a))
    }

    /// Hands to work and carry with: two for most, three for four arms (the lower pair helps,
    /// it does not double).
    pub fn hands(&self) -> f32 {
        let n = self
            .parts
            .iter()
            .filter(|d| d.roles.contains(&Role::Manipulate))
            .count();
        if n > 2 {
            2.0 + (n - 2) as f32 * 0.5
        } else {
            n as f32
        }
    }

    /// A part a harm lands on, weighted by each part's share of wounds.
    pub fn random_part<R: Rng>(&self, r: &mut R, filter: PartFilter) -> PartId {
        let ok = |d: &PartDef| match filter {
            PartFilter::Any => true,
            PartFilter::Role(x) => d.roles.contains(&x),
            PartFilter::Extremity => {
                d.roles.contains(&Role::Manipulate) || d.roles.contains(&Role::Locomote)
            }
        };
        let total: u32 = self
            .parts
            .iter()
            .filter(|d| ok(d))
            .map(|d| d.wound_weight)
            .sum();
        if total == 0 {
            return PartId::TORSO;
        }
        let mut pick = r.gen_range(0..total);
        for d in self.parts.iter().filter(|d| ok(d)) {
            if pick < d.wound_weight {
                return d.id;
            }
            pick -= d.wound_weight;
        }
        PartId::TORSO
    }
}

#[derive(Deserialize)]
struct PlanFile {
    id: String,
    cell: u32,
    tiles: u32,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    count: Option<[u16; 2]>,
    part: Vec<PartFile>,
}

#[derive(Deserialize)]
struct PartFile {
    id: String,
    roles: Vec<Role>,
    wound_weight: u32,
    #[serde(default)]
    optional: bool,
    #[serde(default)]
    form: String,
    #[serde(default)]
    abilities: Vec<Ability>,
}

/// The built-in plan files, in a fixed order (the first is the human one).
const FILES: [(&str, &str); 12] = [
    (
        "biped",
        include_str!("../../../data/peoples/plans/biped.toml"),
    ),
    (
        "small_biped",
        include_str!("../../../data/peoples/plans/small_biped.toml"),
    ),
    (
        "huge_biped",
        include_str!("../../../data/peoples/plans/huge_biped.toml"),
    ),
    (
        "digitigrade_tail",
        include_str!("../../../data/peoples/plans/digitigrade_tail.toml"),
    ),
    (
        "winged",
        include_str!("../../../data/peoples/plans/winged.toml"),
    ),
    (
        "four_armed",
        include_str!("../../../data/peoples/plans/four_armed.toml"),
    ),
    (
        "taur",
        include_str!("../../../data/peoples/plans/taur.toml"),
    ),
    (
        "amphibious",
        include_str!("../../../data/peoples/plans/amphibious.toml"),
    ),
    (
        "serpentine",
        include_str!("../../../data/peoples/plans/serpentine.toml"),
    ),
    // Beasts' bodies that are not skeletons (#423).
    (
        "soft",
        include_str!("../../../data/peoples/plans/soft.toml"),
    ),
    (
        "swarm",
        include_str!("../../../data/peoples/plans/swarm.toml"),
    ),
    (
        "segmented",
        include_str!("../../../data/peoples/plans/segmented.toml"),
    ),
];

/// Parses and checks one plan file: its id matches, every part is in the vocabulary and named
/// once, and the parts are those its kind of body has (`check_kind`).
pub fn parse(name: &str, text: &str) -> Result<Plan, String> {
    let f: PlanFile = toml::from_str(text).map_err(|e| format!("{name}: {e}"))?;
    if f.id != name {
        return Err(format!("{name}: id is {}", f.id));
    }
    let kind = match &f.kind {
        None => Kind::Skeletal,
        Some(k) => Kind::by_name(k).ok_or_else(|| format!("{name}: unknown kind {k}"))?,
    };
    let mut parts = Vec::new();
    for p in f.part {
        let id = PartId::by_key(&p.id).ok_or_else(|| format!("{name}: unknown part {}", p.id))?;
        if parts.iter().any(|d: &PartDef| d.id == id) {
            return Err(format!("{name}: part {} twice", p.id));
        }
        if p.wound_weight == 0 {
            return Err(format!("{name}: part {} can never be hurt", p.id));
        }
        parts.push(PartDef {
            id,
            roles: p.roles,
            wound_weight: p.wound_weight,
            optional: p.optional,
            form: p.form,
            abilities: p.abilities,
        });
    }
    let count = match (kind, f.count) {
        (Kind::Skeletal, None) => [0, 0],
        (Kind::Soft, None) => [SOFT_WHOLE, SOFT_WHOLE],
        (Kind::Swarm | Kind::Segmented, Some(c)) => c,
        (Kind::Swarm | Kind::Segmented, None) => {
            return Err(format!("{name}: a {kind:?} body needs a count"))
        }
        (_, Some(_)) => return Err(format!("{name}: only a swarm or a chain is counted")),
    };
    let plan = Plan {
        id: f.id,
        cell: f.cell,
        tiles: f.tiles,
        parts,
        kind,
        count,
    };
    if !plan.parts.iter().any(|d| d.roles.contains(&Role::Locomote)) {
        return Err(format!("{name}: nothing to move on"));
    }
    if !matches!(plan.cell, 32 | 48 | 64 | 96) || plan.tiles == 0 || plan.tiles > 2 {
        return Err(format!(
            "{name}: cell {} or footprint {}",
            plan.cell, plan.tiles
        ));
    }
    check_kind(name, &plan)?;
    Ok(plan)
}

/// The parts each kind of body has (#423). A skeleton has a head, a torso and a pair of hands,
/// and none of the counted parts. A soft body is its mass alone; a swarm its members alone; a
/// chain a head and its segments. None of the new kinds is more than one tile (#421, Dan
/// 2026-10-31), and a swarm or chain has at least two of what it is counted in.
fn check_kind(name: &str, plan: &Plan) -> Result<(), String> {
    let only =
        |want: &[PartId]| plan.parts.len() == want.len() && want.iter().all(|p| plan.has(*p));
    let vital = |p: PartId| {
        plan.parts
            .iter()
            .any(|d| d.id == p && d.roles.contains(&Role::Vital))
    };
    match plan.kind {
        Kind::Skeletal => {
            if [PartId::MASS, PartId::MEMBER, PartId::SEGMENT]
                .iter()
                .any(|p| plan.has(*p))
            {
                return Err(format!(
                    "{name}: a skeleton has no mass, members or segments"
                ));
            }
            if !plan.has(PartId::HEAD) || !plan.has(PartId::TORSO) {
                return Err(format!("{name}: a body needs a head and a torso"));
            }
            if plan.hands() < 2.0 {
                return Err(format!("{name}: a people needs a pair of hands"));
            }
            if plan.parts.iter().any(|d| !d.abilities.is_empty()) {
                return Err(format!("{name}: a skeleton's limbs do what limbs do"));
            }
            return Ok(());
        }
        Kind::Soft if !only(&[PartId::MASS]) || !vital(PartId::MASS) => {
            return Err(format!(
                "{name}: a soft body is one living mass and nothing else"
            ));
        }
        Kind::Swarm if !only(&[PartId::MEMBER]) || !vital(PartId::MEMBER) => {
            return Err(format!(
                "{name}: a swarm is its living members and nothing else"
            ));
        }
        Kind::Segmented if !only(&[PartId::HEAD, PartId::SEGMENT]) || !vital(PartId::HEAD) => {
            return Err(format!("{name}: a chain is a living head and its segments"));
        }
        _ => {}
    }
    // What only a soft body can do, and what only a counted part can grow back.
    for d in &plan.parts {
        let soft_only =
            d.abilities.contains(&Ability::Split) || d.abilities.contains(&Ability::Engulf);
        if soft_only && plan.kind != Kind::Soft {
            return Err(format!("{name}: only a soft body splits or engulfs"));
        }
        if d.abilities.contains(&Ability::Regrow) && Some(d.id) != plan.kind.counted() {
            return Err(format!("{name}: only what it is counted in grows back"));
        }
    }
    if plan.tiles != 1 {
        return Err(format!("{name}: a beast is one tile (#421)"));
    }
    let [lo, hi] = plan.count;
    if plan.kind != Kind::Soft && (lo < 2 || lo > hi || hi > 500) {
        return Err(format!("{name}: count {lo} to {hi}"));
    }
    Ok(())
}

/// Every built-in plan, parsed once.
pub fn plans() -> &'static [Plan] {
    static PLANS: OnceLock<Vec<Plan>> = OnceLock::new();
    PLANS.get_or_init(|| {
        FILES
            .iter()
            .map(|(n, t)| parse(n, t).unwrap_or_else(|e| panic!("built-in plan: {e}")))
            .collect()
    })
}

/// A built-in plan by id.
pub fn plan(id: &str) -> Option<&'static Plan> {
    plans().iter().find(|p| p.id == id)
}

/// The human plan.
pub fn biped() -> &'static Plan {
    &plans()[0]
}
