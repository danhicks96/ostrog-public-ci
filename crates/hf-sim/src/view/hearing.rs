//! Ephemeral observer receipts from the ordinary, capped direct-perception pass.
use std::{collections::BTreeMap, sync::Arc};

use crate::{
    agents::{Agent, Species},
    ids::{AgentId, CommunityId, TilePos},
    mind::Activity,
    systems::perceive::STRIPE,
    view::known,
    work::{DesignationId, Step},
    World,
};

/// A local awake person whose own work can be reported by the watched colony.
pub fn listener(w: &World, a: &Agent) -> bool {
    a.species == Species::Person
        && a.alive
        && !a.activity.is_asleep()
        && !a.denned
        && w.communities
            .first()
            .is_some_and(|c| a.community == Some(c.id))
        && !known::is_unfound(w, a.id)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cause {
    Animal(Species),
    Work(DesignationId),
}

fn cause(a: &Agent) -> Option<Cause> {
    if !a.alive || a.activity.is_asleep() || a.denned {
        return None;
    }
    if let Activity::Working { plan } = &a.activity {
        if let Some(Step::Work { designation, .. }) = plan.current() {
            return Some(Cause::Work(*designation));
        }
    }
    match a.species {
        Species::Deer
        | Species::Boar
        | Species::Wolf
        | Species::Hare
        | Species::Fox
        | Species::Crow
        | Species::Bear
        | Species::Duck => Some(Cause::Animal(a.species)),
        _ => None,
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Contact {
    community: CommunityId,
    listener: (usize, AgentId, TilePos),
    source: (usize, AgentId, TilePos),
    cause: Cause,
}
impl Contact {
    pub(crate) fn sample(w: &World, i: usize, j: usize) -> Option<Self> {
        let a = &w.agents[i];
        let b = &w.agents[j];
        let cause = cause(b)?;
        if known::is_unfound(w, b.id) {
            return None;
        }
        Some(Self {
            community: a.community?,
            listener: (i, a.id, a.pos),
            source: (j, b.id, b.pos),
            cause,
        })
    }
}

#[derive(Debug, Clone, Default)]
struct Batch {
    tick: u64,
    rows: Option<Arc<Vec<(u64, Contact)>>>,
}

#[derive(Debug, Clone, Default)]
pub struct Contacts {
    batches: [Batch; STRIPE as usize],
    serial: u64,
}

impl Contacts {
    pub(crate) fn begin_tick(&mut self, tick: u64) {
        self.batches[tick as usize % STRIPE as usize] = Batch { tick, rows: None };
    }
    pub(crate) fn record(&mut self, tick: u64, rows: Vec<Contact>) {
        if rows.is_empty() {
            return;
        }
        let batch = &mut self.batches[tick as usize % STRIPE as usize];
        let out = Arc::make_mut(batch.rows.get_or_insert_with(Default::default));
        for r in rows {
            self.serial += 1;
            out.push((self.serial, r));
        }
    }

    /// Returns a listener anchor and an opaque sampling token. Exact source information
    /// remains private. The caller can verify presentation-specific work eligibility.
    pub fn cues(
        &self,
        w: &World,
        z: i32,
        mut audible: impl FnMut(&Agent) -> bool,
    ) -> Vec<(u64, TilePos)> {
        let Some(c) = w.communities.first() else {
            return Vec::new();
        };
        let chart = w.overworld.surveys.get(&c.id);
        let mut chosen = BTreeMap::new();
        for batch in &self.batches {
            if w.meta
                .tick
                .checked_sub(batch.tick)
                .is_none_or(|age| age >= u64::from(STRIPE))
            {
                continue;
            }
            for (token, r) in batch.rows.iter().flat_map(|v| v.iter()) {
                let (i, id, anchor) = r.listener;
                let (j, source, p) = r.source;
                if r.community != c.id || anchor.z != z || p.z != z {
                    continue;
                }
                let Some(a) = w
                    .agents
                    .get(i)
                    .filter(|a| a.id == id && a.pos == anchor && listener(w, a))
                else {
                    continue;
                };
                let Some(b) = w
                    .agents
                    .get(j)
                    .filter(|b| b.id == source && b.pos == p && cause(b) == Some(r.cause))
                else {
                    continue;
                };
                if known::is_unfound(w, b.id)
                    || listener(w, b)
                    || p.dist(anchor) > a.species_def().hearing_radius()
                    || chart.is_some_and(|s| {
                        s.valley.get(&(p.x, p.y, z)).is_some_and(|r| r.0 == s.tick)
                    })
                    || !audible(b)
                {
                    continue;
                }
                // A source heard by several people contributes one cue through a stable listener.
                let row = (id, *token, anchor);
                chosen
                    .entry(source)
                    .and_modify(|old: &mut (AgentId, u64, TilePos)| {
                        if id < old.0 || (id == old.0 && *token > old.1) {
                            *old = row;
                        }
                    })
                    .or_insert(row);
            }
        }
        let mut rows: Vec<_> = chosen
            .into_values()
            .map(|(_, token, anchor)| (token, anchor))
            .collect();
        rows.sort_by_key(|r| r.0);
        rows
    }
}
