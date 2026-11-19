//! Arms (C33). When strangers have threatened a community (seen near home, its stores raided,
//! a fight), those of its people who would fight anyway take up the spears and armour its
//! smiths have made; nobody hands them out. A spear makes a blow land more often and cut
//! deeper, armour turns some blows and blunts the rest, and every blow wears the piece that
//! struck or took it until it breaks. When the danger is a season gone they lay the arms by at
//! home, and the dead leave theirs where they fell. Runs once a game hour; the fight hooks
//! (`edge`, `guard`, `struck_with`, `took_blow`) are called from threat.rs, animals.rs and the
//! hunt.

use crate::events::{EventKind, Severity};
use crate::ids::ItemId;
use crate::items::{ItemKind, ItemPlace};
use crate::mind::beliefs::Topic;
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::world_state::World;

/// A threat this recent sends those who would fight for their arms.
pub const ARM_WITHIN: u64 = 10 * TICKS_PER_DAY;
/// Arms are laid by once the last threat is this far behind.
pub const LAY_BY_AFTER: u64 = 30 * TICKS_PER_DAY;
/// Reach from home within which a weapon or armour counts as the community's.
const REACH: i32 = 45;

/// Strangers threatened the community nearest `at` (threat.rs calls this on a sighting, a raid
/// or a fight).
pub fn threatened(w: &mut World, at: crate::ids::TilePos) {
    let tick = w.meta.tick;
    if let Some(c) = w
        .communities
        .iter_mut()
        .filter(|c| !c.dissolved)
        .min_by_key(|c| c.home.dist(at))
    {
        c.threatened = tick;
    }
}

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_HOUR != 41 {
        return;
    }
    // The dead leave what they carried where they fell.
    for i in 0..w.agents.len() {
        if w.agents[i].alive {
            continue;
        }
        let pos = w.agents[i].pos;
        for id in [w.agents[i].weapon.take(), w.agents[i].armour.take()]
            .into_iter()
            .flatten()
        {
            if let Some(it) = w.items.iter_mut().find(|it| it.id == id) {
                it.place = ItemPlace::Ground(pos);
            }
        }
    }
    for i in 0..w.agents.len() {
        let a = &w.agents[i];
        if !a.alive || a.species != crate::agents::Species::Person || a.band.is_some() {
            continue;
        }
        let Some(cid) = a.community else { continue };
        let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
            continue;
        };
        let (home, since) = (c.home, tick.saturating_sub(c.threatened));
        let threat = c.threatened > 0 && since <= ARM_WITHIN;
        // The same people who go after a raider (decide.rs consider_bands) arm themselves.
        let would_fight =
            a.of_age(tick) && !a.body.needs_care() && a.beliefs.lean(Topic::Violence) > -0.2;
        if threat && would_fight {
            for kind in [ItemKind::Weapon, ItemKind::Armour] {
                let held = match kind {
                    ItemKind::Weapon => w.agents[i].weapon,
                    _ => w.agents[i].armour,
                };
                if held.is_none() {
                    take_up(w, i, kind, home);
                }
            }
        } else if c.threatened == 0 || since > LAY_BY_AFTER {
            for id in [w.agents[i].weapon.take(), w.agents[i].armour.take()]
                .into_iter()
                .flatten()
            {
                if let Some(it) = w.items.iter_mut().find(|it| it.id == id) {
                    it.place = ItemPlace::Ground(home);
                }
            }
        }
    }
}

/// The best spear or armour lying within reach of home, taken up by agent `i`.
fn take_up(w: &mut World, i: usize, kind: ItemKind, home: crate::ids::TilePos) {
    let who = w.agents[i].id;
    let Some(id) = w
        .items
        .iter()
        .filter(|it| {
            it.kind == kind
                && matches!(it.place, ItemPlace::Ground(p) if p.dist(home) <= REACH)
                && w.reservations
                    .is_free_for(crate::work::ResKey::Item(it.id), who)
                // A relic is kept, not taken up (relics.rs).
                && !crate::relics::is_relic(w, it.id)
        })
        .max_by(|a, b| a.quality.total_cmp(&b.quality).then(b.id.cmp(&a.id)))
        .map(|it| it.id)
    else {
        return;
    };
    crate::legend::ensure(w, id, None, "of unknown make");
    let tick = w.meta.tick;
    let name = w.agents[i].name.clone();
    let mut what = String::new();
    if let Some(it) = w.items.iter_mut().find(|it| it.id == id) {
        it.place = ItemPlace::Held(who);
        what = crate::legend::noun(it.kind, it.material);
        if let Some(s) = it.story.as_mut() {
            if s.owners.last().map(|o| o.0) != Some(who) {
                s.owners.push((who, name, tick));
                s.keeper_uses = 0;
                s.cherished = false;
            }
            if let Some(n) = &s.name {
                what = n.clone();
            }
        }
    }
    match kind {
        ItemKind::Weapon => w.agents[i].weapon = Some(id),
        _ => w.agents[i].armour = Some(id),
    }
    let at = w.agents[i].pos;
    w.events.push(
        tick,
        Some(at),
        Severity::Notable,
        EventKind::TookUpArms { agent: who, what },
    );
}

/// How much a held piece of `kind` adds, 0 (none) to about 1 (a fine iron one): its metal's
/// hardness against iron's, from half to all of it by quality.
fn worth(w: &World, id: Option<ItemId>) -> Option<(ItemId, f32)> {
    let id = id?;
    let it = w.items.iter().find(|it| it.id == id)?;
    let hard = w.grid.materials.get(it.material).hardness.max(0.6);
    Some((id, (hard / 1.7).min(1.2) * (0.5 + 0.5 * it.quality)))
}

/// The edge agent `i` fights with: their spear's worth, if they carry one.
pub fn edge(w: &World, i: usize) -> Option<(ItemId, f32)> {
    worth(w, w.agents[i].weapon)
}

/// The guard agent `i` wears: their armour's worth, if they have it on.
pub fn guard(w: &World, i: usize) -> Option<(ItemId, f32)> {
    worth(w, w.agents[i].armour)
}

/// A blow struck with agent `i`'s spear: it wears, and its story counts it. `hunt`: at game
/// rather than at a person.
pub fn struck_with(w: &mut World, i: usize, hunt: bool) {
    let Some(id) = w.agents[i].weapon else { return };
    let words: [&str; 3] = if hunt {
        [
            "brought down game",
            "fed its people through a winter of hunts",
            "was a hunter's spear twenty times over",
        ]
    } else {
        [
            "drew a stranger's blood",
            "was raised against strangers five times",
            "held off raiders ten times",
        ]
    };
    record(w, i, id, hunt, words);
    wear(w, i, id, if hunt { 0.02 } else { 0.06 });
}

/// A blow taken on agent `i`'s armour.
pub fn took_blow(w: &mut World, i: usize) {
    let Some(id) = w.agents[i].armour else { return };
    record(
        w,
        i,
        id,
        false,
        [
            "turned a blow meant for its wearer",
            "took five blows and held",
            "took ten blows and held",
        ],
    );
    wear(w, i, id, 0.08);
}

fn record(w: &mut World, i: usize, id: ItemId, hunt: bool, words: [&str; 3]) {
    let tick = w.meta.tick;
    let Some(it) = w.items.iter_mut().find(|it| it.id == id) else {
        return;
    };
    let Some(s) = it.story.as_mut() else { return };
    let n = if hunt {
        s.hunts += 1;
        s.hunts
    } else {
        s.fights += 1;
        s.fights
    };
    let steps: [u32; 3] = if hunt { [1, 8, 20] } else { [1, 5, 10] };
    if let Some(k) = steps.iter().position(|x| *x == n) {
        s.deeds.push((tick, words[k].to_string()));
    }
    s.uses += 1;
    s.keeper_uses += 1;
    let _ = i;
}

/// Wear a held weapon or armour; broken, it is gone, and the feed says what it was.
fn wear(w: &mut World, i: usize, id: ItemId, by: f32) {
    let tick = w.meta.tick;
    let Some(ix) = w.items.iter().position(|it| it.id == id) else {
        if w.agents[i].weapon == Some(id) {
            w.agents[i].weapon = None;
        }
        if w.agents[i].armour == Some(id) {
            w.agents[i].armour = None;
        }
        return;
    };
    let left = match w.items[ix].story.as_mut() {
        Some(s) => {
            s.wear -= by;
            s.wear
        }
        None => 1.0,
    };
    if left > 0.0 {
        return;
    }
    let it = w.items[ix].clone();
    let s = it.story.clone().unwrap_or_default();
    let who = w.agents[i].id;
    let at = w.agents[i].pos;
    w.consume_item(id, 1, "worn out");
    if w.agents[i].weapon == Some(id) {
        w.agents[i].weapon = None;
    }
    if w.agents[i].armour == Some(id) {
        w.agents[i].armour = None;
    }
    w.events.push(
        tick,
        Some(at),
        if s.name.is_some() {
            Severity::Notable
        } else {
            Severity::Minor
        },
        EventKind::ToolBroke {
            agent: who,
            tier: ItemKind::tier(it.quality),
            what: s
                .name
                .clone()
                .unwrap_or_else(|| crate::legend::noun(it.kind, it.material)),
            history: crate::legend::history_of(w, &it),
        },
    );
}
