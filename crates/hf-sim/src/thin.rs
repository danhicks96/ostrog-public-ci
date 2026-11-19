//! The thin place (issue #55): the portal the ten came through does not close cleanly.
//!
//! The portal stone stands where they arrived. Two seasons on, the air there thins and goes on
//! thinning a little each day. Within its reach things happen that nobody can account for, and
//! each is drawn from what is recorded, never invented: two people dream the same dream of the
//! home they left; a thing from home lies by the stone, made by someone named in a survivor's
//! past, and the one who knew them knows it; at dusk an echo stands by the stone, of one of the
//! seven who died alone, and those who see it know how they died; snow falls on a patch of
//! summer ground; and close to the stone a hollow spreads where nothing grows and nobody sleeps
//! well. Nothing is sent on a schedule: each thing is a chance a day, scaled by how thin it has
//! grown, and Mirre makes it thin faster.
//!
//! The curious, and anyone it has touched, go and sit by the stone and come to understand it.
//! Someone who does then chooses, by temperament and belief: the cautious set four warding
//! stones about it (rough stones raised from the valley's rock), which slow and then undo the
//! thinning; the curious and unafraid leave it
//! open, and it goes on thinning, and in time it leads one of the living lost back to the stone.

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, ItemId, TilePos};
use crate::items::{ItemKind, ItemPlace};
use crate::mind::techniques::{Learned, Technique};
use crate::mind::{MemKind, Subject, ThoughtKind, Trait};
use crate::opening::SurvivorState;
use crate::rng::{purpose, stream};
use crate::structures::StructureKind;
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::World;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Days after the arrival before the air at the stone thins: two seasons.
pub const WAKES_AFTER_DAYS: u64 = 10;
/// How far the stone's reach can grow, beyond its first three tiles.
pub const REACH: f32 = 12.0;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ThinState {
    /// When the air at the stone first thinned.
    pub woke: Option<u64>,
    /// How thin it has grown, 0..1.
    pub strength: f32,
    /// A patch of wrong weather: where, until when, and whether it is cold (snow in a warm
    /// season) or warm (a thaw in the cold).
    pub patches: Vec<(TilePos, u64, bool)>,
    /// Echoes standing by the stone now (for the viewer): where, until, whose.
    pub echoes: Vec<(TilePos, u64, String)>,
    /// Who saw an echo last and when (one sighting in ten days each).
    pub seen: Vec<(AgentId, u64)>,
    /// Things from home that came through, and who they belonged with (the survivor whose
    /// past names their maker), until recognised.
    pub from_home: Vec<(ItemId, AgentId, String)>,
    /// Who has sat by the stone lately, and since when (decide.rs Goal::StudyThin).
    pub studying: Vec<(AgentId, u64)>,
    /// The choice made about it, once: who, and whether they warded it.
    pub chosen: Option<(AgentId, bool)>,
    /// One of the living lost it is leading back.
    pub guiding: Option<AgentId>,
    /// Quiet again after the warding.
    pub quieted: bool,
    /// The valley's own dead whose echo has stood by the stone (C68, #229).
    #[serde(default)]
    pub echoed: Vec<AgentId>,
    /// When an echo last stood: one in fifteen days at most (C68: with the valley's own dead
    /// counted, the first soaks saw one every four days).
    #[serde(default)]
    pub last_echo: u64,
}

/// The portal stone's tile, if one stands.
pub fn stone(w: &World) -> Option<TilePos> {
    w.structures
        .iter()
        .find(|s| s.kind == StructureKind::PortalStone)
        .map(|s| s.pos)
}

/// The least time between two echoes (C68, #229).
pub const ECHO_GAP: u64 = 15 * TICKS_PER_DAY;

/// How far the stone reaches now.
pub fn radius(w: &World) -> f32 {
    3.0 + REACH * w.thin.strength
}

/// Whether a tile is inside the hollow (half the reach, once it has grown thin enough).
pub fn hollow(w: &World, p: TilePos) -> bool {
    match stone(w) {
        Some(s) if w.thin.strength > 0.3 && p.z == s.z => (p.dist(s) as f32) <= radius(w) * 0.5,
        _ => false,
    }
}

/// Place the stone at the arrival: where they came through.
pub fn place_stone(w: &mut World) {
    let p = w.meta.founding_site;
    let sid = w.alloc_structure_id();
    w.structures.push(crate::structures::Structure {
        id: sid,
        kind: StructureKind::PortalStone,
        pos: p,
        material: crate::world::Material::GRANITE,
        progress: 1.0,
        active: false,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
}

fn wards(w: &World, s: TilePos) -> usize {
    w.structures
        .iter()
        .filter(|x| x.kind == StructureKind::WardStone && x.progress >= 1.0 && x.pos.dist(s) <= 5)
        .count()
}

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    let Some(s) = stone(w) else { return };
    w.thin.echoes.retain(|e| e.1 > tick);
    w.thin.patches.retain(|p| p.1 > tick);
    if tick.is_multiple_of(TICKS_PER_HOUR) {
        hourly(w, s);
    }
    if tick % TICKS_PER_DAY != TICKS_PER_HOUR * 4 {
        if w.thin.woke.is_some() && tick % TICKS_PER_DAY == TICKS_PER_HOUR * 19 {
            echo(w, s);
        }
        return;
    }
    // At four in the morning: does it wake, how thin does it grow, what comes of it today.
    if w.thin.woke.is_none() {
        if w.since_arrival() >= WAKES_AFTER_DAYS * TICKS_PER_DAY {
            w.thin.woke = Some(tick);
            w.thin.strength = 0.05;
            set_glow(w, true);
            w.events.push(
                tick,
                Some(s),
                Severity::Notable,
                EventKind::ThinStirred { at: s },
            );
        }
        return;
    }
    let mirre = w.meta.engine == crate::engine::Engine::Whimsical;
    let open = matches!(w.thin.chosen, Some((_, false)));
    let n = wards(w, s);
    let grow = 0.02 * if mirre { 1.5 } else { 1.0 } * if open { 1.5 } else { 1.0 };
    if n >= 4 {
        w.thin.strength = (w.thin.strength - 0.05).max(0.0);
        if w.thin.strength <= 0.0 && !w.thin.quieted {
            w.thin.quieted = true;
            set_glow(w, false);
            w.events.push(
                tick,
                Some(s),
                Severity::Major,
                EventKind::ThinQuieted { at: s },
            );
        }
    } else {
        w.thin.strength = (w.thin.strength + grow * 0.5f32.powi(n as i32)).min(1.0);
    }
    if w.thin.quieted {
        return;
    }
    let mut r = stream(w.meta.seed, &[purpose::THIN, tick]);
    let st = w.thin.strength;
    if r.gen_bool((0.4 * st) as f64) {
        shared_dream(w, s, &mut r);
    }
    if r.gen_bool((0.3 * st) as f64) {
        thing_from_home(w, s, &mut r);
    }
    if r.gen_bool((0.35 * st) as f64) {
        wrong_weather(w, s, &mut r);
    }
    if open && st >= 0.5 && w.thin.guiding.is_none() {
        // The living lost: one is drawn back toward the stone.
        w.thin.guiding = w.survivor_pool.iter().find_map(|sl| match sl.state {
            SurvivorState::Lost {
                agent,
                found: false,
            } if w.agents.iter().any(|a| a.id == agent && a.alive) => Some(agent),
            _ => None,
        });
    }
}

fn set_glow(w: &mut World, on: bool) {
    for x in w
        .structures
        .iter_mut()
        .filter(|x| x.kind == StructureKind::PortalStone)
    {
        x.active = on;
    }
}

/// Hourly: the hollow weighs on whoever lingers or sleeps in it; recognition of things from
/// home; the studying; the lost being led back.
fn hourly(w: &mut World, s: TilePos) {
    let tick = w.meta.tick;
    for i in 0..w.agents.len() {
        let a = &w.agents[i];
        if !a.alive || a.species != crate::agents::Species::Person || !hollow(w, a.pos) {
            continue;
        }
        let k = if a.activity.is_asleep() {
            ThoughtKind::SleptUneasy
        } else {
            ThoughtKind::Uneasy
        };
        w.agents[i].thoughts.add(k, tick, None);
    }
    // A patch of the wrong season is felt by whoever is in it.
    for i in 0..w.agents.len() {
        let a = &w.agents[i];
        if !a.alive || a.species != crate::agents::Species::Person {
            continue;
        }
        match patch_at(w, a.pos) {
            Some(true) => {
                let n = &mut w.agents[i].needs;
                n.warmth = (n.warmth - 0.2).max(0.0);
                w.agents[i].thoughts.add(ThoughtKind::Cold, tick, None);
            }
            Some(false) => {
                let n = &mut w.agents[i].needs;
                n.warmth = (n.warmth + 0.2).min(1.0);
            }
            None => {}
        }
    }
    recognise(w);
    study(w, s);
    if let Some(lost) = w.thin.guiding {
        lead_home(w, s, lost);
    }
}

/// Things from home are known when the one whose past names their maker sees them: lying
/// on the ground near them, in the hands (or on the back) of someone near them, or in their
/// own hands.
fn recognise(w: &mut World) {
    let tick = w.meta.tick;
    let mut done = Vec::new();
    for (k, (item, who, maker)) in w.thin.from_home.clone().into_iter().enumerate() {
        let Some(place) = w.items.iter().find(|it| it.id == item).map(|it| it.place) else {
            done.push(k);
            continue;
        };
        // Where it is now, and whether it is the one it came for who has it.
        let (p, theirs) = match place {
            ItemPlace::Ground(p) => (p, false),
            ItemPlace::Carried(h) | ItemPlace::Held(h) => {
                // Only on someone here and living: one away on a journey, gone from the
                // valley or dead is not where anyone sees what they have.
                let Some(holder) = w.agents.iter().find(|a| a.id == h && a.alive) else {
                    continue;
                };
                (holder.pos, h == who)
            }
        };
        let Some(i) = w.agents.iter().position(|a| {
            a.id == who
                && a.alive
                && !a.activity.is_asleep()
                && (theirs || (a.pos.z == p.z && a.pos.dist(p) <= 6))
        }) else {
            continue;
        };
        w.agents[i].remember(tick, MemKind::Discovery, Subject::Place(p), 0.9, 0.9, None);
        let ev = w.events.push(
            tick,
            Some(p),
            Severity::Major,
            EventKind::Recognised {
                agent: who,
                item,
                maker: maker.clone(),
            },
        );
        if let Some(it) = w.items.iter_mut().find(|it| it.id == item) {
            if let Some(st) = it.story.as_mut() {
                st.deeds
                    .push((tick, format!("known again by {}", w.agents[i].name)));
            }
        }
        let _ = ev;
        done.push(k);
    }
    for k in done.into_iter().rev() {
        w.thin.from_home.remove(k);
    }
}

/// Two or more sleeping in the community dream the same dream, of the home they left: of a
/// thing that happened there, from one dreamer's recorded past.
fn shared_dream(w: &mut World, s: TilePos, r: &mut impl Rng) {
    let tick = w.meta.tick;
    let reach = radius(w) + 20.0;
    let sleepers: Vec<usize> = (0..w.agents.len())
        .filter(|&i| {
            let a = &w.agents[i];
            a.alive
                && a.species == crate::agents::Species::Person
                && a.community.is_some()
                && a.activity.is_asleep()
                && (a.pos.dist(s) as f32) <= reach
        })
        .collect();
    if sleepers.len() < 2 {
        return;
    }
    // Whose past the dream comes from: someone among the sleepers who lived one.
    let Some((src, what)) = sleepers.iter().find_map(|&i| {
        let a = &w.agents[i];
        let slot = slot_of(w, a.id)?;
        let life = crate::past::life(w.meta.seed, slot.0, slot.1, &a.traits);
        let pick = life
            .events
            .iter()
            .filter(|e| e.what.is_trauma())
            .nth(r.gen_range(0..3))?;
        Some((i, dream_of(&pick.what, &life.home)))
    }) else {
        return;
    };
    let mut dreamers = vec![src];
    for &i in sleepers.iter().filter(|&&i| i != src) {
        if dreamers.len() < 4 && r.gen_bool(0.6) {
            dreamers.push(i);
        }
    }
    if dreamers.len() < 2 {
        return;
    }
    let ids: Vec<AgentId> = dreamers.iter().map(|&i| w.agents[i].id).collect();
    let ev = w.events.push(
        tick,
        Some(s),
        Severity::Notable,
        EventKind::SharedDream {
            dreamers: ids.clone(),
            of: what,
        },
    );
    for &i in &dreamers {
        for o in &ids {
            if *o != w.agents[i].id {
                w.agents[i].relate(*o, 0.05, 0.15, tick);
            }
        }
        w.agents[i].remember(
            tick,
            MemKind::Discovery,
            Subject::Place(s),
            0.4,
            0.7,
            Some(ev),
        );
    }
}

/// The survivor slot (index, rerolls) of someone who came through, if they did.
fn slot_of(w: &World, id: AgentId) -> Option<(u8, u32)> {
    let name = &w.agents.iter().find(|a| a.id == id)?.name;
    w.survivor_pool
        .iter()
        .find(|s| crate::opening::slot_name(w.meta.seed, s.index, s.rerolls) == *name)
        .map(|s| (s.index, s.rerolls))
}

fn dream_of(f: &crate::past::Formative, home: &str) -> String {
    use crate::past::Formative as F;
    match f {
        F::Famine => format!("the famine winter in {home}"),
        F::HouseFire | F::VillageFire => format!("{home} burning"),
        F::Raid => format!("the raiders coming into {home}"),
        F::Fever => format!("the fever in {home}"),
        F::Flood => format!("the water rising in {home}"),
        F::ParentDied { parent, .. } => format!("{parent}, dead in {home}"),
        F::SiblingDied { sibling, .. } => format!("{sibling}, dead in {home}"),
        F::ChildDied { child } => format!("{child}, who died in {home}"),
        F::Betrayed { by } => format!("{by}, in {home}"),
        _ => format!("the road out of {home}"),
    }
}

/// A thing from home lies by the stone: made by someone named in the past of one of those
/// who came through, of the kind their craft or life would make.
fn thing_from_home(w: &mut World, s: TilePos, r: &mut impl Rng) {
    use crate::past::Formative as F;
    let tick = w.meta.tick;
    let people: Vec<usize> = (0..w.agents.len())
        .filter(|&i| {
            let a = &w.agents[i];
            a.alive && a.species == crate::agents::Species::Person && a.community.is_some()
        })
        .collect();
    if people.is_empty() {
        return;
    }
    let i = people[r.gen_range(0..people.len())];
    let a = &w.agents[i];
    let Some(slot) = slot_of(w, a.id) else { return };
    let life = crate::past::life(w.meta.seed, slot.0, slot.1, &a.traits);
    let made: Vec<(String, ItemKind, String)> = life
        .events
        .iter()
        .filter_map(|e| match &e.what {
            F::Apprenticed { craft, master } => {
                let kind = match craft.as_str() {
                    "the smith" => ItemKind::Hammer,
                    "the carpenter" => ItemKind::Saw,
                    "the quarry" => ItemKind::Chisel,
                    "the herb-woman" => ItemKind::Needle,
                    "the baker" => ItemKind::Bucket,
                    _ => ItemKind::Tool,
                };
                Some((master.clone(), kind, format!("{craft} of {}", life.home)))
            }
            F::Married { partner } => Some((
                partner.clone(),
                ItemKind::Clothes,
                format!("their partner in {}", life.home),
            )),
            F::LeftBehind { who } => Some((
                who.clone(),
                ItemKind::Clothes,
                format!("left behind in {}", life.home),
            )),
            F::KindElder { elder } => Some((
                elder.clone(),
                ItemKind::Tool,
                format!("an elder of {}", life.home),
            )),
            _ => None,
        })
        .collect();
    if made.is_empty() {
        return;
    }
    let (maker, kind, who) = made[r.gen_range(0..made.len())].clone();
    let here = (0..12).find_map(|_| {
        let p = TilePos::new(s.x + r.gen_range(-2..=2), s.y + r.gen_range(-2..=2), s.z);
        (p != s && w.grid.walkable(p)).then_some(p)
    });
    let Some(p) = here else { return };
    let item = w.create_lone_item(
        kind,
        1,
        ItemPlace::Ground(p),
        r.gen_range(0.4..0.8),
        "thin place",
    );
    let owner = w.agents[i].id;
    if let Some(it) = w.items.iter_mut().find(|it| it.id == item) {
        it.story = Some(Box::new(crate::legend::Story {
            maker: None,
            maker_name: format!("{maker}, {who}"),
            made: tick,
            wear: r.gen_range(0.4..0.9),
            origin: format!("from {}, come through the thin place", life.home),
            ..Default::default()
        }));
    }
    w.thin.from_home.push((item, owner, maker.clone()));
    w.events.push(
        tick,
        Some(p),
        Severity::Notable,
        EventKind::ThingFromHome {
            item,
            maker,
            from: life.home.clone(),
        },
    );
}

/// At dusk an echo may stand by the stone, seen by someone awake within twelve tiles. It is
/// one of the seven who died alone; whoever sees it knows then how they died.
fn echo(w: &mut World, s: TilePos) {
    let tick = w.meta.tick;
    let mut r = stream(w.meta.seed, &[purpose::THIN, 0xEC40, tick]);
    if w.thin.quieted || !r.gen_bool((0.5 * w.thin.strength) as f64) {
        return;
    }
    if w.thin.last_echo > 0 && tick.saturating_sub(w.thin.last_echo) < ECHO_GAP {
        return;
    }
    // Whose echo: one of the seven who died alone and unfound first; then anyone of the
    // valley who died with none of their own near (#229: the lost are nearly always found
    // alive within weeks, so in 2,100 soak days no echo was ever seen). `k` is the pool slot,
    // or none for the valley's own dead.
    let mut dead: Vec<(Option<usize>, AgentId, bool)> = w
        .survivor_pool
        .iter()
        .enumerate()
        .filter_map(|(k, sl)| match sl.state {
            SurvivorState::Lost { agent, found } => w
                .agents
                .iter()
                .find(|a| a.id == agent && !a.alive && a.death.is_some())
                .map(|_| (Some(k), agent, found)),
            _ => None,
        })
        .collect();
    for a in &w.agents {
        if !a.alive
            && a.died_alone
            && a.death.is_some()
            && a.species == crate::agents::Species::Person
            && !dead.iter().any(|d| d.1 == a.id)
        {
            let shown = w.thin.echoed.contains(&a.id);
            dead.push((None, a.id, shown));
        }
    }
    // Unknown fates first: that is what an echo brings. One already known comes back less.
    let Some(&(k, dead_id, found)) = dead.iter().find(|d| !d.2).or_else(|| dead.first()) else {
        return;
    };
    if found && r.gen_bool(0.5) {
        return;
    }
    let Some(seer) = w
        .agents
        .iter()
        .enumerate()
        .filter(|(_, a)| {
            a.alive
                && a.species == crate::agents::Species::Person
                && a.community.is_some()
                && !a.activity.is_asleep()
                && a.pos.z == s.z
                && a.pos.dist(s) <= 12
                && !w
                    .thin
                    .seen
                    .iter()
                    .any(|(who, t)| *who == a.id && tick < t + 10 * TICKS_PER_DAY)
        })
        .map(|(i, _)| i)
        .next()
    else {
        return;
    };
    let d = w.agents.iter().find(|a| a.id == dead_id).unwrap();
    let (name, cause) = (d.name.clone(), d.death.clone().unwrap_or_default());
    let at = TilePos::new(s.x + r.gen_range(-2..=2), s.y + r.gen_range(-2..=2), s.z);
    let at = if w.grid.walkable(at) { at } else { s };
    w.thin
        .echoes
        .push((at, tick + TICKS_PER_HOUR / 2, name.clone()));
    w.thin.last_echo = tick;
    let id = w.agents[seer].id;
    w.thin.seen.retain(|(who, _)| *who != id);
    w.thin.seen.push((id, tick));
    // Seen, their fate is known as finding them would have made it.
    match k {
        Some(k) if !found => {
            w.survivor_pool[k].state = SurvivorState::Lost {
                agent: dead_id,
                found: true,
            };
        }
        None if !w.thin.echoed.contains(&dead_id) => w.thin.echoed.push(dead_id),
        _ => {}
    }
    let ev = w.events.push(
        tick,
        Some(at),
        Severity::Major,
        EventKind::EchoSeen {
            agent: id,
            of: name,
            cause,
            known: found,
        },
    );
    w.agents[seer].remember(
        tick,
        MemKind::Hardship,
        Subject::Place(s),
        -0.5,
        0.9,
        Some(ev),
    );
    w.agents[seer]
        .thoughts
        .add(ThoughtKind::SawDeath, tick, Some(ev));
}

/// A patch of the wrong season: snow on warm ground, or a thaw in the cold.
fn wrong_weather(w: &mut World, s: TilePos, r: &mut impl Rng) {
    let tick = w.meta.tick;
    let rad = radius(w) as i32;
    let p = TilePos::new(
        (s.x + r.gen_range(-rad..=rad)).clamp(2, w.grid.width - 3),
        (s.y + r.gen_range(-rad..=rad)).clamp(2, w.grid.height - 3),
        s.z,
    );
    let cold = w.weather.temperature_c > 5.0;
    w.thin.patches.push((p, tick + 6 * TICKS_PER_HOUR, cold));
    w.events.push(
        tick,
        Some(p),
        Severity::Notable,
        EventKind::WrongWeather { at: p, cold },
    );
}

/// Whether a tile lies in a patch of wrong weather now, and which way: Some(true) for cold.
pub fn patch_at(w: &World, p: TilePos) -> Option<bool> {
    w.thin
        .patches
        .iter()
        .find(|(q, _, _)| q.z == p.z && q.dist(p) <= 3)
        .map(|x| x.2)
}

/// Those sitting by the stone come to understand it. Someone who does chooses what to do.
fn study(w: &mut World, s: TilePos) {
    let tick = w.meta.tick;
    let studying = w.thin.studying.clone();
    w.thin
        .studying
        .retain(|(_, since)| tick < since + 3 * TICKS_PER_HOUR);
    for (id, since) in studying {
        let Some(i) = w.agents.iter().position(|a| a.id == id && a.alive) else {
            continue;
        };
        if w.agents[i].pos.dist(s) > 2 || tick < since + TICKS_PER_HOUR {
            continue;
        }
        let mut r = stream(w.meta.seed, &[purpose::THIN, 0x57D, id.0 as u64, tick]);
        let a = &w.agents[i];
        let chance = 0.25
            + if a.traits.has(Trait::Curious) {
                0.15
            } else {
                0.0
            };
        let learned = r.gen_bool(chance);
        if learned {
            if w.agents[i].techniques.knows(Technique::ThinLore) {
                w.agents[i]
                    .techniques
                    .outcome(Technique::ThinLore, true, tick);
            } else {
                w.agents[i]
                    .techniques
                    .learn(Technique::ThinLore, Learned::Studied, 0.3, tick);
            }
        }
        w.events.push(
            tick,
            Some(s),
            Severity::Minor,
            EventKind::ThinStudied { agent: id, learned },
        );
        // An hour by the woken stone may wake what a carrier brought through (#308).
        crate::resonance::sat_by_stone(w, i);
        w.thin.studying.retain(|(x, _)| *x != id);
        if w.thin.chosen.is_none() && w.agents[i].techniques.skill(Technique::ThinLore) >= 0.4 {
            choose(w, i, s);
        }
    }
}

/// Someone who understands the thin place chooses, by temperament and belief, what to do.
fn choose(w: &mut World, i: usize, s: TilePos) {
    use crate::mind::beliefs::Topic;
    let tick = w.meta.tick;
    let a = &w.agents[i];
    let fearful = a.traits.has(Trait::Cautious)
        || a.beliefs.lean(Topic::Death) > 0.2
        || a.beliefs.lean(Topic::Strangers) > 0.2;
    let ward = fearful || !a.traits.has(Trait::Curious);
    let id = a.id;
    w.thin.chosen = Some((id, ward));
    let (what, because) = if ward {
        (
            "chose to set warding stones about the portal stone",
            if a.traits.has(Trait::Cautious) {
                "because what comes through there is not theirs to let in"
            } else {
                "because the hollow is spreading toward the houses"
            },
        )
    } else {
        (
            "chose to leave the thin place open",
            "because whatever is on the other side has something to tell them",
        )
    };
    w.events.push(
        tick,
        Some(s),
        Severity::Major,
        EventKind::Chose {
            agent: id,
            what: what.into(),
            because: because.into(),
        },
    );
    if ward {
        // One in each quarter about the stone, as near as there is open ground they can reach.
        let me = w.agents[i].pos;
        for (sx, sy) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
            let at = (2..=5)
                .flat_map(|d| {
                    // Strictly inside the quarter, so no two stones share a side.
                    (1..=d)
                        .map(move |k| (sx * d, sy * k))
                        .chain((1..d).map(move |k| (sx * k, sy * d)))
                })
                .map(|(dx, dy)| s.offset(dx, dy, 0))
                .find(|q| {
                    w.grid.walkable(*q)
                        && !w.structures.iter().any(|x| x.pos == *q)
                        && !w.designations.iter().any(|d| !d.done && d.pos == *q)
                        && w.nav.as_ref().is_some_and(|n| n.reach(me, *q))
                });
            let Some(at) = at else { continue };
            let mut d = crate::systems::settlement::make(
                w,
                crate::work::DesignationKind::BuildStructure(
                    StructureKind::WardStone,
                    crate::world::Material::GRANITE,
                ),
                at,
                8,
            );
            // Their decision; the people's work: anyone may help raise them.
            d.community = w.agents[i].community;
            w.designations.push(d);
        }
    }
}

/// The lost it leads walk toward the stone, where the others will find them.
fn lead_home(w: &mut World, s: TilePos, lost: AgentId) {
    let still_lost = w
        .survivor_pool
        .iter()
        .any(|sl| matches!(sl.state, SurvivorState::Lost { agent, found: false } if agent == lost));
    let Some(i) = w.agents.iter().position(|a| a.id == lost && a.alive) else {
        w.thin.guiding = None;
        return;
    };
    if !still_lost {
        w.thin.guiding = None;
        return;
    }
    if w.agents[i].pos.dist(s) <= 2 || w.agents[i].activity.is_asleep() {
        return;
    }
    let to = [
        s.offset(0, 1, 0),
        s.offset(1, 0, 0),
        s.offset(-1, 0, 0),
        s.offset(0, -1, 0),
    ]
    .into_iter()
    .find(|q| w.grid.walkable(*q));
    if let Some(to) = to {
        if w.path_agent_to(i, to) {
            w.agents[i].activity = crate::mind::Activity::Wandering { to };
            w.agents[i].activity_ticks = 0;
        }
    }
}

/// What the portal stone says to someone hovering over it.
pub fn note(w: &World, p: TilePos) -> Option<String> {
    let s = stone(w)?;
    if p != s {
        return None;
    }
    let mut line = "The stone where the ten came through.".to_string();
    match w.thin.woke {
        None => {}
        Some(_) if w.thin.quieted => line.push_str(" The air about it is still again."),
        Some(t) => {
            let days = w.meta.tick.saturating_sub(t) / TICKS_PER_DAY;
            let how = match w.thin.strength {
                x if x < 0.25 => "a little thin",
                x if x < 0.5 => "thin",
                x if x < 0.75 => "very thin",
                _ => "worn almost through",
            };
            line.push_str(&format!(
                " For {days} days the air about it has been {how}."
            ));
        }
    }
    Some(line)
}
