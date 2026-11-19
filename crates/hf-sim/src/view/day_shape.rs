//! The day's shape (#926): what the hours spent `loitering` and `working` actually were, by
//! hour of day. `Activity::name()` alone cannot say whether a loitering hour is the hearth
//! evening, the end of a purposeful walk, or a failed attempt at something else; the goal that
//! was last chosen for a person is read alongside it.
//!
//! Read-only (D7): no rng, no wall clock, no map with an unstable order, and nothing here
//! changes what anyone does, so no digest moves. Fed by a caller (the `day_shape` example, or a
//! test): `observe_decision` whenever a goal is chosen (`debuglog::decision`'s `chose`, read
//! from the agent id and the choice as a string), `observe_tick` once a tick for every agent
//! whose hour should be counted.

use std::collections::BTreeMap;

use crate::agents::Species;
use crate::ids::AgentId;
use crate::mind::Activity;
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::world_state::World;

/// What the run has shown so far, by hour of day (0-23): see the module doc.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DayShape {
    /// The latest goal chosen for a person (`observe_decision`'s `chose`), read into the label
    /// for their next `Loitering` or `Working` hour. Not cleared between hours: a person's goal
    /// stays "theirs" until a later decision replaces it.
    last_chosen: BTreeMap<AgentId, String>,
    /// Per hour of day: hours (ticks over `TICKS_PER_HOUR`) seen under each label.
    hours: [BTreeMap<String, f64>; 24],
}

impl DayShape {
    pub fn new() -> Self {
        Self::default()
    }

    /// A goal was chosen for `agent` (#926): read it as `chose`'s `Debug` string, or whatever
    /// label the caller passes; kept until the next call for the same agent.
    pub fn observe_decision(&mut self, agent: AgentId, chose: &str) {
        self.last_chosen.insert(agent, chose.to_string());
    }

    /// Sample this tick: every living person who belongs to a community is counted under the
    /// hour of day their tick falls in (`tick % TICKS_PER_DAY / TICKS_PER_HOUR`), by a label that
    /// is their activity's name, except for `Loitering` and `Working`, which are
    /// `"{activity}:{goal}"` with the goal last chosen for them (`"{activity}:unknown"` if none
    /// was).
    pub fn observe_tick(&mut self, w: &World) {
        let hour = ((w.meta.tick % TICKS_PER_DAY) / TICKS_PER_HOUR) as usize;
        for a in w
            .agents
            .iter()
            .filter(|a| a.alive && a.species == Species::Person && a.community.is_some())
        {
            let name = a.activity.name();
            let label = match a.activity {
                Activity::Loitering { .. } | Activity::Working { .. } => {
                    let goal = self
                        .last_chosen
                        .get(&a.id)
                        .map(String::as_str)
                        .unwrap_or("unknown");
                    format!("{name}:{goal}")
                }
                _ => name.to_string(),
            };
            *self.hours[hour].entry(label).or_default() += 1.0 / TICKS_PER_HOUR as f64;
        }
    }

    /// The 24 rows, one per hour of day, each the hours seen under every label that hour.
    pub fn report(&self) -> DayShapeReport {
        DayShapeReport {
            hours: self
                .hours
                .iter()
                .map(|m| m.iter().map(|(k, v)| (k.clone(), *v)).collect())
                .collect(),
        }
    }
}

/// `DayShape::report()`'s result: 24 rows, each a label's hours for that hour of day.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DayShapeReport {
    pub hours: Vec<Vec<(String, f64)>>,
}

impl DayShapeReport {
    /// The hours seen under `label` at `hour`; 0.0 if that hour never saw it.
    pub fn get(&self, hour: usize, label: &str) -> f64 {
        self.hours[hour]
            .iter()
            .find(|(k, _)| k == label)
            .map(|(_, v)| *v)
            .unwrap_or(0.0)
    }

    /// The hours seen at `hour` under every label, pooled.
    pub fn total(&self, hour: usize) -> f64 {
        self.hours[hour].iter().map(|(_, v)| v).sum()
    }
}
