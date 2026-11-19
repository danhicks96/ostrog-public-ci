//! What size changes (#422): how much a body eats, how hard it hits, what it can carry, drag
//! or knock down, and how it grows into its size. People read their size from their species
//! (species.rs `Size`); the valley's animals from the table here, with the same height and
//! mass ranges. Every effect is measured against the human mean (`genetics::REFERENCE_KG`), so
//! a person of middling size is where they were before size mattered.

use crate::agents::Species;
use crate::fmath::Det;
use crate::genetics::REFERENCE_KG;

/// An animal's size range: shoulder height and mass of a grown one, and the years it takes to
/// grow into them. The ranges are a 10 to 20% spread either side of the kind's mean.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimalSize {
    pub height_cm: [f32; 2],
    pub mass_kg: [f32; 2],
    pub grown_years: f32,
}

const fn sz(h: [f32; 2], m: [f32; 2], grown_years: f32) -> AnimalSize {
    AnimalSize {
        height_cm: h,
        mass_kg: m,
        grown_years,
    }
}

/// The size of each kind of animal; None for a person (their species says).
pub fn animal(s: Species) -> Option<AnimalSize> {
    Some(match s {
        Species::Person => return None,
        Species::Deer => sz([95.0, 115.0], [75.0, 100.0], 2.0),
        Species::Boar => sz([70.0, 90.0], [60.0, 85.0], 2.0),
        Species::Wolf => sz([70.0, 85.0], [32.0, 45.0], 2.0),
        Species::Hare => sz([25.0, 32.0], [3.0, 4.2], 1.0),
        Species::Fox => sz([35.0, 43.0], [5.0, 7.0], 1.0),
        Species::Crow => sz([40.0, 50.0], [0.42, 0.58], 1.0),
        Species::Bear => sz([100.0, 125.0], [190.0, 260.0], 4.0),
        Species::Duck => sz([50.0, 62.0], [1.0, 1.4], 1.0),
        Species::Goat => sz([65.0, 80.0], [45.0, 62.0], 2.0),
        Species::Hart => sz([115.0, 135.0], [120.0, 160.0], 3.0),
        Species::Bat => sz([6.0, 8.0], [0.018, 0.025], 1.0),
        Species::Crawler => sz([100.0, 120.0], [150.0, 200.0], 3.0),
        Species::Burrower => sz([110.0, 135.0], [300.0, 400.0], 5.0),
        // #423: the slime's height is its heap; the rats' size is the whole swarm's (forty rats
        // of a third of a kilo); the centipede is as long as a man's arm.
        Species::Slime => sz([30.0, 40.0], [25.0, 35.0], 2.0),
        Species::Rats => sz([12.0, 16.0], [9.0, 13.0], 1.0),
        Species::Centipede => sz([10.0, 13.0], [3.0, 4.5], 2.0),
        // #336: the blind bear of the deep runs leaner than a surface bear on cave fare; the
        // cave crocodile is a heavy old body of the pools.
        Species::BlindCaveBear => sz([95.0, 115.0], [130.0, 180.0], 4.0),
        Species::CaveCroc => sz([55.0, 70.0], [90.0, 140.0], 3.0),
    })
}

/// The mean grown mass of a kind of animal (a person: the human mean).
pub fn mean_kg(s: Species) -> f32 {
    animal(s).map_or(REFERENCE_KG, |a| 0.5 * (a.mass_kg[0] + a.mass_kg[1]))
}

/// The kinds that graze the low green growth (plants.rs `grazed`), and the cave slime and the
/// rats, which feed as grazers do on what they browse (systems/animals.rs `browses`, #423).
pub fn grazes(s: Species) -> bool {
    matches!(
        s,
        Species::Deer
            | Species::Boar
            | Species::Hare
            | Species::Duck
            | Species::Goat
            | Species::Hart
            | Species::Slime
            | Species::Rats
    )
}

/// One in about two hundred of a kind is a runt, and one in about two hundred a giant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Build {
    Runt,
    Plain,
    Giant,
}

impl Build {
    /// A runt grows to six tenths of its kind's size, a giant to half again and a tenth more.
    pub fn factor(self) -> f32 {
        match self {
            Build::Runt => 0.6,
            Build::Plain => 1.0,
            Build::Giant => 1.6,
        }
    }
}

/// Whether an animal of this genome is a runt, a giant or neither: one in two hundred each.
pub fn build_of(g: &crate::genetics::Genome) -> Build {
    match g.size_roll() % 200 {
        0 => Build::Runt,
        1 => Build::Giant,
        _ => Build::Plain,
    }
}

/// How far grown a young animal is at `years`: a newborn is three tenths of its grown size,
/// and it grows evenly to its full size at `grown_years`.
pub fn growth(years: f32, grown_years: f32) -> f32 {
    (0.3 + 0.7 * years / grown_years.max(0.1)).clamp(0.3, 1.0)
}

/// How hard a body of this mass hits and how much it takes to hurt it, against the human mean:
/// (mass / 65)^0.5. A person of 51 to 79 kg is 0.89 to 1.10; a bear about 1.9; a hare 0.24.
pub fn might(mass_kg: f32) -> f32 {
    (mass_kg.max(0.001) / REFERENCE_KG).d_powf(0.5)
}

/// What a body of this mass can push over, against the human mean: (mass / 65)^0.67, as
/// `genetics::carry` scales a load.
pub fn force(mass_kg: f32) -> f32 {
    (mass_kg.max(0.001) / REFERENCE_KG).d_powf(0.67)
}

/// Force against hardness: a body breaks a wooden door, fence or wall when its force is at
/// least this many times the wood's hardness. A grown bear (about 2.3) breaks pine (0.5), birch
/// (0.6) and oak (0.7) but not bonewood (0.9); the pale crawler (about 1.9) pine and birch but
/// not oak; a wolf (0.7), a person (1.0) or anything smaller nothing.
pub const BREAK_PER_HARDNESS: f32 = 3.0;

/// Whether a body of this mass can break through something built of stuff this hard.
pub fn breaks(mass_kg: f32, hardness: f32) -> bool {
    force(mass_kg) >= BREAK_PER_HARDNESS * hardness.max(0.1)
}
