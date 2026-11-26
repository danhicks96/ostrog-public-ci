//! Crafts as personal knowledge (C70, design rule 3, docs/design-rules.md): a craft is
//! something a person knows or does not. Only those who know a craft work its bench; when
//! nobody of a community knows one its best hand may try, slowly and with spoilage; when the
//! last who knows a craft dies or leaves, the record says the craft went with them; and one
//! who knows a craft few others do offers to teach it, which some refuse for their traits or
//! their memories, with the reason recorded.

use crate::agents::Agent;
use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, CommunityId};
use crate::mind::techniques::{Learned, Technique, CRAFTS};
use crate::mind::{MemKind, Subject, Trait};
use crate::rng::{purpose, stream};
use crate::time::TICKS_PER_DAY;
use crate::work::Designation;
use crate::world_state::World;

/// How long a refusal (or a lesson) holds before the same person is asked again.
pub const ASK_AGAIN_DAYS: u64 = 30;
/// A lesson leaves the pupil with this much of the craft.
pub const TAUGHT_SKILL: f32 = 0.15;
/// What a try without knowing how is worth, when it works.
pub const TRIED_SKILL: f32 = 0.2;

/// Those alive of `cid` who know `t`, here in the valley.
pub fn knowers(w: &World, cid: CommunityId, t: Technique) -> Vec<usize> {
    (0..w.agents.len())
        .filter(|&i| {
            let a = &w.agents[i];
            a.alive && a.community == Some(cid) && a.techniques.knows(t)
        })
        .collect()
}

/// Whether `cid`'s own ground holds a finished bench of the kind `t`'s recipes ask for: a
/// craft the community uses, not merely one some arrival once learned.
pub fn craft_used(w: &World, cid: CommunityId, t: Technique) -> bool {
    let stations: Vec<crate::structures::StructureKind> = (0..crate::work::recipes::RECIPES.len())
        .filter(|&r| crate::work::recipes::craft_of(r as u8) == Some(t))
        .map(|r| crate::work::recipes::recipe(r as u8).station)
        .collect();
    let home = w.communities.iter().find(|c| c.id == cid).map(|c| c.home);
    home.is_some_and(|h| {
        w.structures
            .iter()
            .any(|s| stations.contains(&s.kind) && s.progress >= 1.0 && s.pos.dist(h) <= 30)
    })
}

/// Those of `cid` who know `t` and are away on the community's own journey (#259): the craft
/// is not lost while its knower is three days down the road.
pub fn knowers_away(w: &World, cid: CommunityId, t: Technique) -> Vec<String> {
    w.overworld
        .away
        .iter()
        .filter(|a| a.alive && a.community == Some(cid) && a.techniques.knows(t))
        .map(|a| a.name.clone())
        .collect()
}

/// Whether `a`, who does not know `t`, may try the order `d` anyway: nobody of their
/// community knows it, the order has waited a day, they are not on cooldown from a last try,
/// and they are the community's best hand at making things (or curious).
pub fn may_try(w: &World, a: &Agent, t: Technique, d: &Designation) -> bool {
    let Some(cid) = a.community else {
        return false;
    };
    let tick = w.meta.tick;
    if tick.saturating_sub(d.created) < TICKS_PER_DAY {
        return false;
    }
    if !knowers(w, cid, t).is_empty() || !knowers_away(w, cid, t).is_empty() {
        return false;
    }
    let key = 0x7300_0000 | t as u32;
    if a.work_cooldowns
        .get(&key)
        .is_some_and(|until| *until > tick)
    {
        return false;
    }
    let mine = a.skills.level(crate::mind::Skill::Crafting);
    let best = w
        .agents
        .iter()
        .filter(|o| o.alive && o.community == Some(cid))
        .map(|o| o.skills.level(crate::mind::Skill::Crafting))
        .fold(0.0f32, f32::max);
    a.traits.has(Trait::Curious) || mine >= best - 0.5
}

/// Agent `i` finished a craft order of `t` without knowing it (C70): whether it came out,
/// and the record of the try either way. Returns true when the work is good.
pub fn tried(w: &mut World, i: usize, t: Technique, spoiled: &str) -> bool {
    let tick = w.meta.tick;
    let seed = w.meta.seed;
    let who = w.agents[i].id;
    let mut r = stream(seed, &[purpose::AGENT_WORK, 0x7A1E, who.0 as u64, tick]);
    let curious = w.agents[i].traits.has(Trait::Curious);
    let skill = w.agents[i].skills.level(crate::mind::Skill::Crafting);
    let chance = 0.2 + if curious { 0.15 } else { 0.0 } + skill / 40.0;
    let won = rand::Rng::gen::<f32>(&mut r) < chance;
    w.agents[i]
        .work_cooldowns
        .insert(0x7300_0000 | t as u32, tick + 2 * TICKS_PER_DAY);
    if won {
        w.agents[i].techniques.learn_from(
            t,
            Learned::Tried,
            TRIED_SKILL,
            tick,
            "worked out by trying",
        );
    }
    let pos = w.agents[i].pos;
    let ev = w.events.push(
        tick,
        Some(pos),
        if won {
            Severity::Notable
        } else {
            Severity::Minor
        },
        EventKind::CraftTried {
            agent: who,
            craft: t.name().to_string(),
            success: won,
            spoiled: spoiled.to_string(),
        },
    );
    if won {
        w.agents[i].remember(tick, MemKind::Discovery, Subject::None, 0.7, 0.7, Some(ev));
    } else {
        w.agents[i].remember(tick, MemKind::Work, Subject::None, -0.3, 0.4, Some(ev));
    }
    won
}

/// Why `pupil` will not learn `t` from `teacher`, if they will not.
pub fn refuses(w: &World, pupil: usize, teacher: AgentId) -> Option<String> {
    let a = &w.agents[pupil];
    let teacher_name = w
        .agents
        .iter()
        .find(|o| o.id == teacher)
        .map(|o| o.name.clone())
        .unwrap_or_default();
    if a.opinion_of(teacher) < -0.2 {
        return Some(format!("does not trust {teacher_name}"));
    }
    if a.memories
        .long
        .iter()
        .any(|m| m.kind == MemKind::Hardship && m.subject == Subject::Person(teacher))
    {
        return Some(format!("remembers {teacher_name} too well"));
    }
    if a.traits.has(Trait::Easygoing) {
        return Some("would rather not be tied to a bench".to_string());
    }
    if a.traits.has(Trait::Solitary) {
        return Some(format!("will not work beside {teacher_name} all day"));
    }
    None
}

/// Once a day: the crafts each community holds, the loss of one with its last knower, and a
/// lesson offered where one person alone knows a craft the community uses.
pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_DAY != crate::time::TICKS_PER_HOUR * 7 + 13 {
        return;
    }
    let cids: Vec<CommunityId> = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .map(|c| c.id)
        .collect();
    for cid in cids {
        // 1. What they know now, and what went with whom.
        let mut now: Vec<(Technique, String)> = Vec::new();
        for t in CRAFTS {
            let k = knowers(w, cid, t);
            if let Some(&i) = k.first() {
                now.push((t, w.agents[i].name.clone()));
            } else if let Some(name) = knowers_away(w, cid, t).into_iter().next() {
                now.push((t, name));
            }
        }
        let before: Vec<(Technique, String)> = w
            .communities
            .iter()
            .find(|c| c.id == cid)
            .map(|c| c.crafts.clone())
            .unwrap_or_default();
        // A craft the community had lost and knows again is said (#259).
        let lost_before: Vec<Technique> = w
            .communities
            .iter()
            .find(|c| c.id == cid)
            .map(|c| c.crafts_lost.clone())
            .unwrap_or_default();
        for (t, who) in &now {
            if lost_before.contains(t) {
                let home = w.communities.iter().find(|c| c.id == cid).map(|c| c.home);
                w.events.push(
                    tick,
                    home,
                    Severity::Notable,
                    EventKind::CraftRegained {
                        community: cid,
                        craft: t.name().to_string(),
                        by: who.clone(),
                    },
                );
            }
        }
        for (t, last) in &before {
            if now.iter().any(|(x, _)| x == t) {
                continue;
            }
            // Gone: dead, gone to another community of the valley (a split), or out of the
            // valley. The record names which (C72: "gone from the valley" was said of
            // everyone who split off, and said again every few days as the split settled).
            let how = w
                .agents
                .iter()
                .find(|a| &a.name == last)
                .map(|a| {
                    if !a.alive {
                        "dead".to_string()
                    } else if let Some(other) = a
                        .community
                        .and_then(|o| w.communities.iter().find(|c| c.id == o))
                    {
                        format!("gone to {}", other.name)
                    } else {
                        "gone from the area".to_string()
                    }
                })
                .unwrap_or_else(|| "gone".to_string());
            let said_lately = w.communities.iter().find(|c| c.id == cid).is_some_and(|c| {
                c.crafts_said
                    .iter()
                    .any(|(x, at)| x == t && tick.saturating_sub(*at) < 60 * TICKS_PER_DAY)
            });
            if said_lately {
                continue;
            }
            if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
                c.crafts_said.retain(|(x, _)| x != t);
                c.crafts_said.push((*t, tick));
            }
            let home = w.communities.iter().find(|c| c.id == cid).map(|c| c.home);
            w.events.push(
                tick,
                home,
                Severity::Major,
                EventKind::CraftLost {
                    community: cid,
                    craft: t.name().to_string(),
                    last: last.clone(),
                    how,
                },
            );
        }
        if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
            for (t, _) in &before {
                if !now.iter().any(|(x, _)| x == t) && !c.crafts_lost.contains(t) {
                    c.crafts_lost.push(*t);
                }
            }
            c.crafts_lost.retain(|t| !now.iter().any(|(x, _)| x == t));
            c.crafts = now.clone();
            c.teaching.retain(|(_, _, until)| *until > tick);
        }
        // 2. A lesson: a craft the community uses (a bench of its kind stands on their
        // ground) that one person alone knows is taught to the best other hand, once a day
        // at most, if they will.
        let used = |w: &World, t: Technique| -> bool { craft_used(w, cid, t) };
        for (t, _) in now.clone() {
            let k = knowers(w, cid, t);
            if k.len() != 1 || !used(w, t) {
                continue;
            }
            let teacher = k[0];
            let teacher_id = w.agents[teacher].id;
            if w.agents[teacher].activity.is_asleep() {
                continue;
            }
            let asked: Vec<AgentId> = w
                .communities
                .iter()
                .find(|c| c.id == cid)
                .map(|c| {
                    c.teaching
                        .iter()
                        .filter(|(_, x, _)| *x == t)
                        .map(|(a, _, _)| *a)
                        .collect()
                })
                .unwrap_or_default();
            let pupil = (0..w.agents.len())
                .filter(|&i| {
                    let a = &w.agents[i];
                    a.alive
                        && a.community == Some(cid)
                        && a.id != teacher_id
                        && a.of_age(tick)
                        && a.species == crate::agents::Species::Person
                        && !a.techniques.knows(t)
                        && !asked.contains(&a.id)
                })
                .max_by(|&x, &y| {
                    w.agents[x]
                        .skills
                        .level(crate::mind::Skill::Crafting)
                        .total_cmp(&w.agents[y].skills.level(crate::mind::Skill::Crafting))
                        .then(w.agents[y].id.cmp(&w.agents[x].id))
                });
            let Some(pupil) = pupil else { continue };
            let pupil_id = w.agents[pupil].id;
            if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
                c.teaching
                    .push((pupil_id, t, tick + ASK_AGAIN_DAYS * TICKS_PER_DAY));
            }
            let pos = w.agents[pupil].pos;
            if let Some(why) = refuses(w, pupil, teacher_id) {
                w.events.push(
                    tick,
                    Some(pos),
                    Severity::Notable,
                    EventKind::Chose {
                        agent: pupil_id,
                        what: format!(
                            "refused to learn {} from {}",
                            t.name(),
                            w.agents[teacher].name
                        ),
                        because: why,
                    },
                );
            } else {
                let from = format!("taught by {}", w.agents[teacher].name);
                w.agents[pupil].techniques.learn_from(
                    t,
                    Learned::Taught,
                    TAUGHT_SKILL,
                    tick,
                    &from,
                );
                let ev = w.events.push(
                    tick,
                    Some(pos),
                    Severity::Notable,
                    EventKind::Taught {
                        teacher: teacher_id,
                        pupil: pupil_id,
                        craft: t.name().to_string(),
                    },
                );
                w.agents[pupil].remember(
                    tick,
                    MemKind::Company,
                    Subject::Person(teacher_id),
                    0.4,
                    0.5,
                    Some(ev),
                );
            }
            break;
        }
    }
}
