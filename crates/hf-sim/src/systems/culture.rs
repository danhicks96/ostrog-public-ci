//! Customs form from significant recorded events, spread to members over time, are observed
//! (which renews them) and fade when nobody renews them. Hooks are read by `decide`,
//! `settlement` and `society`.

use crate::culture::{Custom, CustomId, CustomKind};
use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, CommunityId, EventId};
use crate::mind::{MemKind, Subject};
use crate::time::{Calendar, TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::world_state::World;

/// Remember notable events for the culture pass (kept small).
pub fn observe(w: &mut World) {
    for e in &w.last_events {
        if e.severity >= Severity::Major {
            // The culture log keeps the simulation event, not the viewer's explanation.
            let kind = match &e.kind {
                EventKind::Broke { agent, kind, .. } => EventKind::Broke {
                    agent: *agent,
                    kind: kind.clone(),
                    explanation: None,
                },
                other => other.clone(),
            };
            w.culture_log.push((e.id, e.tick, kind));
        }
    }
    if w.culture_log.len() > 200 {
        let cut = w.culture_log.len() - 200;
        w.culture_log.drain(..cut);
    }
}

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    // Evenings at half past the hour: whoever has gathered at the hearth hears a telling.
    let hour = Calendar::from_tick(tick).hour;
    if tick % TICKS_PER_HOUR == TICKS_PER_HOUR / 2 && HEARTH_HOURS.contains(&hour) {
        hearth_evening(w);
    }
    // At dawn: form, spread, fade. At dawn of a rest day: observe it.
    if tick % TICKS_PER_DAY != TICKS_PER_HOUR * 6 {
        return;
    }
    form(w);
    spread(w);
    observe_rest_days(w);
    fade(w);
    identity(w);
}

/// The hours of a hearth evening.
pub const HEARTH_HOURS: std::ops::Range<u64> = 18..21;

/// The share of the others at a hearth who must lack a realm story for it to be told (OST-506).
pub const HEARTH_FRESH_SHARE: f32 = 0.5;

/// A community's finished hearth.
pub fn hearth_of(w: &World, cid: CommunityId) -> Option<crate::ids::TilePos> {
    let home = w.communities.iter().find(|c| c.id == cid)?.home;
    w.structures
        .iter()
        .filter(|s| s.kind == crate::structures::StructureKind::Hearth && s.progress >= 1.0)
        .filter(|s| s.pos.z == home.z && s.pos.dist(home) <= 40)
        .min_by_key(|s| s.pos.dist(home))
        .map(|s| s.pos)
}

/// How strong a community's habit of hearth evenings is (0 when it has none).
pub fn hearth_habit(w: &World, cid: CommunityId) -> f32 {
    w.customs
        .iter()
        .filter(|c| c.community == cid && !c.lapsed && c.kind == CustomKind::HearthNight)
        .map(|c| c.intensity)
        .fold(0.0, f32::max)
}

/// One telling at each community's hearth, by someone sitting there who has not yet told
/// tonight: a tale of the old valley that others there have not heard, or how they came to
/// hold what their own life taught them. The first such evening becomes a custom; each one
/// after renews it.
fn hearth_evening(w: &mut World) {
    use crate::mind::beliefs::Source;
    let tick = w.meta.tick;
    let day = tick / TICKS_PER_DAY;
    let cids: Vec<CommunityId> = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .map(|c| c.id)
        .collect();
    for cid in cids {
        let Some(h) = hearth_of(w, cid) else { continue };
        let sitting: Vec<usize> = (0..w.agents.len())
            .filter(|&k| {
                let a = &w.agents[k];
                a.alive
                    && a.community == Some(cid)
                    && a.band.is_none()
                    && !a.activity.is_asleep()
                    && a.pos.z == h.z
                    && a.pos.dist(h) <= 2
            })
            .collect();
        if sitting.len() < 2 {
            continue;
        }
        let home = w
            .communities
            .iter()
            .find(|c| c.id == cid)
            .map(|c| c.home)
            .unwrap_or(h);
        share_drink(w, h, home, &sitting);
        let told: Vec<AgentId> = w
            .communities
            .iter()
            .find(|c| c.id == cid)
            .filter(|c| c.hearth_told.0 == day)
            .map(|c| c.hearth_told.1.clone())
            .unwrap_or_default();
        // A story of the world their people tell (P8) that most of the circle has not heard
        // counts most of all (OST-506): one newcomer is not reason enough for everyone to hear
        // the same battle again. A teller offers the story the most sitters lack.
        let mut story: Option<(usize, u32)> = None;
        for &t in &sitting {
            if told.contains(&w.agents[t].id) {
                continue;
            }
            let mut best: Option<(usize, u32)> = None;
            for &s in &w.agents[t].stories {
                let lacking = sitting
                    .iter()
                    .filter(|&&l| l != t && !w.agents[l].stories.contains(&s))
                    .count();
                if lacking > 0
                    && HEARTH_FRESH_SHARE * ((sitting.len() - 1) as f32) <= lacking as f32
                    && best.is_none_or(|(n, _)| lacking > n)
                {
                    best = Some((lacking, s));
                }
            }
            if let Some((_, s)) = best {
                story = Some((t, s));
                break;
            }
        }
        if let Some((t, sid)) = story {
            if let Some(st) = w.realm.stories.iter().find(|s| s.id == sid).cloned() {
                let telling = crate::realm::stories::telling(&w.realm, &st);
                let teller = w.agents[t].id;
                let listeners: Vec<usize> = sitting.iter().copied().filter(|&l| l != t).collect();
                for &l in &listeners {
                    if !w.agents[l].stories.contains(&sid) {
                        w.agents[l].stories.push(sid);
                    }
                    w.agents[l].relate(teller, 0.03, 0.05, tick);
                    w.agents[l].met(teller, tick);
                    let lid = w.agents[l].id;
                    w.agents[t].met(lid, tick);
                }
                let ids: Vec<AgentId> = listeners.iter().map(|&l| w.agents[l].id).collect();
                w.events.push(
                    tick,
                    Some(h),
                    Severity::Notable,
                    EventKind::HearthTale {
                        teller,
                        listeners: ids,
                        what: format!("how their people tell it: {telling}"),
                    },
                );
                if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
                    if c.hearth_told.0 != day {
                        c.hearth_told = (day, Vec::new());
                    }
                    c.hearth_told.1.push(teller);
                }
                continue;
            }
        }
        // Who has something to tell: a tale someone here lacks counts most.
        let mut best: Option<(f32, usize, Option<crate::lore::Tale>)> = None;
        for &t in &sitting {
            if told.contains(&w.agents[t].id) {
                continue;
            }
            let fresh = w.agents[t]
                .tales
                .iter()
                .find(|x| {
                    sitting
                        .iter()
                        .any(|&l| l != t && !crate::lore::has_tale(w, l, x))
                })
                .cloned();
            let lived = w.agents[t]
                .beliefs
                .list
                .iter()
                .any(|b| b.sources.iter().any(|s| matches!(s, Source::Event(_))));
            let score = (if fresh.is_some() { 2.0 } else { 0.0 } + if lived { 1.0 } else { 0.2 })
                * w.agents[t].talk_weight();
            if best.as_ref().is_none_or(|b| score > b.0) {
                best = Some((score, t, fresh));
            }
        }
        let Some((_, t, fresh)) = best else { continue };
        let mut r = crate::rng::stream(
            w.meta.seed,
            &[crate::rng::purpose::LORE, 0x4EA7, cid.0 as u64, tick],
        );
        let listeners: Vec<usize> = sitting.iter().copied().filter(|&l| l != t).collect();
        let teller = w.agents[t].id;
        let tname = w.agents[t].name.clone();
        let what = match fresh {
            Some(tale) => {
                let mut version = None;
                for &l in &listeners {
                    if !crate::lore::has_tale(w, l, &tale) {
                        let (c, _) = crate::lore::tell(
                            w,
                            t,
                            l,
                            &tale,
                            format!("told by {tname} at the hearth"),
                            &mut r,
                        );
                        version.get_or_insert(c);
                    }
                }
                let name = crate::lore::site_name_of(w, tale.site);
                format!(
                    "the tale that {}",
                    version.unwrap_or(tale.claim).telling(&name)
                )
            }
            None => {
                let lived = w.agents[t]
                    .beliefs
                    .list
                    .iter()
                    .filter(|b| b.sources.iter().any(|s| matches!(s, Source::Event(_))))
                    .max_by(|a, b| {
                        (a.stance.abs() * a.confidence)
                            .partial_cmp(&(b.stance.abs() * b.confidence))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .cloned();
                for &l in &listeners {
                    crate::systems::beliefs::share(w, t, l, tick);
                }
                match lived {
                    Some(b) => format!(
                        "how they came to hold that {}",
                        crate::mind::beliefs::phrase(b.topic, b.stance)
                    ),
                    None => "what they remember of the other side of the portal".to_string(),
                }
            }
        };
        for &l in &listeners {
            w.agents[l].relate(teller, 0.03, 0.05, tick);
            let lid = w.agents[l].id;
            w.agents[t].relate(lid, 0.02, 0.05, tick);
            w.agents[l].met(teller, tick);
            w.agents[t].met(lid, tick);
        }
        let ids: Vec<AgentId> = listeners.iter().map(|&l| w.agents[l].id).collect();
        let event = w.events.push(
            tick,
            Some(h),
            Severity::Notable,
            EventKind::HearthTale {
                teller,
                listeners: ids,
                what: what.clone(),
            },
        );
        if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
            if c.hearth_told.0 != day {
                c.hearth_told = (day, Vec::new());
            }
            c.hearth_told.1.push(teller);
        }
        // The habit: the first evening becomes a custom, each evening after renews it.
        if has_custom(w, cid, |k| *k == CustomKind::HearthNight) {
            for c in w
                .customs
                .iter_mut()
                .filter(|c| c.community == cid && !c.lapsed && c.kind == CustomKind::HearthNight)
            {
                c.intensity = (c.intensity + 0.04).min(1.0);
            }
        } else {
            add_custom(
                w,
                cid,
                CustomKind::HearthNight,
                event,
                tick,
                format!(
                    "Since the night {tname} told {what} by the hearth, we gather there at dusk."
                ),
            );
        }
    }
}

/// What a people is known by: its customs, and each conviction held by at least two thirds of
/// its grown members (with a stance past 0.3 the same way). A conviction it is already known
/// by stays until fewer than half still lean that way, so one person's doubt does not undo a
/// people. Recorded when a line first becomes true and when it stops being true.
pub fn identity_lines(w: &World, cid: CommunityId, was: &[String]) -> Vec<String> {
    let mut lines: Vec<String> = w
        .customs
        .iter()
        .filter(|c| c.community == cid && !c.lapsed)
        .map(|c| c.label())
        .collect();
    let adults: Vec<&crate::agents::Agent> = w
        .agents
        .iter()
        .filter(|a| {
            a.alive
                && a.community == Some(cid)
                && a.species == crate::agents::Species::Person
                && crate::time::age_years(a.born, w.meta.tick) >= 14
        })
        .collect();
    if adults.len() >= 2 {
        for t in crate::past::Topic::ALL {
            for side in [-1.0f32, 1.0] {
                let line = format!("hold that {}", crate::setup::held_words(t, side));
                let known = was.contains(&line);
                let firm = if known { 0.15 } else { 0.3 };
                let n = adults
                    .iter()
                    .filter(|a| a.beliefs.get(t).is_some_and(|b| b.stance * side > firm))
                    .count();
                let holds = if known {
                    n * 2 >= adults.len()
                } else {
                    n * 3 >= adults.len() * 2
                };
                if holds {
                    lines.push(line);
                }
            }
        }
    }
    lines
}

/// Record what each people has come to be (daily at dawn, and once at the arrival).
pub fn identity(w: &mut World) {
    let tick = w.meta.tick;
    let cids: Vec<CommunityId> = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .map(|c| c.id)
        .collect();
    for cid in cids {
        let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
            continue;
        };
        let (home, was) = (c.home, c.identity.clone());
        let now = identity_lines(w, cid, &was);
        for line in now.iter().filter(|l| !was.contains(l)) {
            w.events.push(
                tick,
                Some(home),
                Severity::Notable,
                EventKind::IdentityFormed {
                    community: cid,
                    line: line.clone(),
                },
            );
        }
        for line in was.iter().filter(|l| !now.contains(l)) {
            w.events.push(
                tick,
                Some(home),
                Severity::Notable,
                EventKind::IdentityLapsed {
                    community: cid,
                    line: line.clone(),
                },
            );
        }
        if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
            c.identity = now;
        }
    }
}

fn community_of(w: &World, a: AgentId) -> Option<CommunityId> {
    w.agents
        .iter()
        .find(|x| x.id == a)
        .and_then(|x| x.community)
}

fn name(w: &World, a: AgentId) -> String {
    w.agents
        .iter()
        .find(|x| x.id == a)
        .map(|x| x.name.clone())
        .unwrap_or_else(|| format!("#{}", a.0))
}

fn has_custom(w: &World, cid: CommunityId, pred: impl Fn(&CustomKind) -> bool) -> bool {
    w.customs
        .iter()
        .any(|c| c.community == cid && !c.lapsed && pred(&c.kind))
}

/// Whether a custom of this kind, lapsed or not, was already formed from this `origin` event:
/// a lapsed custom needs a new cause, not the same one re-read from the 30-day log (OST-907).
fn formed_from(
    w: &World,
    cid: CommunityId,
    origin: EventId,
    pred: impl Fn(&CustomKind) -> bool,
) -> bool {
    w.customs
        .iter()
        .any(|c| c.community == cid && c.origin == origin && pred(&c.kind))
}

/// A community still has someone to hold a custom: not dissolved, and someone of it is in the
/// valley or on the road (the same check `moving::endings` uses to record it ended,
/// `moving.rs:563-571`).
fn community_alive(w: &World, cid: CommunityId) -> bool {
    let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
        return false;
    };
    if c.dissolved {
        return false;
    }
    w.agents
        .iter()
        .any(|a| a.alive && a.community == Some(cid) && a.species == crate::agents::Species::Person)
        || w.overworld.on_the_road(cid).next().is_some()
}

pub(crate) fn add_custom(
    w: &mut World,
    cid: CommunityId,
    kind: CustomKind,
    origin: EventId,
    origin_tick: u64,
    narrative: String,
) {
    let id = CustomId(w.next_custom_id);
    w.next_custom_id += 1;
    let tick = w.meta.tick;
    let members: Vec<AgentId> = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .map(|c| c.members.clone())
        .unwrap_or_default()
        .into_iter()
        .filter(|m| w.agents.iter().any(|a| a.id == *m && a.alive))
        .collect();
    let label = Custom {
        id,
        community: cid,
        kind: kind.clone(),
        origin,
        origin_tick,
        narrative: narrative.clone(),
        intensity: 0.8,
        adopted_by: members.clone(),
        formed: tick,
        lapsed: false,
        version: 1,
    }
    .label();
    w.customs.push(Custom {
        id,
        community: cid,
        kind: kind.clone(),
        origin,
        origin_tick,
        narrative: narrative.clone(),
        intensity: 0.8,
        adopted_by: members,
        formed: tick,
        lapsed: false,
        version: 1,
    });
    if let CustomKind::Welcome(n) = kind {
        if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
            c.welcome = n;
        }
    }
    let home = w.communities.iter().find(|c| c.id == cid).map(|c| c.home);
    w.events.push(
        tick,
        home,
        Severity::Major,
        EventKind::CustomFormed {
            community: cid,
            custom: id.0,
            label,
            origin,
            narrative,
        },
    );
}

/// Significant events of the last thirty days become customs, once per kind per community.
fn form(w: &mut World) {
    let tick = w.meta.tick;
    let recent: Vec<(EventId, u64, EventKind)> = w
        .culture_log
        .iter()
        .filter(|(_, t, _)| tick.saturating_sub(*t) < TICKS_PER_DAY * 30)
        .cloned()
        .collect();
    for (eid, etick, kind) in recent {
        match kind {
            EventKind::Died { agent, cause } => {
                let Some(cid) = community_of(w, agent) else {
                    continue;
                };
                if !community_alive(w, cid) {
                    continue;
                }
                // A day of rest each season on the day they died, remembering them.
                if !has_custom(w, cid, |k| matches!(k, CustomKind::RestDay { .. }))
                    && !formed_from(w, cid, eid, |k| matches!(k, CustomKind::RestDay { .. }))
                {
                    let day = Calendar::from_tick(etick).day_of_season;
                    let who = name(w, agent);
                    let ordinal = match day {
                        1 => "first".to_string(),
                        2 => "second".to_string(),
                        3 => "third".to_string(),
                        4 => "fourth".to_string(),
                        5 => "fifth".to_string(),
                        n => format!("{n}th"),
                    };
                    let narrative = format!(
                        "On the {} day of each season we rest, because {} died that day, {}.",
                        ordinal, who, cause
                    );
                    add_custom(
                        w,
                        cid,
                        CustomKind::RestDay {
                            day_of_season: day,
                            remembered: agent,
                        },
                        eid,
                        etick,
                        narrative,
                    );
                }
                if cause == "starvation"
                    && !has_custom(w, cid, |k| matches!(k, CustomKind::StoreFood))
                    && !formed_from(w, cid, eid, |k| matches!(k, CustomKind::StoreFood))
                {
                    let who = name(w, agent);
                    let narrative = format!(
                        "We sow and cook more than we need, because {} starved.",
                        who
                    );
                    add_custom(w, cid, CustomKind::StoreFood, eid, etick, narrative);
                }
            }
            EventKind::Split {
                from,
                cause,
                leader,
                ..
            } if community_alive(w, from)
                && !has_custom(w, from, |k| matches!(k, CustomKind::Welcome(_)))
                && !formed_from(w, from, eid, |k| matches!(k, CustomKind::Welcome(_))) =>
            {
                // The ones left behind close their doors; the ones who left open theirs.
                {
                    let who = name(w, leader);
                    let narrative = format!(
                        "We take in no strangers, since {} led people away: {}.",
                        who, cause
                    );
                    add_custom(
                        w,
                        from,
                        CustomKind::Welcome(crate::society::WelcomeNorm::Closed),
                        eid,
                        etick,
                        narrative,
                    );
                }
            }
            EventKind::Starving { agent } => {
                // Someone who went hungry on a food they remember sourly makes it a taboo
                // if they are well known in the community.
                let Some(cid) = community_of(w, agent) else {
                    continue;
                };
                if !community_alive(w, cid) {
                    continue;
                }
                let a = w.agents.iter().find(|x| x.id == agent).unwrap();
                let known: f32 = a.relationships.values().map(|r| r.familiarity).sum();
                let sour = a
                    .memories
                    .long
                    .iter()
                    .filter(|m| m.kind == MemKind::Meal && m.valence < -0.5 && m.strength > 0.5)
                    .find_map(|m| match m.subject {
                        Subject::Food(k) => Some(k),
                        _ => None,
                    });
                if let Some(k) = sour {
                    if known > 1.5
                        && !has_custom(w, cid, |c| matches!(c, CustomKind::FoodTaboo(x) if *x == k))
                        && !formed_from(
                            w,
                            cid,
                            eid,
                            |c| matches!(c, CustomKind::FoodTaboo(x) if *x == k),
                        )
                    {
                        let who = name(w, agent);
                        let narrative = format!(
                            "We do not eat {}, because {} fell ill of it and went hungry.",
                            format!("{k:?}").to_lowercase(),
                            who
                        );
                        add_custom(w, cid, CustomKind::FoodTaboo(k), eid, etick, narrative);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Members who joined later adopt customs after three days; children from age eight.
fn spread(w: &mut World) {
    let tick = w.meta.tick;
    let mut adoptions = Vec::new();
    for c in w.customs.iter_mut().filter(|c| !c.lapsed) {
        let members: Vec<AgentId> = w
            .communities
            .iter()
            .find(|x| x.id == c.community)
            .map(|x| x.members.clone())
            .unwrap_or_default();
        for m in members {
            if c.adopted_by.contains(&m) {
                continue;
            }
            let Some(a) = w.agents.iter().find(|a| a.id == m && a.alive) else {
                continue;
            };
            let joined_long_enough = a
                .relationships
                .values()
                .filter(|r| r.familiarity > 0.2)
                .count()
                >= 2;
            let old_enough = a.age_years(tick) >= 8;
            if joined_long_enough && old_enough {
                c.adopted_by.push(m);
                c.version += 1;
                adoptions.push((
                    m,
                    c.id.0,
                    c.community,
                    c.narrative.clone(),
                    c.origin,
                    c.version,
                ));
            }
        }
    }
    for (m, custom, cid, narrative, origin, version) in adoptions {
        let pos = w.agents.iter().find(|a| a.id == m).map(|a| a.pos);
        w.events.push(
            tick,
            pos,
            Severity::Trace,
            EventKind::CustomAdopted {
                agent: m,
                community: cid,
                custom,
                origin,
                narrative,
                version,
            },
        );
    }
}

/// On a rest day the community gathers; the custom is renewed and the event recorded.
fn observe_rest_days(w: &mut World) {
    let tick = w.meta.tick;
    let cal = Calendar::from_tick(tick);
    let mut observed = Vec::new();
    for c in w.customs.iter_mut().filter(|c| !c.lapsed) {
        if let CustomKind::RestDay {
            day_of_season,
            remembered,
        } = c.kind
        {
            if cal.day_of_season == day_of_season {
                c.intensity = (c.intensity + 0.3).min(1.0);
                observed.push((c.community, remembered, c.id.0));
            }
        }
    }
    for (cid, remembered, custom) in observed {
        let home = w.communities.iter().find(|c| c.id == cid).map(|c| c.home);
        w.events.push(
            tick,
            home,
            Severity::Major,
            EventKind::RestDay {
                community: cid,
                remembered,
                custom,
            },
        );
    }
}

fn fade(w: &mut World) {
    let tick = w.meta.tick;
    let mut lapsed = Vec::new();
    for c in w.customs.iter_mut().filter(|c| !c.lapsed) {
        // A habit kept by doing it fades in weeks when it is not kept; the rest over a year.
        c.intensity *= if c.kind == CustomKind::HearthNight {
            0.96
        } else {
            0.995
        };
        // Nobody left who holds it: it lapses.
        let holders_alive = c
            .adopted_by
            .iter()
            .filter(|m| w.agents.iter().any(|a| a.id == **m && a.alive))
            .count();
        if c.intensity < 0.1 || holders_alive == 0 {
            c.lapsed = true;
            lapsed.push((c.community, c.id.0, c.label()));
        }
    }
    for (cid, custom, label) in lapsed {
        let home = w.communities.iter().find(|c| c.id == cid).map(|c| c.home);
        w.events.push(
            tick,
            home,
            Severity::Notable,
            EventKind::CustomLapsed {
                community: cid,
                custom,
                label,
            },
        );
    }
}

/// Is today a rest day for this person's community?
pub fn rest_day_for(w: &World, a: &crate::agents::Agent) -> bool {
    let Some(cid) = a.community else { return false };
    let day = Calendar::from_tick(w.meta.tick).day_of_season;
    w.customs.iter().any(|c| {
        c.community == cid
            && !c.lapsed
            && c.adopted_by.contains(&a.id)
            && matches!(c.kind, CustomKind::RestDay { day_of_season, .. } if day_of_season == day)
    })
}

/// Food taboo multiplier for this person (0.25 if tabooed, 1.0 otherwise).
pub fn food_factor(w: &World, a: &crate::agents::Agent, kind: crate::items::ItemKind) -> f32 {
    let Some(cid) = a.community else { return 1.0 };
    if w.customs.iter().any(|c| {
        c.community == cid
            && !c.lapsed
            && c.adopted_by.contains(&a.id)
            && matches!(c.kind, CustomKind::FoodTaboo(k) if k == kind)
    }) {
        0.25
    } else {
        1.0
    }
}

pub fn stores_food(w: &World, cid: CommunityId) -> bool {
    w.customs
        .iter()
        .any(|c| c.community == cid && !c.lapsed && matches!(c.kind, CustomKind::StoreFood))
}

/// Work-cooldown key: had a drink at the hearth today (C34).
const DRANK_KEY: u32 = 0xD21C;

/// Ale or wine from the community's store (anywhere within reach of home: someone fetches
/// it) is shared among those sitting at the hearth who do not hold against drink (their
/// past's Drink conviction, past.rs); once a day each. Those who drink feel it and grow a
/// little closer; those set against it go without.
fn share_drink(
    w: &mut World,
    h: crate::ids::TilePos,
    home: crate::ids::TilePos,
    sitting: &[usize],
) {
    use crate::items::{ItemKind, ItemPlace};
    use crate::mind::beliefs::Topic;
    let tick = w.meta.tick;
    let drinkers: Vec<usize> = sitting
        .iter()
        .copied()
        .filter(|&k| {
            let a = &w.agents[k];
            a.beliefs.lean(Topic::Drink) > -0.2
                && a.of_age(tick)
                && a.work_cooldowns.get(&DRANK_KEY).is_none_or(|t| *t <= tick)
        })
        .collect();
    if drinkers.len() < 2 {
        return;
    }
    let mut drank = Vec::new();
    let mut what = "";
    for &k in &drinkers {
        let Some(ix) = w.items.iter().position(|it| {
            matches!(it.kind, ItemKind::Ale | ItemKind::Wine)
                && matches!(it.place, ItemPlace::Ground(p) if p.dist(home) <= 45)
        }) else {
            break;
        };
        what = if w.items[ix].kind == ItemKind::Ale {
            "ale"
        } else {
            "berry wine"
        };
        let id = w.items[ix].id;
        w.consume_item(id, 1, "drunk");
        w.agents[k]
            .work_cooldowns
            .insert(DRANK_KEY, tick + 16 * crate::time::TICKS_PER_HOUR);
        w.agents[k]
            .thoughts
            .add(crate::mind::ThoughtKind::SharedDrink, tick, None);
        drank.push(k);
    }
    if drank.len() < 2 {
        return;
    }
    for &a in &drank {
        for &b in &drank {
            if a != b {
                let other = w.agents[b].id;
                w.agents[a].relate(other, 0.02, 0.04, tick);
                w.agents[a].met(other, tick);
            }
        }
    }
    let ids: Vec<AgentId> = drank.iter().map(|&k| w.agents[k].id).collect();
    w.events.push(
        tick,
        Some(h),
        Severity::Minor,
        EventKind::DrankTogether {
            drinkers: ids,
            what: what.to_string(),
        },
    );
}
