//! The spatial world: a 3D tile grid stored as 32×32×1 chunks keyed by (cx, cy, z).
//!
//! Vertical rules follow Dwarf Fortress: stairs and ladders connect the same (x, y) across
//! z; a ramp connects to an adjacent (x, y) one level up. Absent chunks are synthesized on
//! read (solid rock below the column ground, air above) and allocated only on write.

pub mod chunk;
pub mod gen;
pub mod grid;
pub mod material;
pub mod tile;

pub use chunk::{Chunk, CHUNK_AREA, CHUNK_SIZE};
pub use grid::WorldGrid;
pub use material::{Material, MaterialDef, MaterialTable};
pub use tile::{Shape, Tile, TileFlags};
