//! Figures and polities: the named people of the world and the realms they rule.
//!
//! A figure exists from a birth recorded in its own year; nobody is made up after the fact (a
//! settlement founded when no grown figure is at hand is led by settlers nobody named). Figures
//! live where they were born or where they led others, and die of age or of what befell their
//! home, each death with its cause. A town becomes the seat of a polity, nearby settlements of
//! its people join it, and when a ruler dies a child of theirs at the seat follows, or the
//! eldest there.

use std::collections::BTreeMap;

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::sites::{Realm, RealmEvent, RealmEventKind, SiteKind};
use crate::past::Topic;
use crate::rng::{purpose, stream};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// Remembered for nothing in particular yet: a grown member of a settlement people knew.
    Elder,
    Maker,
    Healer,
    Priest,
    Warrior,
    Wanderer,
}

impl Role {
    pub fn label(self) -> &'static str {
        match self {
            Role::Elder => "elder",
            Role::Maker => "maker",
            Role::Healer => "healer",
            Role::Priest => "priest",
            Role::Warrior => "warrior",
            Role::Wanderer => "wanderer",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Figure {
    pub id: u32,
    pub name: String,
    pub people: u32,
    /// Where they live now.
    pub site: u32,
    pub born_at: u32,
    pub born: i64,
    pub parent: Option<u32>,
    pub role: Role,
    pub died: Option<(i64, String)>,
    /// Whom they married (P9).
    #[serde(default)]
    pub partner: Option<u32>,
}

impl Figure {
    pub fn alive(&self) -> bool {
        self.died.is_none()
    }

    pub fn age(&self, year: i64) -> i64 {
        year - self.born
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Polity {
    pub id: u32,
    pub name: String,
    pub people: u32,
    pub seat: u32,
    pub founded: i64,
    pub ruler: Option<u32>,
    pub members: Vec<u32>,
    pub ended: Option<(i64, String)>,
}

/// How far from its seat a polity's reach extends, in regions.
pub const POLITY_REACH: i64 = 12;

fn dist2(realm: &Realm, size: usize, a: u32, b: u32) -> i64 {
    let (ra, rb) = (
        realm.sites[a as usize].region,
        realm.sites[b as usize].region,
    );
    let (ax, ay) = ((ra % size) as i64, (ra / size) as i64);
    let (bx, by) = ((rb % size) as i64, (rb / size) as i64);
    (ax - bx).pow(2) + (ay - by).pow(2)
}

fn push(out: &mut Vec<RealmEvent>, year: i64, kind: RealmEventKind) {
    out.push(RealmEvent { year, kind });
}

/// The year's figures and polities, after the settlements' own step. `first` is the index in
/// `out` where this year's settlement events begin.
pub fn step(realm: &mut Realm, size: usize, first: usize, out: &mut Vec<RealmEvent>) {
    let seed = realm.seed;
    let year = realm.year;
    let mut r = stream(seed, &[purpose::REALM_HISTORY, year as u64, 0x6669_6775]); // "figu"
    let this_year: Vec<RealmEventKind> = out[first..].iter().map(|e| e.kind.clone()).collect();
    // Living figures by where they live.
    let mut at: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for f in realm.figures.iter().filter(|f| f.alive()) {
        at.entry(f.site).or_default().push(f.id);
    }
    // What happened to settlements touches the people in them.
    for e in &this_year {
        match e {
            RealmEventKind::Founded {
                site,
                parent: Some(pa),
            } => {
                // A grown figure of the mother settlement leads them, if there is one.
                let leader = at.get(pa).and_then(|v| {
                    v.iter()
                        .copied()
                        .filter(|&f| {
                            let fig = &realm.figures[f as usize];
                            (18..50).contains(&fig.age(year))
                                && !realm.polities.iter().any(|p| p.ruler == Some(f))
                        })
                        .min_by_key(|&f| (r.gen::<u32>(), f))
                });
                if let Some(f) = leader {
                    realm.figures[f as usize].site = *site;
                    at.get_mut(pa).unwrap().retain(|x| *x != f);
                    at.entry(*site).or_default().push(f);
                    push(
                        out,
                        year,
                        RealmEventKind::Led {
                            figure: f,
                            site: *site,
                        },
                    );
                }
            }
            RealmEventKind::Famine { site, .. } | RealmEventKind::Plague { site, .. } => {
                let plague = matches!(e, RealmEventKind::Plague { .. });
                let chance = if plague { 0.3 } else { 0.15 };
                for f in at.get(site).cloned().unwrap_or_default() {
                    if r.gen::<f32>() < chance {
                        let cause = if plague { "plague" } else { "famine" };
                        die(realm, f, cause, out);
                        at.get_mut(site).unwrap().retain(|x| *x != f);
                    }
                }
            }
            RealmEventKind::Ended { site, .. } => {
                // Those still living there go to the settlement it came from, or die homeless.
                let parent = realm.sites[*site as usize]
                    .parent
                    .filter(|&p| realm.sites[p as usize].alive());
                for f in at.remove(site).unwrap_or_default() {
                    match parent {
                        Some(p) => {
                            realm.figures[f as usize].site = p;
                            at.entry(p).or_default().push(f);
                            push(out, year, RealmEventKind::Moved { figure: f, to: p });
                        }
                        None => die(realm, f, "lost when their home emptied", out),
                    }
                }
            }
            _ => {}
        }
    }
    // Age.
    for f in realm
        .figures
        .iter()
        .filter(|f| f.alive())
        .map(|f| f.id)
        .collect::<Vec<_>>()
    {
        let age = realm.figures[f as usize].age(year) as f32;
        if age > 45.0 {
            let p = (((age - 45.0) / 30.0).powi(3) * 0.2).min(0.6);
            if r.gen::<f32>() < p {
                let site = realm.figures[f as usize].site;
                die(realm, f, "old age", out);
                if let Some(v) = at.get_mut(&site) {
                    v.retain(|x| *x != f);
                }
            }
        }
    }
    // Births: a settlement brings forth people worth remembering in proportion to its size.
    for k in 0..realm.sites.len() {
        let s = &realm.sites[k];
        if !s.alive() || s.kind == SiteKind::Mine || s.population < 20.0 {
            continue;
        }
        let expect = s.population / 3000.0;
        if r.gen::<f32>() >= expect {
            continue;
        }
        let (site, mut people) = (s.id, s.people);
        // A parent among the figures there, but not one married to someone of a kind they
        // cannot have children with (P9).
        let species = |f: u32| realm.peoples[realm.figures[f as usize].people as usize].species;
        let parent = at.get(&site).and_then(|v| {
            v.iter()
                .copied()
                .filter(|&f| (18..46).contains(&realm.figures[f as usize].age(year)))
                .filter(|&f| {
                    realm.figures[f as usize]
                        .partner
                        .is_none_or(|o| crate::species::fertile(species(f), species(o)))
                })
                .min_by_key(|&f| (r.gen::<u32>(), f))
        });
        // The other parent, their partner while living; the child is one parent's people
        // whole, drawn even (decision 1).
        let other = parent
            .and_then(|f| realm.figures[f as usize].partner)
            .filter(|&o| realm.figures[o as usize].alive());
        if let (Some(f), Some(o)) = (parent, other) {
            let (pf, po) = (
                realm.figures[f as usize].people,
                realm.figures[o as usize].people,
            );
            people = if pf == po || r.gen_bool(0.5) { pf } else { po };
        }
        let p = &realm.peoples[people as usize];
        let violence = p.conviction(Topic::Violence);
        let roll: f32 = r.gen();
        let role = if roll < 0.32 {
            Role::Maker
        } else if roll < 0.42 {
            Role::Healer
        } else if roll < 0.52 {
            Role::Priest
        } else if roll < 0.62 + 0.1 * violence {
            Role::Warrior
        } else if roll < 0.72 {
            Role::Wanderer
        } else {
            Role::Elder
        };
        let id = realm.figures.len() as u32;
        let name = realm.unique_name(people, "person", id as u64);
        realm.figures.push(Figure {
            id,
            name,
            people,
            site,
            born_at: site,
            born: year,
            parent,
            role,
            died: None,
            partner: None,
        });
        at.entry(site).or_default().push(id);
        push(
            out,
            year,
            RealmEventKind::Born {
                figure: id,
                site,
                parent,
                other,
            },
        );
    }
    polities(realm, size, &this_year, &mut at, out);
}

fn die(realm: &mut Realm, f: u32, cause: &str, out: &mut Vec<RealmEvent>) {
    let year = realm.year;
    realm.figures[f as usize].died = Some((year, cause.to_string()));
    push(
        out,
        year,
        RealmEventKind::Died {
            figure: f,
            cause: cause.to_string(),
        },
    );
}

fn polities(
    realm: &mut Realm,
    size: usize,
    this_year: &[RealmEventKind],
    at: &mut BTreeMap<u32, Vec<u32>>,
    out: &mut Vec<RealmEvent>,
) {
    let year = realm.year;
    let seed = realm.seed;
    // A settlement that has just become a town, far from any realm of its people, is a seat.
    for e in this_year {
        if let RealmEventKind::Became {
            site,
            kind: SiteKind::Town,
        } = e
        {
            let people = realm.sites[*site as usize].people;
            let near = realm.polities.iter().any(|p| {
                p.ended.is_none()
                    && p.people == people
                    && dist2(realm, size, p.seat, *site) <= POLITY_REACH.pow(2)
            });
            if near
                || realm
                    .polities
                    .iter()
                    .any(|p| p.members.contains(site) && p.ended.is_none())
            {
                continue;
            }
            let id = realm.polities.len() as u32;
            let ruler = eldest(realm, at, *site);
            let name = realm.peoples[people as usize]
                .language
                .name(seed, "realm", id as u64);
            realm.polities.push(Polity {
                id,
                name,
                people,
                seat: *site,
                founded: year,
                ruler,
                members: vec![*site],
                ended: None,
            });
            push(
                out,
                year,
                RealmEventKind::PolityFounded {
                    polity: id,
                    seat: *site,
                    ruler,
                },
            );
        }
    }
    for k in 0..realm.polities.len() {
        if realm.polities[k].ended.is_some() {
            continue;
        }
        let (pid, seat, people) = (
            realm.polities[k].id,
            realm.polities[k].seat,
            realm.polities[k].people,
        );
        if !realm.sites[seat as usize].alive() {
            realm.polities[k].ended = Some((year, "its seat emptied".into()));
            push(
                out,
                year,
                RealmEventKind::PolityEnded {
                    polity: pid,
                    cause: "its seat emptied".into(),
                },
            );
            continue;
        }
        if realm.sites[seat as usize].people != people {
            let cause = "its seat was taken in war".to_string();
            realm.polities[k].ended = Some((year, cause.clone()));
            push(
                out,
                year,
                RealmEventKind::PolityEnded { polity: pid, cause },
            );
            continue;
        }
        realm.polities[k]
            .members
            .retain(|&s| realm.sites[s as usize].alive());
        // Settlements of its people within reach, in no other realm, join.
        let joining: Vec<u32> = realm
            .sites
            .iter()
            .filter(|s| {
                s.alive()
                    && s.people == people
                    && dist2(realm, size, seat, s.id) <= POLITY_REACH.pow(2)
                    && !realm
                        .polities
                        .iter()
                        .any(|p| p.ended.is_none() && p.members.contains(&s.id))
            })
            .map(|s| s.id)
            .collect();
        for s in joining {
            realm.polities[k].members.push(s);
            push(
                out,
                year,
                RealmEventKind::Joined {
                    polity: pid,
                    site: s,
                },
            );
        }
        // A ruler who has died is followed by a child of theirs at the seat, or the eldest there.
        let ruler_gone = realm.polities[k].ruler.is_none_or(|f| {
            !realm.figures[f as usize].alive() || realm.figures[f as usize].site != seat
        });
        if ruler_gone {
            let old = realm.polities[k].ruler;
            let heir = old
                .and_then(|o| {
                    at.get(&seat).and_then(|v| {
                        v.iter()
                            .copied()
                            .filter(|&f| {
                                realm.figures[f as usize].parent == Some(o)
                                    && realm.figures[f as usize].age(year) >= 16
                            })
                            .min_by_key(|&f| (realm.figures[f as usize].born, f))
                    })
                })
                .or_else(|| eldest(realm, at, seat));
            match heir {
                Some(h) if heir != old => {
                    realm.polities[k].ruler = heir;
                    push(
                        out,
                        year,
                        RealmEventKind::Succeeded {
                            polity: pid,
                            figure: h,
                        },
                    );
                }
                None if old.is_some() => realm.polities[k].ruler = None,
                _ => {}
            }
        }
    }
}

fn eldest(realm: &Realm, at: &BTreeMap<u32, Vec<u32>>, site: u32) -> Option<u32> {
    at.get(&site).and_then(|v| {
        v.iter()
            .copied()
            .filter(|&f| {
                realm.figures[f as usize].alive() && realm.figures[f as usize].age(realm.year) >= 16
            })
            .min_by_key(|&f| (realm.figures[f as usize].born, f))
    })
}
