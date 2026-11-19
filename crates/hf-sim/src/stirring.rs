//! What stirs below (#455, docs/design-deep-ways.md): a beast of the deep woken by a breach comes
//! up because of what the valley does near the way, not on a clock.
//!
//! A breach that wakes a beast gives its cavern a stir. Each hour the stir rises by the noise
//! of the valley's people there: one for each in the cavern or within `EARSHOT` tiles of the
//! breach on its level, two more for each of them at noisy work (mining, digging, building). An
//! hour with nobody there takes one off. At `WARN` those there hear something move below, on
//! record, and keep a bad memory of the place; at `RISE` the beast comes up through the way if
//! it is open, or finds it walled and goes back down. Before this (C75, #273) it came up two days
//! after the breach whatever the valley did; saves from then keep their due day
//! (`Creatures::rising`).

use serde::{Deserialize, Serialize};

use crate::events::{EventKind, Severity};
use crate::mind::{Activity, MemKind, Subject};
use crate::work::DesignationKind;
use crate::world_state::World;

/// How far from the breach, on its level, the valley's noise carries down.
pub const EARSHOT: i32 = 6;
/// The stir at which those there hear something move below.
pub const WARN: u32 = 12;
/// The stir at which it comes up.
pub const RISE: u32 = 30;

/// A beast of the deep stirred by a breach, and how stirred it is.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stir {
    pub beast: String,
    /// The cavern's index in `grid.caverns`.
    pub cavern: usize,
    pub level: u32,
    pub warned: bool,
    pub since: u64,
}

fn noisy(k: DesignationKind) -> bool {
    matches!(
        k,
        DesignationKind::Mine
            | DesignationKind::DigStairs
            | DesignationKind::DigEarth
            | DesignationKind::DigChannel
            | DesignationKind::BuildWall(_)
            | DesignationKind::BuildStructure(..)
            | DesignationKind::BuildFloor(_)
    )
}

/// Hourly (underground.rs `step`).
pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if !tick.is_multiple_of(crate::time::TICKS_PER_HOUR) || w.creatures.stirring.is_empty() {
        return;
    }
    let mut n = 0;
    while n < w.creatures.stirring.len() {
        let s = w.creatures.stirring[n].clone();
        let Some(c) = w.grid.caverns.get(s.cavern).cloned() else {
            w.creatures.stirring.remove(n);
            continue;
        };
        let Some(b) = c.breach else {
            n += 1;
            continue;
        };
        let mut there = Vec::new();
        let mut noise = 0u32;
        for (i, a) in w.agents.iter().enumerate() {
            if !a.alive
                || a.species != crate::agents::Species::Person
                || a.community.is_none()
                || !(c.contains(a.pos) || (a.pos.z == b.z && a.pos.dist(b) <= EARSHOT))
            {
                continue;
            }
            noise += 1;
            if let Activity::Working { plan } = &a.activity {
                if w.designations
                    .iter()
                    .any(|d| d.id == plan.designation && noisy(d.kind))
                {
                    noise += 2;
                }
            }
            there.push(i);
        }
        let st = &mut w.creatures.stirring[n];
        st.level = if noise == 0 {
            st.level.saturating_sub(1)
        } else {
            st.level + noise
        };
        let (level, warned) = (st.level, st.warned);
        if level >= WARN && !warned {
            w.creatures.stirring[n].warned = true;
            let hearers: Vec<_> = there.iter().map(|&i| w.agents[i].id).collect();
            let ev = w.events.push(
                tick,
                Some(b),
                Severity::Notable,
                EventKind::StirBelow {
                    cavern: c.name.clone(),
                    heard: hearers,
                },
            );
            for &i in &there {
                w.agents[i].remember(
                    tick,
                    MemKind::Hardship,
                    Subject::Place(b),
                    -0.5,
                    0.6,
                    Some(ev),
                );
            }
        }
        if level < RISE {
            n += 1;
            continue;
        }
        if c.sealed {
            w.events.push(
                tick,
                Some(c.centre),
                Severity::Notable,
                EventKind::Barred {
                    beast: s.beast.clone(),
                    cavern: c.name.clone(),
                },
            );
            // It goes back down, as one barred on its day did (C75).
            w.creatures.stirring.remove(n);
            continue;
        }
        w.creatures.stirring.remove(n);
        crate::underground::come_up(w, &s.beast, s.cavern);
    }
}
