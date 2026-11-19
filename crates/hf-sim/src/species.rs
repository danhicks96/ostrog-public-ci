//! The species library (P3 of milestone MP, docs/design-peoples.md section 1). A species is
//! data: a body plan it is locked to, the ranges every individual's genome is read within
//! (size, lifespan), what it eats and senses, the climate it is comfortable in, where and how it
//! lives, the leanings its people start from, and the art it is drawn with. The built-in
//! species are files in `data/peoples/species`, compiled in so every world reads the same ones.
//! A species never changes a body's parts: culture dresses a body, injury alone changes it.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// Which species a person is: an index into the library. 0 is human, and every save before P3
/// reads as human.
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct SpeciesId(pub u16);

impl SpeciesId {
    pub const HUMAN: SpeciesId = SpeciesId(0);
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Size {
    pub height_cm: [f32; 2],
    pub mass_kg: [f32; 2],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Life {
    pub mature_years: u32,
    pub lifespan: [f32; 2],
    #[serde(default = "one_one")]
    pub litter: [u32; 2],
}

fn diurnal() -> String {
    "diurnal".into()
}

fn one_one() -> [u32; 2] {
    [1, 1]
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Diet {
    /// herbivore, omnivore, carnivore, fungivore.
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Senses {
    /// 0 none, 1 dim, 2 day, 3 keen.
    pub sight: u8,
    /// 0 none to 3: sees in the dark as by day.
    pub dark_sight: u8,
    /// 0 none to 3: hears through walls to 0, 6, 10, 14 tiles.
    pub hearing: u8,
    /// 0 none to 3: smells without sight to 0, 3, 6, 9 tiles.
    pub smell: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Climate {
    /// Cold stress below, heat stress above (°C).
    pub comfort_c: [f32; 2],
    /// bare, fur, feathers, scales.
    pub coat: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Habitat {
    /// surface, upper_caves, caverns, deep_caverns, underworld.
    pub layers: Vec<String>,
    /// houses, hollows, tents, nests.
    pub builds: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Social {
    /// band, clan, household, hive.
    pub structure: String,
    /// pair, group, none.
    pub bonding: String,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Temperament {
    /// Leanings per belief topic (drink, violence, fire, hunger, strangers, death, trust).
    #[serde(default)]
    pub convictions: BTreeMap<String, f32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Art {
    /// The body sheet set (`body_<body>_<build>_<age>.png`; "human" is the plain biped).
    pub body: String,
    /// Skin swap targets it may be drawn in, by name (swaps.json "skin" names).
    pub skin: Vec<String>,
    /// Whether it grows hair (the hair layer is drawn).
    pub hair: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Living {
    /// Ways of living open to its peoples in the world layer.
    pub ways: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpeciesDef {
    pub id: String,
    pub name: String,
    pub plural: String,
    pub sentient: bool,
    pub plan: String,
    pub rarity: String,
    /// diurnal, nocturnal (P4): when its people sleep.
    #[serde(default = "diurnal")]
    pub cycle: String,
    /// One line for the inspect panel and Legends: "small, quick-breeding hill folk ...".
    pub blurb: String,
    pub size: Size,
    pub life: Life,
    pub diet: Diet,
    pub senses: Senses,
    pub climate: Climate,
    pub habitat: Habitat,
    pub social: Social,
    #[serde(default)]
    pub temperament: Temperament,
    pub art: Art,
    pub living: Living,
    /// Species (file ids) it can have children with (P9, decision 1): only a pair that list
    /// each other. A child takes one parent's species whole.
    #[serde(default)]
    pub fertile_with: Vec<String>,
}

/// Skin swap targets by name, in the order of swaps.json "skin" (tools/art/people.py
/// SKIN_TONES): the six human tones, then the species' own. Never reorder; only append.
pub const SKIN_NAMES: [&str; 26] = [
    "deep",
    "dark",
    "tan",
    "olive",
    "fair",
    "pale",
    "ash",
    "bark",
    "lichen",
    "moss_skin",
    "slate_skin",
    "scale_rust",
    "scale_green",
    "scale_dun",
    "feather_brown",
    "feather_grey",
    "feather_white",
    "copper_skin",
    "umber",
    "dust",
    "chalk",
    "pale_lilac",
    "stone_grey",
    "basalt",
    "ember",
    "abyss",
];

const COATS: [&str; 4] = ["bare", "fur", "feathers", "scales"];
const DIETS: [&str; 4] = ["herbivore", "omnivore", "carnivore", "fungivore"];
const LAYERS: [&str; 5] = [
    "surface",
    "upper_caves",
    "caverns",
    "deep_caverns",
    "underworld",
];
const RARITIES: [&str; 4] = ["common", "uncommon", "rare", "legendary"];

/// Parses and checks one species file.
pub fn parse(name: &str, text: &str) -> Result<SpeciesDef, String> {
    let s: SpeciesDef = toml::from_str(text).map_err(|e| format!("{name}: {e}"))?;
    if s.id != name {
        return Err(format!("{name}: id is {}", s.id));
    }
    validate(&s)?;
    Ok(s)
}

/// The checks every species passes, from a file or from the generator.
pub fn validate(s: &SpeciesDef) -> Result<(), String> {
    let name = s.id.as_str();
    let bad = |what: &str| Err(format!("{name}: {what}"));
    match crate::plan::plan(&s.plan) {
        None => return bad(&format!("no body plan {}", s.plan)),
        // The soft, swarm and segmented bodies are beasts' only (#423, the default Dan may
        // overrule): every species in this library is a people's.
        Some(p) if !p.for_people() => {
            return bad(&format!("a {} body is a beast's, not a people's", s.plan))
        }
        Some(_) => {}
    }
    let ordered = |r: [f32; 2]| r[0] > 0.0 && r[0] <= r[1];
    if !ordered(s.size.height_cm) || !ordered(s.size.mass_kg) || !ordered(s.life.lifespan) {
        return bad("a size or lifespan range out of order");
    }
    if s.life.mature_years as f32 >= s.life.lifespan[0] || s.life.litter[0] == 0 {
        return bad("grown before it is born, or litters of none");
    }
    if s.climate.comfort_c[0] >= s.climate.comfort_c[1] {
        return bad("comfort range out of order");
    }
    if !COATS.contains(&s.climate.coat.as_str()) || !DIETS.contains(&s.diet.kind.as_str()) {
        return bad("unknown coat or diet");
    }
    if !matches!(s.cycle.as_str(), "diurnal" | "nocturnal") {
        return bad("unknown cycle");
    }
    if !RARITIES.contains(&s.rarity.as_str()) {
        return bad("unknown rarity");
    }
    if s.habitat.layers.is_empty()
        || s.habitat
            .layers
            .iter()
            .any(|l| !LAYERS.contains(&l.as_str()))
    {
        return bad("habitat layers");
    }
    if [
        s.senses.sight,
        s.senses.dark_sight,
        s.senses.hearing,
        s.senses.smell,
    ]
    .iter()
    .any(|v| *v > 3)
    {
        return bad("a sense above 3");
    }
    if s.temperament
        .convictions
        .keys()
        .any(|k| crate::past::Topic::from_name(k).is_none())
        || s.temperament.convictions.values().any(|v| v.abs() > 1.0)
    {
        return bad("temperament");
    }
    if s.art.skin.is_empty()
        || s.art.body.is_empty()
        || s.art.skin.iter().any(|k| !SKIN_NAMES.contains(&k.as_str()))
    {
        return bad("art");
    }
    Ok(())
}

/// The built-in species, in a fixed order: their index is their id in every save.
/// Never reorder; only append.
const FILES: [(&str, &str); 12] = [
    (
        "human",
        include_str!("../../../data/peoples/species/human.toml"),
    ),
    (
        "hollin",
        include_str!("../../../data/peoples/species/hollin.toml"),
    ),
    (
        "grisk",
        include_str!("../../../data/peoples/species/grisk.toml"),
    ),
    (
        "screeling",
        include_str!("../../../data/peoples/species/screeling.toml"),
    ),
    (
        "skarri",
        include_str!("../../../data/peoples/species/skarri.toml"),
    ),
    (
        "tharn",
        include_str!("../../../data/peoples/species/tharn.toml"),
    ),
    // The deep's peoples (P10): of the caverns, the deep caverns and the underworld.
    (
        "murrow",
        include_str!("../../../data/peoples/species/murrow.toml"),
    ),
    (
        "holm",
        include_str!("../../../data/peoples/species/holm.toml"),
    ),
    (
        "below",
        include_str!("../../../data/peoples/species/below.toml"),
    ),
    // The last three anchors (C50): the lakeshore's, the grassland's and the thin place's.
    (
        "meer",
        include_str!("../../../data/peoples/species/meer.toml"),
    ),
    (
        "gnarrow",
        include_str!("../../../data/peoples/species/gnarrow.toml"),
    ),
    (
        "seamfolk",
        include_str!("../../../data/peoples/species/seamfolk.toml"),
    ),
];

/// Every built-in species, parsed once.
pub fn library() -> &'static [SpeciesDef] {
    static LIB: OnceLock<Vec<SpeciesDef>> = OnceLock::new();
    LIB.get_or_init(|| {
        FILES
            .iter()
            .map(|(n, t)| parse(n, t).unwrap_or_else(|e| panic!("built-in species: {e}")))
            .collect()
    })
}

/// A species by id; an unknown id reads as human.
pub fn get(id: SpeciesId) -> &'static SpeciesDef {
    let lib = library();
    lib.get(id.0 as usize).unwrap_or(&lib[0])
}

/// Whether two species have children together: the same species, or two of the library that
/// list each other in `fertile_with` (P9). Generated species have children only with their own.
pub fn fertile(a: SpeciesId, b: SpeciesId) -> bool {
    if a == b {
        return true;
    }
    if a.0 >= GENERATED || b.0 >= GENERATED {
        return false;
    }
    let (x, y) = (get(a), get(b));
    x.fertile_with.contains(&y.id) && y.fertile_with.contains(&x.id)
}

/// A library species' file id from what someone typed: its id or its name, singular or plural,
/// in any case ("Tharn", "tharn", "the Below"). #147: the wizard hook was given display names.
pub fn resolve_key(typed: &str) -> Option<String> {
    let t = typed
        .trim()
        .trim_start_matches("the ")
        .trim_start_matches("The ");
    library()
        .iter()
        .find(|s| {
            s.id.eq_ignore_ascii_case(t)
                || s.name.eq_ignore_ascii_case(t)
                || s.plural.eq_ignore_ascii_case(t)
        })
        .map(|s| s.id.clone())
}

/// A species id by its file id ("grisk").
pub fn by_key(key: &str) -> Option<SpeciesId> {
    library()
        .iter()
        .position(|s| s.id == key)
        .map(|i| SpeciesId(i as u16))
}

/// The library's fingerprint: what a world was made with (sha256 of the files in order).
pub fn library_hash() -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for (n, t) in FILES {
        h.update(n.as_bytes());
        h.update(t.as_bytes());
    }
    hex::encode(&h.finalize()[..8])
}

impl SpeciesDef {
    pub fn nocturnal(&self) -> bool {
        self.cycle == "nocturnal"
    }

    /// How much of a food's worth this people gets from it (P4): meat feeds a carnivore well
    /// and a herbivore poorly, and so on. Omnivores take everything at its worth.
    pub fn diet_factor(&self, kind: crate::items::ItemKind) -> f32 {
        use crate::items::ItemKind as K;
        match (self.diet.kind.as_str(), kind) {
            ("carnivore", K::RawMeat) => 1.4,
            ("carnivore", K::Meal) => 1.0,
            ("carnivore", _) => 0.5,
            ("herbivore", K::RawMeat) => 0.3,
            ("herbivore", K::Meal) => 0.9,
            ("herbivore", _) => 1.1,
            ("fungivore", K::Root) => 1.0,
            ("fungivore", K::Meal) => 0.8,
            ("fungivore", _) => 0.6,
            _ => 1.0,
        }
    }

    /// How far this people sees in light `light` (0 dark to 1 day), in tiles (P4): keen eyes a
    /// little further, dim eyes half as far, none at all for the blind; dark sight keeps
    /// some or all of the day's reach in the dark.
    pub fn sight_radius(&self, light: f32) -> i32 {
        let eyes = [0.0, 0.5, 1.0, 1.15][self.senses.sight.min(3) as usize];
        if eyes == 0.0 {
            return 0;
        }
        let dark = [0.25, 0.5, 0.8, 1.0][self.senses.dark_sight.min(3) as usize];
        let r = crate::mind::perception::SIGHT_RADIUS as f32 * eyes * light.max(dark);
        (r.round() as i32).max(1)
    }

    /// How far this people hears people moving and working, through walls (P4).
    pub fn hearing_radius(&self) -> i32 {
        [0, 6, 10, 14][self.senses.hearing.min(3) as usize]
    }

    /// How far this people smells food and living things, without sight (P4).
    pub fn smell_radius(&self) -> i32 {
        [0, 3, 6, 9][self.senses.smell.min(3) as usize]
    }

    pub fn body_plan(&self) -> &'static crate::plan::Plan {
        crate::plan::plan(&self.plan).unwrap_or_else(crate::plan::biped)
    }

    /// "Grisk, small, quick-breeding hill folk ...: see in the dark; live 25 to 40 years".
    pub fn describe(&self) -> String {
        let mut senses = Vec::new();
        if self.senses.sight == 0 {
            senses.push("blind".to_string());
        }
        if self.senses.dark_sight >= 2 {
            senses.push("see in the dark".into());
        }
        if self.senses.hearing >= 2 {
            senses.push("hear through rock".into());
        }
        if self.senses.smell >= 2 {
            senses.push("hunt by smell".into());
        }
        let mut parts = vec![self.blurb.clone()];
        if !senses.is_empty() {
            parts.push(senses.join(", "));
        }
        parts.push(format!("eat as {}s", self.diet.kind));
        parts.push(format!(
            "live {:.0} to {:.0} years",
            self.life.lifespan[0], self.life.lifespan[1]
        ));
        format!("{}: {}", self.name, parts.join("; "))
    }
}

/// Makes the person `agent` one of `sp`, of the realm's people `people` if given: their species,
/// a skin from the species' own tones (read from the genome, so kin look alike), and a name in
/// that people's language.
pub fn give(
    w: &mut crate::world_state::World,
    agent: crate::ids::AgentId,
    sp: SpeciesId,
    people: Option<usize>,
) {
    let seed = w.meta.seed;
    let name = people.and_then(|p| w.realm.peoples.get(p)).map(|p| {
        let mut n = p.language.name(seed, "person", agent.0 as u64);
        if let Some(c) = n.get_mut(0..1) {
            c.make_ascii_uppercase();
        }
        n
    });
    let def = w.kind(sp);
    let Some(a) = w.agents.iter_mut().find(|a| a.id == agent) else {
        return;
    };
    a.people = sp;
    a.kind = (sp.0 >= GENERATED).then_some(def);
    // A grown stranger of another kind is of an age that kind is grown at (P5): a Screeling
    // arriving at forty-five would be at the end of its life, a Hollin at thirty still a child.
    if sp != SpeciesId::HUMAN {
        let now = w.meta.tick;
        let (adult, elder) = a.stage_years();
        let age = a.age_years(now);
        if age < adult || age as f32 > elder as f32 * 0.8 {
            let span = (elder as f32 * 0.8 - adult as f32).max(1.0);
            let f = a.genome.loci[crate::genetics::locus::CALM] as f32 / 255.0;
            let years = adult as f32 + f * span;
            a.born = now as i64 - (years * crate::time::TICKS_PER_YEAR as f32) as i64;
        }
    }
    let tone = def.art.skin
        [a.genome.loci[crate::genetics::locus::SKIN] as usize % def.art.skin.len()]
    .as_str();
    a.appearance.skin = SKIN_NAMES.iter().position(|k| *k == tone).unwrap_or(0) as u8;
    if let Some(n) = name {
        a.name = n;
    }
}

/// The rare people living as households in a settlement beyond the valley (P5), if any.
pub fn minority_of_site(w: &crate::world_state::World, site: u32) -> Option<(SpeciesId, usize)> {
    let s = w.overworld.sites.iter().find(|s| s.id == site)?;
    let rs = w.realm.sites.get(s.realm_site? as usize)?;
    let pi = rs.minority? as usize;
    let p = w.realm.peoples.get(pi)?;
    Some((p.species, pi))
}

/// The species and realm people of a settlement beyond the valley, if it is one of the world's.
pub fn of_site(w: &crate::world_state::World, site: u32) -> Option<(SpeciesId, usize)> {
    let s = w.overworld.sites.iter().find(|s| s.id == site)?;
    let rs = w.realm.sites.get(s.realm_site? as usize)?;
    let p = w.realm.peoples.get(rs.people as usize)?;
    Some((p.species, rs.people as usize))
}

/// The culture held at a settlement beyond the valley (P7), if it is one of the world's.
pub fn culture_of_site(w: &crate::world_state::World, site: u32) -> Option<u32> {
    let s = w.overworld.sites.iter().find(|s| s.id == site)?;
    let rs = w.realm.sites.get(s.realm_site? as usize)?;
    ((rs.culture as usize) < w.realm.cultures.len()).then_some(rs.culture)
}

/// Brings the person `agent` up in a culture (P7): every act and value the culture holds
/// firmly (0.2 or more either way) becomes their belief, sourced to the culture, and their
/// kind's leanings are held faintly where the culture says nothing. Their rite for the dead
/// is the culture's.
pub fn raise_in(w: &mut crate::world_state::World, agent: crate::ids::AgentId, culture: u32) {
    use crate::mind::beliefs::Source;
    let Some(c) = w.realm.cultures.get(culture as usize).cloned() else {
        return;
    };
    // The stories their people tell most lately (P8): the three newest of their culture's.
    let mut known: Vec<u32> = w
        .realm
        .stories
        .iter()
        .rev()
        .filter(|s| s.culture == culture)
        .take(3)
        .map(|s| s.id)
        .collect();
    known.reverse();
    let tick = w.meta.tick;
    let Some(a) = w.agents.iter_mut().find(|a| a.id == agent) else {
        return;
    };
    for (t, v) in &c.stances {
        if v.abs() >= 0.2 {
            a.beliefs.hold(*t, *v, 0.5, Source::Culture(culture), tick);
        }
    }
    let kind = a.species_def();
    // The Hollin keep the unbinding of curses among their herb-wise (P11), learned at home.
    if kind.id == "hollin" && a.skills.level(crate::mind::Skill::Foraging) >= 4.0 {
        a.techniques.learn_from(
            crate::mind::Technique::Unbinding,
            crate::mind::techniques::Learned::Past,
            0.5,
            tick,
            "their people's healers",
        );
    }
    for (k, v) in &kind.temperament.convictions {
        if let Some(t) = crate::past::Topic::from_name(k) {
            if a.beliefs.get(t).is_none() {
                a.beliefs.hold(t, *v, 0.15, Source::Species(a.people), tick);
            }
        }
    }
    a.rite = c.rite;
    a.culture = Some(culture);
    a.stories = known;
}

/// The wizard's choices about the world's peoples (P5, docs/design-peoples.md section 4):
/// which species and body plans are left out, how many peoples the world has, and how rare
/// each species is. Recorded in the setup, so the same seed and choices make the same world.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeopleSetup {
    /// Species left out entirely, by file id ("grisk").
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Body plans left out ("winged"): every species on them goes.
    #[serde(default)]
    pub exclude_plans: Vec<String>,
    /// The world has between these many peoples.
    #[serde(default = "count_default")]
    pub count: [u8; 2],
    /// Rarity overrides: (species id, common | uncommon | rare | legendary).
    #[serde(default)]
    pub rarity: Vec<(String, String)>,
    /// Generated species (P6): off, some, many.
    #[serde(default = "generated_default")]
    pub generated: String,
}

fn count_default() -> [u8; 2] {
    [4, 7]
}

fn generated_default() -> String {
    "some".into()
}

impl Default for PeopleSetup {
    fn default() -> Self {
        PeopleSetup {
            exclude: Vec::new(),
            exclude_plans: Vec::new(),
            count: count_default(),
            rarity: Vec::new(),
            generated: generated_default(),
        }
    }
}

impl PeopleSetup {
    /// Whether a species can be in this world at all.
    pub fn present(&self, s: &SpeciesDef) -> bool {
        s.sentient && !self.exclude.contains(&s.id) && !self.exclude_plans.contains(&s.plan)
    }

    /// How rare a species is in this world: the override, else its own.
    pub fn rarity_of<'a>(&'a self, s: &'a SpeciesDef) -> &'a str {
        self.rarity
            .iter()
            .find(|(k, _)| *k == s.id)
            .map(|(_, r)| r.as_str())
            .unwrap_or(s.rarity.as_str())
    }

    /// For the record: "the Grisk left out; the Skarri rare", or "" for the defaults.
    pub fn summary(&self) -> String {
        let name = |k: &str| {
            library()
                .iter()
                .find(|s| s.id == k)
                .map(|s| s.plural.clone())
                .unwrap_or_else(|| k.to_string())
        };
        let mut parts: Vec<String> = self
            .exclude
            .iter()
            .map(|k| format!("the {} left out", name(k)))
            .collect();
        parts.extend(self.exclude_plans.iter().map(|p| format!("no {p} peoples")));
        parts.extend(
            self.rarity
                .iter()
                .map(|(k, r)| format!("the {} {r}", name(k))),
        );
        if self.count != count_default() {
            parts.push(format!("{} to {} peoples", self.count[0], self.count[1]));
        }
        if self.generated != generated_default() {
            parts.push(format!("{} kinds of its own", self.generated));
        }
        parts.join("; ")
    }

    /// Kinds named by id or display name, in any case, become their ids; rarities lowercase
    /// (#147). What no kind answers to is left as typed, for `check` to name.
    pub fn normalise(&mut self) {
        for k in self.exclude.iter_mut() {
            if let Some(id) = resolve_key(k) {
                *k = id;
            }
        }
        for (k, r) in self.rarity.iter_mut() {
            if let Some(id) = resolve_key(k) {
                *k = id;
            }
            *r = r.trim().to_ascii_lowercase();
        }
        self.rarity.sort();
        self.generated = self.generated.trim().to_ascii_lowercase();
    }

    /// Checks what the wizard sent.
    pub fn check(&self) -> Result<(), String> {
        if self.count[0] == 0 || self.count[0] > self.count[1] || self.count[1] > 16 {
            return Err("peoples: between 1 and 16, the least first".into());
        }
        for (k, r) in &self.rarity {
            if by_key(k).is_none() {
                return Err(format!("peoples: no kind called {k:?} to make {r}"));
            }
            if !RARITIES.contains(&r.as_str()) {
                return Err(format!(
                    "peoples: {k} cannot be {r:?}: common, uncommon, rare or legendary"
                ));
            }
        }
        if !matches!(self.generated.as_str(), "off" | "some" | "many") {
            return Err(format!("peoples: generated kinds {}", self.generated));
        }
        if let Some(k) = self.exclude.iter().find(|k| by_key(k).is_none()) {
            return Err(format!("peoples: no kind called {k:?} to leave out"));
        }
        if !library()
            .iter()
            .any(|s| self.present(s) && s.habitat.layers.iter().any(|l| l == "surface"))
        {
            return Err(
                "peoples: nobody left to live in the world: leave in at least one kind that lives above ground"
                    .into(),
            );
        }
        Ok(())
    }
}

/// Ids of species generated for a world (P6) start here; below are the built-in library's.
pub const GENERATED: u16 = 1000;

/// How many species a world generates for each setting of the wizard (P6).
pub fn generated_count(setting: &str) -> usize {
    match setting {
        "off" => 0,
        "many" => 30,
        _ => 12,
    }
}

/// The plans generated species are drawn on, with weights, and the body sheets each can wear
/// (heads and markings drawn by tools/art/people.py). Huge, taur and serpentine bodies come
/// with the deep (P10).
const GEN_PLANS: [(&str, u32, &[&str]); 5] = [
    ("biped", 30, &["human", "hollin"]),
    ("small_biped", 18, &["grisk"]),
    ("digitigrade_tail", 22, &["screeling", "digitigrade_tail"]),
    ("winged", 14, &["skarri"]),
    ("four_armed", 16, &["tharn"]),
];

/// On how many of the behaviour-changing axes a species differs from humans (docs/
/// design-peoples.md section 11): diet, climate, senses, lifespan, size, social structure, way
/// of living, daily cycle.
pub fn axes_from_human(s: &SpeciesDef) -> usize {
    let h = get(SpeciesId::HUMAN);
    let mean = |r: [f32; 2]| (r[0] + r[1]) / 2.0;
    let mut n = 0;
    n += usize::from(s.diet.kind != h.diet.kind);
    n += usize::from(
        s.climate.coat != h.climate.coat
            || (s.climate.comfort_c[0] - h.climate.comfort_c[0]).abs() >= 6.0
            || (s.climate.comfort_c[1] - h.climate.comfort_c[1]).abs() >= 6.0,
    );
    n += usize::from(s.senses != h.senses);
    n += usize::from((mean(s.life.lifespan) / mean(h.life.lifespan) - 1.0).abs() >= 0.4);
    n += usize::from((mean(s.size.mass_kg) / mean(h.size.mass_kg) - 1.0).abs() >= 0.3);
    n += usize::from(
        s.social.structure != h.social.structure || s.social.bonding != h.social.bonding,
    );
    n += usize::from(s.living.ways != h.living.ways);
    n += usize::from(s.cycle != h.cycle);
    n
}

/// The species generated for a world (P6, docs/design-peoples.md section 1.4): each a pure
/// function of the seed and its index, drawn on a body plan, with values on every simulated axis,
/// redrawn until it differs from humans on at least four, given a body sheet and skin tones no
/// other species of the world wears together, and named in a language of its own.
pub fn generate(seed: u64, count: usize) -> Vec<SpeciesDef> {
    use rand::Rng;
    let mut out: Vec<SpeciesDef> = Vec::new();
    let mut looks: Vec<(String, String)> = library()
        .iter()
        .map(|s| (s.art.body.clone(), s.art.skin[0].clone()))
        .collect();
    for k in 0..count {
        let mut attempt = 0u64;
        let def = loop {
            let mut r = crate::rng::stream(
                seed,
                &[
                    crate::rng::purpose::REALM_PEOPLES,
                    0x5BEC,
                    k as u64,
                    attempt,
                ],
            );
            attempt += 1;
            let total: u32 = GEN_PLANS.iter().map(|p| p.1).sum();
            let mut pick = r.gen_range(0..total);
            let (plan, _, bodies) = *GEN_PLANS
                .iter()
                .find(|p| {
                    if pick < p.1 {
                        true
                    } else {
                        pick -= p.1;
                        false
                    }
                })
                .unwrap();
            let body = bodies[r.gen_range(0..bodies.len())].to_string();
            // Size by plan: small bodies small, four arms heavy, wings light.
            let (h0, m0) = match plan {
                "small_biped" => (r.gen_range(85.0..115.0f32), r.gen_range(22.0..40.0f32)),
                "winged" => (r.gen_range(130.0..170.0), r.gen_range(26.0..45.0)),
                "four_armed" => (r.gen_range(165.0..205.0), r.gen_range(70.0..110.0)),
                "digitigrade_tail" => (r.gen_range(90.0..180.0), r.gen_range(28.0..85.0)),
                _ => (r.gen_range(140.0..200.0), r.gen_range(40.0..95.0)),
            };
            let height = [h0.round(), (h0 * 1.15).round()];
            let mass = [m0.round(), (m0 * 1.45).round()];
            let life_lo = [25.0f32, 35.0, 50.0, 60.0, 90.0, 150.0][r.gen_range(0..6)];
            let lifespan = [life_lo, (life_lo * r.gen_range(1.3..1.7f32)).round()];
            let mature = ((life_lo * r.gen_range(0.2..0.3f32)) as u32).max(6);
            let diet = ["omnivore", "carnivore", "herbivore"][r.gen_range(0..3)].to_string();
            let coat = if plan == "winged" {
                "feathers".to_string()
            } else {
                ["bare", "fur", "scales"][r.gen_range(0..3)].to_string()
            };
            let lo = [-12.0f32, -6.0, 0.0, 2.0, 8.0, 14.0][r.gen_range(0..6)];
            let comfort = [lo, lo + r.gen_range(16.0..26.0f32).round()];
            let senses = Senses {
                sight: [1, 2, 2, 3][r.gen_range(0..4)],
                dark_sight: [0, 0, 1, 2, 3][r.gen_range(0..5)],
                hearing: [0, 0, 1, 2, 3][r.gen_range(0..5)],
                smell: [0, 0, 1, 2, 3][r.gen_range(0..5)],
            };
            let structure = ["household", "clan", "band"][r.gen_range(0..3)].to_string();
            let bonding = if structure == "household" {
                "pair"
            } else {
                ["pair", "group"][r.gen_range(0..2)]
            }
            .to_string();
            let all_ways = ["farming", "herding", "hunting", "fishing"];
            let mut ways: Vec<String> = all_ways
                .iter()
                .filter(|_| r.gen_bool(0.45))
                .map(|w| w.to_string())
                .collect();
            if ways.is_empty() {
                ways.push(all_ways[r.gen_range(0..4)].to_string());
            }
            let cycle = if senses.dark_sight >= 2 && r.gen_bool(0.5) {
                "nocturnal"
            } else {
                "diurnal"
            }
            .to_string();
            // Skin tones: three of the palette's, the first not worn first by another species
            // with this body sheet.
            let mut skin: Vec<String> = Vec::new();
            for _ in 0..12 {
                let t = SKIN_NAMES[r.gen_range(0..SKIN_NAMES.len())].to_string();
                if !skin.contains(&t)
                    && (!skin.is_empty() || !looks.contains(&(body.clone(), t.clone())))
                {
                    skin.push(t);
                }
                if skin.len() == 3 {
                    break;
                }
            }
            if skin.is_empty() {
                continue;
            }
            let mut convictions = BTreeMap::new();
            if senses.dark_sight >= 2 {
                convictions.insert("strangers".to_string(), 0.3);
            }
            if diet == "carnivore" || ways.iter().any(|w| w == "hunting") {
                convictions.insert("violence".to_string(), 0.2);
            }
            if structure == "clan" {
                convictions.insert("trust".to_string(), 0.2);
            }
            let lang = crate::realm::language::Language::of(seed, 5000 + k as u64);
            let mut name = lang.word(seed, "people");
            if let Some(c) = name.get_mut(0..1) {
                c.make_ascii_uppercase();
            }
            let size_word = if mass[1] < 45.0 {
                "small"
            } else if mass[0] > 75.0 {
                "big"
            } else {
                "lean"
            };
            let coat_word = match coat.as_str() {
                "fur" => "furred",
                "scales" => "scaled",
                "feathers" => "feathered",
                _ => "bare-skinned",
            };
            let where_word = if comfort[0] >= 8.0 {
                "warm lands"
            } else if comfort[1] <= 12.0 {
                "cold heights"
            } else {
                "open country"
            };
            let blurb = format!(
                "{size_word}, {coat_word} folk of the {where_word}, {}",
                match (diet.as_str(), cycle.as_str()) {
                    ("carnivore", "nocturnal") => "who hunt by night",
                    ("carnivore", _) => "who live on meat",
                    ("herbivore", _) => "who eat no flesh",
                    (_, "nocturnal") => "who wake at dusk",
                    _ => "who eat what they find",
                }
            );
            let def = SpeciesDef {
                id: format!("gen_{k}"),
                name: name.clone(),
                plural: name,
                sentient: true,
                plan: plan.to_string(),
                rarity: ["common", "uncommon", "uncommon", "rare"][r.gen_range(0..4)].to_string(),
                cycle,
                blurb,
                size: Size {
                    height_cm: height,
                    mass_kg: mass,
                },
                life: Life {
                    mature_years: mature,
                    lifespan,
                    litter: [1, r.gen_range(1..4)],
                },
                diet: Diet { kind: diet },
                senses,
                climate: Climate {
                    comfort_c: comfort,
                    coat,
                },
                habitat: Habitat {
                    layers: vec!["surface".into()],
                    builds: ["houses", "hollows", "tents"][r.gen_range(0..3)].to_string(),
                },
                social: Social { structure, bonding },
                temperament: Temperament { convictions },
                art: Art {
                    body,
                    skin,
                    hair: r.gen_bool(0.5),
                },
                living: Living { ways },
                fertile_with: Vec::new(),
            };
            if axes_from_human(&def) >= 4 || attempt > 60 {
                break def;
            }
        };
        looks.push((def.art.body.clone(), def.art.skin[0].clone()));
        out.push(def);
    }
    out
}
