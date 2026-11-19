//! Inheritance. A genome is a short vector of loci: physical looks and temperament. It is
//! copied from the parents with bounded variation and decides *starting points* only:
//! appearance, trait tendencies, skill aptitudes. Skills, habits, memories and customs are
//! learned and never inherited.
use crate::fmath::Det;

use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::agents::Appearance;
use crate::mind::skills::{Skills, ALL as SKILLS};
use crate::mind::{Trait, Traits};

pub const LOCI: usize = 14;

/// Locus meanings (0..255 each).
pub mod locus {
    pub const SKIN: usize = 0;
    pub const HAIR_COLOUR: usize = 1;
    pub const BUILD: usize = 2;
    pub const HAIR_STYLE: usize = 3;
    pub const SOCIABILITY: usize = 4;
    pub const DRIVE: usize = 5;
    pub const CURIOSITY: usize = 6;
    pub const CALM: usize = 7;
    pub const HARDINESS: usize = 8;
    pub const APT_FIELD: usize = 9; // foraging, farming, animals
    pub const APT_CRAFT: usize = 10; // construction, crafting, mining
    pub const APT_HEARTH: usize = 11; // cooking, hauling
    pub const APT_SOCIAL: usize = 12; // social
    pub const LONGEVITY: usize = 13;
}

/// The body block (P1 of milestone MP, docs/design-peoples.md section 2.3): read within a
/// species' ranges and variant lists. Humans use SIZE for height and, with BUILD, for mass.
pub const BODY: usize = 8;

pub mod body {
    pub const SIZE: usize = 0;
    pub const HEAD: usize = 1;
    pub const PART_A: usize = 2;
    pub const PART_B: usize = 3;
    pub const COLOUR_1: usize = 4;
    pub const COLOUR_2: usize = 5;
    pub const PATTERN: usize = 6;
    pub const MARK: usize = 7;
}

/// The mass all size effects are measured against: the human mean.
pub const REFERENCE_KG: f32 = 65.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Genome {
    pub loci: [u8; LOCI],
    /// Genomes saved before P1 have none; they read as the middle of every range.
    #[serde(default = "mid_body")]
    pub body: [u8; BODY],
}

fn mid_body() -> [u8; BODY] {
    [128; BODY]
}

impl Default for Genome {
    fn default() -> Self {
        Genome {
            loci: [128; LOCI],
            body: mid_body(),
        }
    }
}

/// How fast a body of this mass gets hungry against the human mean: (mass / 65)^0.75.
pub fn hunger_scale(mass_kg: f32) -> f32 {
    (mass_kg / REFERENCE_KG).d_powf(0.75)
}

/// Loads a body of this mass carries against the human mean, with `hands` from its plan.
pub fn carry(mass_kg: f32, hands: f32) -> f32 {
    ((mass_kg / REFERENCE_KG).d_powf(0.67) * hands / 2.0).clamp(0.4, 4.0)
}

/// How fast a body of this mass loses heat against the human mean: small bodies faster.
pub fn heat_loss(mass_kg: f32) -> f32 {
    (mass_kg / REFERENCE_KG).d_powf(-0.33)
}

/// A fixed mix of a genome's loci with a key: the body block is drawn from it rather than from
/// the caller's stream, so adding the block changed no other draw in any world.
fn mix(loci: &[u8; LOCI], k: usize, salt: u64) -> u64 {
    let mut h: u64 = 0x9E37_79B9_7F4A_7C15 ^ salt.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    for (i, b) in loci.iter().enumerate() {
        h ^= (*b as u64) << ((i % 8) * 8);
        h = h.wrapping_mul(0x94D0_49BB_1331_11EB).rotate_left(31);
    }
    h ^= k as u64;
    h = (h ^ (h >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^ (h >> 31)
}

impl Genome {
    pub fn random(r: &mut ChaCha8Rng) -> Genome {
        let mut g = Genome::default();
        for l in g.loci.iter_mut() {
            *l = r.gen();
        }
        for k in 0..BODY {
            g.body[k] = (mix(&g.loci, k, 0) & 0xFF) as u8;
        }
        g
    }

    /// Each locus comes from one parent; a few mutate by a bounded step; rarely a large jump.
    pub fn inherit(mother: &Genome, father: &Genome, r: &mut ChaCha8Rng) -> Genome {
        let mut g = Genome::default();
        for i in 0..LOCI {
            let base = if r.gen_bool(0.5) {
                mother.loci[i]
            } else {
                father.loci[i]
            };
            let roll: f32 = r.gen();
            let v = if roll < 0.08 {
                (base as i32 + r.gen_range(-24..=24)).clamp(0, 255)
            } else if roll < 0.09 {
                r.gen_range(0..=255)
            } else {
                base as i32
            };
            g.loci[i] = v as u8;
        }
        // The body block the same way, drawn from the child's own loci: each locus from one
        // parent, a few drift, rarely one jumps.
        for k in 0..BODY {
            let h = mix(&g.loci, k, 1);
            let base = if h & 1 == 0 {
                mother.body[k]
            } else {
                father.body[k]
            } as i32;
            let roll = (h >> 8) % 100;
            let v = if roll < 8 {
                base + ((h >> 16) % 49) as i32 - 24
            } else if roll < 9 {
                ((h >> 16) & 0xFF) as i32
            } else {
                base
            };
            g.body[k] = v.clamp(0, 255) as u8;
        }
        g
    }

    fn fb(&self, k: usize) -> f32 {
        self.body[k] as f32 / 255.0
    }

    /// Height in cm within the species' range (humans 150 to 190).
    pub fn height_in(&self, sp: &crate::species::SpeciesDef) -> f32 {
        self.height_within(sp.size.height_cm)
    }

    /// Mass in kg within the species' range: height and build together (humans 51 to 79,
    /// mean 65).
    pub fn mass_in(&self, sp: &crate::species::SpeciesDef) -> f32 {
        self.mass_within(sp.size.mass_kg)
    }

    /// A fixed draw from this genome for whether it is a runt or a giant of its kind
    /// (size.rs `Build`, #422). Build is left out, so a body's build within the range and its
    /// being a giant are apart.
    pub fn size_roll(&self) -> u64 {
        let mut l = self.loci;
        l[locus::BUILD] = 0;
        mix(&l, 0x5153, 3)
    }

    /// Height in cm within a range.
    pub fn height_within(&self, [lo, hi]: [f32; 2]) -> f32 {
        lo + self.fb(body::SIZE) * (hi - lo)
    }

    /// Mass in kg within a range: height and build together.
    pub fn mass_within(&self, [lo, hi]: [f32; 2]) -> f32 {
        lo + (hi - lo) * (0.5 * self.fb(body::SIZE) + 0.5 * self.f(locus::BUILD))
    }

    /// Years this body would live within the species' range, before hardship.
    pub fn lifespan_in(&self, sp: &crate::species::SpeciesDef) -> f32 {
        let [lo, hi] = sp.life.lifespan;
        lo + self.f(locus::LONGEVITY) * (hi - lo)
    }

    /// Height in cm, as a human.
    pub fn height_cm(&self) -> f32 {
        self.height_in(crate::species::get(crate::species::SpeciesId::HUMAN))
    }

    /// Mass in kg, as a human.
    pub fn mass_kg(&self) -> f32 {
        self.mass_in(crate::species::get(crate::species::SpeciesId::HUMAN))
    }

    /// How fast this body gets hungry against the human mean: (mass / 65)^0.75.
    pub fn hunger_scale(&self) -> f32 {
        hunger_scale(self.mass_kg())
    }

    /// Loads this body carries against the human mean, with `hands` from its plan.
    pub fn carry(&self, hands: f32) -> f32 {
        carry(self.mass_kg(), hands)
    }

    /// How fast this body loses heat against the human mean: small bodies faster.
    pub fn heat_loss(&self) -> f32 {
        heat_loss(self.mass_kg())
    }

    fn f(&self, i: usize) -> f32 {
        self.loci[i] as f32 / 255.0
    }

    pub fn appearance(&self, r: &mut ChaCha8Rng) -> Appearance {
        Appearance {
            body: (self.f(locus::BUILD) * 2.999) as u8,
            skin: (self.f(locus::SKIN) * 5.999) as u8,
            hair: (self.f(locus::HAIR_STYLE) * 9.999) as u8,
            hair_colour: (self.f(locus::HAIR_COLOUR) * 7.999) as u8,
            clothing: r.gen_range(0..8),
            clothing_colour: r.gen_range(0..12),
        }
    }

    /// Continuous core dispositions (#488), derived without consuming a random stream.
    pub fn dispositions(&self) -> crate::mind::Dispositions {
        let extra = |key: usize| (mix(&self.loci, key, 488) & 0xFFFF) as f32 / 65_535.0;
        crate::mind::Dispositions {
            sociability: self.f(locus::SOCIABILITY),
            industry: self.f(locus::DRIVE),
            curiosity: self.f(locus::CURIOSITY),
            caution: 1.0 - self.f(locus::CALM),
            warmth: extra(0),
            cheerfulness: extra(1),
            order: extra(2),
            independence: extra(3),
        }
    }

    /// Temperament loci set trait tendencies; the extremes become traits.
    pub fn traits(&self) -> Traits {
        let mut list = Vec::new();
        let s = self.f(locus::SOCIABILITY);
        if s > 0.75 {
            list.push(Trait::Sociable);
        } else if s < 0.25 {
            list.push(Trait::Solitary);
        }
        let d = self.f(locus::DRIVE);
        if d > 0.75 {
            list.push(Trait::Industrious);
        } else if d < 0.25 {
            list.push(Trait::Easygoing);
        }
        if self.f(locus::CURIOSITY) > 0.75 {
            list.push(Trait::Curious);
        }
        if self.f(locus::CALM) < 0.25 {
            list.push(Trait::Cautious);
        }
        if self.f(locus::HARDINESS) > 0.75 {
            list.push(Trait::Hearty);
        }
        if self.f(locus::HARDINESS) < 0.2 {
            list.push(Trait::LightSleeper);
        }
        list.sort();
        list.truncate(3);
        Traits { list }
    }

    /// Aptitudes 0.6..1.4 per skill from the four aptitude loci; levels start at zero.
    pub fn aptitudes(&self) -> Skills {
        let mut sk = Skills::default();
        for (i, s) in SKILLS.iter().enumerate() {
            let l = match s {
                crate::mind::Skill::Foraging
                | crate::mind::Skill::Farming
                | crate::mind::Skill::Animals => locus::APT_FIELD,
                crate::mind::Skill::Construction
                | crate::mind::Skill::Crafting
                | crate::mind::Skill::Mining => locus::APT_CRAFT,
                crate::mind::Skill::Cooking | crate::mind::Skill::Hauling => locus::APT_HEARTH,
                crate::mind::Skill::Social => locus::APT_SOCIAL,
            };
            sk.states[i].aptitude = 0.6 + self.f(l) * 0.8;
            sk.states[i].level = 0.0;
            sk.states[i].xp = 0.0;
        }
        sk
    }

    /// Expected lifespan in years (50..70) before hardship.
    pub fn lifespan_years(&self) -> f32 {
        50.0 + self.f(locus::LONGEVITY) * 20.0
    }
    /// 0..1 temperament: how much strain it takes to break.
    pub fn calm(&self) -> f32 {
        self.f(locus::CALM)
    }
    /// 0..1: how long the body holds out without food (same locus as longevity).
    pub fn constitution(&self) -> f32 {
        self.f(locus::LONGEVITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::stream;

    #[test]
    fn children_resemble_parents_but_vary() {
        let mut r = stream(1, &[1]);
        let m = Genome::random(&mut r);
        let f = Genome::random(&mut r);
        let mut same = 0;
        let mut total = 0;
        for i in 0..200u64 {
            let mut rr = stream(2, &[i]);
            let c = Genome::inherit(&m, &f, &mut rr);
            for k in 0..LOCI {
                total += 1;
                if c.loci[k] == m.loci[k] || c.loci[k] == f.loci[k] {
                    same += 1;
                }
            }
        }
        let frac = same as f32 / total as f32;
        assert!(
            frac > 0.85 && frac < 0.99,
            "mostly inherited, some variation: {frac}"
        );
    }
}
