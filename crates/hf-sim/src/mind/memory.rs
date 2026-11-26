//! Memory. Short-term: a bounded ring of compact event records gathered while awake.
//! Long-term: bounded, decaying memories with a valence and a strength, consolidated from
//! the short-term ring at the end of a sleep. Memories bias choices (see `systems::decide`)
//! and move relationships; details fade (strength decays) while attitudes persist (valence
//! is kept as an average).
use crate::fmath::Det;

use serde::{Deserialize, Serialize};

use crate::ids::{AgentId, EventId, TilePos};
use crate::items::ItemKind;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum MemKind {
    /// Eating: valence from meal quality and hunger relief.
    Meal,
    /// A conversation or shared work with someone.
    Company,
    /// Sleeping somewhere (bed vs ground).
    Rest,
    /// Work finished (mastery, pride) or abandoned (frustration).
    Work,
    /// Going hungry or collapsing.
    Hardship,
    /// Finding something new (a cave, ore, a way down).
    Discovery,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Subject {
    Person(AgentId),
    Place(TilePos),
    Food(ItemKind),
    None,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Record {
    pub tick: u64,
    pub kind: MemKind,
    pub subject: Subject,
    /// -1 bad .. +1 good
    pub valence: f32,
    /// 0..1 how much it mattered at the time
    pub salience: f32,
    pub event: Option<EventId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Memory {
    pub kind: MemKind,
    pub subject: Subject,
    pub valence: f32,
    pub strength: f32,
    pub formed: u64,
    pub reinforced: u32,
    /// The tick it was last created or reinforced (#815): 0 on a memory saved before this
    /// field existed, read as `formed` by anything that weighs how recently it was renewed.
    #[serde(default)]
    pub last: u64,
    /// Chronicle-level provenance: the events this memory came from (last few).
    pub origins: Vec<EventId>,
}

/// (total weight, valence × weight sum, origin events)
type Group = (f32, f32, Vec<EventId>);

pub const SHORT_CAP: usize = 64;
pub const LONG_CAP: usize = 48;
pub const CONSOLIDATE_TOP: usize = 6;
pub const DECAY_PER_SLEEP: f32 = 0.97;
pub const FORGET_BELOW: f32 = 0.05;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Memories {
    pub short: Vec<Record>,
    pub long: Vec<Memory>,
}

impl Memories {
    pub fn record(&mut self, r: Record) {
        if self.short.len() >= SHORT_CAP {
            // Drop the least salient old record rather than the newest.
            if let Some((i, _)) = self.short.iter().enumerate().min_by(|a, b| {
                a.1.salience
                    .partial_cmp(&b.1.salience)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }) {
                self.short.remove(i);
            }
        }
        self.short.push(r);
    }

    /// Consolidate at the end of a sleep. `anxious` strengthens negatives, `curious`
    /// strengthens discoveries. Returns the number of memories touched.
    pub fn consolidate(&mut self, now: u64, anxious: bool, curious: bool) -> u32 {
        use std::collections::BTreeMap;
        let mut groups: BTreeMap<(MemKind, Subject), Group> = BTreeMap::new();
        // Vivid moments (a death, a fight, a break) are kept whatever else filled the day:
        // a single loss must not be outvoted by many ordinary meals.
        let vivid: std::collections::BTreeSet<(MemKind, Subject)> = self
            .short
            .iter()
            .filter(|r| r.salience >= 0.9 && r.valence.abs() >= 0.7)
            .map(|r| (r.kind, r.subject))
            .collect();
        for r in &self.short {
            let age = now.saturating_sub(r.tick) as f32 / crate::time::TICKS_PER_DAY as f32;
            let recency = (1.0 / (1.0 + age)).sqrt();
            let mut weight = r.salience * recency;
            if anxious && r.valence < 0.0 {
                weight *= 1.5;
            }
            if curious && r.kind == MemKind::Discovery {
                weight *= 1.5;
            }
            let e = groups
                .entry((r.kind, r.subject))
                .or_insert((0.0, 0.0, Vec::new()));
            e.0 += weight;
            e.1 += r.valence * weight;
            if let Some(id) = r.event {
                if e.2.len() < 4 {
                    e.2.push(id);
                }
            }
        }
        let mut ranked: Vec<((MemKind, Subject), Group)> = groups.into_iter().collect();
        ranked.sort_by(|a, b| {
            b.1 .0
                .partial_cmp(&a.1 .0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut touched = 0;
        let chosen: Vec<((MemKind, Subject), Group)> = ranked
            .iter()
            .enumerate()
            .filter(|(k, (key, _))| *k < CONSOLIDATE_TOP || vivid.contains(key))
            .map(|(_, g)| g.clone())
            .collect();
        for ((kind, subject), (w, vsum, origins)) in chosen {
            if w < 0.05 {
                continue;
            }
            let v = (vsum / w).clamp(-1.0, 1.0);
            let strength = (w * 0.5).min(1.0);
            if let Some(m) = self
                .long
                .iter_mut()
                .find(|m| m.kind == kind && m.subject == subject)
            {
                m.strength = (m.strength + strength * 0.6).min(1.0);
                m.valence = m.valence * 0.7 + v * 0.3;
                m.reinforced += 1;
                m.last = now;
                for o in origins {
                    if m.origins.len() < 6 && !m.origins.contains(&o) {
                        m.origins.push(o);
                    }
                }
            } else {
                self.long.push(Memory {
                    kind,
                    subject,
                    valence: v,
                    strength,
                    formed: now,
                    reinforced: 0,
                    last: now,
                    origins,
                });
            }
            touched += 1;
        }
        self.short.clear();
        for m in self.long.iter_mut() {
            m.strength *= DECAY_PER_SLEEP;
        }
        self.long.retain(|m| m.strength >= FORGET_BELOW);
        if self.long.len() > LONG_CAP {
            self.long.sort_by(|a, b| {
                b.strength
                    .partial_cmp(&a.strength)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            self.long.truncate(LONG_CAP);
        }
        touched
    }

    /// Multiplier for a choice involving `kind`/`subject`: product of (1 + valence × strength)
    /// over matching memories, clamped to [0.2, 3.0].
    pub fn bias(&self, kind: MemKind, subject: Subject) -> f32 {
        let mut b = 1.0;
        for m in &self.long {
            if m.kind == kind && m.subject == subject {
                b *= 1.0 + m.valence * m.strength * 1.5;
            }
        }
        b.clamp(0.2, 3.0)
    }

    /// Net attitude toward a person from company memories (-1..1), weighted by strength.
    pub fn attitude(&self, who: AgentId) -> f32 {
        let mut num = 0.0;
        let mut den = 0.0;
        for m in &self.long {
            if m.subject == Subject::Person(who) {
                num += m.valence * m.strength;
                den += m.strength;
            }
        }
        if den > 0.0 {
            num / den
        } else {
            0.0
        }
    }

    /// Feeling about a place (-1..1), from any memory kind at that tile.
    pub fn place_feeling(&self, p: TilePos) -> f32 {
        let mut v = 0.0;
        for m in &self.long {
            if m.subject == Subject::Place(p) {
                v += m.valence * m.strength;
            }
        }
        v.clamp(-1.0, 1.0)
    }
}

/// A bad memory loses half its grip over about this many ticks (ten days).
pub const TRAUMA_HALF_LIFE: f64 = 288_000.0;
/// OST-202: the most the long memories can add to or take from mood.
pub const MOOD_LONG_SCALE: f32 = 0.35;
/// Ticks for a routine memory (meals, rest, company) to count half as much in mood: 10 days.
pub const ROUTINE_HALF_LIFE: f64 = 288_000.0;
/// Ticks for a vivid memory (hardship, work, discovery) to count half as much: 30 days.
pub const VIVID_HALF_LIFE: f64 = 864_000.0;
/// A routine memory renewed this many times counts half as much as a fresh one (habituation).
pub const HABIT_AFTER: f32 = 8.0;
/// How far the shadow of a bad place reaches, in tiles.
pub const AVERSION_RADIUS: i32 = 6;

impl Memories {
    /// Fear of being at (or near) `p`, 0..1. This is the part of memory that says *do not go
    /// back*: negative memories of that ground, strongest where it happened, fading outwards
    /// and forwards. Good memories of the same ground count against it, so a place they have
    /// since been safe in stops being frightening. Someone with no bad history here reads 0.
    pub fn aversion(&self, p: TilePos, now: u64, cautious: bool) -> f32 {
        let radius = if cautious {
            AVERSION_RADIUS * 2
        } else {
            AVERSION_RADIUS
        };
        let mut fear = 0.0f32;
        let mut calm = 0.0f32;
        for m in &self.long {
            let Subject::Place(q) = m.subject else {
                continue;
            };
            let d = q.dist(p);
            if d > radius {
                continue;
            }
            let spatial = 1.0 / (1.0 + 2.0 * d as f32 / radius as f32);
            let age = now.saturating_sub(m.formed) as f64;
            let recency = 0.5f64.d_powf(age / TRAUMA_HALF_LIFE) as f32;
            let weight = m.strength.clamp(0.0, 1.0) * spatial * recency;
            if m.valence < 0.0 {
                fear = fear.max((-m.valence).clamp(0.0, 1.0) * weight);
            } else {
                calm += m.valence.clamp(0.0, 1.0) * weight;
            }
        }
        if fear <= 0.0 {
            return 0.0;
        }
        // Fear is what the worst thing there can still do; good times since then answer it.
        let still = fear * if cautious { 1.25 } else { 1.0 } - calm * 0.8;
        still.clamp(0.0, 1.0)
    }

    /// What the long memories add to mood (OST-202): bounded by `MOOD_LONG_SCALE` whatever the
    /// store size, each memory weighed by how recently it was renewed and, for routine kinds
    /// (meals, rest, company), by how often, so a vivid loss shows among many meals and a meal
    /// eaten daily for years does not. Not called by mood yet; `bias` and `aversion` read memory
    /// differently and are untouched.
    pub fn mood_term(&self, now: u64) -> f32 {
        let mut sum = 0.0f64;
        for m in &self.long {
            let routine = matches!(m.kind, MemKind::Meal | MemKind::Rest | MemKind::Company);
            let half = if routine {
                ROUTINE_HALF_LIFE
            } else {
                VIVID_HALF_LIFE
            };
            let age = now.saturating_sub(m.last.max(m.formed)) as f64;
            let recency = 0.5f64.d_powf(age / half);
            let habit = if routine {
                1.0 / (1.0 + m.reinforced as f64 / HABIT_AFTER as f64)
            } else {
                1.0
            };
            sum += m.valence as f64 * m.strength as f64 * recency * habit;
        }
        // tanh(x) = (e^2x - 1) / (e^2x + 1), from the deterministic exp.
        let e = (2.0 * sum / 4.0).clamp(-20.0, 20.0).d_exp();
        MOOD_LONG_SCALE * ((e - 1.0) / (e + 1.0)) as f32
    }

    /// Wariness of a person, 0..1: they have been hurt by this person and not since reconciled.
    pub fn aversion_of(&self, who: AgentId) -> f32 {
        let mut fear = 0.0f32;
        let mut calm = 0.0f32;
        for m in &self.long {
            if m.subject != Subject::Person(who) {
                continue;
            }
            let weight = m.strength.clamp(0.0, 1.0);
            if m.valence < 0.0 {
                fear = fear.max((-m.valence).clamp(0.0, 1.0) * weight);
            } else {
                calm += m.valence.clamp(0.0, 1.0) * weight;
            }
        }
        if fear <= 0.0 {
            return 0.0;
        }
        (fear - calm * 0.8).clamp(0.0, 1.0)
    }
}

impl crate::agents::Agent {
    /// What the scorer multiplies a place by: good memories draw them there, trauma pushes them
    /// away, and temperament decides which wins. `Cautious` gives the bad ground a wider berth;
    /// `Curious` goes back anyway to see what it is.
    pub fn place_multiplier(&self, p: TilePos, now: u64) -> f32 {
        let cautious = self.traits.has(crate::mind::Trait::Cautious);
        let curious = self.traits.has(crate::mind::Trait::Curious);
        let fear = self.memories.aversion(p, now, cautious);
        let feel = (1.0 + self.memories.place_feeling(p) * 0.5).max(0.3);
        let damp = if curious { 0.6 } else { 1.0 };
        (feel * (1.0 - fear * damp)).clamp(0.05, 1.5)
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Relation {
    /// -1 hostile .. +1 close
    pub opinion: f32,
    /// 0..1 how well they know each other
    pub familiarity: f32,
    /// Last tick anything moved this relationship, including hearsay (gossip, suspicion told,
    /// feud sides) as well as real contact: "last heard of", not "last seen". See `met` for the
    /// in-person stamp (#1131, 2026-11-15 fix-1131-met-stamp-real-contact).
    pub last_contact: u64,
    /// Last tick of real, in-person contact (talk, quarrel, hearth tale, drinking together,
    /// pairing, making up, fostering, working side by side, childhood play). Zero until it
    /// happens once; hearsay never sets this (#1131, 2026-11-15
    /// fix-1131-met-stamp-real-contact). A fade-by-time-apart feature (#1132) should read this,
    /// not `last_contact`.
    #[serde(default)]
    pub met: u64,
}

pub const RELATION_CAP: usize = 32;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consolidation_is_bounded_and_biases() {
        let mut m = Memories::default();
        let bush = TilePos::new(5, 5, 1);
        for t in 0..200u64 {
            m.record(Record {
                tick: t,
                kind: MemKind::Meal,
                subject: Subject::Place(bush),
                valence: -0.8,
                salience: 0.6,
                event: None,
            });
        }
        assert!(m.short.len() <= SHORT_CAP);
        let n = m.consolidate(300, false, false);
        assert_eq!(n, 1);
        assert!(
            m.bias(MemKind::Meal, Subject::Place(bush)) < 0.6,
            "bad meals there bias away: {}",
            m.bias(MemKind::Meal, Subject::Place(bush))
        );
        assert_eq!(
            m.bias(MemKind::Meal, Subject::Place(TilePos::new(9, 9, 1))),
            1.0
        );
        for i in 0..200u64 {
            m.record(Record {
                tick: 400 + i,
                kind: MemKind::Company,
                subject: Subject::Person(AgentId(i as u32)),
                valence: 0.5,
                salience: 0.5,
                event: None,
            });
            m.consolidate(500 + i, false, false);
        }
        assert!(m.long.len() <= LONG_CAP);
    }
}
