//! The world goes on after the arrival (M3, C29): once a game year the realm takes its next
//! year (settlements, figures, realms, wars), the valley's neighbours follow what became of
//! their settlements, and notable things are kept for news. Nothing of it is told in the valley
//! (view::known::told) until someone back from out there brings word.

use super::sites::{RealmEventKind, SiteKind};
use crate::events::EventKind;
use crate::expedition::SiteKind as OK;
use crate::ids::AgentId;
use crate::time::{TICKS_PER_HOUR, TICKS_PER_YEAR};
use crate::world_state::World;

/// How far from a place news reaches it, in regions.
pub const NEWS_REACH: i64 = 16;

/// Where a realm event happened: the region of the settlement (or a realm's seat) it is about.
fn region_of(r: &super::sites::Realm, what: &RealmEventKind) -> Option<usize> {
    use RealmEventKind::*;
    let site = match what {
        Founded { site, .. }
        | Became { site, .. }
        | Famine { site, .. }
        | Plague { site, .. }
        | Ended { site, .. }
        | Battle { site, .. }
        | Taken { site, .. }
        | Led { site, .. }
        | Joined { site, .. }
        | Fled { from: site, .. }
        | BeastAttacked { site, .. } => Some(*site),
        BeastWoke { region, .. } => return Some(*region),
        Allied { a, .. } => r.polities.get(*a as usize).map(|p| p.seat),
        PolityFounded { seat, .. } => Some(*seat),
        Succeeded { polity, .. } | PolityEnded { polity, .. } => {
            r.polities.get(*polity as usize).map(|p| p.seat)
        }
        WarBegan { a, .. } => r.polities.get(*a as usize).map(|p| p.seat),
        _ => None,
    }?;
    r.sites.get(site as usize).map(|s| s.region)
}

fn newsworthy(what: &RealmEventKind) -> bool {
    !matches!(
        super::chronicle::severity(what),
        crate::events::Severity::Minor | crate::events::Severity::Trace
    )
}

pub fn step(w: &mut World) {
    if w.realm.peoples.is_empty() || w.realm.map_size == 0 {
        return;
    }
    let tick = w.meta.tick;
    let epoch = w.meta.epoch;
    // A year after the arrival and every year after, at six in the morning.
    if tick <= epoch || (tick - epoch) % TICKS_PER_YEAR != 6 * TICKS_PER_HOUR {
        return;
    }
    let seed = w.meta.seed;
    let map = w
        .realm_map
        .take()
        .unwrap_or_else(|| super::generate(seed, &super::RealmParams::default()));
    let mut out = Vec::new();
    w.realm.step(seed, &map, &mut out);
    super::figures::step(&mut w.realm, map.size, 0, &mut out);
    super::war::step(&mut w.realm, map.size, &mut out);
    super::relations::step(&mut w.realm, &map, 0, &mut out);
    super::beasts::step(&mut w.realm, &map, &mut out);
    let mut rows = Vec::new();
    for e in &out {
        let sev = super::chronicle::severity(&e.kind);
        let id = w.events.push(
            tick,
            None,
            sev,
            EventKind::Realm {
                what: e.kind.clone(),
            },
        );
        rows.push((id, e.clone()));
        if newsworthy(&e.kind) {
            if let Some(region) = region_of(&w.realm, &e.kind) {
                w.realm.news.push((e.year, region, e.kind.clone()));
            }
        }
    }
    refugees(w, &out);
    // Each culture's stories of the year, and their retelling (P8).
    super::stories::record(&mut w.realm, &rows);
    let year = w.realm.year;
    super::stories::retell(&mut w.realm, year);
    let n = w.realm.news.len();
    if n > 200 {
        w.realm.news.drain(..n - 200);
    }
    w.realm_map = Some(map);
    // The valley's neighbours are what their settlements have become.
    for s in w.overworld.sites.iter_mut() {
        let Some(rs) = s.realm_site.and_then(|i| w.realm.sites.get(i as usize)) else {
            continue;
        };
        if !rs.alive() {
            s.people = 0;
            if matches!(s.kind, OK::Village | OK::Mine) {
                s.kind = OK::Ruin;
            }
        } else if s.kind == OK::Village {
            s.people = (rs.population as u32).clamp(8, 200);
        } else if s.kind == OK::Mine && rs.kind == SiteKind::Mine {
            s.people = (rs.population as u32).clamp(6, 40);
        }
    }
}

/// Someone back from the overworld place `site` tells what happened near it since the valley
/// last heard from there: up to three things, the latest first.
pub fn news(w: &mut World, site: u32, teller: AgentId) {
    let Some(place) = w.overworld.sites.iter().find(|s| s.id == site) else {
        return;
    };
    let from = place.name.clone();
    let Some(rs) = place.realm_site.and_then(|i| w.realm.sites.get(i as usize)) else {
        return;
    };
    let (rsid, region) = (rs.id, rs.region);
    let n = w.realm.map_size.max(1);
    let since = w.realm.heard.get(&rsid).copied().unwrap_or(w.realm.arrived);
    let near = |r: usize| {
        let (x, y) = ((r % n) as i64, (r / n) as i64);
        let (vx, vy) = ((region % n) as i64, (region / n) as i64);
        (x - vx).abs().max((y - vy).abs()) <= NEWS_REACH
    };
    let mut told: Vec<String> = w
        .realm
        .news
        .iter()
        .rev()
        .filter(|(year, r, _)| *year > since && near(*r))
        .take(3)
        .map(|(_, _, what)| super::chronicle::line(&w.realm, what))
        .collect();
    w.realm.heard.insert(rsid, w.realm.year);
    let tick = w.meta.tick;
    for what in told.drain(..) {
        w.events.push(
            tick,
            None,
            crate::events::Severity::Notable,
            EventKind::NewsHeard {
                teller,
                from: from.clone(),
                what,
            },
        );
    }
}

/// People who fled one of the valley's neighbours this year (P9) come over the edge on that
/// neighbour's side: a few of those the record says fled, of that people and raised in their
/// ways, heading for the nearest community, as wanderers do.
pub fn refugees(w: &mut World, rows: &[super::sites::RealmEvent]) {
    use crate::mind::Activity;
    let Some(home) = w.communities.iter().find(|c| !c.dissolved).map(|c| c.home) else {
        return;
    };
    for e in rows {
        let RealmEventKind::Fled {
            from,
            people,
            count,
            cause,
            ..
        } = &e.kind
        else {
            continue;
        };
        let Some(s) = w
            .overworld
            .sites
            .iter()
            .find(|s| s.realm_site == Some(*from))
            .cloned()
        else {
            continue;
        };
        w.ensure_nav();
        let Some(edge) = crate::systems::expedition::edge_tile(w, s.edge, home) else {
            continue;
        };
        let Some(sp) = w.realm.peoples.get(*people as usize).map(|p| p.species) else {
            continue;
        };
        // Raised in the settlement's ways if it was theirs, else their people's first.
        let culture = w
            .realm
            .sites
            .get(*from as usize)
            .map(|rs| rs.culture)
            .filter(|c| {
                w.realm
                    .cultures
                    .get(*c as usize)
                    .is_some_and(|x| x.people == *people)
            })
            .unwrap_or(*people);
        let n = (*count as usize / 10).clamp(2, 4);
        let ids = w.arrive_party(n, edge, None);
        for id in &ids {
            crate::species::give(w, *id, sp, Some(*people as usize));
            crate::species::raise_in(w, *id, culture);
        }
        crate::lore::newcomers(w, &ids);
        for id in &ids {
            if let Some(i) = w.agents.iter().position(|a| a.id == *id) {
                w.agents[i].needs.hunger = 0.35;
                if w.path_agent_to(i, home) {
                    w.agents[i].activity = Activity::Wandering { to: home };
                }
            }
        }
        let tick = w.meta.tick;
        w.events.push(
            tick,
            Some(edge),
            crate::events::Severity::Notable,
            EventKind::Refugees {
                agents: ids,
                from: s.name.clone(),
                cause: cause.clone(),
            },
        );
    }
}

/// A community that left the valley settles out in the world (C60): a hamlet of its people in
/// the nearest free land near the valley's region, recorded as the realm records a founding,
/// and simulated from then on with the rest of the realm. The settlement's name, if a place was
/// found.
/// A valley departure's destination, named and spotted at the choice (#1322), before anyone
/// has walked anywhere; the site itself is founded only once the first leaver arrives
/// (`settle_arrived`), so the realm holds who arrived, not who set out. Returns the pending
/// settlement id (for `Agent.bound_for`) and the name to narrate now.
pub fn reserve_settle_from_valley(
    w: &mut World,
    species: crate::species::SpeciesId,
) -> Option<(u32, String)> {
    if w.realm.peoples.is_empty() || w.realm.map_size == 0 {
        return None;
    }
    let seed = w.meta.seed;
    let map = w
        .realm_map
        .take()
        .unwrap_or_else(|| super::generate(seed, &super::RealmParams::default()));
    let from = w.realm.reserved.iter().next().copied().unwrap_or(0);
    let folk = w
        .realm
        .peoples
        .iter()
        .find(|p| p.species == species && !p.deep)
        .or_else(|| w.realm.peoples.iter().find(|p| !p.deep))
        .map(|p| p.id);
    // Not a region another pending (not yet founded) departure has already claimed.
    let taken: std::collections::BTreeSet<usize> =
        w.realm.pending.iter().map(|p| p.region).collect();
    let spot = w.realm.spot(&map, from, 12, 0.0, false, |j| {
        map.land(j) && !taken.contains(&j)
    });
    let made = match (folk, spot) {
        (Some(folk), Some(region)) => {
            let pid = w.realm.next_pending;
            w.realm.next_pending += 1;
            let name = w
                .realm
                .unique_name(folk, "site", 0x5E77_0000_0000 | pid as u64);
            w.realm.pending.push(super::sites::PendingSettle {
                id: pid,
                folk,
                region,
                kind: super::sites::SiteKind::Hamlet,
                name: name.clone(),
                parent: None,
                site: None,
            });
            Some((pid, name))
        }
        _ => None,
    };
    w.realm_map = Some(map);
    made
}

/// One of a pending departure's leavers has stepped off the map: founds the realm site on the
/// first arrival (population 1), or grows it by one on every later arrival. Nothing happens, and
/// nothing is founded, if `pending` names no still-open reservation (#1322).
pub fn settle_arrived(w: &mut World, pending: u32) {
    let Some(idx) = w.realm.pending.iter().position(|p| p.id == pending) else {
        return;
    };
    let mut out = Vec::new();
    match w.realm.pending[idx].site {
        Some(site) => {
            let s = &mut w.realm.sites[site as usize];
            s.population += 1.0;
            s.peak = s.peak.max(s.population);
        }
        None => {
            let p = w.realm.pending[idx].clone();
            w.realm
                .found_named(p.folk, p.region, p.kind, 1.0, p.parent, p.name, &mut out);
            let site = w.realm.sites.len() as u32 - 1;
            w.realm.pending[idx].site = Some(site);
        }
    }
    let tick = w.meta.tick;
    for e in &out {
        let sev = super::chronicle::severity(&e.kind);
        w.events.push(
            tick,
            None,
            sev,
            EventKind::Realm {
                what: e.kind.clone(),
            },
        );
    }
}
