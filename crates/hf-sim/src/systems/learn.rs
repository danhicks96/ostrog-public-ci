//! Learning by watching (mind/techniques.rs): someone who sees a technique work may pick it up.
//! A child near a working adult learns too (see systems::needs for skills); here it is the
//! crisis techniques: beating out flames, carrying water to a fire, laying a poultice.

use crate::events::EventKind;
use crate::mind::techniques::{Learned, Technique};
use crate::rng::{purpose, stream};
use crate::World;
use rand::Rng;

pub fn observe(w: &mut World) {
    let tick = w.meta.tick;
    let seed = w.meta.seed;
    let seen: Vec<(
        Technique,
        crate::ids::AgentId,
        crate::ids::TilePos,
        crate::ids::EventId,
    )> = w
        .last_events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::BeatOut { agent, at } => Some((Technique::BeatFlames, *agent, *at, e.id)),
            EventKind::Doused { agent, at } => Some((Technique::CarryWater, *agent, *at, e.id)),
            EventKind::Tended { healer, .. } => {
                e.at.map(|p| (Technique::Poultice, *healer, p, e.id))
            }
            _ => None,
        })
        .collect();
    for (t, doer, at, eid) in seen {
        for a in w.agents.iter_mut() {
            if !a.alive
                || a.id == doer
                || a.species != crate::agents::Species::Person
                || a.pos.z != at.z
                || a.pos.dist(at) > 8
                || a.activity.is_asleep()
                || a.techniques.knows(t)
            {
                continue;
            }
            let mut r = stream(
                seed,
                &[purpose::AGENT_SOCIAL, 0x1EA2, a.id.0 as u64, eid.0 as u64],
            );
            let chance = if a.traits.has(crate::mind::Trait::Curious) {
                0.65
            } else {
                0.45
            };
            if r.gen::<f32>() < chance {
                a.techniques.learn(t, Learned::Watched, 0.15, tick);
            }
        }
    }

    // E16's first outcome hooks stay event-driven and personal: the person who caused a breach
    // or cave-in gets evidence of the real consequence, while nobody else receives a global
    // lesson merely because the world knows the event occurred.
    // FIXLOG 2026-10-30 PR #474: keep outcome valence, confirmation and event provenance
    // separate. In particular, learn_observe runs before aquifer_observe, so a water lesson
    // must use the event's worksite and tick on the following pass, never the actor's new pose.
    let outcomes: Vec<(
        crate::ids::AgentId,
        crate::ids::TilePos,
        crate::mind::ObservationKind,
        crate::mind::OutcomeKind,
        u64,
        crate::ids::EventId,
    )> = w
        .last_events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::CaveIn { agent, at, hurt } => Some((
                *agent,
                *at,
                crate::mind::ObservationKind::StructuralWeakness,
                if *hurt {
                    crate::mind::OutcomeKind::Harm
                } else {
                    crate::mind::OutcomeKind::NoChange
                },
                e.tick,
                e.id,
            )),
            EventKind::Collapse { by, at, hurt, .. } => Some((
                *by,
                *at,
                crate::mind::ObservationKind::StructuralWeakness,
                if hurt.is_empty() {
                    crate::mind::OutcomeKind::NoChange
                } else {
                    crate::mind::OutcomeKind::Harm
                },
                e.tick,
                e.id,
            )),
            EventKind::UpperFloorFell { at, hurt, .. } => w
                .agents
                .iter()
                .find(|a| a.alive && a.pos.dist(*at) <= 10)
                .map(|a| {
                    (
                        a.id,
                        *at,
                        crate::mind::ObservationKind::StructuralWeakness,
                        if hurt.is_empty() {
                            crate::mind::OutcomeKind::NoChange
                        } else {
                            crate::mind::OutcomeKind::Harm
                        },
                        e.tick,
                        e.id,
                    )
                }),
            EventKind::StruckWater { agent, .. } => e.at.map(|at| {
                (
                    *agent,
                    at,
                    crate::mind::ObservationKind::Saturation,
                    crate::mind::OutcomeKind::Discovery,
                    e.tick,
                    e.id,
                )
            }),
            EventKind::WaterBrokeIn { agent, .. } => e.at.map(|at| {
                (
                    *agent,
                    at,
                    crate::mind::ObservationKind::Saturation,
                    crate::mind::OutcomeKind::Discovery,
                    e.tick,
                    e.id,
                )
            }),
            _ => None,
        })
        .collect();
    for (agent, at, cue, outcome, event_tick, event) in outcomes {
        if let Some(a) = w.agents.iter_mut().find(|a| a.id == agent && a.alive) {
            a.learning.observe(crate::mind::Observation {
                tick: event_tick,
                pos: at,
                kind: cue,
                strength: 1.0,
                evidence: crate::mind::Evidence::Direct,
                event: Some(event),
                full: false,
                reading: None,
            });
            a.learning
                .learn(cue, outcome, Some(at), true, event_tick, Some(event));
        }
    }
}

/// Agent `i` just did work with a craft technique they know (C13): it gets better with use,
/// and anyone near and awake who sees it may pick it up, with less chance than a fire's lesson
/// (a craft is harder to see than flames being beaten out).
pub fn witnessed(w: &mut World, i: usize, t: Technique) {
    let tick = w.meta.tick;
    let seed = w.meta.seed;
    let at = w.agents[i].pos;
    let doer = w.agents[i].id;
    w.agents[i].techniques.outcome(t, true, tick);
    let mut learned: Vec<crate::ids::AgentId> = Vec::new();
    for a in w.agents.iter_mut() {
        if !a.alive
            || a.id == doer
            || a.species != crate::agents::Species::Person
            || a.community.is_none()
            || a.pos.z != at.z
            || a.pos.dist(at) > 6
            || a.activity.is_asleep()
            || a.techniques.knows(t)
        {
            continue;
        }
        let mut r = stream(
            seed,
            &[
                purpose::AGENT_SOCIAL,
                0x57D1,
                a.id.0 as u64,
                doer.0 as u64,
                tick,
            ],
        );
        let chance = if a.traits.has(crate::mind::Trait::Curious) {
            0.12
        } else {
            0.06
        };
        if r.gen::<f32>() < chance {
            a.techniques.learn(t, Learned::Watched, 0.2, tick);
            learned.push(a.id);
        }
    }
    let dname = w.agents[i].name.clone();
    for who in learned {
        if let Some(a) = w.agents.iter_mut().find(|a| a.id == who) {
            if let Some(k) = a.techniques.list.iter_mut().find(|k| k.technique == t) {
                k.from = format!("from watching {dname}");
            }
        }
        w.events.push(
            tick,
            Some(at),
            crate::events::Severity::Notable,
            EventKind::LearnedByWatching {
                agent: who,
                from: doer,
                technique: t.name().to_string(),
            },
        );
    }
}
