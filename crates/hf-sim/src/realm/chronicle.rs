//! The realm's history in the chronicle: kind names, the things each event references, a line
//! of narration, and the mapping from the realm's years to ticks before the arrival.

use super::sites::{Realm, RealmEvent, RealmEventKind};
use crate::events::{EventKind, Severity};
use crate::time::TICKS_PER_YEAR;

pub fn kind_name(what: &RealmEventKind) -> &'static str {
    use RealmEventKind::*;
    match what {
        Founded { .. } => "realm_founded",
        Became { .. } => "realm_became",
        Famine { .. } => "realm_famine",
        Plague { .. } => "realm_plague",
        Ended { .. } => "realm_ended",
        Born { .. } => "realm_born",
        Died { .. } => "realm_died",
        Led { .. } => "realm_led",
        Moved { .. } => "realm_moved",
        PolityFounded { .. } => "realm_polity_founded",
        Joined { .. } => "realm_joined",
        Succeeded { .. } => "realm_succeeded",
        PolityEnded { .. } => "realm_polity_ended",
        WarBegan { .. } => "realm_war_began",
        Battle { .. } => "realm_battle",
        Taken { .. } => "realm_taken",
        WarEnded { .. } => "realm_war_ended",
        CultureSplit { .. } => "realm_culture_split",
        Traded { .. } => "realm_traded",
        Married { .. } => "realm_married",
        Allied { .. } => "realm_allied",
        Fled { .. } => "realm_fled",
        BeastWoke { .. } => "realm_beast_woke",
        BeastAttacked { .. } => "realm_beast_attacked",
    }
}

/// (kind, id) of everything an event touches, for entity_ref rows.
pub fn refs(what: &RealmEventKind) -> Vec<(&'static str, u64)> {
    use RealmEventKind::*;
    let s = |id: &u32| ("site", *id as u64);
    let f = |id: &u32| ("figure", *id as u64);
    let p = |id: &u32| ("polity", *id as u64);
    match what {
        Founded { site, parent } => std::iter::once(s(site))
            .chain(parent.iter().map(s))
            .collect(),
        Became { site, .. } | Famine { site, .. } | Plague { site, .. } | Ended { site, .. } => {
            vec![s(site)]
        }
        Born {
            figure,
            site,
            parent,
            other,
        } => {
            let mut v = vec![f(figure), s(site)];
            v.extend(parent.iter().map(f));
            v.extend(other.iter().map(f));
            v
        }
        Died { figure, .. } => vec![f(figure)],
        Led { figure, site } => vec![f(figure), s(site)],
        Moved { figure, to } => vec![f(figure), s(to)],
        PolityFounded {
            polity,
            seat,
            ruler,
        } => {
            let mut v = vec![p(polity), s(seat)];
            v.extend(ruler.iter().map(f));
            v
        }
        Joined { polity, site } => vec![p(polity), s(site)],
        Succeeded { polity, figure } => vec![p(polity), f(figure)],
        PolityEnded { polity, .. } => vec![p(polity)],
        WarBegan { war, a, b, .. } => vec![("war", *war as u64), p(a), p(b)],
        Battle {
            war,
            site,
            won_by,
            with,
        } => {
            let mut v = vec![("war", *war as u64), s(site), p(won_by)];
            v.extend(with.iter().map(p));
            v
        }
        Taken { site, by } => vec![s(site), p(by)],
        WarEnded { war, .. } => vec![("war", *war as u64)],
        CultureSplit { culture, site, .. } => vec![("culture", *culture as u64), s(site)],
        Traded {
            a,
            b,
            cultures: (ca, cb),
            ..
        } => vec![s(a), s(b), ("culture", *ca as u64), ("culture", *cb as u64)],
        Married { a, b, at } => vec![f(a), f(b), s(at)],
        Allied { a, b, against, .. } => vec![p(a), p(b), p(against)],
        Fled { from, to, .. } => std::iter::once(s(from)).chain(to.iter().map(s)).collect(),
        BeastWoke { beast, .. } => vec![("beast", *beast as u64)],
        BeastAttacked { beast, site, .. } => vec![("beast", *beast as u64), s(site)],
    }
}

pub fn severity(what: &RealmEventKind) -> Severity {
    use RealmEventKind::*;
    match what {
        PolityFounded { .. } | PolityEnded { .. } | WarBegan { .. } | WarEnded { .. } => {
            Severity::Major
        }
        Battle { .. } | Taken { .. } => Severity::Notable,
        Founded { .. }
        | Ended { .. }
        | Plague { .. }
        | Succeeded { .. }
        | CultureSplit { .. }
        | Allied { .. }
        | Fled { .. }
        | BeastWoke { .. }
        | BeastAttacked { .. } => Severity::Notable,
        _ => Severity::Minor,
    }
}

/// The realm's year as a tick: the realm's last year is the one before the arrival at `epoch`.
pub fn tick_of(year: i64, last_year: i64, epoch: u64) -> u64 {
    let back = (last_year - year + 1).max(0) as u64;
    epoch.saturating_sub(back * TICKS_PER_YEAR)
}

/// The realm's events as (tick, severity, kind) for the event sink.
pub fn to_events(
    events: &[RealmEvent],
    last_year: i64,
    epoch: u64,
) -> Vec<(u64, Severity, EventKind)> {
    events
        .iter()
        .map(|e| {
            (
                tick_of(e.year, last_year, epoch),
                severity(&e.kind),
                EventKind::Realm {
                    what: e.kind.clone(),
                },
            )
        })
        .collect()
}

fn site(r: &Realm, id: u32) -> String {
    r.sites
        .get(id as usize)
        .map(|s| s.name.clone())
        .unwrap_or_else(|| "a place".into())
}

fn figure(r: &Realm, id: u32) -> String {
    r.figures
        .get(id as usize)
        .map(|f| f.name.clone())
        .unwrap_or_else(|| "someone".into())
}

fn polity(r: &Realm, id: u32) -> String {
    r.polities
        .get(id as usize)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "a realm".into())
}

fn people_of_site(r: &Realm, id: u32) -> String {
    r.sites
        .get(id as usize)
        .and_then(|s| r.peoples.get(s.people as usize))
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "people".into())
}

/// One line for the feed and the history view.
pub fn line(r: &Realm, what: &RealmEventKind) -> String {
    use RealmEventKind::*;
    match what {
        Founded {
            site: s,
            parent: Some(p),
        } => format!("Settlers from {} founded {}.", site(r, *p), site(r, *s)),
        Founded {
            site: s,
            parent: None,
        } => {
            // #1322: "first home" only for a people's actual first site; a later valley
            // hamlet (also founded with no parent) is just founded.
            let first = r
                .sites
                .get(*s as usize)
                .is_some_and(|site| !r.sites.iter().any(|o| o.people == site.people && o.id < *s));
            if first {
                format!(
                    "The {} made their first home at {}.",
                    people_of_site(r, *s),
                    site(r, *s)
                )
            } else {
                format!("The {} founded {}.", people_of_site(r, *s), site(r, *s))
            }
        }
        Became { site: s, kind } => format!("{} grew into a {}.", site(r, *s), kind.label()),
        Famine { site: s, lost } => format!("Famine at {}: {lost} died or went away.", site(r, *s)),
        Plague { site: s, lost } => format!("Plague at {} took {lost}.", site(r, *s)),
        Ended { site: s, cause } => format!("{} was left empty: {cause}.", site(r, *s)),
        Born {
            figure: f,
            site: s,
            parent: Some(p),
            other: Some(o),
        } => {
            let of = |x: u32| {
                r.figures
                    .get(x as usize)
                    .and_then(|fig| r.peoples.get(fig.people as usize))
                    .map(|pe| pe.name.clone())
                    .unwrap_or_default()
            };
            if of(*p) != of(*o) {
                format!(
                    "{} was born at {}, child of {} of the {} and {} of the {}.",
                    figure(r, *f),
                    site(r, *s),
                    figure(r, *p),
                    of(*p),
                    figure(r, *o),
                    of(*o)
                )
            } else {
                format!(
                    "{} was born at {}, child of {} and {}.",
                    figure(r, *f),
                    site(r, *s),
                    figure(r, *p),
                    figure(r, *o)
                )
            }
        }
        Born {
            figure: f,
            site: s,
            parent: Some(p),
            ..
        } => format!(
            "{} was born at {}, child of {}.",
            figure(r, *f),
            site(r, *s),
            figure(r, *p)
        ),
        Born {
            figure: f,
            site: s,
            parent: None,
            ..
        } => format!("{} was born at {}.", figure(r, *f), site(r, *s)),
        Died { figure: f, cause } => format!("{} died ({cause}).", figure(r, *f)),
        Led { figure: f, site: s } => format!(
            "{} led the settlers who founded {}.",
            figure(r, *f),
            site(r, *s)
        ),
        Moved { figure: f, to } => format!("{} went to live at {}.", figure(r, *f), site(r, *to)),
        PolityFounded {
            polity: p,
            seat,
            ruler: Some(f),
        } => format!(
            "{} became the seat of the realm of {}, ruled by {}.",
            site(r, *seat),
            polity(r, *p),
            figure(r, *f)
        ),
        PolityFounded {
            polity: p,
            seat,
            ruler: None,
        } => format!(
            "{} became the seat of the realm of {}.",
            site(r, *seat),
            polity(r, *p)
        ),
        Joined { polity: p, site: s } => {
            format!("{} joined the realm of {}.", site(r, *s), polity(r, *p))
        }
        Succeeded {
            polity: p,
            figure: f,
        } => format!("{} came to rule {}.", figure(r, *f), polity(r, *p)),
        PolityEnded { polity: p, cause } => {
            format!("The realm of {} came to an end: {cause}.", polity(r, *p))
        }
        WarBegan { a, b, cause, .. } => {
            format!(
                "War between {} and {}: {cause}.",
                polity(r, *a),
                polity(r, *b)
            )
        }
        Battle {
            site: s,
            won_by,
            with,
            ..
        } => {
            if with.is_empty() {
                format!(
                    "A battle at {}, won by {}.",
                    site(r, *s),
                    polity(r, *won_by)
                )
            } else {
                let allies: Vec<String> = with.iter().map(|q| polity(r, *q)).collect();
                format!(
                    "A battle at {}, won by {}; {} fought as allies.",
                    site(r, *s),
                    polity(r, *won_by),
                    allies.join(" and ")
                )
            }
        }
        Taken { site: s, by } => format!("{} was taken by {}.", site(r, *s), polity(r, *by)),
        WarEnded { war, cause } => {
            let names = r
                .wars
                .get(*war as usize)
                .map(|w| format!("{} and {}", polity(r, w.a), polity(r, w.b)))
                .unwrap_or_else(|| "two realms".into());
            format!("The war between {names} ended: {cause}.")
        }
        CultureSplit {
            culture,
            from,
            site: s,
        } => {
            let name = |c: u32| {
                r.cultures
                    .get(c as usize)
                    .map(|x| x.name.clone())
                    .unwrap_or_else(|| "a people".into())
            };
            format!(
                "Far from their homeland, the people of {} grew ways of their own: {}, \
                 from {}.",
                site(r, *s),
                name(*culture),
                name(*from)
            )
        }
        Traded {
            a,
            b,
            caravans,
            goods,
            cultures: (ca, cb),
        } => {
            let culture = |c: u32| {
                r.cultures
                    .get(c as usize)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| "a people".into())
            };
            format!(
                "Over ten years {caravans} caravans went between {} and {}, most between {} \
                 and {}: {goods}.",
                culture(*ca),
                culture(*cb),
                site(r, *a),
                site(r, *b)
            )
        }
        Married { a, b, at } => format!(
            "{} married {} and went to live at {}.",
            figure(r, *b),
            figure(r, *a),
            site(r, *at)
        ),
        Allied { a, b, against, .. } => format!(
            "{} and {} made common cause against {}.",
            polity(r, *a),
            polity(r, *b),
            polity(r, *against)
        ),
        Fled {
            from,
            to,
            people,
            count,
            cause,
        } => {
            let who = r
                .peoples
                .get(*people as usize)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "people".into());
            match to {
                Some(t) => format!(
                    "{count} of the {who} fled {} for {} ({cause}).",
                    site(r, *from),
                    site(r, *t)
                ),
                None => format!(
                    "{count} of the {who} fled {} and were scattered ({cause}).",
                    site(r, *from)
                ),
            }
        }
        BeastWoke { beast, .. } => {
            let b = r.beasts.get(*beast as usize);
            format!(
                "Something woke in the deep: {}, a thing of {} {}.",
                b.map(|b| b.name.clone())
                    .unwrap_or_else(|| "a beast".into()),
                b.map(|b| b.made_of.clone()).unwrap_or_default(),
                if b.is_some_and(|b| b.layer == "underworld") {
                    "from the underworld"
                } else {
                    "in the deep caverns"
                }
            )
        }
        BeastAttacked {
            beast,
            site: x,
            dead,
        } => format!(
            "{} fell on {}: {dead} died.",
            r.beasts
                .get(*beast as usize)
                .map(|b| b.name.clone())
                .unwrap_or_else(|| "a beast".into()),
            site(r, *x)
        ),
    }
}
