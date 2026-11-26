//! Local river channels anchored to the drainage network of the selected region.
use super::{GenParams, WorldGrid};
use crate::ids::TilePos;
use crate::realm::RealmMap;
use crate::rng::{hash64, purpose};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SpillEdge {
    pub upper: TilePos,
    pub lower: TilePos,
}

impl crate::World {
    /// A stored edge is active while its original water surfaces are liquid and exposed. Standing
    /// lakes, a deep bed and temporary inundation never create a new edge.
    pub fn active_river_spills(&self) -> impl Iterator<Item = SpillEdge> + '_ {
        self.grid.river_spills.iter().copied().filter(|edge| {
            [edge.upper, edge.lower].iter().all(|p| {
                !self.flooded.contains(p)
                    && !crate::water::frozen_at(self, *p)
                    && crate::floods::top_water(&self.grid, p.x, p.y) == Some(*p)
            })
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiverReach {
    level: f32,
    arms: Vec<Crossing>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Crossing {
    direction: (i32, i32),
    along_edge: f32,
    level: f32,
    radius: i32,
}

impl RiverReach {
    pub fn of_region(seed: u64, map: &RealmMap, region: usize) -> Option<Self> {
        let downstream = map.down[region];
        if !map.river[region] || downstream == usize::MAX {
            return None;
        }
        let (cx, cy) = map.xy(region);
        let level = |i: usize| {
            ((map.filled[i] - map.sea_level) / (1.0 - map.sea_level).max(1e-6)).clamp(0.0, 1.0)
        };
        let crossing = |other: usize, upstream: usize, lower: usize| {
            let (x, y) = map.xy(other);
            let shared = hash64(&[seed, purpose::WORLDGEN_WATER, upstream as u64, lower as u64]);
            Crossing {
                direction: (x as i32 - cx as i32, y as i32 - cy as i32),
                along_edge: 0.25 + (shared % 1001) as f32 / 2000.0,
                level: (level(region) + level(other)) * 0.5,
                radius: ((map.flow[upstream].max(1.0) as u32).ilog2() as i32 / 2).clamp(1, 5),
            }
        };
        let mut arms = vec![crossing(downstream, region, downstream)];
        for y in cy.saturating_sub(1)..=(cy + 1).min(map.size - 1) {
            for x in cx.saturating_sub(1)..=(cx + 1).min(map.size - 1) {
                let i = map.idx(x, y);
                if map.river[i] && map.down[i] == region {
                    arms.push(crossing(i, i, region));
                }
            }
        }
        Some(Self {
            level: level(region),
            arms,
        })
    }

    /// Shape the bed before strata and ramps are generated. Water placement consumes
    /// this same footprint. A headwater starts inside the map; tributaries meet there.
    pub fn carve(&self, seed: u64, p: &GenParams, grid: &mut WorldGrid) -> Vec<bool> {
        let mut bed = vec![i32::MAX; grid.ground.len()];
        let mut branch = vec![usize::MAX; bed.len()];
        let mut bank = vec![i32::MIN; grid.ground.len()];
        let s = hash64(&[seed, purpose::WORLDGEN_WATER]);
        let centre = |size: i32, draw: u64| (size - 1) as f32 * (0.38 + (draw % 25) as f32 / 100.0);
        let (cx, cy) = (centre(p.width, s), centre(p.height, s >> 8));
        for (arm, edge) in self.arms.iter().enumerate() {
            let coord = |size: i32, dir: i32| {
                (size - 1) as f32
                    * match dir {
                        -1 => 0.0,
                        1 => 1.0,
                        _ => edge.along_edge,
                    }
            };
            let (ex, ey) = (
                coord(p.width, edge.direction.0),
                coord(p.height, edge.direction.1),
            );
            let (dx, dy) = (ex - cx, ey - cy);
            let length = dx.hypot(dy).max(1.0);
            let steps = (2.0 * length).ceil() as usize;
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                // A local bend dies out at both the confluence and the shared boundary.
                let noise = super::noise::fbm2(s ^ arm as u64, t * 3.0, 0.0, 3, 2.0, 0.5) - 0.5;
                let bend = t * (1.0 - t) * noise * length * 0.75;
                // Approach a shared edge squarely, with a level lip wide enough that
                // the other region's distinct inland bend cannot shift the crossing.
                let lip = ((edge.radius + 2) as f32 / length).min(0.2);
                let interior = ((1.0 - t - lip) / (1.0 - lip)).clamp(0.0, 1.0);
                let mut x = cx + t * dx - bend * dy / length;
                let mut y = cy + t * dy + bend * dx / length;
                if edge.direction.0 == 0 {
                    x = ex - interior * dx - bend * dy / length * interior;
                }
                if edge.direction.1 == 0 {
                    y = ey - interior * dy + bend * dx / length * interior;
                }
                let (x, y) = (x.round() as i32, y.round() as i32);
                let z = ((edge.level + (self.level - edge.level) * interior) * p.relief as f32)
                    .round() as i32;
                let r = edge.radius;
                for yy in (y - r - 1).max(0)..=(y + r + 1).min(p.height - 1) {
                    for xx in (x - r - 1).max(0)..=(x + r + 1).min(p.width - 1) {
                        let d = (xx - x).pow(2) + (yy - y).pow(2);
                        let c = grid.col(xx, yy);
                        if d <= r * r {
                            if z < bed[c] {
                                bed[c] = z;
                                branch[c] = arm;
                            }
                        } else if d <= (r + 1) * (r + 1) {
                            bank[c] = bank[c].max(z);
                        }
                    }
                }
            }
        }
        for (i, g) in grid.ground.iter_mut().enumerate() {
            *g = if bed[i] != i32::MAX {
                bed[i]
            } else {
                (*g).max(bank[i])
            };
        }
        grid.river_spills.clear();
        for y in 0..p.height {
            for x in 0..p.width {
                let c = grid.col(x, y);
                if branch[c] == usize::MAX {
                    continue;
                }
                for (xx, yy) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                    if xx < 0 || yy < 0 || xx >= p.width || yy >= p.height {
                        continue;
                    }
                    let next = grid.col(xx, yy);
                    // Grades along each arm are monotone. Water from a tributary can
                    // enter the outlet arm, but cannot cut sideways into a sibling.
                    if bed[next] < bed[c] && (branch[next] == branch[c] || branch[next] == 0) {
                        grid.river_spills.push(SpillEdge {
                            upper: TilePos::new(x, y, bed[c] + 1),
                            lower: TilePos::new(xx, yy, bed[next] + 1),
                        });
                    }
                }
            }
        }
        bed.into_iter().map(|z| z != i32::MAX).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[test]
    fn both_tributaries_can_reach_a_lower_outlet_without_climbing() {
        let p = GenParams::small();
        let mut grid = WorldGrid::new(p.width, p.height, p.z_min, p.z_max);
        super::super::heightmap::apply(17, &p, &mut grid);
        let arm = |direction, level| Crossing {
            direction,
            level,
            along_edge: 0.5,
            radius: 2,
        };
        let reach = RiverReach {
            level: 0.5,
            arms: vec![arm((1, 0), 0.0), arm((-1, 0), 1.0), arm((0, -1), 1.0)],
        };
        let wet = reach.carve(17, &p, &mut grid);
        let mut seen = vec![false; wet.len()];
        let outlet = (p.width - 1, p.height / 2);
        let mut queue = VecDeque::from([outlet]);
        seen[grid.col(outlet.0, outlet.1)] = true;
        while let Some((x, y)) = queue.pop_front() {
            for (xx, yy) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                if xx < 0 || yy < 0 || xx >= p.width || yy >= p.height {
                    continue;
                }
                let c = grid.col(xx, yy);
                if wet[c] && !seen[c] && grid.ground_z(xx, yy) >= grid.ground_z(x, y) {
                    seen[c] = true;
                    queue.push_back((xx, yy));
                }
            }
        }
        for (x, y) in [(0, p.height / 2), (p.width / 2, 0)] {
            assert!(seen[grid.col(x, y)]);
            assert_eq!(grid.ground_z(x, y), p.relief);
        }
        assert_eq!(grid.ground_z(outlet.0, outlet.1), 0);
    }
}
