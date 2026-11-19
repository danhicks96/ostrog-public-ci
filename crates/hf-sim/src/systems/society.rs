//! Community life: roles assigned daily by skill, newcomers judged by the welcome norm,
//! cohesion from members' opinions and needs, splits when cohesion collapses, and trade
//! between communities with complementary surpluses. Runs on an hourly stripe.

use rand::Rng;

use crate::agents::Species;
use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, CommunityId, TilePos};
use crate::items::{ItemKind, ItemPlace};
use crate::mind::{Activity, Skill};
use crate::rng::{purpose, stream};
use crate::society::WelcomeNorm;
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::world_state::World;

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick.is_multiple_of(200) {
        leavers(w);
    }
    if !tick.is_multiple_of(TICKS_PER_HOUR) {
        return;
    }
    newcomers(w);
    judge_newcomers(w);
    if (tick / TICKS_PER_HOUR) % 24 == 9 {
        // #454: a role is the work someone has been doing (duties.rs), not one of each handed
        // to the best hand by skill.
        crate::duties::daily(w);
        cohesion_and_splits(w);
    }
    if matches!((tick / TICKS_PER_HOUR) % 24, 9 | 15) {
        trade(w);
    }
}

fn alive_members(w: &World, cid: CommunityId) -> Vec<AgentId> {
    w.communities
        .iter()
        .find(|c| c.id == cid)
        .map(|c| {
            c.members
                .iter()
                .copied()
                .filter(|m| w.agents.iter().any(|a| a.id == *m && a.alive))
                .collect()
        })
        .unwrap_or_default()
}

/// Wanderers arrive at the map edge every few days (seeded), heading for the nearest community.
fn newcomers(w: &mut World) {
    if !w.meta.autoplan || w.communities.iter().all(|c| c.dissolved) {
        return;
    }
    let tick = w.meta.tick;
    // The day key keeps the absolute tick (one roll per world-day); the grace period after the
    // arrival counts from the arrival.
    let day = tick / TICKS_PER_DAY;
    let days_here = w.since_arrival() / TICKS_PER_DAY;
    // One roll per day at 07:00: about one arrival every three days early on, rarer later.
    if (tick % TICKS_PER_DAY) / TICKS_PER_HOUR != 7 || days_here < 3 {
        return;
    }
    let pop = w
        .agents
        .iter()
        .filter(|a| a.alive && a.species == Species::Person)
        .count();
    let mut r = stream(w.meta.seed, &[purpose::AGENT_BIRTH, 0xA11, day]);
    let base = if pop < 12 {
        0.34
    } else if pop < 24 {
        0.2
    } else {
        0.08
    };
    // Word travels: a valley with a thin larder draws fewer strangers. The Engine then sets how
    // often the door is knocked on; it does not decide who knocks.
    let food: u32 = w
        .items
        .iter()
        .filter(|it| it.kind.is_food())
        .map(|it| it.count)
        .sum();
    let per_head = food as f32 / pop.max(1) as f32;
    let chance =
        (base * (per_head / 12.0).clamp(0.15, 1.0) * w.engine_rates().arrivals).clamp(0.0, 0.95);
    if r.gen::<f32>() > chance {
        return;
    }
    let g = &w.grid;
    let side = r.gen_range(0..4);
    let (x, y) = match side {
        0 => (r.gen_range(2..g.width - 2), 2),
        1 => (g.width - 3, r.gen_range(2..g.height - 2)),
        2 => (r.gen_range(2..g.width - 2), g.height - 3),
        _ => (2, r.gen_range(2..g.height - 2)),
    };
    let p = g.surface(x, y);
    if !g.walkable(p) {
        return;
    }
    // From an edge tile the camp can be reached from (#254: four arrivals on a rim a house
    // had shut off stood for days, starved, ate their dead and froze).
    let nearest_home = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .map(|c| c.home)
        .min_by_key(|h| h.dist(p));
    let p = match nearest_home {
        Some(home) => match arrival_tile(w, p, side, home) {
            Some(q) => q,
            None => return,
        },
        None => p,
    };
    let n = if r.gen::<f32>() < 0.25 { 2 } else { 1 };
    let ids = w.arrive_party(n, p, None);
    // From a settlement on that side of the valley, of its people and species (P3); from
    // nowhere the world knows, human.
    let edge = [
        crate::expedition::Edge::North,
        crate::expedition::Edge::East,
        crate::expedition::Edge::South,
        crate::expedition::Edge::West,
    ][side];
    let from: Vec<u32> = w
        .overworld
        .sites
        .iter()
        .filter(|s| s.edge == edge && s.realm_site.is_some() && s.people > 0)
        .map(|s| s.id)
        .collect();
    if !from.is_empty() {
        let site = from[r.gen_range(0..from.len())];
        // A rare people's households (P5) send their own now and then.
        let minor = r.gen::<f32>() < 0.25;
        let pick = if minor {
            crate::species::minority_of_site(w, site).or_else(|| crate::species::of_site(w, site))
        } else {
            crate::species::of_site(w, site)
        };
        if let Some((sp, people)) = pick {
            // Raised in the settlement's culture, or a minority's own first culture (P7).
            let culture = if minor && crate::species::minority_of_site(w, site).is_some() {
                Some(people as u32)
            } else {
                crate::species::culture_of_site(w, site)
            };
            for id in &ids {
                crate::species::give(w, *id, sp, Some(people));
                if let Some(c) = culture {
                    crate::species::raise_in(w, *id, c);
                }
            }
        }
    }
    // From somewhere: what their families remember of the old valley comes with them.
    crate::lore::newcomers(w, &ids);
    // They tell what they passed on the way in, if anything: word of the lost comes only from
    // someone who was near them.
    let home = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .map(|c| c.home)
        .min_by_key(|h| h.dist(p));
    if let Some(home) = home {
        crate::opening::passed_on_the_way(w, &ids, p, home);
    }
    // Head for the nearest community home.
    if let Some(home) = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .map(|c| c.home)
        .min_by_key(|h| h.dist(p))
    {
        for id in ids {
            if let Some(i) = w.agents.iter().position(|a| a.id == id) {
                w.agents[i].needs.hunger = 0.5;
                if w.path_agent_to(i, home) {
                    w.agents[i].activity = Activity::Wandering { to: home };
                } else {
                    // No way in after all: back the way they came (#254).
                    w.agents[i].leaving = true;
                }
            }
        }
    }
}

/// People who were turned away head for the nearest map edge and leave the valley.
/// An edge tile from which `home` can be reached (#254): `p` itself if it can, else the
/// nearest reachable tile on that side, else on another side; None when no side has one.
pub fn arrival_tile(w: &World, p: TilePos, side: usize, home: TilePos) -> Option<TilePos> {
    let reach = |q: TilePos| w.nav.as_ref().is_some_and(|n| n.reach(q, home));
    if reach(p) {
        return Some(p);
    }
    use crate::expedition::Edge;
    let edges = [Edge::North, Edge::East, Edge::South, Edge::West];
    std::iter::once(side)
        .chain((0..4).filter(move |k| *k != side))
        .find_map(|k| crate::systems::expedition::edge_tile(w, edges[k], home))
}

fn leavers(w: &mut World) {
    let tick = w.meta.tick;
    let (wd, ht) = (w.grid.width, w.grid.height);
    let n = w.agents.len();
    for i in 0..n {
        let a = &w.agents[i];
        if !a.alive || !a.leaving || a.community.is_some() {
            continue;
        }
        let p = a.pos;
        let edge_dist = p.x.min(p.y).min(wd - 1 - p.x).min(ht - 1 - p.y);
        if edge_dist <= 2 {
            let id = a.id;
            leave_valley(w, i);
            w.events.push(
                tick,
                Some(p),
                Severity::Notable,
                EventKind::Left { agent: id },
            );
            continue;
        }
        if a.path.is_empty() && !a.is_moving() {
            let target = [
                TilePos::new(p.x, 1, p.z),
                TilePos::new(1, p.y, p.z),
                TilePos::new(p.x, ht - 2, p.z),
                TilePos::new(wd - 2, p.y, p.z),
            ]
            .into_iter()
            .map(|t| w.grid.surface(t.x, t.y))
            .min_by_key(|t| t.dist(p))
            .unwrap();
            w.agents[i].activity = Activity::Wandering { to: target };
            w.agents[i].idle_ticks = 0;
            let _ = w.path_agent_to(i, target);
        }
    }
}

/// Someone walks off the map for good. They are kept, not alive, so the feed can still name
/// them; what they carry, the tool in their hand and the clothes they wear go with them and
/// out of the ledger (`consume_item`), so nothing is left in the hands of someone who is no
/// longer here.
pub fn leave_valley(w: &mut World, i: usize) {
    let id = w.agents[i].id;
    w.reservations.release_all(id);
    let carried: Vec<(crate::ids::ItemId, u32)> = w
        .items
        .iter()
        .filter(|it| it.place == ItemPlace::Carried(id) || it.place == ItemPlace::Held(id))
        .map(|it| (it.id, it.count))
        .collect();
    for (item, n) in carried {
        w.consume_item(item, n, "carried off");
    }
    w.unpair(id);
    let a = &mut w.agents[i];
    a.tool = None;
    a.tool_quality = 0.0;
    a.tool_wear = 0.0;
    a.alive = false;
    a.activity = Activity::Idle;
    a.path.clear();
    // #1322: the realm holds who arrived, not who set out; a valley departure's destination is
    // founded (or grown) only now, the first of them to actually step off the map.
    let bound_for = a.bound_for.take();
    if let Some(pending) = bound_for {
        crate::realm::live::settle_arrived(w, pending);
    }
}

/// Unaffiliated people near a community are judged by its welcome norm.
fn judge_newcomers(w: &mut World) {
    let tick = w.meta.tick;
    let candidates: Vec<(AgentId, TilePos)> = w
        .agents
        .iter()
        .filter(|a| a.alive && a.species == Species::Person && a.community.is_none())
        // A band camped near the settlement is not asking to join (threat.rs), and nor is
        // anyone on their way out of the valley.
        .filter(|a| a.band.is_none() && !a.leaving)
        // Nor is one kept after a fight (C76, siege.rs): ransomed or taken in in time.
        .filter(|a| a.held.is_none())
        // The lost are not strangers: they rejoin when they meet the others (opening::reunite).
        .filter(|a| {
            !w.survivor_pool.iter().any(|s| {
                matches!(s.state, crate::opening::SurvivorState::Lost { agent, .. } if agent == a.id)
            })
        })
        .map(|a| (a.id, a.pos))
        .collect();
    // One read for the whole stripe: tension() walks every agent and every designation.
    let welcome_bar = w.engine_rates().welcome;
    for (who, pos) in candidates {
        let Some(cid) = w
            .communities
            .iter()
            .filter(|c| !c.dissolved && c.home.dist(pos) <= 14)
            .map(|c| c.id)
            .next()
        else {
            continue;
        };
        // Judged by this community recently? (three-day cooldown, kept on the agent)
        let key = 0x4000_0000 | cid.0;
        let already = w
            .agents
            .iter()
            .find(|a| a.id == who)
            .and_then(|a| a.work_cooldowns.get(&key).copied())
            .map(|t| t > tick)
            .unwrap_or(false);
        if already {
            continue;
        }
        if let Some(a) = w.agents.iter_mut().find(|a| a.id == who) {
            a.work_cooldowns.insert(key, tick + TICKS_PER_DAY * 3);
        }
        let members = alive_members(w, cid);
        let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
        let food: u32 = w
            .items
            .iter()
            .filter(|it| {
                it.kind.is_food() && matches!(it.place, ItemPlace::Ground(p) if p.dist(home) <= 40)
            })
            .map(|it| it.count)
            .sum();
        let norm = w.communities.iter().find(|c| c.id == cid).unwrap().welcome;
        // The Engine sets how fussy the welcome is, which is the other half of pacing: how many
        // people arrive and how many are let in are different questions with different answers.
        // What the members believe about strangers makes the welcome warmer or cooler.
        let joiner = w.agents.iter().find(|a| a.id == who).map(|a| {
            (
                a.people,
                a.beliefs.lean(crate::mind::beliefs::Topic::OtherRaces),
            )
        });
        let Some((kind, own)) = joiner else { continue };
        let wary = members
            .iter()
            .filter_map(|m| w.agents.iter().find(|a| a.id == *m))
            // A guest is to be fed (Hospitality, P7) warms it as much as fear of strangers
            // cools it; a guest of another kind is judged too by what each holds of other
            // kinds (P9).
            .map(|a| {
                a.beliefs.lean(crate::mind::beliefs::Topic::Strangers)
                    - a.beliefs.lean(crate::mind::beliefs::Topic::Hospitality)
                    - if a.people != kind {
                        a.beliefs.lean(crate::mind::beliefs::Topic::OtherRaces)
                    } else {
                        0.0
                    }
            })
            .sum::<f32>()
            / members.len().max(1) as f32;
        // The stranger's own mind matters as much (P9): one who holds other kinds best kept
        // apart will not live where most are of another kind.
        let other: Vec<&crate::agents::Agent> = members
            .iter()
            .filter_map(|m| w.agents.iter().find(|a| a.id == *m))
            .filter(|a| a.people != kind)
            .collect();
        if own < -0.4 && other.len() * 2 > members.len() {
            let plural = other[0].species_def().plural.clone();
            w.events.push(
                tick,
                Some(pos),
                Severity::Notable,
                EventKind::WouldNotStay {
                    agent: who,
                    community: cid,
                    because: format!(
                        "they hold that other kinds are best kept apart, and most here are {plural}"
                    ),
                },
            );
            if let Some(a) = w.agents.iter_mut().find(|a| a.id == who) {
                a.leaving = true;
            }
            continue;
        }
        let bar = welcome_bar * (1.0 + crate::mind::beliefs::CAP * wary);
        let verdict = match norm {
            WelcomeNorm::Open => Ok(()),
            WelcomeNorm::Closed => Err("strangers are not taken in"),
            WelcomeNorm::WhenFed => {
                // Fed means the members themselves are not going hungry, or there is a
                // store to share from.
                let mean_hunger = members
                    .iter()
                    .filter_map(|m| w.agents.iter().find(|a| a.id == *m))
                    .map(|a| a.needs.hunger)
                    .sum::<f32>()
                    / members.len().max(1) as f32;
                // A choosier welcome tightens BOTH gates: better fed before they will share, and
                // more put by. Dividing here would have made hard times more generous, which is
                // the opposite of what the Engine is for.
                let bar = bar.clamp(0.5, 2.0);
                let need = ((members.len() as u32 + 1) as f32 * 3.0 * bar).ceil() as u32;
                if mean_hunger >= (0.45 * bar).min(0.95) || food >= need.max(1) {
                    Ok(())
                } else {
                    Err("there is not enough food to share")
                }
            }
        };
        match verdict {
            Ok(()) => {
                w.join_community(who, cid);
                let joined = w.events.push(
                    tick,
                    Some(pos),
                    Severity::Major,
                    EventKind::Joined {
                        agent: who,
                        community: cid,
                    },
                );
                crate::systems::rites::joined(w, who, cid, joined);
                for m in members.iter().take(6) {
                    if let Some(a) = w.agents.iter_mut().find(|a| a.id == *m) {
                        a.relate(who, 0.05, 0.05, tick);
                    }
                }
            }
            Err(reason) => {
                w.events.push(
                    tick,
                    Some(pos),
                    Severity::Notable,
                    EventKind::TurnedAway {
                        agent: who,
                        community: cid,
                        reason: reason.into(),
                    },
                );
                if let Some(a) = w.agents.iter_mut().find(|a| a.id == who) {
                    a.remember(
                        tick,
                        crate::mind::MemKind::Hardship,
                        crate::mind::Subject::Place(home),
                        -0.6,
                        0.7,
                        None,
                    );
                    a.leaving = true;
                    // Move on: wander away from this home.
                    let away = TilePos::new(
                        (pos.x + (pos.x - home.x).signum() * 20).clamp(2, w.grid.width - 3),
                        (pos.y + (pos.y - home.y).signum() * 20).clamp(2, w.grid.height - 3),
                        pos.z,
                    );
                    a.activity = Activity::Wandering { to: away };
                }
                if let Some(i) = w.agents.iter().position(|a| a.id == who) {
                    let to = match &w.agents[i].activity {
                        Activity::Wandering { to } => *to,
                        _ => pos,
                    };
                    let _ = w.path_agent_to(i, to);
                }
            }
        }
    }
}

/// Cohesion = mean opinion among members, lifted by being fed and sheltered. Below 0.2 with
/// enough people, the most disaffected clique leaves to found a new community nearby.
fn cohesion_and_splits(w: &mut World) {
    let ids: Vec<CommunityId> = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .map(|c| c.id)
        .collect();
    for cid in ids {
        let members = alive_members(w, cid);
        if members.len() < 2 {
            continue;
        }
        let mut sum = 0.0;
        let mut n = 0.0;
        let mut hungry = 0;
        for m in &members {
            let a = w.agents.iter().find(|a| a.id == *m).unwrap();
            if a.needs.hunger < 0.2 {
                hungry += 1;
            }
            for o in &members {
                if o != m {
                    sum += a.opinion_of(*o);
                    n += 1.0;
                }
            }
        }
        let mean_opinion = if n > 0.0 { sum / n } else { 0.0 };
        let fed = 1.0 - hungry as f32 / members.len() as f32;
        let cohesion = (0.5 + mean_opinion + (fed - 0.5) * 0.3).clamp(0.0, 1.0);
        if std::env::var("HF_TRACE_SOCIETY").is_ok() {
            eprintln!(
                "COHESION {:?} members {} mean_opinion {:.2} fed {:.2} cohesion {:.2}",
                cid,
                members.len(),
                mean_opinion,
                fed,
                cohesion
            );
        }
        if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
            c.cohesion = cohesion;
        }
        if cohesion < 0.4 && members.len() >= 6 {
            // Leader: the member with the lowest mean opinion of the others; faction: those who
            // like the leader more than the rest.
            let leader = members
                .iter()
                .copied()
                .min_by(|x, y| {
                    let ax = w.agents.iter().find(|a| a.id == *x).unwrap();
                    let ay = w.agents.iter().find(|a| a.id == *y).unwrap();
                    let mx: f32 = members
                        .iter()
                        .filter(|o| *o != x)
                        .map(|o| ax.opinion_of(*o))
                        .sum();
                    let my: f32 = members
                        .iter()
                        .filter(|o| *o != y)
                        .map(|o| ay.opinion_of(*o))
                        .sum();
                    mx.partial_cmp(&my).unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap();
            let faction: Vec<AgentId> = members
                .iter()
                .copied()
                .filter(|m| {
                    if *m == leader {
                        return true;
                    }
                    let a = w.agents.iter().find(|a| a.id == *m).unwrap();
                    let to_leader = a.opinion_of(leader);
                    let to_rest: f32 = members
                        .iter()
                        .filter(|o| *o != m && **o != leader)
                        .map(|o| a.opinion_of(*o))
                        .sum::<f32>()
                        / (members.len() - 2).max(1) as f32;
                    to_leader > to_rest + 0.1
                })
                .collect();
            if std::env::var("HF_TRACE_SOCIETY").is_ok() {
                eprintln!(
                    "SPLIT? leader {:?} faction {} of {}",
                    leader,
                    faction.len(),
                    members.len()
                );
            }
            if faction.len() < 2 || faction.len() >= members.len() {
                continue;
            }
            let cause = if fed < 0.6 {
                "hunger and bad blood".to_string()
            } else {
                "bad blood between the factions".to_string()
            };
            split_off(w, cid, leader, &faction, cause);
        }
    }
}

/// `faction`, led by `leader`, leaves community `cid` to found its own at a flat spot 18 to 30
/// tiles away (on record as `Split`): the new community's id, or None with no place found.
/// Also used by heresy.rs for those who hold the signs apart.
pub(crate) fn split_off(
    w: &mut World,
    cid: CommunityId,
    leader: AgentId,
    faction: &[AgentId],
    cause: String,
) -> Option<CommunityId> {
    let tick = w.meta.tick;
    let home = w.communities.iter().find(|c| c.id == cid)?.home;
    // New home: a flat spot 18..30 tiles away, reachable.
    let mut r = stream(w.meta.seed, &[purpose::AGENT_SOCIAL, cid.0 as u64, tick]);
    let mut new_home = None;
    for _ in 0..12 {
        let dx = r.gen_range(18..30) * if r.gen_bool(0.5) { 1 } else { -1 };
        let dy = r.gen_range(-12..12);
        let near = TilePos::new(
            (home.x + dx).clamp(6, w.grid.width - 7),
            (home.y + dy).clamp(6, w.grid.height - 7),
            home.z,
        );
        if let Some(site) = crate::systems::settlement::find_flat_area(w, near, 3, 3, 1) {
            if site.dist(home) >= 15 {
                new_home = Some(site);
                break;
            }
        }
    }
    if std::env::var("HF_TRACE_SOCIETY").is_ok() {
        eprintln!("SPLIT new_home {:?}", new_home);
    }
    let new_home = new_home?;
    let new_cid = w.found_community(new_home);
    for m in faction {
        w.join_community(*m, new_cid);
        if let Some(i) = w.agents.iter().position(|a| a.id == *m) {
            w.agents[i].activity = Activity::Wandering { to: new_home };
            let _ = w.path_agent_to(i, new_home);
            w.agents[i].remember(
                tick,
                crate::mind::MemKind::Hardship,
                crate::mind::Subject::Place(home),
                -0.5,
                0.8,
                None,
            );
        }
    }
    w.events.push(
        tick,
        Some(home),
        Severity::Major,
        EventKind::Split {
            from: cid,
            to: new_cid,
            leader,
            members: faction.len() as u32,
            cause,
        },
    );
    Some(new_cid)
}

/// Trade: when one community has a surplus of a kind the other lacks, a carrier from the
/// richer side takes a load over and returns with what the other can spare.
fn trade(w: &mut World) {
    let tick = w.meta.tick;
    let live: Vec<(CommunityId, TilePos, u64)> = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .map(|c| (c.id, c.home, c.last_trade))
        .collect();
    if live.len() < 2 {
        return;
    }
    let stock = |w: &World, home: TilePos, kind: ItemKind| -> u32 {
        w.items
            .iter()
            .filter(|it| {
                it.kind == kind && matches!(it.place, ItemPlace::Ground(p) if p.dist(home) <= 18)
            })
            .map(|it| it.count)
            .sum()
    };
    let kinds = [
        ItemKind::Meal,
        ItemKind::Log,
        ItemKind::StoneBlock,
        ItemKind::GrainSheaf,
        ItemKind::Berries,
    ];
    for i in 0..live.len() {
        for j in 0..live.len() {
            if i == j {
                continue;
            }
            let (a_id, a_home, a_last) = live[i];
            let (b_id, b_home, _) = live[j];
            if tick - a_last < TICKS_PER_DAY {
                continue;
            }
            let a_pop = alive_members(w, a_id).len() as u32;
            let b_pop = alive_members(w, b_id).len() as u32;
            // What A has plenty of and B lacks.
            if std::env::var("HF_TRACE_SOCIETY").is_ok() {
                let st: Vec<String> = kinds
                    .iter()
                    .map(|k| {
                        format!(
                            "{:?} a{} b{}",
                            k,
                            stock(w, a_home, *k),
                            stock(w, b_home, *k)
                        )
                    })
                    .collect();
                eprintln!(
                    "TRADE? tick {} {:?}->{:?} stocks {:?}",
                    tick, a_id, b_id, st
                );
            }
            let give = kinds.iter().copied().find(|k| {
                stock(w, a_home, *k) >= 12 + a_pop * 2 && stock(w, b_home, *k) < b_pop * 2
            });
            let Some(give) = give else { continue };
            // What B can spare in return (any kind B has plenty of and A lacks), or nothing.
            // (#243: by the old rule a hamlet of five never had "plenty" of anything while a
            // town of fifty always "lacked", so 104 trades in 104 brought nothing back.) B
            // returns a kind it holds more of a head than A does, beyond its own four days.
            let receive = kinds.iter().copied().find(|k| {
                *k != give && returns(stock(w, b_home, *k), b_pop, stock(w, a_home, *k), a_pop)
            });
            // Carrier: an idle member of A who can reach B's home.
            let members = alive_members(w, a_id);
            let Some(carrier) = members.iter().copied().find(|m| {
                let a = w.agents.iter().find(|a| a.id == *m).unwrap();
                !a.activity.is_asleep()
                    && !matches!(
                        a.activity,
                        Activity::Working { .. }
                            | Activity::Eating { .. }
                            | Activity::Foraging { .. }
                    )
                    && w.nav
                        .as_ref()
                        .map(|n| n.reach(a.pos, b_home))
                        .unwrap_or(false)
            }) else {
                if std::env::var("HF_TRACE_SOCIETY").is_ok() {
                    eprintln!("TRADE no carrier for give {:?}", give);
                }
                continue;
            };
            // Move the goods: take from A's stacks near home, place at B's home; and back.
            let amount = 8u32;
            let load = take_items(w, a_home, give, amount);
            let moved = load.n;
            if moved == 0 {
                continue;
            }
            w.create_item_of(
                give,
                load.material,
                moved,
                ItemPlace::Ground(b_home.offset(1, 1, 0)),
                load.quality,
                "trade",
            );
            let mut received_txt = "nothing".to_string();
            if let Some(rk) = receive {
                let load = take_items(w, b_home, rk, amount);
                let back = load.n;
                if back > 0 {
                    w.create_item_of(
                        rk,
                        load.material,
                        back,
                        ItemPlace::Ground(a_home.offset(1, 1, 0)),
                        load.quality,
                        "trade",
                    );
                    received_txt = format!("{back} {}", kind_name(rk));
                }
            }
            if let Some(i) = w.agents.iter().position(|a| a.id == carrier) {
                // The carrier walks there and back (visible), then resumes life.
                w.agents[i].activity = Activity::Wandering { to: b_home };
                let _ = w.path_agent_to(i, b_home);
                w.agents[i].skills.practise(Skill::Hauling, 1.0);
                w.agents[i].remember(
                    tick,
                    crate::mind::MemKind::Discovery,
                    crate::mind::Subject::Place(b_home),
                    0.3,
                    0.5,
                    None,
                );
            }
            if let Some(c) = w.communities.iter_mut().find(|c| c.id == a_id) {
                c.last_trade = tick;
            }
            w.events.push(
                tick,
                Some(a_home),
                Severity::Major,
                EventKind::Traded {
                    from: a_id,
                    to: b_id,
                    carrier,
                    gave: format!("{moved} {}", kind_name(give)),
                    received: received_txt,
                },
            );
            return;
        }
    }
}

/// Whether B, holding `sb` of a kind with `b_pop` people, sends some back to A, holding `sa`
/// with `a_pop` (C69, #243): beyond B's own four days, and more a head than A has.
pub fn returns(sb: u32, b_pop: u32, sa: u32, a_pop: u32) -> bool {
    // Judged against the receiver's own need (#260: asking the town that lacks to hold more
    // a head than the hamlet that gives meant 115 trades in 115 brought nothing back): what
    // B holds beyond four days of its own use, in a kind A lacks.
    sb > b_pop.max(1) * 4 && sa < a_pop.max(1) * 2
}

/// The plain words for a kind (#215: "carried 8 grainsheaf" was the Rust variant).
pub fn kind_name(k: ItemKind) -> String {
    k.label().to_string()
}

/// What `take_items` loaded: how many, of what (the material most of it was), and how good.
pub(crate) struct Load {
    pub n: u32,
    pub material: crate::world::Material,
    pub quality: f32,
}

/// Remove up to `amount` units of `kind` from ground stacks within 18 tiles of `home`. Never a
/// thing with a story (the White Hart's hide is no trade good) nor a stack someone holds a lease
/// on (FIXLOG 2026-10-20 #169, C62).
pub(crate) fn take_items(w: &mut World, home: TilePos, kind: ItemKind, amount: u32) -> Load {
    let mut left = amount;
    let ids: Vec<(crate::ids::ItemId, u32, crate::world::Material, f32)> = w
        .items
        .iter()
        .filter(|it| {
            it.kind == kind
                && it.story.is_none()
                && matches!(it.place, ItemPlace::Ground(p) if p.dist(home) <= 18)
                && w.reservations
                    .holder(crate::work::ResKey::Item(it.id))
                    .is_none()
        })
        .map(|it| (it.id, it.count, it.material, it.quality))
        .collect();
    let mut by_material: std::collections::BTreeMap<crate::world::Material, u32> =
        Default::default();
    let mut q = 0.0;
    for (id, count, m, quality) in ids {
        if left == 0 {
            break;
        }
        let n = count.min(left);
        w.consume_item(id, n, "trade");
        *by_material.entry(m).or_default() += n;
        q += quality * n as f32;
        left -= n;
    }
    let n = amount - left;
    Load {
        n,
        material: by_material
            .iter()
            .max_by_key(|(m, k)| (**k, std::cmp::Reverse(**m)))
            .map(|(m, _)| *m)
            .unwrap_or(crate::world::Material::AIR),
        quality: if n > 0 { q / n as f32 } else { 0.6 },
    }
}
