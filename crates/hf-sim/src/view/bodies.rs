//! The dead, left where they fell for a while so a death is something to see (#185, QA
//! 2026-10-20, C63). The simulation removes nobody from `w.agents` at death but keeps no tick of
//! it; this read model takes the `Died` events as the client is told them and says, for each
//! person still lying there, which pose to draw: falling for a moment, then lying. A body laid
//! to rest by its people (`LaidToRest`, systems/rites.rs) is tended and taken away after a few
//! hours; one nobody laid to rest lies a day, where the crows come (creatures.rs).
//!
//! View only: nothing here is saved or read by the simulation, so a loaded game starts with no
//! bodies on the ground.

use crate::events::{Event, EventKind};
use crate::ids::{AgentId, TilePos};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};

/// The bridge's pose for a person falling (people sheet column `fall`).
pub const POSE_FALL: i32 = 8;
/// The bridge's pose for a person lying dead (people sheet column `lying`).
pub const POSE_LYING: i32 = 9;
/// How long the fall shows: a second and a half at 1x (docs/TIME.md: 20 ticks a second).
pub const FALL_TICKS: u64 = 30;
/// A body its people laid to rest lies three game-hours after the rite (C68: the rite is
/// given when someone has come to it, not on the tick of the death).
pub const TENDED_TICKS: u64 = 3 * TICKS_PER_HOUR;
/// A body nobody laid to rest lies a game-day.
pub const LEFT_TICKS: u64 = TICKS_PER_DAY;

#[derive(Clone, Debug, PartialEq)]
pub struct Body {
    pub agent: AgentId,
    pub at: Option<TilePos>,
    pub since: u64,
    pub tended: bool,
    /// The tick of the rite, once given.
    pub tended_at: Option<u64>,
}

#[derive(Clone, Debug, Default)]
pub struct Bodies {
    pub list: Vec<Body>,
}

impl Bodies {
    /// Take in events the viewer has been told, oldest first.
    pub fn observe(&mut self, events: &[Event]) {
        for e in events {
            match &e.kind {
                EventKind::Died { agent, .. } => {
                    self.list.retain(|b| b.agent != *agent);
                    // A death with no place died away from the valley (#566): no body here.
                    if e.at.is_none() {
                        continue;
                    }
                    self.list.push(Body {
                        agent: *agent,
                        at: e.at,
                        since: e.tick,
                        tended: false,
                        tended_at: None,
                    });
                }
                EventKind::LaidToRest { agent, .. } => {
                    if let Some(b) = self.list.iter_mut().find(|b| b.agent == *agent) {
                        b.tended = true;
                        b.tended_at = Some(e.tick);
                    }
                }
                _ => {}
            }
        }
    }

    /// Forget the bodies that have been taken away by `tick`.
    pub fn prune(&mut self, tick: u64) {
        self.list.retain(|b| Self::lies(b, tick));
    }

    /// The pose to draw `agent` in at `tick`, if their body is still on the ground.
    pub fn pose(&self, agent: AgentId, tick: u64) -> Option<i32> {
        let b = self.list.iter().find(|b| b.agent == agent)?;
        if !Self::lies(b, tick) {
            return None;
        }
        Some(if tick.saturating_sub(b.since) < FALL_TICKS {
            POSE_FALL
        } else {
            POSE_LYING
        })
    }

    pub fn clear(&mut self) {
        self.list.clear();
    }

    fn lies(b: &Body, tick: u64) -> bool {
        match b.tended_at {
            Some(t) if b.tended => tick.saturating_sub(t) < TENDED_TICKS,
            _ => tick.saturating_sub(b.since) < LEFT_TICKS,
        }
    }
}
