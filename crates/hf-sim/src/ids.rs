//! Newtype identifiers. Ids are allocated monotonically by the world and never reused, so a
//! chronicle row written a game-century ago still names the same entity.

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(
            Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize,
        )]
        pub struct $name(pub u32);
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}#{}", stringify!($name), self.0)
            }
        }
    };
}

id_type!(AgentId);
id_type!(ItemId);
id_type!(JobId);
id_type!(CommunityId);
id_type!(StructureId);
id_type!(EventId);

/// A tile coordinate. `z` is signed: 0 is the surface datum, negative is below ground.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct TilePos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl TilePos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
    pub fn offset(self, dx: i32, dy: i32, dz: i32) -> Self {
        Self::new(self.x + dx, self.y + dy, self.z + dz)
    }
    /// Chebyshev distance in the plane plus a vertical penalty.
    pub fn dist(self, o: TilePos) -> i32 {
        let dxy = (self.x - o.x).abs().max((self.y - o.y).abs());
        dxy + (self.z - o.z).abs() * 4
    }
}

impl fmt::Display for TilePos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({},{},{})", self.x, self.y, self.z)
    }
}
