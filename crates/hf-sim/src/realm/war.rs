//! Wars between realms. Two realms of different peoples that share a border may come to war:
//! more readily when both hold that force answers force, or when both have outgrown their
//! country. A war is fought by the year in battles at border settlements; a settlement can be
//! taken (its people change) or burned (it ends), warriors fall, and peace comes when a war is
//! spent or a realm falls. Every step is an event with its cause.

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::figures::Role;
use super::sites::{Realm, RealmEvent, RealmEventKind, SiteKind};
use crate::past::Topic;
use crate::rng::{purpose, stream};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct War {
    pub id: u32,
    pub a: u32,
    pub b: u32,
    pub began: i64,
    pub cause: String,
    pub battles: u32,
    pub ended: Option<(i64, String)>,
}

/// Regions between the nearest settlements of two realms.
fn border(realm: &Realm, size: usize, a: u32, b: u32) -> i64 {
    let pa = &realm.polities[a as usize];
    let pb = &realm.polities[b as usize];
    let at = |s: u32| {
        let r = realm.sites[s as usize].region;
        ((r % size) as i64, (r / size) as i64)
    };
    let mut best = i64::MAX;
    for &x in &pa.members {
        let (ax, ay) = at(x);
        for &y in &pb.members {
            let (bx, by) = at(y);
            best = best.min((ax - bx).abs().max((ay - by).abs()));
        }
    }
    best
}

fn crowded(realm: &Realm, p: u32) -> bool {
    let m = &realm.polities[p as usize].members;
    !m.is_empty()
        && m.iter()
            .filter(|&&s| realm.sites[s as usize].kind == SiteKind::Town)
            .count()
            * 3
            >= m.len()
}

fn push(out: &mut Vec<RealmEvent>, year: i64, kind: RealmEventKind) {
    out.push(RealmEvent { year, kind });
}

pub fn step(realm: &mut Realm, size: usize, out: &mut Vec<RealmEvent>) {
    let year = realm.year;
    let seed = realm.seed;
    let mut r = stream(seed, &[purpose::REALM_HISTORY, year as u64, 0x0077_6172]); // "war"
    let living: Vec<u32> = realm
        .polities
        .iter()
        .filter(|p| p.ended.is_none())
        .map(|p| p.id)
        .collect();
    // New wars.
    for (i, &a) in living.iter().enumerate() {
        for &b in &living[i + 1..] {
            let (pa, pb) = (
                realm.polities[a as usize].people,
                realm.polities[b as usize].people,
            );
            if pa == pb {
                continue;
            }
            let fighting_or_lately = realm.wars.iter().any(|w| {
                ((w.a == a && w.b == b) || (w.a == b && w.b == a))
                    && w.ended.as_ref().is_none_or(|(y, _)| year - y < 20)
            });
            let allied = realm
                .alliances
                .iter()
                .any(|x| x.ended.is_none() && ((x.a == a && x.b == b) || (x.a == b && x.b == a)));
            if fighting_or_lately || allied || border(realm, size, a, b) > 6 {
                continue;
            }
            let conv = |p: u32, t: Topic| realm.peoples[p as usize].conviction(t);
            let hard = (conv(pa, Topic::Violence) + conv(pb, Topic::Violence)) / 2.0
                + (conv(pa, Topic::Strangers) + conv(pb, Topic::Strangers)) / 4.0;
            let land = crowded(realm, a) && crowded(realm, b);
            // An old grudge (P8): a culture at either seat holding one against the other people.
            let held = |polity: u32, against: u32| -> Option<(f32, crate::ids::EventId)> {
                let seat = realm.polities[polity as usize].seat as usize;
                let c = realm.sites.get(seat)?.culture as usize;
                realm
                    .cultures
                    .get(c)?
                    .grudges
                    .iter()
                    .find(|g| g.against == against)
                    .map(|g| (g.strength, g.since))
            };
            let grudge = held(a, pb)
                .into_iter()
                .chain(held(b, pa))
                .max_by(|x, y| x.0.total_cmp(&y.0));
            // Dread of other kinds (P9): a seat's culture holding other races in dread, toward
            // a realm of another species.
            let seat_culture = |p: u32| {
                let seat = realm.polities[p as usize].seat as usize;
                realm.sites.get(seat).map(|s| s.culture as usize)
            };
            let dread = |p: u32| {
                seat_culture(p)
                    .and_then(|c| realm.cultures.get(c))
                    .is_some_and(|c| c.stance(Topic::OtherRaces) < -0.4)
            };
            let other_kind =
                realm.peoples[pa as usize].species != realm.peoples[pb as usize].species;
            // The deep and the land meet only where the caves open (P10): half as often.
            let layers = if realm.peoples[pa as usize].deep != realm.peoples[pb as usize].deep {
                0.5
            } else {
                1.0
            };
            let chance = 0.008 * (1.0 + 2.5 * hard.max(0.0))
                + if land { 0.012 } else { 0.0 }
                + 0.02 * grudge.map(|g| g.0).unwrap_or(0.0)
                + if other_kind && (dread(a) || dread(b)) {
                    0.006
                } else {
                    0.0
                };
            let chance = chance * layers;
            if r.gen::<f32>() >= chance {
                continue;
            }
            let remembered = grudge.filter(|g| g.0 > 0.3).and_then(|g| {
                realm
                    .stories
                    .iter()
                    .find(|s| s.about == g.1)
                    .and_then(|s| s.site)
                    .and_then(|s| realm.sites.get(s as usize))
                    .map(|s| s.name.clone())
            });
            let cause = if let Some(place) = remembered {
                format!("an old grudge over {place}")
            } else if land {
                "both had outgrown their country".to_string()
            } else if hard > 0.3 {
                "both held that force answers force".to_string()
            } else {
                "a quarrel over the border".to_string()
            };
            let id = realm.wars.len() as u32;
            realm.wars.push(War {
                id,
                a,
                b,
                began: year,
                cause: cause.clone(),
                battles: 0,
                ended: None,
            });
            push(
                out,
                year,
                RealmEventKind::WarBegan {
                    war: id,
                    a,
                    b,
                    cause,
                },
            );
        }
    }
    // Battles, and peace.
    for k in 0..realm.wars.len() {
        if realm.wars[k].ended.is_some() {
            continue;
        }
        let (wid, a, b) = (realm.wars[k].id, realm.wars[k].a, realm.wars[k].b);
        let fallen = [a, b]
            .into_iter()
            .find(|&p| realm.polities[p as usize].ended.is_some());
        if let Some(p) = fallen {
            let cause = format!("the realm of {} fell", realm.polities[p as usize].name);
            realm.wars[k].ended = Some((year, cause.clone()));
            push(out, year, RealmEventKind::WarEnded { war: wid, cause });
            continue;
        }
        if realm.wars[k].battles >= 3 && r.gen::<f32>() < 0.18 {
            realm.wars[k].ended = Some((year, "peace was made".into()));
            push(
                out,
                year,
                RealmEventKind::WarEnded {
                    war: wid,
                    cause: "peace was made".into(),
                },
            );
            continue;
        }
        let (att, def) = if r.gen_bool(0.5) { (a, b) } else { (b, a) };
        // The defender's settlement nearest the attacker's land.
        let target = {
            let at = |s: u32| {
                let rg = realm.sites[s as usize].region;
                ((rg % size) as i64, (rg / size) as i64)
            };
            let am = &realm.polities[att as usize].members;
            realm.polities[def as usize]
                .members
                .iter()
                .copied()
                .filter(|&s| {
                    realm.sites[s as usize].alive() && s != realm.polities[def as usize].seat
                })
                .min_by_key(|&s| {
                    let (x, y) = at(s);
                    (
                        am.iter()
                            .map(|&o| {
                                let (ox, oy) = at(o);
                                (x - ox).abs().max((y - oy).abs())
                            })
                            .min()
                            .unwrap_or(i64::MAX),
                        s,
                    )
                })
                .or(Some(realm.polities[def as usize].seat))
        };
        let Some(site) = target else { continue };
        realm.wars[k].battles += 1;
        let violence = |p: u32| {
            realm.peoples[realm.polities[p as usize].people as usize].conviction(Topic::Violence)
        };
        let might = |p: u32| {
            realm.polities[p as usize]
                .members
                .iter()
                .map(|&s| realm.sites[s as usize].population)
                .sum::<f32>()
                .sqrt()
        };
        // Allies against the other side fight beside each (P9), at half their strength.
        let att_allies = super::relations::allies_against(realm, att, def);
        let def_allies = super::relations::allies_against(realm, def, att);
        let help = |v: &[u32]| v.iter().map(|&q| might(q) * 0.5).sum::<f32>();
        let a_str =
            (might(att) + help(&att_allies)) * (1.0 + 0.4 * violence(att)) * r.gen_range(0.6..1.4);
        let d_str = (might(def) + help(&def_allies))
            * (1.0 + 0.4 * violence(def))
            * r.gen_range(0.6..1.4)
            * if realm.sites[site as usize].kind == SiteKind::Town {
                1.3
            } else {
                1.0
            };
        let won_by = if a_str > d_str { att } else { def };
        push(
            out,
            year,
            RealmEventKind::Battle {
                war: wid,
                site,
                won_by,
                with: att_allies
                    .iter()
                    .chain(def_allies.iter())
                    .copied()
                    .collect(),
            },
        );
        // Warriors of the place and of the attackers' nearest settlements may fall.
        let place = realm.sites[site as usize].name.clone();
        let dead: Vec<u32> = realm
            .figures
            .iter()
            .filter(|f| {
                f.alive()
                    && f.role == Role::Warrior
                    && (f.site == site || realm.polities[att as usize].members.contains(&f.site))
                    && year - f.born >= 16
            })
            .map(|f| f.id)
            .collect();
        for f in dead {
            if r.gen::<f32>() < 0.12 {
                realm.figures[f as usize].died = Some((year, format!("fell at {place}")));
                push(
                    out,
                    year,
                    RealmEventKind::Died {
                        figure: f,
                        // One tally key (#223: 704 "fell at <place>" keys); the figure's own
                        // record above keeps the place.
                        cause: "fell in battle".to_string(),
                    },
                );
            }
        }
        if won_by != att {
            continue;
        }
        let s = &mut realm.sites[site as usize];
        s.population *= r.gen_range(0.5..0.8);
        // Nobody holds a settlement in a layer they cannot live in (P10): the deep's peoples
        // raid the land and the land's raid the deep, but neither takes the other's homes.
        let same_layer = s.deep == realm.peoples[realm.polities[att as usize].people as usize].deep;
        let take = same_layer && s.population >= 15.0 && r.gen::<f32>() < 0.5;
        if take {
            // Taken: it is the attackers' now, and their ways with it (P9); some of its old
            // people flee rather than live under them.
            let (old_people, pop) = (s.people, s.population);
            let new_people = realm.polities[att as usize].people;
            s.people = new_people;
            let seat = realm.polities[att as usize].seat as usize;
            realm.sites[site as usize].culture = realm.sites[seat].culture;
            realm.polities[def as usize].members.retain(|&x| x != site);
            realm.polities[att as usize].members.push(site);
            push(out, year, RealmEventKind::Taken { site, by: att });
            let cause = format!(
                "their home was taken by {}",
                realm.polities[att as usize].name
            );
            realm
                .fleeing
                .push((site, old_people, (pop * 0.2) as u32, cause));
            // A realm whose seat is taken ends there and then (retaken later, it is the
            // takers' settlement that changes hands, not the old realm come back).
            if realm.polities[def as usize].seat == site {
                let cause = format!(
                    "its seat was taken by {}",
                    realm.polities[att as usize].name
                );
                realm.polities[def as usize].ended = Some((year, cause.clone()));
                push(
                    out,
                    year,
                    RealmEventKind::PolityEnded { polity: def, cause },
                );
            }
        } else if s.population < 15.0 || r.gen::<f32>() < 0.3 {
            let cause = format!(
                "burned in the war between {} and {}",
                realm.polities[a as usize].name, realm.polities[b as usize].name
            );
            let sid = s.id;
            realm.end_site(sid, &cause, out);
        }
    }
}
