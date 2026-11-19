//! Languages: each people speaks with its own sounds. The word-for-a-concept idea is ported
//! from the earlier prototype (`naming::word_for`: a concept folded with FNV-1a seeds the word, so the
//! same people always say the same thing the same way); unlike there, every language here has
//! its own inventory of onsets, vowels and endings and its own syllable habits, so one people's
//! names sound like theirs and not like their neighbours'.

use rand::seq::SliceRandom;
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::rng::{purpose, stream};

const ONSETS: [&str; 34] = [
    "b", "c", "d", "f", "g", "h", "j", "k", "l", "m", "n", "p", "r", "s", "t", "v", "w", "z", "th",
    "sh", "ch", "br", "dr", "gr", "kr", "tr", "st", "sk", "sl", "fl", "gl", "vr", "kh", "y",
];
const NUCLEI: [&str; 16] = [
    "a", "e", "i", "o", "u", "y", "aa", "ae", "ai", "au", "ea", "ei", "ia", "io", "oo", "ou",
];
const CODAS: [&str; 18] = [
    "n", "r", "s", "l", "m", "th", "sk", "x", "nd", "rn", "st", "k", "t", "d", "g", "ng", "sh",
    "rd",
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Language {
    pub id: u64,
    onsets: Vec<u8>,
    nuclei: Vec<u8>,
    codas: Vec<u8>,
    /// Chance a syllable closes with an ending.
    coda_rate: f32,
    /// Fewest and most syllables in a word.
    syllables: (u8, u8),
    /// Chance a word starts with a bare vowel.
    vowel_start: f32,
}

fn pick(r: &mut ChaCha8Rng, of: usize, lo: usize, hi: usize) -> Vec<u8> {
    let mut all: Vec<u8> = (0..of as u8).collect();
    all.shuffle(r);
    all.truncate(r.gen_range(lo..=hi));
    all.sort();
    all
}

impl Language {
    pub fn of(seed: u64, id: u64) -> Language {
        let mut r = stream(seed, &[purpose::REALM_PEOPLES, id, 0x6c61_6e67]); // "lang"
        let min = r.gen_range(1..=2u8);
        Language {
            id,
            onsets: pick(&mut r, ONSETS.len(), 6, 12),
            nuclei: pick(&mut r, NUCLEI.len(), 3, 6),
            codas: pick(&mut r, CODAS.len(), 2, 7),
            coda_rate: r.gen_range(0.1..0.7),
            syllables: (min, r.gen_range(min.max(2)..=3)),
            vowel_start: r.gen_range(0.0..0.3),
        }
    }

    fn word_from(&self, r: &mut ChaCha8Rng) -> String {
        let n = r.gen_range(self.syllables.0..=self.syllables.1);
        let mut s = String::new();
        for k in 0..n {
            if k > 0 || r.gen::<f32>() >= self.vowel_start {
                s.push_str(ONSETS[self.onsets[r.gen_range(0..self.onsets.len())] as usize]);
            }
            s.push_str(NUCLEI[self.nuclei[r.gen_range(0..self.nuclei.len())] as usize]);
            if r.gen::<f32>() < self.coda_rate {
                s.push_str(CODAS[self.codas[r.gen_range(0..self.codas.len())] as usize]);
            }
        }
        // Never a one- or two-letter name: add a syllable.
        while s.chars().count() < 3 {
            s.push_str(ONSETS[self.onsets[r.gen_range(0..self.onsets.len())] as usize]);
            s.push_str(NUCLEI[self.nuclei[r.gen_range(0..self.nuclei.len())] as usize]);
        }
        let mut c = s.chars();
        match c.next() {
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            None => s,
        }
    }

    /// This people's word for a concept: always the same word for the same concept.
    pub fn word(&self, seed: u64, concept: &str) -> String {
        let mut k: u64 = 0xcbf2_9ce4_8422_2325;
        for b in concept.bytes() {
            k ^= b as u64;
            k = k.wrapping_mul(0x0100_0000_01b3);
        }
        let mut r = stream(seed, &[purpose::REALM_PEOPLES, self.id, k, 0x776f_7264]); // "word"
        self.word_from(&mut r)
    }

    /// A name for the n-th thing of a kind ("site", "person", "polity").
    pub fn name(&self, seed: u64, kind: &str, n: u64) -> String {
        self.word(seed, &format!("{kind}:{n}"))
    }

    /// The sounds this language uses, for tests and the legends browser.
    pub fn sounds(&self) -> (Vec<&'static str>, Vec<&'static str>, Vec<&'static str>) {
        (
            self.onsets.iter().map(|&i| ONSETS[i as usize]).collect(),
            self.nuclei.iter().map(|&i| NUCLEI[i as usize]).collect(),
            self.codas.iter().map(|&i| CODAS[i as usize]).collect(),
        )
    }
}
