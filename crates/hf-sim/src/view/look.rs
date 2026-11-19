//! What a person looks like right now: the fixed appearance (body, skin, hair) plus what
//! changes with the season, the job, the journey, age and injury. Read-only; the client
//! composes sprite layers from it. Index tables match the sheets under
//! godot/assets/people (tools/art/people.py).

use crate::agents::{Agent, LifeStage};
use crate::time::{Calendar, Season};
use crate::work::DesignationKind;
use crate::world_state::World;

pub const OUTFITS: [&str; 8] = [
    "tunic",
    "dress",
    "apron",
    "cloak",
    "coat",
    "workshirt",
    "robe",
    "vest",
];
pub const EQUIPMENT: [&str; 8] = [
    "pickaxe", "axe", "basket", "hoe", "bundle", "bucket", "sack", "spear",
];
pub const HAIR_STYLES: usize = 10;
pub const HAIR_COLOURS: usize = 8;
pub const CLOTH_COLOURS: usize = 12;
pub const SKIN_TONES: usize = 6;
pub const BUILDS: usize = 3;

// Looked up by name so an index cannot point at the wrong sheet: VEST was a literal 6, which
// is the robe, so every farmer wore a robe (issue 37).
const TUNIC: u8 = outfit("tunic");
const APRON: u8 = outfit("apron");
const CLOAK: u8 = outfit("cloak");
const COAT: u8 = outfit("coat");
const WORKSHIRT: u8 = outfit("workshirt");
const VEST: u8 = outfit("vest");
const HAIR_GREY: u8 = 5;
const HAIR_WHITE: u8 = 6;

/// The index of a named outfit in OUTFITS; a name that is not there fails the build.
const fn outfit(name: &str) -> u8 {
    let mut i = 0;
    while i < OUTFITS.len() {
        if same(OUTFITS[i].as_bytes(), name.as_bytes()) {
            return i as u8;
        }
        i += 1;
    }
    panic!("no such outfit in OUTFITS");
}

const fn same(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Look {
    pub body: u8,
    pub skin: u8,
    pub hair: u8,
    pub hair_colour: u8,
    pub clothing: u8,
    pub clothing_colour: u8,
    pub child: bool,
    pub elder: bool,
    pub injured: bool,
    /// Index into EQUIPMENT, or -1.
    pub equipment: i8,
}

/// Working a Tend order (#485): both hands are on the patient, so no spear is drawn.
fn tending(w: &World, a: &Agent) -> bool {
    match &a.activity {
        crate::mind::Activity::Working { plan } => w
            .designations
            .iter()
            .any(|d| d.id == plan.designation && matches!(d.kind, DesignationKind::Tend(_))),
        _ => false,
    }
}

pub fn look(w: &World, a: &Agent) -> Look {
    let tick = w.tick();
    let ap = a.appearance;
    let stage = a.stage(tick);
    let child = stage == LifeStage::Child;
    let elder = stage == LifeStage::Elder;
    let season = Calendar::from_tick(tick).season;
    let cold = season == Season::Winter || w.weather.temperature_c < 2.0;
    let cool = season == Season::Autumn;
    // Clothing: the season first, then the journey, then the job, then what they own.
    let clothing = if child {
        if cold {
            COAT
        } else {
            ap.clothing % 2 // tunic or dress
        }
    } else if cold {
        COAT
    } else if a.caravan.is_some() {
        CLOAK
    } else {
        match a.role {
            Some(crate::society::Role::Cook) | Some(crate::society::Role::Healer) => APRON,
            Some(crate::society::Role::Builder) | Some(crate::society::Role::Miner) => WORKSHIRT,
            Some(crate::society::Role::Farmer) => VEST,
            Some(crate::society::Role::Forager) | Some(crate::society::Role::Hauler) => {
                if cool {
                    CLOAK
                } else {
                    TUNIC
                }
            }
            None => ap.clothing % OUTFITS.len() as u8,
        }
    };
    let hair_colour = if elder {
        if a.age_years(tick) >= 60 {
            HAIR_WHITE
        } else {
            HAIR_GREY
        }
    } else {
        ap.hair_colour % HAIR_COLOURS as u8
    };
    // Equipment follows the work in hand.
    let equipment: i8 = match &a.activity {
        crate::mind::Activity::Foraging { .. } => 2,
        crate::mind::Activity::Working { plan } => {
            let kind = w
                .designations
                .iter()
                .find(|d| d.id == plan.designation)
                .map(|d| d.kind);
            // A pick or an axe only if there is a tool in hand: without one they work with
            // what they have, and it shows (legend.rs).
            let has_tool = a.tool.is_some() || a.tool_quality > 0.0;
            match kind {
                Some(DesignationKind::Mine) | Some(DesignationKind::DigStairs) if !has_tool => -1,
                Some(DesignationKind::Chop) if !has_tool => -1,
                Some(DesignationKind::Mine) | Some(DesignationKind::DigStairs) => 0,
                Some(DesignationKind::DigEarth) | Some(DesignationKind::DigChannel) => 3,
                Some(DesignationKind::Hunt(_)) if a.weapon.is_some() => 7,
                Some(DesignationKind::Chop) | Some(DesignationKind::Hunt(_)) => 1,
                Some(DesignationKind::Sow) | Some(DesignationKind::Harvest) => 3,
                Some(DesignationKind::Cook) => 5,
                Some(DesignationKind::Haul) => -1, // the carried stack is drawn instead
                // Hands on the patient (#485): nothing is carried that the tending does not use;
                // before this a healer was drawn with the builder's bundle of materials.
                Some(DesignationKind::Tend(_)) => -1,
                Some(_) => 4, // building: a bundle of materials
                None => -1,
            }
        }
        _ => -1,
    };
    // Someone who has taken up a spear (systems/arms.rs) carries it when their hands are
    // otherwise free.
    let equipment = if equipment < 0
        && a.weapon.is_some()
        && !a.activity.is_asleep()
        && !matches!(&a.activity, crate::mind::Activity::Eating { .. })
        && !matches!(&a.activity, crate::mind::Activity::Drinking { .. })
        && !tending(w, a)
    {
        7
    } else {
        equipment
    };
    Look {
        body: ap.body % BUILDS as u8,
        // Any skin target, the species' own included (species.rs SKIN_NAMES; the review of
        // C38-C44 found them all folded into the six human tones).
        skin: ap.skin % crate::species::SKIN_NAMES.len() as u8,
        hair: ap.hair % HAIR_STYLES as u8,
        hair_colour,
        clothing,
        clothing_colour: ap.clothing_colour % CLOTH_COLOURS as u8,
        child,
        elder,
        injured: a.injured(tick),
        equipment,
    }
}
