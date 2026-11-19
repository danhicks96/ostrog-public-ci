//! Accounts of the colony's own deaths (#452, docs/design-stories.md): what happened, what
//! people say happened, and what a later person reads from a grave.
//!
//! The `Died` row is the record and nothing here changes it. Those who saw the death hold an
//! account of it with the true cause. An account passes in talk, with its teller, and may change
//! in the telling by the old valley's rule (lore.rs): a teller who holds that strangers bring
//! harm, or is cautious, tells it darker, so a fever becomes a curse and a fall a killing. One
//! who is buried where they lay gets a grave; a grown person who never knew them and came after
//! reads it, and can only say someone lies there, or, if afraid, that a curse did it. An account
//! of a curse or a taking makes its holder uneasy at the place, and only its holder.

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, EventId, TilePos};
use crate::mind::{MemKind, Subject};
use crate::rng::{purpose, stream};
use crate::world_state::World;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// How far a death is seen from, on the same level, in sight.
pub const WITNESS_WITHIN: i32 = 8;
/// How near a grave a person must stand to read it.
pub const READ_WITHIN: i32 = 3;
/// The most accounts a person keeps; the oldest told ones go first.
pub const KEPT: usize = 24;

/// What is said of how someone died: the record's cause in a person's words, or what it drifted
/// into, or what a grave lets someone guess.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Said {
    Sickness,
    Poison,
    Hunger,
    Thirst,
    Cold,
    Wounds,
    Beast,
    Fell,
    Drowned,
    Rock,
    Lost,
    Age,
    /// The record's own words, for a death of no common kind.
    Other(String),
    // What a telling drifts into.
    Cursed,
    Killed,
    Taken,
    /// A grave read by someone who cannot tell how.
    Unknown,
    /// Words carried on a marked grave (#1743): read back exactly as written, not reworded by
    /// `who`.
    Marked(String),
}

impl Said {
    /// The record's cause, as someone who saw it would say it.
    pub fn of_cause(cause: &str) -> Said {
        let c = cause;
        if c == "old age" {
            Said::Age
        } else if c.contains("starv") {
            Said::Hunger
        } else if c.contains("thirst") {
            Said::Thirst
        } else if c.contains("froze") || c.contains("cold") {
            Said::Cold
        } else if c.contains("poison") {
            Said::Poison
        } else if c.contains("fever") || c.contains("sick") || c.contains("fester") {
            Said::Sickness
        } else if c.contains("bled") || c.contains("wound") || c.starts_with("killed by") {
            Said::Wounds
        } else if c.contains("torn") || c.contains("beast") {
            Said::Beast
        } else if c.contains("drown") {
            Said::Drowned
        } else if c.contains("fallen rock") {
            Said::Rock
        } else if c == "fell" {
            Said::Fell
        } else if c.starts_with("lost") {
            Said::Lost
        } else {
            Said::Other(c.to_string())
        }
    }

    /// What it becomes told by someone afraid: an illness or a hunger is a curse, a fall or a
    /// fight a killing, a beast, the water, the rock or the road a taking.
    pub fn drift(&self) -> Said {
        match self {
            Said::Sickness | Said::Hunger | Said::Thirst => Said::Cursed,
            Said::Poison | Said::Wounds | Said::Fell => Said::Killed,
            Said::Beast | Said::Drowned | Said::Rock | Said::Lost => Said::Taken,
            s => s.clone(),
        }
    }

    /// A version that makes the place feared.
    pub fn dark(&self) -> bool {
        matches!(self, Said::Cursed | Said::Taken)
    }

    /// Whether this came off a marked grave, read back exactly as the burier wrote it.
    pub fn is_marked(&self) -> bool {
        matches!(self, Said::Marked(_))
    }

    /// The telling, of `who`.
    pub fn telling(&self, who: &str) -> String {
        match self {
            Said::Sickness => format!("{who} died of a sickness"),
            Said::Poison => format!("{who} died of something they ate"),
            Said::Hunger => format!("{who} starved"),
            Said::Thirst => format!("{who} died of thirst"),
            Said::Cold => format!("{who} froze"),
            Said::Wounds => format!("{who} died of their wounds"),
            Said::Beast => format!("a beast killed {who}"),
            Said::Fell => format!("{who} fell to their death"),
            Said::Drowned => format!("{who} drowned"),
            Said::Rock => format!("the rock came down on {who}"),
            Said::Lost => format!("{who} was lost"),
            Said::Age => format!("{who} died old"),
            Said::Other(c) => format!("{who} died: {c}"),
            Said::Cursed => format!("{who} was cursed"),
            Said::Killed => format!("someone killed {who}"),
            Said::Taken => format!("something took {who}"),
            Said::Unknown => format!("{who} was put in the ground here, nobody can say how"),
            Said::Marked(words) => words.clone(),
        }
    }
}

/// How someone came by an account.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum How {
    Saw,
    Told,
    /// Read from a grave: the account's place is the grave.
    Grave,
}

/// One person's account of a death.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Account {
    /// The record's row of the death.
    pub death: EventId,
    /// Whose death, if they know; a grave read does not say.
    pub of: Option<AgentId>,
    /// Where it happened, or the grave read.
    pub at: TilePos,
    pub said: Said,
    pub teller: Option<AgentId>,
    pub how: How,
    pub heard: u64,
}

/// A grave dug by the colony's people (systems/rites.rs, `Rite::Bury`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Grave {
    pub pos: TilePos,
    pub dead: AgentId,
    /// The `Died` row and the `LaidToRest` row.
    pub death: EventId,
    pub rite: EventId,
    pub laid: u64,
    /// The dead's name and what the burier marked them as known for (#1743): `None` for a
    /// grave dug with no one to write it (burial `in_absence`, which makes no grave at all
    /// today — see `systems/rites.rs::lay_to_rest`).
    #[serde(default)]
    pub words: Option<String>,
}

/// The burier's words for a short "known for" phrase, from the dead's own strongest skill
/// (#1201's fuller reputation record does not exist yet; this is the simple stand-in the
/// issue asked for). `None` below a skill's practice floor, so an untrained person still just
/// gets their name.
fn known_for(w: &World, dead: AgentId) -> Option<&'static str> {
    let d = w.agents.iter().find(|a| a.id == dead)?;
    let best = crate::mind::skills::ALL
        .iter()
        .copied()
        .map(|s| (s, d.skills.level(s)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())?;
    if best.1 <= 0.0 {
        return None;
    }
    use crate::mind::skills::Skill::*;
    Some(match best.0 {
        Foraging => "who gathered the valley's food",
        Farming => "who tended the fields",
        Cooking => "who fed the table",
        Construction => "who raised the walls",
        Mining => "who dug the stone",
        Crafting => "who made things last",
        Social => "who kept the people together",
        Hauling => "who carried the load",
        Animals => "who minded the animals",
    })
}

fn name(w: &World, id: AgentId) -> String {
    w.agents
        .iter()
        .find(|a| a.id == id)
        .map(|a| a.name.clone())
        .unwrap_or_else(|| "someone".into())
}

/// Keep an account, dropping the oldest told one (else the oldest) past `KEPT`. A version of a
/// curse or a taking makes the place an uneasy one to its holder.
fn keep(w: &mut World, i: usize, acc: Account) {
    let tick = w.meta.tick;
    let a = &mut w.agents[i];
    if acc.said.dark() {
        a.remember(
            tick,
            MemKind::Hardship,
            Subject::Place(acc.at),
            -0.4,
            0.5,
            Some(acc.death),
        );
    }
    a.accounts.push(acc);
    if a.accounts.len() > KEPT {
        let k = a
            .accounts
            .iter()
            .position(|x| x.how != How::Saw)
            .unwrap_or(0);
        a.accounts.remove(k);
    }
}

/// A person has died (systems/life.rs `die`): those awake who saw it, grown, on the same level
/// within `WITNESS_WITHIN` tiles and in sight, hold an account of it with the true cause.
pub fn witnessed(w: &mut World, dead: usize, death: EventId, cause: &str) {
    let tick = w.meta.tick;
    if w.agents[dead].species != crate::agents::Species::Person {
        return;
    }
    let (of, at) = (w.agents[dead].id, w.agents[dead].pos);
    let said = Said::of_cause(cause);
    let mut saw = crate::mind::perception::witnesses(w, at, WITNESS_WITHIN, true);
    saw.retain(|&i| w.agents[i].id != of);
    for i in saw {
        keep(
            w,
            i,
            Account {
                death,
                of: Some(of),
                at,
                said: said.clone(),
                teller: None,
                how: How::Saw,
                heard: tick,
            },
        );
    }
}

/// Two people talked: each may tell the other a death they have no account of, and it may
/// change in the telling.
pub fn on_talk(w: &mut World, i: usize, j: usize) {
    let tick = w.meta.tick;
    for (t, l) in [(i, j), (j, i)] {
        let listener = w.agents[l].id;
        let Some(acc) = w.agents[t]
            .accounts
            .iter()
            .find(|x| {
                x.of.is_some()
                    && x.of != Some(listener)
                    && !w.agents[l]
                        .accounts
                        .iter()
                        .any(|y| y.death == x.death && y.of.is_some())
            })
            .cloned()
        else {
            continue;
        };
        let teller = w.agents[t].id;
        let mut r = stream(
            w.meta.seed,
            &[purpose::ACCOUNTS, teller.0 as u64, listener.0 as u64, tick],
        );
        if !r.gen_bool(0.5) {
            continue;
        }
        let dark = w.agents[t]
            .beliefs
            .lean(crate::mind::beliefs::Topic::Strangers)
            > 0.2
            || w.agents[t].traits.has(crate::mind::Trait::Cautious);
        let said = if r.gen_bool(if dark { 0.45 } else { 0.2 }) {
            acc.said.drift()
        } else {
            acc.said.clone()
        };
        let changed = said != acc.said;
        let dead = acc.of.unwrap_or(teller);
        let version = said.telling(&name(w, dead));
        keep(
            w,
            l,
            Account {
                death: acc.death,
                of: acc.of,
                at: acc.at,
                said,
                teller: Some(teller),
                how: How::Told,
                heard: tick,
            },
        );
        let at = w.agents[l].pos;
        w.events.push(
            tick,
            Some(at),
            if changed {
                Severity::Notable
            } else {
                Severity::Minor
            },
            EventKind::AccountTold {
                teller,
                listener,
                dead,
                version,
                changed,
            },
        );
    }
}

/// The dead were buried where they lay (systems/rites.rs): a grave on that tile if it is open
/// ground with nothing standing on it and no roof over it, else on the nearest such tile within
/// two; none if there is none.
pub fn buried(w: &mut World, dead: AgentId, death: EventId, rite: EventId, by: Option<AgentId>) {
    let tick = w.meta.tick;
    let Some(d) = w.agents.iter().find(|a| a.id == dead) else {
        return;
    };
    let pos = d.pos;
    // Marked by whoever buries them (#1743, FIXLOG 2026-11-16): their name, and the strongest
    // skill they held, or the name alone if they held none. Nothing is written when there is
    // no one to write it.
    let words = by.map(|_| match known_for(w, dead) {
        Some(phrase) => format!("{}, {phrase}", name(w, dead)),
        None => name(w, dead),
    });
    let open = |q: TilePos| {
        w.grid.walkable(q)
            && !w.roofs.contains_key(&q)
            && !w.structures.iter().any(|s| s.pos == q)
            && !w.graves.iter().any(|g| g.pos == q)
    };
    let mut best: Option<(i32, TilePos)> = None;
    for dy in -2..=2 {
        for dx in -2..=2 {
            let q = pos.offset(dx, dy, 0);
            let d = q.dist(pos);
            if open(q) && best.is_none_or(|(bd, bq)| (d, q) < (bd, bq)) {
                best = Some((d, q));
            }
        }
    }
    let Some((_, at)) = best else { return };
    let sid = w.alloc_structure_id();
    w.structures.push(crate::structures::Structure {
        id: sid,
        kind: crate::structures::StructureKind::Grave,
        pos: at,
        material: crate::world::Material::SOIL,
        progress: 1.0,
        active: false,
        condition: 1.0,
        owner: None,
        quality: 0.5,
    });
    w.graves.push(Grave {
        pos: at,
        dead,
        death,
        rite,
        laid: tick,
        words,
    });
}

/// Hourly: a grown person standing by a grave of someone they never knew, who came after the
/// death and holds no account of it, reads it.
pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if !tick.is_multiple_of(crate::time::TICKS_PER_HOUR) || w.graves.is_empty() {
        return;
    }
    let mut news = Vec::new();
    for i in 0..w.agents.len() {
        let a = &w.agents[i];
        if !a.alive
            || a.species != crate::agents::Species::Person
            || a.community.is_none()
            || a.band.is_some()
            || a.activity.is_asleep()
            || a.is_child(tick)
        {
            continue;
        }
        for g in &w.graves {
            if a.pos.z != g.pos.z
                || a.pos.dist(g.pos) > READ_WITHIN
                || a.came <= g.laid
                || a.relationships.contains_key(&g.dead)
                || a.accounts.iter().any(|x| x.death == g.death)
            {
                continue;
            }
            // A marked grave (#1743) tells a newcomer who lies there; an unmarked one only
            // lets them guess, darker if they are fearful of death.
            let (said, of) = match &g.words {
                Some(words) => (Said::Marked(words.clone()), Some(g.dead)),
                None => {
                    let fearful = a.traits.has(crate::mind::Trait::Cautious)
                        || a.beliefs.lean(crate::mind::beliefs::Topic::Death) > 0.3;
                    (if fearful { Said::Cursed } else { Said::Unknown }, None)
                }
            };
            news.push((i, g.death, g.pos, said, of));
        }
    }
    for (i, death, at, said, of) in news {
        let guess = said.telling("someone");
        keep(
            w,
            i,
            Account {
                death,
                of,
                at,
                said,
                teller: None,
                how: How::Grave,
                heard: tick,
            },
        );
        let who = w.agents[i].id;
        w.events.push(
            tick,
            Some(at),
            Severity::Notable,
            EventKind::GraveRead {
                agent: who,
                at,
                guess,
            },
        );
    }
}

/// What someone holds of the colony's own dead, for inspect: "Halgar was cursed (told by
/// Kella)".
pub fn heard(w: &World, a: &crate::agents::Agent) -> Vec<String> {
    a.accounts
        .iter()
        .map(|x| {
            let who = x.of.map(|o| name(w, o)).unwrap_or_else(|| "someone".into());
            let how = match (x.how, x.teller) {
                (How::Saw, _) => "saw it".to_string(),
                (How::Told, Some(t)) => format!("told by {}", name(w, t)),
                (How::Told, None) => "told".to_string(),
                (How::Grave, _) => "read from a grave".to_string(),
            };
            format!("{} ({how})", x.said.telling(&who))
        })
        .collect()
}

/// The hover of a grave the colony dug: what the record says, then what is said of it, by
/// whom. The record is the viewer's; the sayings are the people's, and marked as theirs.
pub fn note(w: &World, p: TilePos) -> Option<String> {
    let g = w.graves.iter().find(|g| g.pos == p)?;
    let who = name(w, g.dead);
    let cause = w
        .agents
        .iter()
        .find(|a| a.id == g.dead)
        .and_then(|a| a.death.clone())
        .unwrap_or_default();
    let mut out = format!(
        "The grave of {who}, day {}. The record: {cause}.",
        g.laid / crate::time::TICKS_PER_DAY
    );
    let mut said: Vec<(String, Vec<String>)> = Vec::new();
    for a in w.agents.iter().filter(|a| a.alive) {
        for x in a.accounts.iter().filter(|x| x.death == g.death) {
            let words = x
                .said
                .telling(if x.of.is_some() { &who } else { "someone" });
            match said.iter_mut().find(|(s, _)| *s == words) {
                Some((_, v)) => v.push(a.name.clone()),
                None => said.push((words, vec![a.name.clone()])),
            }
        }
    }
    for (words, by) in said {
        let n = by.len();
        let by = if n > 3 {
            format!("{} and {} more", by[..3].join(", "), n - 3)
        } else {
            by.join(", ")
        };
        out.push_str(&format!(" Said: {words} ({by})."));
    }
    Some(out)
}
