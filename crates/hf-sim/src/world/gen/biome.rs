//! Climate and biomes. Per-column temperature, moisture and stoniness fields, each biased by
//! the world's character (wet or dry, warm or cold, stony or soft), classify every column into
//! a biome. The biome chooses the surface ground (here) and the flora (flora.rs), so a ground
//! or plant family is present in a seed because some column of that seed has a biome that
//! asks for it. Everything is deterministic from the seed and the generation parameters.

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::noise::fbm2;
use super::GenParams;
use crate::ids::TilePos;
use crate::plants::{Plant, PlantKind};
use crate::rng::{hash64, purpose, stream};
use crate::world::{Material, Shape, TileFlags, WorldGrid};

/// The world-level bias that makes two seeds differ in character, not only in arrangement.
/// Each axis is -1..1. Drawn from the seed unless `GenParams::character` sets it.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct Character {
    /// Dry (-1) to wet (1): more lakes, fens and moss when wet; heath when dry.
    pub wetness: f32,
    /// Cold (-1) to warm (1): frozen heights and pines when cold; meadows when warm.
    pub warmth: f32,
    /// Soft (-1) to stony (1): scree, gravel and packed earth on the uplands when stony.
    pub stoniness: f32,
}

impl Character {
    pub fn of_seed(seed: u64) -> Character {
        let mut r = stream(seed, &[purpose::WORLDGEN_CLIMATE]);
        // Wetness and stoniness are pushed away from the middle (square root of the magnitude)
        // so that most valleys lean clearly one way; warmth is uniform, so frozen valleys stay
        // the exception they should be.
        let mut axis = |push: bool| {
            let v: f32 = r.gen_range(-1.0..1.0);
            if push {
                v.signum() * v.abs().sqrt()
            } else {
                v
            }
        };
        let wetness = axis(true);
        let warmth = axis(false);
        let stoniness = axis(true);
        Character {
            wetness,
            warmth,
            stoniness,
        }
    }

    /// The character for a world: the parameter if set, else drawn from the seed.
    pub fn resolve(seed: u64, p: &GenParams) -> Character {
        p.character.unwrap_or_else(|| Character::of_seed(seed))
    }

    /// A few plain words: "a wet, cold, stony landing site".
    pub fn label(&self) -> String {
        let mut words = Vec::new();
        if self.wetness > 0.4 {
            words.push("wet");
        } else if self.wetness < -0.4 {
            words.push("dry");
        }
        if self.warmth > 0.4 {
            words.push("mild");
        } else if self.warmth < -0.4 {
            words.push("cold");
        }
        if self.stoniness > 0.4 {
            words.push("stony");
        } else if self.stoniness < -0.4 {
            words.push("soft-soiled");
        }
        if words.is_empty() {
            "a temperate landing site".to_string()
        } else {
            format!("a {} landing site", words.join(", "))
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Biome {
    Grassland,
    Meadow,
    Woodland,
    Fen,
    Heath,
    Stony,
    Fell,
    Snowfield,
    /// Appended (C59, D28): pale bone trees over white milkstone, where a deep cavern showing the
    /// underworld's stone lies under the land.
    BoneForest,
}

impl Biome {
    pub const ALL: [Biome; 9] = [
        Biome::Grassland,
        Biome::Meadow,
        Biome::Woodland,
        Biome::Fen,
        Biome::Heath,
        Biome::Stony,
        Biome::Fell,
        Biome::Snowfield,
        Biome::BoneForest,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Biome::Grassland => "grassland",
            Biome::Meadow => "meadow",
            Biome::Woodland => "woodland",
            Biome::Fen => "fen",
            Biome::Heath => "heath",
            Biome::Stony => "stony upland",
            Biome::Fell => "fell",
            Biome::Snowfield => "snowfield",
            Biome::BoneForest => "bone forest",
        }
    }

    /// Why a column is this biome, in climate terms.
    pub fn why(self) -> &'static str {
        match self {
            Biome::Grassland => {
                "temperate and moderately moist, nothing stronger; its bare patches follow the \
                 world's character (gravel if stony, packed earth if dry, wet soil if wet, \
                 frozen soil if cold)"
            }
            Biome::Meadow => "mild and moist ground in the meadow patches",
            Biome::Woodland => "moist ground under the forest field",
            Biome::Fen => "wet lowland: high moisture on low ground",
            Biome::Heath => "dry ground: low moisture",
            Biome::Stony => "stony uplands: high stoniness, not wet",
            Biome::Fell => "cold: low temperature",
            Biome::Snowfield => "cold heights: very low temperature on high ground",
            Biome::BoneForest => {
                "above the underworld's stone: a deep cavern where it shows lies under this ground"
            }
        }
    }

    /// The surface ground materials this biome may choose (the generator never puts any other
    /// on a column of this biome; tests hold it to that).
    pub fn grounds(self) -> &'static [Material] {
        match self {
            Biome::Grassland => &[
                Material::GRASS,
                Material::MEADOW,
                Material::SAND,
                Material::SCREE,
                // the world's character showing through (character_patch)
                Material::GRAVEL,
                Material::PACKED_EARTH,
                Material::WET_SOIL,
                Material::MOSS,
                Material::FROZEN_SOIL,
            ],
            Biome::Meadow => &[Material::MEADOW, Material::GRASS, Material::SAND],
            Biome::Woodland => &[Material::GRASS, Material::MOSS, Material::SOIL],
            Biome::Fen => &[
                Material::PEAT,
                Material::WET_SOIL,
                Material::MUD,
                Material::MOSS,
                Material::GRASS,
            ],
            Biome::Heath => &[
                Material::PACKED_EARTH,
                Material::SAND,
                Material::GRASS,
                Material::SCREE,
            ],
            Biome::Stony => &[
                Material::SCREE,
                Material::GRAVEL,
                Material::PACKED_EARTH,
                Material::GRASS,
            ],
            Biome::Fell => &[
                Material::FROZEN_SOIL,
                Material::GRAVEL,
                Material::SCREE,
                Material::GRASS,
            ],
            Biome::Snowfield => &[Material::SNOW, Material::FROZEN_SOIL],
            Biome::BoneForest => &[Material::BONE_MOSS, Material::SCREE, Material::MOSS],
        }
    }

    /// Low plants per biome and their chance per open column (after trees, reeds and vines).
    /// A kind is placed only within its warmth range and the world's flora draw (#486)
    /// (plants.rs `Lore::climate`, #286), so a
    /// warm valley's meadow has madder and a cold one's has none.
    pub fn flora(self) -> &'static [(PlantKind, f32)] {
        use PlantKind::*;
        match self {
            Biome::Grassland => &[
                (BerryBush, 0.025),
                (Shrub, 0.02),
                (RootCrops, 0.006),
                (Root, 0.004),
                (FloweringPlant, 0.006),
                (Herbs, 0.004),
                (Wildflowers, 0.004),
                (Fern, 0.002),
                // #286
                (Bramble, 0.008),
                (Hazel, 0.005),
                (WildStrawberry, 0.004),
                (Yarrow, 0.004),
                (Woad, 0.003),
                (Madder, 0.002),
                (Nettle, 0.003),
                (Hemlock, 0.002),
                (Nightshade, 0.002),
            ],
            Biome::Meadow => &[
                (Wildflowers, 0.06),
                (FloweringPlant, 0.04),
                (Root, 0.012),
                (Herbs, 0.012),
                (BerryBush, 0.03),
                (Shrub, 0.01),
                (RootCrops, 0.006),
                (WildStrawberry, 0.012),
                (Yarrow, 0.01),
                (Madder, 0.006),
                (Woad, 0.004),
                (Hemlock, 0.003),
            ],
            Biome::Woodland => &[
                (Fern, 0.055),
                (BerryBush, 0.025),
                (Shrub, 0.03),
                (Herbs, 0.012),
                (Vine, 0.012),
                (FallenLog, 0.01),
                (FloweringPlant, 0.004),
                (Hazel, 0.012),
                (WildGarlic, 0.01),
                (Bramble, 0.008),
                (Chanterelle, 0.006),
                (Bilberry, 0.006),
                (Nettle, 0.005),
                (LilyOfTheValley, 0.004),
                (DeathCap, 0.003),
                (Nightshade, 0.003),
                (Comfrey, 0.003),
            ],
            Biome::Fen => &[
                (Fern, 0.03),
                (BerryBush, 0.015),
                (Herbs, 0.01),
                (Shrub, 0.01),
                (FloweringPlant, 0.006),
                (Cattail, 0.03),
                (Comfrey, 0.008),
                (Nettle, 0.006),
                (Hemlock, 0.005),
            ],
            Biome::Heath => &[
                (Shrub, 0.04),
                (Herbs, 0.03),
                (BerryBush, 0.01),
                (RootCrops, 0.012),
                (Wildflowers, 0.004),
                (Bilberry, 0.015),
                (Weld, 0.006),
                (Yarrow, 0.006),
                (Woad, 0.004),
            ],
            Biome::Stony => &[
                (Shrub, 0.02),
                (Herbs, 0.012),
                (BerryBush, 0.012),
                (Vine, 0.01),
                (Fern, 0.003),
                (Weld, 0.008),
                (Bilberry, 0.006),
                (Yarrow, 0.003),
            ],
            Biome::Fell => &[
                (Shrub, 0.025),
                (BerryBush, 0.008),
                (Fern, 0.006),
                (Herbs, 0.004),
                (Bilberry, 0.012),
            ],
            Biome::Snowfield => &[(Shrub, 0.01), (BerryBush, 0.004), (Bilberry, 0.003)],
            Biome::BoneForest => &[
                (Fern, 0.02),
                (Herbs, 0.01),
                (FallenLog, 0.008),
                (Shrub, 0.006),
            ],
        }
    }

    /// The trees of each biome and their weights (#286): `Tree` is oak, birch or pine by
    /// `tree_species`; alder and willow grow only within three tiles of water, and each kind
    /// only within its warmth range and the world's flora draw (#486)
    /// (world/gen/flora.rs falls back to `Tree`).
    pub fn trees(self) -> &'static [(PlantKind, u32)] {
        use PlantKind::*;
        match self {
            Biome::Grassland => &[(Tree, 72), (Ash, 12), (CrabApple, 8), (Willow, 8)],
            Biome::Meadow => &[(Tree, 62), (CrabApple, 18), (Ash, 10), (Willow, 10)],
            Biome::Woodland => &[(Tree, 66), (Ash, 16), (Yew, 8), (Alder, 5), (Willow, 5)],
            Biome::Fen => &[(Tree, 40), (Alder, 36), (Willow, 24)],
            Biome::Heath => &[(Tree, 82), (Yew, 12), (Ash, 6)],
            Biome::Stony => &[(Tree, 74), (Yew, 26)],
            Biome::Fell | Biome::Snowfield => &[(Tree, 100)],
            Biome::BoneForest => &[(BoneTree, 100)],
        }
    }

    /// Multiplier on the forest field's tree chance.
    pub fn tree_factor(self) -> f32 {
        match self {
            Biome::Woodland => 1.4,
            Biome::Grassland => 1.0,
            Biome::Meadow => 0.6,
            Biome::Fell => 0.6,
            Biome::Heath => 0.5,
            Biome::Fen => 0.35,
            Biome::Stony => 0.35,
            Biome::Snowfield => 0.25,
            Biome::BoneForest => 1.3,
        }
    }

    /// Tree species weights (oak, birch, pine): the client draws `variant % 3` in that order.
    pub fn tree_species(self) -> [u32; 3] {
        match self {
            Biome::Grassland | Biome::Meadow => [50, 35, 15],
            Biome::Woodland => [45, 35, 20],
            Biome::Fen => [20, 70, 10],
            Biome::Heath => [40, 20, 40],
            Biome::Stony => [15, 30, 55],
            Biome::Fell => [0, 15, 85],
            Biome::Snowfield => [0, 0, 100],
            // Bone trees grow there instead (world/gen/flora.rs); these never apply.
            Biome::BoneForest => [0, 0, 0],
        }
    }
}

/// The climate fields and the biome of every column (index `grid.col(x, y)`).
#[derive(Clone, Debug)]
pub struct Climate {
    pub character: Character,
    pub temperature: Vec<f32>,
    pub moisture: Vec<f32>,
    pub stoniness: Vec<f32>,
    /// The forest field (also what flora.rs reads for tree density).
    pub forest: Vec<f32>,
    /// Chebyshev distance to the nearest water column, capped at 5.
    pub water_dist: Vec<u8>,
    /// True where a 4-neighbour's ground is higher: the foot of a rise.
    pub rise: Vec<bool>,
    pub biome: Vec<Biome>,
}

/// Compute the climate after relief and water are in place.
pub fn climate(seed: u64, p: &GenParams, grid: &WorldGrid, water: &[bool]) -> Climate {
    let ch = Character::resolve(seed, p);
    let base = hash64(&[seed, purpose::WORLDGEN_CLIMATE]);
    let (st, sm, ss, smd) = (base ^ 0x7e1, base ^ 0x3015, base ^ 0x5701, base ^ 0x3ead);
    let sf = hash64(&[seed, purpose::WORLDGEN_FLORA]);
    let scale = 1.0 / (p.width.max(p.height) as f32 * 0.5);
    let n = (p.width * p.height) as usize;
    let relief = p.relief.max(1) as f32;
    let mut c = Climate {
        character: ch,
        temperature: Vec::with_capacity(n),
        moisture: Vec::with_capacity(n),
        stoniness: Vec::with_capacity(n),
        forest: Vec::with_capacity(n),
        water_dist: Vec::with_capacity(n),
        rise: Vec::with_capacity(n),
        biome: Vec::with_capacity(n),
    };
    for y in 0..p.height {
        for x in 0..p.width {
            let g = grid.ground_z(x, y);
            let e = (g as f32 / relief).clamp(0.0, 1.0);
            let (fx, fy) = (x as f32 * scale, y as f32 * scale);
            let mut d = 5u8;
            'search: for r in 0..5i32 {
                for dy in -r..=r {
                    for dx in -r..=r {
                        if dx.abs() != r && dy.abs() != r {
                            continue;
                        }
                        let (qx, qy) = (x + dx, y + dy);
                        if qx >= 0
                            && qy >= 0
                            && qx < p.width
                            && qy < p.height
                            && water[grid.col(qx, qy)]
                        {
                            d = r as u8;
                            break 'search;
                        }
                    }
                }
            }
            let rise = [(0, -1), (1, 0), (0, 1), (-1, 0)].iter().any(|(dx, dy)| {
                let (qx, qy) = (x + dx, y + dy);
                qx >= 0 && qy >= 0 && qx < p.width && qy < p.height && grid.ground_z(qx, qy) > g
            });
            let tn = fbm2(st, fx * 2.0, fy * 2.0, 3, 2.0, 0.5);
            let mn = fbm2(sm, fx * 2.5 + 11.0, fy * 2.5, 4, 2.0, 0.5);
            let sn = fbm2(ss, fx * 3.0, fy * 3.0 + 5.0, 3, 2.0, 0.5);
            let forest = fbm2(sf, fx * 4.0, fy * 4.0, 4, 2.0, 0.5);
            let meadow = fbm2(smd, fx * 3.5, fy * 3.5, 3, 2.0, 0.5);
            let near_water = if d <= 4 {
                0.12 * (1.0 - d as f32 / 5.0)
            } else {
                0.0
            };
            let temp = 0.62 + 0.25 * ch.warmth - 0.35 * e + (tn - 0.5) * 0.9;
            let moist = 0.55 + 0.22 * ch.wetness - 0.1 * e + (mn - 0.5) * 1.4 + near_water;
            let stony = 0.3
                + 0.35 * ch.stoniness
                + 0.45 * e
                + (sn - 0.5) * 1.2
                + if rise { 0.08 } else { 0.0 };
            let b = classify(temp, moist, stony, forest, meadow, e);
            c.temperature.push(temp);
            c.moisture.push(moist);
            c.stoniness.push(stony);
            c.forest.push(forest);
            c.water_dist.push(d);
            c.rise.push(rise);
            c.biome.push(b);
        }
    }
    // The bone forest (C59, D28): over a deep cavern where the underworld's stone shows, the
    // land above turns pale, a little wider than the cavern beneath it. Nowhere else.
    // Its edge wanders (a ragged noise on the radius), so it is a stand of wood, not a circle.
    let sb = base ^ 0xB0E;
    for cv in grid.caverns.iter().filter(|cv| cv.underworld) {
        let (rx, ry) = (cv.rx as f32 + 6.0, cv.ry as f32 + 6.0);
        for y in (cv.centre.y - cv.ry - 6).max(0)..(cv.centre.y + cv.ry + 7).min(p.height) {
            for x in (cv.centre.x - cv.rx - 6).max(0)..(cv.centre.x + cv.rx + 7).min(p.width) {
                let (dx, dy) = ((x - cv.centre.x) as f32 / rx, (y - cv.centre.y) as f32 / ry);
                let edge = 0.55 + 0.6 * fbm2(sb, x as f32 * 0.18, y as f32 * 0.18, 3, 2.0, 0.5);
                if dx * dx + dy * dy <= edge * edge && !water[grid.col(x, y)] {
                    c.biome[grid.col(x, y)] = Biome::BoneForest;
                }
            }
        }
    }
    c
}

fn classify(temp: f32, moist: f32, stony: f32, forest: f32, meadow: f32, e: f32) -> Biome {
    if temp < 0.14 && e >= 0.6 {
        Biome::Snowfield
    } else if temp < 0.27 {
        Biome::Fell
    } else if moist > 0.7 && e <= 0.5 {
        Biome::Fen
    } else if stony > 0.78 && moist < 0.62 {
        Biome::Stony
    } else if moist < 0.3 {
        Biome::Heath
    } else if forest > 0.55 && moist > 0.4 {
        Biome::Woodland
    } else if meadow > 0.53 && temp > 0.38 {
        Biome::Meadow
    } else {
        Biome::Grassland
    }
}

/// Lay the surface ground by biome: the top tile of every dry column that is still the
/// undisturbed soil the strata left there.
pub fn surface(seed: u64, grid: &mut WorldGrid, water: &[bool], c: &Climate) {
    let s = hash64(&[seed, purpose::WORLDGEN_SURFACE]);
    for y in 0..grid.height {
        for x in 0..grid.width {
            let i = grid.col(x, y);
            if water[i] {
                continue;
            }
            let top = TilePos::new(x, y, grid.ground_z(x, y));
            let t = grid.get(top);
            if t.shape != Shape::Wall || t.material != Material::SOIL {
                continue;
            }
            // Patches a few tiles across, plus a per-tile jitter for scattered picks.
            let pn = fbm2(s, x as f32 * 0.3, y as f32 * 0.3, 2, 2.0, 0.5);
            let j = (hash64(&[s, x as u64, y as u64]) >> 40) as f32 / (1u64 << 24) as f32;
            let mut m = ground_for(c.biome[i], pn, j, c.moisture[i], c.water_dist[i], c.rise[i]);
            if c.biome[i] == Biome::Grassland && m == Material::GRASS {
                let q = fbm2(s ^ 0x99, x as f32 * 0.16, y as f32 * 0.16, 2, 2.0, 0.5);
                m = character_patch(&c.character, q, j).unwrap_or(m);
            }
            debug_assert!(c.biome[i].grounds().contains(&m));
            let mut nt = t;
            nt.material = m;
            nt.flags = TileFlags::NONE;
            grid.set(top, nt);
            // Under the bone forest the rock is milkstone, eight levels down (C59).
            if c.biome[i] == Biome::BoneForest {
                for z in (top.z - 8)..top.z {
                    let q = TilePos::new(x, y, z);
                    let rt = grid.get(q);
                    if rt.shape == Shape::Wall
                        && grid.materials.get(rt.material).kind
                            == crate::world::material::MaterialKind::Stone
                    {
                        let mut mt = rt;
                        mt.material = Material::MILKSTONE;
                        grid.set(q, mt);
                    }
                }
            }
        }
    }
}

/// Which leaning makes grassland take this family as a character patch, if any.
fn character_patch_leaning(family: &str) -> Option<&'static str> {
    match family {
        "gravel" | "scree" => Some("stony"),
        "packed_earth" | "sand" => Some("dry"),
        "wet_soil" | "mossy" => Some("wet"),
        "frozen_soil" => Some("cold"),
        _ => None,
    }
}

/// Bare patches in open grassland that show the world's character: the strongest leaning
/// axis decides the ground, its strength how much of the grass it takes (none when the world
/// leans no way).
fn character_patch(ch: &Character, q: f32, j: f32) -> Option<Material> {
    let axes = [
        (
            ch.stoniness,
            if j < 0.5 {
                Material::GRAVEL
            } else {
                Material::SCREE
            },
        ),
        (
            -ch.wetness,
            if j < 0.7 {
                Material::PACKED_EARTH
            } else {
                Material::SAND
            },
        ),
        (
            ch.wetness,
            if j < 0.6 {
                Material::WET_SOIL
            } else {
                Material::MOSS
            },
        ),
        (-ch.warmth, Material::FROZEN_SOIL),
    ];
    let (amount, m) =
        axes.iter().copied().fold(
            (0.0f32, Material::GRASS),
            |a, b| if b.0 > a.0 { b } else { a },
        );
    let amount = (amount - 0.25).max(0.0) / 0.75;
    (amount > 0.0 && q < 0.33 + 0.14 * amount).then_some(m)
}

fn ground_for(b: Biome, pn: f32, j: f32, moist: f32, d: u8, rise: bool) -> Material {
    use Material as M;
    match b {
        Biome::Grassland => {
            if rise && j < 0.25 {
                M::SCREE
            } else if d == 1 && pn > 0.6 {
                M::SAND
            } else if pn > 0.68 {
                M::MEADOW
            } else {
                M::GRASS
            }
        }
        Biome::Meadow => {
            if d == 1 && pn > 0.62 {
                M::SAND
            } else if pn < 0.36 {
                M::GRASS
            } else {
                M::MEADOW
            }
        }
        Biome::Woodland => {
            if (moist > 0.5 && pn > 0.48) || d == 1 {
                M::MOSS
            } else if pn < 0.3 {
                M::SOIL
            } else {
                M::GRASS
            }
        }
        Biome::Fen => {
            if d == 1 {
                M::MUD
            } else if pn < 0.36 {
                M::PEAT
            } else if pn < 0.47 {
                M::WET_SOIL
            } else if pn < 0.53 {
                M::MUD
            } else if pn < 0.6 || j < 0.08 {
                M::MOSS
            } else {
                M::GRASS
            }
        }
        Biome::Heath => {
            if rise && j < 0.4 {
                M::SCREE
            } else if pn < 0.4 {
                M::PACKED_EARTH
            } else if pn < 0.5 || d == 1 {
                M::SAND
            } else {
                M::GRASS
            }
        }
        Biome::Stony => {
            if rise {
                M::SCREE
            } else if pn < 0.38 {
                M::GRAVEL
            } else if pn < 0.52 {
                M::SCREE
            } else if pn < 0.66 {
                M::PACKED_EARTH
            } else {
                M::GRASS
            }
        }
        Biome::Fell => {
            if rise && j < 0.6 {
                M::SCREE
            } else if pn < 0.5 {
                M::FROZEN_SOIL
            } else if pn < 0.6 {
                M::GRAVEL
            } else {
                M::GRASS
            }
        }
        Biome::Snowfield => {
            if pn < 0.38 {
                M::FROZEN_SOIL
            } else {
                M::SNOW
            }
        }
        Biome::BoneForest => {
            if rise && j < 0.3 {
                M::SCREE
            } else if moist > 0.7 && pn > 0.6 {
                M::MOSS
            } else {
                M::BONE_MOSS
            }
        }
    }
}

/// Counts that say what a generated world holds and why.
#[derive(Clone, Debug, Default)]
pub struct Census {
    /// Land columns per biome (Biome::ALL order).
    pub biomes: Vec<(Biome, usize)>,
    /// Surface families on land columns plus "water" and "deep_water" (sorted by family name).
    pub grounds: Vec<(String, usize)>,
    /// Plants per kind (PlantKind::ALL order).
    pub plants: Vec<(PlantKind, usize)>,
}

impl Census {
    pub fn ground(&self, family: &str) -> usize {
        self.grounds
            .iter()
            .find(|(f, _)| f == family)
            .map_or(0, |(_, n)| *n)
    }
    pub fn plant(&self, k: PlantKind) -> usize {
        self.plants
            .iter()
            .find(|(p, _)| *p == k)
            .map_or(0, |(_, n)| *n)
    }
    pub fn biome(&self, b: Biome) -> usize {
        self.biomes
            .iter()
            .find(|(p, _)| *p == b)
            .map_or(0, |(_, n)| *n)
    }
}

pub fn census(grid: &WorldGrid, plants: &[Plant]) -> Census {
    let mut grounds: std::collections::BTreeMap<String, usize> = Default::default();
    let mut biomes = vec![0usize; Biome::ALL.len()];
    for y in 0..grid.height {
        for x in 0..grid.width {
            let g = grid.ground_z(x, y);
            let w = grid.get(TilePos::new(x, y, g + 1));
            let fam = if w.is_water() {
                if w.flags.has(TileFlags::DEEP) {
                    "deep_water".to_string()
                } else {
                    "water".to_string()
                }
            } else {
                if let Some(b) = grid.biome.get(grid.col(x, y)) {
                    biomes[*b as usize] += 1;
                }
                let t = grid.get(TilePos::new(x, y, g));
                grid.materials.get(t.material).family.clone()
            };
            *grounds.entry(fam).or_default() += 1;
        }
    }
    let mut pk = vec![0usize; PlantKind::ALL.len()];
    for p in plants.iter().filter(|p| p.alive) {
        pk[p.kind as usize] += 1;
    }
    Census {
        biomes: Biome::ALL
            .iter()
            .map(|b| (*b, biomes[*b as usize]))
            .collect(),
        grounds: grounds.into_iter().collect(),
        plants: PlantKind::ALL
            .iter()
            .map(|k| (*k, pk[*k as usize]))
            .collect(),
    }
}

/// The surface families the generator can choose, and the biomes that choose each.
pub fn ground_sources() -> Vec<(String, Vec<Biome>)> {
    let table = crate::world::MaterialTable::default();
    let mut out: Vec<(String, Vec<Biome>)> = Vec::new();
    for b in Biome::ALL {
        for m in b.grounds() {
            let fam = table.get(*m).family.clone();
            match out.iter_mut().find(|(f, _)| *f == fam) {
                Some((_, v)) => v.push(b),
                None => out.push((fam, vec![b])),
            }
        }
    }
    out
}

/// Lines for `ostrog describe`: the character, the biome mix, and every ground and plant
/// family with its count, or the reason it is absent. The flora report uses the world's actual
/// record; `None` is a legacy world whose draw is unknown.
pub fn describe(
    seed: u64,
    p: &GenParams,
    grid: &WorldGrid,
    plants: &[Plant],
    flora_draw: Option<&super::flora::FloraDraw>,
) -> Vec<String> {
    let ch = Character::resolve(seed, p);
    let cs = census(grid, plants);
    let land: usize = cs.biomes.iter().map(|(_, n)| n).sum::<usize>().max(1);
    let mut out = vec![format!(
        "character: wetness {:+.2} warmth {:+.2} stoniness {:+.2} ({}){}",
        ch.wetness,
        ch.warmth,
        ch.stoniness,
        ch.label(),
        if p.character.is_some() {
            ", set by parameter"
        } else {
            ", from the seed"
        }
    )];
    let mut mix: Vec<(Biome, usize)> = cs.biomes.clone();
    mix.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    out.push(format!(
        "biome mix: {}",
        mix.iter()
            .filter(|(_, n)| *n > 0)
            .map(|(b, n)| {
                let pc = (n * 100 + land / 2) / land;
                if pc == 0 {
                    format!("{} <1%", b.label())
                } else {
                    format!("{} {}%", b.label(), pc)
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    ));
    for (b, n) in &mix {
        if *n > 0 {
            out.push(format!("  {:<13} {:>5} columns: {}", b.label(), n, b.why()));
        }
    }
    let absent_b: Vec<&str> = mix
        .iter()
        .filter(|(_, n)| *n == 0)
        .map(|(b, _)| b.label())
        .collect();
    if !absent_b.is_empty() {
        out.push(format!(
            "  absent biomes: {} (the climate never reaches them here)",
            absent_b.join(", ")
        ));
    }
    out.push("ground families:".to_string());
    let mut sources = ground_sources();
    sources.push(("water".into(), vec![]));
    sources.push(("deep_water".into(), vec![]));
    for (fam, from) in sources {
        let n = cs.ground(&fam);
        let names: Vec<&str> = from.iter().map(|b| b.label()).collect();
        let why = match fam.as_str() {
            "water" => "streams and lakes on low ground".to_string(),
            "deep_water" => "lake centres two or more tiles from any shore".to_string(),
            _ => format!("chosen in {}", names.join(", ")),
        };
        let present: Vec<&str> = from
            .iter()
            .filter(|b| cs.biome(**b) > 0)
            .map(|b| b.label())
            .collect();
        let patch = character_patch_leaning(&fam);
        if n > 0 {
            out.push(format!("  {:<13} {:>5}  {}", fam, n, why));
        } else if from.is_empty() {
            out.push(format!(
                "  {:<13}     0  absent: {}, and there are none",
                fam, why
            ));
        } else if present.is_empty() {
            out.push(format!(
                "  {:<13}     0  absent: none of {} in this seed",
                fam,
                names.join(", ")
            ));
        } else if present == ["grassland"] && patch.is_some() {
            out.push(format!(
                "  {:<13}     0  absent: none of {} here, and grassland takes it only when the site leans most to {}; this is {}",
                fam,
                names
                    .iter()
                    .filter(|n| **n != "grassland")
                    .copied()
                    .collect::<Vec<_>>()
                    .join(", "),
                patch.unwrap_or(""),
                ch.label()
            ));
        } else {
            out.push(format!(
                "  {:<13}     0  absent: {} are here but the patch noise never picked it",
                fam,
                present.join(", ")
            ));
        }
    }
    if let Some(draw) = flora_draw {
        let drawn: Vec<&str> = super::flora::FLORA_CANDIDATES
            .iter()
            .filter(|k| draw.allows(**k))
            .map(|k| k.label())
            .collect();
        let not_drawn: Vec<&str> = super::flora::FLORA_CANDIDATES
            .iter()
            .filter(|k| !draw.allows(**k))
            .map(|k| k.label())
            .collect();
        out.push(format!(
            "flora draw: {} of {} wild kinds: {}",
            drawn.len(),
            super::flora::FLORA_CANDIDATES.len(),
            drawn.join(", ")
        ));
        out.push(format!("flora not drawn: {}", not_drawn.join(", ")));
    } else {
        out.push("flora draw: not recorded in this world (legacy save)".to_string());
    }
    out.push("plants:".to_string());
    for (k, n) in &cs.plants {
        if *n == 0
            && flora_draw
                .is_some_and(|draw| super::flora::FLORA_CANDIDATES.contains(k) && !draw.allows(*k))
        {
            out.push(format!(
                "  {:<17}     0  not drawn for this world",
                k.label()
            ));
            continue;
        }
        let from: Vec<&str> = Biome::ALL
            .iter()
            .filter(|b| b.flora().iter().any(|(f, _)| f == k))
            .map(|b| b.label())
            .collect();
        let why = match k {
            PlantKind::Tree => "the forest field, scaled by biome".to_string(),
            PlantKind::Reed => "near water, most in fen".to_string(),
            PlantKind::Mushroom => "cave floors".to_string(),
            k if k.is_crop() => "sown by settlers".to_string(),
            PlantKind::Stump => "left where a tree is felled".to_string(),
            PlantKind::FungusTree => "in caverns, under the surface".to_string(),
            PlantKind::Vine => format!("the foot of rises, and {}", from.join(", ")),
            k if k.is_green_tree() => {
                let woods: Vec<&str> = Biome::ALL
                    .iter()
                    .filter(|b| b.trees().iter().any(|(f, _)| f == k))
                    .map(|b| b.label())
                    .collect();
                format!("trees of {}", woods.join(", "))
            }
            // Placed by something other than a surface biome's flora (#108): say so rather
            // than print an empty list.
            _ if from.is_empty() => "placed outside the surface biomes".to_string(),
            _ => format!("in {}", from.join(", ")),
        };
        out.push(format!("  {:<17} {:>5}  {}", k.label(), n, why));
    }
    out
}
