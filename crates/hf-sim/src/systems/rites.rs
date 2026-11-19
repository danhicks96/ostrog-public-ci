//! Rites and reactions that differ by culture (P7 of milestone MP, docs/design-peoples.md
//! section 6.4). Each reaction is a thought with the row that caused it, and only someone whose
//! belief makes it matter has it.

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, EventId, TilePos};
use crate::mind::activity::Activity;
use crate::mind::beliefs::Topic;
use crate::mind::ThoughtKind;
use crate::realm::culture::Rite;
use crate::world_state::World;

/// A member of a community has died: they are laid to rest by their own rite if anyone left
/// in the community keeps it, else by the rite most of the living keep. Whoever is nearest
/// and closest does it. Those for whom the rite matters (Burial lean 0.3 or more) and who keep
/// another rite are troubled by it.
/// A death waiting for its rite (C68, #220): who is to do it, once they have come.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RiteDue {
    pub dead: AgentId,
    pub community: crate::ids::CommunityId,
    pub since: u64,
    pub death: EventId,
    pub doer: Option<AgentId>,
    /// The body is not in the valley (life::die_away, #566): nobody walks to it; the rite is
    /// given at home in its absence.
    #[serde(default)]
    pub away: bool,
}

/// How long the dead may wait before they are counted unrested: a day.
pub const RITE_WAIT: u64 = crate::time::TICKS_PER_DAY;
/// A body nobody can reach (lost on the road, beyond the valley's ways) is mourned at home
/// after this long.
pub const MOURN_AFTER: u64 = 6 * crate::time::TICKS_PER_HOUR;

/// The rite the community gives: the dead's own if anyone left keeps it, else the one most of
/// the living keep.
fn rite_for(w: &World, own: Rite, living: &[usize]) -> Rite {
    if living.iter().any(|&i| w.agents[i].rite == own) {
        return own;
    }
    let mut counts = std::collections::BTreeMap::new();
    for &i in living {
        *counts.entry(w.agents[i].rite as u8).or_insert(0u32) += 1;
    }
    let best = counts
        .iter()
        .max_by_key(|(r, n)| (**n, std::cmp::Reverse(**r)))
        .map(|(r, _)| *r);
    [Rite::Bury, Rite::Burn, Rite::Expose, Rite::Hall][best.unwrap_or(0) as usize]
}

/// Each tick (C68, #220): for each death waiting, someone awake of the community who keeps
/// the rite (kin first, then the nearest) walks to the body and gives the rite there. A body
/// none of them can reach is mourned at home after six hours; one nobody has come to in a day
/// is left unrested (P11).
pub fn step(w: &mut World) {
    if w.rites_due.is_empty() {
        return;
    }
    let tick = w.meta.tick;
    let due = w.rites_due.clone();
    let mut keep = Vec::new();
    for mut r in due {
        let Some(di) = w.agents.iter().position(|a| a.id == r.dead) else {
            continue;
        };
        let (own, body) = (w.agents[di].rite, w.agents[di].pos);
        let living: Vec<usize> = (0..w.agents.len())
            .filter(|&i| {
                let a = &w.agents[i];
                a.alive && a.community == Some(r.community) && a.id != r.dead
            })
            .collect();
        if living.is_empty() {
            left_unrested(w, di, r.death, "nobody of their people was left", r.away);
            continue;
        }
        if tick.saturating_sub(r.since) >= RITE_WAIT {
            left_unrested(w, di, r.death, "nobody came to them in a day", r.away);
            continue;
        }
        // Not on the tick of the death, even with the keeper beside the body (#255): the
        // body lies an hour first.
        if tick.saturating_sub(r.since) < crate::time::TICKS_PER_HOUR {
            keep.push(r);
            continue;
        }
        let rite = rite_for(w, own, &living);
        // The one who does it: kin who keep that rite first, then the nearest awake who keeps
        // it, who can get there.
        let away = r.away;
        let can_reach = |w: &World, i: usize| {
            let from = w.agents[i].pos;
            !away && from.z == body.z && w.nav.as_ref().is_some_and(|n| n.reach(from, body))
        };
        let doer = r
            .doer
            .filter(|_| !away)
            .and_then(|id| living.iter().copied().find(|&i| w.agents[i].id == id))
            .filter(|&i| !w.agents[i].activity.is_asleep())
            .or_else(|| {
                living
                    .iter()
                    .copied()
                    .filter(|&i| {
                        let a = &w.agents[i];
                        a.rite == rite && !a.activity.is_asleep() && can_reach(w, i)
                    })
                    .min_by_key(|&i| {
                        let a = &w.agents[i];
                        let kin = a.partner == Some(r.dead)
                            || a.children.contains(&r.dead)
                            || a.mother == Some(r.dead)
                            || a.father == Some(r.dead);
                        (u8::from(!kin), a.pos.dist(body), a.id)
                    })
            });
        let Some(i) = doer else {
            // Nobody can get there: mourned at home, by the nearest who keeps the rite.
            let reachable = living.iter().any(|&i| can_reach(w, i));
            if !reachable && tick.saturating_sub(r.since) >= MOURN_AFTER {
                let by = living
                    .iter()
                    .copied()
                    .filter(|&i| w.agents[i].rite == rite)
                    .min_by_key(|&i| w.agents[i].id)
                    .unwrap_or(living[0]);
                lay_to_rest(w, r.dead, r.community, Some(w.agents[by].id), true);
                continue;
            }
            keep.push(r);
            continue;
        };
        r.doer = Some(w.agents[i].id);
        if w.agents[i].pos.z == body.z && w.agents[i].pos.dist(body) <= 1 {
            let by = w.agents[i].id;
            lay_to_rest(w, r.dead, r.community, Some(by), false);
            continue;
        }
        let going = matches!(w.agents[i].activity, Activity::Going { to, .. } if to == body);
        if !going && w.path_agent_to(i, body) {
            w.agents[i].activity = Activity::Going {
                to: body,
                then: Box::new(Activity::Idle),
            };
            w.agents[i].activity_ticks = 0;
        }
        keep.push(r);
    }
    w.rites_due = keep;
}

/// The dead at `i` is given no rite: unrested (P11), and on the record with why, so every
/// death of a person ends in a row saying what became of them (#566, FIXLOG 2026-11-02).
pub fn left_unrested(w: &mut World, i: usize, death: EventId, why: &str, away: bool) {
    crate::conditions::unlaid(w, i, death);
    let (agent, pos) = (w.agents[i].id, w.agents[i].pos);
    w.events.push(
        w.meta.tick,
        (!away).then_some(pos),
        Severity::Notable,
        EventKind::LeftUnrested {
            agent,
            why: why.to_string(),
        },
    );
}

/// A member of a community has died: they are laid to rest by their own rite if anyone left
/// in the community keeps it, else by the rite most of the living keep, by `by` (or, given
/// none, whoever is nearest and closest). Those for whom the rite matters (Burial lean 0.3 or
/// more) and who keep another rite are troubled by it.
pub fn lay_to_rest(
    w: &mut World,
    dead: AgentId,
    community: crate::ids::CommunityId,
    by: Option<AgentId>,
    in_absence: bool,
) -> bool {
    let tick = w.meta.tick;
    // The row of the death, for the grave (accounts.rs): from the rite waiting, else from the
    // restlessness it left.
    let death = w
        .rites_due
        .iter()
        .find(|r| r.dead == dead)
        .map(|r| r.death)
        .or_else(|| {
            w.agents.iter().find(|a| a.id == dead).and_then(|d| {
                d.conditions
                    .iter()
                    .find(|c| c.kind == crate::conditions::ConditionKind::Unrested)
                    .map(|c| c.cause)
            })
        });
    w.rites_due.retain(|r| r.dead != dead);
    let Some(d) = w.agents.iter().find(|a| a.id == dead) else {
        return false;
    };
    let (own, pos) = (d.rite, d.pos);
    let living: Vec<usize> = (0..w.agents.len())
        .filter(|&i| {
            let a = &w.agents[i];
            a.alive && a.community == Some(community) && a.id != dead
        })
        .collect();
    if living.is_empty() {
        return false;
    }
    let rite = rite_for(w, own, &living);
    // The one who does it: kin who keep that rite first, then the nearest who keeps it.
    let by = by
        .and_then(|id| living.iter().copied().find(|&i| w.agents[i].id == id))
        .unwrap_or_else(|| {
            living
                .iter()
                .copied()
                .filter(|&i| w.agents[i].rite == rite)
                .min_by_key(|&i| {
                    let a = &w.agents[i];
                    let kin = a.partner == Some(dead)
                        || a.children.contains(&dead)
                        || a.mother == Some(dead)
                        || a.father == Some(dead);
                    (u8::from(!kin), a.pos.dist(pos), a.id)
                })
                .unwrap_or(living[0])
        });
    let by_id = w.agents[by].id;
    let at = if in_absence { w.agents[by].pos } else { pos };
    let ev = w.events.push(
        tick,
        Some(at),
        Severity::Notable,
        EventKind::LaidToRest {
            agent: dead,
            rite,
            by: by_id,
            in_absence,
        },
    );
    // Buried where they lay, they leave a grave a later person can come on (#452).
    if rite == Rite::Bury && !in_absence {
        if let Some(death) = death {
            crate::accounts::buried(w, dead, death, ev, Some(by_id));
        }
    }
    for &i in &living {
        let a = &mut w.agents[i];
        if a.rite != rite
            && a.beliefs.lean(Topic::Burial) >= 0.3
            && a.relationships.contains_key(&dead)
        {
            a.thoughts.add(ThoughtKind::RiteBroken, tick, Some(ev));
        }
    }
    outcome(w, dead, by, &living, at, in_absence, ev);
    true
}

/// #338: a rite does not simply happen. Who gathers, whether the one who leads it keeps the
/// rite and holds the old ways, and the weather weigh whether it is fitting, plain, or marred;
/// a marred rite leaves the dead unrested for all that, on record.
fn outcome(
    w: &mut World,
    dead: AgentId,
    by: usize,
    living: &[usize],
    at: TilePos,
    in_absence: bool,
    ev: crate::ids::EventId,
) {
    use rand::Rng;
    let tick = w.meta.tick;
    let mourners: Vec<usize> = living
        .iter()
        .copied()
        .filter(|&i| {
            let a = &w.agents[i];
            a.pos.z == at.z && a.pos.dist(at) <= 10 && !a.activity.is_asleep()
        })
        .collect();
    let m = mourners.len().min(6) as f32;
    let lean = w.agents[by].beliefs.lean(Topic::Burial).max(0.0);
    let wet = matches!(
        w.weather.sky,
        crate::systems::weather::Sky::Rain | crate::systems::weather::Sky::Snow
    );
    let p_fit = 0.2 + 0.06 * m + 0.25 * lean;
    let p_mar = 0.06
        + if wet { 0.06 } else { 0.0 }
        + if mourners.is_empty() { 0.08 } else { 0.0 }
        + if in_absence { 0.05 } else { 0.0 };
    let mut r = crate::rng::stream(
        w.meta.seed,
        &[
            crate::rng::purpose::AGENT_SOCIAL,
            0x2173,
            dead.0 as u64,
            tick,
        ],
    );
    let roll: f32 = r.gen();
    let by_id = w.agents[by].id;
    let (word, how) = if roll < p_mar {
        let why = if wet {
            "the rain put out the fire"
        } else if mourners.is_empty() {
            "nobody came to stand with them"
        } else {
            "a word was said wrong"
        };
        if let Some(d) = w.agents.iter().position(|a| a.id == dead) {
            if !w.agents[d]
                .conditions
                .iter()
                .any(|c| c.kind == crate::conditions::ConditionKind::Unrested)
            {
                crate::conditions::unlaid(w, d, ev);
            }
        }
        for &i in &mourners {
            w.agents[i]
                .thoughts
                .add(ThoughtKind::RiteBroken, tick, Some(ev));
        }
        ("marred", why.to_string())
    } else if roll < p_mar + p_fit {
        for &i in &mourners {
            w.agents[i]
                .thoughts
                .add(ThoughtKind::FittingRite, tick, Some(ev));
        }
        ("fitting", format!("{} stood with them", mourners.len()))
    } else {
        ("plain", String::new())
    };
    if word != "plain" {
        w.events.push(
            tick,
            Some(at),
            Severity::Notable,
            EventKind::RiteOutcome {
                agent: dead,
                by: by_id,
                outcome: word.to_string(),
                how,
            },
        );
    }
}

/// A living tree was felled at `at` by `by`: anyone nearby who holds that a living tree is
/// not to be cut (lean -0.4 or less) and could see it is troubled.
pub fn tree_felled(w: &mut World, by: AgentId, at: TilePos) {
    let tick = w.meta.tick;
    let watchers: Vec<usize> = (0..w.agents.len())
        .filter(|&i| {
            let a = &w.agents[i];
            a.alive
                && a.id != by
                && !a.activity.is_asleep()
                && a.pos.z == at.z
                && a.pos.dist(at) <= 10
                && a.beliefs.lean(Topic::CutLivingTree) <= -0.4
        })
        .collect();
    if watchers.is_empty() {
        return;
    }
    let ev = w.events.push(
        tick,
        Some(at),
        Severity::Minor,
        EventKind::TreeFelled { agent: by, at },
    );
    for i in watchers {
        if crate::mind::perception::line_of_sight(&w.grid, w.agents[i].pos, at) {
            w.agents[i]
                .thoughts
                .add(ThoughtKind::TreeFelled, tick, Some(ev));
        }
    }
}

/// Someone of another kind has joined a community (event `joined`): members who hold that any
/// kind may live among them (OtherRaces 0.3 or more) are glad of it, those who hold other kinds
/// are best kept apart (-0.3 or less) are uneasy.
pub fn joined(w: &mut World, who: AgentId, community: crate::ids::CommunityId, joined: EventId) {
    let tick = w.meta.tick;
    let Some(kind) = w.agents.iter().find(|a| a.id == who).map(|a| a.people) else {
        return;
    };
    for a in w.agents.iter_mut() {
        if !a.alive || a.community != Some(community) || a.id == who || a.people == kind {
            continue;
        }
        let lean = a.beliefs.lean(Topic::OtherRaces);
        if lean >= 0.3 {
            a.thoughts
                .add(ThoughtKind::NewKindWelcome, tick, Some(joined));
        } else if lean <= -0.3 {
            a.thoughts
                .add(ThoughtKind::NewKindUneasy, tick, Some(joined));
        }
    }
}
