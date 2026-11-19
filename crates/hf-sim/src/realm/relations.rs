//! Relations between the world's peoples (P9 of milestone MP, docs/design-peoples.md section
//! 7.1), built only from what happens: settlements that make different things send caravans to
//! each other, and the decade's caravans between two of them are one row; figures of two
//! cultures whose settlements trade marry; two realms at war with the same third and trading
//! make common cause; and when a famine, a plague or a war empties homes, people flee to the
//! nearest settlement that will have them, so peoples come to live among each other.

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::map::RealmMap;
use super::peoples::Living;
use super::sites::{Realm, RealmEvent, RealmEventKind, SiteKind};
use crate::past::Topic;
use crate::rng::{purpose, stream};

/// Between two cultures (`a` < `b`), from the rows.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Relation {
    pub a: u32,
    pub b: u32,
    /// Caravans between their settlements, all told.
    pub trade: u32,
    pub last_trade: i64,
    /// Figures married across.
    pub marriages: u32,
}

/// Caravans between two settlements (`a` < `b`) this decade.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Route {
    pub a: u32,
    pub b: u32,
    pub caravans: u32,
    /// The decade's caravans each way.
    pub from_a: u32,
    pub from_b: u32,
    pub last: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Alliance {
    pub id: u32,
    pub a: u32,
    pub b: u32,
    pub against: u32,
    pub since: i64,
    pub ended: Option<(i64, String)>,
}

/// How far, in regions, a caravan goes.
pub const TRADE_REACH: i64 = 12;
/// How far, in regions, people flee.
pub const FLIGHT_REACH: i64 = 8;

/// What a settlement has to trade: a mine its ore, a town what its makers make, others what
/// their people's living gives.
pub fn goods(realm: &Realm, site: u32) -> &'static str {
    let s = &realm.sites[site as usize];
    if s.deep {
        return "worked stone and fungus";
    }
    match s.kind {
        SiteKind::Mine => "ore",
        SiteKind::Town => "tools and cloth",
        _ => match realm.peoples.get(s.people as usize).map(|p| p.living) {
            Some(Living::Herding) => "hides and wool",
            Some(Living::Hunting) => "furs and meat",
            Some(Living::Fishing) => "fish",
            _ => "grain",
        },
    }
}

fn dist(realm: &Realm, a: u32, b: u32) -> i64 {
    let n = realm.map_size.max(1);
    let (ra, rb) = (
        realm.sites[a as usize].region,
        realm.sites[b as usize].region,
    );
    ((ra % n) as i64 - (rb % n) as i64)
        .abs()
        .max(((ra / n) as i64 - (rb / n) as i64).abs())
}

/// The pairs of realms at war now, both ways round, for many lookups in one year.
fn wars_now(realm: &Realm) -> std::collections::BTreeSet<(u32, u32)> {
    realm
        .wars
        .iter()
        .filter(|w| w.ended.is_none())
        .flat_map(|w| [(w.a, w.b), (w.b, w.a)])
        .collect()
}

/// Whether two realms are at war now.
pub fn at_war(realm: &Realm, a: u32, b: u32) -> bool {
    realm
        .wars
        .iter()
        .any(|w| w.ended.is_none() && ((w.a == a && w.b == b) || (w.a == b && w.b == a)))
}

fn stance(realm: &Realm, culture: u32, t: Topic) -> f32 {
    realm
        .cultures
        .get(culture as usize)
        .map(|c| c.stance(t))
        .unwrap_or(0.0)
}

/// Whether the people of settlement `host` will deal with those of `guest`: always their own
/// people; another people unless the host's culture holds other races in dread.
fn will_deal(realm: &Realm, host: u32, guest: u32) -> bool {
    let (h, g) = (&realm.sites[host as usize], &realm.sites[guest as usize]);
    h.people == g.people || stance(realm, h.culture, Topic::OtherRaces) > -0.4
}

/// The relation between two cultures, made when they first meet.
pub fn relation_mut(realm: &mut Realm, x: u32, y: u32) -> &mut Relation {
    let (a, b) = (x.min(y), x.max(y));
    let k = match realm.relations.iter().position(|r| r.a == a && r.b == b) {
        Some(k) => k,
        None => {
            realm.relations.push(Relation {
                a,
                b,
                ..Default::default()
            });
            realm.relations.len() - 1
        }
    };
    &mut realm.relations[k]
}

pub fn relation(realm: &Realm, x: u32, y: u32) -> Option<&Relation> {
    let (a, b) = (x.min(y), x.max(y));
    realm.relations.iter().find(|r| r.a == a && r.b == b)
}

/// One year of relations, after the settlements, figures and wars. `first` is where this
/// year's rows begin in `out`.
pub fn step(realm: &mut Realm, m: &RealmMap, first: usize, out: &mut Vec<RealmEvent>) {
    let this_year: Vec<RealmEventKind> = out[first..].iter().map(|e| e.kind.clone()).collect();
    for (from, people, count, cause) in std::mem::take(&mut realm.fleeing) {
        flee(realm, m, from, people, count, &cause, out);
    }
    refugees(realm, m, &this_year, out);
    trade(realm, out);
    marriages(realm, out);
    alliances(realm, out);
}

fn push(out: &mut Vec<RealmEvent>, year: i64, kind: RealmEventKind) {
    out.push(RealmEvent { year, kind });
}

/// Every settlement with something to spare sends one caravan a year to the nearest within
/// reach that makes something else, will deal with it, and whose realm is not at war with
/// its own. Caravans between settlements of one culture are its own business; those between
/// two cultures are kept as routes, and every ten years each pair of cultures' caravans are
/// one row (when there were ten or more), naming their busiest route.
fn trade(realm: &mut Realm, out: &mut Vec<RealmEvent>) {
    let year = realm.year;
    let n = realm.map_size.max(1);
    let living: Vec<u32> = realm
        .sites
        .iter()
        .filter(|s| s.alive() && (s.kind == SiteKind::Mine || s.population >= 60.0))
        .map(|s| s.id)
        .collect();
    let mut member: std::collections::BTreeMap<u32, u32> = std::collections::BTreeMap::new();
    for p in realm.polities.iter().filter(|p| p.ended.is_none()) {
        for &s in &p.members {
            member.insert(s, p.id);
        }
    }
    let fighting = wars_now(realm);
    // Settlements by squares of four regions, searched ring by ring outward from each, stopping
    // once nothing nearer than the best found can lie further out.
    const CELL: i64 = 4;
    let at = |s: u32| {
        let r = realm.sites[s as usize].region;
        ((r % n) as i64, (r / n) as i64)
    };
    let mut grid: std::collections::BTreeMap<(i64, i64), Vec<u32>> = Default::default();
    for &s in &living {
        let (x, y) = at(s);
        grid.entry((x / CELL, y / CELL)).or_default().push(s);
    }
    let mut sent: Vec<(u32, u32)> = Vec::new();
    for &s in &living {
        let mine = goods(realm, s);
        let (x, y) = at(s);
        let (cx, cy) = (x / CELL, y / CELL);
        let mut best: Option<(i64, u32)> = None;
        for ring in 0..=(TRADE_REACH / CELL + 1) {
            // Everything in ring `ring` or beyond is at least (ring - 1) * CELL + 1 away.
            if best.is_some_and(|b| b.0 <= (ring - 1) * CELL) {
                break;
            }
            for gy in cy - ring..=cy + ring {
                for gx in cx - ring..=cx + ring {
                    if (gx - cx).abs().max((gy - cy).abs()) != ring {
                        continue;
                    }
                    for &o in grid.get(&(gx, gy)).map(|v| v.as_slice()).unwrap_or(&[]) {
                        if o == s || goods(realm, o) == mine {
                            continue;
                        }
                        let d = dist(realm, s, o);
                        if d > TRADE_REACH || best.is_some_and(|b| (d, o) >= b) {
                            continue;
                        }
                        let war = match (member.get(&s), member.get(&o)) {
                            (Some(&x), Some(&y)) => fighting.contains(&(x, y)),
                            _ => false,
                        };
                        if war || !will_deal(realm, o, s) || !will_deal(realm, s, o) {
                            continue;
                        }
                        best = Some((d, o));
                    }
                }
            }
        }
        if let Some((_, o)) = best {
            sent.push((s, o));
        }
    }
    // Where each route and relation sits, looked up once.
    let mut route_at: std::collections::BTreeMap<(u32, u32), usize> = realm
        .routes
        .iter()
        .enumerate()
        .map(|(k, r)| ((r.a, r.b), k))
        .collect();
    let mut rel_at: std::collections::BTreeMap<(u32, u32), usize> = realm
        .relations
        .iter()
        .enumerate()
        .map(|(k, r)| ((r.a, r.b), k))
        .collect();
    for (s, o) in sent {
        let (cs, co) = (
            realm.sites[s as usize].culture,
            realm.sites[o as usize].culture,
        );
        if cs == co {
            continue;
        }
        let key = (cs.min(co), cs.max(co));
        let k = *rel_at.entry(key).or_insert_with(|| {
            realm.relations.push(Relation {
                a: key.0,
                b: key.1,
                ..Default::default()
            });
            realm.relations.len() - 1
        });
        let rel = &mut realm.relations[k];
        rel.trade += 1;
        rel.last_trade = year;
        let (a, b) = (s.min(o), s.max(o));
        let k = match route_at.get(&(a, b)).copied() {
            Some(k) => k,
            None => {
                route_at.insert((a, b), realm.routes.len());
                realm.routes.push(Route {
                    a,
                    b,
                    caravans: 0,
                    from_a: 0,
                    from_b: 0,
                    last: year,
                });
                realm.routes.len() - 1
            }
        };
        let r = &mut realm.routes[k];
        r.caravans += 1;
        if s == a {
            r.from_a += 1;
        } else {
            r.from_b += 1;
        }
        r.last = year;
    }
    if year % 10 == 0 {
        // Per pair of cultures: the decade's caravans and the busiest route.
        let mut pairs: std::collections::BTreeMap<(u32, u32), (u32, u32)> = Default::default();
        for (k, r) in realm.routes.iter().enumerate() {
            if r.caravans == 0 {
                continue;
            }
            let (ca, cb) = (
                realm.sites[r.a as usize].culture,
                realm.sites[r.b as usize].culture,
            );
            // (A settlement taken since may hold its partner's ways now.)
            if ca == cb {
                continue;
            }
            let e = pairs
                .entry((ca.min(cb), ca.max(cb)))
                .or_insert((0, k as u32));
            e.0 += r.caravans;
            if r.caravans > realm.routes[e.1 as usize].caravans {
                e.1 = k as u32;
            }
        }
        // Only a road walked every year or so is history; the odd caravan is not.
        for ((ca, cb), (caravans, k)) in pairs.into_iter().filter(|(_, (c, _))| *c >= 10) {
            let r = &realm.routes[k as usize];
            let goods = format!("{} for {}", goods(realm, r.a), goods(realm, r.b));
            push(
                out,
                year,
                RealmEventKind::Traded {
                    a: r.a,
                    b: r.b,
                    caravans,
                    goods,
                    cultures: (ca, cb),
                },
            );
        }
        for r in realm.routes.iter_mut() {
            r.caravans = 0;
            r.from_a = 0;
            r.from_b = 0;
        }
        // A route nobody has walked in thirty years is forgotten.
        realm.routes.retain(|r| year - r.last <= 30);
    }
}

/// Along a route walked this decade between two cultures, a grown unmarried figure of each
/// may marry, when both cultures are easy with other kinds (or they are one people). The one
/// from `b`'s settlement goes to live at `a`'s.
fn marriages(realm: &mut Realm, out: &mut Vec<RealmEvent>) {
    let year = realm.year;
    let mut r = stream(
        realm.seed,
        &[purpose::REALM_HISTORY, year as u64, 0x6d61_7272], // "marr"
    );
    let routes: Vec<(u32, u32)> = realm
        .routes
        .iter()
        .filter(|x| year - x.last < 10)
        .map(|x| (x.a, x.b))
        .collect();
    // Unmarried grown figures by where they live, made once a year when first wanted (a scan
    // of every figure for each route made the thousand-year history twice as slow).
    let mut singles: Option<std::collections::BTreeMap<u32, Vec<u32>>> = None;
    for (sa, sb) in routes {
        let (ca, cb) = (
            realm.sites[sa as usize].culture,
            realm.sites[sb as usize].culture,
        );
        // (Nobody goes to live in a layer they cannot live in, P10.)
        if ca == cb
            || !realm.sites[sa as usize].alive()
            || !realm.sites[sb as usize].alive()
            || realm.sites[sa as usize].deep != realm.sites[sb as usize].deep
        {
            continue;
        }
        let same_people = realm.sites[sa as usize].people == realm.sites[sb as usize].people;
        if !same_people
            && (stance(realm, ca, Topic::OtherRaces) <= 0.0
                || stance(realm, cb, Topic::OtherRaces) <= 0.0)
        {
            continue;
        }
        if r.gen::<f32>() >= 0.08 {
            continue;
        }
        let index = singles.get_or_insert_with(|| {
            let mut m: std::collections::BTreeMap<u32, Vec<u32>> = Default::default();
            for f in realm
                .figures
                .iter()
                .filter(|f| f.alive() && f.partner.is_none() && (18..46).contains(&f.age(year)))
            {
                m.entry(f.site).or_default().push(f.id);
            }
            m
        });
        let seed = realm.seed;
        let mut single = |site: u32| -> Option<u32> {
            let v = index.get_mut(&site)?;
            let k = (0..v.len())
                .min_by_key(|&k| (crate::rng::hash64(&[seed, year as u64, v[k] as u64]), v[k]))?;
            Some(v.swap_remove(k))
        };
        let (Some(fa), Some(fb)) = (single(sa), single(sb)) else {
            continue;
        };
        realm.figures[fa as usize].partner = Some(fb);
        realm.figures[fb as usize].partner = Some(fa);
        realm.figures[fb as usize].site = sa;
        relation_mut(realm, ca, cb).marriages += 1;
        push(
            out,
            year,
            RealmEventKind::Married {
                a: fa,
                b: fb,
                at: sa,
            },
        );
    }
}

/// Two realms at war with the same third, whose seats' cultures have traded and hold no
/// grudge against each other's people, may make common cause. An alliance ends with either
/// realm.
fn alliances(realm: &mut Realm, out: &mut Vec<RealmEvent>) {
    let year = realm.year;
    for k in 0..realm.alliances.len() {
        if realm.alliances[k].ended.is_some() {
            continue;
        }
        let (a, b) = (realm.alliances[k].a, realm.alliances[k].b);
        let gone = [a, b]
            .into_iter()
            .find(|&p| realm.polities[p as usize].ended.is_some());
        if let Some(p) = gone {
            let cause = format!("the realm of {} fell", realm.polities[p as usize].name);
            realm.alliances[k].ended = Some((year, cause));
        }
    }
    let mut r = stream(
        realm.seed,
        &[purpose::REALM_HISTORY, year as u64, 0x616c_6c79], // "ally"
    );
    let living: Vec<u32> = realm
        .polities
        .iter()
        .filter(|p| p.ended.is_none())
        .map(|p| p.id)
        .collect();
    let fighting = wars_now(realm);
    let enemies = |p: u32| -> Vec<u32> {
        fighting
            .range((p, 0)..=(p, u32::MAX))
            .map(|&(_, q)| q)
            .collect()
    };
    let mut made: Vec<(u32, u32, u32)> = Vec::new();
    for (i, &a) in living.iter().enumerate() {
        let ea = enemies(a);
        if ea.is_empty() {
            continue;
        }
        for &b in &living[i + 1..] {
            if fighting.contains(&(a, b))
                || realm.alliances.iter().any(|x| {
                    x.ended.is_none() && ((x.a == a && x.b == b) || (x.a == b && x.b == a))
                })
            {
                continue;
            }
            let Some(&against) = enemies(b).iter().find(|e| ea.contains(e)) else {
                continue;
            };
            let seat_culture =
                |p: u32| realm.sites[realm.polities[p as usize].seat as usize].culture;
            let (ca, cb) = (seat_culture(a), seat_culture(b));
            let traded = ca == cb || relation(realm, ca, cb).is_some_and(|x| x.trade > 0);
            let people = |p: u32| realm.polities[p as usize].people;
            let grudge = |c: u32, against: u32| {
                realm.cultures[c as usize]
                    .grudges
                    .iter()
                    .filter(|g| g.against == against)
                    .map(|g| g.strength)
                    .fold(0.0, f32::max)
            };
            if !traded || grudge(ca, people(b)) >= 0.2 || grudge(cb, people(a)) >= 0.2 {
                continue;
            }
            if r.gen::<f32>() < 0.1 {
                made.push((a, b, against));
            }
        }
    }
    for (a, b, against) in made {
        // Each realm in one new alliance a year.
        if realm
            .alliances
            .iter()
            .any(|x| x.since == year && [x.a, x.b].iter().any(|p| *p == a || *p == b))
        {
            continue;
        }
        let id = realm.alliances.len() as u32;
        realm.alliances.push(Alliance {
            id,
            a,
            b,
            against,
            since: year,
            ended: None,
        });
        push(
            out,
            year,
            RealmEventKind::Allied {
                alliance: id,
                a,
                b,
                against,
            },
        );
    }
}

/// Realms allied with `p` against `enemy` now.
pub fn allies_against(realm: &Realm, p: u32, enemy: u32) -> Vec<u32> {
    realm
        .alliances
        .iter()
        .filter(|x| x.ended.is_none() && x.against == enemy && (x.a == p || x.b == p))
        .map(|x| if x.a == p { x.b } else { x.a })
        .filter(|&q| realm.polities[q as usize].ended.is_none())
        .collect()
}

/// After a great famine or plague, or a settlement burned this year, some of its people flee
/// (a settlement taken in war sends its people away from war.rs, before it changes hands).
fn refugees(
    realm: &mut Realm,
    m: &RealmMap,
    this_year: &[RealmEventKind],
    out: &mut Vec<RealmEvent>,
) {
    for e in this_year {
        let (from, count, cause) = match e {
            // A hard year sends a few to kin; only a great famine or plague sends people
            // out to strangers.
            RealmEventKind::Famine { site, lost } if *lost >= 100 => {
                (*site, lost / 3, "famine".to_string())
            }
            RealmEventKind::Plague { site, lost } if *lost >= 100 => {
                (*site, lost / 4, "plague".to_string())
            }
            RealmEventKind::Ended { site, cause } if cause.starts_with("burned") => (
                *site,
                6 + (realm.sites[*site as usize].peak / 40.0) as u32,
                cause.clone(),
            ),
            _ => continue,
        };
        let people = realm.sites[from as usize].people;
        flee(realm, m, from, people, count, &cause, out);
    }
}

/// `count` of `people` flee settlement `from` for the nearest living settlement within reach
/// that will take them and has room, their own people's first; it takes as many as its land
/// will feed, and the rest scatter. Among another people they live as households of their own.
/// A row either way; fewer than three is no flight.
pub fn flee(
    realm: &mut Realm,
    m: &RealmMap,
    from: u32,
    people: u32,
    count: u32,
    cause: &str,
    out: &mut Vec<RealmEvent>,
) {
    if count < 3 {
        return;
    }
    let year = realm.year;
    let room = |s: &super::sites::RSite| {
        (super::sites::capacity_of(m, s.region, s.deep) * 3.0 - s.population).max(0.0) as u32
    };
    let deep = realm.sites[from as usize].deep;
    let to = realm
        .sites
        .iter()
        .filter(|s| {
            s.alive()
                && s.id != from
                && s.kind != SiteKind::Mine
                && s.deep == deep
                && dist(realm, from, s.id) <= FLIGHT_REACH
                && room(s) >= 3
                && (s.people == people || stance(realm, s.culture, Topic::OtherRaces) > -0.4)
        })
        .min_by_key(|s| (s.people != people, dist(realm, from, s.id), s.id))
        .map(|s| s.id);
    let count = count.min(60);
    let taken = to
        .map(|t| count.min(room(&realm.sites[t as usize])))
        .unwrap_or(0);
    let s = &mut realm.sites[from as usize];
    s.population = (s.population - count as f32).max(0.0);
    if let Some(t) = to {
        let host = &mut realm.sites[t as usize];
        host.population += taken as f32;
        if host.people != people && host.minority.is_none() {
            host.minority = Some(people);
        }
    }
    let count = if to.is_some() { taken } else { count };
    push(
        out,
        year,
        RealmEventKind::Fled {
            from,
            to,
            people,
            count,
            cause: cause.to_string(),
        },
    );
}
