//! Engravings (C35). A carver cuts into a wall of a house a real event their people remember:
//! a death, a raid, strangers seen, a fire, a birth, an arrival, weightier events first. The event is chosen from the
//! world's record of its major events (culture.rs keeps the last two hundred), only one the
//! viewer may be told of, never one already carved, and preferably one the carver themselves
//! remembers; nothing is invented. The engraving keeps the event's id and its words as told
//! when it was carved, so hovering it tells the scene and the record can be followed back.

use serde::{Deserialize, Serialize};

use crate::events::{Event, EventKind, Severity};
use crate::ids::{AgentId, EventId, TilePos};
use crate::mind::techniques::{Technique, CRAFTS};
use crate::world::{Material, Shape};
use crate::world_state::World;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Engraving {
    /// The chronicle row it shows.
    pub event: EventId,
    /// When that happened.
    pub tick: u64,
    /// The event's words as the feed told it, at the carving.
    pub line: String,
    pub carver: AgentId,
    pub carver_name: String,
    pub carved: u64,
    /// 0..1, from the carver's skill.
    pub quality: f32,
    /// The craft this carving sets down (D37, #1395): set when the carver was the only one
    /// left who knew it, carving the way of it so it outlasts them. `None` for an ordinary
    /// event carving, and for every engraving an old save loads with.
    #[serde(default)]
    pub craft: Option<Technique>,
}

/// How much an event is worth carving: what changes a people, not what fills a day. 0: not
/// carved at all. (The first soaks carved mostly quarrels, a hundred walls of them.)
fn weight(k: &EventKind) -> u32 {
    match k {
        EventKind::Died { .. } | EventKind::Arrived { .. } => 6,
        EventKind::Born { .. }
        | EventKind::Paired { .. }
        | EventKind::GaveRing { .. }
        | EventKind::Burned { .. }
        | EventKind::Flooded { .. } => 4,
        EventKind::StrangersSighted { .. }
        | EventKind::Raided { .. }
        | EventKind::TookUpArms { .. } => 3,
        EventKind::Discovered { .. } | EventKind::HearthTale { .. } => 2,
        EventKind::Fought { .. } => 1,
        _ => 0,
    }
}

/// How much more a carver's culture cares to carve a kind of event (P8), in halves: 3 for
/// what its values prize (fights for those who honour skill at arms, births, pairings and
/// deaths for those who put kin first, arrivals for those who keep the old ways), else 2.
fn art_form(w: &World, i: usize, k: &EventKind) -> u32 {
    use crate::past::Topic;
    let Some(c) = w.agents[i]
        .culture
        .and_then(|c| w.realm.cultures.get(c as usize))
    else {
        return 2;
    };
    let prized = match k {
        EventKind::Fought { .. } | EventKind::Raided { .. } | EventKind::TookUpArms { .. } => {
            c.stance(Topic::MartialProwess) > 0.3
        }
        EventKind::Born { .. }
        | EventKind::Paired { .. }
        | EventKind::GaveRing { .. }
        | EventKind::Died { .. } => c.stance(Topic::Family) > 0.4,
        EventKind::Arrived { .. } => c.stance(Topic::Tradition) > 0.4,
        _ => false,
    };
    if prized {
        3
    } else {
        2
    }
}

/// What agent `i` would carve: the event of their people's record they and their people
/// remember most, not yet carved; ties go to the older.
pub fn choose(w: &World, i: usize) -> Option<(EventId, u64, String)> {
    let a = &w.agents[i];
    let cid = a.community?;
    let members: Vec<AgentId> = w
        .agents
        .iter()
        .filter(|x| x.community == Some(cid))
        .map(|x| x.id)
        .collect();
    let carved: std::collections::BTreeSet<EventId> =
        w.engravings.values().map(|e| e.event).collect();
    // The deaths of the restless dead near home (P11): carving one lays them to rest.
    let home = w.communities.iter().find(|c| c.id == cid).map(|c| c.home);
    let haunting: std::collections::BTreeSet<EventId> = w
        .agents
        .iter()
        .filter(|d| !d.alive && home.is_some_and(|h| h.z == d.pos.z && h.dist(d.pos) <= 20))
        .flat_map(|d| d.conditions.iter())
        .filter(|c| c.kind == crate::conditions::ConditionKind::Unrested && c.stage == 1)
        .map(|c| c.cause)
        .collect();
    let mut best: Option<(u32, u64, EventId, EventKind)> = None;
    for (id, tick, kind) in &w.culture_log {
        if carved.contains(id) || weight(kind) == 0 {
            continue;
        }
        let e = Event {
            id: *id,
            tick: *tick,
            at: None,
            severity: Severity::Major,
            kind: kind.clone(),
        };
        if !crate::view::known::told(w, &e) {
            continue;
        }
        let about_us = e.participants().iter().any(|p| members.contains(p));
        // Who among them carries it: a memory that came from it.
        let remembered = |x: &crate::agents::Agent| {
            x.memories.long.iter().any(|m| m.origins.contains(id))
                || x.memories.short.iter().any(|r| r.event == Some(*id))
        };
        let mine = remembered(a);
        let ours = w
            .agents
            .iter()
            .filter(|x| x.alive && x.community == Some(cid) && remembered(x))
            .count() as u32;
        let haunt = haunting.contains(id);
        if !about_us && !mine && ours == 0 && !haunt {
            continue;
        }
        let score = weight(kind) * 2 * art_form(w, i, kind) / 2
            + if haunt { 6 } else { 0 }
            + ours
            + if mine { 3 } else { 0 }
            + u32::from(about_us);
        if best
            .as_ref()
            .is_none_or(|b| score > b.0 || (score == b.0 && *tick < b.1))
        {
            best = Some((score, *tick, *id, kind.clone()));
        }
    }
    let (_, tick, id, kind) = best?;
    let e = Event {
        id,
        tick,
        at: None,
        severity: Severity::Major,
        kind,
    };
    Some((id, tick, crate::view::narrate::line(w, &e)))
}

/// The craft agent `i` would rather carve than any event (D37, #1395): they alone, of their
/// community, still know it, their community's own ground uses it, and they are past their
/// ailing years or already needing care — old enough that the craft could die with them
/// tonight. The last knower carves the way of it while they still can.
pub fn craft_to_carve(w: &World, i: usize) -> Option<Technique> {
    let a = &w.agents[i];
    let cid = a.community?;
    let old = a.age_years(w.meta.tick) >= a.ailing_years().0 || a.body.needs_care();
    if !old {
        return None;
    }
    CRAFTS.iter().copied().find(|&t| {
        a.techniques.knows(t)
            && crate::systems::crafts::knowers(w, cid, t) == [i]
            && crate::systems::crafts::craft_used(w, cid, t)
    })
}

/// Whether agent `i` has anything worth carving at all: a craft of their own that would
/// otherwise die with them, or an event, so a project can be proposed for either.
pub fn can_carve(w: &World, i: usize) -> bool {
    craft_to_carve(w, i).is_some() || choose(w, i).is_some()
}

/// A wall of a house that can take a carving: built of timber or block, standing beside a
/// floor someone can stand on, not already carved.
pub fn wall_to_carve(w: &World, near: TilePos, radius: i32) -> Option<TilePos> {
    let mut best: Option<(i32, TilePos)> = None;
    for site in w
        .communities
        .iter()
        .flat_map(|c| c.house_sites.iter())
        .filter(|s| s.dist(near) <= radius)
    {
        let plan = crate::houses::plan(w, *site);
        for (p, _) in crate::houses::walls(&plan.floor) {
            {
                let t = w.grid.get(p);
                if t.shape != Shape::Wall
                    || !matches!(t.material, Material::STONE_BLOCK | Material::WOOD)
                    || w.engravings.contains_key(&p)
                    || w.designations.iter().any(|d| d.pos == p)
                {
                    continue;
                }
                let open = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(x, y)| {
                    w.grid.walkable(p.offset(*x, *y, 0))
                        && w.roofs.contains_key(&p.offset(*x, *y, 0))
                });
                if !open {
                    continue;
                }
                let d = p.dist(near);
                if best.is_none_or(|b| d < b.0) {
                    best = Some((d, p));
                }
            }
        }
    }
    best.map(|b| b.1)
}

/// Agent `i` has finished carving the wall at `p`.
pub fn carve(w: &mut World, i: usize, p: TilePos) {
    let craft = craft_to_carve(w, i);
    let chosen = choose(w, i);
    if craft.is_none() && chosen.is_none() {
        return;
    }
    let (event, tick) = chosen
        .as_ref()
        .map(|(e, t, _)| (*e, *t))
        .unwrap_or((EventId(0), w.meta.tick));
    let line = match craft {
        Some(t) => format!("the way of {}, as {} did it", t.name(), w.agents[i].name),
        None => chosen.map(|(_, _, line)| line).unwrap_or_default(),
    };
    let skill = w.agents[i].skills.level(crate::mind::Skill::Construction);
    let e = Engraving {
        event,
        tick,
        line: line.clone(),
        carver: w.agents[i].id,
        carver_name: w.agents[i].name.clone(),
        carved: w.meta.tick,
        quality: (0.3 + skill / 20.0).clamp(0.1, 1.0),
        craft,
    };
    w.engravings.insert(p, e);
    let now = w.meta.tick;
    let who = w.agents[i].id;
    w.events.push(
        now,
        Some(p),
        Severity::Notable,
        EventKind::Engraved {
            agent: who,
            at: p,
            what: line,
        },
    );
    w.agents[i]
        .thoughts
        .add(crate::mind::ThoughtKind::MadeSomething, now, None);
}

/// The hover line for a carved wall: who carved it, when, and the scene.
pub fn note(w: &World, p: TilePos) -> Option<String> {
    let e = w.engravings.get(&p)?;
    let when = crate::legend::when(w, e.carved);
    Some(format!(
        "Carved by {} in {when}: {}",
        e.carver_name,
        e.line.trim_end_matches('.')
    ))
}
