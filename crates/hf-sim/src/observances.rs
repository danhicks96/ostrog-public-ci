//! Observances (#297, epic #376, ask 2): at noon on the first day of each season a community
//! keeps an observance, led by the one it follows in the spirit (justice.rs) or, with nobody
//! followed, by whoever of them most heeds the signs. Those of the community near the one who
//! leads it keep it with them: they are glad of it, they hold the signs a little surer, and
//! they follow the one who led it a little more. A community that kept the last one and has
//! nobody to lead this one, or nobody who comes, lets it lapse, on record, and those who heed
//! the signs are troubled by that. FIXLOG 2026-10-26 "no observance was ever kept".
//!
//! It is kept at the community's shrine (shrines.rs), else at home: an hour before noon those
//! free to go walk there, and at noon it is led by whoever of those there would lead (the most
//! devout, by `devotion`, where nobody is followed in the spirit), and kept by those within
//! reach. FIXLOG 2026-10-27 "nowhere to keep the observances" and "observances were almost
//! never kept".

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, CommunityId};
use crate::mind::beliefs::{Source, Topic};
use crate::mind::ThoughtKind;
use crate::time::{Season, TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::World;

/// Those of the community within this many tiles of where it is kept keep it.
pub const GATHERED_WITHIN: i32 = 20;
/// The lean on signs of one who heeds them: troubled by a lapse, wanting a shrine.
pub const LEADS_AT: f32 = 0.2;

fn season_word(s: Season) -> &'static str {
    match s {
        Season::Spring => "spring",
        Season::Summer => "summer",
        Season::Autumn => "autumn",
        Season::Winter => "winter",
    }
}

/// Once a season, at noon of its first day (after the arrival's first day); an hour before,
/// those free to go walk to their community's shrine, if it has one.
pub fn step(w: &mut World) {
    let since = w.since_arrival();
    let hour = since % TICKS_PER_DAY;
    if since < TICKS_PER_DAY || (hour != 12 * TICKS_PER_HOUR && hour != 11 * TICKS_PER_HOUR) {
        return;
    }
    let cal = w.calendar();
    if cal.day_of_season != 1 {
        return;
    }
    if hour == 11 * TICKS_PER_HOUR {
        gather(w);
        return;
    }
    let season = season_word(cal.season);
    let cids: Vec<CommunityId> = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .map(|c| c.id)
        .collect();
    for cid in cids {
        keep(w, cid, season);
    }
}

fn members(w: &World, cid: CommunityId) -> Vec<usize> {
    (0..w.agents.len())
        .filter(|&k| {
            let a = &w.agents[k];
            a.alive && a.species == crate::agents::Species::Person && a.community == Some(cid)
        })
        .collect()
}

/// How devout someone is: their lean on the signs if they hold a view, else their own untried
/// leaning (faith.rs `inclination`), faintly. FIXLOG 2026-10-27 "observances were almost never
/// kept": the founders hold no view on signs and nobody is followed in the spirit for months,
/// so with a lean of 0.2 required nobody could ever lead one.
pub fn devotion(w: &World, k: usize) -> f32 {
    let a = &w.agents[k];
    if a.beliefs.get(Topic::DeitySigns).is_some() {
        a.beliefs.lean(Topic::DeitySigns)
    } else {
        0.3 * crate::faith::inclination(w, a.id)
    }
}

/// Who leads it, of those gathered: the keeper or the one followed in the spirit, awake; else
/// the most devout awake, if they lean toward the signs at all. Leading the rite is a grown
/// person's standing: both must be of age (#1581).
fn leader(w: &World, cid: CommunityId, members: &[usize]) -> Option<usize> {
    // The keeper, else the one followed in the spirit (faith.rs `reader_of`).
    let followed = crate::faith::reader_of(w, cid);
    if let Some(i) = followed.and_then(|id| members.iter().copied().find(|&k| w.agents[k].id == id))
    {
        if !w.agents[i].activity.is_asleep() && w.agents[i].of_age(w.meta.tick) {
            return Some(i);
        }
    }
    members
        .iter()
        .copied()
        .filter(|&k| {
            !w.agents[k].activity.is_asleep()
                && w.agents[k].of_age(w.meta.tick)
                && devotion(w, k) > 0.0
        })
        .max_by(|&a, &b| {
            devotion(w, a)
                .total_cmp(&devotion(w, b))
                .then(w.agents[b].id.cmp(&w.agents[a].id))
        })
}

/// Where a community keeps it: at its shrine, else at home.
fn place(w: &World, cid: CommunityId) -> Option<crate::ids::TilePos> {
    crate::shrines::of(w, cid).or_else(|| {
        w.communities
            .iter()
            .find(|c| c.id == cid)
            .map(|c| w.grid.surface(c.home.x, c.home.y))
    })
}

/// An hour before noon: everyone of a community who is about nothing in particular (idle,
/// loitering, talking) walks to where it is kept (its shrine, else home), and waits there.
fn gather(w: &mut World) {
    use crate::mind::Activity;
    let shrines: Vec<(CommunityId, crate::ids::TilePos)> = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .filter_map(|c| place(w, c.id).map(|p| (c.id, p)))
        .collect();
    for (cid, at) in shrines {
        // Nobody leaves the work of food and shelter to keep one (#415, design rule 7): a
        // community short of food does not gather, nor does anyone hungry.
        let days = crate::systems::settlement::food_days(w, cid);
        let free: Vec<usize> = members(w, cid)
            .into_iter()
            .filter(|&k| {
                let a = &w.agents[k];
                // A faction keeps its own (heresy.rs `gather_apart`).
                !crate::heresy::apart(w, cid, a.id)
                    && crate::systems::decide::secured(w, a, Some(days)).all()
                    && a.band.is_none()
                    && a.caravan.is_none()
                    && !a.leaving
                    && a.pos.dist(at) <= 45
                    && a.pos.dist(at) > 2
                    && matches!(
                        a.activity,
                        Activity::Idle | Activity::Loitering { .. } | Activity::Talking { .. }
                    )
            })
            .collect();
        for (n, k) in free.into_iter().enumerate() {
            // Round the place, a ring two out.
            let (dx, dy) = [
                (0, 2),
                (2, 0),
                (0, -2),
                (-2, 0),
                (2, 2),
                (-2, 2),
                (2, -2),
                (-2, -2),
            ][n % 8];
            let spot = w.grid.surface(at.x + dx, at.y + dy);
            let spot = if w.grid.walkable(spot) { spot } else { at };
            if w.path_agent_to(k, spot) {
                w.agents[k].activity = Activity::Going {
                    to: spot,
                    then: Box::new(Activity::Loitering {
                        ticks_left: 2 * TICKS_PER_HOUR as u32,
                    }),
                };
                w.agents[k].activity_ticks = 0;
            }
        }
        crate::heresy::gather_apart(w, cid);
    }
}

fn keep(w: &mut World, cid: CommunityId, season: &str) {
    let tick = w.meta.tick;
    // A faction keeps its own, apart (heresy.rs), and is not of this one.
    crate::heresy::keep_apart(w, cid);
    let all: Vec<usize> = members(w, cid)
        .into_iter()
        .filter(|&k| !crate::heresy::apart(w, cid, w.agents[k].id))
        .collect();
    if all.is_empty() {
        return;
    }
    // At the shrine, where there is one (shrines.rs), else at home: led by whoever of those
    // come to it would lead, and kept by those who came.
    let shrine = crate::shrines::of(w, cid);
    let Some(spot) = place(w, cid) else { return };
    let there: Vec<usize> = all
        .iter()
        .copied()
        .filter(|&k| {
            let a = &w.agents[k];
            a.pos.z == spot.z && a.pos.dist(spot) <= GATHERED_WITHIN
        })
        .collect();
    let lead = leader(w, cid, &there);
    let came: Vec<usize> = match lead {
        Some(l) => {
            let at = spot;
            all.iter()
                .copied()
                .filter(|&k| {
                    let a = &w.agents[k];
                    k != l
                        && a.pos.z == at.z
                        && a.pos.dist(at) <= GATHERED_WITHIN
                        && !a.activity.is_asleep()
                })
                .collect()
        }
        None => Vec::new(),
    };
    let kept_last = w.faith.kept.iter().any(|(c, _)| *c == cid);
    let Some(l) = lead.filter(|_| !came.is_empty()) else {
        if !kept_last {
            return;
        }
        w.faith.kept.retain(|(c, _)| *c != cid);
        let why = if lead.is_none() {
            "nobody led it"
        } else {
            "nobody came"
        };
        let at = w.communities.iter().find(|c| c.id == cid).map(|c| c.home);
        let ev = w.events.push(
            tick,
            at,
            Severity::Notable,
            EventKind::ObservanceLapsed {
                community: cid,
                season: season.to_string(),
                why: why.to_string(),
            },
        );
        for k in all {
            if w.agents[k].beliefs.lean(Topic::DeitySigns) >= LEADS_AT {
                w.agents[k]
                    .thoughts
                    .add(ThoughtKind::ObservanceLapsed, tick, Some(ev));
            }
        }
        return;
    };
    let by: AgentId = w.agents[l].id;
    let ev = w.events.push(
        tick,
        Some(w.agents[l].pos),
        Severity::Notable,
        EventKind::Observance {
            by,
            community: cid,
            season: season.to_string(),
            came: came.len() as u32,
            at_shrine: shrine.is_some(),
        },
    );
    if let Some(at) = shrine {
        crate::shrines::tended(w, at);
    }
    for &k in came.iter().chain(std::iter::once(&l)) {
        w.agents[k]
            .thoughts
            .add(ThoughtKind::KeptObservance, tick, Some(ev));
        w.agents[k]
            .beliefs
            .experience(Topic::DeitySigns, 1.0, 0.08, Source::Event(ev), tick);
    }
    for &k in &came {
        crate::justice::credit(w, k, by, crate::justice::Domain::Spirit, 0.1);
    }
    w.faith.kept.retain(|(c, _)| *c != cid);
    w.faith.kept.push((cid, tick));
}
