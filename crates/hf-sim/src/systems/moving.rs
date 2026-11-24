//! Moving camp (C60, docs/design-homes.md section 5). A community keeps a record of what has
//! gone hard for it lately (deaths that were not of old age, raids on its store, the rock coming
//! down near home); when enough has, someone it touched proposes leaving the site, and each of
//! the rest goes with them or stays, for reasons of their own, each recorded. Few followers:
//! nobody goes. Many on both sides: a split. Nearly all: the whole community moves, within the
//! valley if a better place is found there, else out of it to settle in the world, which is
//! the third way a valley's story can end, beside dying out and splitting. When nobody of a
//! community is left in the valley, its end is recorded.
use crate::fmath::Det;

use rand::Rng;

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, CommunityId, TilePos};
use crate::project::{open_of, ProjectKind};
use crate::rng::{purpose, stream};
use crate::time::TICKS_PER_DAY;
use crate::world_state::World;

/// How long a hardship is remembered for this purpose.
pub const HARD_DAYS: u64 = 60;
/// How many hardships in that time move someone to propose leaving.
pub const HARD_ENOUGH: usize = 4;

/// Each tick: what has gone hard for each community (C60).
pub fn observe(w: &mut World) {
    let tick = w.meta.tick;
    let events = w.last_events.clone();
    for e in &events {
        let found: Option<(CommunityId, String)> = match &e.kind {
            EventKind::Died { agent, cause } if cause != "old age" => w
                .agents
                .iter()
                .find(|a| a.id == *agent && a.species == crate::agents::Species::Person)
                .and_then(|a| a.community.map(|c| (c, hard_death(&a.name, cause)))),
            // A band's raiding is one hardship however many nights it lasts (below): in C60's
            // first soaks one band camped for days and its nightly raids alone moved a village.
            // Food guests were told they could take is no hardship (#213): a hospitable
            // village broke up over its own hospitality.
            EventKind::Raided { open: true, .. } => None,
            EventKind::Raided { band, .. } => {
                e.at.and_then(|p| nearest_community(w, p, 60)).map(|c| {
                    (
                        c,
                        format!("strangers from {} took from the store", band_name(w, *band)),
                    )
                })
            }
            EventKind::Collapse { at, .. } | EventKind::UpperFloorFell { at, .. } => {
                nearest_community(w, *at, 45).map(|c| (c, "the rock came down".to_string()))
            }
            EventKind::Breached { .. } => {
                e.at.and_then(|p| nearest_community(w, p, 60))
                    .map(|c| (c, "the deep broke through".to_string()))
            }
            _ => None,
        };
        if let Some((cid, what)) = found {
            if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
                // A night's raiding is one hardship, not one per armful.
                let again = c
                    .hardships
                    .iter()
                    .any(|(t, s, _)| *s == what && tick - t < 20 * TICKS_PER_DAY);
                if !again {
                    c.hardships.push((tick, what, e.id));
                }
            }
        }
    }
    let cut = tick.saturating_sub(HARD_DAYS * TICKS_PER_DAY);
    for c in w.communities.iter_mut() {
        c.hardships.retain(|(t, _, _)| *t >= cut);
    }
}

/// A death as a hardship, in words: "Ana died of hunger", "Ulvane was lost on the road".
fn hard_death(name: &str, cause: &str) -> String {
    let how = crate::opening::died_how(cause);
    if how.starts_with("of ") {
        format!("{name} died {how}")
    } else {
        format!("{name} was {how}")
    }
}

fn band_name(w: &World, band: u32) -> String {
    w.bands
        .iter()
        .find(|b| b.id == band)
        .map(|b| b.site_name.clone())
        .unwrap_or_else(|| "beyond the valley".into())
}

fn nearest_community(w: &World, p: TilePos, radius: i32) -> Option<CommunityId> {
    w.communities
        .iter()
        .filter(|c| !c.dissolved && c.home.z == p.z && c.home.dist(p) <= radius)
        .min_by_key(|c| (c.home.dist(p), c.id))
        .map(|c| c.id)
}

fn members(w: &World, cid: CommunityId) -> Vec<usize> {
    (0..w.agents.len())
        .filter(|&i| {
            let a = &w.agents[i];
            a.alive && a.community == Some(cid) && a.species == crate::agents::Species::Person
        })
        .collect()
}

/// Daily (from systems/projects.rs): someone proposes leaving a failing site, and the
/// community answers.
pub fn consider(w: &mut World, cid: CommunityId) {
    if w.meta.settled {
        return;
    }
    if open_of(w, cid, |k| matches!(k, ProjectKind::Move { .. })).is_some() {
        return;
    }
    let tick = w.meta.tick;
    let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
        return;
    };
    // Not twice in a season (#208: kept on the community; the project list is capped at
    // fifty and the record could fall out of it inside the twenty days).
    if c.last_move_proposal > 0 && tick.saturating_sub(c.last_move_proposal) < 20 * TICKS_PER_DAY {
        return;
    }
    // A death weighs more in a hamlet than in a town: four hardships for a small community,
    // one more for every twenty people (in the 500-person profile, deaths from the mixed
    // party's cold and diets split it apart on its third day with the bare four).
    let size = c.members.len();
    // Dry (#433): no water within a long walk of home, and the well they dug found none.
    let dry = c
        .no_site
        .iter()
        .any(|(k, until)| *k == crate::structures::StructureKind::Well && *until > tick)
        && crate::wells::water_within(w, c.home, 30).is_none();
    if !dry && c.hardships.len() < HARD_ENOUGH + size / 20 {
        return;
    }
    let home = c.home;
    let founded = c.founded;
    let hard: Vec<(u64, String, crate::ids::EventId)> = c.hardships.clone();
    let people = members(w, cid);
    let grown: Vec<usize> = people
        .iter()
        .copied()
        .filter(|&i| w.agents[i].of_age(tick))
        .collect();
    if grown.len() < 2 {
        return;
    }
    // Who proposes it: one the hardships touched most, kin of the dead first, then the most
    // unhappy (first id on a tie).
    let dead: Vec<String> = hard
        .iter()
        .filter_map(|(_, s, _)| {
            [" died ", " was "]
                .iter()
                .find(|sep| s.contains(**sep))
                .and_then(|sep| s.split(*sep).next())
        })
        .map(|s| s.to_string())
        .collect();
    // The dead the hardships name, found once (not once for every member asked).
    let lost: Vec<(AgentId, Vec<AgentId>)> = w
        .agents
        .iter()
        .filter(|d| !d.alive && dead.contains(&d.name))
        .map(|d| (d.id, d.children.clone()))
        .collect();
    let touched = |i: usize| {
        let a = &w.agents[i];
        let kin_lost = lost.iter().filter(|(d, kids)| {
            a.partner == Some(*d) || a.children.contains(d) || kids.contains(&a.id)
        });
        kin_lost.count() as f32 * 2.0 - a.mood
    };
    let Some(&lead) = grown.iter().max_by(|&&a, &&b| {
        touched(a)
            .total_cmp(&touched(b))
            .then(w.agents[b].id.cmp(&w.agents[a].id))
    }) else {
        return;
    };
    let lead_id = w.agents[lead].id;
    let what: Vec<String> = {
        let mut v: Vec<String> = Vec::new();
        for (_, s, _) in hard.iter().rev() {
            if !v.contains(s) {
                v.push(s.clone());
            }
        }
        v.truncate(3);
        v
    };
    let because = if dry {
        "no water within a long walk of home, and the well they dug found none".to_string()
    } else {
        format!(
            "{}: {} hard things in two seasons",
            what.join("; "),
            number(hard.len())
        )
    };
    let source = hard.last().map(|h| h.2);
    // Where to: a flat place 25 to 45 tiles off in the valley, away from the camps of any
    // strangers near; none found, out of the valley.
    let mut r = stream(
        w.meta.seed,
        &[purpose::AGENT_SOCIAL, 0x60C, cid.0 as u64, tick],
    );
    let bands: Vec<TilePos> = w.bands.iter().map(|b| b.camp).collect();
    // Not back to a camp left in the last hundred days (#243: a community moved back to the
    // camp it left twenty days before).
    let old_camps: Vec<TilePos> = c
        .old_camps
        .iter()
        .filter(|(_, t)| tick.saturating_sub(*t) < 100 * TICKS_PER_DAY)
        .map(|(p, _)| *p)
        .collect();
    let mut to = None;
    // Water first (#433): of the places found, the first with water a short walk off; only if
    // none has, the first found.
    let mut dry = None;
    for _ in 0..16 {
        let ang = r.gen_range(0.0..std::f32::consts::TAU);
        let d = r.gen_range(25.0..45.0f32);
        let near = TilePos::new(
            (home.x + (ang.d_cos() * d) as i32).clamp(8, w.grid.width - 9),
            (home.y + (ang.d_sin() * d) as i32).clamp(8, w.grid.height - 9),
            home.z,
        );
        if let Some(site) = crate::systems::settlement::find_flat_area_within(w, near, 7, 6, 2, 8) {
            let far_from_bands = bands.iter().all(|b| b.dist(site) > 30);
            let not_back = old_camps.iter().all(|o| o.dist(site) > 20);
            let reach = w.nav.as_ref().is_some_and(|n| n.reach(home, site));
            if site.dist(home) >= 20 && far_from_bands && not_back && reach {
                if crate::wells::water_within(w, site, crate::wells::SHORT_WALK).is_some() {
                    to = Some(site);
                    break;
                }
                dry = dry.or(Some(site));
            }
        }
    }
    let to = to.or(dry);
    // Out of the valley only to somewhere they have heard of: a settlement of the world they
    // know (a journey's, a trader's). Knowing none, nobody proposes leaving for nowhere. (In
    // the 500-person profile, a valley with no realm known, the whole mixed party walked out.)
    if to.is_none() && !w.overworld.sites.iter().any(|s| s.known_by.contains(&cid)) {
        return;
    }
    // The proposal answers the hardships it names (#208): the next one needs new ones. Before
    // this the same three raids drove three proposals twenty days apart, each tearing a party
    // off the community.
    if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
        c.hardships.clear();
        c.last_move_proposal = tick;
    }
    let id = w.next_project_id;
    w.next_project_id += 1;
    let pos = w.agents[lead].pos;
    w.projects.push(crate::project::Project {
        id,
        by: lead_id,
        community: cid,
        kind: ProjectKind::Move { to },
        because: because.clone(),
        source,
        proposed: tick,
        work: None,
        done: true,
        site: to,
    });
    w.events.push(
        tick,
        Some(pos),
        Severity::Notable,
        EventKind::Proposed {
            agent: lead_id,
            what: if to.is_some() {
                "to move camp to a better place in the valley".into()
            } else {
                "to leave the valley".into()
            },
            because: because.clone(),
        },
    );
    if std::env::var("HF_TRACE_PROJECTS").is_ok() {
        eprintln!(
            "MOVE community {} day {} to {:?}: {because}",
            cid.0,
            tick / TICKS_PER_DAY,
            to
        );
    }
    // Each grown person answers; children go with a parent who goes.
    let leader_name = w.agents[lead].name.clone();
    let gone: std::collections::BTreeSet<AgentId> =
        w.agents.iter().filter(|d| !d.alive).map(|d| d.id).collect();
    let mut going: Vec<AgentId> = vec![lead_id];
    let mut staying: Vec<AgentId> = Vec::new();
    let mut stay_reasons: Vec<String> = Vec::new();
    for &i in &grown {
        if i == lead {
            continue;
        }
        let a = &w.agents[i];
        let regard = a.opinion_of(lead_id);
        let hardship = touched(i).max(0.0) * 0.2 + (-a.mood).max(0.0) * 0.5;
        // What holds them: kin buried here, and the years they have lived here.
        let graves = (a.partner.is_some_and(|p| gone.contains(&p)) as usize
            + a.children.iter().filter(|c| gone.contains(c)).count()) as f32;
        // The years they have lived here, not their age (#209: every grown arrival was held
        // by "this is home" from day one). An old save has no `came`: the community's founding.
        let since = if a.came > 0 { a.came } else { founded };
        let years = tick.saturating_sub(since) as f32 / crate::time::TICKS_PER_YEAR as f32;
        let long_here = years > 20.0;
        let hold = 0.25 * graves + if long_here { 0.2 } else { 0.0 };
        let go = regard + hardship - hold > 0.0;
        let (choice, reason) = if go {
            (
                format!("went with {leader_name}"),
                if regard >= hardship {
                    format!("thinks well of {leader_name}")
                } else {
                    "has had enough of this place".to_string()
                },
            )
        } else {
            (
                "stayed".to_string(),
                if graves > 0.0 {
                    "will not leave the graves of their own".to_string()
                } else if regard < 0.0 {
                    format!("does not trust {leader_name}")
                } else if long_here {
                    "this is home".to_string()
                } else {
                    "saw no reason to go".to_string()
                },
            )
        };
        let id = a.id;
        let p = a.pos;
        if !go {
            stay_reasons.push(format!("{} {}", a.name, reason));
        }
        w.events.push(
            tick,
            Some(p),
            Severity::Minor,
            EventKind::Chose {
                agent: id,
                what: choice,
                because: reason,
            },
        );
        if go {
            going.push(id);
        } else {
            staying.push(id);
        }
    }
    // One line the feed keeps (#214): twenty Minor answers on one tick were throttled to none.
    {
        let went = (going.len() - 1) as u32;
        let stayed = staying.len() as u32;
        let reasons: Vec<String> = stay_reasons.iter().take(3).cloned().collect();
        w.events.push(
            tick,
            Some(w.agents[lead].pos),
            Severity::Notable,
            EventKind::Answered {
                leader: lead_id,
                went,
                stayed,
                reasons,
            },
        );
    }
    // Children under age go with a parent who goes.
    let goers_children: std::collections::BTreeSet<AgentId> = w
        .agents
        .iter()
        .filter(|p| going.contains(&p.id))
        .flat_map(|p| p.children.iter().copied())
        .collect();
    for &i in &people {
        let a = &w.agents[i];
        if !a.of_age(tick) && goers_children.contains(&a.id) {
            going.push(a.id);
        }
    }
    if going.len() < 2 {
        return;
    }
    let stay_grown = staying.len();
    match to {
        Some(dest) if stay_grown <= 1 => {
            // The whole community moves; anyone who stayed stays as one of it still.
            // The old site is left: its houses and hollows stand, but are no longer theirs to
            // plan from (in C60's 600-day soak a moved community kept placing its workshops by
            // its old first house, on another community's ground, and built eight benches).
            // Their orders at the old site go with the site (C69: seed 3's community moved on
            // day 73 and never built a house again, its old camp's seventeen wall orders
            // holding the "walls posted" gate for five hundred days); what was delivered to
            // them is given back on the ground there.
            // The old site is recorded as it stands (C72): its houses weather into ruins and
            // those who come on them later wonder; and what the stores hold goes with them,
            // a stack a head.
            crate::left_sites::record_move(w, cid, home, dest);
            crate::left_sites::carry(w, &going, home);
            let (mut kept, mut dropped) = (Vec::new(), Vec::new());
            for d in std::mem::take(&mut w.designations) {
                if !d.done && d.community == Some(cid) && d.pos.dist(home) <= 45 {
                    dropped.push(d);
                } else {
                    kept.push(d);
                }
            }
            w.designations = kept;
            w.give_back(dropped);
            // A wall that already stood can still have an open project until the next
            // daily planner pass. End that record with the old site so the moved community
            // cannot carry a stale defence order into its new camp (FIXLOG 2026-10-30, M5/#403).
            for p in w
                .projects
                .iter_mut()
                .filter(|p| p.community == cid && !p.done && matches!(p.kind, ProjectKind::Wall))
            {
                p.done = true;
            }
            if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
                c.old_camps.push((home, tick));
                c.old_camps
                    .retain(|(_, t)| tick.saturating_sub(*t) < 100 * TICKS_PER_DAY);
                c.home = dest;
                c.house_sites.clear();
                c.house_stage.clear();
                c.house_plans.clear();
                c.hollows.clear();
                c.field_site = None;
                c.extra_fields.clear();
                c.field_tiles.clear();
                c.quarry_site = None;
                // #405: the mine is left with the quarry it was sunk from.
                c.mine = None;
                // The wall round the old houses is left with them (#403, M5 review 2026-10-26:
                // seed 3's Holtwick kept its first camp's ring through three moves, the planner
                // reposting its orders there, and could never propose a wall at a new camp).
                c.palisade = None;
            }
            walk(w, &going, dest);
            w.events.push(
                tick,
                Some(home),
                Severity::Major,
                EventKind::MovedCamp {
                    community: cid,
                    leader: lead_id,
                    from: home,
                    to: dest,
                    cause: because,
                },
            );
        }
        Some(dest) => {
            let new_cid = w.found_community(dest);
            for m in &going {
                w.join_community(*m, new_cid);
            }
            walk(w, &going, dest);
            w.events.push(
                tick,
                Some(home),
                Severity::Major,
                EventKind::Split {
                    from: cid,
                    to: new_cid,
                    leader: lead_id,
                    members: going.len() as u32,
                    cause: because,
                },
            );
        }
        None => {
            // Out of the valley: they leave the community and walk to the edge (society.rs
            // leavers); the world gets a settlement of theirs, founded only once the first of
            // them actually arrives (#1322: the realm should hold who arrived, not who set
            // out). The site is named and spotted now so the departure line can say where.
            let species = w.agents[lead].people;
            let reserved = crate::realm::live::reserve_settle_from_valley(w, species);
            let name = reserved
                .as_ref()
                .map(|s| s.1.clone())
                .unwrap_or_else(|| "the world beyond".into());
            let pending = reserved.map(|s| s.0);
            // Only a community that went as one "left the valley" (#210): a party of seven
            // leaving forty-one does not make the forty-one's ending a departure.
            if stay_grown <= 1 {
                if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
                    c.left_for = Some(name.clone());
                }
            }
            for m in &going {
                if let Some(i) = w.agents.iter().position(|a| a.id == *m) {
                    w.agents[i].community = None;
                    w.agents[i].leaving = true;
                    w.agents[i].bound_for = pending;
                }
                if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
                    c.members.retain(|x| x != m);
                }
            }
            w.events.push(
                tick,
                Some(home),
                Severity::Major,
                EventKind::LeftTheValley {
                    community: cid,
                    leader: lead_id,
                    people: going.len() as u32,
                    to: name,
                    cause: because,
                },
            );
        }
    }
}

fn walk(w: &mut World, who: &[AgentId], to: TilePos) {
    for m in who {
        if let Some(i) = w.agents.iter().position(|a| a.id == *m) {
            w.reservations.release_all(*m);
            w.agents[i].activity = crate::mind::Activity::Wandering { to };
            w.agents[i].path.clear();
            let _ = w.path_agent_to(i, to);
        }
    }
}

fn number(n: usize) -> String {
    match n {
        4 => "four".into(),
        5 => "five".into(),
        6 => "six".into(),
        7 => "seven".into(),
        8 => "eight".into(),
        9 => "nine".into(),
        10 => "ten".into(),
        n => n.to_string(),
    }
}

/// Daily: a community nobody of which is left in the valley has ended; it is recorded once,
/// with how (C60).
pub fn endings(w: &mut World) {
    let tick = w.meta.tick;
    let ended: Vec<(CommunityId, String, TilePos)> = w
        .communities
        .iter()
        .filter(|c| !c.dissolved && c.founded + TICKS_PER_DAY < tick)
        .filter(|c| {
            !w.agents.iter().any(|a| {
                a.alive && a.community == Some(c.id) && a.species == crate::agents::Species::Person
            })
            // Everyone on the road (a caravan, a journey) is away, not gone (#197 case 7's
            // neighbour, FIXLOG 2026-11-12): it was recorded as died out while they travelled.
            && w.overworld.on_the_road(c.id).next().is_none()
        })
        .map(|c| (c.id, c.name.clone(), c.home))
        .collect();
    for (cid, name, home) in ended {
        let how = match w
            .communities
            .iter()
            .find(|c| c.id == cid)
            .and_then(|c| c.left_for.clone())
        {
            Some(to) => format!("left the valley for {to}"),
            // #353: and what killed them.
            None => {
                let d = w
                    .communities
                    .iter()
                    .find(|c| c.id == cid)
                    .map(|c| {
                        crate::fate::account(
                            &c.deaths
                                .iter()
                                .filter(|x| x.0 != "#last")
                                .cloned()
                                .collect::<Vec<_>>(),
                        )
                    })
                    .unwrap_or_default();
                if d.is_empty() {
                    "died out".to_string()
                } else {
                    format!("died out ({d})")
                }
            }
        };
        if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
            c.dissolved = true;
            c.ended = Some((tick, how.clone()));
        }
        w.events.push(
            tick,
            Some(home),
            Severity::Major,
            EventKind::CommunityEnded {
                community: cid,
                name,
                how,
            },
        );
    }
}

/// The valley's ending, once every community of it has ended (C60): a line for each, how it
/// ended. Empty while anyone of a community is left.
pub fn ending(w: &World) -> String {
    if w.communities.is_empty() || w.communities.iter().any(|c| c.ended.is_none()) {
        return String::new();
    }
    w.communities
        .iter()
        .filter_map(|c| {
            c.ended
                .as_ref()
                .map(|(_, how)| format!("{}: they {how}.", c.name))
        })
        .collect::<Vec<_>>()
        .join("\n")
}
