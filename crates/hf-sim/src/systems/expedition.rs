//! Caravans: formed by a community for a purpose, walked to the map edge, resolved in
//! day-steps off the map, and brought back with persistent outcomes. People who leave are
//! moved whole into `overworld.away` and come back as the same people.

use rand::Rng;
use std::collections::BTreeMap;

use crate::agents::{Agent, Species};
use crate::events::{EventKind, Severity};
use crate::expedition::{Caravan, CaravanState, Edge, Purpose, SiteKind};
use crate::ids::{AgentId, CommunityId, TilePos};
use crate::items::{ItemKind, ItemPlace};
use crate::mind::{Activity, MemKind, Skill, Subject};
use crate::rng::{purpose, stream};
use crate::systems::society::take_items;
use crate::time::{Season, TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::world_state::World;

/// How long the departure gathering may take before the party leaves with whoever is there.
const FORMING_LIMIT: u64 = TICKS_PER_DAY;
/// A returned traveller stays home at least this long; a community waits this long between journeys.
pub(crate) const REST_AFTER: u64 = TICKS_PER_DAY * 3;
/// Able adults a community needs before it can spare a party of up to three.
pub(crate) const MIN_ABLE: usize = 10;

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % 200 == 17 {
        forming(w);
    }
    // New journeys are proposed by someone who wants to go (systems/projects.rs).
    if tick % TICKS_PER_DAY == TICKS_PER_HOUR * 6 {
        // The world's settlements make and use their stores before the day's travel.
        crate::trade::daily(w);
        travel_day(w);
    }
}

pub(crate) fn edge_tile(w: &World, edge: Edge, near: TilePos) -> Option<TilePos> {
    let g = &w.grid;
    let (x, y) = match edge {
        Edge::North => (near.x.clamp(2, g.width - 3), 2),
        Edge::South => (near.x.clamp(2, g.width - 3), g.height - 3),
        Edge::West => (2, near.y.clamp(2, g.height - 3)),
        Edge::East => (g.width - 3, near.y.clamp(2, g.height - 3)),
    };
    let nav = w.nav.as_ref()?;
    let mut best = None;
    let mut bd = i32::MAX;
    for k in -40..=40 {
        let (tx, ty) = match edge {
            Edge::North | Edge::South => ((x + k).clamp(2, g.width - 3), y),
            _ => (x, (y + k).clamp(2, g.height - 3)),
        };
        let p = g.surface(tx, ty);
        if g.walkable(p) && nav.reach(near, p) {
            let d = k.abs();
            if d < bd {
                bd = d;
                best = Some(p);
            }
        }
    }
    best
}

fn site_of(p: &Purpose) -> u32 {
    match p {
        Purpose::Trade { site, .. } | Purpose::Scout { site } | Purpose::Fetch { site, .. } => {
            *site
        }
    }
}

pub(crate) fn lower(k: ItemKind) -> String {
    k.label().to_string()
}

/// Grown people of the community free to travel: not hurt, not already on a journey, rested.
pub(crate) fn able(w: &World, cid: CommunityId) -> Vec<AgentId> {
    let tick = w.meta.tick;
    w.agents
        .iter()
        .filter(|a| {
            a.alive
                && a.community == Some(cid)
                && a.species == Species::Person
                && !a.is_child(tick)
                && !a.injured(tick)
                && a.caravan.is_none()
                && (a.last_caravan == 0 || tick - a.last_caravan >= REST_AFTER)
        })
        .map(|a| a.id)
        .collect()
}

/// Form a caravan from `cid` for `purpose`: pick the party, take the cargo, and send the
/// members to the departure tile. Returns the caravan id. Used by the trigger above and by
/// scripted scenarios.
pub fn form_caravan(w: &mut World, cid: CommunityId, purpose: Purpose) -> Option<u32> {
    form_caravan_led(w, cid, purpose, None)
}

/// As `form_caravan`, with `lead` (the one who proposed the journey, project.rs) first in the
/// party when they can go.
pub fn form_caravan_led(
    w: &mut World,
    cid: CommunityId,
    purpose: Purpose,
    lead: Option<AgentId>,
) -> Option<u32> {
    let tick = w.meta.tick;
    let home = w.communities.iter().find(|c| c.id == cid)?.home;
    let able = able(w, cid);
    let site = w.overworld.site(site_of(&purpose)).unwrap().clone();
    let depart = edge_tile(w, site.edge, home)?;
    // The party: the three best travellers (hauling + social) without a young child.
    let mut cands: Vec<AgentId> = able
        .iter()
        .copied()
        .filter(|m| {
            let a = w.agents.iter().find(|a| a.id == *m).unwrap();
            !a.children.iter().any(|c| {
                w.agents
                    .iter()
                    .any(|x| x.id == *c && x.alive && x.age_years(tick) < 8)
            })
        })
        .collect();
    let fit = |w: &World, id: AgentId| -> f32 {
        let a = w.agents.iter().find(|a| a.id == id).unwrap();
        a.skills.level(Skill::Hauling) + a.skills.level(Skill::Social)
    };
    cands.sort_by(|x, y| {
        fit(w, *y)
            .partial_cmp(&fit(w, *x))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    // Never more than three, and at least seven able adults stay home.
    if let Some(l) = lead.filter(|l| cands.contains(l)) {
        cands.retain(|c| *c != l);
        cands.insert(0, l);
    }
    let take = 3.min(able.len().saturating_sub(MIN_ABLE - 3));
    let party: Vec<AgentId> = cands.into_iter().take(take).collect();
    if party.len() < 2 {
        return None;
    }
    let mut cargo = Vec::new();
    let mut materials = BTreeMap::new();
    let mut qualities = BTreeMap::new();
    if let Purpose::Trade { carry, .. } | Purpose::Fetch { carry, .. } = purpose {
        let load = take_items(w, home, carry, 12);
        if load.n == 0 {
            // Nothing to carry; nobody walks out to trade it.
            return None;
        }
        cargo.push((carry, load.n));
        materials.insert(carry, load.material);
        qualities.insert(carry, load.quality);
    }
    // Rations only from a surplus: the people at home eat first.
    let meals_home: u32 = w
        .items
        .iter()
        .filter(|it| {
            it.kind == ItemKind::Meal
                && matches!(it.place, ItemPlace::Ground(p) if p.dist(home) <= 18)
        })
        .map(|it| it.count)
        .sum();
    let want = party.len() as u32 * 2;
    let mut rations = 0;
    if meals_home >= want + able.len() as u32 {
        rations = take_items(w, home, ItemKind::Meal, want).n;
    }
    let id = w.overworld.next_caravan_id;
    w.overworld.next_caravan_id += 1;
    for m in &party {
        if let Some(i) = w.agents.iter().position(|a| a.id == *m) {
            w.reservations.release_all(*m);
            w.agents[i].activity = Activity::Wandering { to: depart };
            w.agents[i].activity_ticks = 0;
            w.agents[i].path.clear();
            let _ = w.path_agent_to(i, depart);
            w.agents[i].caravan = Some(id);
        }
    }
    let label = match &purpose {
        Purpose::Trade { carry, .. } => {
            format!("to trade {} at {}", lower(*carry), site.name)
        }
        Purpose::Scout { .. } => format!("to find {}", site.name),
        Purpose::Fetch { want, .. } => format!("to fetch {} from {}", lower(*want), site.name),
    };
    w.overworld.caravans.push(Caravan {
        id,
        community: cid,
        members: party.clone(),
        purpose,
        cargo,
        materials,
        qualities,
        rations,
        lost: 0,
        state: CaravanState::Forming,
        formed: tick,
        departed: 0,
        depart_tile: depart,
        finished: 0,
        log: vec![format!("set out {label}")],
    });
    w.events.push(
        tick,
        Some(home),
        Severity::Major,
        EventKind::CaravanFormed {
            caravan: id,
            community: cid,
            leader: party[0],
            members: party.len() as u32,
            purpose: label,
        },
    );
    Some(id)
}

/// Members walk to the departure tile; when all are there (or a day passes) the party
/// leaves the map. Stragglers are released and stay home.
fn forming(w: &mut World) {
    let tick = w.meta.tick;
    let n = w.overworld.caravans.len();
    for ci in 0..n {
        if w.overworld.caravans[ci].state != CaravanState::Forming {
            continue;
        }
        let depart = w.overworld.caravans[ci].depart_tile;
        let members = w.overworld.caravans[ci].members.clone();
        let mut all_there = true;
        for m in &members {
            let Some(i) = w.agents.iter().position(|a| a.id == *m && a.alive) else {
                continue;
            };
            if w.agents[i].pos.dist(depart) > 2 {
                all_there = false;
                let a = &w.agents[i];
                let heading = matches!(a.activity, Activity::Wandering { to } if to == depart);
                if !heading
                    && matches!(
                        a.activity,
                        Activity::Idle | Activity::Loitering { .. } | Activity::Wandering { .. }
                    )
                {
                    w.agents[i].activity = Activity::Wandering { to: depart };
                    w.agents[i].activity_ticks = 0;
                    let _ = w.path_agent_to(i, depart);
                }
            }
        }
        let waited = tick - w.overworld.caravans[ci].formed;
        if !(all_there || waited > FORMING_LIMIT) {
            continue;
        }
        let mut gone: Vec<AgentId> = Vec::new();
        for m in &members {
            let Some(i) = w.agents.iter().position(|a| a.id == *m && a.alive) else {
                continue;
            };
            if w.agents[i].pos.dist(depart) <= 2 {
                w.reservations.release_all(*m);
                for it in w.items.iter_mut() {
                    if it.place == ItemPlace::Carried(*m) {
                        it.place = ItemPlace::Ground(depart);
                    }
                }
                let mut a: Agent = w.agents.remove(i);
                a.activity = Activity::Idle;
                a.activity_ticks = 0;
                a.path.clear();
                a.pos = depart;
                a.from = depart;
                w.overworld.away.push(a);
                gone.push(*m);
            } else {
                w.agents[i].caravan = None;
            }
        }
        let c = &mut w.overworld.caravans[ci];
        c.members = gone;
        if c.members.is_empty() {
            c.state = CaravanState::Done;
            c.finished = tick;
            c.log
                .push("nobody came to the gathering; the journey was given up".into());
            // What was loaded for it is set down where they were to gather (FIXLOG 2026-10-20
            // #166, C62: it was destroyed with the journey).
            let c = w.overworld.caravans[ci].clone();
            set_down(w, &c, "returned");
            continue;
        }
        c.state = CaravanState::Outbound { day: 0 };
        c.departed = tick;
        let (cid, id, leader, n) = (c.community, c.id, c.members[0], c.members.len() as u32);
        w.events.push(
            tick,
            Some(depart),
            Severity::Major,
            EventKind::CaravanLeft {
                caravan: id,
                community: cid,
                leader,
                members: n,
            },
        );
    }
}

/// One day of travel for every caravan off the map.
fn travel_day(w: &mut World) {
    let tick = w.meta.tick;
    let seed = w.meta.seed;
    let n = w.overworld.caravans.len();
    for ci in 0..n {
        let c = w.overworld.caravans[ci].clone();
        if matches!(
            c.state,
            CaravanState::Forming | CaravanState::Done | CaravanState::Lost
        ) {
            continue;
        }
        let site = w.overworld.site(site_of(&c.purpose)).unwrap().clone();
        let mut r = stream(
            seed,
            &[
                purpose::AGENT_SOCIAL,
                0xCA1,
                c.id as u64,
                tick / TICKS_PER_DAY,
            ],
        );
        let mut log = c.log.clone();
        let mut cargo = c.cargo.clone();
        let mut rations = c.rations;
        let mut members = c.members.clone();
        let mut hurt: Vec<AgentId> = Vec::new();
        let mut lost_members: Vec<AgentId> = Vec::new();
        let mut new_state = match c.state {
            CaravanState::Outbound { day } | CaravanState::Returning { day } => {
                let returning = matches!(c.state, CaravanState::Returning { .. });
                let leg = if returning { "home, day" } else { "day" };
                let mut stranded = None;
                let roll: f32 = r.gen();
                if roll < site.danger {
                    let kind: f32 = r.gen();
                    if kind < 0.45 {
                        let taken = cargo.pop().or_else(|| {
                            (rations > 0).then(|| (ItemKind::Meal, std::mem::take(&mut rations)))
                        });
                        if let Some(k) = taken {
                            log.push(format!(
                                "{} {}: robbed on the road, lost {} {}",
                                leg,
                                day + 1,
                                k.1,
                                lower(k.0)
                            ));
                        } else {
                            log.push(format!(
                                "{} {}: waylaid, but had nothing to take",
                                leg,
                                day + 1
                            ));
                        }
                    } else if kind < 0.8 {
                        let who = members[r.gen_range(0..members.len())];
                        hurt.push(who);
                        log.push(format!(
                            "{} {}: a fall on bad ground hurt {}",
                            leg,
                            day + 1,
                            name(w, who)
                        ));
                    } else {
                        log.push(format!("{} {}: lost the way in fog", leg, day + 1));
                        stranded = Some(CaravanState::Stranded {
                            days: r.gen_range(1..4),
                            then_return: returning,
                            back_day: day,
                        });
                    }
                    if r.gen::<f32>() < 0.08 && members.len() > 1 {
                        let who = members.remove(r.gen_range(0..members.len()));
                        lost_members.push(who);
                        log.push(format!(
                            "day {}: {} was lost on the road",
                            day + 1,
                            name(w, who)
                        ));
                    }
                } else {
                    // #1587 (FIXLOG 2026-11-17): food is gathered from the land, by season, by a
                    // named member; it no longer appears from nowhere on any day.
                    let season = crate::time::Calendar::from_tick(tick).season;
                    let land = match site.kind {
                        SiteKind::Forest | SiteKind::Lakeshore => 1.0,
                        SiteKind::Village => 0.6,
                        SiteKind::Mine | SiteKind::Ruin => 0.3,
                    };
                    let food = match season {
                        Season::Summer => Some(ItemKind::Berries),
                        Season::Autumn => Some(ItemKind::Root),
                        _ => None,
                    };
                    if let Some(food) = food {
                        let found = r.gen::<f32>() < 0.15 * land;
                        let skill_of = |w: &World, m: AgentId| {
                            w.overworld
                                .away
                                .iter()
                                .find(|a| a.id == m)
                                .map_or(0.0, |a| a.skills.level(Skill::Foraging))
                        };
                        let best = members.iter().copied().max_by(|a, b| {
                            skill_of(w, *a)
                                .partial_cmp(&skill_of(w, *b))
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                        if let (true, Some(who)) = (found, best) {
                            let amount = (members.len() as f32 * (1.0 + skill_of(w, who) / 10.0))
                                .round()
                                .max(1.0) as u32;
                            cargo.push((food, amount));
                            log.push(format!(
                                "{} {}: {} gathered {} {} by the way",
                                leg,
                                day + 1,
                                name(w, who),
                                amount,
                                lower(food)
                            ));
                            if let Some(a) = w.overworld.away.iter_mut().find(|a| a.id == who) {
                                a.skills.practise(Skill::Foraging, 2.0);
                            }
                        }
                    }
                }
                if let Some(s) = stranded {
                    s
                } else {
                    let day = day + 1;
                    if returning {
                        if day >= site.days {
                            CaravanState::Done
                        } else {
                            CaravanState::Returning { day }
                        }
                    } else if day >= site.days {
                        CaravanState::AtSite { day: 0 }
                    } else {
                        CaravanState::Outbound { day }
                    }
                }
            }
            CaravanState::AtSite { day } => {
                if day == 0 {
                    match c.purpose {
                        Purpose::Trade { carry, .. } | Purpose::Fetch { carry, .. } => {
                            let wanting = match c.purpose {
                                Purpose::Fetch { want, .. } => Some(want),
                                _ => None,
                            };
                            // The goods only: the rations are their own (#165).
                            let gave = match cargo.iter().position(|(k, _)| *k == carry) {
                                Some(ix) => cargo.remove(ix).1,
                                None => 0,
                            };
                            if gave == 0 {
                                log.push(format!("at {}: nothing left to trade", site.name));
                                // (Eaten or robbed on the road.)
                            } else {
                                // Sold at the settlement's price, paid from its own stores in what
                                // the valley is shortest of (trade.rs, C36).
                                let home = w
                                    .communities
                                    .iter()
                                    .find(|x| x.id == c.community)
                                    .map(|x| (x.home, x.members.len() as u32))
                                    .unwrap_or((c.depart_tile, 10));
                                // Priced by what it is (#274): marble is not granite.
                                let material = w.overworld.caravans[ci]
                                    .materials
                                    .get(&carry)
                                    .copied()
                                    .unwrap_or(crate::world::Material::AIR);
                                let (took, paid) = crate::trade::barter_wanting(
                                    w,
                                    site.id,
                                    c.community,
                                    home.0,
                                    home.1,
                                    carry,
                                    material,
                                    gave,
                                    members.len() as u32 * 10,
                                    wanting,
                                );
                                if took < gave {
                                    cargo.push((carry, gave - took));
                                }
                                let got: Vec<(ItemKind, u32)> =
                                    paid.iter().map(|(k, _, n)| (*k, *n)).collect();
                                for (k, m, _) in &paid {
                                    w.overworld.caravans[ci].materials.insert(*k, *m);
                                }
                                cargo.extend(got.iter().copied());
                                let got_s = got
                                    .iter()
                                    .map(|(k, n)| format!("{} {}", n, lower(*k)))
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                log.push(format!(
                                    "at {}: traded {} {} for {}",
                                    site.name,
                                    took,
                                    lower(carry),
                                    if got_s.is_empty() {
                                        "nothing".to_string()
                                    } else {
                                        got_s
                                    }
                                ));
                            }
                        }
                        Purpose::Scout { .. } => {
                            let found = site
                                .stores
                                .iter()
                                .filter(|(_, v)| **v >= 1.0)
                                .map(|(k, _)| lower(*k))
                                .collect::<Vec<_>>()
                                .join(", ");
                            log.push(format!(
                                "reached {}, a {}: they have {}",
                                site.name,
                                format!("{:?}", site.kind).to_lowercase(),
                                found
                            ));
                            if matches!(site.kind, SiteKind::Ruin | SiteKind::Forest) {
                                cargo.push((ItemKind::Tool, 1));
                            }
                        }
                    }
                    CaravanState::AtSite { day: 1 }
                } else {
                    CaravanState::Returning { day: 0 }
                }
            }
            CaravanState::Stranded {
                days,
                then_return,
                back_day,
            } => {
                if days <= 1 {
                    log.push("found the road again".into());
                    if then_return {
                        CaravanState::Returning { day: back_day }
                    } else {
                        CaravanState::Outbound { day: back_day }
                    }
                } else if rations == 0
                    && cargo.iter().all(|(k, _)| !k.is_food())
                    && r.gen::<f32>() < 0.35
                {
                    log.push("no food and no road; nothing more was heard".into());
                    CaravanState::Lost
                } else {
                    if days > 2 {
                        // Hungry days eat the food they carry: rations first, then goods.
                        if rations > 0 {
                            rations = rations.saturating_sub(members.len() as u32);
                        } else if let Some(ix) = cargo
                            .iter()
                            .position(|(k, _)| *k == ItemKind::Meal)
                            .or_else(|| cargo.iter().position(|(k, _)| k.is_food()))
                        {
                            cargo[ix].1 = cargo[ix].1.saturating_sub(members.len() as u32);
                            if cargo[ix].1 == 0 {
                                cargo.remove(ix);
                            }
                        }
                    }
                    CaravanState::Stranded {
                        days: days - 1,
                        then_return,
                        back_day,
                    }
                }
            }
            CaravanState::Lost | CaravanState::Done | CaravanState::Forming => c.state.clone(),
        };
        if members.is_empty() {
            new_state = CaravanState::Lost;
        }
        // Injuries and losses land on the real people.
        for who in &hurt {
            if let Some(a) = w.overworld.away.iter_mut().find(|a| a.id == *who) {
                a.injured_until = tick + TICKS_PER_DAY * 2;
                a.remember(tick, MemKind::Hardship, Subject::None, -0.5, 0.6, None);
            }
        }
        let was_lost =
            matches!(new_state, CaravanState::Lost) && !matches!(c.state, CaravanState::Lost);
        let mut lost_whole = 0;
        if was_lost {
            lost_whole = members.len() as u32;
            lost_members.extend(members.iter().copied());
            members.clear();
        }
        let home = w
            .communities
            .iter()
            .find(|x| x.id == c.community)
            .map(|x| x.home);
        {
            let cc = &mut w.overworld.caravans[ci];
            cc.log = log;
            cc.cargo = cargo;
            cc.rations = rations;
            cc.lost += lost_whole;
            cc.members = members.clone();
            cc.state = new_state.clone();
            if matches!(new_state, CaravanState::Done | CaravanState::Lost) {
                cc.finished = tick;
            }
        }
        // A night on the road: travellers sleep on what the day brought.
        for m in &members {
            if let Some(a) = w.overworld.away.iter_mut().find(|a| a.id == *m) {
                let anxious = a.traits.has(crate::mind::Trait::Cautious);
                let curious = a.traits.has(crate::mind::Trait::Curious);
                a.memories.consolidate(tick, anxious, curious);
            }
        }
        for who in lost_members {
            // Bring them home to the record: the person dies, off the map, as themselves; the
            // body is not here, so they are mourned in its absence (#566, FIXLOG 2026-11-02).
            if let Some(ix) = w.overworld.away.iter().position(|a| a.id == who) {
                let a = w.overworld.away.remove(ix);
                w.agents.push(a);
                crate::systems::life::die_away(w, who, "lost on the road".into());
            }
        }
        if was_lost {
            let last = w.overworld.caravans[ci]
                .log
                .last()
                .cloned()
                .unwrap_or_default();
            w.events.push(
                tick,
                home,
                Severity::Major,
                EventKind::CaravanLost {
                    caravan: c.id,
                    community: c.community,
                    members: c.members.len() as u32,
                    last,
                },
            );
        }
        let arrived = matches!(new_state, CaravanState::Done)
            && matches!(c.state, CaravanState::Returning { .. });
        if arrived {
            arrive(w, ci);
        }
    }
}

/// The caravan is back: members reappear at the departure tile, cargo is set down there to
/// be hauled, the journey becomes memory, and the outcome is recorded.
fn arrive(w: &mut World, ci: usize) {
    let tick = w.meta.tick;
    let c = w.overworld.caravans[ci].clone();
    let site_id = site_of(&c.purpose);
    let site_name = w
        .overworld
        .site(site_id)
        .map(|s| s.name.clone())
        .unwrap_or_default();
    let depart = c.depart_tile;
    let days = ((tick - c.departed) / TICKS_PER_DAY) as u32;
    let received = set_down(w, &c, "caravan");
    if let Some(s) = w.overworld.sites.iter_mut().find(|s| s.id == site_id) {
        if !s.known_by.contains(&c.community) {
            s.known_by.push(c.community);
        }
    }
    let eid = w.events.push(
        tick,
        Some(depart),
        Severity::Major,
        EventKind::CaravanReturned {
            caravan: c.id,
            community: c.community,
            leader: c.members[0],
            members: c.members.len() as u32,
            days,
            site: site_name.clone(),
            received: received.join(", "),
            story: c.log.join("; "),
        },
    );
    // They bring word of what happened out there (realm/live.rs).
    crate::realm::live::news(w, site_id, c.members[0]);
    let went_badly = c
        .log
        .iter()
        .any(|l| l.contains("lost") || l.contains("robbed") || l.contains("hurt"));
    for m in &c.members {
        if let Some(ix) = w.overworld.away.iter().position(|a| a.id == *m) {
            let mut a = w.overworld.away.remove(ix);
            a.caravan = None;
            a.last_caravan = tick;
            a.pos = depart;
            a.from = depart;
            a.path.clear();
            a.activity = Activity::Idle;
            a.activity_ticks = 0;
            a.needs.hunger = (a.needs.hunger - 0.25).max(0.12);
            a.needs.rest = (a.needs.rest - 0.3).max(0.1);
            a.skills.practise(Skill::Hauling, 4.0);
            a.remember(
                tick,
                MemKind::Discovery,
                Subject::Place(depart),
                // A journey come home well is a vivid memory (C71: with the nights the cold
                // half of the day, a scout's hardships on the ground outvoted it at
                // consolidation and the journey was never kept).
                if went_badly { -0.2 } else { 0.7 },
                0.9,
                Some(eid),
            );
            w.agents.push(a);
        }
    }
    if let Some(cc) = w.communities.iter_mut().find(|x| x.id == c.community) {
        cc.discoveries
            .push((format!("the road to {site_name}"), depart, eid));
    }
}

/// Set a caravan's goods and rations down by its departure tile, each with the material and
/// quality it went out with (or was paid in); returns "n kind" for each.
fn set_down(w: &mut World, c: &Caravan, cause: &'static str) -> Vec<String> {
    let depart = c.depart_tile;
    let mut received = Vec::new();
    let rations = (c.rations > 0).then_some((ItemKind::Meal, c.rations));
    for (k, n) in c.cargo.iter().copied().chain(rations) {
        if n == 0 {
            continue;
        }
        let off = received.len() as i32 % 3 - 1;
        let drop = TilePos::new(
            (depart.x + off).clamp(1, w.grid.width - 2),
            depart.y,
            depart.z,
        );
        let drop = if w.grid.walkable(drop) { drop } else { depart };
        let m = c
            .materials
            .get(&k)
            .copied()
            .unwrap_or(crate::world::Material::AIR);
        let q = c.qualities.get(&k).copied().unwrap_or(0.6);
        w.create_item_of(k, m, n, ItemPlace::Ground(drop), q, cause);
        received.push(format!("{} {}", n, lower(k)));
    }
    received
}

fn name(w: &World, a: AgentId) -> String {
    w.agents
        .iter()
        .chain(w.overworld.away.iter())
        .find(|x| x.id == a)
        .map(|x| x.name.clone())
        .unwrap_or_else(|| format!("#{}", a.0))
}
