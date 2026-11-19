//! Peoples: who lives in the world, where their homeland is, how they live, what they hold to,
//! and the tongue they name things in.

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::embark::habitable;
use super::language::Language;
use super::map::{RBiome, RealmMap};
use crate::past::Topic;
use crate::rng::{purpose, stream};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Living {
    Farming,
    Herding,
    Hunting,
    Fishing,
}

impl Living {
    pub fn label(self) -> &'static str {
        match self {
            Living::Farming => "farmers",
            Living::Herding => "herders",
            Living::Hunting => "hunters",
            Living::Fishing => "fisherfolk",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct People {
    pub id: u32,
    pub name: String,
    pub language: Language,
    pub homeland: usize,
    pub living: Living,
    /// Shared convictions, the same stances people hold (mind::beliefs): -1..1 per topic.
    pub convictions: Vec<(Topic, f32)>,
    pub colour: [u8; 3],
    /// The sentient species this people is (species.rs, P3); human in worlds made before.
    #[serde(default)]
    pub species: crate::species::SpeciesId,
    /// How rare it is in this world (P5): common and uncommon peoples found homelands; a rare
    /// one has none and lives as households among others; a legendary one's land fails it.
    #[serde(default = "common")]
    pub rarity: String,
    /// A people of the deep (P10): its homeland and settlements lie in the caverns under the
    /// land, a layer of their own.
    #[serde(default)]
    pub deep: bool,
}

fn common() -> String {
    "common".into()
}

impl People {
    pub fn conviction(&self, t: Topic) -> f32 {
        self.convictions
            .iter()
            .find(|c| c.0 == t)
            .map(|c| c.1)
            .unwrap_or(0.0)
    }
}

fn living_of(m: &RealmMap, i: usize) -> Living {
    let coast = {
        let (x, y) = m.xy(i);
        let mut sea = false;
        for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x as i32 + dx, y as i32 + dy);
            if nx >= 0 && ny >= 0 && (nx as usize) < m.size && (ny as usize) < m.size {
                sea |= m.biome[m.idx(nx as usize, ny as usize)].is_water();
            }
        }
        sea
    };
    match m.biome[i] {
        RBiome::Grassland | RBiome::TemperateForest | RBiome::Savanna if !coast => Living::Farming,
        RBiome::Steppe | RBiome::Tundra => Living::Herding,
        RBiome::Taiga | RBiome::Rainforest | RBiome::Swamp => Living::Hunting,
        _ if coast => Living::Fishing,
        _ => Living::Farming,
    }
}

/// Which surface species a people is: of those the setup leaves in, weighted by rarity (common
/// 6, uncommon 3, rare and legendary 1), twice as likely when its way of living suits the land's,
/// and not one already drawn until every one has been (then a second people of a kind is
/// possible, as humans on two continents are two peoples). Drawn from its own stream.
fn species_of(
    seed: u64,
    k: usize,
    living: Living,
    setup: &crate::species::PeopleSetup,
    taken: &[crate::species::SpeciesId],
    kinds: &[(crate::species::SpeciesId, &crate::species::SpeciesDef)],
) -> crate::species::SpeciesId {
    let mut r = stream(seed, &[purpose::REALM_PEOPLES, 2, k as u64]);
    let way = match living {
        Living::Farming => "farming",
        Living::Herding => "herding",
        Living::Hunting => "hunting",
        Living::Fishing => "fishing",
    };
    let open = |s: &crate::species::SpeciesDef| {
        setup.present(s) && s.habitat.layers.iter().any(|l| l == "surface")
    };
    let all_taken = kinds
        .iter()
        .filter(|(_, s)| open(s))
        .all(|(id, _)| taken.contains(id));
    let weights: Vec<u32> = kinds
        .iter()
        .map(|(id, s)| {
            if !open(s) || (!all_taken && taken.contains(id)) {
                return 0;
            }
            let base = match setup.rarity_of(s) {
                "common" => 6,
                "uncommon" => 3,
                _ => 1,
            };
            base * if s.living.ways.iter().any(|w| w == way) {
                2
            } else {
                1
            }
        })
        .collect();
    let total: u32 = weights.iter().sum();
    if total == 0 {
        return crate::species::SpeciesId::HUMAN;
    }
    let mut pick = r.gen_range(0..total);
    for (i, w) in weights.iter().enumerate() {
        if pick < *w {
            return kinds[i].0;
        }
        pick -= w;
    }
    crate::species::SpeciesId::HUMAN
}

const COLOURS: [[u8; 3]; 8] = [
    [217, 99, 57],
    [121, 98, 186],
    [233, 188, 89],
    [65, 128, 159],
    [182, 66, 46],
    [170, 201, 101],
    [230, 222, 208],
    [111, 67, 46],
];

/// Four to seven peoples, homelands spread apart over habitable land (each new homeland the
/// habitable region farthest from those already chosen, among a seeded sample).
pub fn peoples(seed: u64, m: &RealmMap) -> Vec<People> {
    peoples_with(seed, m, &crate::species::PeopleSetup::default(), &[])
}

/// As `peoples`, with the wizard's choices of which species, how many and how rare (P5).
pub fn peoples_with(
    seed: u64,
    m: &RealmMap,
    setup: &crate::species::PeopleSetup,
    generated: &[&crate::species::SpeciesDef],
) -> Vec<People> {
    // Every kind the world can draw from: the library, then those generated for it (P6).
    let mut kinds: Vec<(crate::species::SpeciesId, &crate::species::SpeciesDef)> =
        crate::species::library()
            .iter()
            .enumerate()
            .map(|(i, d)| (crate::species::SpeciesId(i as u16), d))
            .collect();
    kinds.extend(generated.iter().enumerate().map(|(i, d)| {
        (
            crate::species::SpeciesId(crate::species::GENERATED + i as u16),
            *d,
        )
    }));
    let kind_of = |id: crate::species::SpeciesId| {
        kinds
            .iter()
            .find(|(k, _)| *k == id)
            .map(|(_, d)| *d)
            .unwrap_or_else(|| crate::species::get(id))
    };
    let mut r = stream(seed, &[purpose::REALM_PEOPLES, 1]);
    // Homelands on land that can feed a people (a swamp hamlet starved within years).
    let mut land: Vec<usize> = (0..m.size * m.size)
        .filter(|&i| habitable(m, i) && super::sites::capacity(m, i) >= 70.0)
        .collect();
    if land.is_empty() {
        land = (0..m.size * m.size).filter(|&i| habitable(m, i)).collect();
    }
    if land.is_empty() {
        return Vec::new();
    }
    let count = r
        .gen_range(setup.count[0] as usize..=setup.count[1].max(setup.count[0]) as usize)
        .min(land.len());
    let mut homes: Vec<usize> = vec![land[r.gen_range(0..land.len())]];
    while homes.len() < count {
        let best = (0..60)
            .map(|_| land[r.gen_range(0..land.len())])
            .max_by_key(|&c| {
                let (cx, cy) = m.xy(c);
                homes
                    .iter()
                    .map(|&h| {
                        let (hx, hy) = m.xy(h);
                        (cx as i64 - hx as i64).pow(2) + (cy as i64 - hy as i64).pow(2)
                    })
                    .min()
                    .unwrap_or(0)
            })
            .unwrap();
        homes.push(best);
    }
    let mut taken: Vec<crate::species::SpeciesId> = Vec::new();
    let mut out: Vec<People> = homes
        .into_iter()
        .enumerate()
        .map(|(k, home)| {
            let id = k as u32;
            let language = Language::of(seed, id as u64);
            let living = living_of(m, home);
            let species = species_of(seed, k, living, setup, &taken, &kinds);
            taken.push(species);
            let rarity = setup.rarity_of(kind_of(species)).to_string();
            let lean = &kind_of(species).temperament.convictions;
            // The species' leanings shift its people's convictions (P3); humans have none.
            let convictions = Topic::ALL
                .iter()
                .map(|&t| {
                    let c = (r.gen_range(-0.8f32..0.8) * 100.0).round() / 100.0;
                    let l = lean.get(t.name()).copied().unwrap_or(0.0);
                    (t, ((c + l).clamp(-1.0, 1.0) * 100.0).round() / 100.0)
                })
                .collect();
            People {
                id,
                name: language.word(seed, "people"),
                language,
                homeland: home,
                living,
                convictions,
                colour: COLOURS[k % COLOURS.len()],
                species,
                rarity,
                deep: false,
            }
        })
        .collect();
    deep_peoples(seed, m, setup, &kinds, &mut out);
    out
}

/// The peoples of the deep (P10): each kind that cannot live on the surface, present in this
/// world, is a people by chance of its rarity (common always, uncommon 3 in 4, rare 1 in 2,
/// legendary 1 in 4), its homeland in the caverns under the most mountainous ground, apart from
/// the others. Drawn from a stream of their own, after the surface peoples, so those are the
/// same as in a world without them.
fn deep_peoples(
    seed: u64,
    m: &RealmMap,
    setup: &crate::species::PeopleSetup,
    kinds: &[(crate::species::SpeciesId, &crate::species::SpeciesDef)],
    out: &mut Vec<People>,
) {
    let mut r = stream(seed, &[purpose::REALM_PEOPLES, 5]);
    let ground: Vec<usize> = (0..m.size * m.size)
        .filter(|&i| m.land(i) && m.biome[i] != RBiome::Ice)
        .collect();
    if ground.is_empty() {
        return;
    }
    // Ground under mountains has the most caves.
    let hills = |i: usize| {
        let (x, y) = m.xy(i);
        let mut n = 0;
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                if nx >= 0
                    && ny >= 0
                    && (nx as usize) < m.size
                    && (ny as usize) < m.size
                    && m.biome[m.idx(nx as usize, ny as usize)] == RBiome::Mountain
                {
                    n += 1;
                }
            }
        }
        n
    };
    let mut homes: Vec<usize> = Vec::new();
    for (species, def) in kinds
        .iter()
        .filter(|(_, d)| setup.present(d) && !d.habitat.layers.iter().any(|l| l == "surface"))
    {
        let chance = match setup.rarity_of(def) {
            "common" => 1.0,
            "uncommon" => 0.75,
            "rare" => 0.5,
            _ => 0.25,
        };
        if r.gen::<f32>() >= chance {
            continue;
        }
        let home = (0..60)
            .map(|_| ground[r.gen_range(0..ground.len())])
            .max_by_key(|&c| {
                let (cx, cy) = m.xy(c);
                let apart = homes
                    .iter()
                    .map(|&h| {
                        let (hx, hy) = m.xy(h);
                        (cx as i64 - hx as i64).pow(2) + (cy as i64 - hy as i64).pow(2)
                    })
                    .min()
                    .unwrap_or(10_000)
                    .min(10_000);
                (hills(c) as i64 * 400 + apart, c)
            })
            .unwrap();
        homes.push(home);
        let id = out.len() as u32;
        let language = Language::of(seed, id as u64);
        let living = match def.living.ways.first().map(|w| w.as_str()) {
            Some("herding") => Living::Herding,
            Some("hunting") => Living::Hunting,
            Some("fishing") => Living::Fishing,
            _ => Living::Farming,
        };
        let lean = &def.temperament.convictions;
        let convictions = Topic::ALL
            .iter()
            .map(|&t| {
                let c = (r.gen_range(-0.8f32..0.8) * 100.0).round() / 100.0;
                let l = lean.get(t.name()).copied().unwrap_or(0.0);
                (t, ((c + l).clamp(-1.0, 1.0) * 100.0).round() / 100.0)
            })
            .collect();
        out.push(People {
            id,
            name: language.word(seed, "people"),
            language,
            homeland: home,
            living,
            convictions,
            colour: COLOURS[id as usize % COLOURS.len()],
            species: *species,
            rarity: setup.rarity_of(def).to_string(),
            deep: true,
        });
    }
}
