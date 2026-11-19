//! Rivals (Dan, 2026-10-23: "a nemesis style or reputation system to go along with" gossip).
//!
//! A stranger who fights someone of the valley and lives remembers them. Their place keeps them
//! as a rival with a foe; when it sends people again, the rival comes with them, bolder each
//! time, and goes looking for that one person. Each meeting is on record and earns a name
//! ("who came back", "the Scarred", "Bane of Halgar"); it ends when one of them falls.

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, TilePos};
use crate::mind::Activity;
use crate::rng::{purpose, stream};
use crate::time::TICKS_PER_DAY;
use crate::World;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rival {
    pub id: u32,
    pub name: String,
    /// The place beyond the valley they come from.
    pub site: u32,
    /// The one of the valley they come back for.
    pub foe: AgentId,
    pub meetings: u32,
    /// Times they went home hurt.
    pub scars: u32,
    /// Times they left their foe hurt.
    pub wounded_foe: u32,
    pub title: String,
    pub alive: bool,
    /// Their body in the valley now, if they are here.
    pub agent: Option<AgentId>,
    pub last: u64,
}

fn idx(w: &World, id: AgentId) -> Option<usize> {
    w.agents.iter().position(|a| a.id == id && a.alive)
}

fn first_name(w: &World, id: AgentId) -> String {
    w.agents
        .iter()
        .chain(w.overworld.away.iter())
        .find(|a| a.id == id)
        .map(|a| a.name.split_whitespace().next().unwrap_or("").to_string())
        .unwrap_or_default()
}

fn title_of(w: &World, r: &Rival) -> String {
    if r.wounded_foe >= 1 {
        format!("Bane of {}", first_name(w, r.foe))
    } else if r.scars >= 2 {
        "the Twice-Scarred".to_string()
    } else if r.scars == 1 {
        "the Scarred".to_string()
    } else if r.meetings >= 3 {
        "who keeps coming back".to_string()
    } else if r.meetings == 2 {
        "who came back".to_string()
    } else {
        String::new()
    }
}

/// The raider `ri` and the settler `si` have just fought, out of band `site`.
pub fn fought(w: &mut World, si: usize, ri: usize, site: u32) {
    let tick = w.meta.tick;
    let (foe, raider) = (w.agents[si].id, w.agents[ri].id);
    let foe_hurt = w.agents[si].body.needs_care();
    let raider_hurt = w.agents[ri].body.needs_care();
    if !w.agents[ri].alive {
        return;
    }
    let existing = w.agents[ri].rival;
    let k = match existing.and_then(|id| w.rivals.iter().position(|r| r.id == id)) {
        Some(k) => k,
        None => {
            let id = w.rivals.iter().map(|r| r.id).max().unwrap_or(0) + 1;
            w.rivals.push(Rival {
                id,
                name: w.agents[ri].name.clone(),
                site,
                foe,
                meetings: 0,
                scars: 0,
                wounded_foe: 0,
                title: String::new(),
                alive: true,
                agent: Some(raider),
                last: 0,
            });
            w.agents[ri].rival = Some(id);
            w.rivals.len() - 1
        }
    };
    let first = w.rivals[k].meetings == 0;
    w.rivals[k].meetings += 1;
    w.rivals[k].last = tick;
    if raider_hurt {
        w.rivals[k].scars += 1;
    }
    if foe_hurt && w.rivals[k].foe == foe {
        w.rivals[k].wounded_foe += 1;
    }
    let t = title_of(w, &w.rivals[k]);
    w.rivals[k].title = t.clone();
    let at = w.agents[si].pos;
    let name = w.rivals[k].name.clone();
    let foe = w.rivals[k].foe;
    if first {
        w.events.push(
            tick,
            Some(at),
            Severity::Notable,
            EventKind::RivalMade { name, foe },
        );
    } else {
        w.events.push(
            tick,
            Some(at),
            Severity::Major,
            EventKind::RivalMet {
                name,
                title: t,
                foe,
                meetings: w.rivals[k].meetings,
            },
        );
    }
}

/// A band from `site` has just been made of `members`: a rival of that place, alive and not in
/// the valley, comes with them, as the first of them, bolder for each meeting.
pub fn joins_band(w: &mut World, site: u32, members: &[AgentId]) {
    let tick = w.meta.tick;
    let Some(k) = w.rivals.iter().position(|r| {
        r.site == site
            && r.alive
            && r.agent.is_none_or(|a| idx(w, a).is_none())
            && w.agents.iter().any(|a| a.id == r.foe && a.alive)
    }) else {
        return;
    };
    let Some(&m) = members.first() else { return };
    let Some(i) = idx(w, m) else { return };
    let r = w.rivals[k].clone();
    w.agents[i].name = r.name.clone();
    w.agents[i].rival = Some(r.id);
    let bold = 0.2 * r.meetings as f32;
    w.agents[i].beliefs.hold(
        crate::mind::beliefs::Topic::Violence,
        (0.4 + bold).min(1.0),
        0.8,
        crate::mind::beliefs::Source::Person(m),
        tick,
    );
    w.rivals[k].agent = Some(m);
    let at = w.agents[i].pos;
    w.events.push(
        tick,
        Some(at),
        Severity::Major,
        EventKind::RivalReturned {
            name: r.name,
            title: r.title,
            foe: r.foe,
        },
    );
}

/// Each band step: a rival in the valley goes looking for their foe, out of doors within
/// thirty tiles, and has it out with them, once a day at most.
pub fn seek(w: &mut World, members: &[AgentId], site: u32) {
    let tick = w.meta.tick;
    for &m in members {
        let Some(i) = idx(w, m) else { continue };
        let Some(rid) = w.agents[i].rival else {
            continue;
        };
        let Some(k) = w.rivals.iter().position(|r| r.id == rid) else {
            continue;
        };
        if tick.saturating_sub(w.rivals[k].last) < TICKS_PER_DAY || w.agents[i].body.needs_care() {
            continue;
        }
        let foe = w.rivals[k].foe;
        let Some(j) = idx(w, foe) else { continue };
        let (me, at): (TilePos, TilePos) = (w.agents[i].pos, w.agents[j].pos);
        if at.z != me.z || at.dist(me) > 30 || crate::systems::hazards::indoors(w, at) {
            continue;
        }
        if at.dist(me) > 1 {
            if !matches!(w.agents[i].activity, Activity::Going { to, .. } if to.dist(at) <= 1)
                && w.path_agent_to(i, at)
            {
                w.agents[i].activity = Activity::Going {
                    to: at,
                    then: Box::new(Activity::Loitering { ticks_left: 30 }),
                };
                w.agents[i].activity_ticks = 0;
            }
            continue;
        }
        let mut r = stream(w.meta.seed, &[purpose::THREATS, 0x4E3E, m.0 as u64, tick]);
        crate::systems::combat::blow(w, i, j, &mut r, "by an old enemy");
        crate::systems::combat::blow(w, j, i, &mut r, "fighting an old enemy");
        w.agents[i].fought_at = tick;
        w.agents[j].fought_at = tick;
        fought(w, j, i, site);
    }
}

/// The dead of the last tick: a rival who fell is on record, and done.
pub fn observe(w: &mut World) {
    let tick = w.meta.tick;
    let died: Vec<AgentId> = w
        .last_events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Died { agent, .. } => Some(*agent),
            _ => None,
        })
        .collect();
    for d in died {
        let Some(rid) = w.agents.iter().find(|a| a.id == d).and_then(|a| a.rival) else {
            continue;
        };
        let Some(k) = w.rivals.iter().position(|r| r.id == rid && r.alive) else {
            continue;
        };
        w.rivals[k].alive = false;
        let (name, title, foe, meetings) = (
            w.rivals[k].name.clone(),
            w.rivals[k].title.clone(),
            w.rivals[k].foe,
            w.rivals[k].meetings,
        );
        let at = w.agents.iter().find(|a| a.id == d).map(|a| a.pos);
        w.events.push(
            tick,
            at,
            Severity::Major,
            EventKind::RivalFell {
                name,
                title,
                foe,
                meetings,
            },
        );
    }
}
