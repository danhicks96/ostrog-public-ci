//! Communities: who belongs where, what the group has decided, how it holds together.
//! A community owns its settlement sites and its work orders; people contribute to it while
//! keeping their own needs. Newcomers are accepted by a norm; low cohesion splits a group.

use serde::{Deserialize, Serialize};

use crate::ids::{AgentId, CommunityId, TilePos};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Role {
    Forager,
    Builder,
    Farmer,
    Cook,
    Miner,
    Hauler,
    Healer,
}

pub const ROLES: [Role; 7] = [
    Role::Forager,
    Role::Builder,
    Role::Farmer,
    Role::Cook,
    Role::Miner,
    Role::Hauler,
    Role::Healer,
];

/// How newcomers are judged. Starts as "feed them if we can"; later phases let customs change it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum WelcomeNorm {
    /// Accept while there is at least a day of food per head.
    #[default]
    WhenFed,
    /// Accept anyone.
    Open,
    /// Accept nobody.
    Closed,
}

/// The floor a house was grown over and its doors (C67): its walls are whatever closes the
/// floor in (houses.rs).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HousePlan {
    pub floor: Vec<TilePos>,
    pub doors: Vec<TilePos>,
    /// The one upper sleeping level grown onto this house (#483). It is the full footprint at
    /// z+1, including `upper_stair`; the stair is not a floor order, but remains in the plan so
    /// the wall ring does not close the opening. Empty in old saves and houses without need.
    #[serde(default)]
    pub upper_floor: Vec<TilePos>,
    /// The lower tile of the material-backed flight into `upper_floor`.
    #[serde(default)]
    pub upper_stair: Option<TilePos>,
    /// Why this house chose to grow upward (#483): a recorded reason, not a prefab choice.
    /// Empty on old saves and houses that never needed an upper level.
    #[serde(default)]
    pub upper_reason: Option<UpperStoreyReason>,
}

/// The household's recorded reason for adding an upper storey (#483, WP-03 3.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpperStoreyReason {
    /// The site cannot carry another useful ground-floor room or house.
    CrampedSite,
    /// The household has grown beyond the standing ground-floor sleeping space.
    GrownHousehold,
    /// The community has an active, recorded custom favouring an upper room.
    CulturalPreference,
}

impl UpperStoreyReason {
    pub fn label(self) -> &'static str {
        match self {
            Self::CrampedSite => "a cramped site",
            Self::GrownHousehold => "a grown household",
            Self::CulturalPreference => "their cultural preference",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Community {
    pub id: CommunityId,
    pub name: String,
    pub home: TilePos,
    pub members: Vec<AgentId>,
    pub founded: u64,
    pub house_sites: Vec<TilePos>,
    /// How far each house in `house_sites` has come: 0 unknown (an old save), 1 marked out,
    /// 2 walls up, 3 roofed, 4 roofed once and lost some (building.rs).
    #[serde(default)]
    pub house_stage: Vec<u8>,
    /// Each house's floor and doors (C67, houses.rs), grown from what its household needed.
    /// A house without one (an old save) is the 7×6 at its site.
    #[serde(default)]
    pub house_plans: Vec<HousePlan>,
    pub field_site: Option<TilePos>,
    /// Further 6×4 plots added as the community grows (before C71; kept for old saves).
    #[serde(default)]
    pub extra_fields: Vec<TilePos>,
    /// The field's tiles, grown from need (C71): the first from a seed near home, more as the
    /// mouths grow. Empty in an old save until the plots are read into it.
    #[serde(default)]
    pub field_tiles: Vec<TilePos>,
    pub quarry_site: Option<TilePos>,
    pub welcome: WelcomeNorm,
    /// 0..1, recomputed daily from members' opinions of each other and their needs.
    pub cohesion: f32,
    /// Tick of the last trade with another community (for pacing).
    pub last_trade: u64,
    pub dissolved: bool,
    /// What the community knows it has found: (what, where, event).
    #[serde(default)]
    pub discoveries: Vec<(String, TilePos, crate::ids::EventId)>,
    /// 8×8 cells (x/8, y/8, z) any member has stood in and looked around: what the
    /// community has seen of the map, for the discovery-respecting view.
    #[serde(default)]
    pub explored: std::collections::BTreeSet<(i32, i32, i32)>,
    /// #274: the stones its people have cut, in the order they first cut them.
    #[serde(default)]
    pub stones: Vec<crate::world::Material>,
    /// #349: the peoples beyond the valley it knows of, by sighting (known.rs).
    #[serde(default)]
    pub known_others: Vec<crate::known::KnownOther>,
    /// What this people has come to be (systems/culture.rs identity): its customs and the
    /// convictions most of its members share, as last recorded.
    #[serde(default)]
    pub identity: Vec<String>,
    /// The day of the last hearth evening and who has told something there that evening.
    #[serde(default)]
    pub hearth_told: (u64, Vec<AgentId>),
    /// When strangers last threatened them (C33): seen near, stores raided, a fight. 0: never.
    #[serde(default)]
    pub threatened: u64,
    /// Rooms dug into the rock to live in (C57), each its mouth and the tiles behind it.
    #[serde(default)]
    pub hollows: Vec<Hollow>,
    /// What has gone hard for it lately (C60): (tick, what, the event), the last sixty days.
    #[serde(default)]
    pub hardships: Vec<(u64, String, crate::ids::EventId)>,
    /// Tick of the last proposal to move (C68, #208): the cooldown lives here, not on the
    /// capped project list.
    #[serde(default)]
    pub last_move_proposal: u64,
    /// How it ended, once nobody of it is left in the valley (C60): "died out", "left the valley
    /// for Ashby".
    #[serde(default)]
    pub ended: Option<(u64, String)>,
    /// #353: how it is faring (fate.rs): 0 thriving, 1 struggling, 2 failing, 3 collapsed.
    #[serde(default)]
    pub fate: u8,
    /// #353: the most it ever numbered.
    #[serde(default)]
    pub peak: u32,
    /// #353: its dead by cause, for the account of a failed valley.
    #[serde(default)]
    pub deaths: Vec<(String, u32)>,
    /// Where those who left the valley went (C60).
    #[serde(default)]
    pub left_for: Option<String>,
    /// C77: whom the community follows in each domain (justice.rs `Domain`), by the deference
    /// its people give: provisioning, protection, the spirit, and the social.
    #[serde(default)]
    pub leaders: [Option<AgentId>; 4],
    /// C77: accusations waiting on a judgement.
    #[serde(default)]
    pub accused: Vec<crate::justice::Accusation>,
    /// C77: offences judged against each person, and when (for repeat offenders and feuds).
    #[serde(default)]
    pub record: Vec<(AgentId, AgentId, u64)>,
    /// #1796: food found short in a household's chest, nobody watching, not yet found by a
    /// keeper who could lay blame for it. Capped at 4; a shortage older than ten days is
    /// dropped unfound (`justice::find_missing_food`).
    #[serde(default)]
    pub missing_food: Vec<crate::justice::MissingFood>,
    /// Workshops no place was found for, and until when they are not wanted (C69, #242).
    #[serde(default)]
    pub no_site: Vec<(crate::structures::StructureKind, u64)>,
    /// Drinks of water in the household's store (#433, water.rs), kept in a barrel at home.
    #[serde(default)]
    pub water: u32,
    /// Camps left behind and when (C69, #243): not moved back to for a hundred days.
    #[serde(default)]
    pub old_camps: Vec<(TilePos, u64)>,
    /// The crafts its members know (C70), and who knew each last: when the last who knows one
    /// dies or leaves, the record says the craft went with them.
    #[serde(default)]
    pub crafts: Vec<(crate::mind::Technique, String)>,
    /// Who was asked to learn what, and until when they are not asked again (C70).
    #[serde(default)]
    pub teaching: Vec<(crate::ids::AgentId, crate::mind::Technique, u64)>,
    /// The wall round the houses (C73), once proposed: its ring, its gate, and when it stood.
    #[serde(default)]
    pub palisade: Option<Palisade>,
    /// Crafts recorded lost and not known again since (C72, #259): known again is said.
    #[serde(default)]
    pub crafts_lost: Vec<crate::mind::Technique>,
    /// When each craft's loss was last said (C72): not again within a season.
    #[serde(default)]
    pub crafts_said: Vec<(crate::mind::Technique, u64)>,
    /// #405: the mine sunk below the quarry when what the community wants lies deeper
    /// (delving.rs).
    #[serde(default)]
    pub mine: Option<crate::delving::Mine>,
}

/// A wall round a community's houses and stores (C73), grown round what stood when it was
/// proposed: the ring's tiles, the gate, when it was begun and when it first stood whole.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Palisade {
    pub ring: Vec<TilePos>,
    pub gate: TilePos,
    pub begun: u64,
    pub stood: Option<u64>,
    /// Whether it is closed now (C73): every ring tile standing, the gate a door, and every
    /// other tile of the ring's outline impassable. Only a closed wall keeps raiders out.
    #[serde(default)]
    pub closed: bool,
}

/// A home cut into a rock face: the mouth (a door once the room is dug) and the room behind.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hollow {
    pub mouth: TilePos,
    pub room: Vec<TilePos>,
    /// Dug to keep the stores from raiders (C58): a store, not a bedroom.
    #[serde(default)]
    pub shelter: bool,
    /// When it was marked out (#163): a hollow not dug in twenty days is given up.
    #[serde(default)]
    pub begun: u64,
    /// Dug down from flat ground where there is no rock face (C67): `mouth` is the top of a
    /// stair shaft and `room[0]` its foot, so there is no door, and the shaft is dug, not mined.
    #[serde(default)]
    pub dug_down: bool,
}

impl Hollow {
    /// Where its door stands: the mouth of one cut into a face; for a shelter dug down, the
    /// room's second tile, beyond the stair's foot (#405); none for a dug-down home.
    pub fn door(&self) -> Option<TilePos> {
        match (self.dug_down, self.shelter) {
            (false, _) => Some(self.mouth),
            (true, true) => self.room.get(1).copied(),
            (true, false) => None,
        }
    }
}

/// A word for how close-knit a camp is, for the HUD: "close" from 0.75, "uneasy" below 0.55,
/// "divided" below 0.45 (the split line is 0.4); nothing in between.
pub fn cohesion_word(c: f32) -> Option<&'static str> {
    if c >= 0.75 {
        Some("close")
    } else if c < 0.45 {
        Some("divided")
    } else if c < 0.55 {
        Some("uneasy")
    } else {
        None
    }
}

impl Community {
    pub fn new(id: CommunityId, name: String, home: TilePos, founded: u64) -> Self {
        Community {
            id,
            name,
            home,
            members: Vec::new(),
            founded,
            house_sites: Vec::new(),
            house_stage: Vec::new(),
            house_plans: Vec::new(),
            field_site: None,
            extra_fields: Vec::new(),
            field_tiles: Vec::new(),
            quarry_site: None,
            welcome: WelcomeNorm::WhenFed,
            cohesion: 0.7,
            last_trade: 0,
            dissolved: false,
            discoveries: Vec::new(),
            explored: Default::default(),
            stones: Vec::new(),
            known_others: Vec::new(),
            identity: Vec::new(),
            hearth_told: (0, Vec::new()),
            threatened: 0,
            hollows: Vec::new(),
            hardships: Vec::new(),
            last_move_proposal: 0,
            no_site: Vec::new(),
            water: 0,
            old_camps: Vec::new(),
            crafts: Vec::new(),
            teaching: Vec::new(),
            crafts_lost: Vec::new(),
            palisade: None,
            crafts_said: Vec::new(),
            mine: None,
            ended: None,
            fate: 0,
            peak: 0,
            deaths: Vec::new(),
            left_for: None,
            leaders: [None; 4],
            accused: Vec::new(),
            record: Vec::new(),
            missing_food: Vec::new(),
        }
    }
}
