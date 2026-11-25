//! Ostrog simulation core: deterministic, headless, engine-agnostic.

pub mod accounts;
pub mod agents;
pub mod aquifer;
pub mod arcs;
pub mod body;
pub mod building;
pub mod cascade;
pub mod cave_ecology;
pub mod cave_life;
pub mod channels;
pub mod chronicle;
pub mod conditions;
pub mod cover;
pub mod creatures;
pub mod culture;
pub mod debuglog;
pub mod deep;
pub mod deity;
pub mod delving;
pub mod digest;
pub mod duties;
pub mod ecology;
pub mod economy;
pub mod engine;
pub mod engrave;
pub mod environment;
pub mod events;
pub mod expedition;
pub mod faith;
pub mod fate;
pub mod feuds;
pub mod fish;
mod flight_recorder;
pub mod floods;
pub mod fmath;
pub mod fostering;
pub mod genetics;
pub mod grounds;
pub mod heresy;
pub mod houses;
pub mod ids;
pub mod items;
pub mod journeys;
pub mod justice;
pub mod keepers;
pub mod known;
pub mod left_sites;
pub mod legend;
pub mod livestock;
pub mod lore;
pub mod mind;
pub mod names;
pub mod nav;
pub mod nemesis;
pub mod observances;
pub mod omens;
pub mod opening;
pub mod par;
pub mod past;
pub mod paths;
pub mod pilgrimage;
pub mod plan;
pub mod plants;
pub mod project;
pub mod realm;
pub mod relics;
pub mod resonance;
pub mod rng;
pub mod save;
pub mod scenario;
pub mod setup;
pub mod shrines;
pub mod siege;
pub mod size;
pub mod social;
pub mod societies;
pub mod society;
pub mod species;
pub mod spells;
pub mod stirring;
pub mod structures;
pub mod systems;
pub mod table_census;
pub mod taken;
pub mod thermal;
pub mod thin;
pub mod threat;
pub mod tiers;
pub mod ties;
pub mod time;
pub mod trade;
pub mod traps;
pub mod underground;
pub mod view;
pub mod water;
pub mod wells;
pub mod why;
pub mod work;
pub mod world;
pub mod world_state;

pub use world_state::World;

/// Run a world with a chronicle attached, recording every tick's events.
pub struct Session {
    pub world: World,
    pub chronicle: chronicle::Chronicle,
    /// The newest event id already handed to the chronicle, so nothing is written twice: the
    /// events waiting for the first tick can be written early (`write_pending`), and a
    /// restored chronicle already holds everything up to its save.
    written: Option<u32>,
}

impl Session {
    pub fn new(world: World, chronicle: chronicle::Chronicle) -> Self {
        let written = chronicle.max_event_id().ok().flatten();
        Session {
            world,
            chronicle,
            written,
        }
    }

    /// Has this event yet to be handed to the chronicle.
    pub fn unwritten(&self, e: &events::Event) -> bool {
        self.written.is_none_or(|w| e.id.0 > w)
    }

    /// Write the events still waiting for the first tick (the valley's past, the arrival) now,
    /// so the record is whole before anything is stepped. Returns them, for the viewer. The
    /// world is untouched: its systems still see them after the next tick's flush, so a world
    /// run this way stays the same world.
    pub fn write_pending(&mut self) -> anyhow::Result<Vec<events::Event>> {
        let fresh: Vec<events::Event> = self
            .world
            .events
            .pending
            .iter()
            .filter(|e| self.unwritten(e))
            .cloned()
            .collect();
        if let Some(w) = write(&mut self.chronicle, &fresh)? {
            self.written = Some(w);
        }
        Ok(fresh)
    }

    pub fn step(&mut self) -> anyhow::Result<()> {
        self.world.step();
        let tick = self.world.tick();
        if tick > 0 && tick.is_multiple_of(time::TICKS_PER_YEAR) {
            // Year end: fold the year's counts into era_summary (derived from real rows).
            let from = tick - time::TICKS_PER_YEAR;
            for (kind, n) in self.chronicle.counts_between(from, tick)? {
                self.chronicle.write_era(from, tick, &kind, n)?;
            }
        }
        // Event ids only grow, so what was written early is a prefix.
        let k = self
            .world
            .last_events
            .partition_point(|e| self.written.is_some_and(|w| e.id.0 <= w));
        if let Some(w) = write(&mut self.chronicle, &self.world.last_events[k..])? {
            self.written = Some(w);
        }
        Ok(())
    }

    pub fn run(&mut self, ticks: u64) -> anyhow::Result<()> {
        for _ in 0..ticks {
            self.step()?;
        }
        Ok(())
    }
}

/// Record a batch, and the taught tier for customs (a community's telling of a real event).
/// Returns the newest event id written, if any.
fn write(c: &mut chronicle::Chronicle, events: &[events::Event]) -> anyhow::Result<Option<u32>> {
    if events.is_empty() {
        return Ok(None);
    }
    c.record(events)?;
    for e in events {
        match &e.kind {
            events::EventKind::CustomFormed {
                community,
                origin,
                narrative,
                ..
            } => {
                if let Some(hid) = c.history_id(*origin)? {
                    c.teach(hid, community.0 as i64, 1, narrative, 0.8)?;
                }
            }
            events::EventKind::CustomAdopted {
                community,
                origin,
                narrative,
                version,
                ..
            } => {
                if let Some(hid) = c.history_id(*origin)? {
                    c.teach(hid, community.0 as i64, *version as i64, narrative, 0.6)?;
                }
            }
            _ => {}
        }
    }
    Ok(events.iter().map(|e| e.id.0).max())
}
