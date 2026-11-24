//! Plants: trees, bushes, reeds, crops. Entities with growth; harvested for wood/food.
//!
//! Every kind's data is one row of `PlantKind::lore` (#286): how long it takes to grow, the
//! seasons it bears in, what it gives and how much, and the warmth it is placed in. The world's
//! flora draw and the biome tables (world/gen/biome.rs) say what can grow where; the rest of
//! the sim asks the row.

use serde::{Deserialize, Serialize};

use crate::ids::TilePos;
use crate::items::ItemKind;
use crate::time::{Season, TICKS_PER_DAY};
use crate::world::Material;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum PlantKind {
    Tree,
    Shrub,
    BerryBush,
    Reed,
    Grain,
    Root,
    /// Cave floors only; foraged like berries.
    Mushroom,
    // Appended (issue #58): the generator places these by biome (world/gen/flora.rs).
    /// Wild herbs: dry heath and meadow. Foraged as roots-and-herbs, which the hearth turns
    /// into poultices (work/recipes.rs POULTICE).
    Herbs,
    /// Damp shade: woodland and fen. Scenery and grazing.
    Fern,
    /// Climbs the foot of rises and rock. Scenery and grazing.
    Vine,
    /// Wild turnips and the like: grassland and heath. Foraged as food.
    RootCrops,
    /// Single flowering plants: meadows. Scenery and grazing.
    FloweringPlant,
    /// Wildflower clumps: meadows in spring and summer. Scenery and grazing.
    Wildflowers,
    /// What a felled tree leaves behind (work: Chop). Does not grow.
    Stump,
    /// A fallen trunk: deadfall in woodland, and sometimes the top of a felled tree.
    FallenLog,
    /// A pale fungus as tall as a sapling, in the forests of the caverns. Its stalks are cave
    /// timber.
    FungusTree,
    /// The bone forest's tree (C59): white-barked and leafless, over the underworld's stone.
    BoneTree,
    // Appended (#286): the valley's flora by species. Trees by climate, each its own wood.
    Alder,
    Ash,
    Yew,
    Willow,
    CrabApple,
    // Wild food, each in its season.
    Hazel,
    Bilberry,
    WildStrawberry,
    Bramble,
    WildGarlic,
    Chanterelle,
    Cattail,
    // Look-alikes: each looks like a food above and sickens whoever eats it.
    DeathCap,
    Hemlock,
    Nightshade,
    LilyOfTheValley,
    // Useful plants: fibre, dye and medicine.
    Nettle,
    Woad,
    Madder,
    Weld,
    Yarrow,
    Comfrey,
    // Crops sown in fields, each to its own ground (the old `Grain` is the wheat).
    Barley,
    Oats,
    Rye,
    Peas,
    Flax,
    /// Appended (#286): a spiny weed of worked ground. It arrives on worked fields over time,
    /// crowds the crops beside it, and is pulled by hand where it presses them.
    Thistle,
}

/// Seasons as bits, for `Lore::bears`.
pub const SPRING: u8 = 1;
pub const SUMMER: u8 = 2;
pub const AUTUMN: u8 = 4;
pub const WINTER: u8 = 8;
pub const ALL_YEAR: u8 = SPRING | SUMMER | AUTUMN | WINTER;

/// What a plant gives the people who know what to do with it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Gives {
    Nothing,
    /// Gathered from the plant in its season and eaten.
    Food(ItemKind),
    /// Sown in a field and harvested when grown: the item, and its material (AIR when the item
    /// has none of its own).
    Crop(ItemKind, Material),
    /// Cut for the loom when it is grown (work: Harvest).
    Fibre(Material),
    /// Cut for the dyer: the colour it gives cloth.
    Dye(Material),
    /// A herb laid on a wound: one who knows herb lore makes a poultice of it where it grows.
    Medicine,
    /// Looks like the given kind's food and sickens whoever eats it; the number is how badly
    /// (the illness's starting severity).
    LookAlike(PlantKind, f32),
}

/// One kind's data (#286).
#[derive(Clone, Copy, Debug)]
pub struct Lore {
    /// Plain words for reports.
    pub label: &'static str,
    /// A line of prose for the record and the hover.
    pub prose: &'static str,
    /// Days from seedling to full growth (docs/TIME.md: a season is 5 days, crops 3 to 8).
    pub days: f32,
    /// The seasons it bears in (bits): fruit, nuts, greens or roots ready to gather.
    pub bears: u8,
    pub gives: Gives,
    /// How many of what it gives one gathering or harvest yields, before skill.
    pub yields: u32,
    /// The wood it gives when felled, if it is felled for wood.
    pub wood: Option<Material>,
    /// Low green growth that grazing animals seek out.
    pub grazed: bool,
    /// The warmth it is placed in: the generator's temperature field, 0 cold to 1 warm.
    pub climate: (f32, f32),
}

const ANY: (f32, f32) = (-9.0, 9.0);

const fn row(label: &'static str, prose: &'static str, days: f32, gives: Gives) -> Lore {
    Lore {
        label,
        prose,
        days,
        bears: ALL_YEAR,
        gives,
        yields: 2,
        wood: None,
        grazed: false,
        climate: ANY,
    }
}

impl PlantKind {
    /// The kind's data (#286).
    pub fn lore(self) -> Lore {
        use Gives::*;
        use ItemKind as I;
        use Material as M;
        use PlantKind as P;
        match self {
            P::Tree => Lore {
                wood: Some(M::OAK),
                ..row(
                    "trees",
                    "Oak, birch or pine, as the ground suits.",
                    34.3,
                    Nothing,
                )
            },
            P::Shrub => Lore {
                grazed: true,
                ..row("shrubs", "Scrub, browsed by deer and goats.", 10.0, Nothing)
            },
            P::BerryBush => Lore {
                grazed: true,
                ..row(
                    "berry bushes",
                    "Red berries in clusters.",
                    8.0,
                    Food(I::Berries),
                )
            },
            P::Reed => Lore {
                grazed: true,
                yields: 3,
                ..row(
                    "reeds",
                    "Cut for the loom and bundled for thatch.",
                    5.0,
                    Fibre(M::REED),
                )
            },
            P::Grain => Lore {
                yields: 6,
                ..row(
                    "grain",
                    "Wheat: the best of the field crops on good ground.",
                    5.0,
                    Crop(I::GrainSheaf, M::AIR),
                )
            },
            P::Root => row(
                "wild carrots",
                "A pale root under feathery leaves.",
                4.0,
                Food(I::Root),
            ),
            P::Mushroom => row(
                "mushrooms",
                "Pale caps on the cave floor.",
                3.0,
                Food(I::Root),
            ),
            P::Herbs => Lore {
                grazed: true,
                ..row(
                    "herbs",
                    "Thyme and sage; the hearth makes poultices of them.",
                    6.0,
                    Food(I::Root),
                )
            },
            P::Fern => Lore {
                grazed: true,
                ..row("ferns", "Bracken in the damp shade.", 7.0, Nothing)
            },
            P::Vine => Lore {
                grazed: true,
                ..row("vines", "Climbs the foot of the rock.", 12.0, Nothing)
            },
            P::RootCrops => row(
                "wild turnips",
                "Purple-topped roots, good keeping.",
                6.0,
                Food(I::Root),
            ),
            P::FloweringPlant => Lore {
                grazed: true,
                ..row("flowering plants", "Tall purple flowers.", 5.0, Nothing)
            },
            P::Wildflowers => Lore {
                grazed: true,
                ..row(
                    "wildflowers",
                    "Gone under the winter; back each spring.",
                    3.0,
                    Nothing,
                )
            },
            P::Stump => row("stumps", "What a felled tree leaves.", 1.0e6, Nothing),
            P::FallenLog => row("fallen logs", "A fallen trunk.", 1.0e6, Nothing),
            P::FungusTree => Lore {
                wood: Some(M::FUNGUS_WOOD),
                ..row(
                    "fungus trees",
                    "A pale fungus as tall as a sapling; its stalks are cave timber.",
                    20.0,
                    Nothing,
                )
            },
            P::BoneTree => Lore {
                wood: Some(M::BONEWOOD),
                ..row(
                    "bone trees",
                    "White-barked, over the underworld's stone.",
                    34.3,
                    Nothing,
                )
            },
            // Trees by climate (#286): each its own wood, so a wall is of the wood the valley
            // grows.
            P::Alder => Lore {
                wood: Some(M::ALDER),
                climate: (0.15, 0.9),
                ..row(
                    "alders",
                    "Wet-footed by streams and in the fen; its wood lasts in water.",
                    30.0,
                    Nothing,
                )
            },
            P::Ash => Lore {
                wood: Some(M::ASH),
                climate: (0.35, 9.0),
                ..row(
                    "ash trees",
                    "Tall and open-crowned on good ground; tough wood for hafts.",
                    38.6,
                    Nothing,
                )
            },
            P::Yew => Lore {
                wood: Some(M::YEW),
                climate: (0.2, 9.0),
                ..row(
                    "yews",
                    "Dark and slow on the stony ground; close wood, dear to carvers.",
                    68.6,
                    Nothing,
                )
            },
            P::Willow => Lore {
                wood: Some(M::WILLOW),
                climate: (0.25, 9.0),
                ..row(
                    "willows",
                    "Leaning over the water; quick to grow, light and soft.",
                    21.4,
                    Nothing,
                )
            },
            P::CrabApple => Lore {
                wood: Some(M::APPLEWOOD),
                bears: AUTUMN,
                yields: 3,
                climate: (0.45, 9.0),
                ..row(
                    "crab apples",
                    "Small sour apples in autumn on a crooked tree.",
                    30.0,
                    Food(I::Berries),
                )
            },
            // Wild food, each in its season.
            P::Hazel => Lore {
                bears: AUTUMN,
                yields: 3,
                climate: (0.3, 9.0),
                ..row(
                    "hazels",
                    "Many-stemmed; its nuts ripen in autumn and keep.",
                    12.0,
                    Food(I::Root),
                )
            },
            P::Bilberry => Lore {
                bears: SUMMER,
                grazed: true,
                climate: (-9.0, 0.65),
                ..row(
                    "bilberries",
                    "Low and wiry on cold ground; dark blue berries in summer.",
                    8.0,
                    Food(I::Berries),
                )
            },
            P::WildStrawberry => Lore {
                bears: SPRING | SUMMER,
                grazed: true,
                climate: (0.45, 9.0),
                ..row(
                    "wild strawberries",
                    "Runners in the grass; small sweet berries from late spring.",
                    5.0,
                    Food(I::Berries),
                )
            },
            P::Bramble => Lore {
                bears: SUMMER | AUTUMN,
                yields: 3,
                climate: (0.35, 9.0),
                ..row(
                    "brambles",
                    "Thorned canes; blackberries from late summer.",
                    10.0,
                    Food(I::Berries),
                )
            },
            P::WildGarlic => Lore {
                bears: SPRING,
                climate: (0.3, 0.9),
                ..row(
                    "wild garlic",
                    "Broad leaves and white stars in damp shade; the first greens of spring.",
                    4.0,
                    Food(I::Root),
                )
            },
            P::Chanterelle => Lore {
                bears: SUMMER | AUTUMN,
                climate: (0.25, 9.0),
                ..row(
                    "chanterelles",
                    "Egg-yellow funnels under the trees after rain.",
                    3.0,
                    Food(I::Root),
                )
            },
            P::Cattail => Lore {
                grazed: true,
                yields: 3,
                ..row(
                    "bulrushes",
                    "Brown heads on tall stalks at the water's edge; the roots are starchy.",
                    6.0,
                    Food(I::Root),
                )
            },
            // Look-alikes (#286): each looks like a food and sickens whoever eats it. Those
            // who know herb lore, or have been sick from one, leave them (`knows_look_alike`).
            P::DeathCap => Lore {
                bears: SUMMER | AUTUMN,
                climate: (0.3, 9.0),
                ..row(
                    "death caps",
                    "A pale green-white cap with a skirt, under oak and birch; it kills.",
                    3.0,
                    LookAlike(P::Chanterelle, 0.55),
                )
            },
            P::Hemlock => Lore {
                bears: SUMMER,
                climate: (0.3, 9.0),
                ..row(
                    "hemlock",
                    "Like a wild carrot gone tall, with purple-blotched stems.",
                    5.0,
                    LookAlike(P::Root, 0.5),
                )
            },
            P::Nightshade => Lore {
                bears: SUMMER | AUTUMN,
                climate: (0.4, 9.0),
                ..row(
                    "nightshade",
                    "Shining black berries, one to a stalk.",
                    7.0,
                    LookAlike(P::Bilberry, 0.4),
                )
            },
            P::LilyOfTheValley => Lore {
                bears: SPRING,
                climate: (0.3, 0.9),
                ..row(
                    "lily of the valley",
                    "Leaves like wild garlic's and white bells; no garlic smell.",
                    4.0,
                    LookAlike(P::WildGarlic, 0.35),
                )
            },
            // Useful plants: fibre, dye and medicine.
            P::Nettle => Lore {
                bears: SUMMER,
                yields: 3,
                climate: (0.25, 9.0),
                ..row(
                    "nettles",
                    "Stinging stands on rich damp ground; retted, the stems spin to cloth.",
                    5.0,
                    Fibre(M::NETTLE),
                )
            },
            P::Woad => Lore {
                bears: SUMMER,
                grazed: true,
                climate: (0.35, 9.0),
                ..row(
                    "woad",
                    "Yellow-flowered; its leaves dye cloth blue.",
                    8.0,
                    Dye(M::WOAD),
                )
            },
            P::Madder => Lore {
                bears: SUMMER | AUTUMN,
                grazed: true,
                climate: (0.55, 9.0),
                ..row(
                    "madder",
                    "A sprawling plant of warm ground; its root dyes cloth red.",
                    10.0,
                    Dye(M::MADDER),
                )
            },
            P::Weld => Lore {
                bears: SUMMER,
                grazed: true,
                climate: (0.3, 9.0),
                ..row(
                    "weld",
                    "Tall pale-yellow spikes on poor stony ground; it dyes cloth yellow.",
                    7.0,
                    Dye(M::WELD),
                )
            },
            P::Yarrow => Lore {
                bears: SPRING | SUMMER | AUTUMN,
                grazed: true,
                climate: ANY,
                ..row(
                    "yarrow",
                    "Flat white flower heads and feathered leaves; it stanches a wound.",
                    5.0,
                    Medicine,
                )
            },
            P::Comfrey => Lore {
                bears: SPRING | SUMMER,
                grazed: true,
                climate: (0.3, 9.0),
                ..row(
                    "comfrey",
                    "Broad rough leaves and purple bells by wet ground; knitbone.",
                    6.0,
                    Medicine,
                )
            },
            // Crops (#286): each to its own ground (`crop_suits`).
            P::Barley => Lore {
                yields: 5,
                ..row(
                    "barley",
                    "Quick and hardy; it takes dry and stony ground wheat will not.",
                    4.0,
                    Crop(I::GrainSheaf, M::AIR),
                )
            },
            P::Oats => Lore {
                yields: 5,
                ..row(
                    "oats",
                    "Loose heads; it thrives in wet and cool ground.",
                    5.0,
                    Crop(I::GrainSheaf, M::AIR),
                )
            },
            P::Rye => Lore {
                yields: 5,
                ..row(
                    "rye",
                    "Tall and blue-green; it stands a frost and grows on into autumn.",
                    6.0,
                    Crop(I::GrainSheaf, M::AIR),
                )
            },
            P::Peas => Lore {
                yields: 5,
                ..row(
                    "peas",
                    "Climbing pods in mild meadow ground; thirsty.",
                    4.0,
                    Crop(I::Root, M::AIR),
                )
            },
            P::Flax => Lore {
                yields: 4,
                ..row(
                    "flax",
                    "Slender stems with blue flowers, sown for linen; thirsty.",
                    6.0,
                    Crop(I::Fibre, M::FLAX),
                )
            },
            P::Thistle => Lore {
                yields: 0,
                ..row(
                    "thistles",
                    "A spiny weed of worked ground; crowds the crops beside it until pulled.",
                    3.0,
                    Nothing,
                )
            },
        }
    }

    /// Days from seedling to full growth (docs/TIME.md: crops 3–8 days; trees much longer).
    pub fn days_to_mature(self) -> f32 {
        self.lore().days
    }
    /// Bears something people gather and eat (look-alikes are not food).
    pub fn yields_food(self) -> bool {
        match self.lore().gives {
            Gives::Food(_) => true,
            Gives::Crop(k, _) => k.is_food(),
            _ => false,
        }
    }
    /// Bears something that looks like food: food, and the look-alikes. What foragers see.
    pub fn bears_fruit(self) -> bool {
        self.yields_food() || self.look_alike().is_some()
    }
    /// The food this kind looks like and how badly it sickens, if it is a look-alike.
    pub fn look_alike(self) -> Option<(PlantKind, f32)> {
        match self.lore().gives {
            Gives::LookAlike(k, harm) => Some((k, harm)),
            _ => None,
        }
    }
    /// Sown in fields and harvested when grown.
    pub fn is_crop(self) -> bool {
        matches!(self.lore().gives, Gives::Crop(..))
    }
    /// A weed of worked ground (#286): not a crop, not forage; it crowds crops and is pulled.
    pub fn is_weed(self) -> bool {
        matches!(self, PlantKind::Thistle)
    }
    /// Whether it bears in `season`.
    pub fn bears_in(self, season: Season) -> bool {
        let bit = match season {
            Season::Spring => SPRING,
            Season::Summer => SUMMER,
            Season::Autumn => AUTUMN,
            Season::Winter => WINTER,
        };
        self.lore().bears & bit != 0
    }
    /// Living and growing (not dead wood).
    pub fn grows(self) -> bool {
        !matches!(self, PlantKind::Stump | PlantKind::FallenLog)
    }
    /// Low green growth that grazing animals seek out.
    pub fn grazed(self) -> bool {
        self.lore().grazed
    }
    /// Every kind, in enum order (for reports and tests).
    pub const ALL: [PlantKind; 45] = [
        PlantKind::Tree,
        PlantKind::Shrub,
        PlantKind::BerryBush,
        PlantKind::Reed,
        PlantKind::Grain,
        PlantKind::Root,
        PlantKind::Mushroom,
        PlantKind::Herbs,
        PlantKind::Fern,
        PlantKind::Vine,
        PlantKind::RootCrops,
        PlantKind::FloweringPlant,
        PlantKind::Wildflowers,
        PlantKind::Stump,
        PlantKind::FallenLog,
        PlantKind::FungusTree,
        PlantKind::BoneTree,
        PlantKind::Alder,
        PlantKind::Ash,
        PlantKind::Yew,
        PlantKind::Willow,
        PlantKind::CrabApple,
        PlantKind::Hazel,
        PlantKind::Bilberry,
        PlantKind::WildStrawberry,
        PlantKind::Bramble,
        PlantKind::WildGarlic,
        PlantKind::Chanterelle,
        PlantKind::Cattail,
        PlantKind::DeathCap,
        PlantKind::Hemlock,
        PlantKind::Nightshade,
        PlantKind::LilyOfTheValley,
        PlantKind::Nettle,
        PlantKind::Woad,
        PlantKind::Madder,
        PlantKind::Weld,
        PlantKind::Yarrow,
        PlantKind::Comfrey,
        PlantKind::Barley,
        PlantKind::Oats,
        PlantKind::Rye,
        PlantKind::Peas,
        PlantKind::Flax,
        PlantKind::Thistle,
    ];
    /// Plain words for reports.
    pub fn label(self) -> &'static str {
        self.lore().label
    }
    pub fn yields_wood(self) -> bool {
        self.lore().wood.is_some()
    }
    /// A tree of the surface, felled for its wood: every wood-giver but the cavern fungus.
    pub fn is_tree(self) -> bool {
        self.yields_wood() && self != PlantKind::FungusTree
    }
    /// A tree of the valley's woods (not the bone forest's, not the cavern's).
    pub fn is_green_tree(self) -> bool {
        self.is_tree() && self != PlantKind::BoneTree
    }
    pub fn blocks_movement(self) -> bool {
        self.yields_wood()
    }
    /// The kind of a plant tag (the `kind as u16` knowledge notes carry).
    pub fn from_tag(tag: u16) -> Option<PlantKind> {
        PlantKind::ALL.get(tag as usize).copied()
    }
}

/// Whether this person knows `kind` for a look-alike and leaves it (#286, Dan's rule 3:
/// knowledge belongs to people): those who know herb lore know them all; anyone else knows
/// the ones that made them sick, that they saw make someone sick, or that they were told of.
pub fn knows_look_alike(a: &crate::agents::Agent, kind: PlantKind) -> bool {
    a.techniques.knows(crate::mind::Technique::HerbLore) || a.shuns.contains(&kind)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plant {
    pub pos: TilePos,
    pub kind: PlantKind,
    /// 0..1
    pub growth: f32,
    /// Per-plant visual seed (variant selection).
    pub variant: u32,
    /// Fruit/berries ready (0..1); regrows after harvest.
    pub fruit: f32,
    pub alive: bool,
}

impl Plant {
    pub fn new(pos: TilePos, kind: PlantKind, growth: f32, variant: u32) -> Plant {
        Plant {
            pos,
            kind,
            growth,
            variant,
            fruit: if kind.bears_fruit() { growth } else { 0.0 },
            alive: true,
        }
    }

    /// Advance by `ticks` at a seasonal factor (0 = dormant, 1 = full growth). Fruit comes only
    /// in the seasons the kind bears in, and out of them what is left falls in about a day
    /// (#286).
    pub fn grow(&mut self, ticks: u64, season_factor: f32, season: Season) {
        if !self.kind.grows() {
            return;
        }
        let per_tick = 1.0 / (self.kind.days_to_mature() * TICKS_PER_DAY as f32);
        let d = per_tick * ticks as f32 * season_factor;
        self.growth = (self.growth + d).min(1.0);
        if self.kind.bears_fruit() && self.growth >= 0.6 {
            if self.kind.bears_in(season) {
                // Fruit returns in about a fifth of the time to maturity (berries: ~1.5 days).
                self.fruit = (self.fruit + d * 5.0).min(1.0);
            } else {
                self.fruit = (self.fruit - ticks as f32 / TICKS_PER_DAY as f32).max(0.0);
            }
        }
    }

    pub fn mature(&self) -> bool {
        self.growth >= 0.95
    }
}

/// How well `crop` does on ground of `biome` (#286), 0..1: a growth factor, and what the
/// farmers weigh when they choose what to sow. Wheat wants good mild ground; barley takes dry
/// and stony ground, oats wet and cool, rye the cold heights; peas want mild meadow.
pub fn crop_suits(crop: PlantKind, biome: crate::world::gen::biome::Biome) -> f32 {
    use crate::world::gen::biome::Biome as B;
    use PlantKind as P;
    // Grassland, meadow, woodland, fen, heath, stony, fell, snowfield, bone forest.
    let row: [f32; 9] = match crop {
        P::Grain => [1.0, 0.9, 0.85, 0.5, 0.6, 0.5, 0.3, 0.1, 0.5],
        P::Barley => [0.9, 0.8, 0.7, 0.4, 0.95, 0.9, 0.6, 0.3, 0.6],
        P::Oats => [0.85, 0.8, 0.9, 0.9, 0.5, 0.6, 0.75, 0.4, 0.7],
        P::Rye => [0.7, 0.6, 0.7, 0.5, 0.8, 0.85, 0.95, 0.7, 0.7],
        P::Peas => [0.8, 1.0, 0.7, 0.6, 0.5, 0.4, 0.3, 0.1, 0.4],
        P::Flax => [0.9, 0.9, 0.8, 0.9, 0.5, 0.5, 0.5, 0.2, 0.5],
        _ => return 1.0,
    };
    row[match biome {
        B::Grassland => 0,
        B::Meadow => 1,
        B::Woodland => 2,
        B::Fen => 3,
        B::Heath => 4,
        B::Stony => 5,
        B::Fell => 6,
        B::Snowfield => 7,
        B::BoneForest => 8,
    }]
}

/// How well any plant grows on the material beneath it, before a crop's own taste: the table `crop_ground_factor` starts from, shared with wild plants.
pub fn ground_factor(ground: Material) -> f32 {
    use Material as M;
    match ground {
        M::MEADOW | M::SOIL | M::GRASS => 1.0,
        M::PEAT | M::WET_SOIL => 1.05,
        M::MOSS | M::MUD => 0.9,
        M::CLAY | M::PACKED_EARTH => 0.8,
        M::SAND => 0.65,
        M::GRAVEL => 0.55,
        M::SCREE => 0.45,
        M::FROZEN_SOIL => 0.5,
        M::SNOW => 0.35,
        M::BONE_MOSS => 0.7,
        _ => 1.0,
    }
}

/// How well a crop grows in the material beneath its surface tile, 0..1.2.
/// Guards the local-material behavior in FIXLOG 2026-10-30, M6/#493.
///
/// Biome is the broad climate and ground family; this factor preserves the local material
/// differences the farmer can see and work: meadow and peat feed a crop better than loose sand,
/// scree or frozen ground. Materials not used as surface ground keep the neutral factor so this
/// helper remains safe for hand-built and older worlds.
pub fn crop_ground_factor(crop: PlantKind, ground: Material) -> f32 {
    use Material as M;
    let base: f32 = ground_factor(ground);
    match crop {
        PlantKind::Oats | PlantKind::Peas if matches!(ground, M::PEAT | M::WET_SOIL | M::MUD) => {
            (base * 1.05).min(1.2)
        }
        PlantKind::Barley | PlantKind::Rye if matches!(ground, M::SAND | M::GRAVEL) => {
            (base * 1.1).min(1.2)
        }
        _ => base,
    }
}

/// The food crops farmers choose among, in order of preference when two suit equally.
pub const FOOD_CROPS: [PlantKind; 5] = [
    PlantKind::Grain,
    PlantKind::Barley,
    PlantKind::Oats,
    PlantKind::Rye,
    PlantKind::Peas,
];

/// Crops that want water near (#286): away from it they grow at half the rate.
pub fn thirsty(crop: PlantKind) -> bool {
    matches!(crop, PlantKind::Peas | PlantKind::Flax)
}

/// Crops a killing frost does not take (#286): rye.
pub fn frost_hardy(crop: PlantKind) -> bool {
    crop == PlantKind::Rye
}

/// A crop's growth by season (#286): rye grows on into autumn, peas want the summer's warmth.
pub fn crop_season(crop: PlantKind, season: Season) -> f32 {
    match (crop, season) {
        (PlantKind::Rye, Season::Autumn) => 1.6,
        (PlantKind::Peas, Season::Spring) => 0.6,
        (PlantKind::Peas, Season::Summer) => 1.15,
        (PlantKind::Barley | PlantKind::Oats, Season::Spring) => 1.15,
        _ => 1.0,
    }
}

/// The food crop that does best on ground of `biome`, watered or not (first in `FOOD_CROPS` on
/// a tie).
pub fn best_crop(biome: crate::world::gen::biome::Biome, watered: bool) -> PlantKind {
    let mut best = (PlantKind::Grain, f32::MIN);
    for c in FOOD_CROPS {
        let s = crop_suits(c, biome) * if thirsty(c) && !watered { 0.5 } else { 1.0 };
        if s > best.1 {
            best = (c, s);
        }
    }
    best.0
}
