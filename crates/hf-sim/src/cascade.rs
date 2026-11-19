//! #319, the first cascade family: a flood drowns a stockpile, the drenched grain rots, the
//! rot draws vermin, the vermin bite a sleeper, the bite festers. Chaos from complexity,
//! not from randomness (docs/design-emergence.md): every step is causally recorded, holding
//! the previous step's event row the way a condition holds its giving row (conditions.rs) —
//! the chronicle can trace the whole spiral, and no step fires without the recorded row
//! before it (D9: history is recorded, never invented). The one chance in the family is
//! the vermin's bite, from a seeded stream of the outbreak's site and the night.
//! (FIXLOG 2026-11-01 #319.)
//!
//! The later families (FIXLOG 2026-11-02 #319) are made the same way, of ordinary systems
//! reading each other, each step holding the row before it:
//! - the lean winter: the stores run short in the cold months (`StoresFailed`), the hungry of
//!   that community grow weak (`Weakened`), the weak catch what goes round more readily and the
//!   catching is recorded (`SickenedWeak`, health.rs), and a sickness can kill as any does;
//! - the lost roof: a roof burned off (hazards.rs `Burned`) or fallen in (building.rs
//!   `RoofFell`) lets the rain in, the rain spoils the food lying under it (`RainSpoiled`), and
//!   the spoiled food draws vermin as the flood's does, so the first family's bite and
//!   festering follow;
//! - the lost healer: when the last healer of a community dies (`HealerLost`, holding the
//!   death's row), nobody lays a poultice, wounds are nursed but not dressed, and one that turns
//!   bad is recorded as festering for want of a healer (`WoundFestered`, health.rs), and the
//!   fever follows.
//!
//! A community that keeps stores, roofs its stores again and has more than one healer does not
//! start these chains, or does not go far down them.

use std::collections::BTreeMap;

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::body::WoundKind;
use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, CommunityId, EventId, TilePos};
use crate::items::ItemPlace;
use crate::rng::{purpose, stream};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::world_state::World;

/// The most outbreaks at once, so a long wet season does not pile them up without bound.
pub const MAX_OUTBREAKS: usize = 24;
/// An outbreak disperses after this many days.
const OUTBREAK_DAYS: u64 = 5;
/// The vermin reach a sleeper this many tiles from the rot.
const BITE_REACH: i32 = 10;
/// The seeded chance of a bite on a night a sleeper is in reach.
const BITE_CHANCE: f64 = 0.8;

/// A burned roof lets the rain in for at most this long; after it the place is taken as given
/// up, or roofed again (building.rs).
const UNROOFED_DAYS: u64 = 20;
/// Hours of rain that ruin food lying open to it.
const RAIN_HOURS: f32 = 6.0;
/// In the cold months a community whose stores hold less than this many days a head has
/// failed stores; at this many again, or when spring comes, the failure is over.
const LEAN_DAYS: f32 = 1.0;
const LEAN_OVER_DAYS: f32 = 2.0;
/// Below this fullness one of a lean community is weak from hunger; recorded once a spell,
/// they are weak while they stay below `WEAK_OVER`.
pub const WEAK_HUNGER: f32 = 0.25;
pub const WEAK_OVER: f32 = 0.6;

/// Whether `a` is weak from hunger in a failed winter now: weakened this spell and not since
/// fed (health.rs reads it).
pub fn weak_now(w: &World, a: &crate::agents::Agent) -> bool {
    a.needs.hunger < WEAK_OVER && w.cascades.weakened.contains_key(&a.id)
}

/// The standing causes the later cascade families read: each holds the row of the event that
/// started it, so what follows can be recorded with its cause.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Roots {
    /// Tiles whose roof burned or fell: that row and its tick.
    #[serde(default)]
    pub unroofed: BTreeMap<TilePos, (EventId, u64)>,
    /// Communities whose last healer died: the `HealerLost` row.
    #[serde(default)]
    pub healerless: BTreeMap<CommunityId, EventId>,
    /// Communities whose stores failed in the cold months: the failure's row.
    #[serde(default)]
    pub lean: BTreeMap<CommunityId, EventId>,
    /// People weak from hunger in a lean community: the weakening's row.
    #[serde(default)]
    pub weakened: BTreeMap<AgentId, EventId>,
}

/// A vermin outbreak drawn to rotted food: the row that recorded their coming, and the row
/// of the rot that drew them (the conditions.rs cause machinery).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vermin {
    pub at: TilePos,
    pub since: u64,
    /// The VerminDrawn event that recorded their coming.
    pub row: EventId,
    /// The Drench event that drew them.
    pub cause: EventId,
}

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    let hour = (tick % TICKS_PER_DAY) / TICKS_PER_HOUR;
    let minute = tick % TICKS_PER_HOUR;
    notice(w);
    if minute != 53 {
        return;
    }
    rain(w);
    if hour == 6 {
        lean(w);
    }
    if hour != 21 {
        return;
    }
    drench(w);
    bite(w);
    w.vermin
        .retain(|v| tick.saturating_sub(v.since) < OUTBREAK_DAYS * TICKS_PER_DAY);
}

/// Food standing under flood water rots in a day, and the rot is recorded with the flood's
/// row; each drenched tile draws vermin, recorded with the rot's row.
fn drench(w: &mut World) {
    let tick = w.meta.tick;
    // Nobody is taken by chance alone: with no recorded flood there is no cascade.
    let Some(flood) = w.waters.flood_event else {
        return;
    };
    if w.flooded.is_empty() {
        return;
    }
    let mut lost: BTreeMap<TilePos, (u32, String)> = BTreeMap::new();
    let mut gone: Vec<(crate::ids::ItemId, u32)> = Vec::new();
    for it in w.items.iter_mut() {
        if it.kind.keeps_days().is_none() {
            continue;
        }
        let ItemPlace::Ground(p) = it.place else {
            continue;
        };
        if !w.flooded.contains(&p) {
            continue;
        }
        // A day under water ruins it; the stack loses a quarter of itself, as in spoil.rs.
        it.rot += 1.0;
        if it.rot >= 1.0 {
            it.rot = 0.75;
            let lose = it.count.div_ceil(4);
            let entry = lost.entry(p).or_insert((0, it.kind.label().into()));
            entry.0 += lose;
            let id = it.id;
            gone.push((id, lose));
        }
    }
    for (at, (count, what)) in lost {
        let drench = w.events.push(
            tick,
            Some(at),
            Severity::Minor,
            EventKind::Drenched {
                at,
                what: format!("{count} {what}"),
                count,
                cause: flood,
            },
        );
        draw_vermin(w, at, drench);
    }
    for (id, lose) in gone {
        w.consume_item(id, lose, "drenched");
    }
}

/// Rotted food at `at`, recorded in the row `rot`, draws vermin, unless an outbreak is near
/// already or there are too many.
fn draw_vermin(w: &mut World, at: TilePos, rot: EventId) {
    let tick = w.meta.tick;
    if w.vermin.len() >= MAX_OUTBREAKS
        || w.vermin
            .iter()
            .any(|v| v.at.z == at.z && v.at.dist(at) <= 15)
    {
        return;
    }
    let row = w.events.push(
        tick,
        Some(at),
        Severity::Notable,
        EventKind::VerminDrawn { at, cause: rot },
    );
    w.vermin.push(Vermin {
        at,
        since: tick,
        row,
        cause: rot,
    });
}

/// A healer: one who knows how to make and lay a poultice, or holds the healer's place
/// (systems/decide.rs, who goes to tend the hurt).
pub fn is_healer(a: &crate::agents::Agent) -> bool {
    a.techniques.knows(crate::mind::Technique::Poultice)
        || a.role == Some(crate::society::Role::Healer)
}

/// Whether community `c` has a living healer among its people.
fn has_healer(w: &World, c: CommunityId) -> bool {
    w.agents
        .iter()
        .any(|a| a.alive && a.community == Some(c) && is_healer(a))
}

/// This tick's events that start a later family: a roof burned off, the death of a
/// community's last healer.
fn notice(w: &mut World) {
    let tick = w.meta.tick;
    let mut lost: Vec<(CommunityId, AgentId, EventId, Option<TilePos>)> = Vec::new();
    for e in &w.events.pending {
        match &e.kind {
            EventKind::Burned { at, what } if what == "roof" => {
                w.cascades.unroofed.insert(*at, (e.id, tick));
            }
            EventKind::Died { agent, .. } => {
                let Some(a) = w.agents.iter().find(|a| a.id == *agent) else {
                    continue;
                };
                if let (Some(c), true) = (a.community, is_healer(a)) {
                    lost.push((c, a.id, e.id, e.at));
                }
            }
            _ => {}
        }
    }
    for (community, healer, died, at) in lost {
        if w.cascades.healerless.contains_key(&community) || has_healer(w, community) {
            continue;
        }
        let row = w.events.push(
            tick,
            at,
            Severity::Major,
            EventKind::HealerLost {
                community,
                healer,
                cause: died,
            },
        );
        w.cascades.healerless.insert(community, row);
    }
}

/// Hourly: rain on food lying where a roof burned spoils it in `RAIN_HOURS`, recorded with the
/// burning's row, and the spoiled food draws vermin. A place roofed again, or long given up,
/// is no longer open to it.
fn rain(w: &mut World) {
    let tick = w.meta.tick;
    let roofs = &w.roofs;
    w.cascades.unroofed.retain(|p, (_, since)| {
        !roofs.contains_key(p) && tick.saturating_sub(*since) < UNROOFED_DAYS * TICKS_PER_DAY
    });
    if w.cascades.unroofed.is_empty() || !w.weather.sky.wet() {
        return;
    }
    let mut lost: BTreeMap<TilePos, (u32, String, EventId)> = BTreeMap::new();
    let mut gone: Vec<(crate::ids::ItemId, u32)> = Vec::new();
    for it in w.items.iter_mut() {
        if it.kind.keeps_days().is_none() || it.story.is_some() {
            continue;
        }
        let ItemPlace::Ground(p) = it.place else {
            continue;
        };
        let Some((roof, _)) = w.cascades.unroofed.get(&p) else {
            continue;
        };
        it.rot += 1.0 / RAIN_HOURS;
        if it.rot >= 1.0 {
            it.rot = 0.75;
            let lose = it.count.div_ceil(4);
            let entry = lost.entry(p).or_insert((0, it.kind.label().into(), *roof));
            entry.0 += lose;
            gone.push((it.id, lose));
        }
    }
    for (at, (count, what, roof)) in lost {
        let row = w.events.push(
            tick,
            Some(at),
            Severity::Minor,
            EventKind::RainSpoiled {
                at,
                what: format!("{count} {what}"),
                count,
                cause: roof,
            },
        );
        draw_vermin(w, at, row);
    }
    for (id, lose) in gone {
        w.consume_item(id, lose, "spoiled by the rain");
    }
}

/// Each morning: in the cold months a community whose stores have run short has failed
/// stores, recorded once until they are full again; and those of it gone hungry are weak,
/// recorded with the failure's row, until they have eaten.
fn lean(w: &mut World) {
    use crate::time::Season;
    let tick = w.meta.tick;
    let cold = matches!(w.calendar().season, Season::Autumn | Season::Winter);
    let ids: Vec<CommunityId> = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .map(|c| c.id)
        .collect();
    w.cascades.lean.retain(|c, _| ids.contains(c));
    for cid in ids {
        let days = crate::systems::settlement::food_days(w, cid);
        let lean = w.cascades.lean.contains_key(&cid);
        if lean && (days >= LEAN_OVER_DAYS || !cold) {
            w.cascades.lean.remove(&cid);
        } else if !lean && cold && days < LEAN_DAYS {
            let home = w.communities.iter().find(|c| c.id == cid).map(|c| c.home);
            let row = w.events.push(
                tick,
                home,
                Severity::Major,
                EventKind::StoresFailed {
                    community: cid,
                    food_hours: (days * 24.0) as u32,
                },
            );
            w.cascades.lean.insert(cid, row);
        }
    }
    // A community with a healer again (taught, or one come among them) is no longer without.
    let healed: Vec<CommunityId> = w
        .cascades
        .healerless
        .keys()
        .copied()
        .filter(|c| has_healer(w, *c))
        .collect();
    for c in healed {
        w.cascades.healerless.remove(&c);
    }
    // A weakening is recorded once a spell: kept while the community's stores are short and
    // dropped when they are not, or when the person is gone from it (`weak_now` says whether
    // they are weak at the moment).
    let agents = &w.agents;
    let lean_now = &w.cascades.lean;
    w.cascades.weakened.retain(|id, _| {
        agents.iter().any(|a| {
            a.id == *id && a.alive && a.community.is_some_and(|c| lean_now.contains_key(&c))
        })
    });
    let mut weak: Vec<(AgentId, TilePos, EventId)> = Vec::new();
    for a in w.agents.iter() {
        if !a.alive
            || a.species != crate::agents::Species::Person
            || a.needs.hunger >= WEAK_HUNGER
            || w.cascades.weakened.contains_key(&a.id)
        {
            continue;
        }
        if let Some(row) = a.community.and_then(|c| w.cascades.lean.get(&c)) {
            weak.push((a.id, a.pos, *row));
        }
    }
    for (agent, at, cause) in weak {
        let row = w.events.push(
            tick,
            Some(at),
            Severity::Notable,
            EventKind::Weakened { agent, cause },
        );
        w.cascades.weakened.insert(agent, row);
    }
}

/// Each outbreak bites at most one sleeper a night, if a sleeper is in reach and the seeded
/// roll passes; the bite is recorded with the outbreak's row, and the festering with the
/// bite's.
fn bite(w: &mut World) {
    let tick = w.meta.tick;
    let day = tick / TICKS_PER_DAY;
    for v in 0..w.vermin.len() {
        let site = w.vermin[v].at;
        let row = w.vermin[v].row;
        let Some(j) = (0..w.agents.len())
            .filter(|&j| {
                let a = &w.agents[j];
                a.alive
                    && a.species == crate::agents::Species::Person
                    && a.activity.is_asleep()
                    && a.pos.z == site.z
                    && a.pos.dist(site) <= BITE_REACH
            })
            .min_by_key(|&j| (w.agents[j].pos.dist(site), w.agents[j].id))
        else {
            continue;
        };
        let mut r = stream(
            w.meta.seed,
            &[
                purpose::CASCADE,
                site.x as u64,
                site.y as u64,
                site.z as u64,
                day,
            ],
        );
        if !r.gen_bool(BITE_CHANCE) {
            continue;
        }
        let (who, at) = (w.agents[j].id, w.agents[j].pos);
        crate::systems::health::wound(
            w,
            j,
            crate::body::Part::TORSO,
            WoundKind::Puncture,
            0.2,
            0.05,
            "bitten by vermin",
            None,
        );
        // The vermin's filth is in it from the first hour: the bite festers, and both rows
        // are recorded (the health pass turns the festering wound into the WoundRot).
        if let Some(wd) = w.agents[j].body.wounds.last_mut() {
            wd.infected = true;
        }
        w.agents[j].body.blood = (w.agents[j].body.blood - 0.1).max(0.2);
        w.agents[j]
            .thoughts
            .add(crate::mind::ThoughtKind::Hurt, tick, None);
        let bite = w.events.push(
            tick,
            Some(at),
            Severity::Major,
            EventKind::VerminBite {
                agent: who,
                at,
                cause: row,
            },
        );
        w.events.push(
            tick,
            Some(at),
            Severity::Notable,
            EventKind::Festered {
                agent: who,
                cause: bite,
            },
        );
    }
}
