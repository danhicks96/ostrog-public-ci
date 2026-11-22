//! Personal projects proposed (P0 of milestone MP; project.rs). Once a game day each community's
//! people look at what they want and what the place has: a carver with a memory and a bare wall,
//! someone who holds that drink eases things when the store is full, a hand at a craft with no
//! workshop for it, someone unafraid of strangers when a known settlement would pay for the
//! surplus. The best placed among them proposes it, recorded with the reason, and the work
//! follows. Projects end when their work is done, when nothing comes of them in five days, or
//! when their proposer is gone.

use rand::Rng;

use crate::agents::Species;
use crate::events::{EventKind, Severity};
use crate::expedition::{CaravanState, Purpose};
use crate::ids::{AgentId, CommunityId, EventId};
use crate::items::ItemKind;
use crate::mind::beliefs::{Source, Topic};
use crate::mind::Skill;
use crate::project::{open_of, Project, ProjectKind, PROJECT_DAYS};
use crate::rng::{purpose, stream};
use crate::structures::StructureKind;
use crate::systems::settlement::{count_kind, Here, EATING_HOURS_A_DAY};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::world_state::World;

/// "a smelter", "an anvil".
fn shop_name(k: StructureKind) -> &'static str {
    match k {
        StructureKind::CarpenterBench => "a carpenter's bench",
        StructureKind::Loom => "a loom",
        StructureKind::Smelter => "a smelter",
        StructureKind::Anvil => "an anvil",
        StructureKind::Kiln => "a kiln",
        _ => "a workshop",
    }
}

/// When people take stock of what they want, each day.
pub const AT: u64 = TICKS_PER_HOUR * 8 + 17;

/// A project rule, run for one community.
type Rule = fn(&mut World, CommunityId);

pub fn step(w: &mut World) {
    if w.meta.tick % TICKS_PER_DAY != AT || !w.meta.autoplan {
        return;
    }
    close(w);
    let ids: Vec<CommunityId> = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .map(|c| c.id)
        .collect();
    let trace = std::env::var("HF_TRACE_PROJECTS").is_ok();
    for cid in ids {
        let steps: [(&str, Rule); 12] = [
            ("carve", carve),
            ("brew", brew),
            ("workshops", workshops),
            ("journey", journey),
            ("arms", arms),
            ("home", home),
            ("shelter", shelter),
            ("wall", wall),
            ("shrine", shrine),
            ("well", well),
            ("channel", channel),
            ("move", crate::systems::moving::consider),
        ];
        for (name, f) in steps {
            let t0 = trace.then(std::time::Instant::now);
            f(w, cid);
            if let Some(t0) = t0 {
                let ms = t0.elapsed().as_secs_f64() * 1000.0;
                if ms > 5.0 {
                    eprintln!("PROJECTS {name} community {} {ms:.1} ms", cid.0);
                }
            }
        }
    }
    crate::systems::moving::endings(w);
}

/// Grown people of the community who can want things.
fn members(w: &World, cid: CommunityId) -> Vec<usize> {
    let tick = w.meta.tick;
    (0..w.agents.len())
        .filter(|&i| {
            let a = &w.agents[i];
            a.alive
                && a.community == Some(cid)
                && a.species == Species::Person
                && a.band.is_none()
                && !a.leaving
                && a.of_age(tick)
        })
        .collect()
}

fn propose(
    w: &mut World,
    i: usize,
    cid: CommunityId,
    kind: ProjectKind,
    what: String,
    because: String,
    source: Option<EventId>,
) {
    let tick = w.meta.tick;
    let id = w.next_project_id;
    w.next_project_id += 1;
    let by = w.agents[i].id;
    let pos = w.agents[i].pos;
    w.projects.push(Project {
        id,
        by,
        community: cid,
        kind,
        because: because.clone(),
        source,
        proposed: tick,
        work: None,
        done: false,
        site: None,
    });
    w.events.push(
        tick,
        Some(pos),
        Severity::Notable,
        EventKind::Proposed {
            agent: by,
            what,
            because,
        },
    );
}

/// The first recorded event behind a belief, if it has one.
fn belief_source(w: &World, i: usize, t: Topic) -> Option<EventId> {
    w.agents[i].beliefs.get(t).and_then(|b| {
        b.sources.iter().find_map(|s| match s {
            Source::Event(e) | Source::Ideology(e) => Some(*e),
            Source::Person(_) | Source::Species(_) | Source::Culture(_) => None,
        })
    })
}

/// ", as Hann taught them" when a belief came from someone and not from an event.
fn taught_by(w: &World, i: usize, t: Topic) -> String {
    let Some(b) = w.agents[i].beliefs.get(t) else {
        return String::new();
    };
    if b.sources
        .iter()
        .any(|s| matches!(s, Source::Event(_) | Source::Ideology(_)))
    {
        return String::new();
    }
    b.sources
        .iter()
        .find_map(|s| match s {
            Source::Person(p) => w.agents.iter().find(|a| a.id == *p).map(|a| a.name.clone()),
            _ => None,
        })
        .map(|n| format!(", as {n} taught them"))
        .unwrap_or_default()
}

/// Ends projects whose work is done or gone, that came to nothing in their days, or whose
/// proposer is dead or gone; keeps the record of the last fifty that ended.
fn close(w: &mut World) {
    let tick = w.meta.tick;
    for k in 0..w.projects.len() {
        if w.projects[k].done {
            continue;
        }
        let p = w.projects[k].clone();
        // Someone on the road (a caravan under way, a journey of their own) is kept whole in
        // `overworld.away` until they return; they are away, not gone (#197 case 14, FIXLOG
        // 2026-11-12: their project ended the day they set off). A captive or a child gone
        // below for good is also kept in `away`, and is gone: `Overworld::on_the_road` tells
        // the two apart (#197 case 7).
        let gone = !w
            .agents
            .iter()
            .any(|a| a.id == p.by && a.alive && a.community == Some(p.community))
            && !w.overworld.on_the_road(p.community).any(|a| a.id == p.by);
        let old = tick.saturating_sub(p.proposed) >= PROJECT_DAYS * TICKS_PER_DAY;
        let ended = match &p.kind {
            // A well ends when it is built over water struck, or when the planner gives it up
            // (wells.rs); a shaft takes longer than the days a work order is waited for.
            ProjectKind::Well => match p.work {
                Some(d) => !w.designations.iter().any(|x| x.id == d && !x.done),
                None => tick.saturating_sub(p.proposed) >= 30 * TICKS_PER_DAY,
            },
            ProjectKind::Carve | ProjectKind::Workshop(_) | ProjectKind::Shrine => match p.work {
                Some(d) => !w.designations.iter().any(|x| x.id == d && !x.done),
                None => old,
            },
            // A channel ends when its cut is whole or given up (channels.rs): it takes as many
            // days as it has tiles.
            ProjectKind::Channel => !w.channels.iter().any(|c| {
                c.community == p.community && !c.done && !c.gave_up && !c.tiles.is_empty()
            }),
            ProjectKind::Brew => old,
            ProjectKind::Journey(_) => true,
            ProjectKind::Arms => match p.work {
                Some(d) => !w.designations.iter().any(|x| x.id == d && !x.done),
                None => true,
            },
            // A home ends when it stands, or when no place was found for it in its days.
            ProjectKind::Home(form) => match p.site {
                Some(site) => crate::systems::settlement::home_built(w, p.community, *form, site),
                None => old,
            },
            ProjectKind::Move { .. } => true,
            // A wall ends when it stands whole, when its ring was left behind by a move (#403),
            // or when no ring could be laid in its days.
            ProjectKind::Wall => match p.site {
                Some(_) => w
                    .communities
                    .iter()
                    .find(|c| c.id == p.community)
                    .is_none_or(|c| c.palisade.as_ref().is_none_or(|pal| pal.stood.is_some())),
                None => old,
            },
            ProjectKind::Shelter => match p.site {
                Some(site) => crate::systems::settlement::home_built(
                    w,
                    p.community,
                    crate::project::HomeForm::Hollow,
                    site,
                ),
                None => old,
            },
        };
        if gone || ended {
            w.projects[k].done = true;
        }
    }
    let done: Vec<usize> = (0..w.projects.len())
        .filter(|k| w.projects[*k].done)
        .collect();
    if done.len() > 50 {
        let drop: std::collections::BTreeSet<u32> = done[..done.len() - 50]
            .iter()
            .map(|k| w.projects[*k].id)
            .collect();
        w.projects.retain(|p| !drop.contains(&p.id));
    }
}

/// A carving: someone who builds well, remembers something not yet carved, and has a bare wall
/// to cut it into; one at a time, five days apart, at most three for each house.
fn carve(w: &mut World, cid: CommunityId) {
    if open_of(w, cid, |k| *k == ProjectKind::Carve).is_some() {
        return;
    }
    let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
        return;
    };
    let (home, houses) = (c.home, c.house_sites.len());
    let ours: Vec<u64> = w
        .engravings
        .iter()
        .filter(|(p, _)| p.dist(home) <= 45)
        .map(|(_, e)| e.carved)
        .collect();
    let last = ours.iter().copied().max().unwrap_or(0);
    if ours.len() >= houses * 3
        || (!ours.is_empty() && w.meta.tick.saturating_sub(last) < 5 * TICKS_PER_DAY)
        || crate::engrave::wall_to_carve(w, home, 40).is_none()
    {
        return;
    }
    // The most skilled who has something to carve (ties to the first): asked in order of skill,
    // since asking each of five hundred what they would carve was a 111 ms day (MP review).
    let mut able: Vec<(f32, usize)> = members(w, cid)
        .into_iter()
        .map(|i| (w.agents[i].skills.level(Skill::Construction), i))
        .filter(|(skill, _)| *skill >= 4.0)
        .collect();
    able.sort_by(|a, b| b.0.total_cmp(&a.0));
    // The last knower of a craft their own ground uses, old enough it could die with them,
    // carves the way of it before anything else (D37, #1395): the craft outlives them.
    let craft: Option<(usize, crate::mind::Technique)> = able
        .iter()
        .find_map(|&(_, i)| crate::engrave::craft_to_carve(w, i).map(|t| (i, t)));
    let best: Option<(usize, Option<EventId>, String)> = if let Some((i, t)) = craft {
        Some((
            i,
            None,
            format!("the way of {}, as {} did it", t.name(), w.agents[i].name),
        ))
    } else {
        able.into_iter().find_map(|(_, i)| {
            crate::engrave::choose(w, i).map(|(event, _, line)| (i, Some(event), line))
        })
    };
    if let Some((i, event, line)) = best {
        let line = line.trim_end_matches('.').to_string();
        let because = match event {
            Some(_) => format!("remembers it: {line}"),
            None => format!("is the last who knows it: {line}"),
        };
        propose(
            w,
            i,
            cid,
            ProjectKind::Carve,
            "to carve it into a wall".into(),
            because,
            event,
        );
    }
}

/// Ale or wine for a season: someone who holds that drink eases things, from a store with ten
/// days' eating a mouth and grain or berries to spare.
fn brew(w: &mut World, cid: CommunityId) {
    if open_of(w, cid, |k| *k == ProjectKind::Brew).is_some() {
        return;
    }
    let Some(home) = w.communities.iter().find(|c| c.id == cid).map(|c| c.home) else {
        return;
    };
    let here = Here::of(w, cid, home);
    let stock = |k: ItemKind| count_kind(w, &here, k);
    let people = members(w, cid).len().max(1) as f32;
    let food_hours: f32 = [
        ItemKind::Meal,
        ItemKind::RawMeat,
        ItemKind::Root,
        ItemKind::GrainSheaf,
        ItemKind::Berries,
    ]
    .iter()
    .map(|k| stock(*k) as f32 * k.nutrition_hours())
    .sum();
    let plenty = food_hours >= people * EATING_HOURS_A_DAY * 10.0;
    let drink = stock(ItemKind::Ale) + stock(ItemKind::Wine);
    let makings = stock(ItemKind::GrainSheaf) >= 8 || stock(ItemKind::Berries) >= 10;
    if !plenty || !makings || drink >= 6 {
        return;
    }
    // Someone who holds that drink eases things, or that a merry night is good for everyone
    // (Merriment, P7).
    let best = members(w, cid)
        .into_iter()
        .map(|i| {
            let b = &w.agents[i].beliefs;
            (b.lean(Topic::Drink).max(b.lean(Topic::Merriment)), i)
        })
        .filter(|(l, _)| *l > 0.15)
        .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)));
    if let Some((_, i)) = best {
        let source = belief_source(w, i, Topic::Drink);
        let because = format!(
            "holds that {}{}",
            crate::mind::beliefs::phrase(Topic::Drink, 1.0),
            taught_by(w, i, Topic::Drink)
        );
        propose(
            w,
            i,
            cid,
            ProjectKind::Brew,
            "to brew from the full store".into(),
            because,
            source,
        );
    }
}

/// A workshop: someone with the craft and nowhere to work it, once the materials are at hand
/// (ore for a smelter, bars for an anvil, dug clay for a kiln) and there are hands to spare.
fn workshops(w: &mut World, cid: CommunityId) {
    let Some(home) = w.communities.iter().find(|c| c.id == cid).map(|c| c.home) else {
        return;
    };
    let people = members(w, cid);
    if people.len() < 8 {
        return;
    }
    let here = Here::of(w, cid, home);
    let stock = |k: ItemKind| count_kind(w, &here, k);
    let tick = w.meta.tick;
    // Not wanted again for a season where no place was found (#242).
    let no_site: Vec<StructureKind> = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .map(|c| {
            c.no_site
                .iter()
                .filter(|(_, until)| *until > tick)
                .map(|(k, _)| *k)
                .collect()
        })
        .unwrap_or_default();
    // (workshop, materials at hand, the skill it wants, the least level, the craft of a past)
    let shops = [
        (
            StructureKind::CarpenterBench,
            true,
            Skill::Construction,
            2.0,
            "the carpenter",
        ),
        (StructureKind::Loom, true, Skill::Crafting, 1.0, ""),
        (
            StructureKind::Smelter,
            stock(ItemKind::IronOre) + stock(ItemKind::CopperOre) + stock(ItemKind::Ore) >= 2,
            Skill::Crafting,
            2.0,
            "the smith",
        ),
        (
            StructureKind::Anvil,
            stock(ItemKind::IronBar) + stock(ItemKind::CopperBar) >= 1,
            Skill::Crafting,
            2.0,
            "the smith",
        ),
        (
            StructureKind::Kiln,
            stock(ItemKind::Clay) >= 2,
            Skill::Crafting,
            1.0,
            "",
        ),
    ];
    for (kind, ready, skill, least, craft) in shops {
        if no_site.contains(&kind) {
            continue;
        }
        let built = w
            .structures
            .iter()
            .any(|s| s.kind == kind && here.holds(s.pos));
        if !ready || built || open_of(w, cid, |k| *k == ProjectKind::Workshop(kind)).is_some() {
            continue;
        }
        // The best placed: a past in the craft first, then skill.
        let mut best: Option<(f32, usize, String, Option<EventId>)> = None;
        for &i in &people {
            let level = w.agents[i].skills.level(skill);
            let learned = if craft.is_empty() {
                None
            } else {
                crate::opening::formatives(w, w.agents[i].id)
                    .into_iter()
                    .find_map(|(e, f)| match f {
                        crate::past::Formative::Apprenticed { craft: c, master } if c == craft => {
                            Some((e, master))
                        }
                        _ => None,
                    })
            };
            if level < least && learned.is_none() {
                continue;
            }
            let score = level + if learned.is_some() { 3.0 } else { 0.0 };
            let (because, source) = match learned {
                Some((e, master)) => (format!("learned the craft from {master}, {craft}"), Some(e)),
                None => (
                    format!(
                        "is a good hand at {}",
                        if skill == Skill::Construction {
                            "building"
                        } else {
                            "making things"
                        }
                    ),
                    None,
                ),
            };
            if best.as_ref().is_none_or(|b| score > b.0) {
                best = Some((score, i, because, source));
            }
        }
        if let Some((_, i, because, source)) = best {
            propose(
                w,
                i,
                cid,
                ProjectKind::Workshop(kind),
                format!("to build {}", shop_name(kind)),
                because,
                source,
            );
        }
    }
}

/// Spears (C52): while a band that has raided the store is camped near, someone who holds that
/// force answers force, or that strangers bring harm, proposes making them at the bench, two at
/// a time, until there is one for every three grown people. The raid is the cause the record
/// holds: the band's camp near and the store it took from.
fn arms(w: &mut World, cid: CommunityId) {
    if open_of(w, cid, |k| *k == ProjectKind::Arms).is_some() {
        return;
    }
    let Some(home) = w.communities.iter().find(|c| c.id == cid).map(|c| c.home) else {
        return;
    };
    let Some(band) = w
        .bands
        .iter()
        .find(|b| b.taken > 0 && b.camp.z == home.z && b.camp.dist(home) <= 60)
        .map(|b| b.site_name.clone())
    else {
        return;
    };
    let people = members(w, cid);
    let here = Here::of(w, cid, home);
    let spears = count_kind(w, &here, ItemKind::Weapon)
        + people
            .iter()
            .filter(|&&i| w.agents[i].weapon.is_some())
            .count() as u32;
    if spears * 3 >= people.len() as u32 {
        return;
    }
    let Some(bench) = w
        .structures
        .iter()
        .find(|s| {
            s.kind == crate::structures::StructureKind::Workbench
                && s.progress >= 1.0
                && here.holds(s.pos)
        })
        .map(|s| s.pos)
    else {
        return;
    };
    if count_kind(w, &here, ItemKind::Log) < 2 {
        return;
    }
    let best = people
        .iter()
        .map(|&i| {
            let b = &w.agents[i].beliefs;
            let (v, s) = (b.lean(Topic::Violence), b.lean(Topic::Strangers));
            (v.max(s), v >= s, i)
        })
        .filter(|(l, _, _)| *l > 0.15)
        .max_by(|a, b| a.0.total_cmp(&b.0).then(b.2.cmp(&a.2)));
    let Some((_, force, i)) = best else { return };
    let topic = if force {
        Topic::Violence
    } else {
        Topic::Strangers
    };
    let source = belief_source(w, i, topic);
    let because = format!(
        "holds that {}{}, with strangers from {band} taking from the store",
        crate::mind::beliefs::phrase(topic, 1.0),
        taught_by(w, i, topic)
    );
    propose(
        w,
        i,
        cid,
        ProjectKind::Arms,
        "to make spears".into(),
        because,
        source,
    );
    // The bench work, at once: two spears, owned by the community.
    let mut first = None;
    for _ in 0..2 {
        let mut d = crate::systems::settlement::make(
            w,
            crate::work::DesignationKind::Craft(crate::work::recipes::WOOD_SPEAR),
            bench,
            8,
        );
        d.community = Some(cid);
        first.get_or_insert(d.id);
        w.designations.push(d);
    }
    if let Some(p) = w
        .projects
        .iter_mut()
        .rev()
        .find(|p| p.community == cid && p.kind == ProjectKind::Arms && !p.done)
    {
        p.work = first;
    }
}

/// A journey: when the community can spare the hands and the food, someone unafraid of
/// strangers proposes to carry the surplus to a settlement they know will pay for it, or to go
/// and find one they have only heard of. They lead the party.
fn journey(w: &mut World, cid: CommunityId) {
    let tick = w.meta.tick;
    let rest = crate::systems::expedition::REST_AFTER;
    if w.overworld.caravans.iter().any(|c| {
        c.community == cid
            && (!matches!(c.state, CaravanState::Done | CaravanState::Lost)
                || tick - c.finished < rest)
    }) {
        return;
    }
    let Some(home) = w.communities.iter().find(|c| c.id == cid).map(|c| c.home) else {
        return;
    };
    let mut r = stream(
        w.meta.seed,
        &[
            purpose::AGENT_SOCIAL,
            0xCA,
            cid.0 as u64,
            tick / TICKS_PER_DAY,
        ],
    );
    // About as often as a party used to form: most days, not every day.
    if r.gen::<f32>() > 0.68 {
        return;
    }
    let able = crate::systems::expedition::able(w, cid);
    if able.len() < crate::systems::expedition::MIN_ABLE {
        return;
    }
    let food_home: u32 = w
        .items
        .iter()
        .filter(|it| {
            it.kind.is_food()
                && matches!(it.place, crate::items::ItemPlace::Ground(p) if p.dist(home) <= 40)
        })
        .map(|it| it.count)
        .sum();
    if food_home < able.len() as u32 * 3 {
        return;
    }
    // Who would go: not afraid of strangers, the best travellers first.
    let willing: Vec<(f32, AgentId)> = able
        .iter()
        .filter_map(|id| {
            let a = w.agents.iter().find(|a| a.id == *id)?;
            let fear = a.beliefs.lean(Topic::Strangers);
            // Nobody with a small child leads a party away (form_caravan leaves them home).
            let small_child = a.children.iter().any(|c| {
                w.agents
                    .iter()
                    .any(|x| x.id == *c && x.alive && x.age_years(tick) < 8)
            });
            if small_child {
                return None;
            }
            // Someone back from the road lately leaves the next journey to another.
            let lately = a.last_caravan > 0 && tick - a.last_caravan < 20 * TICKS_PER_DAY;
            (fear <= 0.2).then(|| {
                (
                    a.skills.level(Skill::Hauling) + a.skills.level(Skill::Social)
                        - fear * 5.0
                        // A fair trade helps both (Commerce, P7).
                        + a.beliefs.lean(Topic::Commerce) * 4.0
                        - if lately { 3.0 } else { 0.0 },
                    *id,
                )
            })
        })
        .collect();
    let Some(&(_, lead)) = willing
        .iter()
        .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)))
    else {
        return;
    };
    let people = able.len() as u32;
    // A building material the community lacks and cannot get at home (C59): stone with no
    // rock face within reach, timber with no tree. Someone goes to fetch it from a settlement
    // they know has it to spare, carrying what they can best trade.
    if let Some((site, carry, want, why, name)) = lacking(w, cid, home, people) {
        let Some(i) = w.agents.iter().position(|a| a.id == lead) else {
            return;
        };
        let purpose = Purpose::Fetch { site, carry, want };
        let source = belief_source(w, i, Topic::Strangers);
        let Some(cv) =
            crate::systems::expedition::form_caravan_led(w, cid, purpose.clone(), Some(lead))
        else {
            return;
        };
        if !w
            .overworld
            .caravans
            .iter()
            .find(|c| c.id == cv)
            .is_some_and(|c| c.members.contains(&lead))
        {
            return;
        }
        let because = format!(
            "{why}; {name} has {} to spare",
            crate::systems::expedition::lower(want)
        );
        propose(
            w,
            i,
            cid,
            ProjectKind::Journey(purpose),
            format!(
                "to fetch {} from {name}",
                crate::systems::expedition::lower(want)
            ),
            because,
            source,
        );
        return;
    }
    let (purpose, because) = match crate::trade::best_errand(w, cid, home, people) {
        Some((site, carry, _)) => {
            let name = w
                .overworld
                .site(site)
                .map(|s| s.name.clone())
                .unwrap_or_default();
            (
                Purpose::Trade { site, carry },
                format!(
                    "thinks {name} will pay for their {}",
                    crate::systems::expedition::lower(carry)
                ),
            )
        }
        None => {
            let unknown: Vec<(u32, String)> = w
                .overworld
                .sites
                .iter()
                .filter(|s| !s.known_by.contains(&cid))
                .map(|s| (s.id, s.name.clone()))
                .collect();
            if unknown.is_empty() || r.gen::<f32>() >= 0.5 {
                return;
            }
            let (site, name) = unknown[r.gen_range(0..unknown.len())].clone();
            (
                Purpose::Scout { site },
                format!("wants to see what lies at {name}"),
            )
        }
    };
    let Some(i) = w.agents.iter().position(|a| a.id == lead) else {
        return;
    };
    let source = belief_source(w, i, Topic::Strangers);
    // The party forms first: a journey that cannot go (too few free to travel, no way out of
    // the valley) is not proposed, so it is neither recorded nor tried again every day.
    let Some(cv) =
        crate::systems::expedition::form_caravan_led(w, cid, purpose.clone(), Some(lead))
    else {
        return;
    };
    let led = w
        .overworld
        .caravans
        .iter()
        .find(|c| c.id == cv)
        .is_some_and(|c| c.members.contains(&lead));
    if !led {
        return;
    }
    propose(
        w,
        i,
        cid,
        ProjectKind::Journey(purpose),
        "to lead a party out of the valley".into(),
        because,
        source,
    );
}

/// A home (C57): when someone of the community slept on the ground last night and there are
/// fewer places to sleep than people, they propose one, in the form their kind builds: a house,
/// a hollow dug into the rock, tents or nests. The night on the ground is the cause the record
/// holds. One home is under way at a time; the planner finds the place and posts the work.
fn home(w: &mut World, cid: CommunityId) {
    if open_of(w, cid, |k| matches!(k, ProjectKind::Home(_))).is_some() {
        return;
    }
    // A home that found no place in its days is not proposed again for ten days: in the C57
    // soaks, seed 3's planner found no flat ground for a while and the same wish was made 58
    // times in 300 days.
    let tick = w.meta.tick;
    let lately = w.projects.iter().any(|p| {
        p.community == cid
            && p.done
            && p.site.is_none()
            && matches!(p.kind, ProjectKind::Home(_))
            && tick.saturating_sub(p.proposed) < (PROJECT_DAYS + 10) * TICKS_PER_DAY
    });
    // Nor while a house is still going up.
    let building = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .is_some_and(|c| c.house_stage.iter().any(|s| *s == 1 || *s == 2));
    if lately || building {
        return;
    }
    let Some(home) = w.communities.iter().find(|c| c.id == cid).map(|c| c.home) else {
        return;
    };
    let here = Here::of(w, cid, home);
    let everyone = w
        .agents
        .iter()
        .filter(|a| a.alive && a.community == Some(cid) && a.species == Species::Person)
        .count() as u32;
    let places = w
        .structures
        .iter()
        .filter(|s| s.kind.is_sleeping_place() && here.holds(s.pos))
        .count() as u32
        + w
            .designations
            .iter()
            .filter(|d| {
                !d.done
                    && matches!(d.kind, crate::work::DesignationKind::BuildStructure(k, _) if k.is_sleeping_place())
                    && here.holds(d.pos)
            })
            .count() as u32;
    if places >= everyone {
        return;
    }
    // Who slept rough: the best builder among them speaks for it (first id on a tie).
    let rough = crate::mind::thoughts::ThoughtKind::SleptRough;
    let Some(i) = members(w, cid)
        .into_iter()
        .filter(|&i| w.agents[i].thoughts.has(rough))
        .max_by(|&a, &b| {
            let (sa, sb) = (
                w.agents[a].skills.level(Skill::Construction),
                w.agents[b].skills.level(Skill::Construction),
            );
            sa.total_cmp(&sb).then(w.agents[b].id.cmp(&w.agents[a].id))
        })
    else {
        return;
    };
    let source = w.agents[i].thoughts.source_of(rough);
    let builds = w.agents[i].species_def().habitat.builds.clone();
    let form = crate::project::HomeForm::of(&builds);
    let because = format!("slept on the ground, {everyone} of them and {places} places to sleep");
    propose(
        w,
        i,
        cid,
        ProjectKind::Home(form),
        format!("to build {}", form.name()),
        because,
        source,
    );
}

/// A shelter (C58): after strangers took from the store, someone who holds that strangers bring
/// harm, or that you never raise a hand, or who is a careful one, proposes digging a room into
/// the rock (or down into the ground) behind a barred door and keeping the stores there. Those
/// who answer force with force make spears instead (`arms`); one who holds that the deep is not
/// to be dug will not dig. One shelter is enough; the raid is the cause in the record.
///
/// #405 (FIXLOG 2026-10-27): it once wanted a raiding band camped on home's level at the
/// morning's planning, a holder of either belief, and a rock face near; in four 200-day
/// openings no morning had all three. The raid is now the one the community recorded (as the
/// wall's is, C73), a careful nature will do (as for the wall), and ground to dig down into
/// serves where no face is near (as for a dug-out home, C67).
fn shelter(w: &mut World, cid: CommunityId) {
    if open_of(w, cid, |k| *k == ProjectKind::Shelter).is_some() {
        return;
    }
    let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
        return;
    };
    if c.hollows.iter().any(|h| h.shelter) {
        return;
    }
    let home = c.home;
    let tick = w.meta.tick;
    let Some(band) = c
        .hardships
        .iter()
        .rev()
        .find(|(t, what, _)| {
            what.ends_with("took from the store")
                && tick.saturating_sub(*t) < 20 * crate::time::TICKS_PER_DAY
        })
        .map(|(_, what, _)| {
            what.trim_start_matches("strangers from ")
                .trim_end_matches(" took from the store")
                .to_string()
        })
    else {
        return;
    };
    let best = members(w, cid)
        .into_iter()
        .map(|i| {
            let b = &w.agents[i].beliefs;
            let (harm, meek) = (b.lean(Topic::Strangers), -b.lean(Topic::Violence));
            let held = harm.max(meek);
            let cautious = w.agents[i].traits.has(crate::mind::Trait::Cautious);
            (
                held.max(if cautious { 0.2 } else { 0.0 }),
                harm >= meek,
                held > 0.15,
                b.lean(Topic::DeepDigging),
                i,
            )
        })
        .filter(|(l, _, _, deep, _)| *l > 0.15 && *deep >= -0.15)
        .max_by(|a, b| a.0.total_cmp(&b.0).then(b.4.cmp(&a.4)));
    let Some((_, harm, by_belief, _, i)) = best else {
        return;
    };
    w.ensure_nav();
    let size = crate::systems::settlement::shelter_room(members(w, cid).len() as u32);
    if crate::systems::settlement::find_hollow_site(w, home, 30, &|r, _| r.len() >= size).is_none()
        && crate::systems::settlement::find_barred_dugout_site(w, home, 20, &|r, _| r.len() > size)
            .is_none()
    {
        return;
    }
    let (topic, stance) = if harm {
        (Topic::Strangers, 1.0)
    } else {
        (Topic::Violence, -1.0)
    };
    let (because, source) = if by_belief {
        (
            format!(
                "holds that {}{}, with strangers from {band} taking from the store",
                crate::mind::beliefs::phrase(topic, stance),
                taught_by(w, i, topic)
            ),
            belief_source(w, i, topic),
        )
    } else {
        (
            format!("is a careful one, with strangers from {band} taking from the store"),
            None,
        )
    };
    propose(
        w,
        i,
        cid,
        ProjectKind::Shelter,
        "to dig a shelter for the stores".into(),
        because,
        source,
    );
}

/// A wall round the houses (C73, D42): after strangers took from the store, one who holds
/// strangers a danger (or will not fight them) proposes it, once the community is eight and
/// has a house to wall; the planner grows the ring (settlement.rs `palisade`).
fn wall(w: &mut World, cid: CommunityId) {
    if open_of(w, cid, |k| *k == ProjectKind::Wall).is_some() {
        return;
    }
    let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
        return;
    };
    if c.palisade.is_some() || c.house_sites.is_empty() {
        return;
    }
    let home = c.home;
    let people = members(w, cid);
    if people.len() < 8 {
        return;
    }
    // A night's raid this community recorded as a hardship in the last twenty days (C73: a
    // band camped at home's level at the morning's planning was often gone or on another
    // level, and 45 night raids in a soak brought no wall). Takings by welcomed guests are
    // not hardships (#213).
    let tick = w.meta.tick;
    let Some(band) = c
        .hardships
        .iter()
        .rev()
        .find(|(t, what, _)| {
            what.ends_with("took from the store")
                && tick.saturating_sub(*t) < 20 * crate::time::TICKS_PER_DAY
        })
        .map(|(_, what, _)| {
            what.trim_start_matches("strangers from ")
                .trim_end_matches(" took from the store")
                .to_string()
        })
        // Or a people who hold strangers a danger wall themselves in unraided (#54: what a
        // community is known by shapes what it builds).
        .or_else(|| {
            c.identity
                .iter()
                .any(|l| l.contains("strangers are a danger"))
                .then(|| "beyond the valley".to_string())
        })
    else {
        return;
    };
    let _ = home;
    let best = people
        .into_iter()
        .map(|i| {
            let b = &w.agents[i].beliefs;
            let (harm, meek) = (b.lean(Topic::Strangers), -b.lean(Topic::Violence));
            let cautious = w.agents[i].traits.has(crate::mind::Trait::Cautious);
            // A careful nature alone is reason enough to want a wall (C73: in the soaks few
            // held strangers a danger strongly, and 238 night raids brought no wall).
            let held = harm.max(meek);
            (
                held.max(if cautious { 0.2 } else { 0.0 }),
                harm >= meek,
                held > 0.15,
                i,
            )
        })
        .filter(|(l, _, _, _)| *l > 0.15)
        .max_by(|a, b| a.0.total_cmp(&b.0).then(b.3.cmp(&a.3)));
    let Some((_, harm, by_belief, i)) = best else {
        return;
    };
    let (topic, stance) = if harm {
        (Topic::Strangers, 1.0)
    } else {
        (Topic::Violence, -1.0)
    };
    let (because, source) = if by_belief {
        (
            format!(
                "holds that {}{}, with strangers from {band} taking from the store",
                crate::mind::beliefs::phrase(topic, stance),
                taught_by(w, i, topic)
            ),
            belief_source(w, i, topic),
        )
    } else {
        (
            format!("is a careful one, with strangers from {band} taking from the store"),
            None,
        )
    };
    propose(
        w,
        i,
        cid,
        ProjectKind::Wall,
        "to raise a wall round the houses".into(),
        because,
        source,
    );
}

/// A shrine (#297, shrines.rs): a community that kept the last season's observance and has no
/// shrine standing; proposed by whoever of them most heeds the signs, the one they follow in
/// the spirit first. One whose community lost its last shrine remembers it.
fn shrine(w: &mut World, cid: CommunityId) {
    if open_of(w, cid, |k| *k == ProjectKind::Shrine).is_some()
        || crate::shrines::of(w, cid).is_some()
        || !w.faith.kept.iter().any(|(c, _)| *c == cid)
    {
        return;
    }
    let tick = w.meta.tick;
    let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
        return;
    };
    if c.no_site
        .iter()
        .any(|(k, until)| *k == StructureKind::Shrine && *until > tick)
    {
        return;
    }
    let spirit = c.leaders[crate::justice::Domain::Spirit as usize];
    let people = members(w, cid);
    if people.len() < 4 {
        return;
    }
    let lean = |i: usize| {
        w.agents[i].beliefs.lean(Topic::DeitySigns)
            + if Some(w.agents[i].id) == spirit {
                1.0
            } else {
                0.0
            }
    };
    let Some(i) = people
        .into_iter()
        .filter(|&i| w.agents[i].beliefs.lean(Topic::DeitySigns) >= crate::observances::LEADS_AT)
        .max_by(|&a, &b| {
            lean(a)
                .total_cmp(&lean(b))
                .then(w.agents[b].id.cmp(&w.agents[a].id))
        })
    else {
        return;
    };
    let lost = w
        .faith
        .lost
        .iter()
        .find(|(c, _, _)| *c == cid)
        .map(|(_, e, why)| (*e, why.clone()));
    let (because, source) = match lost {
        Some((e, why)) => (format!("remembers the shrine they had: {why}"), Some(e)),
        None => (
            format!(
                "holds that {}{}, and the observances want a place",
                crate::mind::beliefs::phrase(Topic::DeitySigns, 1.0),
                taught_by(w, i, Topic::DeitySigns)
            ),
            belief_source(w, i, Topic::DeitySigns),
        ),
    };
    propose(
        w,
        i,
        cid,
        ProjectKind::Shrine,
        "to raise a shrine".into(),
        because,
        source,
    );
}

/// A well (#433, wells.rs): a community whose home has no fresh water within a short walk,
/// and no well. The best digger among them proposes it, with how far they walk for water.
/// After one is given up, not again for a while.
fn well(w: &mut World, cid: CommunityId) {
    if open_of(w, cid, |k| *k == ProjectKind::Well).is_some() {
        return;
    }
    let tick = w.meta.tick;
    let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
        return;
    };
    if c.no_site
        .iter()
        .any(|(k, until)| *k == StructureKind::Well && *until > tick)
    {
        return;
    }
    let home = c.home;
    let far = crate::wells::water_within(w, home, 60);
    if far.is_some_and(|d| d <= crate::wells::SHORT_WALK) {
        return;
    }
    // A well already ordered, or one standing within the short walk, is enough.
    if w.designations.iter().any(|d| {
        !d.done
            && d.kind
                == crate::work::DesignationKind::BuildStructure(
                    StructureKind::Well,
                    crate::world::Material::STONE_BLOCK,
                )
            && d.pos.dist(home) <= crate::wells::SHORT_WALK
    }) {
        return;
    }
    let Some(i) = members(w, cid).into_iter().max_by(|&a, &b| {
        w.agents[a]
            .skills
            .level(Skill::Mining)
            .total_cmp(&w.agents[b].skills.level(Skill::Mining))
            .then(w.agents[b].id.cmp(&w.agents[a].id))
    }) else {
        return;
    };
    let because = match far {
        Some(d) => format!("the nearest water is {d} paces from home"),
        None => "there is no water within a long walk of home".to_string(),
    };
    propose(
        w,
        i,
        cid,
        ProjectKind::Well,
        "to dig a well".into(),
        because,
        None,
    );
}

/// A ditch from open water to a dry field (#325, channels.rs): someone who works the ground
/// proposes it when the field's crops are not drinking, water lies within reach of home and a
/// downhill cut from the shore can be found; one at a time, and not again for a while after one
/// was given up or no route was found.
fn channel(w: &mut World, cid: CommunityId) {
    if open_of(w, cid, |k| *k == ProjectKind::Channel).is_some()
        || !crate::channels::may_try(w, cid)
    {
        return;
    }
    if crate::channels::field_is_dry(w, cid).is_none() {
        return;
    }
    let Some(i) = members(w, cid).into_iter().max_by(|&a, &b| {
        w.agents[a]
            .skills
            .level(Skill::Farming)
            .total_cmp(&w.agents[b].skills.level(Skill::Farming))
            .then(w.agents[b].id.cmp(&w.agents[a].id))
    }) else {
        return;
    };
    let Some(path) = crate::channels::route(w, cid) else {
        crate::channels::begin(w, cid, Vec::new());
        return;
    };
    let because = format!("the field is dry and water lies {} paces off", path.len());
    crate::channels::begin(w, cid, path);
    propose(
        w,
        i,
        cid,
        ProjectKind::Channel,
        "to dig a channel to the field".into(),
        because,
        None,
    );
}

/// What the community lacks to build with and cannot get at home (C59): blocks when stone is
/// wanted and no rock face is within the planner's reach, logs when no tree is; with a known
/// settlement that has it to spare and something to carry there. (Site, carry, want, why, the
/// site's name.)
fn lacking(
    w: &World,
    cid: CommunityId,
    home: crate::ids::TilePos,
    people: u32,
) -> Option<(u32, ItemKind, ItemKind, String, String)> {
    let here = Here::of(w, cid, home);
    let blocks =
        count_kind(w, &here, ItemKind::StoneBlock) + count_kind(w, &here, ItemKind::RoughStone);
    let logs = count_kind(w, &here, ItemKind::Log);
    // As the quarry rule counts it (settlement.rs, step 4): the blocks the sites want, and four
    // for a hearth if the community has none.
    let hearth = w
        .structures
        .iter()
        .any(|s| s.kind == crate::structures::StructureKind::Hearth && here.holds(s.pos));
    let wanted =
        crate::systems::settlement::wanted_blocks_for_sites(w) + if hearth { 0 } else { 4 };
    // Reach is the paths' to judge: without them, nothing is known to be out of reach.
    w.nav.as_ref()?;
    // Out of reach as the planner judges it (#164): no face it would cut and no cut or quarry
    // shaft under way; no tree it would fell and no felling under way.
    let open =
        |k: crate::work::DesignationKind| w.designations.iter().any(|d| !d.done && d.kind == k);
    let want_stone = wanted > blocks
        && blocks < 4
        && !open(crate::work::DesignationKind::DigStairs)
        && !w.designations.iter().any(|d| {
            !d.done
                && d.kind == crate::work::DesignationKind::Mine
                && w.grid.materials.get(w.grid.get(d.pos).material).kind
                    == crate::world::material::MaterialKind::Stone
        })
        && crate::systems::settlement::nearest_rock_face(w, home, 45).is_none();
    let want_wood = logs < 4
        && !open(crate::work::DesignationKind::Chop)
        && crate::systems::settlement::chop_candidates(w, home).is_empty();
    let (want, why) = if want_stone {
        (
            ItemKind::StoneBlock,
            "the walls want stone and no rock face is within reach",
        )
    } else if want_wood {
        (
            ItemKind::Log,
            "the building wants timber and no tree stands within reach",
        )
    } else {
        return None;
    };
    // A settlement that will pay in it, for a cargo it will take (#164: the site was any known
    // one holding eight, empty or not, and the cargo was chosen for another site).
    let (site, carry) = crate::trade::best_fetch(w, cid, home, people, want)?;
    let name = w.overworld.site(site)?.name.clone();
    Some((site, carry, want, why.to_string(), name))
}
