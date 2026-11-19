//! The world grid: chunk map plus per-column ground heights.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::chunk::{Chunk, CHUNK_SIZE};
use super::material::{Material, MaterialTable};
use super::tile::{Shape, Tile, TileFlags};
use crate::ids::TilePos;

pub type ChunkKey = (i32, i32, i32);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldGrid {
    pub width: i32,
    pub height: i32,
    pub z_min: i32,
    pub z_max: i32,
    /// Ground z per column (the highest solid tile). Surface tiles sit at ground_z + 1.
    pub ground: Vec<i32>,
    /// Material of the undisturbed rock column below the soil, per column (for synthesis):
    /// the upper stratum where `strata` is set, the whole column in older saves.
    pub bedrock: Vec<Material>,
    /// The rock strata per column (world/gen/strata.rs): the middle stratum's rock and the z
    /// of the top of the middle and of the deep (basalt) strata. Empty in older saves.
    #[serde(default)]
    pub strata: Vec<(Material, i8, i8)>,
    /// #267: the deep stratum's rock per column (basalt, gabbro or gneiss by region) and the z
    /// of a coal seam in the upper stratum (`i8::MIN` for none). Empty in older saves, where the
    /// deep is basalt and there are no seams.
    #[serde(default)]
    pub deep: Vec<(Material, i8)>,
    /// #267: sub-strata within the middle stratum, one small list per column, each entry a
    /// band's rock and the z of that band's top, deepest first, every top lying between the
    /// deep break and the middle break. The shallowest middle band keeps `strata.0`'s rock, so
    /// a dug shaft reads an alternation (granite over slate over granite, or the other way)
    /// before the deep. Empty in older saves, which have a single middle rock.
    #[serde(default)]
    pub bands: Vec<Vec<(Material, i8)>>,
    /// #267: levels of soil per column, the surface tile among them (world/gen/strata.rs):
    /// deeper on low, level ground, one on slopes and heights. Empty in older saves (one).
    #[serde(default)]
    pub soil: Vec<u8>,
    /// The great chambers of the middle stratum (world/gen/caverns.rs).
    #[serde(default)]
    pub caverns: Vec<crate::world::gen::caverns::Cavern>,
    pub chunks: BTreeMap<ChunkKey, Chunk>,
    pub materials: MaterialTable,
    /// Biome per column, as chosen at generation (world/gen/biome.rs). Empty in saves from
    /// before biomes existed and in hand-built grids.
    #[serde(default)]
    pub biome: Vec<crate::world::gen::biome::Biome>,
}

#[cfg(test)]
thread_local! {
    static WALKABLE_READS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

impl WorldGrid {
    pub fn new(width: i32, height: i32, z_min: i32, z_max: i32) -> Self {
        assert!(width > 0 && height > 0 && z_min <= 0 && z_max >= 0);
        WorldGrid {
            width,
            height,
            z_min,
            z_max,
            ground: vec![0; (width * height) as usize],
            bedrock: vec![Material::GRANITE; (width * height) as usize],
            strata: Vec::new(),
            deep: Vec::new(),
            bands: Vec::new(),
            soil: Vec::new(),
            caverns: Vec::new(),
            chunks: BTreeMap::new(),
            materials: MaterialTable::default(),
            biome: Vec::new(),
        }
    }

    #[inline]
    pub fn in_bounds(&self, p: TilePos) -> bool {
        p.x >= 0
            && p.y >= 0
            && p.x < self.width
            && p.y < self.height
            && p.z >= self.z_min
            && p.z <= self.z_max
    }

    #[inline]
    pub fn col(&self, x: i32, y: i32) -> usize {
        (y * self.width + x) as usize
    }

    /// The biome of a column, if the grid was generated with biomes.
    pub fn biome_at(&self, x: i32, y: i32) -> Option<crate::world::gen::biome::Biome> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        self.biome.get(self.col(x, y)).copied()
    }

    /// The ground's level in a column; a column off the map is read as the nearest one on it
    /// (#177: neighbours of a tile at the edge read another row, or past the end).
    pub fn ground_z(&self, x: i32, y: i32) -> i32 {
        let (x, y) = (x.clamp(0, self.width - 1), y.clamp(0, self.height - 1));
        self.ground[self.col(x, y)]
    }

    /// The tile an inhabitant stands on at the surface of this column.
    pub fn surface(&self, x: i32, y: i32) -> TilePos {
        TilePos::new(x, y, self.ground_z(x, y) + 1)
    }

    #[inline]
    pub fn key(p: TilePos) -> (ChunkKey, usize, usize) {
        let cs = CHUNK_SIZE as i32;
        let cx = p.x.div_euclid(cs);
        let cy = p.y.div_euclid(cs);
        (
            (cx, cy, p.z),
            p.x.rem_euclid(cs) as usize,
            p.y.rem_euclid(cs) as usize,
        )
    }

    /// What an unallocated tile is: rock at or below ground, air above.
    pub fn synthesized(&self, p: TilePos) -> Tile {
        if !self.in_bounds(p) {
            return Tile::solid(Material::GRANITE);
        }
        let c = self.col(p.x, p.y);
        let g = self.ground[c];
        if p.z <= g {
            // The soil (#267: one to three levels by the lie of the land), below it the
            // column's rock at that depth.
            let soil = self.soil_depth(c);
            let m = if p.z > g - soil {
                Material::SOIL
            } else {
                self.rock_at(c, p.z)
            };
            Tile::solid(m)
        } else {
            Tile::AIR
        }
    }

    /// Levels of soil in column `c`, the surface tile among them.
    pub fn soil_depth(&self, c: usize) -> i32 {
        self.soil.get(c).copied().unwrap_or(1).max(1) as i32
    }

    /// The undisturbed rock of column `c` at height `z`: the upper stratum (with its seam),
    /// then the middle stratum, split into its sub-bands (`bands`) where a world has them,
    /// then the deep at the bottom.
    pub fn rock_at(&self, c: usize, z: i32) -> Material {
        let (deep_rock, seam) = self
            .deep
            .get(c)
            .copied()
            .unwrap_or((Material::BASALT, i8::MIN));
        let Some(&(first, top, deep)) = self.strata.get(c) else {
            // A world from before the strata: one rock, with the old seeded coal patches.
            return if z == seam as i32 {
                Material::COAL
            } else {
                self.bedrock[c]
            };
        };
        if z <= deep as i32 {
            return deep_rock;
        }
        if z <= top as i32 {
            // The middle stratum: the shallowest band's rock, or a deeper sub-band's where
            // the shaft has crossed its break (#267).
            return self
                .bands
                .get(c)
                .and_then(|bs| bs.iter().find(|&&(_, btop)| z <= btop as i32))
                .map(|&(rock, _)| rock)
                .unwrap_or(first);
        }
        if z == seam as i32 {
            return Material::COAL;
        }
        self.bedrock[c]
    }

    pub fn get(&self, p: TilePos) -> Tile {
        let (k, lx, ly) = Self::key(p);
        match self.chunks.get(&k) {
            Some(c) => c.get(lx, ly),
            None => self.synthesized(p),
        }
    }

    /// Allocate the chunk holding `p` (filled from synthesis) if it does not exist.
    pub fn chunk_mut(&mut self, p: TilePos) -> &mut Chunk {
        let (k, _, _) = Self::key(p);
        if !self.chunks.contains_key(&k) {
            let cs = CHUNK_SIZE as i32;
            let mut c = Chunk::filled(Tile::AIR);
            for ly in 0..CHUNK_SIZE {
                for lx in 0..CHUNK_SIZE {
                    let q = TilePos::new(k.0 * cs + lx as i32, k.1 * cs + ly as i32, k.2);
                    c.tiles[Chunk::idx(lx, ly)] = self.synthesized(q);
                }
            }
            c.mark_all_dirty();
            self.chunks.insert(k, c);
        }
        self.chunks.get_mut(&k).expect("chunk allocated")
    }

    /// Set a tile. Returns true if it changed. Keeps `ground` consistent for solid changes.
    pub fn set(&mut self, p: TilePos, t: Tile) -> bool {
        if !self.in_bounds(p) {
            return false;
        }
        let (_, lx, ly) = Self::key(p);
        let changed = self.chunk_mut(p).set(lx, ly, t);
        if changed {
            self.refresh_ground(p.x, p.y);
        }
        changed
    }

    fn refresh_ground(&mut self, x: i32, y: i32) {
        // Ground = highest z whose tile is Wall (solid). Scan from z_max down.
        let mut z = self.z_max;
        while z >= self.z_min {
            if self.get(TilePos::new(x, y, z)).shape == Shape::Wall {
                break;
            }
            z -= 1;
        }
        let c = self.col(x, y);
        self.ground[c] = z.max(self.z_min);
    }

    /// Can something stand here? Needs a non-solid tile that is self-supporting or has
    /// support below (solid, floor, or ramp top).
    pub fn walkable(&self, p: TilePos) -> bool {
        #[cfg(test)]
        WALKABLE_READS.with(|reads| reads.set(reads.get() + 1));
        if !self.in_bounds(p) {
            return false;
        }
        let t = self.get(p);
        if t.is_solid() || t.flags.has(TileFlags::DEEP) || t.flags.has(TileFlags::FIRE) {
            return false;
        }
        if t.shape.self_supporting() || t.flags.has(TileFlags::DOOR) {
            return true;
        }
        let below = self.get(p.offset(0, 0, -1));
        below.shape == Shape::Wall || below.shape == Shape::Floor
    }

    /// Water over a person's head here (#390): the tile is full to its roof, or there is water
    /// on it and on the one above. Water standing shallow in a working (#272) is not.
    pub fn over_head(&self, p: TilePos) -> bool {
        let t = self.get(p);
        t.flags.has(TileFlags::FULL) || (t.is_water() && self.get(p.offset(0, 0, 1)).is_water())
    }

    pub fn chunk_keys(&self) -> impl Iterator<Item = &ChunkKey> {
        self.chunks.keys()
    }
}

#[cfg(test)]
pub(crate) fn reset_walkable_reads() {
    WALKABLE_READS.with(|reads| reads.set(0));
}

#[cfg(test)]
pub(crate) fn walkable_reads() -> usize {
    WALKABLE_READS.with(std::cell::Cell::get)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthesized_column_and_walkability() {
        let mut g = WorldGrid::new(64, 64, -8, 8);
        let c = g.col(3, 3);
        g.ground[c] = 2;
        assert!(g.get(TilePos::new(3, 3, 2)).is_solid());
        assert!(!g.get(TilePos::new(3, 3, 3)).is_solid());
        assert!(g.walkable(TilePos::new(3, 3, 3)));
        assert!(!g.walkable(TilePos::new(3, 3, 4)));
        // Dig the top: ground moves down, the tile above is no longer walkable (air over air)
        // until something supports it.
        assert!(g.set(TilePos::new(3, 3, 2), Tile::AIR));
        assert_eq!(g.ground_z(3, 3), 1);
        assert!(g.walkable(TilePos::new(3, 3, 2)));
        assert!(!g.walkable(TilePos::new(3, 3, 3)));
    }
}
