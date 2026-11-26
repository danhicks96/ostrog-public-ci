//! The world the valley is part of (M3, docs/design-world.md). C24: the region map.

pub mod beasts;
pub mod chronicle;
pub mod culture;
pub mod embark;
pub mod figures;
pub mod language;
pub mod live;
pub mod map;
pub mod peoples;
pub mod relations;
pub mod sites;
pub mod stories;
pub mod terrain;
pub mod war;

pub use map::{generate, RBiome, RealmMap, RealmParams};
