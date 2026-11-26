//! Built things: walls, doors, floors are tiles (see world::tile); furniture, workstations
//! and lights are `Structure` entities occupying one tile at a z-level.

use serde::{Deserialize, Serialize};

use crate::ids::{StructureId, TilePos};
use crate::world::Material;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum StructureKind {
    Bed,
    Table,
    Chair,
    Stool,
    Chest,
    Shelf,
    Hearth,
    Workbench,
    Loom,
    Anvil,
    CookingPot,
    Barrel,
    Crate,
    Well,
    FencePost,
    Lamp,
    Stockpile, // zone marker, one per tile
    CarpenterBench,
    Smelter,
    Furnace,
    Kiln,
    /// A grave marker from the valley's past (deep.rs).
    Grave,
    /// A timber prop holding up the roof of a dug gallery (underground.rs).
    Support,
    /// The stone where the ten came through (thin.rs).
    PortalStone,
    /// A standing stone set to quiet the thin place.
    WardStone,
    // Appended (C57): homes of the kinds that do not build houses.
    /// A hide-and-pole shelter for one sleeper (the Gnarrow's way), quick to raise and strike.
    Tent,
    /// A raised woven platform for one sleeper (the Skarri's way).
    Nest,
    /// Appended (#297): a community's shrine, where its observances are kept (shrines.rs).
    Shrine,
    /// Appended (#1081): a one-tile hide-and-pole shelter a person raises alone, the same
    /// day, from what lies about (D35, D36). Its own tile counts as roofed.
    LeanTo,
}

impl StructureKind {
    pub fn label(self) -> String {
        match self {
            Self::CookingPot => "cooking pot".into(),
            Self::CarpenterBench => "carpenter's bench".into(),
            Self::FencePost => "fence post".into(),
            Self::PortalStone => "portal stone".into(),
            Self::WardStone => "ward stone".into(),
            Self::LeanTo => "lean-to".into(),
            other => format!("{other:?}").to_lowercase(),
        }
    }

    /// Where one person sleeps: a bed, a tent, a nest, a lean-to (C57, #1081).
    pub fn is_sleeping_place(self) -> bool {
        matches!(
            self,
            StructureKind::Bed | StructureKind::Tent | StructureKind::Nest | StructureKind::LeanTo
        )
    }

    /// Furniture a person in a rage can smash. Not stone, not graves, not props holding a roof
    /// up, not the portal stone or the wards about it, not the hearth or the workshops.
    pub fn breakable_in_anger(self) -> bool {
        matches!(
            self,
            StructureKind::Bed
                | StructureKind::Table
                | StructureKind::Chair
                | StructureKind::Stool
                | StructureKind::Chest
                | StructureKind::Shelf
                | StructureKind::Barrel
                | StructureKind::Crate
                | StructureKind::CookingPot
                | StructureKind::Lamp
        )
    }

    pub fn blocks_movement(self) -> bool {
        matches!(
            self,
            StructureKind::Table
                | StructureKind::Chest
                | StructureKind::Shelf
                | StructureKind::Hearth
                | StructureKind::Workbench
                | StructureKind::Loom
                | StructureKind::Anvil
                | StructureKind::Barrel
                | StructureKind::Crate
                | StructureKind::Well
                | StructureKind::PortalStone
                | StructureKind::WardStone
                | StructureKind::FencePost
                | StructureKind::CarpenterBench
                | StructureKind::Smelter
                | StructureKind::Furnace
                | StructureKind::Kiln
                | StructureKind::Shrine
        )
    }
    pub fn emits_light(self) -> bool {
        matches!(
            self,
            StructureKind::Hearth | StructureKind::Lamp | StructureKind::Smelter
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Structure {
    pub id: StructureId,
    pub kind: StructureKind,
    pub pos: TilePos,
    pub material: Material,
    /// 0..1 construction progress; 1 = complete.
    pub progress: f32,
    /// Hearth lit, lamp lit, pot cooking, etc.
    pub active: bool,
    /// 0..1 wear.
    pub condition: f32,
    /// Whose it is, if anyone's (#1089, #1791): set directly today (no claiming yet, #1090).
    #[serde(default)]
    pub owner: Option<crate::ids::AgentId>,
    /// How well it was made (#1020): a set fitting keeps its item's quality (D57). 0.5 for kinds
    /// built from raw planks or blocks, and for a structure from before this field.
    #[serde(default = "half")]
    pub quality: f32,
}

fn half() -> f32 {
    0.5
}

/// What a thing standing here is made of, for the hover line, when it is made of something in
/// particular (C32, C33): "A table of oak", "A smelter of granite". Things of plain timber or
/// block from before materials say nothing.
pub fn note(w: &crate::world_state::World, p: TilePos) -> Option<String> {
    use crate::world::material::MaterialKind as MK;
    let s = w
        .structures
        .iter()
        .find(|s| s.pos == p && s.progress >= 1.0)?;
    let def = w.grid.materials.get(s.material);
    if !matches!(def.kind, MK::Wood | MK::Stone | MK::Metal) {
        return None;
    }
    let what = format!("{:?}", s.kind);
    let mut words = String::new();
    for (i, c) in what.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            words.push(' ');
        }
        words.push(c.to_ascii_lowercase());
    }
    Some(format!("A {words} of {}", def.name))
}
