//! What the valley knows of the peoples beyond it (#349): built only from what its people saw.
//! A place is added when strangers from it are sighted, with what was seen (how many, armed or
//! not, from which side) and when it was last confirmed; where they live stays unknown until
//! someone of the valley has been there (the place's `known_by`). Nothing in the simulation
//! reads this: it is what the valley could tell a viewer.

use crate::events::{EventKind, Severity};
use crate::expedition::Edge;
use crate::World;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnownOther {
    /// The place they are of (overworld site id) and its name as they gave it.
    pub site: u32,
    pub name: String,
    pub first_seen: u64,
    pub last_seen: u64,
    /// How many came the largest time.
    pub most: u32,
    /// Whether any came armed (a host, or carrying arms).
    pub armed: bool,
    /// The side of the valley they came in by.
    pub from: Edge,
}

impl KnownOther {
    /// Whether the valley knows where they live (someone has been there), else only that they are.
    pub fn located(&self, w: &World, cid: crate::ids::CommunityId) -> bool {
        w.overworld
            .sites
            .iter()
            .any(|s| s.id == self.site && s.known_by.contains(&cid))
    }
}

/// The last tick's sightings, added to what the valley knows.
pub fn observe(w: &mut World) {
    let tick = w.meta.tick;
    let seen: Vec<(crate::ids::AgentId, u32, u32)> = w
        .last_events
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::StrangersSighted {
                agent, band, size, ..
            } => Some((agent, band, size)),
            _ => None,
        })
        .collect();
    for (seer, band, size) in seen {
        let Some(cid) = w
            .agents
            .iter()
            .find(|a| a.id == seer)
            .and_then(|a| a.community)
        else {
            continue;
        };
        let Some(b) = w.bands.iter().find(|b| b.id == band) else {
            continue;
        };
        let site = b.site;
        let name = b.site_name.clone();
        let members = b.members.clone();
        let Some(s) = w.overworld.sites.iter().find(|s| s.id == site) else {
            continue;
        };
        let from = s.edge;
        let armed = members.iter().any(|m| {
            w.agents
                .iter()
                .position(|a| a.id == *m)
                .is_some_and(|i| crate::systems::arms::edge(w, i).is_some())
        });
        let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) else {
            continue;
        };
        match c.known_others.iter_mut().find(|k| k.site == site) {
            Some(k) => {
                k.last_seen = tick;
                k.most = k.most.max(size);
                k.armed |= armed;
            }
            None => {
                c.known_others.push(KnownOther {
                    site,
                    name: name.clone(),
                    first_seen: tick,
                    last_seen: tick,
                    most: size,
                    armed,
                    from,
                });
                let at = c.home;
                w.events.push(
                    tick,
                    Some(at),
                    Severity::Notable,
                    EventKind::FirstWord {
                        community: cid,
                        people: name,
                    },
                );
            }
        }
    }
}
