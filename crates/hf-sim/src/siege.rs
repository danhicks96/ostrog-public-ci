//! Sieges and prisoners (C76, backlog 9.2).
//!
//! A place beyond the valley whose people were hurt, routed or kept by the valley holds a
//! grudge (`Site::grudge`); at two it sends a host, not a hungry band (threat.rs). The host
//! camps, raids the stores like any band, and by day goes for anyone of the valley out beyond
//! the wall, carrying off those it brings down. It lifts its siege when the wall holds and the
//! stores stay out of reach. Prisoners go both ways: a raider hurt in a fight may be kept, then
//! ransomed by their own people for food or, in time, taken in; someone carried off may be let
//! go and walk home. Preparation decides it: a closed wall, people with the nerve to stand, and
//! stores behind the wall.

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, TilePos};
use crate::mind::Activity;
use crate::rng::{purpose, stream};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::World;
use rand::Rng;

fn idx(w: &World, id: AgentId) -> Option<usize> {
    w.agents.iter().position(|a| a.id == id && a.alive)
}

fn go(w: &mut World, i: usize, to: TilePos) {
    if w.agents[i].pos != to && w.grid.walkable(to) && w.path_agent_to(i, to) {
        w.agents[i].activity = Activity::Going {
            to,
            then: Box::new(Activity::Loitering { ticks_left: 60 }),
        };
        w.agents[i].activity_ticks = 0;
    }
}

/// Out beyond the wall: not inside a closed palisade and not under a roof.
fn exposed(w: &World, p: TilePos) -> bool {
    !crate::systems::settlement::in_walls(w, p) && !crate::systems::hazards::indoors(w, p)
}

/// A raider brought down in a fight (`ri`, by `si`) may be kept rather than let go home: by a
/// community with a roof to keep them under, half the time. Returns true if kept.
pub fn keep(w: &mut World, k: usize, si: usize, ri: usize, r: &mut impl Rng) -> bool {
    let Some(cid) = w.agents[si].community else {
        return false;
    };
    let roof = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .is_some_and(|c| !c.house_sites.is_empty() || c.palisade.is_some());
    if !roof || !r.gen_bool(0.5) {
        return false;
    }
    let tick = w.meta.tick;
    let r_id = w.agents[ri].id;
    let site = w.bands[k].site;
    w.bands[k].lost += 1;
    w.bands[k].members.retain(|m| *m != r_id);
    let a = &mut w.agents[ri];
    a.band = None;
    a.leaving = false;
    a.held = Some((cid, site, tick));
    if let Some(st) = w.overworld.sites.iter_mut().find(|x| x.id == site) {
        st.grudge += 1;
    }
    let home = w.communities.iter().find(|c| c.id == cid).map(|c| c.home);
    if let Some(h) = home {
        go(w, ri, h);
    }
    let at = w.agents[ri].pos;
    w.events.push(
        tick,
        Some(at),
        Severity::Major,
        EventKind::Held { agent: r_id },
    );
    true
}

/// A camped host, by day: each member not busy raiding goes for the nearest person of the
/// valley out beyond the wall within twenty tiles of the camp, strikes when beside them, and
/// carries off one they bring down (one a day). Someone already down is not struck again: they
/// are carried off if the host may still take one, and else left lying; a host that holds all
/// the captives it may take goes for nobody and keeps to its camp, guarding them (#402).
pub fn besiege(w: &mut World, k: usize, since: u64) {
    let tick = w.meta.tick;
    let hour = (tick % TICKS_PER_DAY) / TICKS_PER_HOUR;
    if !(7..19).contains(&hour) {
        return;
    }
    let camp = w.bands[k].camp;
    let days = (tick.saturating_sub(since) / TICKS_PER_DAY) as usize + 1;
    let raiders: Vec<AgentId> = w.bands[k].raiders.iter().map(|x| x.0).collect();
    let members = w.bands[k].members.clone();
    for m in members {
        if raiders.contains(&m) {
            continue;
        }
        let Some(i) = idx(w, m) else { continue };
        if w.agents[i].body.needs_care() {
            continue;
        }
        let me = w.agents[i].pos;
        // #402 (M5 review 2026-10-26, seed 3: one person took 592 blows in three days): once
        // the host held its two captives, whoever lay downed nearest was struck again every
        // pass. A host with all it may take guards them at its camp instead.
        let can_take = w.bands[k].captives.len() < days.min(2);
        if w.bands[k].captives.len() >= 2 {
            if me.dist(camp) > 3
                && !matches!(w.agents[i].activity, Activity::Going { to, .. } if to.dist(camp) <= 3)
            {
                go(w, i, camp);
            }
            continue;
        }
        let target = (0..w.agents.len())
            .filter(|&j| {
                let o = &w.agents[j];
                o.alive
                    && o.community.is_some()
                    && o.band.is_none()
                    && o.held.is_none()
                    // Someone down is only gone for to be carried off (#402).
                    && (can_take || !o.body.needs_care())
                    && o.pos.z == me.z
                    && o.pos.dist(camp) <= 20
                    && exposed(w, o.pos)
            })
            .min_by_key(|&j| (w.agents[j].pos.dist(me), w.agents[j].id));
        let Some(j) = target else { continue };
        let at = w.agents[j].pos;
        if at.dist(me) > 1 {
            if !matches!(w.agents[i].activity, Activity::Going { to, .. } if to.dist(at) <= 1) {
                go(w, i, at);
            }
            continue;
        }
        if w.agents[j].body.needs_care() {
            // Down already: taken, not struck (#402).
            let mut rc = stream(w.meta.seed, &[purpose::THREATS, 0xCA9E, m.0 as u64, tick]);
            if rc.gen_bool(0.5) {
                carry_off(w, k, j);
            }
            continue;
        }
        let mut r = stream(w.meta.seed, &[purpose::THREATS, 0x5E6E, m.0 as u64, tick]);
        w.agents[i].fought_at = tick;
        w.agents[j].fought_at = tick;
        crate::systems::combat::blow(w, i, j, &mut r, "by the besiegers");
        // The struck turns on them if they have the nerve.
        if crate::systems::combat::nerve(w, j) >= 0.3 {
            crate::systems::combat::blow(w, j, i, &mut r, "fighting the besiegers");
        } else {
            crate::systems::combat::run_home(w, j);
        }
        let victim = w.agents[j].id;
        w.events.push(
            tick,
            Some(at),
            Severity::Major,
            EventKind::Fought { a: m, b: victim },
        );
        // At most two carried off by one host, and not every time (C77 soaks: 20 to 76 a run).
        let mut rc = stream(w.meta.seed, &[purpose::THREATS, 0xCA9E, m.0 as u64, tick]);
        if w.agents[j].body.needs_care()
            && w.bands[k].captives.len() < days.min(2)
            && rc.gen_bool(0.5)
        {
            carry_off(w, k, j);
        }
    }
}

/// Days of battering a wall of hardness 1.0 stands before a stretch of it comes down (#274).
pub const BATTER_DAYS_PER_HARDNESS: f32 = 2.5;

/// #274: how many days a host must batter a wall tile at `q` to bring it down: as hard as the
/// stuff it was built of (a pine wall two, granite four, the deep's basalt or gabbro five).
pub fn batter_days(w: &World, q: TilePos) -> u64 {
    let m = w.made_of(q).unwrap_or(w.grid.get(q).material);
    let h = w.grid.materials.get(m).hardness.max(0.1);
    (h * BATTER_DAYS_PER_HARDNESS).ceil() as u64
}

/// #274: a camped host batters the closed wall nearest its camp each day at noon. The stretch
/// it strikes comes down once it has been camped as many days as the wall's stuff stands
/// (`batter_days`); then the wall is open, and the builders are set to mend it. Stone gives
/// back rubble of its kind.
pub fn batter(w: &mut World, k: usize, since: u64) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_DAY != 12 * TICKS_PER_HOUR {
        return;
    }
    let camp = w.bands[k].camp;
    let days = tick.saturating_sub(since) / TICKS_PER_DAY;
    // The nearest standing stretch of any closed wall, not its gate.
    let target = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .filter_map(|c| c.palisade.as_ref().filter(|p| p.closed).map(|p| (c.id, p)))
        .flat_map(|(cid, p)| {
            p.ring
                .iter()
                .filter(move |q| **q != p.gate)
                .map(move |q| (cid, *q))
        })
        .filter(|(_, q)| w.grid.get(*q).shape == crate::world::Shape::Wall)
        .min_by_key(|(_, q)| (q.dist(camp), *q));
    let Some((cid, q)) = target else { return };
    if days < batter_days(w, q) {
        return;
    }
    let of = knock_down(w, q, Some(cid));
    let site = w.bands[k].site_name.clone();
    w.events.push(
        tick,
        Some(q),
        Severity::Major,
        EventKind::WallBroken {
            site,
            of,
            days: days as u32,
        },
    );
}

/// #274, #422: bring down the built wall, door or fence at `q`, by a host's battering or a
/// beast's weight. It is open ground after; stone gives back rubble of its kind; a closed wall
/// with it in its ring is open; and the builders of `cid`, if any, are set to mend it, of what
/// it was built of. Returns the name of the stuff it was built of.
pub fn knock_down(w: &mut World, q: TilePos, cid: Option<crate::ids::CommunityId>) -> String {
    let tile = w.grid.get(q);
    let of = w.made_of(q).unwrap_or(tile.material);
    let stone = tile.material == crate::world::Material::STONE_BLOCK;
    let door = tile.flags.has(crate::world::TileFlags::DOOR);
    w.grid.set(q, crate::world::Tile::AIR);
    w.built_of.remove(&q);
    if stone {
        w.create_item_of(
            crate::items::ItemKind::RoughStone,
            of,
            1,
            crate::items::ItemPlace::Ground(q),
            0.5,
            "battered",
        );
    }
    for c in w.communities.iter_mut() {
        if let Some(p) = c.palisade.as_mut().filter(|p| p.ring.contains(&q)) {
            p.closed = false;
        }
    }
    if let Some(cid) = cid {
        // The builders mend it, of what it was built of.
        let kind = if door {
            crate::work::designation::DesignationKind::BuildDoor
        } else {
            crate::work::designation::DesignationKind::BuildWall(if stone {
                crate::world::Material::STONE_BLOCK
            } else {
                crate::world::Material::WOOD
            })
        };
        let was = w.meta.planning_for;
        w.meta.planning_for = Some(cid);
        let d = crate::systems::settlement::make(w, kind, q, 7);
        w.meta.planning_for = was;
        w.designations.push(d);
    }
    w.grid.materials.get(of).name.clone()
}

/// Someone of the valley brought down by a host is carried off to its place.
fn carry_off(w: &mut World, k: usize, j: usize) {
    let tick = w.meta.tick;
    let id = w.agents[j].id;
    let at = w.agents[j].pos;
    let site = w.bands[k].site;
    let site_name = w.bands[k].site_name.clone();
    w.reservations.release_all(id);
    for it in w.items.iter_mut() {
        if it.place == crate::items::ItemPlace::Carried(id) {
            it.place = crate::items::ItemPlace::Ground(at);
        }
    }
    let mut a = w.agents.remove(j);
    a.activity = Activity::Idle;
    a.activity_ticks = 0;
    a.path.clear();
    w.overworld.away.push(a);
    w.bands[k].captives.push(id);
    if let Some(st) = w.overworld.sites.iter_mut().find(|x| x.id == site) {
        st.captives.push((id, tick));
    }
    w.events.push(
        tick,
        Some(at),
        Severity::Major,
        EventKind::Captured {
            agent: id,
            site: site_name,
        },
    );
}

/// Whether a camped host gives up: three days camped, the valley walled, and less than half of
/// what it came for taken.
pub fn lifts(w: &World, k: usize, since: u64) -> bool {
    let b = &w.bands[k];
    let walled = w
        .communities
        .iter()
        .any(|c| !c.dissolved && c.palisade.as_ref().is_some_and(|p| p.closed));
    w.meta.tick.saturating_sub(since) >= 3 * TICKS_PER_DAY && walled && b.taken * 2 < b.want
}

/// The siege is over, one way or another: on record, with how long it lasted and what it cost.
/// A host that took all it came for went home sated; any other end is a siege lifted, with why
/// (#402, M5 review 2026-10-26: every end was written `siege_lifted`, and all ten in the soaks
/// were hosts that took what they wanted).
pub fn ended(w: &mut World, k: usize, since: u64, why: Ending) {
    let tick = w.meta.tick;
    let b = &w.bands[k];
    let days = (tick.saturating_sub(since) / TICKS_PER_DAY) as u32;
    let (site, taken, captives, camp) = (
        b.site_name.clone(),
        b.taken,
        b.captives.len() as u32,
        b.camp,
    );
    if why == Ending::Sated {
        w.events.push(
            tick,
            Some(camp),
            Severity::Major,
            EventKind::HostSated {
                site,
                days,
                taken,
                captives,
            },
        );
        return;
    }
    let why = match why {
        Ending::WallHeld => "the wall held",
        Ending::DrivenOff => "driven off",
        _ => "tired of waiting",
    };
    w.events.push(
        tick,
        Some(camp),
        Severity::Major,
        EventKind::SiegeLifted {
            site,
            days,
            taken,
            captives,
            why: why.into(),
        },
    );
}

/// How a siege ended (#402).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ending {
    /// The host took all it came for.
    Sated,
    /// Half the host or more was lost.
    DrivenOff,
    /// Three days at a closed wall with less than half of what it came for (`lifts`).
    WallHeld,
    /// Ten days camped and not sated.
    Tired,
}

/// Once a day: the kept are ransomed by their own people or taken in; the carried off may be let
/// go and walk home.
pub fn daily(w: &mut World) {
    let tick = w.meta.tick;
    let day = tick / TICKS_PER_DAY;
    // The kept.
    for i in 0..w.agents.len() {
        let Some((cid, site, since)) = w.agents[i].held else {
            continue;
        };
        if !w.agents[i].alive {
            continue;
        }
        let id = w.agents[i].id;
        let held_days = tick.saturating_sub(since) / TICKS_PER_DAY;
        let home = w.communities.iter().find(|c| c.id == cid).map(|c| c.home);
        // Kept near the houses.
        if let Some(h) = home {
            if w.agents[i].pos.dist(h) > 10 && !w.agents[i].is_moving() {
                go(w, i, h);
            }
        }
        if held_days < 4 {
            continue;
        }
        let fed_home = w
            .overworld
            .sites
            .iter()
            .any(|s| s.id == site && s.people > 0 && s.food >= 10.0);
        if fed_home {
            // Their own people pay for them in food, left at the houses.
            let mut name = String::new();
            if let Some(s) = w.overworld.sites.iter_mut().find(|s| s.id == site) {
                s.food -= 10.0;
                s.grudge = s.grudge.saturating_sub(1);
                name = s.name.clone();
            }
            if let Some(h) = home {
                w.create_item(
                    crate::items::ItemKind::Meal,
                    10,
                    crate::items::ItemPlace::Ground(h),
                    0.6,
                    "a ransom",
                );
            }
            w.agents[i].held = None;
            w.agents[i].leaving = true;
            let at = w.agents[i].pos;
            w.events.push(
                tick,
                Some(at),
                Severity::Major,
                EventKind::Ransomed {
                    agent: id,
                    site: name,
                    food: 10,
                },
            );
        } else if held_days >= 10 {
            // Nobody came for them: in time they are one of the valley.
            w.agents[i].held = None;
            w.agents[i].community = Some(cid);
            let at = w.agents[i].pos;
            w.events.push(
                tick,
                Some(at),
                Severity::Major,
                EventKind::TookIn { agent: id },
            );
        }
    }
    // The carried off.
    let seed = w.meta.seed;
    let mut home_again: Vec<(AgentId, u32)> = Vec::new();
    for s in w.overworld.sites.iter_mut() {
        let mut keep = Vec::new();
        for (who, since) in std::mem::take(&mut s.captives) {
            let mut r = stream(seed, &[purpose::THREATS, 0xCA97, who.0 as u64, day]);
            if tick.saturating_sub(since) >= 8 * TICKS_PER_DAY && r.gen_bool(0.25) {
                home_again.push((who, s.id));
                s.grudge = s.grudge.saturating_sub(1);
            } else {
                keep.push((who, since));
            }
        }
        s.captives = keep;
    }
    for (who, site) in home_again {
        let Some(ix) = w.overworld.away.iter().position(|a| a.id == who) else {
            continue;
        };
        let Some(s) = w.overworld.sites.iter().find(|s| s.id == site).cloned() else {
            continue;
        };
        let home = w.communities.iter().find(|c| !c.dissolved).map(|c| c.home);
        let Some(home) = home else { continue };
        let Some(edge) = crate::systems::expedition::edge_tile(w, s.edge, home) else {
            continue;
        };
        let mut a = w.overworld.away.remove(ix);
        a.pos = edge;
        a.from = edge;
        a.path.clear();
        a.activity = Activity::Idle;
        a.activity_ticks = 0;
        a.needs.hunger = a.needs.hunger.min(0.4);
        w.agents.push(a);
        let i = w.agents.len() - 1;
        go(w, i, home);
        w.events.push(
            tick,
            Some(edge),
            Severity::Major,
            EventKind::Returned {
                agent: who,
                site: s.name.clone(),
            },
        );
    }
}
