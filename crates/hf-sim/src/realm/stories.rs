//! Stories (P8 of milestone MP, docs/design-peoples.md section 5.2). A culture tells the
//! world's history its own way: each story is one culture's telling of a recorded event, and
//! the same battle is a victory or a lament to one side and a defeat to the other. Stories drift
//! as they are retold: a defeat becomes a betrayal by a named figure who was there, a victory
//! becomes the will of the gods, a famine a curse. A story always keeps the row it began from,
//! so its telling can be set against the record. Defeats leave grudges; what strikes a culture
//! often enough becomes a god of it.

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::sites::{Realm, RealmEvent, RealmEventKind};
use crate::ids::EventId;
use crate::past::Topic;
use crate::rng::{purpose, stream};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum StoryKind {
    Victory,
    /// A victory mourned, or a famine, a plague, a place lost.
    Lament,
    Defeat,
    /// A defeat retold: someone sold them out.
    Betrayal,
    /// A victory retold: the gods gave it.
    Destined,
    Founding,
    /// A founding retold: they were driven there.
    DrivenOut,
    /// A famine or plague retold: it was a curse.
    Curse,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Story {
    pub id: u32,
    pub culture: u32,
    pub kind: StoryKind,
    /// The row it began from: never changes, however it is retold.
    pub about: EventId,
    pub year: i64,
    pub site: Option<u32>,
    /// A figure it honours.
    pub hero: Option<u32>,
    /// The people it holds against.
    pub foe: Option<u32>,
    /// Who it says betrayed them (a real figure of their own people, alive at the time).
    pub traitor: Option<u32>,
    /// For a famine or plague: how many it took.
    pub lost: u32,
    /// How many retellings changed it.
    pub version: u16,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Grudge {
    pub against: u32,
    pub since: EventId,
    pub strength: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct God {
    pub name: String,
    /// "hunger", "sickness", "war".
    pub domain: String,
    /// The row of the first trouble it was named for.
    pub first: EventId,
}

/// Stories a culture keeps; the oldest least-retold go first past this.
pub const KEEP: usize = 200;

fn culture_of_polity(r: &Realm, polity: u32) -> Option<u32> {
    let p = r.polities.get(polity as usize)?;
    r.sites.get(p.seat as usize).map(|s| s.culture)
}

fn people_of_polity(r: &Realm, polity: u32) -> Option<u32> {
    r.polities.get(polity as usize).map(|p| p.people)
}

fn push(r: &mut Realm, mut s: Story) {
    s.id = r.next_story;
    r.next_story += 1;
    r.stories.push(s);
}

fn grudge(r: &mut Realm, culture: u32, against: u32, since: EventId, by: f32) {
    let Some(c) = r.cultures.get_mut(culture as usize) else {
        return;
    };
    if c.people == against {
        return;
    }
    match c.grudges.iter_mut().find(|g| g.against == against) {
        Some(g) => {
            g.strength = (g.strength + by).min(1.0);
            g.since = since;
        }
        None => c.grudges.push(Grudge {
            against,
            since,
            strength: by.min(1.0),
        }),
    }
}

/// Makes each culture's stories of a year's rows. `rows` are the chronicle ids the events
/// were recorded under, with the events.
pub fn record(r: &mut Realm, rows: &[(EventId, RealmEvent)]) {
    let mut defeated: Vec<(u32, EventId)> = Vec::new();
    for (id, e) in rows {
        match &e.kind {
            RealmEventKind::Battle {
                war, site, won_by, ..
            } => {
                let Some(w) = r.wars.get(*war as usize).cloned() else {
                    continue;
                };
                let lost_by = if w.a == *won_by { w.b } else { w.a };
                let (Some(wc), Some(lc)) =
                    (culture_of_polity(r, *won_by), culture_of_polity(r, lost_by))
                else {
                    continue;
                };
                let (wp, lp) = (people_of_polity(r, *won_by), people_of_polity(r, lost_by));
                // A hero: one of the winners' people who fell that year in battle.
                let hero = r
                    .figures
                    .iter()
                    .find(|f| {
                        Some(f.people) == wp
                            && f.died
                                .as_ref()
                                .is_some_and(|(y, c)| *y == e.year && c.contains("battle"))
                    })
                    .map(|f| f.id);
                // A people that holds peace dear mourns even a battle it won.
                let mourn = r
                    .cultures
                    .get(wc as usize)
                    .is_some_and(|c| c.stance(Topic::Peace) > 0.3);
                push(
                    r,
                    Story {
                        id: 0,
                        culture: wc,
                        kind: if mourn {
                            StoryKind::Lament
                        } else {
                            StoryKind::Victory
                        },
                        about: *id,
                        year: e.year,
                        site: Some(*site),
                        hero,
                        foe: lp,
                        traitor: None,
                        lost: 0,
                        version: 0,
                    },
                );
                push(
                    r,
                    Story {
                        id: 0,
                        culture: lc,
                        kind: StoryKind::Defeat,
                        about: *id,
                        year: e.year,
                        site: Some(*site),
                        hero: None,
                        foe: wp,
                        traitor: None,
                        lost: 0,
                        version: 0,
                    },
                );
                if let Some(wp) = wp {
                    grudge(r, lc, wp, *id, 0.3);
                }
                defeated.push((lc, *id));
            }
            RealmEventKind::Taken { site, by } => {
                let Some(c) = r.sites.get(*site as usize).map(|s| s.culture) else {
                    continue;
                };
                let foe = people_of_polity(r, *by);
                push(
                    r,
                    Story {
                        id: 0,
                        culture: c,
                        kind: StoryKind::Lament,
                        about: *id,
                        year: e.year,
                        site: Some(*site),
                        hero: None,
                        foe,
                        traitor: None,
                        lost: 0,
                        version: 0,
                    },
                );
                if let Some(f) = foe {
                    grudge(r, c, f, *id, 0.4);
                }
            }
            RealmEventKind::Famine { site, lost } | RealmEventKind::Plague { site, lost } => {
                let Some(c) = r.sites.get(*site as usize).map(|s| s.culture) else {
                    continue;
                };
                push(
                    r,
                    Story {
                        id: 0,
                        culture: c,
                        kind: StoryKind::Lament,
                        about: *id,
                        year: e.year,
                        site: Some(*site),
                        hero: None,
                        foe: None,
                        traitor: None,
                        lost: *lost,
                        version: 0,
                    },
                );
                let domain = if matches!(e.kind, RealmEventKind::Famine { .. }) {
                    "hunger"
                } else {
                    "sickness"
                };
                name_god(r, c, domain, *id);
            }
            RealmEventKind::Founded {
                site,
                parent: Some(_),
            } => {
                let Some(c) = r.sites.get(*site as usize).map(|s| s.culture) else {
                    continue;
                };
                // The one who led them there, if the record names someone.
                let hero = rows.iter().find_map(|(_, x)| match x.kind {
                    RealmEventKind::Led { figure, site: s } if s == *site => Some(figure),
                    _ => None,
                });
                push(
                    r,
                    Story {
                        id: 0,
                        culture: c,
                        kind: StoryKind::Founding,
                        about: *id,
                        year: e.year,
                        site: Some(*site),
                        hero,
                        foe: None,
                        traitor: None,
                        lost: 0,
                        version: 0,
                    },
                );
            }
            _ => {}
        }
    }
    // Defeats bring a god of war to those who suffer them often enough.
    for (c, about) in defeated {
        name_god(r, c, "war", about);
    }
    // Keep each culture's stories to KEEP: the oldest of the least retold go first.
    let n = r.stories.len();
    if n > KEEP * r.cultures.len().max(1) {
        r.stories.sort_by_key(|s| {
            (
                s.culture,
                std::cmp::Reverse(s.version),
                std::cmp::Reverse(s.year),
                s.id,
            )
        });
        let mut seen = std::collections::BTreeMap::<u32, usize>::new();
        r.stories.retain(|s| {
            let k = seen.entry(s.culture).or_insert(0);
            *k += 1;
            *k <= KEEP
        });
        r.stories.sort_by_key(|s| s.id);
    }
}

/// A culture that has suffered a kind of trouble three times names a god of it, in its people's
/// language, for the first of those troubles.
fn name_god(r: &mut Realm, culture: u32, domain: &str, about: EventId) {
    let kinds: &[StoryKind] = match domain {
        "war" => &[StoryKind::Defeat, StoryKind::Betrayal],
        _ => &[StoryKind::Lament, StoryKind::Curse],
    };
    let count = r
        .stories
        .iter()
        .filter(|s| {
            s.culture == culture && kinds.contains(&s.kind) && (domain == "war" || s.lost > 0)
        })
        .count();
    let Some(c) = r.cultures.get(culture as usize) else {
        return;
    };
    if count < 3 || c.gods.iter().any(|g| g.domain == domain) {
        return;
    }
    let people = c.people as usize;
    let first = r
        .stories
        .iter()
        .find(|s| s.culture == culture && kinds.contains(&s.kind))
        .map(|s| s.about)
        .unwrap_or(about);
    let mut name = r.peoples[people]
        .language
        .word(r.seed, &format!("god:{domain}"));
    if let Some(ch) = name.get_mut(0..1) {
        ch.make_ascii_uppercase();
    }
    r.cultures[culture as usize].gods.push(God {
        name,
        domain: domain.to_string(),
        first,
    });
}

/// Once every twenty-five years each culture retells its stories: some change on the way
/// (more often among a people wary of strangers), along the drift map. Grudges fade a little
/// each year they are not renewed.
pub fn retell(r: &mut Realm, year: i64) {
    for c in r.cultures.iter_mut() {
        for g in c.grudges.iter_mut() {
            g.strength *= 0.99;
        }
        c.grudges.retain(|g| g.strength > 0.05);
    }
    if year % 25 != 0 {
        return;
    }
    let seed = r.seed;
    for k in 0..r.stories.len() {
        let s = r.stories[k].clone();
        // One roll a story, in the generation after it (#222: a roll every twenty-five
        // years for five centuries turned nearly every lament into a curse, and a site page
        // read the same curse four times).
        if year - s.year < 25 || year - s.year >= 50 || s.version > 0 {
            continue;
        }
        let Some(c) = r.cultures.get(s.culture as usize) else {
            continue;
        };
        let wary = r.peoples[c.people as usize].conviction(Topic::Strangers) > 0.2;
        let mut rng = stream(seed, &[purpose::LORE, 0x5701, s.id as u64, year as u64]);
        if !rng.gen_bool(if wary { 0.45 } else { 0.2 }) {
            continue;
        }
        let kind = match s.kind {
            StoryKind::Defeat => StoryKind::Betrayal,
            StoryKind::Victory => StoryKind::Destined,
            StoryKind::Founding => StoryKind::DrivenOut,
            // A loss worth a curse (#246: "1 of the Driagliawe ... were lost" as a curse).
            StoryKind::Lament if s.lost >= 5 => StoryKind::Curse,
            other => other,
        };
        if kind == s.kind {
            continue;
        }
        // A betrayer is a real figure of their own people, alive in the year of the defeat.
        let traitor = if kind == StoryKind::Betrayal {
            let people = c.people;
            let alive: Vec<u32> = r
                .figures
                .iter()
                .filter(|f| {
                    f.people == people
                        && f.born < s.year
                        && f.died.as_ref().is_none_or(|(y, _)| *y >= s.year)
                })
                .map(|f| f.id)
                .collect();
            if alive.is_empty() {
                continue;
            }
            Some(alive[rng.gen_range(0..alive.len())])
        } else {
            None
        };
        let st = &mut r.stories[k];
        st.kind = kind;
        st.traitor = traitor;
        st.version += 1;
    }
}

/// How a culture tells a story now: "At Tessen Ford the Vashka fell; Vrikka sold them out."
pub fn telling(r: &Realm, s: &Story) -> String {
    let site = s
        .site
        .and_then(|i| r.sites.get(i as usize))
        .map(|x| x.name.clone())
        .unwrap_or_else(|| "a place now forgotten".into());
    let us = r
        .cultures
        .get(s.culture as usize)
        .and_then(|c| r.peoples.get(c.people as usize))
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "their people".into());
    let foe = s
        .foe
        .and_then(|p| r.peoples.get(p as usize))
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "their enemies".into());
    let fig = |f: Option<u32>| {
        f.and_then(|i| r.figures.get(i as usize))
            .map(|x| x.name.clone())
    };
    match s.kind {
        StoryKind::Victory => match fig(s.hero) {
            Some(h) => format!("At {site} the {us} beat the {foe}, and {h} fell winning it"),
            None => format!("At {site} the {us} beat the {foe}"),
        },
        StoryKind::Destined => format!("At {site} the gods gave the {us} the day over the {foe}"),
        // From the losers' side (C68: with retellings rarer, the first pair a test found read
        // the same sentence from both sides).
        StoryKind::Defeat => format!("At {site} the {us} were beaten by the {foe}"),
        StoryKind::Betrayal => match fig(s.traitor) {
            Some(t) => format!("At {site} the {us} were sold to the {foe} by {t}"),
            None => format!("At {site} the {us} were betrayed to the {foe}"),
        },
        StoryKind::Lament if s.lost > 0 => {
            if s.lost == 1 {
                format!("At {site} one of the {us} was lost to a hard time")
            } else {
                format!("At {site} {} of the {us} were lost to a hard time", s.lost)
            }
        }
        StoryKind::Lament => match (s.foe.is_some(), fig(s.hero)) {
            (true, Some(h)) => format!("{h} fell at {site} so the {us} could keep it"),
            (true, None) => format!("At {site} the {us} lost what they had to the {foe}"),
            _ => format!("The {us} mourn {site}"),
        },
        StoryKind::Curse if s.lost > 0 => format!(
            "A curse fell on {site}: {} of the {us} who lived there were lost",
            s.lost
        ),
        StoryKind::Curse => format!("A curse fell on {site} and the {us} who lived there"),
        StoryKind::Founding => match fig(s.hero) {
            Some(h) => format!("{h} led the {us} to found {site}"),
            None => format!("The {us} went out and founded {site}"),
        },
        StoryKind::DrivenOut => format!("The {us} were driven out to {site}"),
    }
}

/// Points stories, grudges and gods made during the history pass at the chronicle's ids for
/// their rows (`ids[i]` is the id of the history's i-th event).
pub fn remap(r: &mut Realm, ids: &[EventId]) {
    let map = |e: &mut EventId| {
        if let Some(id) = ids.get(e.0 as usize) {
            *e = *id;
        }
    };
    for s in r.stories.iter_mut() {
        map(&mut s.about);
    }
    for c in r.cultures.iter_mut() {
        for g in c.grudges.iter_mut() {
            map(&mut g.since);
        }
        for g in c.gods.iter_mut() {
            map(&mut g.first);
        }
    }
}
