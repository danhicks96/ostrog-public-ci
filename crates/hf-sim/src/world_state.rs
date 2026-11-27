//! The world: plain data plus the tick entry point. No god methods; behaviour lives in
//! `systems/`. Everything that must persist is captured by `save::snapshot::Snapshot`.

use serde::{Deserialize, Serialize};

use crate::agents::Agent;
use crate::events::{EventKind, EventSink, Severity};
use crate::ids::{AgentId, ItemId, StructureId, TilePos};
use crate::items::Item;
use crate::plants::Plant;
use crate::structures::Structure;
use crate::work::{Designation, DesignationId, ReservationTable};
use crate::world::gen::{self, GenParams};
use crate::world::WorldGrid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldMeta {
    pub seed: u64,
    pub tick: u64,
    pub params: GenParams,
    pub founding_site: TilePos,
    pub name: String,
    /// Settlement plan anchors (chosen by the community planner).
    #[serde(default)]
    pub house_site: Option<TilePos>,
    #[serde(default)]
    pub field_site: Option<TilePos>,
    #[serde(default)]
    pub quarry_site: Option<TilePos>,
    /// The community planner posts work orders (tests may switch it off).
    #[serde(default = "default_true")]
    pub autoplan: bool,
    /// Harness only (C60): nobody proposes moving camp. The 500-person profile holds its party
    /// in place so it measures 500 people, not a valley its people have left.
    #[serde(default)]
    pub settled: bool,
    /// Harness only (#405): no mine is sunk (delving.rs), to measure what the mine costs.
    #[serde(default)]
    pub no_delving: bool,
    /// Transient: which community the planner is currently posting for.
    #[serde(skip)]
    pub planning_for: Option<crate::ids::CommunityId>,
    /// Life-cycle speed-up used by long-horizon tests (1.0 in play): aging and birth pacing.
    #[serde(default = "default_one")]
    pub life_speed: f32,
    /// Whoever is pacing this world (see `engine`). Fires no events of its own.
    #[serde(default)]
    pub engine: crate::engine::Engine,
    /// The tick the portal opened (decision D18). History before the arrival lives at ticks
    /// below it; the calendar people see counts from it. 0 for worlds with no arrival.
    #[serde(default)]
    pub epoch: u64,
    /// The tick the latest party arrived (OST-130): 0 means the first portal, so `epoch`.
    /// The "first days" graces count from it; the calendar still counts from `epoch`.
    #[serde(default)]
    pub arrived: u64,
    /// The new-game choices this world was made from (setup.rs); None for older saves and
    /// worlds made without an arrival.
    #[serde(default)]
    pub setup: Option<crate::setup::WorldSetup>,
    /// The limited wild-flora community (#486); None for saves from before the draw.
    #[serde(default)]
    pub flora_draw: Option<crate::world::gen::flora::FloraDraw>,
}

fn default_one() -> f32 {
    1.0
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug)]
pub struct World {
    pub meta: WorldMeta,
    pub grid: WorldGrid,
    pub plants: Vec<Plant>,
    pub agents: Vec<Agent>,
    pub structures: Vec<Structure>,
    pub items: Vec<Item>,
    pub designations: Vec<Designation>,
    pub reservations: ReservationTable,
    pub communities: Vec<crate::society::Community>,
    pub next_community_id: u32,
    pub customs: Vec<crate::culture::Custom>,
    pub next_custom_id: u32,
    /// The party that came through: ten, of whom three begin here (see `opening`).
    pub survivor_pool: Vec<crate::opening::SurvivorSlot>,
    /// Word from outside about the ones who are not here.
    pub rumours: Vec<crate::opening::Rumour>,
    /// Tiles burned out since the fires last all went out (C71, #258), for the record's "burned N".
    pub fire_tiles: u32,
    /// The camps communities left when they moved (C72): their houses stand and weather.
    pub left_sites: Vec<crate::left_sites::LeftSite>,
    /// Graves the colony's people dug for their dead (accounts.rs, #452).
    pub graves: Vec<crate::accounts::Grave>,
    /// Secret societies (societies.rs, #322).
    pub societies: Vec<crate::societies::Society>,
    /// Nights a way down was opened to let the deep's people up (societies.rs, #322).
    pub takings: Vec<crate::societies::Taking>,
    /// Children carried below, and the going after them (taken.rs, #322).
    pub gone_below: Vec<crate::taken::Taken>,
    /// Feuds between families (feuds.rs, #404).
    pub feuds: Vec<crate::feuds::Feud>,
    /// Spells over the valley (#278, spells.rs).
    pub spells: Vec<crate::spells::Spell>,
    /// Rivals of the valley from beyond it (nemesis.rs).
    pub rivals: Vec<crate::nemesis::Rival>,
    /// Fish in each stretch of water (fish.rs): (stock, as of tick), by 16-tile square.
    pub fish: std::collections::BTreeMap<(i32, i32), (f32, u64)>,
    /// Snares set (traps.rs, #316).
    pub traps: Vec<crate::traps::Trap>,
    /// Vermin outbreaks drawn by flood-rotted food (cascade.rs, #319), each holding the
    /// rows of its coming and of the rot that drew them.
    pub vermin: Vec<crate::cascade::Vermin>,
    /// The standing causes the later cascade families read (cascade.rs, #319): roofs burned
    /// off, herb lore lost, stores failed, the weakened.
    pub cascades: crate::cascade::Roots,
    /// Footsteps per surface column, fading (paths.rs, #84).
    pub wear: std::collections::BTreeMap<(i32, i32), u16>,
    /// Ground worn to a path, and what it was (paths.rs).
    pub paths: std::collections::BTreeMap<(i32, i32), crate::world::Material>,
    /// Water let in from the rock still moving, and how many tiles more it fills (aquifer.rs).
    pub seeping: Vec<(crate::ids::TilePos, u32)>,
    /// Who is under water, and for how many ticks (hazards.rs, #390).
    pub drowning: std::collections::BTreeMap<AgentId, u32>,
    /// The dead not yet laid to rest (C68, #220): someone awake goes to them first.
    pub rites_due: Vec<crate::systems::rites::RiteDue>,
    /// Who is out looking at this moment.
    pub searches: Vec<crate::opening::Search>,
    /// Tiles opened by digging (to tell a natural cave from a worked gallery).
    pub dug: std::collections::BTreeSet<TilePos>,
    /// Named sites beyond the map edge and the caravans going to them.
    pub overworld: crate::expedition::Overworld,
    /// Burning tiles, flooded tiles, and the rain and dry counters that drive the water.
    pub fires: Vec<crate::systems::hazards::Fire>,
    pub flooded: std::collections::BTreeSet<TilePos>,
    /// The valley's bodies of water, their catchments and how high they stand (floods.rs, #393).
    pub waters: crate::floods::Waters,
    /// Where fire burned (the ground under it) and when: ash for a while, then grown over.
    pub scorched: std::collections::BTreeMap<TilePos, u64>,
    /// Bands of outsiders in the valley (threat.rs).
    pub bands: Vec<crate::threat::Band>,
    pub next_band_id: u32,
    /// The deity's answers, active and recently ended (deity.rs), and what it has noticed.
    pub deity_answers: Vec<crate::deity::Answer>,
    /// The thin place at the portal stone (thin.rs).
    pub thin: crate::thin::ThinState,
    /// Where the next tick's decisions begin (decide.rs round robin).
    pub decide_cursor: usize,
    pub deity_watch: crate::deity::Watch,
    /// Signs read and waiting to be borne out, observances kept (faith.rs, #297).
    pub faith: crate::faith::Faith,
    /// The world the valley is part of: its peoples, settlements, figures and realms (M3).
    pub realm: crate::realm::sites::Realm,
    /// The realm's region map, regenerated from the seed when the world first needs it after
    /// a load (never saved).
    pub realm_map: Option<crate::realm::RealmMap>,
    /// What rotted today, by kind, for the day's line (systems/spoil.rs).
    pub rotted_today: std::collections::BTreeMap<String, u32>,
    /// What the settlement is going through now (arcs.rs).
    pub arc: crate::arcs::ArcState,
    /// Roofs over walkable tiles (building.rs), keyed by the tile they cover.
    pub roofs: std::collections::BTreeMap<crate::ids::TilePos, crate::building::Roof>,
    /// Walls carved with an event of the record (engrave.rs, C35), keyed by the wall tile.
    pub engravings: std::collections::BTreeMap<crate::ids::TilePos, crate::engrave::Engraving>,
    /// What a built wall, door or floor was made of (C56: an oak wall, a granite floor), keyed by
    /// its tile. The tile itself stays timber or stone to everything that asks; an entry counts
    /// only while the tile is still a built one, and a rebuild overwrites it.
    pub built_of: std::collections::BTreeMap<crate::ids::TilePos, crate::world::Material>,
    /// The species generated for this world (P6), by index from species::GENERATED.
    pub generated: Vec<&'static crate::species::SpeciesDef>,
    /// Work someone wants done, and why (project.rs, P0): open ones and the last fifty ended.
    pub projects: Vec<crate::project::Project>,
    pub next_project_id: u32,
    /// Ditches dug from open water to dry fields (channels.rs, #325), open and ended.
    pub channels: Vec<crate::channels::Channel>,
    /// The valley's past before the portal (deep.rs).
    pub history: crate::deep::History,
    /// Migrating ducks, carrion, beasts' records, the Hart's sightings (creatures.rs).
    pub creatures: crate::creatures::CreatureState,
    /// Light below ground: (where, radius in tiles, whether it is fire rather than fungus glow),
    /// rebuilt hourly (underground.rs). Saved (P4: perception reads it, and a resumed game must
    /// see the hour-old list an uninterrupted one would).
    pub lights: Vec<(TilePos, f32, bool)>,
    pub rain_hours: u32,
    pub dry_hours: u32,
    /// Recent major events (id, tick, kind) kept for the culture pass.
    pub culture_log: Vec<(crate::ids::EventId, u64, crate::events::EventKind)>,
    /// Conservation ledger: items created and destroyed by cause (debug/soak metric).
    pub ledger: crate::economy::Ledger,
    pub events: EventSink,
    pub next_agent_id: u32,
    pub next_item_id: u32,
    pub next_structure_id: u32,
    pub next_designation_id: u32,
    // ---- transient caches (rebuilt on restore, never saved) ----
    /// Events drained this tick, kept for the client feed until it pulls them.
    pub last_events: Vec<crate::events::Event>,
    /// Weather state derived each tick (Phase 1: a simple seasonal/daily curve).
    pub weather: crate::systems::weather::Weather,
    /// Test knob: when set, the weather system uses this instead of the seeded sky.
    pub force_weather: Option<crate::systems::weather::Weather>,
    /// Navigation regions (rebuilt incrementally from chunk versions).
    pub nav: Option<crate::nav::Nav>,
    pub astar: Option<crate::nav::AStarScratch>,
    /// The person the viewer's Why? lens traces, and their recent decision cycles (#567).
    /// Never saved and never read by a system, so tracing cannot change the simulation.
    pub why: Option<crate::why::WhyTrace>,
    /// Optional client observation, excluded from saves and decision-making like the Why trace.
    pub watcher_hearing: Option<Box<crate::view::hearing::Contacts>>,
    /// The census behind #940: how often a chosen Talk, Tend, Forage or EatItem acted on
    /// someone (or a claim) the chooser could not see. Read-only tallying; never read by a
    /// system, so it cannot change the simulation.
    pub knowing: crate::view::knowing::Knowing,
}

impl World {
    /// The air temperature at `p` (#268): the valley reading lapsed by height above (or
    /// warmth by depth below) the founding site's ground — heights frost earlier and
    /// drain warmth faster, dug cellars keep warmer than the surface reading.
    pub fn temp_at(&self, p: TilePos) -> f32 {
        let base = self
            .grid
            .ground_z(self.meta.founding_site.x, self.meta.founding_site.y);
        crate::thermal::lapse_c(self.weather.temperature_c, base, p.z)
    }

    /// A species of this world: a built-in one or one generated for it (P6).
    pub fn kind(&self, id: crate::species::SpeciesId) -> &'static crate::species::SpeciesDef {
        if id.0 >= crate::species::GENERATED {
            if let Some(d) = self
                .generated
                .get((id.0 - crate::species::GENERATED) as usize)
            {
                return d;
            }
        }
        crate::species::get(id)
    }

    /// Every species of this world, with its id: the built-in library, then the generated.
    pub fn kinds(
        &self,
    ) -> Vec<(
        crate::species::SpeciesId,
        &'static crate::species::SpeciesDef,
    )> {
        let mut out: Vec<_> = crate::species::library()
            .iter()
            .enumerate()
            .map(|(i, d)| (crate::species::SpeciesId(i as u16), d))
            .collect();
        out.extend(self.generated.iter().enumerate().map(|(i, d)| {
            (
                crate::species::SpeciesId(crate::species::GENERATED + i as u16),
                *d,
            )
        }));
        out
    }

    pub fn generate(seed: u64, params: GenParams) -> World {
        Self::generate_at(seed, params, 0)
    }

    /// A generated world whose clock starts at `epoch` (the arrival), so that history before
    /// it can be recorded at real ticks below it (D18).
    pub fn generate_at(seed: u64, params: GenParams, epoch: u64) -> World {
        let g = gen::generate(seed, &params);
        let meta = WorldMeta {
            seed,
            tick: epoch,
            params,
            founding_site: g.founding_site,
            name: crate::names::world_name(seed),
            house_site: None,
            field_site: None,
            quarry_site: None,
            autoplan: true,
            settled: false,
            no_delving: false,
            planning_for: None,
            life_speed: 1.0,
            engine: crate::engine::Engine::default(),
            epoch,
            arrived: 0,
            setup: None,
            flora_draw: Some(g.flora_draw),
        };
        let mut w = World::from_parts(meta, g.grid, g.plants, Vec::new());
        w.events.push(
            epoch,
            Some(g.founding_site),
            Severity::Major,
            EventKind::WorldCreated { seed },
        );
        w
    }

    pub fn from_parts(
        meta: WorldMeta,
        grid: WorldGrid,
        plants: Vec<Plant>,
        agents: Vec<Agent>,
    ) -> World {
        let meta_seed = meta.seed;
        let mut w = World {
            meta,
            grid,
            plants,
            agents,
            structures: Vec::new(),
            items: Vec::new(),
            designations: Vec::new(),
            reservations: ReservationTable::default(),
            communities: Vec::new(),
            next_community_id: 0,
            customs: Vec::new(),
            next_custom_id: 0,
            survivor_pool: crate::opening::pool(meta_seed, &[0; crate::opening::PARTY]),
            rumours: Vec::new(),
            searches: Vec::new(),
            rites_due: Vec::new(),
            fire_tiles: 0,
            left_sites: Vec::new(),
            graves: Vec::new(),
            societies: Vec::new(),
            takings: Vec::new(),
            gone_below: Vec::new(),
            feuds: Vec::new(),
            spells: Vec::new(),
            rivals: Vec::new(),
            fish: Default::default(),
            traps: Vec::new(),
            vermin: Vec::new(),
            cascades: Default::default(),
            wear: Default::default(),
            paths: Default::default(),
            seeping: Vec::new(),
            drowning: Default::default(),
            dug: Default::default(),
            overworld: crate::expedition::Overworld::generate(meta_seed),
            fires: Vec::new(),
            flooded: Default::default(),
            waters: Default::default(),
            scorched: Default::default(),
            bands: Vec::new(),
            next_band_id: 0,
            deity_answers: Vec::new(),
            thin: Default::default(),
            decide_cursor: 0,
            deity_watch: Default::default(),
            faith: Default::default(),
            realm: Default::default(),
            realm_map: None,
            rotted_today: Default::default(),
            arc: Default::default(),
            roofs: Default::default(),
            history: Default::default(),
            creatures: Default::default(),
            lights: Vec::new(),
            rain_hours: 0,
            dry_hours: 0,
            engravings: Default::default(),
            built_of: Default::default(),
            projects: Vec::new(),
            channels: Vec::new(),
            next_project_id: 0,
            generated: Vec::new(),
            culture_log: Vec::new(),
            ledger: Default::default(),
            events: EventSink::default(),
            next_agent_id: 0,
            next_item_id: 0,
            next_structure_id: 0,
            next_designation_id: 0,
            last_events: Vec::new(),
            weather: Default::default(),
            force_weather: None,
            nav: None,
            astar: None,
            why: None,
            watcher_hearing: None,
            knowing: Default::default(),
        };
        w.rebuild_caches();
        w
    }

    /// Ticks since the arrival (all of them, in a world with no arrival).
    /// What the built wall, door, floor or stair at `p` was made of (C56), while it is still
    /// one: `None` for rock, open ground, or a tile nobody built.
    pub fn made_of(&self, p: crate::ids::TilePos) -> Option<crate::world::Material> {
        use crate::world::{Material, Shape};
        if !self.grid.in_bounds(p) {
            return None;
        }
        let t = self.grid.get(p);
        let built_shape = matches!(
            t.shape,
            Shape::Wall | Shape::Floor | Shape::StairsUp | Shape::StairsDown | Shape::StairsBoth
        );
        if !built_shape || (t.material != Material::WOOD && t.material != Material::STONE_BLOCK) {
            return None;
        }
        self.built_of.get(&p).copied()
    }

    pub fn since_arrival(&self) -> u64 {
        self.meta.tick.saturating_sub(self.meta.epoch)
    }

    /// Ticks since the latest party arrived (OST-130): the first portal's, unless a later party
    /// has settled the valley. The newcomer grace, the first days no band knows of the valley,
    /// the deity's first day and fate's settled days count from here; years do not.
    pub fn since_party(&self) -> u64 {
        self.meta
            .tick
            .saturating_sub(self.meta.epoch.max(self.meta.arrived))
    }

    /// The calendar people live by: counted from the arrival. The epoch is a whole number of
    /// years, so seasons and days are the same either way; only the year differs.
    pub fn calendar(&self) -> crate::time::Calendar {
        crate::time::Calendar::from_tick(self.since_arrival())
    }

    pub fn rebuild_caches(&mut self) {
        self.weather = crate::systems::weather::current(self);
        for a in &mut self.agents {
            a.problems.repair_ids();
        }
        for c in self.grid.chunks.values_mut() {
            c.mark_all_dirty();
        }
        self.nav = None;
        self.astar = None;
    }

    /// Navigation, built on first use and kept current by the `nav` system.
    pub fn ensure_nav(&mut self) {
        if self.nav.is_none() {
            self.nav = Some(crate::nav::Nav::build(&self.grid));
        }
        if self.astar.is_none() {
            self.astar = Some(crate::nav::AStarScratch::new(&self.grid));
        }
    }

    /// Plan a path for an agent. Returns false if unreachable or over budget.
    pub fn path_agent_to(&mut self, agent_idx: usize, goal: TilePos) -> bool {
        self.ensure_nav();
        let start = self.agents[agent_idx].pos;
        if self
            .astar
            .as_ref()
            .is_some_and(|astar| astar.failed_path(start, goal))
        {
            return false;
        }
        let nav = self.nav.as_ref().expect("nav");
        if !nav.reach(start, goal) {
            self.astar
                .as_mut()
                .expect("astar")
                .remember_failed_path(start, goal);
            return false;
        }
        let result = {
            let scratch = self.astar.as_mut().expect("astar");
            crate::nav::find_path(
                &self.grid,
                scratch,
                start,
                goal,
                crate::nav::astar::DEFAULT_MAX_EXPANSIONS,
            )
        };
        match result {
            crate::nav::PathResult::Found(mut p) => {
                // Nobody on dry ground sets out through water over their head (#418, FIXLOG
                // 2026-10-27: on seed 2 people walked to bed and to meals through a stream in
                // spate and drowned). Someone already under it may go through it to get out.
                if !self.grid.over_head(start) && p.iter().any(|q| self.grid.over_head(*q)) {
                    return false;
                }
                p.reverse();
                p.pop(); // drop the start tile
                let a = &mut self.agents[agent_idx];
                a.path = p;
                a.path_blocked = false;
                true
            }
            _ => {
                self.astar
                    .as_mut()
                    .expect("astar")
                    .remember_failed_path(start, goal);
                false
            }
        }
    }

    pub fn tick(&self) -> u64 {
        self.meta.tick
    }
    pub fn seed(&self) -> u64 {
        self.meta.seed
    }

    pub fn alloc_agent_id(&mut self) -> AgentId {
        let id = AgentId(self.next_agent_id);
        self.next_agent_id += 1;
        id
    }
    pub fn alloc_item_id(&mut self) -> ItemId {
        let id = ItemId(self.next_item_id);
        self.next_item_id += 1;
        id
    }
    pub fn alloc_structure_id(&mut self) -> StructureId {
        let id = StructureId(self.next_structure_id);
        self.next_structure_id += 1;
        id
    }
    pub fn alloc_designation_id(&mut self) -> DesignationId {
        let id = DesignationId(self.next_designation_id);
        self.next_designation_id += 1;
        id
    }

    /// Create an item on the ground, recording it in the ledger.
    pub fn create_item(
        &mut self,
        kind: crate::items::ItemKind,
        count: u32,
        place: crate::items::ItemPlace,
        quality: f32,
        cause: &'static str,
    ) -> ItemId {
        self.create_item_of(
            kind,
            crate::world::Material::AIR,
            count,
            place,
            quality,
            cause,
        )
    }

    /// Create an item of a material (oak logs, granite blocks), recording it in the ledger.
    pub fn create_item_of(
        &mut self,
        kind: crate::items::ItemKind,
        material: crate::world::Material,
        count: u32,
        place: crate::items::ItemPlace,
        quality: f32,
        cause: &'static str,
    ) -> ItemId {
        // Loose stacks of one kind and material on one tile merge (the 600-day soak reached
        // 3,000 stacks of harvest dropped six at a time; every pass over the items paid for
        // it). A thing with a story is never one of a stack (legend.rs): it is that one thing.
        if let crate::items::ItemPlace::Ground(_) = place {
            let cap = kind.max_stack();
            if let Some(it) = self.items.iter_mut().find(|it| {
                it.kind == kind
                    && it.material == material
                    && it.place == place
                    && it.count + count <= cap
                    && it.story.is_none()
            }) {
                self.ledger.created(kind, count, cause);
                let total = (it.count + count) as f32;
                it.quality = (it.quality * it.count as f32 + quality * count as f32) / total;
                // Fresh food added to an older stack: the stack is as far gone as its share.
                it.rot = it.rot * it.count as f32 / total;
                it.count += count;
                return it.id;
            }
        }
        let id = self.create_lone_item(kind, count, place, quality, cause);
        if let Some(it) = self.items.last_mut() {
            it.material = material;
        }
        id
    }

    /// Create an item that does not merge into a stack already there, recording it in the
    /// ledger: one about to be given a story (a trophy, a thing from home, a crafted tool or
    /// garment), which must not stand for a stack or vanish into one.
    pub fn create_lone_item(
        &mut self,
        kind: crate::items::ItemKind,
        count: u32,
        place: crate::items::ItemPlace,
        quality: f32,
        cause: &'static str,
    ) -> ItemId {
        self.ledger.created(kind, count, cause);
        let id = self.alloc_item_id();
        self.items.push(Item {
            id,
            kind,
            count,
            place,
            quality,
            material: crate::world::Material::AIR,
            story: None,
            rot: 0.0,
        });
        id
    }

    /// Whoever was paired to `id` (dead, or gone from the valley) is free again: everyone in the
    /// valley and everyone away on a journey, whether or not they still remember `id` (FIXLOG
    /// 2026-10-20 #175, C62: the partner was cleared only through a relationship entry, which
    /// a full list can evict, and never for the away or on leaving).
    pub fn unpair(&mut self, id: crate::ids::AgentId) {
        for a in self.agents.iter_mut().chain(self.overworld.away.iter_mut()) {
            if a.partner == Some(id) {
                a.partner = None;
            }
        }
    }

    /// Orders dropped before they were finished give back what had been delivered to them: it
    /// lies at the site again (ledger "returned"). A finished order has none left to give
    /// (systems/work.rs counts it used). FIXLOG 2026-10-20 #162, C62: dropped orders once ate
    /// their materials without a word.
    pub fn give_back(&mut self, dropped: Vec<crate::work::Designation>) {
        for d in dropped {
            // On open ground by the order (#233: a dropped Mine order's tile is rock), at
            // the quality that was brought.
            let at = self.open_ground_near(d.pos, 3).unwrap_or(d.pos);
            for r in &d.requirements {
                if r.delivered > 0 {
                    let q = if r.quality > 0.0 { r.quality } else { 0.5 };
                    self.create_item_of(
                        r.kind,
                        r.material,
                        r.delivered,
                        crate::items::ItemPlace::Ground(at),
                        q,
                        "returned",
                    );
                }
            }
        }
    }

    /// The nearest walkable tile to `p` within `reach` on its level (`p` itself first).
    pub fn open_ground_near(&self, p: TilePos, reach: i32) -> Option<TilePos> {
        if self.grid.walkable(p) {
            return Some(p);
        }
        let mut best: Option<(i32, TilePos)> = None;
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                let q = p.offset(dx, dy, 0);
                if self.grid.walkable(q) {
                    let d = dx.abs().max(dy.abs());
                    if best.is_none_or(|(bd, _)| d < bd) {
                        best = Some((d, q));
                    }
                }
            }
        }
        best.map(|(_, q)| q)
    }

    /// Remove `count` units from a stack (deleting it when empty), recording the ledger.
    pub fn consume_item(&mut self, id: ItemId, count: u32, cause: &'static str) -> bool {
        let Some(ix) = self.items.iter().position(|it| it.id == id) else {
            return false;
        };
        let it = &mut self.items[ix];
        let n = count.min(it.count);
        it.count -= n;
        let kind = it.kind;
        if it.count == 0 {
            self.items.remove(ix);
        }
        self.ledger.destroyed(kind, n, cause);
        n > 0
    }

    /// Advance one tick through the fixed pipeline.
    pub fn step(&mut self) {
        self.meta.tick += 1;
        crate::systems::run_pipeline(self);
    }

    pub fn run(&mut self, ticks: u64) {
        for _ in 0..ticks {
            self.step();
        }
    }

    /// One tick with per-system timings (profiling).
    pub fn step_timed(&mut self, out: &mut [f64]) {
        self.meta.tick += 1;
        crate::systems::run_pipeline_timed(self, out);
    }
}
