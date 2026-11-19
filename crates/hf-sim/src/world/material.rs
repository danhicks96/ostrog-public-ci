//! Materials are data: a table of definitions indexed by a small id. The table is part of the
//! save so that a renamed or reordered material never silently changes an old world.

use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize, Default,
)]
pub struct Material(pub u16);

impl Material {
    pub const AIR: Material = Material(0);
    pub const SOIL: Material = Material(1);
    pub const CLAY: Material = Material(2);
    pub const SAND: Material = Material(3);
    pub const GRAVEL: Material = Material(4);
    pub const LIMESTONE: Material = Material(5);
    pub const SANDSTONE: Material = Material(6);
    pub const GRANITE: Material = Material(7);
    pub const SLATE: Material = Material(8);
    pub const BASALT: Material = Material(9);
    pub const IRON_ORE: Material = Material(10);
    pub const COPPER_ORE: Material = Material(11);
    pub const COAL: Material = Material(12);
    pub const WATER: Material = Material(13);
    pub const WOOD: Material = Material(14);
    pub const STONE_BLOCK: Material = Material(15);
    pub const THATCH: Material = Material(16);
    pub const GRASS: Material = Material(17); // soil with grass cover (surface only)
    pub const PEAT: Material = Material(18);
    pub const SNOW: Material = Material(19);
    pub const MUD: Material = Material(20);
    // Surface ground chosen by biome (world/gen/biome.rs). Appended so old saves keep their ids.
    /// Grass with wildflowers: meadows. Shows as plain grass out of spring and summer.
    pub const MEADOW: Material = Material(21);
    /// Moss over damp earth: woodland shade and stream banks.
    pub const MOSS: Material = Material(22);
    /// Hard, dry, bare earth: dry uplands and heath.
    pub const PACKED_EARTH: Material = Material(23);
    /// Permanently frozen soil: cold heights.
    pub const FROZEN_SOIL: Material = Material(24);
    /// Waterlogged soil: wet lowlands.
    pub const WET_SOIL: Material = Material(25);
    /// Loose broken stone below rises in stony country.
    pub const SCREE: Material = Material(26);
    /// Deep rock shot through with rough gems: the deep stratum only (basalt, gabbro or gneiss,
    /// #267).
    pub const GEM_ROCK: Material = Material(27);
    // M4 (C32): the rest of the valley's geology, metals, woods, fibres, hides and gems.
    // Appended; tiles use the stones and ores, items the rest.
    /// Limestone baked hard beside the granite: white, veined, prized for carving.
    pub const MARBLE: Material = Material(28);
    pub const TIN_ORE: Material = Material(29);
    pub const SILVER_ORE: Material = Material(30);
    pub const GOLD_ORE: Material = Material(31);
    pub const IRON: Material = Material(32);
    pub const COPPER: Material = Material(33);
    pub const TIN: Material = Material(34);
    /// Copper with a tenth of tin: harder than either.
    pub const BRONZE: Material = Material(35);
    pub const SILVER: Material = Material(36);
    pub const GOLD: Material = Material(37);
    pub const OAK: Material = Material(38);
    pub const BIRCH: Material = Material(39);
    pub const PINE: Material = Material(40);
    /// The pale, light stalks of the cavern fungus trees.
    pub const FUNGUS_WOOD: Material = Material(41);
    pub const REED: Material = Material(42);
    pub const DEER_HIDE: Material = Material(43);
    pub const BOAR_HIDE: Material = Material(44);
    pub const WOLF_PELT: Material = Material(45);
    pub const BEAR_PELT: Material = Material(46);
    pub const GOAT_HIDE: Material = Material(47);
    pub const FOX_PELT: Material = Material(48);
    pub const HARE_PELT: Material = Material(49);
    pub const HART_HIDE: Material = Material(50);
    pub const GARNET: Material = Material(51);
    pub const SAPPHIRE: Material = Material(52);
    pub const EMERALD: Material = Material(53);
    pub const AMETHYST: Material = Material(54);
    // C34: glass from sand, and what food is made of, so a meal remembers its ingredient.
    pub const GLASS: Material = Material(55);
    pub const MEAT: Material = Material(56);
    pub const ROOT_VEG: Material = Material(57);
    pub const GRAIN: Material = Material(58);
    pub const BERRY: Material = Material(59);
    // Appended (C59): the bone forest, above the underworld's stone (D28).
    pub const BONEWOOD: Material = Material(60);
    pub const MILKSTONE: Material = Material(61);
    pub const BONE_MOSS: Material = Material(62);
    // Appended (#267): the deep stratum differs by region, and the sandstone carries seams.
    /// Coarse dark rock of the deep, the slow-cooled kin of basalt: the deep in some regions.
    pub const GABBRO: Material = Material(63);
    /// Banded rock of the deep, squeezed and folded: the deep under the slate.
    pub const GNEISS: Material = Material(64);
    // Appended (#286): the woods of the valley's other trees, and fibre and dye plants.
    /// Light and soft-grained; it lasts in water (piles, sluices).
    pub const ALDER: Material = Material(65);
    /// Pale, tough and springy: hafts and handles.
    pub const ASH: Material = Material(66);
    /// Dense, red-hearted, close-grained: prized for carving.
    pub const YEW: Material = Material(67);
    /// Light and soft; it splits and bends.
    pub const WILLOW: Material = Material(68);
    /// Hard and fine, from the crab apple.
    pub const APPLEWOOD: Material = Material(69);
    /// Flax fibre: linen.
    pub const FLAX: Material = Material(70);
    pub const NETTLE: Material = Material(71);
    /// Dyestuffs: the colour they give cloth (systems/life.rs `dyed_colour`).
    pub const WOAD: Material = Material(72);
    pub const MADDER: Material = Material(73);
    pub const WELD: Material = Material(74);
}

/// What a raw food is made of, for the meal cooked from it (C34).
pub fn food_of(k: crate::items::ItemKind) -> Material {
    use crate::items::ItemKind as K;
    match k {
        K::RawMeat => Material::MEAT,
        K::Root => Material::ROOT_VEG,
        K::GrainSheaf => Material::GRAIN,
        K::Berries => Material::BERRY,
        _ => Material::AIR,
    }
}

/// The wood of a felled tree: the three species of `Tree` (`Plant::variant % 3`; the client
/// draws the same index), and every other kind's own wood (plants.rs `Lore::wood`, #286).
pub fn wood_of(kind: crate::plants::PlantKind, variant: u32) -> Material {
    if kind == crate::plants::PlantKind::Tree {
        return [Material::OAK, Material::BIRCH, Material::PINE][(variant % 3) as usize];
    }
    kind.lore().wood.unwrap_or(Material::WOOD)
}

/// The hide an animal gives, if it gives one worth keeping.
pub fn hide_of(s: crate::agents::Species) -> Option<Material> {
    use crate::agents::Species as S;
    Some(match s {
        S::Deer => Material::DEER_HIDE,
        S::Boar => Material::BOAR_HIDE,
        S::Wolf => Material::WOLF_PELT,
        S::Bear => Material::BEAR_PELT,
        S::Goat => Material::GOAT_HIDE,
        S::Fox => Material::FOX_PELT,
        S::Hare => Material::HARE_PELT,
        S::Hart => Material::HART_HIDE,
        _ => return None,
    })
}

/// Which gem a pocket in the deep basalt holds (`gem_in` with basalt).
pub fn gem_at(seed: u64, pos: crate::ids::TilePos) -> Material {
    gem_in(seed, pos, Material::BASALT)
}

/// Which gem a pocket holds, fixed by where it lies and the rock it lies in (#267): in basalt
/// garnets are common, amethysts less so, emeralds and sapphires rare; the gabbro holds
/// sapphires more often, the gneiss mostly garnets.
pub fn gem_in(seed: u64, pos: crate::ids::TilePos, host: Material) -> Material {
    let h = crate::rng::hash64(&[
        seed,
        crate::rng::purpose::WORLDGEN_ORE,
        0x6E4,
        pos.x as u64,
        pos.y as u64,
        pos.z as i64 as u64,
    ]) % 20;
    match host {
        Material::GABBRO => match h {
            0..=7 => Material::GARNET,
            8..=11 => Material::AMETHYST,
            12..=13 => Material::EMERALD,
            _ => Material::SAPPHIRE,
        },
        Material::GNEISS => match h {
            0..=14 => Material::GARNET,
            15..=17 => Material::AMETHYST,
            18 => Material::EMERALD,
            _ => Material::SAPPHIRE,
        },
        _ => match h {
            0..=9 => Material::GARNET,
            10..=15 => Material::AMETHYST,
            16..=17 => Material::EMERALD,
            _ => Material::SAPPHIRE,
        },
    }
}

impl Material {
    /// Living ground cover that burns (grass, peat, meadow, moss).
    pub fn is_vegetated(self) -> bool {
        matches!(
            self,
            Material::GRASS | Material::PEAT | Material::MEADOW | Material::MOSS
        )
    }

    /// Loose surface ground (soil, sand, gravel and their kin) that floodwater can spread over.
    pub fn is_loose_ground(self) -> bool {
        matches!(
            self,
            Material::SOIL
                | Material::GRASS
                | Material::CLAY
                | Material::SAND
                | Material::PEAT
                | Material::GRAVEL
                | Material::MUD
                | Material::MEADOW
                | Material::MOSS
                | Material::PACKED_EARTH
                | Material::WET_SOIL
                | Material::SCREE
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum MaterialKind {
    Air,
    Soil,
    Stone,
    Ore,
    Liquid,
    Built,
    Plant,
    // Appended (C32): item materials.
    Metal,
    Wood,
    Fibre,
    Hide,
    Gem,
    // Appended (C34).
    Glass,
    Food,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaterialDef {
    pub id: Material,
    pub name: String,
    pub kind: MaterialKind,
    /// Mining time multiplier against docs/TIME.md base (1.0 = 15 game-minutes per tile).
    pub hardness: f32,
    /// Item material yielded when mined (None = nothing).
    pub yields: Option<Material>,
    /// Palette family used by the renderer's terrain sets.
    pub family: String,
    // Properties (C32). Saved worlds from before have none; `MaterialTable::refresh` fills
    // them from the code's table when a world is loaded.
    /// Kilograms per cubic metre.
    #[serde(default)]
    pub density: f32,
    /// Degrees Celsius at which it melts (metals; sand for glass), if it melts at all.
    #[serde(default)]
    pub melts: Option<f32>,
    /// Worth of one unit against a day's food for one person (about 1).
    #[serde(default)]
    pub value: f32,
    /// Its colour, a palette entry (godot/assets/palette/ostrog.json).
    #[serde(default)]
    pub colour: String,
    /// What working it turns it into: an ore's metal.
    #[serde(default)]
    pub becomes: Option<Material>,
}

/// One row of the property table: material, density, melting point, value, colour, becomes.
type Props = (
    Material,
    f32,
    Option<f32>,
    f32,
    &'static str,
    Option<Material>,
);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MaterialTable {
    pub defs: Vec<MaterialDef>,
}

impl Default for MaterialTable {
    fn default() -> Self {
        use Material as M;
        use MaterialKind::*;
        let d = |id, name: &str, kind, hardness, yields, family: &str| MaterialDef {
            id,
            name: name.to_string(),
            kind,
            hardness,
            yields,
            family: family.to_string(),
            density: 0.0,
            melts: None,
            value: 0.0,
            colour: String::new(),
            becomes: None,
        };
        let mut t = MaterialTable {
            defs: vec![
                d(M::AIR, "air", Air, 0.0, None, "air"),
                d(M::SOIL, "soil", Soil, 0.5, None, "soil"),
                d(M::CLAY, "clay", Soil, 0.7, Some(M::CLAY), "clay"),
                d(M::SAND, "sand", Soil, 0.4, Some(M::SAND), "sand"),
                d(M::GRAVEL, "gravel", Soil, 0.6, Some(M::GRAVEL), "gravel"),
                d(
                    M::LIMESTONE,
                    "limestone",
                    Stone,
                    1.0,
                    Some(M::STONE_BLOCK),
                    "pale_stone",
                ),
                d(
                    M::SANDSTONE,
                    "sandstone",
                    Stone,
                    0.9,
                    Some(M::STONE_BLOCK),
                    "warm_stone",
                ),
                d(
                    M::GRANITE,
                    "granite",
                    Stone,
                    1.6,
                    Some(M::STONE_BLOCK),
                    "grey_stone",
                ),
                d(
                    M::SLATE,
                    "slate",
                    Stone,
                    1.2,
                    Some(M::STONE_BLOCK),
                    "dark_stone",
                ),
                d(
                    M::BASALT,
                    "basalt",
                    Stone,
                    1.8,
                    Some(M::STONE_BLOCK),
                    "black_stone",
                ),
                d(
                    M::IRON_ORE,
                    "iron ore",
                    Ore,
                    1.7,
                    Some(M::IRON_ORE),
                    "ore_iron",
                ),
                d(
                    M::COPPER_ORE,
                    "copper ore",
                    Ore,
                    1.5,
                    Some(M::COPPER_ORE),
                    "ore_copper",
                ),
                d(M::COAL, "coal", Ore, 1.1, Some(M::COAL), "ore_coal"),
                d(M::WATER, "water", Liquid, 0.0, None, "water"),
                d(M::WOOD, "wood", Built, 0.6, Some(M::WOOD), "wood"),
                d(
                    M::STONE_BLOCK,
                    "stone block",
                    Built,
                    1.2,
                    Some(M::STONE_BLOCK),
                    "block",
                ),
                d(M::THATCH, "thatch", Built, 0.3, None, "thatch"),
                d(M::GRASS, "grass", Soil, 0.5, None, "grass"),
                d(M::PEAT, "peat", Soil, 0.6, Some(M::PEAT), "peat"),
                d(M::SNOW, "snow", Soil, 0.2, None, "snow"),
                d(M::MUD, "mud", Soil, 0.5, None, "mud"),
                d(M::MEADOW, "meadow", Soil, 0.5, None, "wildflower"),
                d(M::MOSS, "moss", Soil, 0.5, None, "mossy"),
                d(
                    M::PACKED_EARTH,
                    "packed earth",
                    Soil,
                    0.7,
                    None,
                    "packed_earth",
                ),
                d(
                    M::FROZEN_SOIL,
                    "frozen soil",
                    Soil,
                    0.9,
                    None,
                    "frozen_soil",
                ),
                d(M::WET_SOIL, "wet soil", Soil, 0.5, None, "wet_soil"),
                d(M::SCREE, "scree", Soil, 0.6, Some(M::GRAVEL), "scree"),
                d(
                    M::GEM_ROCK,
                    "gem-bearing rock",
                    Ore,
                    1.8,
                    Some(M::GEM_ROCK),
                    "ore_gem",
                ),
                d(
                    M::MARBLE,
                    "marble",
                    Stone,
                    1.3,
                    Some(M::STONE_BLOCK),
                    "marble",
                ),
                d(M::TIN_ORE, "tin ore", Ore, 1.4, Some(M::TIN_ORE), "ore_tin"),
                d(
                    M::SILVER_ORE,
                    "silver ore",
                    Ore,
                    1.5,
                    Some(M::SILVER_ORE),
                    "ore_silver",
                ),
                d(
                    M::GOLD_ORE,
                    "gold ore",
                    Ore,
                    1.6,
                    Some(M::GOLD_ORE),
                    "ore_gold",
                ),
                d(M::IRON, "iron", Metal, 1.7, None, "metal"),
                d(M::COPPER, "copper", Metal, 1.1, None, "metal"),
                d(M::TIN, "tin", Metal, 0.6, None, "metal"),
                d(M::BRONZE, "bronze", Metal, 1.5, None, "metal"),
                d(M::SILVER, "silver", Metal, 0.9, None, "metal"),
                d(M::GOLD, "gold", Metal, 0.7, None, "metal"),
                d(M::OAK, "oak", Wood, 0.7, None, "wood"),
                d(M::BIRCH, "birch", Wood, 0.6, None, "wood"),
                d(M::PINE, "pine", Wood, 0.5, None, "wood"),
                d(M::FUNGUS_WOOD, "fungus wood", Wood, 0.4, None, "wood"),
                d(M::REED, "reed", Fibre, 0.1, None, "fibre"),
                d(M::DEER_HIDE, "deer hide", Hide, 0.2, None, "hide"),
                d(M::BOAR_HIDE, "boar hide", Hide, 0.25, None, "hide"),
                d(M::WOLF_PELT, "wolf pelt", Hide, 0.2, None, "hide"),
                d(M::BEAR_PELT, "bear pelt", Hide, 0.25, None, "hide"),
                d(M::GOAT_HIDE, "goat hide", Hide, 0.2, None, "hide"),
                d(M::FOX_PELT, "fox pelt", Hide, 0.15, None, "hide"),
                d(M::HARE_PELT, "hare pelt", Hide, 0.1, None, "hide"),
                d(M::HART_HIDE, "white hart's hide", Hide, 0.2, None, "hide"),
                d(M::GARNET, "garnet", Gem, 2.0, None, "gem"),
                d(M::SAPPHIRE, "sapphire", Gem, 2.6, None, "gem"),
                d(M::EMERALD, "emerald", Gem, 2.2, None, "gem"),
                d(M::AMETHYST, "amethyst", Gem, 2.0, None, "gem"),
                d(M::GLASS, "glass", Glass, 0.8, None, "glass"),
                d(M::MEAT, "meat", Food, 0.0, None, "food"),
                d(M::ROOT_VEG, "roots", Food, 0.0, None, "food"),
                d(M::GRAIN, "grain", Food, 0.0, None, "food"),
                d(M::BERRY, "berries", Food, 0.0, None, "food"),
                d(M::BONEWOOD, "bonewood", Wood, 0.9, None, "wood"),
                d(
                    M::MILKSTONE,
                    "milkstone",
                    Stone,
                    1.0,
                    Some(M::STONE_BLOCK),
                    "milkstone",
                ),
                d(M::BONE_MOSS, "bone moss", Soil, 0.5, None, "bone_moss"),
                d(
                    M::GABBRO,
                    "gabbro",
                    Stone,
                    1.9,
                    Some(M::STONE_BLOCK),
                    "black_stone",
                ),
                d(
                    M::GNEISS,
                    "gneiss",
                    Stone,
                    1.7,
                    Some(M::STONE_BLOCK),
                    "dark_stone",
                ),
                d(M::ALDER, "alder", Wood, 0.5, None, "wood"),
                d(M::ASH, "ash", Wood, 0.7, None, "wood"),
                d(M::YEW, "yew", Wood, 0.8, None, "wood"),
                d(M::WILLOW, "willow", Wood, 0.4, None, "wood"),
                d(M::APPLEWOOD, "applewood", Wood, 0.75, None, "wood"),
                d(M::FLAX, "linen", Fibre, 0.1, None, "fibre"),
                d(M::NETTLE, "nettle", Fibre, 0.1, None, "fibre"),
                d(M::WOAD, "woad-blue", Fibre, 0.1, None, "fibre"),
                d(M::MADDER, "madder-red", Fibre, 0.1, None, "fibre"),
                d(M::WELD, "weld-yellow", Fibre, 0.1, None, "fibre"),
            ],
        };
        // Density kg/m3, melting point C, value (a day's food = 1), palette colour, becomes.
        #[rustfmt::skip]
        let props: &[Props] = &[
            (M::AIR, 1.2, None, 0.0, "#000000", None),
            (M::SOIL, 1500.0, None, 0.0, "#6f432e", None),
            (M::CLAY, 1800.0, None, 0.5, "#9c5132", None),
            (M::SAND, 1600.0, Some(1700.0), 0.2, "#cea773", None),
            (M::GRAVEL, 1700.0, None, 0.1, "#7b8190", None),
            (M::LIMESTONE, 2600.0, None, 1.0, "#c3c1be", None),
            (M::SANDSTONE, 2300.0, None, 1.0, "#cea773", None),
            (M::GRANITE, 2700.0, None, 1.2, "#9ea1a7", None),
            (M::SLATE, 2800.0, None, 1.0, "#484e5f", None),
            (M::BASALT, 3000.0, None, 1.0, "#333748", None),
            (M::IRON_ORE, 5000.0, None, 2.0, "#8b5c3a", Some(M::IRON)),
            (M::COPPER_ORE, 4200.0, None, 2.0, "#be6d45", Some(M::COPPER)),
            (M::COAL, 1350.0, None, 1.0, "#2b2634", None),
            (M::WATER, 1000.0, None, 0.0, "#286387", None),
            (M::WOOD, 600.0, None, 1.0, "#8b5c3a", None),
            (M::STONE_BLOCK, 2600.0, None, 1.0, "#9ea1a7", None),
            (M::THATCH, 150.0, None, 0.2, "#d0a044", None),
            (M::GRASS, 1500.0, None, 0.0, "#53893a", None),
            (M::PEAT, 700.0, None, 0.5, "#40282e", None),
            (M::SNOW, 300.0, None, 0.0, "#e9e3d6", None),
            (M::MUD, 1700.0, None, 0.0, "#56362f", None),
            (M::MEADOW, 1500.0, None, 0.0, "#7ea94f", None),
            (M::MOSS, 1500.0, None, 0.0, "#366e32", None),
            (M::PACKED_EARTH, 1800.0, None, 0.0, "#ac8156", None),
            (M::FROZEN_SOIL, 1700.0, None, 0.0, "#95c8ce", None),
            (M::WET_SOIL, 1800.0, None, 0.0, "#56362f", None),
            (M::SCREE, 1700.0, None, 0.1, "#7b8190", None),
            (M::GEM_ROCK, 3000.0, None, 2.0, "#333748", None),
            (M::MARBLE, 2700.0, None, 3.0, "#e9e3d6", None),
            (M::TIN_ORE, 6900.0, None, 3.0, "#5e6678", Some(M::TIN)),
            (M::SILVER_ORE, 7400.0, None, 8.0, "#9ea1a7", Some(M::SILVER)),
            (M::GOLD_ORE, 2700.0, None, 15.0, "#d0a044", Some(M::GOLD)),
            (M::IRON, 7870.0, Some(1538.0), 6.0, "#7b8190", None),
            (M::COPPER, 8960.0, Some(1085.0), 5.0, "#be6d45", None),
            (M::TIN, 7290.0, Some(232.0), 6.0, "#c3c1be", None),
            (M::BRONZE, 8800.0, Some(950.0), 9.0, "#b07f28", None),
            (M::SILVER, 10490.0, Some(962.0), 25.0, "#e9e3d6", None),
            (M::GOLD, 19300.0, Some(1064.0), 50.0, "#f0d893", None),
            (M::OAK, 750.0, None, 2.0, "#6f432e", None),
            (M::BIRCH, 670.0, None, 1.5, "#cea773", None),
            (M::PINE, 510.0, None, 1.0, "#ac8156", None),
            (M::FUNGUS_WOOD, 400.0, None, 1.5, "#9589d1", None),
            (M::REED, 300.0, None, 0.5, "#9fa667", None),
            (M::DEER_HIDE, 950.0, None, 3.0, "#ac8156", None),
            (M::BOAR_HIDE, 980.0, None, 3.0, "#56362f", None),
            (M::WOLF_PELT, 900.0, None, 5.0, "#7b8190", None),
            (M::BEAR_PELT, 950.0, None, 8.0, "#40282e", None),
            (M::GOAT_HIDE, 920.0, None, 2.5, "#cea773", None),
            (M::FOX_PELT, 880.0, None, 4.0, "#d96339", None),
            (M::HARE_PELT, 850.0, None, 1.0, "#9ea1a7", None),
            (M::HART_HIDE, 950.0, None, 40.0, "#fbf7ef", None),
            (M::GARNET, 4000.0, None, 20.0, "#e0365a", None),
            (M::SAPPHIRE, 4000.0, None, 45.0, "#4f8ef0", None),
            (M::EMERALD, 2760.0, None, 45.0, "#3fcf86", None),
            (M::AMETHYST, 2650.0, None, 25.0, "#b878f0", None),
            (M::GLASS, 2500.0, Some(1400.0), 4.0, "#95c8ce", None),
            (M::MEAT, 1050.0, None, 1.0, "#b9422e", None),
            (M::ROOT_VEG, 900.0, None, 0.6, "#cea773", None),
            (M::GRAIN, 750.0, None, 0.5, "#e0bc6d", None),
            (M::BERRY, 950.0, None, 0.8, "#8e3234", None),
            // Pale and close-grained, heavier than oak; burns slow and white.
            (M::BONEWOOD, 820.0, None, 3.0, "#e9e3d6", None),
            (M::MILKSTONE, 2400.0, None, 2.0, "#e9e3d6", None),
            (M::BONE_MOSS, 1400.0, None, 0.0, "#c3c1be", None),
            // #274: the deep's rock dresses to a fine dark face (gabbro) or a banded one
            // (gneiss), worth twice a plain stone (tiers.rs `FINE_VALUE`).
            (M::GABBRO, 3100.0, None, 2.0, "#2b2634", None),
            (M::GNEISS, 2800.0, None, 2.0, "#5e6678", None),
            // #286: the other trees' woods, worth by how they work (yew is the carvers').
            (M::ALDER, 500.0, None, 1.0, "#ac8156", None),
            (M::ASH, 700.0, None, 1.8, "#cea773", None),
            (M::YEW, 670.0, None, 3.0, "#9c5132", None),
            (M::WILLOW, 420.0, None, 0.8, "#cea773", None),
            (M::APPLEWOOD, 750.0, None, 2.2, "#8b5c3a", None),
            (M::FLAX, 300.0, None, 1.0, "#c3c1be", None),
            (M::NETTLE, 300.0, None, 0.6, "#9fa667", None),
            (M::WOAD, 400.0, None, 2.0, "#286387", None),
            (M::MADDER, 400.0, None, 2.5, "#b9422e", None),
            (M::WELD, 400.0, None, 1.5, "#d0a044", None),
        ];
        for &(m, density, melts, value, colour, becomes) in props {
            let def = &mut t.defs[m.0 as usize];
            def.density = density;
            def.melts = melts;
            def.value = value;
            def.colour = colour.to_string();
            def.becomes = becomes;
        }
        t
    }
}

impl MaterialTable {
    /// Bring a loaded world's table up to the code's: a definition the code still has under the
    /// same id and name takes the code's properties, and materials added since are appended.
    /// One the code names differently is kept as it was saved (C32).
    pub fn refresh(&mut self) {
        let code = MaterialTable::default();
        for def in code.defs {
            let ix = def.id.0 as usize;
            let len = self.defs.len();
            match self.defs.get_mut(ix) {
                Some(old) if old.name == def.name => *old = def,
                Some(_) => {}
                None if ix == len => self.defs.push(def),
                None => {}
            }
        }
    }

    pub fn get(&self, m: Material) -> &MaterialDef {
        match self.defs.get(m.0 as usize) {
            Some(d) => d,
            None => &self.defs[0],
        }
    }
}
