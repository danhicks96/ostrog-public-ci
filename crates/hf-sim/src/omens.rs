//! Omens (#297, epic #376): the one a community follows in the spirit (justice.rs) reads a
//! great event as a sign, a warning or a blessing, and those near feel it (feelings): fear for a
//! warning, gladness for a blessing. The reading is on record.
//!
//! Outside pressure is read before it is seen (#297 ask 4, threat.rs): while a band is on the
//! road to the valley and nobody has sighted it, each community's reader has a one in five
//! chance each daylight hour of reading a warning in the birds, once for each band. A warning
//! read this way is a sign like any other (faith.rs), borne out if the band costs a life.
//! FIXLOG 2026-10-27 "the omens never foretold a band".

use crate::events::{EventKind, Severity};
use crate::mind::feelings::{gain, FeelingKind};
use crate::World;

/// Chance each daylight hour that a band on the road is read in the signs.
pub const FORETOLD_A_HOUR: f64 = 0.2;

/// A band on the road, not yet sighted, read as a warning (once for each band).
fn foretell(w: &mut World) -> Vec<(String, bool, Option<crate::ids::TilePos>)> {
    use rand::Rng;
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
        let mut r = crate::rng::stream(
            w.meta.seed,
            &[crate::rng::purpose::AGENT_SOCIAL, 0x2974, id as u64, tick],
        );
        if !r.gen_bool(FORETOLD_A_HOUR) {
            continue;
        }
        w.faith.foretold.push(id);
        if w.faith.foretold.len() > 32 {
            w.faith.foretold.remove(0);
        }
        out.push((
            "crows coming in over the valley's rim".to_string(),
            false,
            None,
        ));
    }
    out
}

pub fn observe(w: &mut World) {
    let tick = w.meta.tick;
    let foretold = foretell(w);
    let signs: Vec<(String, bool, Option<crate::ids::TilePos>)> = w
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
            Some((what, good, e.at))
        })
        .chain(foretold)
        .collect();
    for (what, good, at) in signs {
        let cids: Vec<_> = w
            .communities
            .iter()
            .filter(|c| !c.dissolved)
            .map(|c| (c.id, c.home))
            .collect();
        let cids: Vec<_> = cids
            .into_iter()
            .map(|(id, home)| (id, crate::faith::reader_of(w, id), home))
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
