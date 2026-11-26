//! Omens (#297, epic #376): the one a community follows in the spirit (justice.rs) reads a
//! great event as a sign, a warning or a blessing, and those near feel it (feelings): fear for a
//! warning, gladness for a blessing. The reading is on record.
//!
//! Outside pressure is read before it is seen (#297 ask 4, threat.rs): while a band is on the
//! road to the valley and nobody has sighted it, birds rise over the strangers; someone of a
//! community who is awake, outdoors and within `BIRDS_SEEN` tiles of a walker (beyond the 12 of
//! a sighting) sees it, and reads a warning in it if devout enough (`observances::LEADS_AT`),
//! once for each band. A warning read this way is a sign like any other (faith.rs), borne out
//! if the band costs a life. Crows were once read by a roll, not by anyone seeing them
//! (OST-1312).

use crate::events::{EventKind, Severity};
use crate::mind::feelings::{gain, FeelingKind};
use crate::World;

/// How far off someone outdoors can see birds lift over strangers walking in.
pub const BIRDS_SEEN: i32 = 30;

type Sign = (
    String,
    bool,
    Option<crate::ids::TilePos>,
    Option<crate::ids::AgentId>,
);

/// A band on the road, not yet sighted, read as a warning (once for each band).
fn foretell(w: &mut World) -> Vec<Sign> {
    let tick = w.meta.tick;
    let hour = (tick % crate::time::TICKS_PER_DAY) / crate::time::TICKS_PER_HOUR;
    if tick % crate::time::TICKS_PER_HOUR != 53 || !(6..=20).contains(&hour) {
        return Vec::new();
    }
    let road: Vec<u32> = w
        .bands
        .iter()
        .filter(|b| {
            b.state == crate::threat::BandState::Approaching
                && !b.sighted
                && !w.faith.foretold.contains(&b.id)
        })
        .map(|b| b.id)
        .collect();
    let mut out = Vec::new();
    for id in road {
        let Some(band) = w.bands.iter().find(|b| b.id == id) else {
            continue;
        };
        let walkers: Vec<crate::ids::TilePos> = band
            .members
            .iter()
            .filter_map(|m| w.agents.iter().find(|a| a.id == *m && a.alive))
            .map(|a| a.pos)
            .collect();
        let seer = (0..w.agents.len()).find(|&k| {
            let a = &w.agents[k];
            a.alive
                && a.community.is_some()
                && a.band.is_none()
                && !a.activity.is_asleep()
                && !crate::systems::hazards::indoors(w, a.pos)
                && walkers
                    .iter()
                    .any(|p| p.z == a.pos.z && a.pos.dist(*p) > 12 && a.pos.dist(*p) <= BIRDS_SEEN)
                && crate::observances::devotion(w, k) >= crate::observances::LEADS_AT
        });
        let Some(k) = seer else { continue };
        let seer = w.agents[k].id;
        w.faith.foretold.push(id);
        if w.faith.foretold.len() > 32 {
            w.faith.foretold.remove(0);
        }
        out.push((
            "crows coming in over the horizon".to_string(),
            false,
            None,
            Some(seer),
        ));
    }
    out
}

pub fn observe(w: &mut World) {
    let tick = w.meta.tick;
    let foretold = foretell(w);
    let signs: Vec<Sign> = w
        .last_events
        .iter()
        .filter_map(|e| {
            let (what, good) = match &e.kind {
                EventKind::SpellBegan { what, .. } => (what.clone(), what == "a kind season"),
                EventKind::Besieged { site, .. } => (format!("the host from {site}"), false),
                EventKind::Rose { .. } => ("the beast come up from the deep".to_string(), false),
                EventKind::Shifted { .. } => {
                    ("a neighbour gone out in a beast's shape".to_string(), false)
                }
                EventKind::Breached { cavern, .. } => (format!("the way into {cavern}"), false),
                EventKind::RivalReturned { name, .. } => (format!("{name} come back"), false),
                _ => return None,
            };
            Some((what, good, e.at, None))
        })
        .chain(foretold)
        .collect();
    for (what, good, at, seen_by) in signs {
        let cids: Vec<_> = w
            .communities
            .iter()
            .filter(|c| !c.dissolved)
            .map(|c| (c.id, c.home))
            .collect();
        let cids: Vec<_> = cids
            .into_iter()
            .map(|(id, home)| (id, crate::faith::reader_of(w, id), home))
            // Someone who saw it themselves: their own community, and they are named.
            .filter_map(|(id, reader, home)| match seen_by {
                Some(s) => w
                    .agents
                    .iter()
                    .any(|a| a.id == s && a.community == Some(id))
                    .then_some((id, Some(s), home)),
                None => Some((id, reader, home)),
            })
            .collect();
        for (cid, reader, home) in cids {
            let Some(reader) = reader else { continue };
            let Some(ri) = w.agents.iter().position(|a| a.id == reader && a.alive) else {
                continue;
            };
            let near = at.unwrap_or(home);
            let kind = if good {
                FeelingKind::Joy
            } else {
                FeelingKind::Fear
            };
            for k in 0..w.agents.len() {
                let a = &w.agents[k];
                if a.alive
                    && a.community == Some(cid)
                    && a.pos.z == near.z
                    && a.pos.dist(w.agents[ri].pos) <= 15
                {
                    let g = gain(&w.agents[k], kind);
                    w.agents[k].feelings.feel(kind, None, 0.3 * g, tick, None);
                }
            }
            let rp = w.agents[ri].pos;
            w.events.push(
                tick,
                Some(rp),
                Severity::Notable,
                EventKind::Omen {
                    reader,
                    of: what.clone(),
                    good,
                },
            );
        }
    }
}
