//! Faith (#297, epic #376): what people make of the deity and the signs, each in their own way.
//!
//! The deity's answers are hidden from the feed (deity.rs). The one a community follows in the
//! spirit (justice.rs) reads them aloud in words that say what the deity seems to want, so its
//! mood is told by a person, not shown as a meter. Those who hear a reading or an omen
//! (omens.rs) take it as they are inclined: most heed it, held as the reader holds it; one who
//! thinks signs are only weather says so to the reader's face, and the two think less of each
//! other. A person's leaning is their own (`inclination`), so a community is not of one mind.
//! FIXLOG 2026-10-26 "the deity answered unread".

use crate::deity::{Lever, Motive, Trigger};
use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, CommunityId, EventId};
use crate::justice::Domain;
use crate::mind::beliefs::{Source, Topic};
use crate::mind::feelings::{gain, FeelingKind};
use crate::World;
use serde::{Deserialize, Serialize};

/// How far a reading carries: those of the community within this many tiles of the reader.
pub const HEARD_WITHIN: i32 = 15;

/// Faith's own state, kept with the world.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Faith {
    /// Readings and omens still waiting to be borne out or belied.
    #[serde(default)]
    pub signs: Vec<SignRead>,
    /// Communities that kept the last observance, and when (observances.rs).
    #[serde(default)]
    pub kept: Vec<(CommunityId, u64)>,
    /// Each community's shrine, once built (shrines.rs).
    #[serde(default)]
    pub shrines: Vec<crate::shrines::ShrineSite>,
    /// The last shrine each community lost: the event of its loss, and why.
    #[serde(default)]
    pub lost: Vec<(CommunityId, EventId, String)>,
    /// Each community's keeper of the shrine and the observances (keepers.rs).
    #[serde(default)]
    pub keepers: Vec<crate::keepers::Keeper>,
    /// Signs each reader has had borne out and belied: (reader, borne, belied).
    #[serde(default)]
    pub record: Vec<(AgentId, u32, u32)>,
    /// Bands on the road a warning has already been read for (omens.rs `foretell`).
    #[serde(default)]
    pub foretold: Vec<u32>,
    /// Relics of the revered dead (relics.rs).
    #[serde(default)]
    pub relics: Vec<crate::relics::Relic>,
    /// People on their way to a shrine or a relic in hard times (pilgrimage.rs).
    #[serde(default)]
    pub pilgrims: Vec<crate::pilgrimage::Pilgrim>,
    /// Those who hold the signs apart from their keeper (heresy.rs).
    #[serde(default)]
    pub factions: Vec<crate::heresy::Faction>,
}

impl Faith {
    /// (borne out, belied) of the signs `reader` has read.
    pub fn record_of(&self, reader: AgentId) -> (u32, u32) {
        self.record
            .iter()
            .find(|r| r.0 == reader)
            .map(|r| (r.1, r.2))
            .unwrap_or((0, 0))
    }
}

/// Who reads signs to a community: its keeper (keepers.rs), else the one it follows in the
/// spirit (justice.rs).
pub fn reader_of(w: &World, cid: CommunityId) -> Option<AgentId> {
    crate::keepers::keeper_of(w, cid).or_else(|| {
        w.communities
            .iter()
            .find(|c| c.id == cid && !c.dissolved)
            .and_then(|c| c.leaders[Domain::Spirit as usize])
    })
}

/// A sign read to a community, open until `until`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SignRead {
    pub reader: AgentId,
    pub community: CommunityId,
    /// A blessing (true) or a warning.
    pub good: bool,
    /// What was read, as the record tells it.
    pub of: String,
    /// The Omen or DeityRead event.
    pub event: EventId,
    pub until: u64,
}

/// A person's own leaning toward signs, -1 (they are only weather) to 1, drawn once for them.
/// It shapes how they take the first sign they hear, before they hold any belief about it.
pub fn inclination(w: &World, id: AgentId) -> f32 {
    use rand::Rng;
    let mut r = crate::rng::stream(
        w.meta.seed,
        &[crate::rng::purpose::AGENT_SOCIAL, 0x297, id.0 as u64],
    );
    r.gen_range(-1.0..1.0)
}

/// Is this answer a kindness to the valley, or a hardness?
fn kind_answer(lever: Lever, strength: f32) -> bool {
    match lever {
        Lever::Cold | Lever::DryStorms | Lever::WolfHunger => false,
        Lever::Sickness => strength < 1.0,
        Lever::Harvest | Lever::Herds => strength > 1.0,
    }
}

/// What the reader says the deity wants, by the deity's motive and what it answered.
fn words(name: &str, motive: Motive, trigger: Trigger, good: bool) -> String {
    let s = match (motive, trigger, good) {
        (Motive::Balance, Trigger::Hardship, _) => "has seen our hard days and is easing them",
        (Motive::Balance, Trigger::Plenty, _) => "thinks we have more than our share",
        (Motive::Balance, Trigger::Killing, _) => "has turned cold to us for the killing",
        (Motive::Test, Trigger::Plenty, _) => "has seen how well we do and means to test us",
        (Motive::Test, Trigger::Hardship, _) => "has let us be, now that we are low",
        (Motive::Test, Trigger::Killing, _) => "has cursed the killing and those who allow it",
        (Motive::Amusement, _, true) => "is playing with us, and kindly this time",
        (Motive::Amusement, _, false) => "is bored with us and means to stir things",
        (_, _, true) => "is pleased with us",
        (_, _, false) => "is displeased with us",
    };
    format!("{name} {s}")
}

fn idx(w: &World, id: AgentId) -> Option<usize> {
    w.agents.iter().position(|a| a.id == id && a.alive)
}

/// After the flush: the deity's answers over the valley are read aloud; readings and omens are
/// heard, heeded or scoffed at.
pub fn observe(w: &mut World) {
    let before = leans(w);
    settle_signs(w);
    read_answers(w);
    hear_signs(w);
    turned(w, &before);
}

/// Each living person's lean on signs, by index.
fn leans(w: &World) -> Vec<(usize, f32)> {
    (0..w.agents.len())
        .filter(|&k| w.agents[k].alive && w.agents[k].species == crate::agents::Species::Person)
        .map(|k| (k, w.agents[k].beliefs.lean(Topic::DeitySigns)))
        .collect()
}

/// A death of one of a community's own bears out a warning read to them and belies a
/// blessing; a sign whose five days pass quietly leaves the opposite, more faintly. Each sign
/// is settled once. FIXLOG 2026-10-26 "a sign was never borne out".
fn settle_signs(w: &mut World) {
    let tick = w.meta.tick;
    let deaths: Vec<(CommunityId, AgentId)> = w
        .last_events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Died { agent, .. } => w
                .agents
                .iter()
                .find(|a| a.id == *agent)
                .and_then(|a| a.community)
                .map(|c| (c, *agent)),
            _ => None,
        })
        .collect();
    for (cid, dead) in deaths {
        while let Some(k) = w
            .faith
            .signs
            .iter()
            .position(|s| s.community == cid && s.reader != dead)
        {
            let s = w.faith.signs.remove(k);
            settle(w, &s, Some(dead));
        }
    }
    let (done, open): (Vec<SignRead>, Vec<SignRead>) = std::mem::take(&mut w.faith.signs)
        .into_iter()
        .partition(|s| s.until <= tick);
    w.faith.signs = open;
    for s in done {
        settle(w, &s, None);
    }
}

fn settle(w: &mut World, s: &SignRead, dead: Option<AgentId>) {
    let tick = w.meta.tick;
    // Toward heeding when a warning is borne out or a blessing kept.
    let heed = s.good == dead.is_none();
    let force = match (dead.is_some(), s.good) {
        (true, true) => 0.25,
        (true, false) => 0.2,
        (false, _) => 0.06,
    };
    let members: Vec<usize> = (0..w.agents.len())
        .filter(|&k| {
            let a = &w.agents[k];
            a.alive
                && a.species == crate::agents::Species::Person
                && a.community == Some(s.community)
        })
        .collect();
    for &k in &members {
        w.agents[k].beliefs.experience(
            Topic::DeitySigns,
            if heed { 1.0 } else { -1.0 },
            force,
            Source::Event(s.event),
            tick,
        );
    }
    let Some(dead) = dead else { return };
    // Kept against the reader's name: a keeper's claim rests on it (keepers.rs).
    match w.faith.record.iter_mut().find(|r| r.0 == s.reader) {
        Some(r) if heed => r.1 += 1,
        Some(r) => r.2 += 1,
        None => w
            .faith
            .record
            .push((s.reader, u32::from(heed), u32::from(!heed))),
    }
    // The reader is followed the more for a warning borne out, the less for a blessing belied.
    for &k in &members {
        if w.agents[k].id == s.reader {
            continue;
        }
        if heed {
            crate::justice::credit(w, k, s.reader, Domain::Spirit, 0.2);
        } else {
            crate::justice::discredit(w, k, s.reader, Domain::Spirit, 0.3);
        }
    }
    let at = idx(w, s.reader).map(|i| w.agents[i].pos);
    w.events.push(
        tick,
        at,
        Severity::Notable,
        EventKind::SignSettled {
            reader: s.reader,
            of: s.of.clone(),
            good: s.good,
            dead,
        },
    );
}

/// Anyone whose lean on signs crossed from one side to the other, from a view held to its
/// opposite, has turned; it is on record.
fn turned(w: &mut World, before: &[(usize, f32)]) {
    let tick = w.meta.tick;
    for &(k, b) in before {
        let a = w.agents[k].beliefs.lean(Topic::DeitySigns);
        let heeds = if b <= -0.08 && a >= 0.08 {
            true
        } else if b >= 0.08 && a <= -0.08 {
            false
        } else {
            continue;
        };
        let (agent, at) = (w.agents[k].id, w.agents[k].pos);
        w.events.push(
            tick,
            Some(at),
            Severity::Notable,
            EventKind::FaithTurned { agent, heeds },
        );
    }
}

/// Each answer the deity made over the valley this tick is read by each community's one
/// followed in the spirit, once per trigger.
fn read_answers(w: &mut World) {
    let tick = w.meta.tick;
    let ids: Vec<EventId> = w
        .last_events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::DeityAnswered { .. }))
        .map(|e| e.id)
        .collect();
    if ids.is_empty() {
        return;
    }
    let deity = crate::deity::of(w.meta.engine);
    let mut said: Vec<Trigger> = Vec::new();
    for id in ids {
        let Some(a) = w
            .deity_answers
            .iter()
            .find(|a| a.event == id && a.scope == crate::deity::Scope::Valley)
            .cloned()
        else {
            continue;
        };
        if said.contains(&a.trigger) {
            continue;
        }
        said.push(a.trigger);
        let good = kind_answer(a.lever, a.strength);
        let text = words(deity.name, deity.motive, a.trigger, good);
        let cids: Vec<CommunityId> = w
            .communities
            .iter()
            .filter(|c| !c.dissolved)
            .map(|c| c.id)
            .collect();
        let readers: Vec<AgentId> = cids.into_iter().filter_map(|c| reader_of(w, c)).collect();
        for reader in readers {
            let Some(ri) = idx(w, reader) else { continue };
            let at = w.agents[ri].pos;
            w.events.push(
                tick,
                Some(at),
                Severity::Notable,
                EventKind::DeityRead {
                    reader,
                    deity: deity.name.to_string(),
                    words: text.clone(),
                    good,
                },
            );
        }
    }
}

/// Readings and omens of the last tick, heard by the reader's community nearby.
fn hear_signs(w: &mut World) {
    let tick = w.meta.tick;
    let signs: Vec<(EventId, AgentId, bool, String, bool)> = w
        .last_events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Omen { reader, of, good } => Some((e.id, *reader, *good, of.clone(), false)),
            EventKind::DeityRead {
                reader,
                words,
                good,
                ..
            } => Some((e.id, *reader, *good, words.clone(), true)),
            _ => None,
        })
        .collect();
    for (ev, reader, good, of, answer) in signs {
        let Some(ri) = idx(w, reader) else { continue };
        let Some(cid) = w.agents[ri].community else {
            continue;
        };
        let rp = w.agents[ri].pos;
        let hearers: Vec<usize> = (0..w.agents.len())
            .filter(|&k| {
                let a = &w.agents[k];
                k != ri
                    && a.alive
                    && a.species == crate::agents::Species::Person
                    && a.community == Some(cid)
                    && a.pos.z == rp.z
                    && a.pos.dist(rp) <= HEARD_WITHIN
                    && !a.activity.is_asleep()
            })
            .collect();
        // The reader holds that signs are to be heeded, at least as much as they read them.
        let rs = w.agents[ri].beliefs.lean(Topic::DeitySigns).max(0.6);
        let mut doubters: Vec<(f32, AgentId, usize)> = Vec::new();
        for k in hearers {
            let id = w.agents[k].id;
            if w.agents[k].beliefs.get(Topic::DeitySigns).is_none() && inclination(w, id) < -0.4 {
                // Their first sign, and they were never the kind to take one.
                w.agents[k]
                    .beliefs
                    .hold(Topic::DeitySigns, -0.4, 0.3, Source::Event(ev), tick);
            }
            let lean = w.agents[k].beliefs.lean(Topic::DeitySigns);
            if lean <= -0.1 {
                doubters.push((lean, id, k));
                continue;
            }
            let trust = (0.5 + w.agents[k].opinion_of(reader)).clamp(0.0, 1.0);
            w.agents[k]
                .beliefs
                .hear(Topic::DeitySigns, rs, reader, trust, tick);
            if answer {
                // An omen is already felt (omens.rs); a reading of the deity is felt here.
                let kind = if good {
                    FeelingKind::Joy
                } else {
                    FeelingKind::Fear
                };
                let g = gain(&w.agents[k], kind);
                w.agents[k].feelings.feel(kind, None, 0.3 * g, tick, None);
            }
        }
        // The one most sure it is only weather says so; the two think the less of each other.
        doubters.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        if let Some(&(_, by, k)) = doubters.first() {
            w.agents[k]
                .beliefs
                .experience(Topic::DeitySigns, -1.0, 0.1, Source::Event(ev), tick);
            w.agents[k].relate(reader, -0.1, 0.02, tick);
            w.agents[ri].relate(by, -0.1, 0.02, tick);
            let at = w.agents[k].pos;
            w.events.push(
                tick,
                Some(at),
                Severity::Notable,
                EventKind::Scoffed { by, at: reader },
            );
        }
        w.faith.signs.push(SignRead {
            reader,
            community: cid,
            good,
            of,
            event: ev,
            until: tick + SIGN_OPEN,
        });
    }
}

/// How long a sign waits to be borne out or belied: five days, a season (docs/TIME.md).
pub const SIGN_OPEN: u64 = 5 * crate::time::TICKS_PER_DAY;
