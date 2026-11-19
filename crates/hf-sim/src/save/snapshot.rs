//! The snapshot mirrors all persistent world state. Transient caches are rebuilt on restore.
//! Adding a field with `#[serde(default)]` is backward compatible (named MessagePack); removing,
//! renaming or changing the meaning of a field needs a `FORMAT_VERSION` bump and a migration
//! step with a fixture save for the old version.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::agents::Agent;
use crate::events::EventSink;
use crate::items::Item;
use crate::plants::Plant;
use crate::structures::Structure;
use crate::world::WorldGrid;
use crate::world_state::{World, WorldMeta};

/// Every field that is not a plain number borrows from the world when a save is made
/// (`capture`) and owns its value when one is read (`decode`): a save used to clone the whole
/// world first, and the clone's many small allocations, freed at once, left glibc holding
/// tens of megabytes it did not give back, more with every day of a long game (#431). A
/// `Cow` serializes exactly as the value it holds, so the bytes are unchanged.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot<'a> {
    pub meta: Cow<'a, WorldMeta>,
    pub grid: Cow<'a, WorldGrid>,
    pub plants: Cow<'a, Vec<Plant>>,
    pub agents: Cow<'a, Vec<Agent>>,
    pub structures: Cow<'a, Vec<Structure>>,
    pub items: Cow<'a, Vec<Item>>,
    pub designations: Cow<'a, Vec<crate::work::Designation>>,
    pub reservations: Cow<'a, crate::work::ReservationTable>,
    #[serde(default)]
    pub communities: Cow<'a, Vec<crate::society::Community>>,
    #[serde(default)]
    pub next_community_id: u32,
    #[serde(default)]
    pub customs: Cow<'a, Vec<crate::culture::Custom>>,
    #[serde(default)]
    pub next_custom_id: u32,
    #[serde(default)]
    pub dug: Cow<'a, std::collections::BTreeSet<crate::ids::TilePos>>,
    #[serde(default)]
    pub overworld: Option<Cow<'a, crate::expedition::Overworld>>,
    #[serde(default)]
    pub fires: Cow<'a, Vec<crate::systems::hazards::Fire>>,
    #[serde(default)]
    pub flooded: Cow<'a, std::collections::BTreeSet<crate::ids::TilePos>>,
    /// The waters' survey and how high each stands (#393).
    #[serde(default)]
    pub waters: Cow<'a, crate::floods::Waters>,
    #[serde(default)]
    pub scorched: Cow<'a, std::collections::BTreeMap<crate::ids::TilePos, u64>>,
    #[serde(default)]
    pub bands: Cow<'a, Vec<crate::threat::Band>>,
    #[serde(default)]
    pub next_band_id: u32,
    #[serde(default)]
    pub deity_answers: Cow<'a, Vec<crate::deity::Answer>>,
    #[serde(default)]
    pub thin: Cow<'a, crate::thin::ThinState>,
    #[serde(default)]
    pub decide_cursor: usize,
    #[serde(default)]
    pub deity_watch: Cow<'a, crate::deity::Watch>,
    #[serde(default)]
    pub realm: Cow<'a, crate::realm::sites::Realm>,
    #[serde(default)]
    pub rotted_today: Cow<'a, std::collections::BTreeMap<String, u32>>,
    #[serde(default)]
    pub arc: Cow<'a, crate::arcs::ArcState>,
    #[serde(default)]
    pub roofs: Cow<'a, std::collections::BTreeMap<crate::ids::TilePos, crate::building::Roof>>,
    /// Written true by every save since #159; a save without it is from before roofs existed.
    #[serde(default)]
    pub roofs_saved: bool,
    /// The events of the tick the save was made on (#158): the stages that run before the next
    /// flush read them (an heirloom on a death, carrion, the deity's and the arcs' memory), so a
    /// game saved on the tick of a death goes on as it would have.
    #[serde(default)]
    pub last_events: Cow<'a, Vec<crate::events::Event>>,
    #[serde(default)]
    pub engravings:
        Cow<'a, std::collections::BTreeMap<crate::ids::TilePos, crate::engrave::Engraving>>,
    #[serde(default)]
    pub built_of: Cow<'a, std::collections::BTreeMap<crate::ids::TilePos, crate::world::Material>>,
    #[serde(default)]
    pub projects: Cow<'a, Vec<crate::project::Project>>,
    /// The species generated for the world (P6), whole, so a save never depends on the
    /// generator's version to read its people.
    #[serde(default)]
    pub generated: Vec<crate::species::SpeciesDef>,
    #[serde(default)]
    pub lights: Cow<'a, Vec<(crate::ids::TilePos, f32, bool)>>,
    #[serde(default)]
    pub next_project_id: u32,
    #[serde(default)]
    pub channels: Cow<'a, Vec<crate::channels::Channel>>,
    #[serde(default)]
    pub history: Cow<'a, crate::deep::History>,
    #[serde(default)]
    pub creatures: Cow<'a, crate::creatures::CreatureState>,
    #[serde(default)]
    pub rain_hours: u32,
    #[serde(default)]
    pub dry_hours: u32,
    #[serde(default)]
    pub culture_log: Cow<'a, Vec<(crate::ids::EventId, u64, crate::events::EventKind)>>,
    /// The party that came through: the three who began and the missing seven.
    #[serde(default)]
    pub survivor_pool: Cow<'a, Vec<crate::opening::SurvivorSlot>>,
    #[serde(default)]
    pub rumours: Cow<'a, Vec<crate::opening::Rumour>>,
    #[serde(default)]
    pub searches: Cow<'a, Vec<crate::opening::Search>>,
    /// The dead waiting for their rite (C68).
    #[serde(default)]
    pub rites_due: Cow<'a, Vec<crate::systems::rites::RiteDue>>,
    /// Signs waiting to be borne out, observances kept (#297).
    #[serde(default)]
    pub faith: Cow<'a, crate::faith::Faith>,
    /// The camps left when communities moved (C72).
    #[serde(default)]
    pub left_sites: Cow<'a, Vec<crate::left_sites::LeftSite>>,
    #[serde(default)]
    pub graves: Cow<'a, Vec<crate::accounts::Grave>>,
    #[serde(default)]
    pub societies: Cow<'a, Vec<crate::societies::Society>>,
    #[serde(default)]
    pub takings: Cow<'a, Vec<crate::societies::Taking>>,
    #[serde(default)]
    pub gone_below: Cow<'a, Vec<crate::taken::Taken>>,
    #[serde(default)]
    pub feuds: Cow<'a, Vec<crate::feuds::Feud>>,
    /// Spells over the valley (#278).
    #[serde(default)]
    pub spells: Cow<'a, Vec<crate::spells::Spell>>,
    #[serde(default)]
    pub rivals: Cow<'a, Vec<crate::nemesis::Rival>>,
    #[serde(default)]
    pub fish: Cow<'a, std::collections::BTreeMap<(i32, i32), (f32, u64)>>,
    #[serde(default)]
    pub traps: Cow<'a, Vec<crate::traps::Trap>>,
    /// Vermin outbreaks drawn by flood-rotted food (cascade.rs, #319).
    #[serde(default)]
    pub vermin: Cow<'a, Vec<crate::cascade::Vermin>>,
    /// The later cascade families' standing causes (cascade.rs, #319).
    #[serde(default)]
    pub cascades: Cow<'a, crate::cascade::Roots>,
    #[serde(default)]
    pub wear: Cow<'a, std::collections::BTreeMap<(i32, i32), u16>>,
    #[serde(default)]
    pub paths: Cow<'a, std::collections::BTreeMap<(i32, i32), crate::world::Material>>,
    #[serde(default)]
    pub seeping: Cow<'a, Vec<(crate::ids::TilePos, u32)>>,
    #[serde(default)]
    pub drowning: Cow<'a, std::collections::BTreeMap<crate::ids::AgentId, u32>>,
    pub ledger: Cow<'a, crate::economy::Ledger>,
    pub events: Cow<'a, EventSink>,
    pub next_agent_id: u32,
    pub next_item_id: u32,
    pub next_structure_id: u32,
    pub next_designation_id: u32,
}

impl<'a> Snapshot<'a> {
    pub fn capture(w: &'a World) -> Snapshot<'a> {
        Snapshot {
            meta: Cow::Borrowed(&w.meta),
            grid: Cow::Borrowed(&w.grid),
            plants: Cow::Borrowed(&w.plants),
            agents: Cow::Borrowed(&w.agents),
            structures: Cow::Borrowed(&w.structures),
            items: Cow::Borrowed(&w.items),
            designations: Cow::Borrowed(&w.designations),
            reservations: Cow::Borrowed(&w.reservations),
            communities: Cow::Borrowed(&w.communities),
            next_community_id: w.next_community_id,
            customs: Cow::Borrowed(&w.customs),
            next_custom_id: w.next_custom_id,
            dug: Cow::Borrowed(&w.dug),
            overworld: Some(Cow::Borrowed(&w.overworld)),
            fires: Cow::Borrowed(&w.fires),
            flooded: Cow::Borrowed(&w.flooded),
            waters: Cow::Borrowed(&w.waters),
            scorched: Cow::Borrowed(&w.scorched),
            bands: Cow::Borrowed(&w.bands),
            next_band_id: w.next_band_id,
            deity_answers: Cow::Borrowed(&w.deity_answers),
            thin: Cow::Borrowed(&w.thin),
            decide_cursor: w.decide_cursor,
            deity_watch: Cow::Borrowed(&w.deity_watch),
            realm: Cow::Borrowed(&w.realm),
            rotted_today: Cow::Borrowed(&w.rotted_today),
            arc: Cow::Borrowed(&w.arc),
            roofs: Cow::Borrowed(&w.roofs),
            roofs_saved: true,
            last_events: Cow::Borrowed(&w.last_events),
            engravings: Cow::Borrowed(&w.engravings),
            built_of: Cow::Borrowed(&w.built_of),
            projects: Cow::Borrowed(&w.projects),
            generated: w.generated.iter().map(|d| (*d).clone()).collect(),
            lights: Cow::Borrowed(&w.lights),
            next_project_id: w.next_project_id,
            channels: Cow::Borrowed(&w.channels),
            history: Cow::Borrowed(&w.history),
            creatures: Cow::Borrowed(&w.creatures),
            rain_hours: w.rain_hours,
            dry_hours: w.dry_hours,
            culture_log: Cow::Borrowed(&w.culture_log),
            survivor_pool: Cow::Borrowed(&w.survivor_pool),
            rumours: Cow::Borrowed(&w.rumours),
            searches: Cow::Borrowed(&w.searches),
            rites_due: Cow::Borrowed(&w.rites_due),
            faith: Cow::Borrowed(&w.faith),
            left_sites: Cow::Borrowed(&w.left_sites),
            graves: Cow::Borrowed(&w.graves),
            societies: Cow::Borrowed(&w.societies),
            takings: Cow::Borrowed(&w.takings),
            gone_below: Cow::Borrowed(&w.gone_below),
            feuds: Cow::Borrowed(&w.feuds),
            spells: Cow::Borrowed(&w.spells),
            rivals: Cow::Borrowed(&w.rivals),
            fish: Cow::Borrowed(&w.fish),
            traps: Cow::Borrowed(&w.traps),
            vermin: Cow::Borrowed(&w.vermin),
            cascades: Cow::Borrowed(&w.cascades),
            wear: Cow::Borrowed(&w.wear),
            paths: Cow::Borrowed(&w.paths),
            seeping: Cow::Borrowed(&w.seeping),
            drowning: Cow::Borrowed(&w.drowning),
            ledger: Cow::Borrowed(&w.ledger),
            events: Cow::Borrowed(&w.events),
            next_agent_id: w.next_agent_id,
            next_item_id: w.next_item_id,
            next_structure_id: w.next_structure_id,
            next_designation_id: w.next_designation_id,
        }
    }

    pub fn restore(self) -> World {
        let mut w = World::from_parts(
            self.meta.into_owned(),
            self.grid.into_owned(),
            self.plants.into_owned(),
            self.agents.into_owned(),
        );
        w.grid.materials.refresh();
        w.structures = self.structures.into_owned();
        w.items = self.items.into_owned();
        w.designations = self.designations.into_owned();
        w.reservations = self.reservations.into_owned();
        w.communities = self.communities.into_owned();
        w.next_community_id = self.next_community_id;
        w.customs = self.customs.into_owned();
        w.next_custom_id = self.next_custom_id;
        w.dug = self.dug.into_owned();
        if let Some(o) = self.overworld {
            w.overworld = o.into_owned();
        }
        w.fires = self.fires.into_owned();
        w.flooded = self.flooded.into_owned();
        w.waters = self.waters.into_owned();
        w.scorched = self.scorched.into_owned();
        w.bands = self.bands.into_owned();
        w.next_band_id = self.next_band_id;
        w.deity_answers = self.deity_answers.into_owned();
        w.thin = self.thin.into_owned();
        w.decide_cursor = self.decide_cursor;
        w.deity_watch = self.deity_watch.into_owned();
        w.realm = self.realm.into_owned();
        w.rotted_today = self.rotted_today.into_owned();
        w.arc = self.arc.into_owned();
        w.roofs = self.roofs.into_owned();
        w.engravings = self.engravings.into_owned();
        w.built_of = self.built_of.into_owned();
        w.projects = self.projects.into_owned();
        w.channels = self.channels.into_owned();
        w.generated = self
            .generated
            .into_iter()
            .map(|d| &*Box::leak(Box::new(d)))
            .collect();
        for a in w.agents.iter_mut().chain(w.overworld.away.iter_mut()) {
            if a.people.0 >= crate::species::GENERATED {
                a.kind = w
                    .generated
                    .get((a.people.0 - crate::species::GENERATED) as usize)
                    .copied();
            }
        }
        w.lights = self.lights.into_owned();
        w.last_events = self.last_events.into_owned();
        w.next_project_id = self.next_project_id;
        w.history = self.history.into_owned();
        w.creatures = self.creatures.into_owned();
        // Only a save from before roofs existed gets its houses roofed (C10's upgrade). A save
        // that merely has no roof at the moment (walls just up, a roof burned) keeps none (#159).
        if !self.roofs_saved {
            crate::building::roof_old_houses(&mut w);
        }
        w.rain_hours = self.rain_hours;
        w.dry_hours = self.dry_hours;
        w.culture_log = self.culture_log.into_owned();
        w.survivor_pool = self.survivor_pool.into_owned();
        w.rumours = self.rumours.into_owned();
        w.searches = self.searches.into_owned();
        w.rites_due = self.rites_due.into_owned();
        w.faith = self.faith.into_owned();
        w.left_sites = self.left_sites.into_owned();
        w.graves = self.graves.into_owned();
        w.societies = self.societies.into_owned();
        w.takings = self.takings.into_owned();
        w.gone_below = self.gone_below.into_owned();
        w.feuds = self.feuds.into_owned();
        w.spells = self.spells.into_owned();
        w.rivals = self.rivals.into_owned();
        w.fish = self.fish.into_owned();
        w.traps = self.traps.into_owned();
        w.vermin = self.vermin.into_owned();
        w.cascades = self.cascades.into_owned();
        w.wear = self.wear.into_owned();
        w.paths = self.paths.into_owned();
        w.seeping = self.seeping.into_owned();
        w.drowning = self.drowning.into_owned();
        w.ledger = self.ledger.into_owned();
        w.events = self.events.into_owned();
        w.next_agent_id = self.next_agent_id;
        w.next_item_id = self.next_item_id;
        w.next_structure_id = self.next_structure_id;
        w.next_designation_id = self.next_designation_id;
        w.rebuild_caches();
        w.upgrade_old_opening();
        w
    }
}
