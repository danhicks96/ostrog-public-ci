//! Relics (#297, epic #376, ask 7): a thing with a story (legend.rs) that belonged to someone
//! revered becomes a relic of their community when they die. Revered is the keeper of the
//! observances (keepers.rs), one whose readings of the signs were borne out more than belied
//! (twice at least), or one whose death bore out a warning read to their community (faith.rs).
//! The relic is taken out of use and set down at the community's shrine (shrines.rs) if it has
//! one, else at home; nobody takes it up as a tool again. Pilgrims go to it in hard times
//! (pilgrimage.rs).
//!
//! A community that moves camp takes its relics with it to the new home. A relic that is gone
//! (broken, burned) or carried out of the colony by someone is lost, on record (`RelicLost`);
//! those who heed the signs take it hard, and think the less of whoever carried it off. FIXLOG 2026-10-28 "nothing was kept
//! in memory of the revered dead".

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, CommunityId, ItemId, TilePos};
use crate::items::ItemPlace;
use crate::mind::beliefs::Topic;
use crate::mind::ThoughtKind;
use crate::systems::settlement::Here;
use crate::World;
use serde::{Deserialize, Serialize};

/// Signs borne out, at least, and more than belied, for a reader to be revered.
pub const REVERED_BORNE: u32 = 2;

/// A community's relic.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Relic {
    pub item: ItemId,
    pub community: CommunityId,
    /// Whose it was.
    pub of: AgentId,
    /// What it is called: its name, else what it is.
    pub what: String,
    pub since: u64,
    /// Who carries or holds it now, if anyone; so a relic carried out of the colony is known to
    /// have been carried off, and by whom.
    #[serde(default)]
    pub held_by: Option<AgentId>,
}

/// Whether `id` is a relic: not to be taken up as a tool.
pub fn is_relic(w: &World, id: ItemId) -> bool {
    w.faith.relics.iter().any(|r| r.item == id)
}

/// Where a relic of `cid` lies (or who carries it stands), the first of them if several.
pub fn place_of(w: &World, cid: CommunityId) -> Option<(TilePos, String)> {
    w.faith
        .relics
        .iter()
        .filter(|r| r.community == cid)
        .find_map(|r| item_pos(w, r.item).map(|p| (p, r.what.clone())))
}

fn item_pos(w: &World, id: ItemId) -> Option<TilePos> {
    let it = w.items.iter().find(|it| it.id == id)?;
    match it.place {
        ItemPlace::Ground(p) => Some(p),
        ItemPlace::Carried(a) | ItemPlace::Held(a) => w
            .agents
            .iter()
            .find(|x| x.id == a && x.alive)
            .map(|x| x.pos),
    }
}

/// Where a community keeps its relics: beside its shrine, else at home.
fn keeping_place(w: &World, cid: CommunityId, home: TilePos) -> TilePos {
    // Not a shrine left behind at an old camp.
    crate::shrines::of(w, cid)
        .filter(|s| Here::of(w, cid, home).holds(*s))
        .map(|s| {
            [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .into_iter()
                .map(|(dx, dy)| w.grid.surface(s.x + dx, s.y + dy))
                .find(|p| w.grid.walkable(*p))
                .unwrap_or(s)
        })
        .unwrap_or_else(|| w.grid.surface(home.x, home.y))
}

/// Every tick: the revered dead's things become relics; lost relics are told.
pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    let mut dead: Vec<(AgentId, &'static str)> = Vec::new();
    for e in &w.last_events {
        match &e.kind {
            EventKind::Died { agent, .. } => {
                if let Some(why) = revered(w, *agent) {
                    dead.push((*agent, why));
                }
            }
            // A warning borne out by a death: the dead are those it foretold.
            EventKind::SignSettled {
                dead: d,
                good: false,
                ..
            } => dead.push((*d, "whose death bore out a warning")),
            _ => {}
        }
    }
    for (who, why) in dead {
        make(w, who, why);
    }
    if !w.faith.relics.is_empty() {
        watch(w, tick);
    }
}

/// Why `who`, just dead, is revered, if they are.
fn revered(w: &World, who: AgentId) -> Option<&'static str> {
    if w.faith.keepers.iter().any(|k| k.agent == who) {
        return Some("keeper of the observances");
    }
    let (borne, belied) = w.faith.record_of(who);
    (borne >= REVERED_BORNE && borne > belied).then_some("whose readings were borne out")
}

/// The storied thing of `who` that becomes a relic: the one they held last, by when they took
/// it up. Their kin may hold it already (legend.rs `step` runs first).
fn thing_of(w: &World, who: AgentId) -> Option<ItemId> {
    w.items
        .iter()
        .filter_map(|it| {
            let s = it.story.as_ref()?;
            let since = s.owners.iter().rev().find(|o| o.0 == who)?.2;
            Some((since, it.id))
        })
        .max_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)))
        .map(|x| x.1)
}

fn make(w: &mut World, who: AgentId, why: &str) {
    let tick = w.meta.tick;
    let Some(cid) = w
        .agents
        .iter()
        .find(|a| a.id == who)
        .and_then(|a| a.community)
    else {
        return;
    };
    let Some(home) = w
        .communities
        .iter()
        .find(|c| c.id == cid && !c.dissolved)
        .map(|c| c.home)
    else {
        return;
    };
    let Some(id) = thing_of(w, who) else { return };
    if is_relic(w, id) {
        return;
    }
    // Taken out of use: out of whoever's hand, and set down at the shrine, else at home.
    let at = keeping_place(w, cid, home);
    for a in w.agents.iter_mut() {
        if a.tool == Some(id) {
            a.tool = None;
            a.tool_quality = 0.0;
            a.tool_wear = 0.0;
        }
    }
    let Some(it) = w.items.iter_mut().find(|it| it.id == id) else {
        return;
    };
    it.place = ItemPlace::Ground(at);
    let what = it
        .story
        .as_ref()
        .and_then(|s| s.name.clone())
        .unwrap_or_else(|| crate::legend::noun(it.kind, it.material));
    w.faith.relics.push(Relic {
        item: id,
        community: cid,
        of: who,
        what: what.clone(),
        since: tick,
        held_by: None,
    });
    w.events.push(
        tick,
        Some(at),
        Severity::Notable,
        EventKind::RelicMade {
            community: cid,
            of: who,
            what,
            why: why.to_string(),
        },
    );
}

/// Each relic: who has it now, and whether it is lost.
fn watch(w: &mut World, tick: u64) {
    let all = w.faith.relics.clone();
    for r in all {
        let place = w.items.iter().find(|it| it.id == r.item).map(|it| it.place);
        let home = w
            .communities
            .iter()
            .find(|c| c.id == r.community && !c.dissolved)
            .map(|c| c.home);
        let Some(home) = home else {
            w.faith.relics.retain(|x| x.item != r.item);
            continue;
        };
        let (why, by) = match place {
            None => match r.held_by {
                Some(by) => ("it was carried out of the colony", Some(by)),
                None => ("it is gone", None),
            },
            Some(ItemPlace::Ground(p)) => {
                if let Some(x) = w.faith.relics.iter_mut().find(|x| x.item == r.item) {
                    x.held_by = None;
                }
                // A community that moves camp takes its relic with it, as anyone would.
                if tick % crate::time::TICKS_PER_HOUR == 43
                    && !Here::of(w, r.community, home).holds(p)
                {
                    let to = keeping_place(w, r.community, home);
                    if let Some(it) = w.items.iter_mut().find(|it| it.id == r.item) {
                        it.place = ItemPlace::Ground(to);
                    }
                }
                continue;
            }
            Some(ItemPlace::Carried(a)) | Some(ItemPlace::Held(a)) => {
                if let Some(x) = w.faith.relics.iter_mut().find(|x| x.item == r.item) {
                    x.held_by = Some(a);
                }
                continue;
            }
        };
        lose(w, r.community, r.item, &r.what, why, by);
    }
}

/// A relic of `cid` is lost to it: on record, and taken hard by those who heed the signs.
pub fn lose(
    w: &mut World,
    cid: CommunityId,
    item: ItemId,
    what: &str,
    why: &str,
    by: Option<AgentId>,
) {
    let tick = w.meta.tick;
    w.faith.relics.retain(|x| x.item != item);
    let at = w.communities.iter().find(|c| c.id == cid).map(|c| c.home);
    let ev = w.events.push(
        tick,
        at,
        Severity::Major,
        EventKind::RelicLost {
            community: cid,
            what: what.to_string(),
            why: why.to_string(),
            by,
        },
    );
    for k in 0..w.agents.len() {
        let a = &w.agents[k];
        if a.alive
            && a.community == Some(cid)
            && Some(a.id) != by
            && a.beliefs.lean(Topic::DeitySigns) >= crate::observances::LEADS_AT
        {
            w.agents[k]
                .thoughts
                .add(ThoughtKind::RelicLost, tick, Some(ev));
            if let Some(by) = by {
                w.agents[k].relate(by, -0.3, 0.05, tick);
            }
        }
    }
}

/// A faction that splits off (heresy.rs) takes a relic of the community it leaves with it:
/// lost to the old community, carried off by `leader`, and the new community's.
pub fn carried_off_by_split(w: &mut World, from: CommunityId, to: CommunityId, leader: AgentId) {
    let Some(r) = w.faith.relics.iter().find(|r| r.community == from).cloned() else {
        return;
    };
    lose(
        w,
        from,
        r.item,
        &r.what,
        "it was taken by those who split away",
        Some(leader),
    );
    // Carried to the new home.
    let home = w.communities.iter().find(|c| c.id == to).map(|c| c.home);
    if let (Some(h), Some(it)) = (home, w.items.iter_mut().find(|it| it.id == r.item)) {
        it.place = ItemPlace::Ground(h);
    }
    w.faith.relics.push(Relic {
        community: to,
        since: w.meta.tick,
        held_by: None,
        ..r
    });
}
