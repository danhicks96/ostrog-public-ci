//! Original name generation for people, places and the world. Deterministic per seed.

use rand::seq::SliceRandom;
use rand::Rng;

use crate::rng::{purpose, stream};

const ONSETS: &[&str] = &[
    "Ael", "Bran", "Cor", "Dael", "Eir", "Fen", "Gal", "Hal", "Ir", "Kel", "Lir", "Mor", "Nal",
    "Or", "Per", "Ros", "Sae", "Tor", "Ul", "Vel", "Wyn", "Yl",
];
const MIDS: &[&str] = &["a", "e", "i", "o", "u", "ae", "ei", "ou"];
const CODAS: &[&str] = &[
    "dric", "wen", "lin", "mar", "ric", "sa", "thi", "vane", "wick", "ya", "nor", "del", "sin",
    "gar", "la", "n",
];
const WORLD_A: &[&str] = &[
    "Hearth", "Ash", "Fern", "Stone", "Mere", "Holt", "Wyn", "Thorn", "Elder", "Reed",
];
const WORLD_B: &[&str] = &[
    "fall", "dale", "reach", "mere", "hollow", "moor", "bank", "fold", "wold", "cross",
];

pub fn person_name(seed: u64, agent_id: u32) -> String {
    let mut r = stream(seed, &[purpose::NAMES, agent_id as u64]);
    let a = ONSETS.choose(&mut r).unwrap();
    let b = if r.gen_bool(0.5) {
        MIDS.choose(&mut r).unwrap()
    } else {
        ""
    };
    let c = CODAS.choose(&mut r).unwrap();
    format!("{a}{b}{c}")
}

/// A different draw of the same slot (the same trick `opening::slot_key` uses for a reroll),
/// so retrying a collision keeps being deterministic from the seed and the id.
fn reroll_key(agent_id: u32, reroll: u32) -> u32 {
    let mut x = ((agent_id as u64) << 32 | reroll as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    (x ^ (x >> 31)) as u32
}

/// Tried at most this many times before giving up and keeping the last roll: the name space
/// (a few thousand combinations) makes running out this way effectively impossible for any
/// valley's population.
const MAX_NAME_REROLLS: u32 = 64;

/// `person_name`, rerolled until `taken` says no one already answers to it (#862: two living
/// people, or one newly arrived and one who died, sharing a name with nothing in the feed,
/// history or an account to tell them apart).
pub fn unique_person_name(seed: u64, agent_id: u32, taken: impl Fn(&str) -> bool) -> String {
    let mut name = person_name(seed, agent_id);
    for reroll in 1..MAX_NAME_REROLLS {
        if !taken(&name) {
            break;
        }
        name = person_name(seed, reroll_key(agent_id, reroll));
    }
    name
}

const COMM_A: &[&str] = &[
    "Oak", "Ash", "Mere", "Hearth", "Stone", "Fern", "Reed", "Thorn", "Elder", "Holt", "Birch",
    "Ridge",
];
const COMM_B: &[&str] = &[
    "folk", "stead", "kin", "ford", "hollow", "croft", "bank", "wick", "side", "gate",
];

pub fn community_name(seed: u64, id: u32) -> String {
    let mut r = stream(seed, &[purpose::NAMES, 0xC0_0000 + id as u64]);
    format!(
        "{}{}",
        COMM_A.choose(&mut r).unwrap(),
        COMM_B.choose(&mut r).unwrap()
    )
}

pub fn world_name(seed: u64) -> String {
    let mut r = stream(seed, &[purpose::NAMES, u64::MAX]);
    format!(
        "{}{}",
        WORLD_A.choose(&mut r).unwrap(),
        WORLD_B.choose(&mut r).unwrap()
    )
}
