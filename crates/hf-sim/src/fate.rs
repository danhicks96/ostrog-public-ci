//! How a community is faring, and the account of a failed valley (#353: "losing is fun").
//!
//! Daily, each community is thriving, struggling (under three meals a head in store), failing
//! (half its peak gone, or under one a head), or collapsed (three quarters gone); a change is on
//! record with why. When one person is left, that is on record. A community that dies out ends
//! with an account of what killed it (moving.rs `endings`).

use crate::events::{EventKind, Severity};
use crate::ids::CommunityId;
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::World;

pub const WORDS: [&str; 4] = ["thriving", "struggling", "failing", "collapsed"];

/// The dead of the last tick, tallied by cause against their community.
pub fn observe(w: &mut World) {
    let died: Vec<(crate::ids::AgentId, String)> = w
        .last_events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Died { agent, cause } => Some((*agent, cause.clone())),
            _ => None,
        })
        .collect();
    for (who, cause) in died {
        let Some(cid) = w
            .agents
            .iter()
            .chain(w.overworld.away.iter())
            .find(|a| a.id == who && a.species == crate::agents::Species::Person)
            .and_then(|a| a.community)
        else {
            continue;
        };
        if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
            match c.deaths.iter_mut().find(|x| x.0 == cause) {
                Some(x) => x.1 += 1,
                None => c.deaths.push((cause, 1)),
            }
        }
    }
}

/// "11 of starvation, 5 froze, 3 of old age": the three greatest causes.
pub fn account(deaths: &[(String, u32)]) -> String {
    let mut d = deaths.to_vec();
    d.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    d.iter()
        .take(3)
        .map(|(c, n)| {
            let verb = c.split_whitespace().next().unwrap_or("");
            if verb.ends_with("ed") || verb == "froze" || verb == "fell" || verb == "slipped" {
                format!("{n} {c}")
            } else {
                format!("{n} of {c}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_DAY != 20 * TICKS_PER_HOUR + 7 {
        return;
    }
    let food: u32 = w
        .items
        .iter()
        .filter(|it| it.kind.is_food() && matches!(it.place, crate::items::ItemPlace::Ground(_)))
        .map(|it| it.count)
        .sum();
    let cids: Vec<CommunityId> = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .map(|c| c.id)
        .collect();
    for cid in cids {
        let members: Vec<crate::ids::AgentId> = w
            .agents
            .iter()
            .filter(|a| {
                a.alive && a.community == Some(cid) && a.species == crate::agents::Species::Person
            })
            .map(|a| a.id)
            .collect();
        let n = members.len() as u32;
        if n == 0 {
            continue;
        }
        let settled = w.since_arrival() >= 10 * TICKS_PER_DAY;
        let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
        let was_more = c.peak > 1;
        c.peak = c.peak.max(n);
        let per = food as f32 / n as f32;
        let (stage, why) = if c.peak >= 4 && n * 4 <= c.peak {
            (3, format!("{n} left of {}", c.peak))
        } else if c.peak >= 2 && n * 2 <= c.peak {
            (2, format!("{n} left of {}", c.peak))
        } else if per < 1.0 {
            (2, "almost nothing left to eat".to_string())
        } else if per < 3.0 {
            (1, "little in store".to_string())
        } else {
            (0, "fed and holding".to_string())
        };
        let name = c.name.clone();
        let home = c.home;
        if settled && stage != c.fate {
            c.fate = stage;
            w.events.push(
                tick,
                Some(home),
                if stage >= 2 {
                    Severity::Major
                } else {
                    Severity::Notable
                },
                EventKind::Faring {
                    community: name.clone(),
                    now: WORDS[stage as usize].to_string(),
                    why,
                },
            );
        }
        if n == 1 && was_more {
            let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
            if c.deaths.iter().all(|x| x.0 != "#last") {
                c.deaths.push(("#last".to_string(), 0));
                let at = w.agents.iter().find(|a| a.id == members[0]).map(|a| a.pos);
                w.events.push(
                    tick,
                    at,
                    Severity::Major,
                    EventKind::LastOne {
                        agent: members[0],
                        community: name,
                    },
                );
            }
        }
    }
}
