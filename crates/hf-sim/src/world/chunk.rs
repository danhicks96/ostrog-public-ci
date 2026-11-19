//! A 32×32 horizontal slice at one z. Stored as a flat array; dirty bits per tile drive the
//! renderer deltas, and a version counter lets the client detect missed updates.

use serde::{Deserialize, Serialize};

use super::tile::Tile;

pub const CHUNK_SIZE: usize = 32;
pub const CHUNK_AREA: usize = CHUNK_SIZE * CHUNK_SIZE;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chunk {
    pub tiles: Vec<Tile>,
    /// Bumped on every change; transient (renderer bookkeeping), not part of the save.
    #[serde(skip)]
    pub version: u64,
    /// One bit per tile: changed since the last `take_dirty`.
    #[serde(skip)]
    pub dirty: Vec<u64>,
    #[serde(skip)]
    pub any_dirty: bool,
}

impl Chunk {
    pub fn filled(t: Tile) -> Chunk {
        Chunk {
            tiles: vec![t; CHUNK_AREA],
            version: 0,
            dirty: vec![0; CHUNK_AREA / 64],
            any_dirty: false,
        }
    }

    #[inline]
    pub fn idx(lx: usize, ly: usize) -> usize {
        ly * CHUNK_SIZE + lx
    }

    #[inline]
    pub fn get(&self, lx: usize, ly: usize) -> Tile {
        self.tiles[Self::idx(lx, ly)]
    }

    /// Set a tile; marks it dirty and bumps the version if it changed.
    pub fn set(&mut self, lx: usize, ly: usize, t: Tile) -> bool {
        let i = Self::idx(lx, ly);
        if self.tiles[i] == t {
            return false;
        }
        self.tiles[i] = t;
        self.mark_dirty(i);
        true
    }

    pub fn mark_dirty(&mut self, i: usize) {
        if self.dirty.len() != CHUNK_AREA / 64 {
            self.dirty = vec![0; CHUNK_AREA / 64];
        }
        self.dirty[i / 64] |= 1u64 << (i % 64);
        self.any_dirty = true;
        self.version += 1;
    }

    pub fn mark_all_dirty(&mut self) {
        self.dirty = vec![u64::MAX; CHUNK_AREA / 64];
        self.any_dirty = true;
        self.version += 1;
    }

    /// Drain dirty indices in ascending order.
    pub fn take_dirty(&mut self) -> Vec<usize> {
        if !self.any_dirty {
            return Vec::new();
        }
        let mut out = Vec::new();
        for (w, word) in self.dirty.iter_mut().enumerate() {
            let mut bits = *word;
            while bits != 0 {
                let b = bits.trailing_zeros() as usize;
                out.push(w * 64 + b);
                bits &= bits - 1;
            }
            *word = 0;
        }
        self.any_dirty = false;
        out
    }

    pub fn dirty_count(&self) -> usize {
        self.dirty.iter().map(|w| w.count_ones() as usize).sum()
    }
}
