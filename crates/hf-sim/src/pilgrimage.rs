//! Pilgrimage (#297, epic #376, ask 7): in hard times someone who heeds the signs walks to
//! their community's shrine (shrines.rs), or with none standing to where its relic lies
//! (relics.rs), and prays there an hour. Hard times are their own grief, a warning read to their
//! community and not yet borne out or belied (faith.rs), or the loss of the shrine or a relic.
//! At the place they are eased: their grief and fear lessen, they hold the signs a little surer,
//! and they are glad of it. Each going is on record (`Pilgrimage`).
//!
//! Only by day, never more than three of a community at once, and only by those whose food is
//! secured (#415, design rule 7: nobody leaves the work of tomorrow's food and shelter to pray).
//! FIXLOG 2026-10-28 "nobody went to the shrine in hard times".

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, CommunityId, EventId, TilePos};
use crate::mind::beliefs::{Source, Topic};
use crate::mind::feelings::FeelingKind;
use crate::mind::{Activity, ThoughtKind};
use crate::rng::{purpose, stream};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::World;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Chance each daylight hour that someone in hard times sets out.
pub const SETS_OUT_A_HOUR: f64 = 0.1;
/// Never more of a community on the way at once.
pub const AT_ONCE: usize = 3;
/// How far a pilgrim walks: about the width of a community's ground and its fields.
pub const WITHIN: i32 = 60;

/// Someone on the way to pray.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pilgrim {
    pub agent: AgentId,
    pub community: CommunityId,
    pub to: TilePos,
    pub since: u64,
    /// The `Pilgrimage` event of their setting out.
    pub event: EventId,
}

/// Whether agent `k` may leave the day's work to pray or gather apart: everything secured
/// (#415, `systems::decide::is_secured`: their hunger, the community's stores, a bed or roof
/// before dark), and able to walk there.
pub fn free_to_go(w: &World, k: usize) -> bool {
    let a = &w.agents[k];
    crate::systems::decide::is_secured(w, k) && !a.injured(w.meta.tick) && !a.body.needs_care()
}

/// What makes the times hard for agent `k`, if they are.
fn hard_times(w: &World, k: usize, cid: CommunityId) -> Option<&'static str> {
    let a = &w.agents[k];
    if a.feelings
        .list
        .iter()
        .any(|f| f.kind == FeelingKind::Grief && f.strength >= 0.3)
    {
        return Some("in grief");
    }
    if w.faith.signs.iter().any(|s| s.community == cid && !s.good) {
        return Some("afraid of the warning read to them");
    }
    if a.thoughts.has(ThoughtKind::ShrineLost) || a.thoughts.has(ThoughtKind::RelicLost) {
        return Some("troubled by what was lost");
    }
    None
}

/// Where a community's pilgrims go: its shrine, else where its relic lies.
fn goal(w: &World, cid: CommunityId) -> Option<(TilePos, String)> {
    crate::shrines::of(w, cid)
        .map(|p| (p, "the shrine".to_string()))
        .or_else(|| crate::relics::place_of(w, cid))
}

/// Hourly by day: pilgrims who have come pray; those in hard times may set out.
pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_HOUR != 29 {
        return;
    }
    arrive(w);
    let hour = (tick % TICKS_PER_DAY) / TICKS_PER_HOUR;
    if !(8..=16).contains(&hour) || w.since_arrival() < TICKS_PER_DAY {
        return;
    }
    for k in 0..w.agents.len() {
        let a = &w.agents[k];
        let Some(cid) = a.community else { continue };
        if !a.alive
            || a.species != crate::agents::Species::Person
            || a.is_child(tick)
            || a.band.is_some()
            || a.caravan.is_some()
            || a.leaving
            || !matches!(
                a.activity,
                Activity::Idle | Activity::Loitering { .. } | Activity::Wandering { .. }
            )
            || w.faith.pilgrims.iter().any(|p| p.agent == a.id)
            || crate::observances::devotion(w, k) < crate::observances::LEADS_AT
        {
            continue;
        }
        let Some(why) = hard_times(w, k, cid) else {
            continue;
        };
        if !free_to_go(w, k) {
            continue;
        }
        if w.faith
            .pilgrims
            .iter()
            .filter(|p| p.community == cid)
            .count()
            >= AT_ONCE
        {
            continue;
        }
        let Some((to, what)) = goal(w, cid) else {
            continue;
        };
        let d = a.pos.dist(to);
        if d <= 3 || d > WITHIN {
            continue;
        }
        let mut r = stream(
            w.meta.seed,
            &[purpose::AGENT_SOCIAL, 0x2977, a.id.0 as u64, tick],
        );
        if !r.gen_bool(SETS_OUT_A_HOUR) {
            continue;
        }
        let spot = [(1, 1), (-1, 1), (1, -1), (-1, -1), (0, 2), (2, 0)]
            .into_iter()
            .map(|(dx, dy)| w.grid.surface(to.x + dx, to.y + dy))
            .find(|p| w.grid.walkable(*p))
            .unwrap_or(to);
        if !w.path_agent_to(k, spot) {
            continue;
        }
        let id = w.agents[k].id;
        w.agents[k].activity = Activity::Going {
            to: spot,
            then: Box::new(Activity::Loitering {
                ticks_left: 2 * TICKS_PER_HOUR as u32,
            }),
        };
        w.agents[k].activity_ticks = 0;
        let at = w.agents[k].pos;
        let event = w.events.push(
            tick,
            Some(at),
            Severity::Notable,
            EventKind::Pilgrimage {
                agent: id,
                to: what,
                why: why.to_string(),
            },
        );
        w.faith.pilgrims.push(Pilgrim {
            agent: id,
            community: cid,
            to,
            since: tick,
            event,
        });
    }
}

/// Pilgrims at the place pray there; one a day on the way and not come gives it up.
fn arrive(w: &mut World) {
    let tick = w.meta.tick;
    let all = std::mem::take(&mut w.faith.pilgrims);
    let mut still = Vec::new();
    for p in all {
        let Some(k) = w.agents.iter().position(|a| a.id == p.agent && a.alive) else {
            continue;
        };
        if w.agents[k].pos.dist(p.to) > 3 {
            if tick.saturating_sub(p.since) < TICKS_PER_DAY {
                still.push(p);
            }
            continue;
        }
        let a = &mut w.agents[k];
        for f in a.feelings.list.iter_mut() {
            if matches!(f.kind, FeelingKind::Grief | FeelingKind::Fear) {
                f.strength *= 0.6;
            }
        }
        a.thoughts.add(ThoughtKind::Prayed, tick, Some(p.event));
        a.beliefs
            .experience(Topic::DeitySigns, 1.0, 0.05, Source::Event(p.event), tick);
    }
    w.faith.pilgrims = still;
}
