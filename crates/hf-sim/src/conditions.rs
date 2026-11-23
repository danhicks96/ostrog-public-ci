//! Conditions with recorded causes (P11 of milestone MP, docs/design-peoples.md section 8): the
//! restless dead, and three curses. Every condition holds the row that gave it and who passed
//! it; nobody is taken by chance alone, and the living never know who is cursed. They see
//! patterns (someone gone at the full moon, a sleeper bitten, the dead eaten) and come to
//! suspect someone, with those rows as the source; a suspicion can be wrong.
//!
//! - The restless dead: one who died with no rite given and nothing carved is restless ten days
//!   on, or later, once nobody living remembers them, and is seen at dusk where they died by someone
//!   awake near; one who sees them and holds that the dead must be given their rite gives it.
//! - The Beast-hunger: a killer cursed by a deity who presses the strong (Hask) for the killing.
//!   On the night of the full moon they go out from where they sleep, kill a beast in the dark
//!   and may bite a sleeper out of doors, who is taken too. Each moon it holds them longer.
//! - The Thirst: one who, starving, ate of their own dead kin, when their people hold it
//!   abhorrent, cursed by the same deity. They want no food, but every three days they drink
//!   a sleeper's blood, and the sun burns them.
//! - The Stone-sleep: one who stood first in a deep cavern where the underworld's stone shows.
//!   Over forty days the body stiffens, and they die standing, a statue.
//! - The true-dream (#308): the thin place marks one who sleeps within its reach while it
//!   stands woke. Every third night they cry out in their sleep, of something true to the
//!   record, and their community suspects them of dealing with the hollow — a suspicion with
//!   a real row as its source, and wrong: they are the mark's victim, not its agent.
//! - The unbinding: a healer who knows it (the Hollin keep it) lifts the Beast-hunger, the
//!   Thirst or the true-dream from one of their community they have come to suspect.

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, EventId, TilePos};
use crate::mind::beliefs::Topic;
use crate::rng::{purpose, stream};
use crate::time::{Calendar, TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::world_state::World;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ConditionKind {
    /// On the dead: not laid to rest (stage 0), restless (stage 1).
    Unrested,
    BeastHunger,
    Thirst,
    StoneSleep,
    /// Appended (#308): the thin place's mark, from sleeping within its reach (`marked`).
    TrueDream,
}

impl ConditionKind {
    pub fn label(self) -> &'static str {
        match self {
            ConditionKind::Unrested => "unrested",
            ConditionKind::BeastHunger => "the Beast-hunger",
            ConditionKind::Thirst => "the Thirst",
            ConditionKind::StoneSleep => "the Stone-sleep",
            ConditionKind::TrueDream => "the true-dream",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Condition {
    pub kind: ConditionKind,
    pub since: u64,
    /// The row that gave it: a death, a curse, a bite, a breach.
    pub cause: EventId,
    pub from: Option<AgentId>,
    pub stage: u8,
    /// The last tick it acted (a sighting, a night out, a drink).
    #[serde(default)]
    pub last: u64,
    /// Where it was taken (the cavern of a Stone-sleep), or how they died (the restless).
    #[serde(default)]
    pub note: String,
}

impl Condition {
    pub fn new(kind: ConditionKind, since: u64, cause: EventId, from: Option<AgentId>) -> Self {
        Condition {
            kind,
            since,
            cause,
            from,
            stage: 0,
            last: since,
            note: String::new(),
        }
    }
}

/// Whether agent `i` holds a condition.
pub fn has(w: &World, i: usize, kind: ConditionKind) -> bool {
    w.agents[i].conditions.iter().any(|c| c.kind == kind)
}

/// How a condition changes the work of agent `i` (the Stone-sleep: strong at first, then
/// stiffening to nothing).
pub fn work_factor(w: &World, i: usize) -> f32 {
    let tick = w.meta.tick;
    w.agents[i]
        .conditions
        .iter()
        .filter(|c| c.kind == ConditionKind::StoneSleep)
        .map(|c| {
            let days = tick.saturating_sub(c.since) as f32 / TICKS_PER_DAY as f32;
            if days < 15.0 {
                1.2
            } else {
                (1.0 - (days - 15.0) / 25.0).clamp(0.1, 1.0)
            }
        })
        .product()
}

/// Gives agent `i` a condition, with its row.
pub fn take(w: &mut World, i: usize, kind: ConditionKind, because: &str, from: Option<AgentId>) {
    if has(w, i, kind) {
        return;
    }
    let tick = w.meta.tick;
    let (who, pos) = (w.agents[i].id, w.agents[i].pos);
    let eid = w.events.push(
        tick,
        Some(pos),
        Severity::Major,
        EventKind::Cursed {
            agent: who,
            condition: kind.label().to_string(),
            because: because.to_string(),
        },
    );
    w.agents[i]
        .conditions
        .push(Condition::new(kind, tick, eid, from));
}

/// At a death: one not laid to rest by a rite begins unrested, with their death's row.
pub fn unlaid(w: &mut World, i: usize, death: EventId) {
    let tick = w.meta.tick;
    let mut c = Condition::new(ConditionKind::Unrested, tick, death, None);
    c.note = w.agents[i].death.clone().unwrap_or_default();
    w.agents[i].conditions.push(c);
}

/// The deity who presses the strong, if it is this world's (deity.rs Motive::Test).
fn presses(w: &World) -> bool {
    crate::deity::of(w.meta.engine).motive == crate::deity::Motive::Test
}

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    let hour = (tick % TICKS_PER_DAY) / TICKS_PER_HOUR;
    let minute = tick % TICKS_PER_HOUR;
    if minute != 7 {
        return;
    }
    if hour == 6 {
        restless(w);
        stone(w);
        unbinding(w);
    }
    if hour == 19 {
        sightings(w);
    }
    if hour == 13 {
        starving(w);
        glare(w);
    }
    if hour == 1 {
        nights(w);
        marked(w);
        true_dreams(w);
    }
}

/// Ten days after a death with no rite, the dead are restless once nobody living remembers
/// them, unless their death is carved; a restless dead whose death is carved is at rest.
fn restless(w: &mut World) {
    let tick = w.meta.tick;
    for i in 0..w.agents.len() {
        let Some(k) = w.agents[i]
            .conditions
            .iter()
            .position(|c| c.kind == ConditionKind::Unrested)
        else {
            continue;
        };
        let c = w.agents[i].conditions[k].clone();
        let id = w.agents[i].id;
        let remembered = w.agents.iter().any(|a| {
            a.alive
                && a.memories
                    .long
                    .iter()
                    .any(|m| m.subject == crate::mind::Subject::Person(id))
        });
        let carved = w
            .engravings
            .values()
            .any(|e| e.craft.is_none() && e.event == c.cause);
        if c.stage == 0 {
            if tick.saturating_sub(c.since) < 10 * TICKS_PER_DAY {
                continue;
            }
            if carved {
                w.agents[i].conditions.remove(k);
                continue;
            }
            // Remembered, they wait: when the last who remembers them is gone or has
            // forgotten, they are restless.
            if remembered {
                continue;
            }
            w.agents[i].conditions[k].stage = 1;
            let at = w.agents[i].pos;
            w.events.push(
                tick,
                Some(at),
                Severity::Notable,
                EventKind::Restless { agent: id },
            );
        } else if carved {
            w.agents[i].conditions.remove(k);
            let at = w.agents[i].pos;
            w.events.push(
                tick,
                Some(at),
                Severity::Notable,
                EventKind::AtRest {
                    agent: id,
                    how: "their death is carved, and remembered".into(),
                },
            );
        }
    }
}

/// At dusk the restless are seen where they died, if that is near a community or within the thin
/// place's reach, by someone awake
/// within twelve tiles; one who sees and holds that the dead must be given their rite gives it.
fn sightings(w: &mut World) {
    let tick = w.meta.tick;
    let homes: Vec<(crate::ids::CommunityId, TilePos)> = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .map(|c| (c.id, c.home))
        .collect();
    for i in 0..w.agents.len() {
        let Some(k) = w.agents[i]
            .conditions
            .iter()
            .position(|c| c.kind == ConditionKind::Unrested && c.stage == 1)
        else {
            continue;
        };
        let c = w.agents[i].conditions[k].clone();
        if tick.saturating_sub(c.last) < 3 * TICKS_PER_DAY && c.last != c.since {
            continue;
        }
        let at = w.agents[i].pos;
        // Near a community's home, or within the thin place's reach, where the dead are near.
        let thin = crate::thin::stone(w).is_some_and(|s| {
            w.thin.woke.is_some() && s.z == at.z && s.dist(at) as f32 <= crate::thin::radius(w)
        });
        if !thin && !homes.iter().any(|(_, h)| h.z == at.z && h.dist(at) <= 20) {
            continue;
        }
        let Some(j) = (0..w.agents.len())
            .filter(|&j| {
                let a = &w.agents[j];
                a.alive
                    && a.species == crate::agents::Species::Person
                    && a.community.is_some()
                    && !a.activity.is_asleep()
                    && a.pos.z == at.z
                    && a.pos.dist(at) <= 12
            })
            .min_by_key(|&j| (w.agents[j].pos.dist(at), w.agents[j].id))
        else {
            continue;
        };
        let (of, by) = (w.agents[i].id, w.agents[j].id);
        w.agents[i].conditions[k].last = tick;
        let ev = w.events.push(
            tick,
            Some(at),
            Severity::Notable,
            EventKind::GhostSeen {
                of,
                by,
                cause: c.note.clone(),
            },
        );
        w.agents[j]
            .thoughts
            .add(crate::mind::ThoughtKind::GhostSeen, tick, Some(ev));
        if w.agents[j].beliefs.lean(Topic::Burial) >= 0.3 {
            if let Some(cid) = w.agents[j].community {
                // The dead are given the rite where they lie, by one who saw them.
                crate::systems::rites::lay_to_rest(w, of, cid, Some(w.agents[j].id), false);
                w.agents[i].conditions.remove(k);
                w.events.push(
                    tick,
                    Some(at),
                    Severity::Notable,
                    EventKind::AtRest {
                        agent: of,
                        how: format!("{} gave them their rite", w.agents[j].name.clone()),
                    },
                );
            }
        }
    }
}

/// The Stone-sleep stiffens a body over forty days, and then they die, standing, a statue.
fn stone(w: &mut World) {
    let tick = w.meta.tick;
    for i in 0..w.agents.len() {
        if !w.agents[i].alive {
            continue;
        }
        let Some(c) = w.agents[i]
            .conditions
            .iter()
            .find(|c| c.kind == ConditionKind::StoneSleep)
            .cloned()
        else {
            continue;
        };
        let days = tick.saturating_sub(c.since) / TICKS_PER_DAY;
        if days >= 15 {
            w.agents[i]
                .thoughts
                .add(crate::mind::ThoughtKind::Stiffening, tick, Some(c.cause));
        }
        if days < 40 {
            continue;
        }
        let (id, pos, name) = (w.agents[i].id, w.agents[i].pos, w.agents[i].name.clone());
        w.events.push(
            tick,
            Some(pos),
            Severity::Major,
            EventKind::TurnedToStone {
                agent: id,
                cavern: c.note.clone(),
            },
        );
        // The statue they became, its story the breach that began it.
        let item = w.create_lone_item(
            crate::items::ItemKind::Figurine,
            1,
            crate::items::ItemPlace::Ground(pos),
            0.5,
            "stone-sleep",
        );
        if let Some(it) = w.items.iter_mut().find(|it| it.id == item) {
            it.material = crate::world::Material::BASALT;
            it.story = Some(Box::new(crate::legend::Story {
                maker: None,
                maker_name: name.clone(),
                made: tick,
                wear: 1.0,
                name: Some(format!("the stone that was {name}")),
                origin: format!("{name}, who touched the underworld's rock in {}", c.note),
                ..Default::default()
            }));
        }
        crate::systems::life::die(
            w,
            id,
            format!(
                "turned to stone, since they touched the underworld's rock in {}",
                c.note
            ),
        );
        // Those who knew them hold that the deep is not to be dug, from the breach.
        for a in w
            .agents
            .iter_mut()
            .filter(|a| a.alive && a.relationships.contains_key(&id))
        {
            a.beliefs.experience(
                Topic::DeepDigging,
                -1.0,
                0.5,
                crate::mind::beliefs::Source::Event(c.cause),
                tick,
            );
        }
    }
}

/// Starving, someone who does not hold eating the dead abhorrent eats of an unburied body near;
/// those who hold it abhorrent and see it are troubled. If the dead was kin and their own
/// people hold it abhorrent, the deity who presses the strong curses them with the Thirst.
fn starving(w: &mut World) {
    let tick = w.meta.tick;
    for i in 0..w.agents.len() {
        let a = &w.agents[i];
        if !a.alive
            || a.species != crate::agents::Species::Person
            || a.needs.hunger > 0.05
            || a.starving_ticks < TICKS_PER_DAY as u32
            || a.beliefs.lean(Topic::EatDead) < -0.2
        {
            continue;
        }
        let me = a.pos;
        let Some(j) = (0..w.agents.len())
            .filter(|&j| {
                let d = &w.agents[j];
                !d.alive
                    && d.death.is_some()
                    && d.species == crate::agents::Species::Person
                    && d.conditions
                        .iter()
                        .any(|c| c.kind == ConditionKind::Unrested)
                    && d.pos.z == me.z
                    && d.pos.dist(me) <= 10
            })
            .min_by_key(|&j| (w.agents[j].pos.dist(me), w.agents[j].id))
        else {
            continue;
        };
        let (who, of) = (w.agents[i].id, w.agents[j].id);
        let kin = {
            let a = &w.agents[i];
            let d = &w.agents[j];
            a.mother == Some(of)
                || a.father == Some(of)
                || a.children.contains(&of)
                || a.partner == Some(of)
                || d.mother == a.mother && a.mother.is_some()
        };
        w.agents[i].needs.hunger = 0.6;
        w.agents[i].starving_ticks = 0;
        let ev = w.events.push(
            tick,
            Some(me),
            Severity::Major,
            EventKind::AteDead { agent: who, of },
        );
        for k in 0..w.agents.len() {
            let o = &w.agents[k];
            if k != i
                && o.alive
                && !o.activity.is_asleep()
                && o.pos.z == me.z
                && o.pos.dist(me) <= 10
                && o.beliefs.lean(Topic::EatDead) <= -0.4
            {
                w.agents[k]
                    .thoughts
                    .add(crate::mind::ThoughtKind::SawTheDeadEaten, tick, Some(ev));
                w.agents[k].relate(who, -0.4, 0.2, tick);
            }
        }
        let abhorrent = w.agents[i]
            .culture
            .and_then(|c| w.realm.cultures.get(c as usize))
            .is_some_and(|c| c.stance(Topic::EatDead) <= -0.4);
        if kin && abhorrent && presses(w) {
            let d = crate::deity::of(w.meta.engine).name;
            take(
                w,
                i,
                ConditionKind::Thirst,
                &format!("for eating of their own dead, which their people hold abhorrent, by {d}'s will"),
                None,
            );
        }
    }
}

/// The Thirst burns its own in the sun.
fn glare(w: &mut World) {
    let tick = w.meta.tick;
    for i in 0..w.agents.len() {
        if !w.agents[i].alive || !has(w, i, ConditionKind::Thirst) {
            continue;
        }
        let p = w.agents[i].pos;
        if crate::systems::hazards::indoors(w, p) || p.z < w.grid.ground_z(p.x, p.y) {
            continue;
        }
        let cause = w.agents[i]
            .conditions
            .iter()
            .find(|c| c.kind == ConditionKind::Thirst)
            .map(|c| c.cause);
        w.agents[i]
            .thoughts
            .add(crate::mind::ThoughtKind::Glare, tick, cause);
        crate::systems::health::wound(
            w,
            i,
            crate::body::Part::HEAD,
            crate::body::WoundKind::Burn,
            0.08,
            0.0,
            "burned by the sun",
            None,
        );
    }
}

/// The night's work of the curses, at one in the morning: the Beast-hunger at the full moon,
/// the Thirst every third night.
fn nights(w: &mut World) {
    let tick = w.meta.tick;
    let day = tick / TICKS_PER_DAY;
    let full = Calendar::full_moon(tick);
    for i in 0..w.agents.len() {
        if !w.agents[i].alive {
            continue;
        }
        let me = w.agents[i].pos;
        let who = w.agents[i].id;
        let mut r = stream(
            w.meta.seed,
            &[purpose::AGENT_WORK, who.0 as u64, day, 0xC125E],
        );
        if full && has(w, i, ConditionKind::BeastHunger) {
            let k = w.agents[i]
                .conditions
                .iter()
                .position(|c| c.kind == ConditionKind::BeastHunger)
                .unwrap();
            w.agents[i].conditions[k].stage = (w.agents[i].conditions[k].stage + 1).min(3);
            w.agents[i].conditions[k].last = tick;
            let gone = w.events.push(
                tick,
                Some(me),
                Severity::Notable,
                EventKind::Missing { agent: who },
            );
            // C75: from the second full moon on, they go out in a beast's shape until dawn and
            // hunt whoever is out alone (systems/combat.rs, `shifted`).
            if w.agents[i].conditions[k].stage >= 2 {
                let dawn = tick - tick % TICKS_PER_DAY + 6 * crate::time::TICKS_PER_HOUR;
                w.creatures.shifted.retain(|x| x.0 != who);
                w.creatures.shifted.push((who, dawn));
                w.events.push(
                    tick,
                    Some(me),
                    Severity::Major,
                    EventKind::Shifted { agent: who },
                );
            }
            // A beast of the valley killed in the dark, near.
            if let Some(prey) = w
                .agents
                .iter()
                .filter(|a| {
                    a.alive
                        && matches!(
                            a.species,
                            crate::agents::Species::Deer
                                | crate::agents::Species::Hare
                                | crate::agents::Species::Goat
                        )
                        && a.pos.z == me.z
                        && a.pos.dist(me) <= 30
                })
                .min_by_key(|a| (a.pos.dist(me), a.id))
                .map(|a| a.id)
            {
                crate::systems::life::die(w, prey, "torn apart in the night".into());
            }
            bite(w, i, me, ConditionKind::BeastHunger, &mut r);
            suspect(w, i, gone, "they were gone on the night of the full moon");
        }
        if has(w, i, ConditionKind::Thirst) {
            let k = w.agents[i]
                .conditions
                .iter()
                .position(|c| c.kind == ConditionKind::Thirst)
                .unwrap();
            // No hunger for food.
            w.agents[i].needs.hunger = 1.0;
            if tick.saturating_sub(w.agents[i].conditions[k].last) >= 3 * TICKS_PER_DAY {
                if let Some(ev) = bite(w, i, me, ConditionKind::Thirst, &mut r) {
                    w.agents[i].conditions[k].last = tick;
                    suspect(w, i, ev, "a sleeper woke weak, with two small wounds");
                }
            }
        }
    }
}

/// The thin place marks one who sleeps within its reach while it stands woke: one night in
/// four, the hollow keeps them. The cursing is recorded with its row, as every condition is.
fn marked(w: &mut World) {
    let tick = w.meta.tick;
    if w.thin.woke.is_none() {
        return;
    }
    for i in 0..w.agents.len() {
        let a = &w.agents[i];
        if !a.alive
            || a.species != crate::agents::Species::Person
            || !a.activity.is_asleep()
            || has(w, i, ConditionKind::TrueDream)
            || !crate::thin::hollow(w, a.pos)
        {
            continue;
        }
        let who = a.id;
        let mut r = stream(
            w.meta.seed,
            &[purpose::THIN, who.0 as u64, tick / TICKS_PER_DAY, 0x7D1E],
        );
        if r.gen_bool(0.25) {
            take(
                w,
                i,
                ConditionKind::TrueDream,
                "for sleeping within the thin place's reach",
                None,
            );
        }
    }
}

/// The true-dream, every third night: the marked one cries out in their sleep, of something
/// true to the record — the nearest of the restless dead, else the stone — and their
/// community comes to suspect them of dealing with the hollow. The suspicion holds the cry
/// for its source row, and it is wrong: the mark is the thin place's, and they are its
/// victim, not its agent (FIXLOG 2026-11-01 #308).
fn true_dreams(w: &mut World) {
    let tick = w.meta.tick;
    for i in 0..w.agents.len() {
        if !w.agents[i].alive || !has(w, i, ConditionKind::TrueDream) {
            continue;
        }
        let k = w.agents[i]
            .conditions
            .iter()
            .position(|c| c.kind == ConditionKind::TrueDream)
            .unwrap();
        if tick.saturating_sub(w.agents[i].conditions[k].last) < 3 * TICKS_PER_DAY {
            continue;
        }
        w.agents[i].conditions[k].last = tick;
        let (who, me) = (w.agents[i].id, w.agents[i].pos);
        // What they cry of: the nearest of the restless dead, if any lie near; else the stone.
        let dead = (0..w.agents.len())
            .filter(|&j| {
                !w.agents[j].alive
                    && w.agents[j]
                        .conditions
                        .iter()
                        .any(|c| c.kind == ConditionKind::Unrested)
                    && w.agents[j].pos.z == me.z
            })
            .min_by_key(|&j| (w.agents[j].pos.dist(me), w.agents[j].id));
        let what = match dead {
            Some(j) => format!("{} walking, unrested", w.agents[j].name.clone()),
            None => "the stone that should not be".to_string(),
        };
        // They wake tired; the cry is recorded with their curse's row.
        w.agents[i].needs.rest = (w.agents[i].needs.rest - 0.3).max(0.0);
        let cause = w.agents[i].conditions[k].cause;
        let ev = w.events.push(
            tick,
            Some(me),
            Severity::Notable,
            EventKind::TrueDream {
                agent: who,
                what,
                cause,
            },
        );
        suspect(
            w,
            i,
            ev,
            "they cry out in their sleep of things no one told them",
        );
    }
}

/// Agent `i` bites the nearest sleeper within twenty tiles: blood lost, a bite, and three in
/// ten times the curse with it. The row, if there was a bite.
fn bite(
    w: &mut World,
    i: usize,
    me: TilePos,
    kind: ConditionKind,
    r: &mut impl Rng,
) -> Option<EventId> {
    let tick = w.meta.tick;
    let by = w.agents[i].id;
    let j = (0..w.agents.len())
        .filter(|&j| {
            let a = &w.agents[j];
            j != i
                && a.alive
                && a.species == crate::agents::Species::Person
                && a.activity.is_asleep()
                && a.pos.z == me.z
                && a.pos.dist(me) <= 20
                && (kind == ConditionKind::Thirst || !crate::systems::hazards::indoors(w, a.pos))
        })
        .min_by_key(|&j| (w.agents[j].pos.dist(me), w.agents[j].id))?;
    let of = w.agents[j].id;
    let at = w.agents[j].pos;
    let ev = w.events.push(
        tick,
        Some(at),
        Severity::Major,
        EventKind::Bitten { by, of },
    );
    crate::systems::health::wound(
        w,
        j,
        crate::body::Part::TORSO,
        crate::body::WoundKind::Puncture,
        0.2,
        0.05,
        "bitten in the night",
        Some(by),
    );
    w.agents[j].body.blood = (w.agents[j].body.blood - 0.25).max(0.2);
    if r.gen_bool(0.3) && !has(w, j, kind) {
        let mut c = Condition::new(kind, tick, ev, Some(by));
        c.last = tick;
        w.agents[j].conditions.push(c);
    }
    Some(ev)
}

/// Those of the community who noticed come to suspect agent `i`: the one who saw or was hurt
/// first, then whoever slept nearest. A suspicion is a memory of them with the row as its
/// source, and a cooler opinion; it can be wrong.
fn suspect(w: &mut World, i: usize, ev: EventId, because: &str) {
    let tick = w.meta.tick;
    let (of, me, cid) = (w.agents[i].id, w.agents[i].pos, w.agents[i].community);
    let Some(cid) = cid else { return };
    let Some(j) = (0..w.agents.len())
        .filter(|&j| {
            let a = &w.agents[j];
            j != i && a.alive && a.community == Some(cid) && a.pos.z == me.z && a.pos.dist(me) <= 20
        })
        .min_by_key(|&j| (w.agents[j].pos.dist(me), w.agents[j].id))
    else {
        return;
    };
    let by = w.agents[j].id;
    w.agents[j].remember(
        tick,
        crate::mind::MemKind::Hardship,
        crate::mind::Subject::Person(of),
        -0.4,
        0.5,
        Some(ev),
    );
    w.agents[j].relate(of, -0.2, 0.1, tick);
    w.events.push(
        tick,
        Some(me),
        Severity::Notable,
        EventKind::Suspected {
            by,
            of,
            because: because.to_string(),
        },
    );
}

/// A healer who knows the unbinding lifts the Beast-hunger or the Thirst from one of their
/// community they have come to suspect.
fn unbinding(w: &mut World) {
    let tick = w.meta.tick;
    for i in 0..w.agents.len() {
        let a = &w.agents[i];
        if !a.alive || !a.techniques.knows(crate::mind::Technique::Unbinding) {
            continue;
        }
        let (by, cid) = (a.id, a.community);
        let Some(cid) = cid else { continue };
        let target = (0..w.agents.len()).find_map(|j| {
            let t = &w.agents[j];
            if !t.alive || t.community != Some(cid) || j == i {
                return None;
            }
            let k = t.conditions.iter().position(|c| {
                matches!(
                    c.kind,
                    ConditionKind::BeastHunger | ConditionKind::Thirst | ConditionKind::TrueDream
                )
            })?;
            let suspected = w.agents[i]
                .memories
                .short
                .iter()
                .any(|m| m.subject == crate::mind::Subject::Person(t.id) && m.valence < 0.0);
            suspected.then_some((j, k))
        });
        let Some((j, k)) = target else { continue };
        let kind = w.agents[j].conditions.remove(k).kind;
        let (agent, at) = (w.agents[j].id, w.agents[j].pos);
        w.events.push(
            tick,
            Some(at),
            Severity::Major,
            EventKind::Unbound {
                agent,
                by,
                condition: kind.label().to_string(),
            },
        );
    }
}
