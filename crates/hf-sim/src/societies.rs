//! Secret societies and betrayal (#322, epic #376; docs/design-feelings.md section 6): three or
//! more of a community angry at the same person, who think well of one another, meet apart,
//! by night, away from the houses: a society against that one. A member who has come to like
//! the one they meet against may give them away, and the rest turn on the betrayer.
//!
//! The deep (#322, 2026-10-26 fix-322-taken): a society whose community holds a walled way down
//! to the deep, with a people of the deep near enough under the valley, may come to terms with
//! them against the one they hate: the deep's people take that one's child. One of them, the
//! keeper of the way if the keeper is one of them, goes to the wall at midnight and cuts it
//! out; two of the deep's people come up for the child and carry it down. In the morning the
//! way is found open and the valley sets about walling it again.

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, CommunityId};
use crate::mind::feelings::FeelingKind;
use crate::rng::{purpose, stream};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::World;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Society {
    pub community: CommunityId,
    pub against: AgentId,
    pub members: Vec<AgentId>,
    pub formed: u64,
    /// #322: what it means to do, hidden from the rest.
    #[serde(default)]
    pub aim: Option<Aim>,
    /// #322: it has done it once; it does not bargain again.
    #[serde(default)]
    pub struck: bool,
}

/// A society's bargain with the deep (#322): the way to open, who comes up, whose child they
/// take, and which of the members goes to the wall.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Aim {
    pub cavern: usize,
    pub site: String,
    pub people: u32,
    pub culture: u32,
    pub child: AgentId,
    pub opener: Option<AgentId>,
}

/// One night a way down was opened to let the deep's people up (#322), kept until it is judged
/// or forgotten.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Taking {
    pub community: CommunityId,
    pub cavern: usize,
    pub site: String,
    pub opener: AgentId,
    pub against: AgentId,
    pub members: Vec<AgentId>,
    pub child: AgentId,
    pub takers: Vec<AgentId>,
    /// Someone of the valley, not of the society, awake by the wall when it was cut.
    pub seen_by: Option<AgentId>,
    pub opened: u64,
    pub taken: bool,
    pub found: bool,
    /// Others of the valley awake and further off (within twenty tiles) who saw someone out by
    /// the way that night, not near enough to be sure who cut it: they suspect the opener.
    #[serde(default)]
    pub glimpsed_by: Vec<AgentId>,
    /// Everyone accused of it so far, so a rumour does not accuse the same one twice.
    #[serde(default)]
    pub accused: Vec<AgentId>,
}

/// Thinking this well of the one they meet against is liking them: such a one may give the
/// society away, and is not taken into one (#404).
pub const LIKES: f32 = 0.4;

fn angry_at(a: &crate::agents::Agent, who: AgentId) -> bool {
    a.feelings
        .list
        .iter()
        .any(|f| f.kind == FeelingKind::Anger && f.about == Some(who) && f.strength >= 0.3)
}

pub fn step(w: &mut World) {
    night(w);
    crate::taken::step(w);
    let tick = w.meta.tick;
    let hour = (tick % TICKS_PER_DAY) / TICKS_PER_HOUR;
    if tick % TICKS_PER_HOUR != 77 {
        return;
    }
    if hour == 6 {
        dawn(w);
    }
    if hour == 17 {
        rumoured(w);
    }
    if hour == 1 {
        seen_meeting(w);
    }
    // At midnight the members go out to meet.
    if hour == 0 {
        for s in w.societies.clone() {
            let Some(spot) = meeting_place(w, &s) else {
                continue;
            };
            for m in &s.members {
                if let Some(i) = w.agents.iter().position(|a| a.id == *m && a.alive) {
                    if w.path_agent_to(i, spot) {
                        w.agents[i].activity = crate::mind::Activity::Wandering { to: spot };
                        w.agents[i].activity_ticks = 0;
                    }
                }
            }
        }
        send_openers(w);
    }
    if hour != 20 {
        return;
    }
    let day = tick / TICKS_PER_DAY;
    // Forming.
    let cids: Vec<CommunityId> = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .map(|c| c.id)
        .collect();
    for cid in cids {
        let people: Vec<usize> = (0..w.agents.len())
            .filter(|&k| {
                w.agents[k].alive
                    && w.agents[k].community == Some(cid)
                    && !w.agents[k].is_child(tick)
            })
            .collect();
        for &t in &people {
            let target = w.agents[t].id;
            if w.societies.iter().any(|s| s.against == target) {
                continue;
            }
            let angry: Vec<usize> = people
                .iter()
                .copied()
                // #404 (2026-10-27 fix-404-feuds): angry at them but not liking them as well;
                // one who did both was taken in and gave the society away the evening it formed.
                .filter(|&k| {
                    k != t
                        && angry_at(&w.agents[k], target)
                        && w.agents[k].opinion_of(target) <= LIKES
                })
                .collect();
            if angry.len() < 3 {
                continue;
            }
            // Those of them who think well of one another.
            let core: Vec<usize> = angry
                .iter()
                .copied()
                .filter(|&k| {
                    angry
                        .iter()
                        .filter(|&&j| j != k && w.agents[k].opinion_of(w.agents[j].id) > 0.3)
                        .count()
                        >= 2
                })
                .collect();
            if core.len() < 3 {
                continue;
            }
            let members: Vec<AgentId> = core.iter().map(|&k| w.agents[k].id).collect();
            w.societies.push(Society {
                community: cid,
                against: target,
                members: members.clone(),
                formed: tick,
                aim: None,
                struck: false,
            });
            let at = w.agents[core[0]].pos;
            w.events.push(
                tick,
                Some(at),
                Severity::Notable,
                EventKind::SocietyFormed {
                    against: target,
                    members: members.len() as u32,
                },
            );
        }
    }
    bargain(w, day);
    // Betrayal, and lapsing.
    let mut keep = Vec::new();
    for mut s in std::mem::take(&mut w.societies) {
        if !w.agents.iter().any(|a| a.id == s.against && a.alive) {
            continue;
        }
        s.members
            .retain(|m| w.agents.iter().any(|a| a.id == *m && a.alive));
        if s.members.len() < 2 {
            continue;
        }
        // #404: nobody gives it away the evening it forms; a member has to come round first.
        if tick.saturating_sub(s.formed) < TICKS_PER_DAY {
            keep.push(s);
            continue;
        }
        let traitor = s.members.iter().copied().find(|m| {
            let a = w.agents.iter().find(|a| a.id == *m).unwrap();
            let mut r = stream(
                w.meta.seed,
                &[purpose::AGENT_SOCIAL, 0x3220, m.0 as u64, day],
            );
            a.opinion_of(s.against) > LIKES && r.gen_bool(0.3)
        });
        if let Some(b) = traitor {
            let rest: Vec<AgentId> = s.members.iter().copied().filter(|m| *m != b).collect();
            for m in &rest {
                if let Some(i) = w.agents.iter().position(|a| a.id == *m) {
                    w.agents[i].relate(b, -0.6, 0.1, tick);
                }
            }
            if let Some(t) = w.agents.iter().position(|a| a.id == s.against) {
                for m in &rest {
                    w.agents[t].relate(*m, -0.4, 0.1, tick);
                }
            }
            let at = w.agents.iter().find(|a| a.id == b).map(|a| a.pos);
            w.events.push(
                tick,
                at,
                Severity::Major,
                EventKind::Betrayed {
                    by: b,
                    to: s.against,
                    members: rest.len() as u32,
                },
            );
            continue;
        }
        keep.push(s);
    }
    w.societies = keep;
}

/// Where a society meets: away from the houses, fourteen tiles east of home.
pub fn meeting_place(w: &World, s: &Society) -> Option<crate::ids::TilePos> {
    let home = w
        .communities
        .iter()
        .find(|c| c.id == s.community)
        .map(|c| c.home)?;
    Some(w.grid.surface((home.x + 14).min(w.grid.width - 2), home.y))
}

/// Within this many tiles of a society's meeting, someone awake sees who is there.
pub const SEES_MEETING: i32 = 10;

/// #322 (2026-10-28 round-17): at one in the morning, anyone of the community awake within ten
/// tiles of where a society meets, while two or more of it are there, sees them gathered apart:
/// they come to suspect each one there (0.35) and think the less of them, and if they think well
/// of the one the society meets against, they tell them, who suspects them too (0.3). Talk
/// carries it on (social.rs `tell_suspicion`). Seen once, it is not on record again until
/// someone new is seen there. FIXLOG 2026-10-28 "nobody ever saw a society meet".
fn seen_meeting(w: &mut World) {
    let tick = w.meta.tick;
    for s in w.societies.clone() {
        let Some(spot) = meeting_place(w, &s) else {
            continue;
        };
        let there: Vec<AgentId> = s
            .members
            .iter()
            .copied()
            .filter(|m| {
                w.agents
                    .iter()
                    .any(|a| a.id == *m && a.alive && a.pos.z == spot.z && a.pos.dist(spot) <= 3)
            })
            .collect();
        if there.len() < 2 {
            continue;
        }
        let seers: Vec<usize> = (0..w.agents.len())
            .filter(|&k| {
                let a = &w.agents[k];
                a.alive
                    && a.community == Some(s.community)
                    && !s.members.contains(&a.id)
                    && !a.is_child(tick)
                    && !a.activity.is_asleep()
                    && a.pos.z == spot.z
                    && a.pos.dist(spot) <= SEES_MEETING
                    && there
                        .iter()
                        .any(|m| !a.suspects.iter().any(|x| x.0 == *m && x.1 >= 0.35))
            })
            .collect();
        for k in seers {
            let by = w.agents[k].id;
            for m in &there {
                suspect(w, by, *m, 0.35);
                w.agents[k].relate(*m, -0.1, 0.02, tick);
            }
            if by != s.against && w.agents[k].opinion_of(s.against) > 0.3 {
                for m in &there {
                    suspect(w, s.against, *m, 0.3);
                }
            }
            let at = w.agents[k].pos;
            w.events.push(
                tick,
                Some(at),
                Severity::Notable,
                EventKind::MeetingSeen {
                    by,
                    against: s.against,
                    members: there.len() as u32,
                },
            );
        }
    }
}

/// A child of `of`, alive, a child still and in the valley.
fn child_of(w: &World, of: AgentId) -> Option<AgentId> {
    let tick = w.meta.tick;
    let parent = w.agents.iter().find(|a| a.id == of && a.alive)?;
    parent.children.iter().copied().find(|c| {
        w.agents
            .iter()
            .any(|a| a.id == *c && a.alive && a.is_child(tick) && a.community == parent.community)
    })
}

/// #322: of an evening, a society with no aim yet whose community holds a walled way down,
/// with a people of the deep near enough under the valley and the one they hate a parent, may
/// come to terms with that people: one evening in eight.
fn bargain(w: &mut World, day: u64) {
    let tick = w.meta.tick;
    let Some((site, people, culture)) = crate::underground::deep_neighbour(w) else {
        return;
    };
    for n in 0..w.societies.len() {
        let s = &w.societies[n];
        if s.aim.is_some() || s.struck {
            continue;
        }
        let Some(cavern) = w
            .grid
            .caverns
            .iter()
            .position(|c| c.deep && c.sealed && c.gate_of == Some(s.community))
        else {
            continue;
        };
        let Some(child) = child_of(w, s.against) else {
            continue;
        };
        let mut r = stream(
            w.meta.seed,
            &[purpose::AGENT_SOCIAL, 0x3221, s.against.0 as u64, day],
        );
        if !r.gen_bool(0.125) {
            continue;
        }
        let (against, members) = (s.against, s.members.len() as u32);
        w.societies[n].aim = Some(Aim {
            cavern,
            site: site.clone(),
            people,
            culture,
            child,
            opener: None,
        });
        let at = w.grid.caverns[cavern].breach;
        w.events.push(
            tick,
            at,
            Severity::Notable,
            EventKind::DeepPact {
                against,
                site: site.clone(),
                members,
                child: Some(child),
            },
        );
    }
}

/// #322: at midnight one of a society with a bargain goes to the wall: the keeper of the way if
/// the keeper is one of them, else the angriest.
fn send_openers(w: &mut World) {
    for n in 0..w.societies.len() {
        let Some(aim) = w.societies[n].aim.clone() else {
            continue;
        };
        let Some(c) = w.grid.caverns.get(aim.cavern).cloned() else {
            continue;
        };
        let (Some(b), true) = (c.breach, c.sealed) else {
            w.societies[n].aim = None;
            continue;
        };
        let s = &w.societies[n];
        let able = |id: AgentId| {
            w.agents
                .iter()
                .any(|a| a.id == id && a.alive && a.community == Some(s.community))
        };
        let opener = c
            .keeper
            .filter(|k| s.members.contains(k) && able(*k))
            .or_else(|| {
                s.members
                    .iter()
                    .copied()
                    .filter(|m| able(*m))
                    .max_by(|x, y| {
                        let anger = |id: AgentId| {
                            w.agents
                                .iter()
                                .find(|a| a.id == id)
                                .map(|a| {
                                    a.feelings
                                        .list
                                        .iter()
                                        .filter(|f| {
                                            f.kind == FeelingKind::Anger
                                                && f.about == Some(s.against)
                                        })
                                        .map(|f| f.strength)
                                        .fold(0.0f32, f32::max)
                                })
                                .unwrap_or(0.0)
                        };
                        anger(*x).total_cmp(&anger(*y)).then(y.cmp(x))
                    })
            });
        let Some(opener) = opener else { continue };
        if let Some(a) = w.societies[n].aim.as_mut() {
            a.opener = Some(opener);
        }
        let Some(i) = w.agents.iter().position(|a| a.id == opener) else {
            continue;
        };
        if w.agents[i].pos.dist(b) <= 1 {
            // Already there: they stay, and do not go to the meeting.
            w.agents[i].path.clear();
            w.agents[i].activity = crate::mind::Activity::Loitering { ticks_left: 60 };
            continue;
        }
        // The valley's side of the wall.
        let stand = [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .iter()
            .map(|(dx, dy)| b.offset(*dx, *dy, 0))
            .filter(|p| !c.contains(*p) && w.grid.walkable(*p))
            .min_by_key(|p| (p.dist(w.agents[i].pos), p.x, p.y));
        if let Some(stand) = stand {
            if w.path_agent_to(i, stand) {
                w.agents[i].activity = crate::mind::Activity::Wandering { to: stand };
                w.agents[i].activity_ticks = 0;
            }
        }
    }
}

/// #322: through the night: an opener at the wall cuts it out and lets the deep's people up;
/// they go for the child and carry it down.
fn night(w: &mut World) {
    let tick = w.meta.tick;
    let hour = (tick % TICKS_PER_DAY) / TICKS_PER_HOUR;
    if hour < 6 {
        for n in 0..w.societies.len() {
            let Some(aim) = w.societies[n].aim.clone() else {
                continue;
            };
            let Some(opener) = aim.opener else { continue };
            let Some(i) = w.agents.iter().position(|a| a.id == opener && a.alive) else {
                continue;
            };
            let at_wall = w.grid.caverns.get(aim.cavern).is_some_and(|c| {
                c.sealed && c.breach.is_some_and(|b| b.dist(w.agents[i].pos) <= 1)
            });
            if !at_wall || crate::underground::open_gate(w, i).is_none() {
                continue;
            }
            let s = w.societies[n].clone();
            w.societies[n].aim = None;
            w.societies[n].struck = true;
            let_up(w, &s, &aim, opener);
        }
    }
    if w.takings.iter().all(|t| t.found) {
        return;
    }
    for t in 0..w.takings.len() {
        if w.takings[t].found {
            continue;
        }
        let tk = w.takings[t].clone();
        let Some(c) = w.grid.caverns.get(tk.cavern).cloned() else {
            continue;
        };
        if tk.taken {
            // Back down with it.
            for id in &tk.takers {
                if let Some(i) = w.agents.iter().position(|a| a.id == *id && a.alive) {
                    if c.contains(w.agents[i].pos) {
                        crate::systems::society::leave_valley(w, i);
                    }
                }
            }
            continue;
        }
        let Some(ci) = w.agents.iter().position(|a| a.id == tk.child && a.alive) else {
            continue;
        };
        let child_at = w.agents[ci].pos;
        let by = tk.takers.iter().copied().find(|id| {
            w.agents
                .iter()
                .any(|a| a.id == *id && a.alive && a.pos.dist(child_at) <= 1)
        });
        if by.is_some() {
            take(w, t, ci);
            continue;
        }
        // Every so often, after the child where it lies now.
        if tick.is_multiple_of(200) {
            for id in &tk.takers {
                if let Some(i) = w.agents.iter().position(|a| a.id == *id && a.alive) {
                    if w.path_agent_to(i, child_at) {
                        w.agents[i].activity = crate::mind::Activity::Wandering { to: child_at };
                    }
                }
            }
        }
    }
}

/// The way is open: two of the deep's people come up for the child.
fn let_up(w: &mut World, s: &Society, aim: &Aim, opener: AgentId) {
    let tick = w.meta.tick;
    let c = w.grid.caverns[aim.cavern].clone();
    let b = c.breach.unwrap_or(c.centre);
    // Anyone of the valley, not of them, awake near enough to see who cut the wall.
    let seen_by = w
        .agents
        .iter()
        .filter(|a| {
            a.alive
                && a.id != opener
                && a.community == Some(s.community)
                && !s.members.contains(&a.id)
                && !a.activity.is_asleep()
                && !a.is_child(tick)
                && a.pos.z == b.z
                && a.pos.dist(b) <= 8
        })
        .min_by_key(|a| (a.pos.dist(b), a.id))
        .map(|a| a.id);
    let glimpsed_by: Vec<AgentId> = w
        .agents
        .iter()
        .filter(|a| {
            a.alive
                && a.id != opener
                && Some(a.id) != seen_by
                && a.community == Some(s.community)
                && !s.members.contains(&a.id)
                && !a.activity.is_asleep()
                && !a.is_child(tick)
                && a.pos.z == b.z
                && a.pos.dist(b) <= 20
        })
        .map(|a| a.id)
        .collect();
    w.events.push(
        tick,
        Some(b),
        Severity::Major,
        EventKind::LetIn {
            by: opener,
            from: aim.site.clone(),
            against: s.against,
            child: Some(aim.child),
        },
    );
    let sp = w.realm.peoples.get(aim.people as usize).map(|p| p.species);
    let floors: Vec<_> = crate::underground::floors(w, &c).take(2).collect();
    let child_at = w.agents.iter().find(|a| a.id == aim.child).map(|a| a.pos);
    let mut takers = Vec::new();
    for p in floors {
        let id = w.spawn_agent(crate::agents::Species::Person, p);
        if let Some(sp) = sp {
            crate::species::give(w, id, sp, Some(aim.people as usize));
        }
        crate::species::raise_in(w, id, aim.culture);
        if let Some(i) = w.agents.iter().position(|a| a.id == id) {
            // Not asking to join, and walked by nobody's choice but this (decide.rs passes
            // over the leaving).
            w.agents[i].leaving = true;
            if let Some(to) = child_at {
                if w.path_agent_to(i, to) {
                    w.agents[i].activity = crate::mind::Activity::Wandering { to };
                }
            }
        }
        takers.push(id);
    }
    // A child taken to them before, grown among them, may be with them (taken.rs).
    crate::taken::among_them(w, &aim.site, b);
    w.takings.push(Taking {
        community: s.community,
        cavern: aim.cavern,
        site: aim.site.clone(),
        opener,
        against: s.against,
        members: s.members.clone(),
        child: aim.child,
        takers,
        seen_by,
        opened: tick,
        taken: false,
        found: false,
        glimpsed_by,
        accused: Vec::new(),
    });
}

/// The child is carried off, down into the deep: out of the valley, to the site's people.
fn take(w: &mut World, t: usize, ci: usize) {
    let tick = w.meta.tick;
    let id = w.agents[ci].id;
    let at = w.agents[ci].pos;
    w.reservations.release_all(id);
    for it in w.items.iter_mut() {
        if it.place == crate::items::ItemPlace::Carried(id) {
            it.place = crate::items::ItemPlace::Ground(at);
        }
    }
    let mut a = w.agents.remove(ci);
    a.activity = crate::mind::Activity::Idle;
    a.activity_ticks = 0;
    a.path.clear();
    w.overworld.away.push(a);
    w.takings[t].taken = true;
    let site = w.takings[t].site.clone();
    w.events.push(
        tick,
        Some(at),
        Severity::Major,
        EventKind::TakenBelow {
            agent: id,
            from: site,
        },
    );
    // Back the way they came.
    let centre = w.grid.caverns[w.takings[t].cavern].centre;
    for id in w.takings[t].takers.clone() {
        if let Some(i) = w.agents.iter().position(|a| a.id == id && a.alive) {
            if w.path_agent_to(i, centre) {
                w.agents[i].activity = crate::mind::Activity::Wandering { to: centre };
            }
        }
    }
}

/// #322: in the morning the way is found open, the child missed; the deep's people still up go
/// back down; the valley sets about walling the way again, and the child's parents grieve.
fn dawn(w: &mut World) {
    let tick = w.meta.tick;
    for t in 0..w.takings.len() {
        if w.takings[t].found {
            continue;
        }
        w.takings[t].found = true;
        let tk = w.takings[t].clone();
        for id in &tk.takers {
            if let Some(i) = w.agents.iter().position(|a| a.id == *id && a.alive) {
                crate::systems::society::leave_valley(w, i);
            }
        }
        let c = w.grid.caverns[tk.cavern].clone();
        let b = c.breach.unwrap_or(c.centre);
        w.events.push(
            tick,
            Some(b),
            Severity::Major,
            EventKind::GateFoundOpen {
                cavern: c.name.clone(),
                child: tk.taken.then_some(tk.child),
            },
        );
        if !w.grid.get(b).shape.is_solid() {
            let mut d = crate::systems::settlement::make(
                w,
                crate::work::DesignationKind::BuildWall(crate::world::Material::STONE_BLOCK),
                b,
                8,
            );
            d.community = Some(tk.community);
            w.designations.push(d);
        }
        if let Some(of) = reckon(w, &tk, b) {
            w.takings[t].accused.push(of);
        }
        if tk.taken {
            let parents: Vec<AgentId> = w
                .agents
                .iter()
                .filter(|a| a.alive && a.children.contains(&tk.child))
                .map(|a| a.id)
                .collect();
            for p in parents {
                if let Some(i) = w.agents.iter().position(|a| a.id == p) {
                    w.agents[i]
                        .feelings
                        .feel(FeelingKind::Grief, Some(tk.child), 0.9, tick, None);
                }
            }
            // Those who will go down after the child set out (taken.rs), knowing who was
            // accused of it.
            let tk = w.takings[t].clone();
            crate::taken::gone(w, &tk);
        }
    }
    // Forgotten after forty days if never brought home to the one who did it (`exposed` takes
    // one judged); until then talk of it goes round (`rumoured`).
    w.takings
        .retain(|t| tick.saturating_sub(t.opened) < 40 * TICKS_PER_DAY);
}

/// #322 (2026-10-26 fix-322-reckoning): who is blamed for the open way. Someone who saw the
/// wall cut names the one who cut it. Else the child's parent (or whoever is nearest the wall)
/// lays it on the keeper of the way, who could open it; failing a keeper, on the one of them
/// they like least. That can be wrong, and the judging (justice.rs) does not know it.
fn reckon(w: &mut World, tk: &Taking, b: crate::ids::TilePos) -> Option<AgentId> {
    let tick = w.meta.tick;
    let cid = tk.community;
    // What each knows or thinks, to pass on (`social::tell_suspicion`): the one who saw it
    // done is sure; those who glimpsed someone out by the way think it was them.
    if let Some(s) = tk.seen_by {
        suspect(w, s, tk.opener, 1.0);
    }
    for g in &tk.glimpsed_by {
        suspect(w, *g, tk.opener, 0.5);
    }
    let here = |w: &World, id: AgentId| {
        w.agents
            .iter()
            .any(|a| a.id == id && a.alive && a.community == Some(cid))
    };
    if let Some(s) = tk.seen_by.filter(|s| here(w, *s)) {
        crate::justice::accuse(
            w,
            cid,
            s,
            tk.opener,
            tk.opener,
            crate::justice::DEEP_GATE,
            b,
        );
        return Some(tk.opener);
    }
    let adult = |a: &crate::agents::Agent| a.alive && a.community == Some(cid) && !a.is_child(tick);
    let finder = w
        .agents
        .iter()
        .filter(|a| adult(a) && a.children.contains(&tk.child))
        .map(|a| a.id)
        .min()
        .or_else(|| {
            w.agents
                .iter()
                .filter(|a| adult(a))
                .min_by_key(|a| (a.pos.dist(b), a.id))
                .map(|a| a.id)
        });
    let finder = finder?;
    let fi = w.agents.iter().position(|a| a.id == finder).unwrap();
    let keeper = w.grid.caverns[tk.cavern]
        .keeper
        .filter(|k| *k != finder && here(w, *k));
    let of = keeper.or_else(|| {
        w.agents
            .iter()
            .filter(|a| adult(a) && a.id != finder)
            .min_by(|x, y| {
                w.agents[fi]
                    .opinion_of(x.id)
                    .total_cmp(&w.agents[fi].opinion_of(y.id))
                    .then(x.id.cmp(&y.id))
            })
            .map(|a| a.id)
    });
    if let Some(of) = of {
        // The finder believes it, and says so.
        suspect(w, finder, of, 0.6);
        crate::justice::accuse(w, cid, finder, of, tk.opener, crate::justice::DEEP_GATE, b);
    }
    of
}

/// `who` comes to suspect `of` at least `sure`.
pub fn suspect(w: &mut World, who: AgentId, of: AgentId, sure: f32) {
    if who == of {
        return;
    }
    let Some(a) = w.agents.iter_mut().find(|a| a.id == who && a.alive) else {
        return;
    };
    match a.suspects.iter_mut().find(|x| x.0 == of) {
        Some(x) => x.1 = x.1.max(sure).min(1.0),
        None => {
            a.suspects.push((of, sure.min(1.0)));
            if a.suspects.len() > 4 {
                // The least sure is let go.
                let k = (0..a.suspects.len())
                    .min_by(|x, y| a.suspects[*x].1.total_cmp(&a.suspects[*y].1))
                    .unwrap();
                a.suspects.remove(k);
            }
        }
    }
}

/// Whether `who` knows the truth of a taking they were part of, with `of`: a member does not
/// take up talk against another member. Nor, of a society that meets, against one of their own
/// (round-17, since a meeting seen starts talk of its members).
pub fn in_it_together(w: &World, who: AgentId, of: AgentId) -> bool {
    w.takings.iter().any(|t| {
        let party = |x: AgentId| t.opener == x || t.members.contains(&x);
        party(who) && party(of)
    }) || w
        .societies
        .iter()
        .any(|s| s.members.contains(&who) && s.members.contains(&of))
}

/// #322 (2026-10-26 fix-322-rumour): of an evening, before the judging, someone of the valley
/// who has come to be sure (0.8) from what they saw and what was said that a member of their
/// community let the deep's people up, and who has not yet been accused of it, accuses them.
/// One accusation a taking a day, by the surest.
fn rumoured(w: &mut World) {
    let tick = w.meta.tick;
    for t in 0..w.takings.len() {
        let tk = w.takings[t].clone();
        if !tk.found {
            continue;
        }
        let here = |id: AgentId| {
            w.agents
                .iter()
                .any(|a| a.id == id && a.alive && a.community == Some(tk.community))
        };
        let best = w
            .agents
            .iter()
            .filter(|a| a.alive && a.community == Some(tk.community) && !a.is_child(tick))
            .flat_map(|a| a.suspects.iter().map(move |(of, s)| (*s, a.id, *of)))
            .filter(|(s, by, of)| {
                *s >= 0.8 && !tk.accused.contains(of) && here(*of) && !in_it_together(w, *by, *of)
            })
            .max_by(|x, y| x.0.total_cmp(&y.0).then(y.1.cmp(&x.1)).then(y.2.cmp(&x.2)));
        let Some((_, by, of)) = best else { continue };
        let Some(at) = w.agents.iter().find(|a| a.id == by).map(|a| a.pos) else {
            continue;
        };
        w.takings[t].accused.push(of);
        crate::justice::accuse(
            w,
            tk.community,
            by,
            of,
            tk.opener,
            crate::justice::DEEP_GATE,
            at,
        );
    }
}

/// #322: the one who opened the way, convicted (justice.rs): the society comes out. The rest of
/// the community thinks the worse of every one of them, and the society is done.
pub fn exposed(w: &mut World, cid: CommunityId, opener: AgentId) {
    let tick = w.meta.tick;
    let Some(t) = w
        .takings
        .iter()
        .position(|t| t.community == cid && t.opener == opener)
    else {
        return;
    };
    let tk = w.takings.remove(t);
    for i in 0..w.agents.len() {
        let a = &w.agents[i];
        if !a.alive || a.community != Some(cid) || tk.members.contains(&a.id) {
            continue;
        }
        for m in &tk.members {
            w.agents[i].relate(*m, -0.5, 0.1, tick);
        }
    }
    w.societies
        .retain(|s| !(s.community == cid && s.against == tk.against));
    let at = w.agents.iter().find(|a| a.id == opener).map(|a| a.pos);
    w.events.push(
        tick,
        at,
        Severity::Major,
        EventKind::SocietyExposed {
            against: tk.against,
            members: tk.members.len() as u32,
            from: tk.site.clone(),
        },
    );
}
