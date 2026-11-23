//! Designations: community work orders. They carry their own material requirements and
//! delivered counts, a worker capacity, and progress. They are data; the settlement planner
//! creates them, people claim and fulfil them.

use serde::{Deserialize, Serialize};

use crate::ids::TilePos;
use crate::items::ItemKind;
use crate::structures::StructureKind;
use crate::world::Material;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct DesignationId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum DesignationKind {
    /// Build a wall tile of `material` here (consumes 1 log or 1 stone block).
    BuildWall(Material),
    /// Build a door in a wall position (wood).
    BuildDoor,
    /// Lay a built floor tile.
    BuildFloor(Material),
    /// Roof the tile here over (THATCH or WOOD shingles); it must be held by a wall
    /// (building.rs).
    BuildRoof(Material),
    /// Study a record or a tool from the valley's past (C13): a personal undertaking.
    Study(crate::ids::ItemId),
    /// Climb up and clear the snow off the roof over this tile (building.rs) before it gives.
    ClearRoof,
    /// Place a structure (bed, hearth, workbench, stockpile marker ...).
    BuildStructure(StructureKind, Material),
    /// Fell the tree at this tile → logs.
    Chop,
    /// Mine the solid tile here → open + material item.
    Mine,
    /// Carve stairs into the solid tile here (the tile above becomes the way down).
    DigStairs,
    /// Sow a crop on this tile.
    Sow,
    /// Harvest the mature crop here.
    Harvest,
    /// Carry one item stack from the ground (not in a stockpile) to a stockpile tile.
    Haul,
    /// Cook raw food into meals at a hearth.
    Cook,
    /// Hunt the animal with this id (its position moves; the planner re-paths).
    Hunt(crate::ids::AgentId),
    /// Make something at the workshop on this tile (see work::recipes).
    Craft(u8),
    /// Look after a hurt or sick person (bring a poultice, sit with them).
    Tend(crate::ids::AgentId),
    /// Fetch water and throw it on the fire burning on this tile.
    Douse,
    /// Beat out the fire on this tile with whatever is to hand (open ground and plants only).
    BeatOut,
    /// Appended (C34): dig clay from a wet bank, or sand, at this open tile; the ground stays.
    DigEarth,
    /// Appended (C35): carve an event of the record into the wall here (engrave.rs).
    Engrave,
    /// Appended (#299): fish the water at this tile from its bank.
    Fish,
    /// Appended (#316): set a snare on this tile.
    Snare,
    /// Appended (#285): tame this animal into the community's herd.
    Tame(crate::ids::AgentId),
    /// Appended (#433): fill a vessel at the water and carry it to the water store here.
    FetchWater,
    /// Appended (#483): build a staircase from this tile to the open tile immediately above.
    BuildStairs(Material),
    /// Appended (#325): dig one tile of an irrigation channel (channels.rs); the ground stays,
    /// the tile is recorded as a cut that carries water from the source.
    DigChannel,
    /// Appended (#286): pull the weed on this tile out by the root; nothing of it is kept.
    Uproot,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Requirement {
    pub kind: ItemKind,
    pub needed: u32,
    pub delivered: u32,
    /// What the first delivered units were made of (C32): planks keep their logs' wood, a bar
    /// its ore's metal, a table its timber.
    #[serde(default)]
    pub material: crate::world::Material,
    /// The material it must be (C33): a tin bar for bronze, not any bar. None: any.
    #[serde(default)]
    pub want: Option<crate::world::Material>,
    /// The mean quality of what was delivered (#233), so a dropped order gives back what it
    /// took. 0: unknown (an old save), read as middling.
    #[serde(default)]
    pub quality: f32,
}

impl Requirement {
    /// Whether a stack of this material will do.
    pub fn accepts(&self, m: crate::world::Material) -> bool {
        self.want.is_none_or(|w| w == m)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Designation {
    pub id: DesignationId,
    pub kind: DesignationKind,
    pub pos: TilePos,
    pub requirements: Vec<Requirement>,
    /// Work ticks remaining at skill 1.0 once materials are in place.
    pub work_left: u32,
    pub work_total: u32,
    pub max_workers: u8,
    pub priority: u8,
    pub created: u64,
    /// Retry bookkeeping: how often someone gave up on it.
    pub failures: u32,
    pub done: bool,
    /// Which community posted it (members work their own orders).
    #[serde(default)]
    pub community: Option<crate::ids::CommunityId>,
    /// Someone's own undertaking: only they take it (a fire they chose to fight, a patient they
    /// chose to tend). None for community orders.
    #[serde(default)]
    pub owner: Option<crate::ids::AgentId>,
}

/// Take out of `ds` the orders `keep` says no to, and hand them back (for `World::give_back`).
pub fn split_off(
    ds: &mut Vec<Designation>,
    mut keep: impl FnMut(&Designation) -> bool,
) -> Vec<Designation> {
    let mut gone = Vec::new();
    let mut i = 0;
    while i < ds.len() {
        if keep(&ds[i]) {
            i += 1;
        } else {
            gone.push(ds.remove(i));
        }
    }
    gone
}

impl Designation {
    pub fn materials_complete(&self) -> bool {
        self.requirements.iter().all(|r| r.delivered >= r.needed)
    }
    pub fn missing(&self) -> impl Iterator<Item = (ItemKind, u32)> + '_ {
        self.requirements
            .iter()
            .filter(|r| r.delivered < r.needed)
            .map(|r| (r.kind, r.needed - r.delivered))
    }
    /// What is still to be brought, with the material it must be (C33).
    pub fn missing_wants(
        &self,
    ) -> impl Iterator<Item = (ItemKind, u32, Option<crate::world::Material>)> + '_ {
        self.requirements
            .iter()
            .filter(|r| r.delivered < r.needed)
            .map(|r| (r.kind, r.needed - r.delivered, r.want))
    }
    pub fn label(&self) -> String {
        match self.kind {
            DesignationKind::BuildWall(_) => "wall".into(),
            DesignationKind::BuildDoor => "door".into(),
            DesignationKind::BuildStairs(_) => "stairs".into(),
            DesignationKind::BuildFloor(_) => "floor".into(),
            DesignationKind::BuildRoof(_) => "roof".into(),
            DesignationKind::ClearRoof => "clearing snow".into(),
            DesignationKind::Study(_) => "study".into(),
            DesignationKind::BuildStructure(k, _) => format!("{k:?}").to_lowercase(),
            DesignationKind::Chop => "felling".into(),
            DesignationKind::Mine => "mining".into(),
            DesignationKind::DigStairs => "shaft".into(),
            DesignationKind::Sow => "sowing".into(),
            DesignationKind::Harvest => "harvest".into(),
            DesignationKind::DigEarth => "clay pit".into(),
            DesignationKind::DigChannel => "channel".into(),
            DesignationKind::Engrave => "carving".into(),
            DesignationKind::Haul => "hauling".into(),
            DesignationKind::Cook => "cooking".into(),
            DesignationKind::Hunt(_) => "hunting".into(),
            DesignationKind::Fish => "fishing".into(),
            DesignationKind::Snare => "setting a snare".into(),
            DesignationKind::Tame(_) => "taming".into(),
            DesignationKind::Craft(r) => crate::work::recipes::recipe(r).label.into(),
            DesignationKind::Tend(_) => "tending".into(),
            DesignationKind::Douse => "dousing".into(),
            DesignationKind::BeatOut => "beating out the fire".into(),
            DesignationKind::FetchWater => "fetching water".into(),
            DesignationKind::Uproot => "pulling weeds".into(),
        }
    }
}

/// Work times at skill 1.0 (docs/TIME.md "Work durations").
pub mod times {
    use crate::time::TICKS_PER_MINUTE;
    pub const WALL: u32 = 20 * TICKS_PER_MINUTE as u32;
    pub const DOOR: u32 = 25 * TICKS_PER_MINUTE as u32;
    pub const BUILD_STAIRS: u32 = 40 * TICKS_PER_MINUTE as u32;
    pub const FLOOR: u32 = 8 * TICKS_PER_MINUTE as u32;
    pub const ROOF: u32 = 15 * TICKS_PER_MINUTE as u32;
    pub const CLEAR_ROOF: u32 = 10 * TICKS_PER_MINUTE as u32;
    pub const STUDY: u32 = 120 * TICKS_PER_MINUTE as u32;
    pub const CHOP: u32 = 25 * TICKS_PER_MINUTE as u32;
    pub const MINE: u32 = 15 * TICKS_PER_MINUTE as u32;
    pub const DIG_STAIRS: u32 = 40 * TICKS_PER_MINUTE as u32;
    pub const SOW: u32 = 5 * TICKS_PER_MINUTE as u32;
    pub const HARVEST: u32 = 5 * TICKS_PER_MINUTE as u32;
    pub const DIG_EARTH: u32 = 20 * TICKS_PER_MINUTE as u32;
    /// One tile of a ditch, cut a spade deep and a spade wide (#325).
    pub const DIG_CHANNEL: u32 = 30 * TICKS_PER_MINUTE as u32;
    pub const ENGRAVE: u32 = 180 * TICKS_PER_MINUTE as u32;
    pub const COOK: u32 = 15 * TICKS_PER_MINUTE as u32;
    pub const HUNT: u32 = 3 * TICKS_PER_MINUTE as u32;
    /// A morning at the water (#299).
    pub const FISH: u32 = 8 * TICKS_PER_MINUTE as u32;
    /// Setting a snare (#316).
    pub const SNARE: u32 = 2 * TICKS_PER_MINUTE as u32;
    pub const TEND: u32 = 10 * TICKS_PER_MINUTE as u32;
    pub const DOUSE: u32 = 2 * TICKS_PER_MINUTE as u32;
    /// Pouring a carried vessel into the store (#433).
    pub const POUR: u32 = 2 * TICKS_PER_MINUTE as u32;
    /// Pulling a weed, root and all (#286).
    pub const UPROOT: u32 = 3 * TICKS_PER_MINUTE as u32;
    pub const BEAT: u32 = TICKS_PER_MINUTE as u32;
    pub fn structure(kind: crate::structures::StructureKind) -> u32 {
        use crate::structures::StructureKind as K;
        let minutes = match kind {
            K::Bed => 45,
            K::Hearth => 60,
            K::Workbench => 90,
            K::Stockpile => 2,
            K::Table | K::Chair | K::Stool | K::Chest | K::Shelf => 40,
            K::Well => 180,
            K::Lamp | K::FencePost => 15,
            K::CarpenterBench | K::Smelter | K::Kiln => 120,
            // A rough standing stone levered up out of the valley's own rock.
            K::WardStone => 150,
            K::Tent => 30,
            K::Nest => 60,
            // The tent's time: raised alone, the same day (#1081).
            K::LeanTo => 30,
            // A plinth and an upright, dressed and set with care.
            K::Shrine => 240,
            _ => 60,
        };
        minutes * TICKS_PER_MINUTE as u32
    }
}

/// Material cost of structures (what must be hauled to the site).
pub fn structure_cost(kind: StructureKind) -> Vec<(ItemKind, u32)> {
    use StructureKind as K;
    match kind {
        // Fittings are made at a bench and set in place (C67, docs/design-rules.md section 2).
        K::Bed => vec![(ItemKind::BedFrame, 1)],
        // A ring of stones as they came from the rock.
        K::Hearth => vec![(ItemKind::RoughStone, 4)],
        K::Workbench => vec![(ItemKind::Log, 4)],
        K::Table => vec![(ItemKind::Table, 1)],
        K::Chair | K::Stool => vec![(ItemKind::Stool, 1)],
        K::Chest => vec![(ItemKind::Chest, 1)],
        K::Shelf => vec![(ItemKind::Plank, 2)],
        K::Well => vec![(ItemKind::StoneBlock, 6)],
        K::Lamp => vec![(ItemKind::Plank, 1)],
        K::Support => vec![(ItemKind::Log, 1)],
        // Raised from the ground where it stands: work, not hauled stone.
        K::WardStone => vec![],
        K::PortalStone => vec![],
        K::FencePost => vec![(ItemKind::Log, 1)],
        K::Stockpile => vec![],
        K::CarpenterBench => vec![(ItemKind::Log, 3)],
        K::Smelter => vec![(ItemKind::StoneBlock, 6)],
        K::Kiln => vec![(ItemKind::StoneBlock, 4), (ItemKind::Clay, 2)],
        K::Anvil => vec![(ItemKind::StoneBlock, 3)],
        // A vessel from store, turned into the pot on the fire.
        K::CookingPot => vec![(ItemKind::Vessel, 1)],
        K::Loom => vec![(ItemKind::Plank, 3)],
        // Two poles and a hide; a nest is woven of three logs' worth of withies.
        K::Tent => vec![(ItemKind::Log, 2), (ItemKind::Hide, 1)],
        K::Nest => vec![(ItemKind::Log, 3)],
        // A person's own lean-to, grown tile by tile from what lies about (D35, D36, #1081):
        // two poled logs and two hides (thatch bundles would do as well, but structure_cost
        // has no "either" mechanism yet — judgement call, see FIXLOG 2026-11-16).
        K::LeanTo => vec![(ItemKind::Log, 2), (ItemKind::Hide, 2)],
        // Of stone; one of timber is costed in settlement.rs `make` (#297).
        K::Shrine => vec![(ItemKind::StoneBlock, 4)],
        _ => vec![(ItemKind::Log, 2)],
    }
}
