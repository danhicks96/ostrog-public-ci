//! Arcs (issue #53): what the settlement is going through, as one story rather than separate
//! events.
//!
//! Each hour this looks at what is wrong right now: strangers camped near, fever in the houses,
//! hard cold, hunger, fire, people hurt. It fires nothing and changes nothing. When a new trouble
//! starts while others are still going, the feed says so, and says why if one trouble plainly
//! fed another (hunger after the stores were raided, fever on empty stomachs or in the cold,
//! wounds from the fighting). When everything has been quiet for half a day after a hard
//! stretch, the feed says it eased, how long it lasted and who did not live to see it.

use crate::events::{EventKind, Severity};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::World;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Trouble {
    Strangers,
    Fever,
    Cold,
    Hunger,
    Fire,
    Hurt,
}

impl Trouble {
    pub fn words(self) -> &'static str {
        match self {
            Trouble::Strangers => "strangers camped near",
            Trouble::Fever => "fever in the houses",
            Trouble::Cold => "a hard cold",
            Trouble::Hunger => "hunger",
            Trouble::Fire => "fire",
            Trouble::Hurt => "people hurt",
        }
    }
    fn short(self) -> &'static str {
        match self {
            Trouble::Strangers => "strangers",
            Trouble::Fever => "fever",
            Trouble::Cold => "cold",
            Trouble::Hunger => "hunger",
            Trouble::Fire => "fire",
            Trouble::Hurt => "wounds",
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ArcState {
    /// Troubles going on now: the tick each began and the tick it was last seen. A trouble that
    /// comes and goes by the hour ends only after hours unseen (see `step`).
    pub now: Vec<(Trouble, u64, u64)>,
    /// When this stretch began (0: none going).
    pub since: u64,
    /// Every kind of trouble seen in this stretch, in the order they came.
    pub seen: Vec<Trouble>,
    /// Settlers who died during it.
    pub lost: u32,
    /// When the last trouble cleared.
    pub clear_since: u64,
}

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if w.arc.since > 0 {
        for e in &w.last_events {
            if let EventKind::Died { agent, .. } = e.kind {
                if w.agents
                    .iter()
                    .any(|a| a.id == agent && a.community.is_some() && a.band.is_none())
                {
                    w.arc.lost += 1;
                }
            }
        }
    }
    if tick % TICKS_PER_HOUR != 31 || w.communities.iter().all(|c| c.dissolved) {
        return;
    }
    let seen_now = troubles(w);
    let before: Vec<Trouble> = w.arc.now.iter().map(|x| x.0).collect();
    let mut news = Vec::new();
    for t in &seen_now {
        if !before.contains(t) {
            news.push(*t);
        }
    }
    for x in w.arc.now.iter_mut() {
        if seen_now.contains(&x.0) {
            x.2 = tick;
        }
    }
    // Strangers and fire have clear ends; cold, hunger, fever and wounds come and go by the hour.
    w.arc.now.retain(|x| {
        let hold = match x.0 {
            Trouble::Strangers | Trouble::Fire => 0,
            // A winter cold lifts at midday and is back by dark: still the same cold.
            Trouble::Cold => 18 * TICKS_PER_HOUR,
            _ => 8 * TICKS_PER_HOUR,
        };
        tick - x.2 <= hold
    });
    for t in &news {
        w.arc.now.push((*t, tick, tick));
        if !w.arc.seen.contains(t) {
            w.arc.seen.push(*t);
        }
    }
    let current: Vec<Trouble> = w.arc.now.iter().map(|x| x.0).collect();
    if !current.is_empty() {
        if w.arc.since == 0 {
            w.arc.since = tick;
            w.arc.lost = 0;
            w.arc.seen = current.clone();
        }
        w.arc.clear_since = 0;
    } else if w.arc.since > 0 {
        if w.arc.clear_since == 0 {
            w.arc.clear_since = tick;
        } else if tick - w.arc.clear_since >= 12 * TICKS_PER_HOUR {
            eased(w);
        }
    }
    // A new trouble on top of others: the turn in the story.
    for t in news {
        let during: Vec<Trouble> = before
            .iter()
            .copied()
            .filter(|b| current.contains(b))
            .collect();
        if during.is_empty() {
            continue;
        }
        let because = cause(w, t, &during).map(|s| s.to_string());
        w.events.push(
            tick,
            None,
            Severity::Notable,
            EventKind::ArcTurn {
                now: t.words().to_string(),
                during: during.iter().map(|d| d.words().to_string()).collect(),
                because,
            },
        );
    }
}

fn eased(w: &mut World) {
    let tick = w.meta.tick;
    let a = std::mem::take(&mut w.arc);
    let days = (a.clear_since.saturating_sub(a.since)) / TICKS_PER_DAY;
    // A single short trouble is its own story; an arc is a stretch.
    if a.seen.len() >= 2 || days >= 2 {
        w.events.push(
            tick,
            None,
            Severity::Notable,
            EventKind::ArcEased {
                days: days.max(1) as u32,
                what: a.seen.iter().map(|t| t.short().to_string()).collect(),
                lost: a.lost,
            },
        );
    }
}

/// What is wrong right now, for the settlement.
pub fn troubles(w: &World) -> Vec<Trouble> {
    let people: Vec<&crate::agents::Agent> = w
        .agents
        .iter()
        .filter(|a| a.alive && a.community.is_some() && a.band.is_none())
        .collect();
    let mut out = Vec::new();
    if people.is_empty() {
        return out;
    }
    if w.bands
        .iter()
        .any(|b| b.sighted && !matches!(b.state, crate::threat::BandState::Leaving) && !b.welcomed)
    {
        out.push(Trouble::Strangers);
    }
    // A sickness in the houses; a plant eaten in error (#286) is one person's bad hours, not
    // a fever, and the feed would call it one.
    if people.iter().any(|a| {
        a.body
            .illness
            .is_some_and(|ill| ill.kind != crate::body::IllnessKind::Poisoned)
    }) {
        out.push(Trouble::Fever);
    }
    if w.weather.temperature_c < -6.0 {
        out.push(Trouble::Cold);
    }
    let starving = people.iter().filter(|a| a.needs.hunger < 0.12).count();
    if starving >= 2 || (starving >= 1 && people.len() <= 3) {
        out.push(Trouble::Hunger);
    }
    if !w.fires.is_empty() {
        out.push(Trouble::Fire);
    }
    let hurt = people.iter().filter(|a| a.body.needs_care()).count();
    if hurt >= 2 {
        out.push(Trouble::Hurt);
    }
    out
}

/// Did one of the troubles already going plainly feed this one?
fn cause(w: &World, t: Trouble, during: &[Trouble]) -> Option<&'static str> {
    let raided = w.bands.iter().any(|b| b.taken > 0 && !b.welcomed);
    match t {
        Trouble::Hunger if during.contains(&Trouble::Strangers) && raided => {
            Some("the strangers took from the stores")
        }
        Trouble::Hunger if during.contains(&Trouble::Fever) => Some("the sick cannot work"),
        Trouble::Hunger if during.contains(&Trouble::Cold) => Some("nothing grows in the cold"),
        Trouble::Fever if during.contains(&Trouble::Hunger) => Some("people are weak from hunger"),
        Trouble::Fever if during.contains(&Trouble::Cold) => Some("the cold gets into everyone"),
        Trouble::Fever if during.contains(&Trouble::Hurt) => Some("wounds went bad"),
        Trouble::Hurt if during.contains(&Trouble::Strangers) => Some("there has been fighting"),
        Trouble::Hurt if during.contains(&Trouble::Fire) => Some("the fire caught people"),
        _ => None,
    }
}

/// One line for the screen while troubles overlap or a hard stretch goes on, else None:
/// "Hard times, day 3: strangers camped near, fever in the houses."
pub fn status_line(w: &World) -> Option<String> {
    if w.arc.since == 0 || w.arc.now.is_empty() {
        return None;
    }
    let days = w.meta.tick.saturating_sub(w.arc.since) / TICKS_PER_DAY + 1;
    if w.arc.now.len() < 2 && days < 2 {
        return None;
    }
    let what: Vec<&str> = w.arc.now.iter().map(|x| x.0.words()).collect();
    Some(format!("Hard times, day {days}: {}.", what.join(", ")))
}

/// How much is going wrong at once, 0..1, for the music: each trouble going on adds a quarter.
pub fn weight(w: &World) -> f32 {
    (w.arc.now.len() as f32 * 0.25).min(1.0)
}
