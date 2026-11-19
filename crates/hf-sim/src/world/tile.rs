//! One tile. Packed to 32 bits for the delta protocol: material | shape << 16 | flags << 24.

use serde::{Deserialize, Serialize};

use super::material::Material;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum Shape {
    /// Air: walkable only if there is a floor or ground beneath (i.e. the tile below is solid
    /// or a Floor). Open sky above ground.
    #[default]
    Open = 0,
    /// A constructed or natural floor: walkable, blocks vertical movement.
    Floor = 1,
    /// Solid: rock, soil, built wall. Blocks movement and sight.
    Wall = 2,
    /// Ramp rising toward +z at an adjacent tile. Walkable.
    RampUp = 3,
    /// Top of a ramp: an Open tile that a ramp arrives at. Walkable.
    RampTop = 4,
    StairsUp = 5,
    StairsDown = 6,
    StairsBoth = 7,
    Ladder = 8,
}

impl Shape {
    pub fn from_u8(v: u8) -> Shape {
        match v {
            1 => Shape::Floor,
            2 => Shape::Wall,
            3 => Shape::RampUp,
            4 => Shape::RampTop,
            5 => Shape::StairsUp,
            6 => Shape::StairsDown,
            7 => Shape::StairsBoth,
            8 => Shape::Ladder,
            _ => Shape::Open,
        }
    }
    pub fn is_solid(self) -> bool {
        matches!(self, Shape::Wall)
    }
    pub fn connects_up(self) -> bool {
        matches!(self, Shape::StairsUp | Shape::StairsBoth | Shape::Ladder)
    }
    pub fn connects_down(self) -> bool {
        matches!(self, Shape::StairsDown | Shape::StairsBoth | Shape::Ladder)
    }
    /// A tile whose own shape supports standing (regardless of what is below).
    pub fn self_supporting(self) -> bool {
        !matches!(self, Shape::Open | Shape::Wall)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub struct TileFlags(pub u8);

impl TileFlags {
    pub const NONE: TileFlags = TileFlags(0);
    pub const WATER: u8 = 1 << 0; // shallow water on this tile
    pub const DEEP: u8 = 1 << 1; // deep water (impassable without a boat)
    pub const SKY: u8 = 1 << 2; // open to the sky (computed by light)
    pub const DOOR: u8 = 1 << 3; // a door in this wall tile (passable, blocks sight)
    pub const DESIGNATED: u8 = 1 << 4; // a work designation exists here (for rendering)
    pub const SNOW: u8 = 1 << 5;
    pub const FIRE: u8 = 1 << 6; // burning (nobody walks into it)
    pub const FULL: u8 = 1 << 7; // water to the roof: no air to breathe (#390)

    pub fn has(self, f: u8) -> bool {
        self.0 & f != 0
    }
    pub fn with(self, f: u8) -> TileFlags {
        TileFlags(self.0 | f)
    }
    pub fn without(self, f: u8) -> TileFlags {
        TileFlags(self.0 & !f)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub struct Tile {
    pub material: Material,
    pub shape: Shape,
    pub flags: TileFlags,
}

impl Tile {
    pub const AIR: Tile = Tile {
        material: Material::AIR,
        shape: Shape::Open,
        flags: TileFlags(TileFlags::SKY),
    };
    pub const fn solid(material: Material) -> Tile {
        Tile {
            material,
            shape: Shape::Wall,
            flags: TileFlags(0),
        }
    }
    pub const fn floor(material: Material) -> Tile {
        Tile {
            material,
            shape: Shape::Floor,
            flags: TileFlags(0),
        }
    }
    pub fn pack(self) -> u32 {
        (self.material.0 as u32) | ((self.shape as u32) << 16) | ((self.flags.0 as u32) << 24)
    }
    pub fn unpack(v: u32) -> Tile {
        Tile {
            material: Material((v & 0xffff) as u16),
            shape: Shape::from_u8(((v >> 16) & 0xff) as u8),
            flags: TileFlags(((v >> 24) & 0xff) as u8),
        }
    }
    pub fn is_solid(self) -> bool {
        self.shape.is_solid() && !self.flags.has(TileFlags::DOOR)
    }
    pub fn is_water(self) -> bool {
        self.flags.has(TileFlags::WATER | TileFlags::DEEP)
    }
    pub fn blocks_sight(self) -> bool {
        self.shape.is_solid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pack_roundtrip() {
        let t = Tile {
            material: Material(513),
            shape: Shape::StairsBoth,
            flags: TileFlags(TileFlags::WATER | TileFlags::DOOR),
        };
        assert_eq!(Tile::unpack(t.pack()), t);
    }
}
