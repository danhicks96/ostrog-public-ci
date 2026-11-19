//! Personal projects (P0 of milestone MP, docs/design-peoples.md section 10). Some work exists
//! only because someone wants it: a carving of what they remember, ale because they hold that
//! drink eases things, the workshop of the craft they learned, a journey to a place they think
//! will pay. A person proposes it, with the belief, memory or past behind it; the settlement
//! planner then only finds the tiles and materials (systems/settlement.rs), and the caravan
//! forms around its proposer (systems/expedition.rs). Without someone who wants it, it is not
//! done, however full the store.

use serde::{Deserialize, Serialize};

use crate::expedition::Purpose;
use crate::ids::{AgentId, CommunityId, EventId};
use crate::structures::StructureKind;
use crate::work::DesignationId;
use crate::world_state::World;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ProjectKind {
    /// Carve an event of the record into a house wall.
    Carve,
    /// Brew ale or press wine for a season.
    Brew,
    /// Build the workshop of a craft.
    Workshop(StructureKind),
    /// Lead a caravan.
    Journey(Purpose),
    /// Make spears, after strangers have raided the store (C52).
    Arms,
    /// A home for those sleeping on the ground, in the form their kind builds (C57).
    Home(HomeForm),
    /// A shelter dug into the rock after strangers raided the store (C58): a room behind a
    /// barred door where the stores are kept.
    Shelter,
    /// A wall round the houses after strangers raided the store (C73): the ring grown round
    /// what stands, with one gate.
    Wall,
    /// Moving camp away from a failing site (C60): within the valley to `to`, or out of it
    /// (`to` None).
    Move { to: Option<crate::ids::TilePos> },
    /// Appended (#297): a shrine for a community that keeps its observances (shrines.rs).
    Shrine,
    /// Appended (#433): a well, for a community with no fresh water a short walk from home
    /// (wells.rs).
    Well,
    /// Appended (#325): a ditch from open water to a dry field (channels.rs).
    Channel,
}

/// The form a home takes: what the proposer's kind builds (`habitat.builds` in the species
/// file), a house where a hollow-building kind finds no rock to dig into.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HomeForm {
    House,
    Hollow,
    Tent,
    Nest,
}

impl HomeForm {
    /// From the species file's `habitat.builds`; anything unknown builds a house.
    pub fn of(builds: &str) -> HomeForm {
        match builds {
            "hollows" => HomeForm::Hollow,
            "tents" => HomeForm::Tent,
            "nests" => HomeForm::Nest,
            _ => HomeForm::House,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            HomeForm::House => "a house",
            HomeForm::Hollow => "a hollow in the rock",
            HomeForm::Tent => "tents",
            HomeForm::Nest => "nests",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: u32,
    pub by: AgentId,
    pub community: CommunityId,
    pub kind: ProjectKind,
    /// "remembers it: Aeleiric died on the road", "holds that drink eases things",
    /// "learned the craft from Brannoc, the smith".
    pub because: String,
    /// The row behind the reason, where there is one: the memory's event, the belief's first
    /// recorded source, the past's Lived row.
    pub source: Option<EventId>,
    pub proposed: u64,
    /// The order the planner posted for it, once it has.
    pub work: Option<DesignationId>,
    pub done: bool,
    /// Where it is being built, once the planner has found the place (a home's site, C57).
    #[serde(default)]
    pub site: Option<crate::ids::TilePos>,
}

/// Days a project waits for its work to be posted, or a brewing season lasts.
pub const PROJECT_DAYS: u64 = 5;

impl Project {
    pub fn open(&self) -> bool {
        !self.done
    }
}

/// The open project of this kind in this community, if any.
pub fn open_of(
    w: &World,
    cid: CommunityId,
    kind: impl Fn(&ProjectKind) -> bool,
) -> Option<&Project> {
    w.projects
        .iter()
        .find(|p| p.open() && p.community == cid && kind(&p.kind))
}

/// The open project of this kind in this community, mutably.
pub fn open_of_mut(
    w: &mut World,
    cid: CommunityId,
    kind: impl Fn(&ProjectKind) -> bool,
) -> Option<&mut Project> {
    w.projects
        .iter_mut()
        .find(|p| p.open() && p.community == cid && kind(&p.kind))
}

/// The hover line for an order someone wished for, at its tile: "Kerrow's wish: remembers
/// it: Tamsin fought Orl by the well".
pub fn note(w: &World, at: crate::ids::TilePos) -> Option<String> {
    let d = w.designations.iter().find(|d| d.pos == at && !d.done)?.id;
    let p = w.projects.iter().find(|p| p.work == Some(d))?;
    let who = w.agents.iter().find(|a| a.id == p.by)?.name.clone();
    Some(format!("{who}'s wish: {}", p.because))
}
