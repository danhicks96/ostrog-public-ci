//! Flora placement and founding-site choice.

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::biome::{Biome, Climate};
use super::GenParams;
use crate::ids::TilePos;
use crate::plants::{Plant, PlantKind};
use crate::rng::{purpose, stream};
use crate::world::{Shape, WorldGrid};

pub const FLORA_DRAW_COUNT: usize = 8;

/// The wild kinds added by #286. The original flora, cave flora and crops are not bag-drawn.
pub const FLORA_CANDIDATES: [PlantKind; 22] = [
    PlantKind::Alder,
    PlantKind::Ash,
    PlantKind::Yew,
    PlantKind::Willow,
    PlantKind::CrabApple,
    PlantKind::Hazel,
    PlantKind::Bilberry,
    PlantKind::WildStrawberry,
    PlantKind::Bramble,
    PlantKind::WildGarlic,
    PlantKind::Chanterelle,
    PlantKind::Cattail,
    PlantKind::DeathCap,
    PlantKind::Hemlock,
    PlantKind::Nightshade,
    PlantKind::LilyOfTheValley,
    PlantKind::Nettle,
    PlantKind::Woad,
    PlantKind::Madder,
    PlantKind::Weld,
    PlantKind::Yarrow,
    PlantKind::Comfrey,
];

/// The world's recorded draw from the existing wild catalogue (#486).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FloraDraw {
    pub drawn: Vec<PlantKind>,
}

impl FloraDraw {
    pub fn of_seed(seed: u64) -> FloraDraw {
        let mut r = stream(seed, &[purpose::WORLDGEN_FLORA_DRAW]);
        let mut bag = FLORA_CANDIDATES.to_vec();
        for i in (1..bag.len()).rev() {
            let j = r.gen_range(0..=i);
            bag.swap(i, j);
        }
        bag.truncate(FLORA_DRAW_COUNT);
        bag.sort();
        FloraDraw { drawn: bag }
    }

    pub fn allows(&self, kind: PlantKind) -> bool {
        !FLORA_CANDIDATES.contains(&kind) || self.drawn.contains(&kind)
    }
}

pub fn apply(
    seed: u64,
    p: &GenParams,
    grid: &WorldGrid,
    water: &[bool],
    climate: &Climate,
    draw: &FloraDraw,
) -> Vec<Plant> {
    let mut r = stream(seed, &[purpose::WORLDGEN_FLORA]);
    // Kind-specific draws must not shift later site rolls (FIXLOG 2026-10-31, #486).
    let mut selection = stream(seed, &[purpose::WORLDGEN_FLORA, 1]);
    let mut variation = stream(seed, &[purpose::WORLDGEN_FLORA, 2]);
    let mut plants = Vec::new();
    for y in 0..p.height {
        for x in 0..p.width {
            let c = grid.col(x, y);
            if water[c] {
                continue;
            }
            let surf = grid.surface(x, y);
            if !grid.walkable(surf) {
                continue;
            }
            let b = climate.biome[c];
            let forest = climate.forest[c];
            let near_water = climate.water_dist[c] <= 3;
            let tf = b.tree_factor();
            let roll: f32 = r.gen();
            let roll2: f32 = r.gen();
            let reed_chance = match b {
                Biome::Fen if near_water => 0.3,
                Biome::Fen => 0.06,
                Biome::Fell | Biome::Snowfield if near_water => 0.03,
                _ if near_water => 0.12,
                _ => 0.0,
            };
            let vine_chance = match (climate.rise[c], b) {
                (true, Biome::Stony) => 0.08,
                (true, _) => 0.03,
                _ => 0.0,
            };
            // The bone forest is a forest throughout, not only where the forest field runs high.
            let bone_wood = b == Biome::BoneForest && roll < 0.1 + p.tree_density * forest;
            let temp = climate.temperature[c];
            let suits = |k: PlantKind| {
                let (lo, hi) = k.lore().climate;
                (lo..=hi).contains(&temp)
            };
            let kind = if bone_wood
                || (forest > 0.6 && roll < p.tree_density * tf)
                || (forest > 0.45 && roll < 0.08 * tf)
            {
                // #486 / FIXLOG 2026-10-31: an undrawn tree must not leak through the weighted
                // biome table. The core Tree remains as the habitat fallback.
                let w: Vec<_> = b
                    .trees()
                    .iter()
                    .copied()
                    .filter(|(k, _)| draw.allows(*k))
                    .collect();
                let mut pick = selection.gen_range(0..w.iter().map(|(_, n)| n).sum::<u32>());
                let k = w
                    .iter()
                    .find(|(_, n)| {
                        let hit = pick < *n;
                        pick = pick.saturating_sub(*n);
                        hit
                    })
                    .map(|(k, _)| *k)
                    .unwrap_or(PlantKind::Tree);
                let by_water = matches!(k, PlantKind::Alder | PlantKind::Willow);
                Some(if suits(k) && (!by_water || climate.water_dist[c] <= 3) {
                    k
                } else {
                    PlantKind::Tree
                })
            } else if roll2 < reed_chance {
                Some(PlantKind::Reed)
            } else if roll2 < reed_chance + vine_chance {
                Some(PlantKind::Vine)
            } else {
                // #486 / FIXLOG 2026-10-31: normalize only climate-eligible kinds, so a drawn
                // but unsuitable kind cannot silently reduce realized low-flora density.
                let total: f32 = b
                    .flora()
                    .iter()
                    .filter(|(k, _)| suits(*k))
                    .map(|(_, chance)| chance)
                    .sum();
                let available: f32 = b
                    .flora()
                    .iter()
                    .filter(|(k, _)| draw.allows(*k) && suits(*k))
                    .map(|(_, chance)| chance)
                    .sum();
                let scale = if available > 0.0 {
                    total / available
                } else {
                    0.0
                };
                let mut acc = reed_chance + vine_chance;
                b.flora()
                    .iter()
                    .filter(|(k, _)| draw.allows(*k) && suits(*k))
                    .find_map(|(k, chance)| {
                        acc += chance * scale;
                        (roll2 < acc).then_some(*k)
                    })
            };
            if let Some(kind) = kind {
                let growth = if kind.grows() {
                    variation.gen_range(0.4..1.0)
                } else {
                    1.0
                };
                let variant = if kind == PlantKind::Tree {
                    // The client draws species `variant % 3` (oak, birch, pine); variants stay
                    // below 1000 because the bridge sends `variant % 1000`.
                    let w = b.tree_species();
                    let pick = variation.gen_range(0..w.iter().sum::<u32>());
                    let sp = if pick < w[0] {
                        0
                    } else if pick < w[0] + w[1] {
                        1
                    } else {
                        2
                    };
                    variation.gen_range(0..333u32) * 3 + sp
                } else {
                    variation.gen()
                };
                plants.push(Plant::new(surf, kind, growth, variant));
            }
        }
    }
    // Mushrooms on cave floors: an Open tile below ground with solid rock beneath.
    for ((cx, cy, z), chunk) in grid.chunks.iter() {
        if *z >= 0 {
            continue;
        }
        for (i, t) in chunk.tiles.iter().enumerate() {
            // Nothing grows in a cavern pool.
            if t.shape != Shape::Open || t.is_water() {
                continue;
            }
            let x = cx * crate::world::CHUNK_SIZE as i32 + (i % crate::world::CHUNK_SIZE) as i32;
            let y = cy * crate::world::CHUNK_SIZE as i32 + (i / crate::world::CHUNK_SIZE) as i32;
            let p = TilePos::new(x, y, *z);
            // A cavern floor grows a forest of fungus: tall stalks with mushrooms between.
            if grid.walkable(p)
                && grid
                    .caverns
                    .iter()
                    .any(|c| c.centre.z == *z && c.contains(p))
            {
                let roll: f32 = r.gen();
                if roll < 0.22 {
                    plants.push(Plant::new(
                        p,
                        PlantKind::FungusTree,
                        r.gen_range(0.6..1.0),
                        r.gen(),
                    ));
                } else if roll < 0.42 {
                    plants.push(Plant::new(
                        p,
                        PlantKind::Mushroom,
                        r.gen_range(0.5..1.0),
                        r.gen(),
                    ));
                }
                continue;
            }
            if grid.walkable(p) && r.gen::<f32>() < 0.12 {
                plants.push(Plant::new(
                    p,
                    PlantKind::Mushroom,
                    r.gen_range(0.5..1.0),
                    r.gen(),
                ));
            }
        }
    }
    plants
}

/// Score candidate columns for a founding site. A site must be survivable: flat ground to
/// build on, water within reach, trees for wood and food plants nearby, not too far from the
/// middle of the map; fen, fell and snowfield are excluded while anything else is offered.
/// Among survivable sites the settlers prefer ground typical of the valley: the biome mix
/// around the site is scored against the valley's own mix, so a dry valley founds on heath
/// or dry grassland and a stony one near its scree, not in the one moist wood it has.
pub fn founding_site(seed: u64, grid: &WorldGrid, water: &[bool], plants: &[Plant]) -> TilePos {
    // The valley's biome mix over land columns.
    let mut valley = [0f32; Biome::ALL.len()];
    for (c, b) in grid.biome.iter().enumerate() {
        if !water[c] {
            valley[*b as usize] += 1.0;
        }
    }
    let land = valley.iter().sum::<f32>().max(1.0);
    valley.iter_mut().for_each(|v| *v /= land);
    let mut best = (f32::MIN, grid.surface(grid.width / 2, grid.height / 2));
    // The best excluded site, used only if every sample is fen, fell or snowfield.
    let mut fallback = best;
    let mut r = stream(seed, &[purpose::WORLDGEN_FOUNDERS]);
    let samples = 400;
    for _ in 0..samples {
        let x = r.gen_range(8..grid.width - 8);
        let y = r.gen_range(8..grid.height - 8);
        let c = grid.col(x, y);
        // Keep the village's surroundings on the map: a site by the edge is half off it.
        let m = (grid.width.min(grid.height) / 8).min(16);
        if water[c] || x < m || y < m || x >= grid.width - m || y >= grid.height - m {
            continue;
        }
        let g = grid.ground_z(x, y);
        let mut flat = 0;
        let mut wet = 0;
        for dy in -6..=6 {
            for dx in -6..=6 {
                let (qx, qy) = (x + dx, y + dy);
                let qc = grid.col(qx, qy);
                if water[qc] {
                    wet += 1;
                } else if grid.ground_z(qx, qy) == g {
                    flat += 1;
                }
            }
        }
        let near = |p: &Plant, r: i32| (p.pos.x - x).abs() <= r && (p.pos.y - y).abs() <= r;
        let trees = plants
            .iter()
            .filter(|p| p.kind.is_green_tree() && near(p, 12))
            .count();
        let food = plants
            .iter()
            .filter(|p| p.kind.yields_food() && near(p, 12))
            .count();
        // The biome mix within 12 tiles, every second column: the share it has in common
        // with the valley's mix (1 when the surroundings are the valley in miniature).
        let mut local = [0f32; Biome::ALL.len()];
        for dy in (-12..=12).step_by(2) {
            for dx in (-12..=12).step_by(2) {
                let (qx, qy) = (x + dx, y + dy);
                if dx * dx + dy * dy > 144
                    || qx < 0
                    || qy < 0
                    || qx >= grid.width
                    || qy >= grid.height
                {
                    continue;
                }
                let qc = grid.col(qx, qy);
                if !water[qc] {
                    local[grid.biome[qc] as usize] += 1.0;
                }
            }
        }
        let n = local.iter().sum::<f32>().max(1.0);
        let typical: f32 = valley.iter().zip(local).map(|(v, l)| v.min(l / n)).sum();
        // The ground the village itself stands on (within 4 tiles): how common its biomes are
        // in the valley, so the houses sit on the valley's main ground and not at the edge of
        // a rarer patch that happens to border it.
        let (mut core, mut cn) = (0f32, 0f32);
        for dy in -4..=4 {
            for dx in -4..=4 {
                let qc = grid.col(x + dx, y + dy);
                if !water[qc] {
                    core += valley[grid.biome[qc] as usize];
                    cn += 1.0;
                }
            }
        }
        let core = core / cn.max(1.0);
        let cx = (x as f32 / grid.width as f32 - 0.5).abs()
            + (y as f32 / grid.height as f32 - 0.5).abs();
        let score = flat as f32 * 1.0
            + (wet.min(20) as f32) * 2.0
            // Enough wood and forage to start with; more does not make a better site, so a
            // dry valley's settlers are not drawn into its one wood.
            + (trees.min(15) as f32) * 1.5
            + (food.min(12) as f32) * 2.0
            + typical * TYPICAL_WEIGHT
            + core * CORE_WEIGHT
            - cx * 60.0
            - if wet == 0 { 80.0 } else { 0.0 };
        let site = grid.surface(x, y);
        // Settlers avoid bog and frozen ground when the valley offers anything better.
        if matches!(
            grid.biome_at(x, y),
            Some(Biome::Fen | Biome::Fell | Biome::Snowfield)
        ) {
            if score > fallback.0 {
                fallback = (score, site);
            }
        } else if score > best.0 {
            best = (score, site);
        }
    }
    if best.0 == f32::MIN {
        fallback.1
    } else {
        best.1
    }
}

/// How much a founding site's resemblance to the valley's biome mix counts against flatness,
/// water, wood and food (resemblance runs 0..1).
const TYPICAL_WEIGHT: f32 = 150.0;

/// How much the commonness of the village's own ground counts (a valley share, 0..1).
const CORE_WEIGHT: f32 = 100.0;
