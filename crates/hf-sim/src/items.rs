//! Items: discrete stacks that exist on the ground, in a stockpile slot, or carried.

use serde::{Deserialize, Serialize};

use crate::ids::{AgentId, ItemId, TilePos};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum ItemKind {
    Log,
    StoneBlock,
    IronOre,
    CopperOre,
    Coal,
    Berries,
    GrainSheaf,
    Root,
    RawMeat,
    Meal,
    Tool,
    Cloth,
    Hide,
    Firewood,
    Plank,
    CopperBar,
    IronBar,
    Fibre,
    Clothes,
    Poultice,
    Saw,
    Hammer,
    SledgeHammer,
    Chisel,
    Needle,
    Bucket,
    Torch,
    /// A carved slate or bark record from the valley's past (deep.rs): it can be studied.
    Record,
    /// Rough gems from the deep basalt.
    Gem,
    /// Appended (C32): ore of a metal with no kind of its own (tin, silver, gold); its
    /// material says which.
    Ore,
    /// Appended (C33): a bar of a metal with no kind of its own (tin, bronze, silver, gold).
    Bar,
    /// A spear of the metal it is made of (C33): carried when strangers threaten, used in a
    /// fight, worn down by it.
    Weapon,
    /// Mail or scale of its metal (C33): worn when strangers threaten; it takes the blows.
    Armour,
    /// Appended (C34): clay dug from a wet bank, for the kiln.
    Clay,
    /// Sand, for glass.
    Sand,
    /// A pot or bottle of its material (clay, glass): food kept in one lasts.
    Vessel,
    /// Tanned hide, of its beast.
    Leather,
    /// Ale brewed from grain.
    Ale,
    /// Wine from berries.
    Wine,
    /// A ring of its metal, set with a gem (C34): given at a pairing.
    Jewel,
    /// Appended (C35): a carved figure of its stone or wood, made in a fey mood.
    Figurine,
    // Appended (C67): the building chain (docs/design-rules.md section 2). Stone comes out of
    // the rock rough and is dressed into blocks at a bench; fittings are made at a bench as
    // things, carried to the house and set there.
    /// Stone as it comes out of the rock, of that rock.
    RoughStone,
    /// A door, made of planks at the carpenter's bench.
    Door,
    /// A bed's frame, of planks.
    BedFrame,
    Table,
    Stool,
    Chest,
    /// Reeds or grass bundled for a roof.
    Thatch,
    /// Appended (#286): dye plants cut for the loom; its material is the colour (woad-blue,
    /// madder-red, weld-yellow), and clothes dyed with it wear that colour.
    Dye,
}

/// What the fittings set in a house are made as (C67): the structure and the thing made at a
/// bench that becomes it.
pub const FITTINGS: [(crate::structures::StructureKind, ItemKind); 5] = [
    (crate::structures::StructureKind::Bed, ItemKind::BedFrame),
    (crate::structures::StructureKind::Table, ItemKind::Table),
    (crate::structures::StructureKind::Stool, ItemKind::Stool),
    (crate::structures::StructureKind::Chair, ItemKind::Stool),
    (crate::structures::StructureKind::Chest, ItemKind::Chest),
];

/// Raw food a cook can turn into meals, in the order the planner looks for it.
pub const COOKABLE: [ItemKind; 4] = [
    ItemKind::RawMeat,
    ItemKind::Root,
    ItemKind::GrainSheaf,
    ItemKind::Berries,
];

impl ItemKind {
    /// Plain words for feed and history text.
    pub fn label(self) -> &'static str {
        match self {
            ItemKind::Log => "logs",
            ItemKind::StoneBlock => "stone",
            ItemKind::IronOre => "iron ore",
            ItemKind::CopperOre => "copper ore",
            ItemKind::Coal => "coal",
            ItemKind::Berries => "berries",
            ItemKind::GrainSheaf => "grain",
            ItemKind::Root => "roots",
            ItemKind::RawMeat => "meat",
            ItemKind::Meal => "meals",
            ItemKind::Tool => "tools",
            ItemKind::Cloth => "cloth",
            ItemKind::Hide => "hides",
            ItemKind::Firewood => "firewood",
            ItemKind::Plank => "planks",
            ItemKind::CopperBar => "copper bars",
            ItemKind::IronBar => "iron bars",
            ItemKind::Fibre => "fibre",
            ItemKind::Clothes => "clothes",
            ItemKind::Poultice => "poultices",
            ItemKind::Saw => "saws",
            ItemKind::Hammer => "hammers",
            ItemKind::SledgeHammer => "sledgehammers",
            ItemKind::Chisel => "chisels",
            ItemKind::Needle => "needles",
            ItemKind::Bucket => "buckets",
            ItemKind::Torch => "torches",
            ItemKind::Record => "records",
            ItemKind::Gem => "rough gems",
            ItemKind::Ore => "ore",
            ItemKind::Bar => "bars",
            ItemKind::Weapon => "weapons",
            ItemKind::Armour => "armour",
            ItemKind::Clay => "clay",
            ItemKind::Sand => "sand",
            ItemKind::Vessel => "pots",
            ItemKind::Leather => "leather",
            ItemKind::Ale => "ale",
            ItemKind::Wine => "wine",
            ItemKind::Jewel => "rings",
            ItemKind::Figurine => "figures",
            ItemKind::RoughStone => "rough stone",
            ItemKind::Door => "doors",
            ItemKind::BedFrame => "bed frames",
            ItemKind::Table => "tables",
            ItemKind::Stool => "stools",
            ItemKind::Chest => "chests",
            ItemKind::Thatch => "thatch",
            ItemKind::Dye => "dyestuff",
        }
    }
    /// `label()`, but singular for a count of one (#863): "bed frame" rather than "1 bed
    /// frames". Mass nouns and words with no clean singular ("clothes", "armour", "ore")
    /// keep their plain form.
    pub fn label_for_count(self, count: u32) -> &'static str {
        if count != 1 {
            return self.label();
        }
        match self {
            ItemKind::Log => "log",
            ItemKind::Berries => "berry",
            ItemKind::Root => "root",
            ItemKind::Meal => "meal",
            ItemKind::Tool => "tool",
            ItemKind::Hide => "hide",
            ItemKind::Plank => "plank",
            ItemKind::CopperBar => "copper bar",
            ItemKind::IronBar => "iron bar",
            ItemKind::Poultice => "poultice",
            ItemKind::Saw => "saw",
            ItemKind::Hammer => "hammer",
            ItemKind::SledgeHammer => "sledgehammer",
            ItemKind::Chisel => "chisel",
            ItemKind::Needle => "needle",
            ItemKind::Bucket => "bucket",
            ItemKind::Torch => "torch",
            ItemKind::Record => "record",
            ItemKind::Gem => "rough gem",
            ItemKind::Bar => "bar",
            ItemKind::Weapon => "weapon",
            ItemKind::Vessel => "pot",
            ItemKind::Jewel => "ring",
            ItemKind::Figurine => "figure",
            ItemKind::Door => "door",
            ItemKind::BedFrame => "bed frame",
            ItemKind::Table => "table",
            ItemKind::Stool => "stool",
            ItemKind::Chest => "chest",
            other => other.label(),
        }
    }
    /// Quality 0..1 as a tier 0..5 (shown as marks on the icon, named in text).
    pub fn tier(quality: f32) -> u8 {
        (quality.clamp(0.0, 1.0) * 5.0).round() as u8
    }
    pub fn tier_name(tier: u8) -> &'static str {
        match tier {
            0 => "crude",
            1 => "poor",
            2 => "plain",
            3 => "good",
            4 => "fine",
            _ => "masterly",
        }
    }
    /// Days food lasts lying in the open in mild weather (None: it does not spoil).
    pub fn keeps_days(self) -> Option<f32> {
        match self {
            ItemKind::Berries => Some(4.0),
            ItemKind::RawMeat => Some(3.0),
            ItemKind::Meal => Some(5.0),
            ItemKind::Root => Some(20.0),
            ItemKind::GrainSheaf => Some(30.0),
            // An untanned hide goes bad in a season (C34); tanned, it keeps.
            ItemKind::Hide => Some(20.0),
            _ => None,
        }
    }

    pub fn is_food(self) -> bool {
        matches!(
            self,
            ItemKind::Berries
                | ItemKind::Root
                | ItemKind::Meal
                | ItemKind::GrainSheaf
                | ItemKind::RawMeat
        )
    }
    /// Raw food a cook can turn into a meal. Every place that decides what is cookable (the
    /// planner's pickup, the cook's work, the settlement's count of raw food) asks this.
    pub fn is_cookable(self) -> bool {
        COOKABLE.contains(&self)
    }
    /// Nutrition per unit in hours of hunger satisfied.
    pub fn nutrition_hours(self) -> f32 {
        match self {
            ItemKind::Berries => 1.5,
            ItemKind::Root => 2.5,
            ItemKind::GrainSheaf => 2.0,
            ItemKind::Meal => 6.0,
            ItemKind::RawMeat => 2.0,
            _ => 0.0,
        }
    }
    pub fn max_stack(self) -> u32 {
        match self {
            ItemKind::Tool => 1,
            ItemKind::Clothes => 5,
            ItemKind::Poultice => 10,
            ItemKind::Saw => 1,
            ItemKind::Hammer => 1,
            ItemKind::SledgeHammer => 1,
            ItemKind::Chisel => 1,
            ItemKind::Needle => 1,
            ItemKind::Bucket => 1,
            ItemKind::Meal => 10,
            ItemKind::Torch => 10,
            ItemKind::Record => 1,
            ItemKind::Weapon => 1,
            ItemKind::Armour => 1,
            ItemKind::Jewel => 1,
            ItemKind::Figurine => 1,
            ItemKind::Vessel => 10,
            ItemKind::Door | ItemKind::BedFrame | ItemKind::Table | ItemKind::Chest => 2,
            ItemKind::Stool => 4,
            _ => 25,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ItemPlace {
    Ground(TilePos),
    Carried(AgentId),
    /// A tool in someone's hand (legend.rs): theirs until it breaks or they die. Also a garment
    /// with a story that someone wears (life.rs), until a new set replaces it.
    Held(AgentId),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub kind: ItemKind,
    pub count: u32,
    pub place: ItemPlace,
    /// 0..1 quality (affects meal satisfaction, tool efficiency).
    pub quality: f32,
    /// What it is made of (the tile or ore it came from); AIR when it does not matter.
    #[serde(default = "air")]
    pub material: crate::world::Material,
    /// Who made it, when, what it did and who held it (legend.rs); tools and clothes only.
    #[serde(default)]
    pub story: Option<Box<crate::legend::Story>>,
    /// How far food has gone off, 0 fresh .. 1 rotting (systems/spoil.rs); 0 for everything
    /// that does not spoil.
    #[serde(default)]
    pub rot: f32,
}

fn air() -> crate::world::Material {
    crate::world::Material::AIR
}

/// A stack's name with what it is made of: "oak logs", "granite stone", "garnets", "tin ore".
/// Kinds whose label already names the material (iron ore, copper bars) and things of no
/// particular material keep their plain label.
pub fn stack_name(
    table: &crate::world::MaterialTable,
    kind: ItemKind,
    m: crate::world::Material,
) -> String {
    use crate::world::Material as M;
    if m == M::AIR {
        return kind.label().to_string();
    }
    let mat = &table.get(m).name;
    match kind {
        ItemKind::Gem => format!("{mat}s"),
        ItemKind::Ore => mat.clone(),
        ItemKind::IronOre
        | ItemKind::CopperOre
        | ItemKind::CopperBar
        | ItemKind::IronBar
        | ItemKind::Clay
        | ItemKind::Sand => kind.label().to_string(),
        ItemKind::StoneBlock => format!("{mat} blocks"),
        ItemKind::Hide => format!("{mat}s"),
        ItemKind::Weapon => format!("{mat} spears"),
        ItemKind::Armour => format!("{mat} armour"),
        ItemKind::Vessel if m == M::GLASS => "glass bottles".to_string(),
        ItemKind::Vessel => format!("{mat} pots"),
        ItemKind::Jewel => format!("{mat} rings"),
        ItemKind::Figurine => format!("{mat} figures"),
        ItemKind::Leather => format!("{mat} leather"),
        // A meal remembers what went into it (C34).
        ItemKind::Meal => match m {
            M::MEAT => "meals of stew".to_string(),
            M::ROOT_VEG => "meals of mashed roots".to_string(),
            M::GRAIN => "meals of porridge".to_string(),
            M::BERRY => "meals of berry cake".to_string(),
            _ => kind.label().to_string(),
        },
        _ => format!("{mat} {}", kind.label()),
    }
}

/// What lies loose on a tile, for the hover line: "Lying here: 12 oak logs, 4 granite blocks".
/// Only stacks the viewer has been told of (view/known.rs); storied things have their own note.
pub fn stack_note(w: &crate::world_state::World, p: TilePos) -> Option<String> {
    let parts: Vec<String> = w
        .items
        .iter()
        .filter(|it| {
            it.place == ItemPlace::Ground(p)
                && it.story.is_none()
                && crate::view::known::item_told(w, it)
        })
        .map(|it| {
            format!(
                "{} {}",
                it.count,
                stack_name(&w.grid.materials, it.kind, it.material)
            )
        })
        .collect();
    (!parts.is_empty()).then(|| format!("Lying here: {}", parts.join(", ")))
}

/// A material's name as a file-name suffix: lower case, words joined by underscores, other
/// marks dropped ("white hart's hide" -> "white_harts_hide").
pub fn slug(name: &str) -> String {
    name.chars()
        .filter_map(|c| match c {
            ' ' | '-' => Some('_'),
            c if c.is_ascii_alphanumeric() => Some(c.to_ascii_lowercase()),
            _ => None,
        })
        .collect()
}
