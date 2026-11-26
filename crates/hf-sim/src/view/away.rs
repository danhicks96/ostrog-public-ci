//! Those of the valley who are in another place (#562): on a journey of their own (#347), with a
//! caravan (C36), or carried off by a host (C76). Each row says where they are, what they are
//! doing there and how it is going for them, read from what the simulation holds: their own
//! feelings, the caravan's state and log, the place's grudge, hunger and trade with the valley.
//! Nothing here is invented, and nothing changes the simulation.
//!
//! A place shows on the world map only when the valley knows where it is (the place's
//! `known_by`); someone carried off to a place nobody of the valley has been to is listed
//! without one. The unfound lost (view::known) are never listed.

use crate::expedition::{CaravanState, Purpose};
use crate::mind::feelings::FeelingKind;
use crate::time::TICKS_PER_DAY;
use crate::world_state::World;

/// How it is going for them there, for the map's colour and the list's word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reaction {
    /// Among people who trade with the valley and hold nothing against it.
    Welcomed,
    /// Held captive, stranded, or among people who hold a grudge against the valley.
    Threatened,
    /// Trading or looking over a place for the valley.
    Working,
    /// Grief is the strongest thing they feel.
    Grieving,
    /// On the road there or back.
    Travelling,
    /// None of these: among strangers.
    Among,
}

impl Reaction {
    pub fn word(self) -> &'static str {
        match self {
            Reaction::Welcomed => "welcomed",
            Reaction::Threatened => "threatened",
            Reaction::Working => "working",
            Reaction::Grieving => "grieving",
            Reaction::Travelling => "travelling",
            Reaction::Among => "among strangers",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Away {
    pub agent: crate::ids::AgentId,
    pub name: String,
    /// The place's name ("" when nobody knows where).
    pub place: String,
    /// The world map region of the place, when it is a settlement of the world the valley knows
    /// the way to.
    pub region: Option<usize>,
    pub reaction: Reaction,
    /// One line: what they are doing there and how it goes.
    pub line: String,
    /// What they feel most, and about whom ("grieving for Ada"), if anything.
    pub feeling: Option<String>,
}

fn name_of(w: &World, id: crate::ids::AgentId) -> Option<String> {
    w.agents
        .iter()
        .chain(w.overworld.away.iter())
        .find(|a| a.id == id)
        .map(|a| a.name.clone())
}

/// Everyone of the valley in another place, in the order they left.
pub fn list(w: &World) -> Vec<Away> {
    let tick = w.meta.tick;
    let mut out = Vec::new();
    for a in &w.overworld.away {
        if !a.alive || crate::view::known::is_unfound(w, a.id) {
            continue;
        }
        let Some(cid) = a.community else { continue };
        let captive_at = w
            .overworld
            .sites
            .iter()
            .find_map(|s| s.captives.iter().find(|c| c.0 == a.id).map(|c| (s, c.1)));
        let caravan = a
            .caravan
            .and_then(|id| w.overworld.caravans.iter().find(|c| c.id == id));
        let site = if let Some((s, _)) = captive_at {
            Some(s)
        } else if let Some(c) = caravan {
            let sid = match c.purpose {
                Purpose::Trade { site, .. }
                | Purpose::Scout { site }
                | Purpose::Fetch { site, .. } => site,
            };
            w.overworld.sites.iter().find(|s| s.id == sid)
        } else if let Some((_, to, _)) = &a.journey {
            w.overworld.sites.iter().find(|s| &s.name == to)
        } else {
            None
        };
        let located = site.is_some_and(|s| s.known_by.contains(&cid));
        let place = match site {
            Some(s) if located || captive_at.is_none() => s.name.clone(),
            _ => String::new(),
        };
        let region = site
            .filter(|_| located)
            .and_then(|s| s.realm_site)
            .and_then(|rs| w.realm.sites.get(rs as usize))
            .map(|x| x.region);
        let feeling = a.feelings.strongest().map(|f| {
            let about = f.about.and_then(|id| name_of(w, id));
            match (f.kind, about) {
                (FeelingKind::Grief, Some(n)) => format!("grieving for {n}"),
                (FeelingKind::Anger, Some(n)) => format!("angry with {n}"),
                (FeelingKind::Fear, Some(n)) => format!("afraid of {n}"),
                (FeelingKind::Joy, Some(n)) => format!("glad of {n}"),
                (k, _) => k.word().to_string(),
            }
        });
        let grieving = a
            .feelings
            .strongest()
            .is_some_and(|f| f.kind == FeelingKind::Grief && f.strength >= 0.2);
        let there = if place.is_empty() {
            "a place the colony does not know".to_string()
        } else {
            place.clone()
        };
        let (mut reaction, mut line) = if let Some((_, since)) = captive_at {
            let days = tick.saturating_sub(since) / TICKS_PER_DAY;
            (
                Reaction::Threatened,
                format!("held captive at {there}, carried off {days} days ago"),
            )
        } else if let Some(c) = caravan {
            let s_name = site.map(|s| s.name.as_str()).unwrap_or("the road");
            let days = site.map(|s| s.days).unwrap_or(0);
            match c.state {
                CaravanState::AtSite { .. } => {
                    // What was done there, once it is in the log (the set-out line is not).
                    let did = c
                        .log
                        .last()
                        .filter(|l| l.starts_with("at ") || l.starts_with("reached "))
                        .cloned()
                        .unwrap_or_default();
                    let doing = match c.purpose {
                        Purpose::Scout { .. } => "looking the place over",
                        _ => "trading",
                    };
                    (
                        Reaction::Working,
                        if did.is_empty() {
                            format!("{doing} at {s_name}")
                        } else {
                            let did = did.strip_prefix(&format!("at {s_name}: ")).unwrap_or(&did);
                            format!("{doing} at {s_name}: {did}")
                        },
                    )
                }
                CaravanState::Outbound { day } => (
                    Reaction::Travelling,
                    format!("on the road to {s_name}, day {} of {days}", day + 1),
                ),
                CaravanState::Returning { day } => (
                    Reaction::Travelling,
                    format!("on the way home from {s_name}, day {} of {days}", day + 1),
                ),
                CaravanState::Stranded { .. } => (
                    Reaction::Threatened,
                    format!("stranded on the road to {s_name}"),
                ),
                _ => (Reaction::Travelling, format!("with a caravan to {s_name}")),
            }
        } else if let (Some(s), Some((until, _, why))) = (site, &a.journey) {
            let back = until.saturating_sub(tick).div_ceil(TICKS_PER_DAY);
            let mut l = format!("at {} {why}", s.name);
            let r = if s.grudge > 0 {
                l += "; they hold a grudge against the colony";
                Reaction::Threatened
            } else if s.people == 0 {
                l += "; nobody lives there now";
                Reaction::Among
            } else if !s.trades.is_empty() {
                l += "; they trade with the colony";
                Reaction::Welcomed
            } else if why.contains("trade") {
                Reaction::Working
            } else {
                Reaction::Among
            };
            if s.hungry_days > 0 && s.people > 0 {
                l += "; the place is going hungry";
            }
            l += &format!("; back in {back} days");
            (r, l)
        } else {
            (Reaction::Among, format!("away at {there}"))
        };
        if grieving && reaction != Reaction::Threatened {
            reaction = Reaction::Grieving;
        }
        if let Some(f) = &feeling {
            line += &format!(", {f}");
        }
        out.push(Away {
            agent: a.id,
            name: a.name.clone(),
            place,
            region,
            reaction,
            line,
            feeling,
        });
    }
    out
}
