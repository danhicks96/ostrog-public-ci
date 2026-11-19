//! Recipes: what a workshop turns into what. Data, not code paths; the planner posts
//! `DesignationKind::Craft(id)` from need and the work system applies the recipe.

use crate::items::ItemKind;
use crate::mind::Skill;
use crate::structures::StructureKind;
use crate::world::Material as M;

#[derive(Clone, Debug)]
pub struct Recipe {
    pub id: u8,
    pub label: &'static str,
    pub station: StructureKind,
    pub inputs: &'static [(ItemKind, u32)],
    /// The material each input must be, by position (C33): tin ore, not any ore. Missing or
    /// None: any material of that kind will do.
    pub wants: &'static [Option<M>],
    pub output: (ItemKind, u32),
    /// What the output is made of.
    pub makes: Makes,
    pub skill: Skill,
    pub minutes: u32,
}

/// What a recipe's output is made of (C32, C33).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Makes {
    /// The first input's material, worked (an ore becomes its metal); the given material when
    /// the input carried none (things from before C32, or brought through the portal).
    FromInput(M),
    /// Always this material: an alloy is neither of the metals that went into it.
    Fixed(M),
}

impl Recipe {
    /// The material input `ix` must be, if any.
    pub fn want(&self, ix: usize) -> Option<M> {
        self.wants.get(ix).copied().flatten()
    }
}

pub const PLANK: u8 = 0;
pub const COPPER_BAR: u8 = 1;
pub const IRON_BAR: u8 = 2;
pub const IRON_TOOL: u8 = 3;
pub const COPPER_TOOL: u8 = 4;
pub const CLOTH: u8 = 5;
pub const CLOTHES: u8 = 6;
pub const POULTICE: u8 = 7;
pub const STONE_TOOL: u8 = 8;
// C33: the rest of the metals, bronze, weapons and armour.
pub const TIN_BAR: u8 = 9;
pub const SILVER_BAR: u8 = 10;
pub const GOLD_BAR: u8 = 11;
pub const BRONZE: u8 = 12;
pub const IRON_WEAPON: u8 = 13;
pub const BRONZE_WEAPON: u8 = 14;
pub const COPPER_WEAPON: u8 = 15;
pub const IRON_ARMOUR: u8 = 16;
pub const BRONZE_ARMOUR: u8 = 17;
pub const BRONZE_TOOL: u8 = 18;
// C34: the kiln, the tanner, the brewer and the jeweller.
pub const POT: u8 = 19;
pub const GLASS: u8 = 20;
pub const LEATHER: u8 = 21;
pub const LEATHER_CLOTHES: u8 = 22;
pub const ALE: u8 = 23;
pub const WINE: u8 = 24;
pub const SILVER_RING: u8 = 25;
pub const GOLD_RING: u8 = 26;
// C35: the pieces made in a fey mood (systems/moods.rs); never posted by the planner.
pub const FEY_FIGURE: u8 = 27;
pub const FEY_CARVING: u8 = 28;
pub const FEY_BLADE: u8 = 29;
pub const FEY_IRON_BLADE: u8 = 30;
// C52: a spear anyone can make at a bench, for a community the strangers have raided.
pub const WOOD_SPEAR: u8 = 31;
// C67: the building chain (docs/design-rules.md section 2).
pub const DRESS_STONE: u8 = 32;
pub const DOOR: u8 = 33;
pub const BED_FRAME: u8 = 34;
pub const TABLE: u8 = 35;
pub const STOOL: u8 = 36;
pub const CHEST: u8 = 37;
pub const THATCH: u8 = 38;
// #286: clothes dyed at the loom with a dye plant; they take the dye's colour.
pub const DYED_CLOTHES: u8 = 39;

pub const RECIPES: &[Recipe] = &[
    Recipe {
        id: PLANK,
        label: "planks",
        station: StructureKind::CarpenterBench,
        inputs: &[(ItemKind::Log, 1)],
        wants: &[],
        output: (ItemKind::Plank, 2),
        makes: Makes::FromInput(M::WOOD),
        skill: Skill::Crafting,
        minutes: 20,
    },
    Recipe {
        id: COPPER_BAR,
        label: "a copper bar",
        station: StructureKind::Smelter,
        inputs: &[(ItemKind::CopperOre, 2), (ItemKind::Coal, 1)],
        wants: &[],
        output: (ItemKind::CopperBar, 1),
        makes: Makes::FromInput(M::COPPER),
        skill: Skill::Crafting,
        minutes: 40,
    },
    Recipe {
        id: IRON_BAR,
        label: "an iron bar",
        station: StructureKind::Smelter,
        inputs: &[(ItemKind::IronOre, 2), (ItemKind::Coal, 1)],
        wants: &[],
        output: (ItemKind::IronBar, 1),
        makes: Makes::FromInput(M::IRON),
        skill: Skill::Crafting,
        minutes: 50,
    },
    Recipe {
        id: IRON_TOOL,
        label: "an iron tool",
        station: StructureKind::Anvil,
        inputs: &[(ItemKind::IronBar, 1), (ItemKind::Plank, 1)],
        wants: &[],
        output: (ItemKind::Tool, 1),
        makes: Makes::FromInput(M::IRON),
        skill: Skill::Crafting,
        minutes: 60,
    },
    Recipe {
        id: COPPER_TOOL,
        label: "a copper tool",
        station: StructureKind::Anvil,
        inputs: &[(ItemKind::CopperBar, 1), (ItemKind::Plank, 1)],
        wants: &[],
        output: (ItemKind::Tool, 1),
        makes: Makes::FromInput(M::COPPER),
        skill: Skill::Crafting,
        minutes: 45,
    },
    Recipe {
        id: CLOTH,
        label: "cloth",
        station: StructureKind::Loom,
        inputs: &[(ItemKind::Fibre, 3)],
        wants: &[],
        output: (ItemKind::Cloth, 1),
        makes: Makes::FromInput(M::AIR),
        skill: Skill::Crafting,
        minutes: 30,
    },
    Recipe {
        id: CLOTHES,
        label: "a set of clothes",
        station: StructureKind::Loom,
        inputs: &[(ItemKind::Cloth, 2)],
        wants: &[],
        output: (ItemKind::Clothes, 1),
        makes: Makes::FromInput(M::AIR),
        skill: Skill::Crafting,
        minutes: 40,
    },
    Recipe {
        id: POULTICE,
        label: "a poultice",
        station: StructureKind::Hearth,
        inputs: &[(ItemKind::Root, 2)],
        wants: &[],
        output: (ItemKind::Poultice, 1),
        makes: Makes::FromInput(M::AIR),
        skill: Skill::Cooking,
        minutes: 15,
    },
    Recipe {
        id: STONE_TOOL,
        label: "a stone tool",
        station: StructureKind::Workbench,
        inputs: &[(ItemKind::RoughStone, 1), (ItemKind::Log, 1)],
        wants: &[],
        output: (ItemKind::Tool, 1),
        makes: Makes::FromInput(M::GRANITE),
        skill: Skill::Crafting,
        minutes: 40,
    },
    Recipe {
        id: TIN_BAR,
        label: "a tin bar",
        station: StructureKind::Smelter,
        inputs: &[(ItemKind::Ore, 2), (ItemKind::Coal, 1)],
        wants: &[Some(M::TIN_ORE)],
        output: (ItemKind::Bar, 1),
        makes: Makes::FromInput(M::TIN),
        skill: Skill::Crafting,
        minutes: 30,
    },
    Recipe {
        id: SILVER_BAR,
        label: "a silver bar",
        station: StructureKind::Smelter,
        inputs: &[(ItemKind::Ore, 2), (ItemKind::Coal, 1)],
        wants: &[Some(M::SILVER_ORE)],
        output: (ItemKind::Bar, 1),
        makes: Makes::FromInput(M::SILVER),
        skill: Skill::Crafting,
        minutes: 40,
    },
    Recipe {
        id: GOLD_BAR,
        label: "a gold bar",
        station: StructureKind::Smelter,
        inputs: &[(ItemKind::Ore, 2), (ItemKind::Coal, 1)],
        wants: &[Some(M::GOLD_ORE)],
        output: (ItemKind::Bar, 1),
        makes: Makes::FromInput(M::GOLD),
        skill: Skill::Crafting,
        minutes: 40,
    },
    Recipe {
        id: BRONZE,
        label: "bronze",
        station: StructureKind::Smelter,
        inputs: &[
            (ItemKind::CopperBar, 2),
            (ItemKind::Bar, 1),
            (ItemKind::Coal, 1),
        ],
        wants: &[None, Some(M::TIN)],
        output: (ItemKind::Bar, 3),
        makes: Makes::Fixed(M::BRONZE),
        skill: Skill::Crafting,
        minutes: 45,
    },
    Recipe {
        id: IRON_WEAPON,
        label: "an iron spear",
        station: StructureKind::Anvil,
        inputs: &[(ItemKind::IronBar, 1), (ItemKind::Plank, 1)],
        wants: &[],
        output: (ItemKind::Weapon, 1),
        makes: Makes::FromInput(M::IRON),
        skill: Skill::Crafting,
        minutes: 60,
    },
    Recipe {
        id: BRONZE_WEAPON,
        label: "a bronze spear",
        station: StructureKind::Anvil,
        inputs: &[(ItemKind::Bar, 1), (ItemKind::Plank, 1)],
        wants: &[Some(M::BRONZE)],
        output: (ItemKind::Weapon, 1),
        makes: Makes::FromInput(M::BRONZE),
        skill: Skill::Crafting,
        minutes: 55,
    },
    Recipe {
        id: COPPER_WEAPON,
        label: "a copper spear",
        station: StructureKind::Anvil,
        inputs: &[(ItemKind::CopperBar, 1), (ItemKind::Plank, 1)],
        wants: &[],
        output: (ItemKind::Weapon, 1),
        makes: Makes::FromInput(M::COPPER),
        skill: Skill::Crafting,
        minutes: 45,
    },
    Recipe {
        id: IRON_ARMOUR,
        label: "a coat of iron mail",
        station: StructureKind::Anvil,
        inputs: &[(ItemKind::IronBar, 2), (ItemKind::Cloth, 1)],
        wants: &[],
        output: (ItemKind::Armour, 1),
        makes: Makes::FromInput(M::IRON),
        skill: Skill::Crafting,
        minutes: 120,
    },
    Recipe {
        id: BRONZE_ARMOUR,
        label: "a bronze scale coat",
        station: StructureKind::Anvil,
        inputs: &[(ItemKind::Bar, 2), (ItemKind::Cloth, 1)],
        wants: &[Some(M::BRONZE)],
        output: (ItemKind::Armour, 1),
        makes: Makes::FromInput(M::BRONZE),
        skill: Skill::Crafting,
        minutes: 110,
    },
    Recipe {
        id: BRONZE_TOOL,
        label: "a bronze tool",
        station: StructureKind::Anvil,
        inputs: &[(ItemKind::Bar, 1), (ItemKind::Plank, 1)],
        wants: &[Some(M::BRONZE)],
        output: (ItemKind::Tool, 1),
        makes: Makes::FromInput(M::BRONZE),
        skill: Skill::Crafting,
        minutes: 50,
    },
    Recipe {
        id: POT,
        label: "a clay pot",
        station: StructureKind::Kiln,
        inputs: &[(ItemKind::Clay, 2), (ItemKind::Log, 1)],
        wants: &[],
        output: (ItemKind::Vessel, 1),
        makes: Makes::Fixed(M::CLAY),
        skill: Skill::Crafting,
        minutes: 60,
    },
    Recipe {
        id: GLASS,
        label: "a glass bottle",
        station: StructureKind::Kiln,
        inputs: &[(ItemKind::Sand, 3), (ItemKind::Coal, 1)],
        wants: &[],
        output: (ItemKind::Vessel, 1),
        makes: Makes::Fixed(M::GLASS),
        skill: Skill::Crafting,
        minutes: 90,
    },
    Recipe {
        id: LEATHER,
        label: "leather",
        station: StructureKind::Workbench,
        inputs: &[(ItemKind::Hide, 1)],
        wants: &[],
        output: (ItemKind::Leather, 1),
        makes: Makes::FromInput(M::DEER_HIDE),
        skill: Skill::Crafting,
        minutes: 90,
    },
    Recipe {
        id: LEATHER_CLOTHES,
        label: "leather clothes",
        station: StructureKind::Workbench,
        inputs: &[(ItemKind::Leather, 2)],
        wants: &[],
        output: (ItemKind::Clothes, 1),
        makes: Makes::FromInput(M::DEER_HIDE),
        skill: Skill::Crafting,
        minutes: 60,
    },
    Recipe {
        id: ALE,
        label: "ale",
        station: StructureKind::Hearth,
        inputs: &[(ItemKind::GrainSheaf, 4)],
        wants: &[],
        output: (ItemKind::Ale, 3),
        makes: Makes::Fixed(M::AIR),
        skill: Skill::Cooking,
        minutes: 120,
    },
    Recipe {
        id: WINE,
        label: "berry wine",
        station: StructureKind::Hearth,
        inputs: &[(ItemKind::Berries, 5)],
        wants: &[],
        output: (ItemKind::Wine, 2),
        makes: Makes::Fixed(M::AIR),
        skill: Skill::Cooking,
        minutes: 120,
    },
    Recipe {
        id: SILVER_RING,
        label: "a silver ring",
        station: StructureKind::Anvil,
        inputs: &[(ItemKind::Bar, 1), (ItemKind::Gem, 1)],
        wants: &[Some(M::SILVER)],
        output: (ItemKind::Jewel, 1),
        makes: Makes::FromInput(M::SILVER),
        skill: Skill::Crafting,
        minutes: 90,
    },
    Recipe {
        id: GOLD_RING,
        label: "a gold ring",
        station: StructureKind::Anvil,
        inputs: &[(ItemKind::Bar, 1), (ItemKind::Gem, 1)],
        wants: &[Some(M::GOLD)],
        output: (ItemKind::Jewel, 1),
        makes: Makes::FromInput(M::GOLD),
        skill: Skill::Crafting,
        minutes: 90,
    },
    Recipe {
        id: FEY_FIGURE,
        label: "a figure in stone",
        station: StructureKind::Workbench,
        inputs: &[(ItemKind::StoneBlock, 2)],
        wants: &[],
        output: (ItemKind::Figurine, 1),
        makes: Makes::FromInput(M::GRANITE),
        skill: Skill::Crafting,
        minutes: 240,
    },
    Recipe {
        id: FEY_CARVING,
        label: "a figure in wood",
        station: StructureKind::Workbench,
        inputs: &[(ItemKind::Log, 2)],
        wants: &[],
        output: (ItemKind::Figurine, 1),
        makes: Makes::FromInput(M::OAK),
        skill: Skill::Crafting,
        minutes: 200,
    },
    Recipe {
        id: FEY_BLADE,
        label: "a blade",
        station: StructureKind::Anvil,
        inputs: &[(ItemKind::Bar, 2)],
        wants: &[],
        output: (ItemKind::Weapon, 1),
        makes: Makes::FromInput(M::BRONZE),
        skill: Skill::Crafting,
        minutes: 240,
    },
    Recipe {
        id: FEY_IRON_BLADE,
        label: "an iron blade",
        station: StructureKind::Anvil,
        inputs: &[(ItemKind::IronBar, 2)],
        wants: &[],
        output: (ItemKind::Weapon, 1),
        makes: Makes::FromInput(M::IRON),
        skill: Skill::Crafting,
        minutes: 240,
    },
    Recipe {
        id: WOOD_SPEAR,
        label: "a fire-hardened spear",
        station: StructureKind::Workbench,
        inputs: &[(ItemKind::Log, 1)],
        wants: &[],
        output: (ItemKind::Weapon, 1),
        makes: Makes::FromInput(M::OAK),
        skill: Skill::Crafting,
        minutes: 40,
    },
    Recipe {
        id: DRESS_STONE,
        label: "dressed stone",
        station: StructureKind::Workbench,
        inputs: &[(ItemKind::RoughStone, 1)],
        wants: &[],
        output: (ItemKind::StoneBlock, 1),
        makes: Makes::FromInput(M::GRANITE),
        skill: Skill::Construction,
        minutes: 25,
    },
    Recipe {
        id: DOOR,
        label: "a door",
        station: StructureKind::CarpenterBench,
        inputs: &[(ItemKind::Plank, 2)],
        wants: &[],
        output: (ItemKind::Door, 1),
        makes: Makes::FromInput(M::OAK),
        skill: Skill::Crafting,
        minutes: 60,
    },
    Recipe {
        id: BED_FRAME,
        label: "a bed frame",
        station: StructureKind::CarpenterBench,
        inputs: &[(ItemKind::Plank, 3)],
        wants: &[],
        output: (ItemKind::BedFrame, 1),
        makes: Makes::FromInput(M::OAK),
        skill: Skill::Crafting,
        minutes: 80,
    },
    Recipe {
        id: TABLE,
        label: "a table",
        station: StructureKind::CarpenterBench,
        inputs: &[(ItemKind::Plank, 2)],
        wants: &[],
        output: (ItemKind::Table, 1),
        makes: Makes::FromInput(M::OAK),
        skill: Skill::Crafting,
        minutes: 60,
    },
    Recipe {
        id: STOOL,
        label: "a stool",
        station: StructureKind::CarpenterBench,
        inputs: &[(ItemKind::Plank, 1)],
        wants: &[],
        output: (ItemKind::Stool, 1),
        makes: Makes::FromInput(M::OAK),
        skill: Skill::Crafting,
        minutes: 30,
    },
    Recipe {
        id: CHEST,
        label: "a chest",
        station: StructureKind::CarpenterBench,
        inputs: &[(ItemKind::Plank, 2)],
        wants: &[],
        output: (ItemKind::Chest, 1),
        makes: Makes::FromInput(M::OAK),
        skill: Skill::Crafting,
        minutes: 60,
    },
    Recipe {
        id: THATCH,
        label: "thatch",
        station: StructureKind::Workbench,
        inputs: &[(ItemKind::Fibre, 2)],
        wants: &[],
        output: (ItemKind::Thatch, 1),
        makes: Makes::Fixed(M::THATCH),
        skill: Skill::Construction,
        minutes: 20,
    },
    Recipe {
        id: DYED_CLOTHES,
        label: "a set of dyed clothes",
        station: StructureKind::Loom,
        // The dye first: the clothes are of its colour (`Makes::FromInput`).
        inputs: &[(ItemKind::Dye, 1), (ItemKind::Cloth, 2)],
        wants: &[],
        output: (ItemKind::Clothes, 1),
        makes: Makes::FromInput(M::AIR),
        skill: Skill::Crafting,
        minutes: 50,
    },
];

pub fn recipe(id: u8) -> &'static Recipe {
    &RECIPES[id as usize]
}

/// The craft a recipe asks of its maker (C70, Dan's rule 3): None, anyone may make it.
pub fn craft_of(r: u8) -> Option<crate::mind::Technique> {
    use crate::mind::Technique as T;
    Some(match r {
        PLANK | DOOR | BED_FRAME | TABLE | STOOL | CHEST => T::Carpentry,
        DRESS_STONE => T::Masonry,
        COPPER_BAR | IRON_BAR | TIN_BAR | SILVER_BAR | GOLD_BAR | BRONZE => T::Smelting,
        IRON_TOOL | COPPER_TOOL | BRONZE_TOOL | IRON_WEAPON | BRONZE_WEAPON | COPPER_WEAPON
        | IRON_ARMOUR | BRONZE_ARMOUR | SILVER_RING | GOLD_RING => T::Smithing,
        CLOTH | CLOTHES | LEATHER_CLOTHES | DYED_CLOTHES => T::Weaving,
        POULTICE => T::HerbLore,
        POT | GLASS => T::Potting,
        ALE | WINE => T::Brewing,
        _ => return None,
    })
}
