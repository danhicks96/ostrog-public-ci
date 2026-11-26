//! The region map: elevation, rivers and lakes, temperature, rain and biomes for a world of
//! `size` x `size` regions, the valley being one of them. Regenerated from the seed whenever
//! it is needed and never saved; every stage draws from its own purpose stream.
use crate::fmath::Det;

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use rand::Rng;

use crate::rng::{hash64, purpose, stream};
use crate::world::gen::noise::fbm2;

/// Biomes at region scale (a valley's own surface biomes are finer: world::gen::biome).
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum RBiome {
    Ocean,
    Lake,
    Ice,
    Tundra,
    Taiga,
    TemperateForest,
    Grassland,
    Steppe,
    Desert,
    Savanna,
    Rainforest,
    Swamp,
    Mountain,
}

impl RBiome {
    pub fn label(self) -> &'static str {
        match self {
            RBiome::Ocean => "ocean",
            RBiome::Lake => "lake",
            RBiome::Ice => "ice",
            RBiome::Tundra => "tundra",
            RBiome::Taiga => "taiga",
            RBiome::TemperateForest => "temperate forest",
            RBiome::Grassland => "grassland",
            RBiome::Steppe => "steppe",
            RBiome::Desert => "desert",
            RBiome::Savanna => "savanna",
            RBiome::Rainforest => "rainforest",
            RBiome::Swamp => "swamp",
            RBiome::Mountain => "mountains",
        }
    }

    pub fn is_water(self) -> bool {
        matches!(self, RBiome::Ocean | RBiome::Lake)
    }
}

#[derive(Clone, Debug)]
pub struct RealmParams {
    pub size: usize,
    /// Share of regions under the sea.
    pub sea_fraction: f32,
    /// Mountain chains laid as ridge lines.
    pub ranges: u32,
    /// Degrees Celsius at the cold (north) and warm (south) edges, at sea level.
    pub north_temp: f32,
    pub south_temp: f32,
}

impl Default for RealmParams {
    fn default() -> Self {
        RealmParams {
            size: 192,
            sea_fraction: 0.42,
            ranges: 5,
            north_temp: -6.0,
            south_temp: 33.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RealmMap {
    pub size: usize,
    pub sea_level: f32,
    /// 0..1, normalised.
    pub elevation: Vec<f32>,
    /// The surface water settles at: elevation, or higher where a depression holds a lake.
    pub filled: Vec<f32>,
    /// Where water from each region goes next (index), or usize::MAX at the sea.
    pub down: Vec<usize>,
    /// Water passing through each region (rain gathered from everything upstream).
    pub flow: Vec<f32>,
    pub river: Vec<bool>,
    pub lake: Vec<bool>,
    /// Degrees Celsius, yearly mean.
    pub temperature: Vec<f32>,
    /// 0..1.
    pub rain: Vec<f32>,
    pub biome: Vec<RBiome>,
}

impl RealmMap {
    pub fn idx(&self, x: usize, y: usize) -> usize {
        y * self.size + x
    }

    pub fn xy(&self, i: usize) -> (usize, usize) {
        (i % self.size, i / self.size)
    }

    pub fn land(&self, i: usize) -> bool {
        !self.biome[i].is_water()
    }

    /// Height above the sea, 0 at or below it.
    pub fn height(&self, i: usize) -> f32 {
        (self.elevation[i] - self.sea_level).max(0.0) / (1.0 - self.sea_level).max(1e-6)
    }

    /// A digest of the whole map, for determinism tests.
    pub fn digest(&self) -> u64 {
        let mut h = hash64(&[self.size as u64]);
        for i in 0..self.elevation.len() {
            h = hash64(&[
                h,
                self.elevation[i].to_bits() as u64,
                self.rain[i].to_bits() as u64,
                self.biome[i] as u64,
                self.river[i] as u64,
            ]);
        }
        h
    }
}

const NEIGHBOURS: [(i32, i32); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (-1, 1),
    (1, -1),
    (-1, -1),
];

pub(crate) fn neighbours(size: usize, i: usize) -> impl Iterator<Item = usize> {
    let (x, y) = ((i % size) as i32, (i / size) as i32);
    NEIGHBOURS.iter().filter_map(move |(dx, dy)| {
        let (nx, ny) = (x + dx, y + dy);
        (nx >= 0 && ny >= 0 && nx < size as i32 && ny < size as i32)
            .then(|| ny as usize * size + nx as usize)
    })
}

/// Elevation: a low-frequency continent shape sinking toward the map's edges, finer relief on
/// top, and mountain chains raised along ridge lines with a jagged crest.
fn elevation(seed: u64, p: &RealmParams) -> Vec<f32> {
    let n = p.size;
    let s = hash64(&[seed, purpose::REALM_MAP, 1]);
    let mut r = stream(seed, &[purpose::REALM_MAP, 2]);
    // (start, end, height) of each mountain chain, in map units.
    type Range = ((f32, f32), (f32, f32), f32);
    let ranges: Vec<Range> = (0..p.ranges)
        .map(|_| {
            let a = (r.gen_range(0.15..0.85f32), r.gen_range(0.15..0.85f32));
            let ang: f32 = r.gen_range(0.0..std::f32::consts::TAU);
            let len: f32 = r.gen_range(0.18..0.45);
            let b = (a.0 + ang.d_cos() * len, a.1 + ang.d_sin() * len);
            (a, b, r.gen_range(0.18..0.32))
        })
        .collect();
    let mut e = vec![0.0f32; n * n];
    for y in 0..n {
        for x in 0..n {
            let (u, v) = (x as f32 / n as f32, y as f32 / n as f32);
            let continent = fbm2(s, u * 3.0, v * 3.0, 4, 2.0, 0.55);
            let relief = fbm2(s ^ 0x5151, u * 10.0, v * 10.0, 5, 2.0, 0.5);
            // Sink toward a ragged rim so the world is ringed by sea with a coast that wanders:
            // the distance from the centre is pushed about by noise before it is used.
            let wob = fbm2(s ^ 0x2323, u * 4.0, v * 4.0, 3, 2.0, 0.5) - 0.5;
            let r = ((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt() + wob * 0.28;
            let edge = ((0.5 - r) * 4.0).clamp(0.0, 1.0);
            let mut h = continent * 0.7 + relief * 0.3;
            h = h * (0.3 + 0.7 * edge) - (1.0 - edge) * 0.3;
            for &((ax, ay), (bx, by), height) in &ranges {
                let d = dist_to_segment((u, v), (ax, ay), (bx, by));
                let crest = fbm2(s ^ 0x7777, u * 26.0, v * 26.0, 3, 2.0, 0.5);
                let w = 0.012 + 0.014 * crest;
                h += height * (-(d * d) / (w * w)).d_exp() * (0.45 + 0.55 * crest);
            }
            e[y * n + x] = h;
        }
    }
    let (lo, hi) = e
        .iter()
        .fold((f32::MAX, f32::MIN), |(a, b), &v| (a.min(v), b.max(v)));
    for v in &mut e {
        *v = (*v - lo) / (hi - lo).max(1e-6);
    }
    e
}

fn dist_to_segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (cx, cy) = (a.0 + t * dx, a.1 + t * dy);
    ((p.0 - cx).powi(2) + (p.1 - cy).powi(2)).sqrt()
}

/// Ordered float for the priority flood (elevations are finite).
#[derive(PartialEq, Clone, Copy)]
struct F(f32);
impl Eq for F {}
impl PartialOrd for F {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for F {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&o.0)
    }
}

/// Priority flood from the sea inward: every land region learns the level its water settles at
/// (raised over depressions, which hold lakes) and which neighbour it drains to, so all water
/// reaches the sea and no river ends in a pit.
fn flood(n: usize, e: &[f32], sea: f32) -> (Vec<f32>, Vec<usize>) {
    let mut filled = e.to_vec();
    let mut down = vec![usize::MAX; n * n];
    let mut done = vec![false; n * n];
    let mut heap = BinaryHeap::new();
    for i in 0..n * n {
        let (x, y) = (i % n, i / n);
        if e[i] <= sea || x == 0 || y == 0 || x == n - 1 || y == n - 1 {
            filled[i] = e[i].max(sea.min(e[i]));
            done[i] = true;
            heap.push(Reverse((F(filled[i]), i)));
        }
    }
    while let Some(Reverse((F(level), i))) = heap.pop() {
        for j in neighbours(n, i) {
            if done[j] {
                continue;
            }
            done[j] = true;
            // A tiny rise keeps a filled flat draining toward where the flood came from.
            filled[j] = e[j].max(level + 1e-5);
            down[j] = i;
            heap.push(Reverse((F(filled[j]), j)));
        }
    }
    (filled, down)
}

/// Rain carried by a west wind: moisture picked up over the sea, given up over land, most of
/// all where the ground rises, so ranges leave dry country in their lee.
fn rain(n: usize, e: &[f32], sea: f32, seed: u64) -> Vec<f32> {
    let s = hash64(&[seed, purpose::REALM_MAP, 3]);
    let mut out = vec![0.0f32; n * n];
    for y in 0..n {
        let mut moisture = 1.0f32;
        let mut prev = e[y * n];
        for x in 0..n {
            let i = y * n + x;
            if e[i] <= sea {
                moisture = (moisture + 0.12).min(1.0);
                out[i] = 0.8 * moisture;
            } else {
                let rise = (e[i] - prev).max(0.0) * 30.0;
                let fall = (moisture * (0.012 + 0.5 * rise)).min(moisture);
                moisture -= fall;
                // Local variation, and some regained from land (lakes, forests breathe it back).
                let jitter = fbm2(s, x as f32 / 14.0, y as f32 / 14.0, 3, 2.0, 0.5);
                out[i] = (fall * 6.0 + moisture * 0.75) * (0.7 + 0.6 * jitter);
                moisture = (moisture + 0.004).min(1.0);
            }
            prev = e[i];
        }
    }
    // Rank on land: 0.5 is the median land region, so the biome table reads relative wetness
    // and one steep windward slope cannot squash every other region toward dry.
    let mut land: Vec<(f32, usize)> = (0..n * n)
        .filter(|&i| e[i] > sea)
        .map(|i| (out[i], i))
        .collect();
    land.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.1.cmp(&b.1))
    });
    let m = land.len().max(2) - 1;
    let mut ranked = vec![1.0f32; n * n];
    for (k, &(_, i)) in land.iter().enumerate() {
        ranked[i] = k as f32 / m as f32;
    }
    ranked
}

fn biome_of(temp: f32, rain: f32, height: f32, flat: bool) -> RBiome {
    if height > 0.55 {
        return if temp < 0.0 {
            RBiome::Ice
        } else {
            RBiome::Mountain
        };
    }
    if temp < -6.0 {
        RBiome::Ice
    } else if temp < 1.0 {
        RBiome::Tundra
    } else if temp < 8.0 {
        if rain > 0.3 {
            RBiome::Taiga
        } else {
            RBiome::Steppe
        }
    } else if temp < 20.0 {
        if rain > 0.72 && flat {
            RBiome::Swamp
        } else if rain > 0.5 {
            RBiome::TemperateForest
        } else if rain > 0.26 {
            RBiome::Grassland
        } else {
            RBiome::Steppe
        }
    } else if rain > 0.65 {
        RBiome::Rainforest
    } else if rain > 0.32 {
        RBiome::Savanna
    } else {
        RBiome::Desert
    }
}

pub fn generate(seed: u64, p: &RealmParams) -> RealmMap {
    let n = p.size;
    let elevation = elevation(seed, p);
    let mut sorted = elevation.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let sea_level = sorted[((n * n) as f32 * p.sea_fraction) as usize];
    // Land rises slowly from the coast and steeply at the ranges: broad lowlands and valleys,
    // not one high plateau under the whole continent.
    let elevation: Vec<f32> = elevation
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            // The rim of the map is always sea, so every river has somewhere to end.
            let (x, y) = (i % n, i / n);
            if x == 0 || y == 0 || x == n - 1 || y == n - 1 {
                return v.min(sea_level * 0.98);
            }
            if v <= sea_level {
                v
            } else {
                let h = (v - sea_level) / (1.0 - sea_level).max(1e-6);
                sea_level + h.d_powf(1.7) * (1.0 - sea_level)
            }
        })
        .collect();
    let (filled, down) = flood(n, &elevation, sea_level);
    let rain = rain(n, &elevation, sea_level, seed);
    // Lakes: land whose water stands clearly above the ground.
    let lake: Vec<bool> = (0..n * n)
        .map(|i| elevation[i] > sea_level && filled[i] - elevation[i] > 0.01)
        .collect();
    // Flow: each region passes on its rain and everything that reached it, highest first.
    let mut order: Vec<usize> = (0..n * n).collect();
    order.sort_by(|&a, &b| {
        filled[b]
            .partial_cmp(&filled[a])
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(&b))
    });
    let mut flow: Vec<f32> = rain.iter().map(|r| 0.2 + r).collect();
    for &i in &order {
        if down[i] != usize::MAX {
            let f = flow[i];
            flow[down[i]] += f;
        }
    }
    let land_flows: Vec<f32> = (0..n * n)
        .filter(|&i| elevation[i] > sea_level)
        .map(|i| flow[i])
        .collect();
    let mut lf = land_flows.clone();
    lf.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let threshold = lf
        .get(lf.len() * 97 / 100)
        .copied()
        .unwrap_or(f32::MAX)
        .max(12.0);
    let river: Vec<bool> = (0..n * n)
        .map(|i| elevation[i] > sea_level && !lake[i] && flow[i] >= threshold)
        .collect();
    let temperature: Vec<f32> = (0..n * n)
        .map(|i| {
            let lat = (i / n) as f32 / (n - 1) as f32;
            let h = (elevation[i] - sea_level).max(0.0) / (1.0 - sea_level).max(1e-6);
            p.north_temp + (p.south_temp - p.north_temp) * lat - 26.0 * h.d_powf(1.8)
        })
        .collect();
    let biome: Vec<RBiome> = (0..n * n)
        .map(|i| {
            if elevation[i] <= sea_level {
                return RBiome::Ocean;
            }
            if lake[i] {
                return RBiome::Lake;
            }
            let h = (elevation[i] - sea_level) / (1.0 - sea_level).max(1e-6);
            let flat = neighbours(n, i).all(|j| (elevation[j] - elevation[i]).abs() < 0.012);
            biome_of(temperature[i], rain[i], h, flat)
        })
        .collect();
    RealmMap {
        size: n,
        sea_level,
        elevation,
        filled,
        down,
        flow,
        river,
        lake,
        temperature,
        rain,
        biome,
    }
}

/// The land as RGB bytes (size x size x 3): country by biome, rivers in blue. The Legends
/// browser and the wizard draw settlements and marks on top.
pub fn rgb(m: &RealmMap) -> Vec<u8> {
    let mut out = Vec::with_capacity(m.size * m.size * 3);
    for i in 0..m.size * m.size {
        let c: [u8; 3] = if m.river[i] {
            [64, 128, 200]
        } else {
            match m.biome[i] {
                RBiome::Ocean => [34, 75, 112],
                RBiome::Lake => [65, 128, 159],
                RBiome::Ice => [233, 227, 214],
                RBiome::Tundra => [158, 161, 167],
                RBiome::Taiga => [42, 86, 52],
                RBiome::TemperateForest => [54, 110, 50],
                RBiome::Grassland => [126, 169, 79],
                RBiome::Steppe => [172, 160, 98],
                RBiome::Desert => [206, 167, 115],
                RBiome::Savanna => [170, 170, 85],
                RBiome::Rainforest => [31, 90, 40],
                RBiome::Swamp => [70, 95, 70],
                RBiome::Mountain => [123, 129, 144],
            }
        };
        out.extend_from_slice(&c);
    }
    out
}
