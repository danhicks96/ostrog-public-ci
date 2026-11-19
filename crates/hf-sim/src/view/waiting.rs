//! Why is nobody doing this? (#292 item 6, WP-08 slice 8.6.) For a planned work order, the
//! reason it stands untaken, read from the same conditions `systems::decide` applies when a
//! person weighs community work: someone already holds it, it was set aside after too many
//! failed tries, its materials exist nowhere on the ground, or every hand who could take it is
//! ruled out (a child, a belief against it, a craft they do not know, a recent failed try).
//! Read-only: nothing here changes what anyone does, and it states only what the record holds.

use crate::agents::Agent;
use crate::items::ItemPlace;
use crate::mind::beliefs::Topic;
use crate::work::{Designation, DesignationKind, ResKey};
use crate::world_state::World;

/// Failed tries after which `decide` no longer considers an order (systems/decide.rs).
const SET_ASIDE_AFTER: u32 = 12;

/// "Planned: wall (Aelsin has taken it)", "Planned: felling (nobody can: 2 will not fell a
/// living tree, 1 is a child)". The block panel's line for an order on a tile.
pub fn line(w: &World, d: &Designation) -> String {
    format!("Planned: {} ({})", d.label(), reason(w, d))
}

/// Why the order stands as it does, in plain words.
pub fn reason(w: &World, d: &Designation) -> String {
    if let Some(n) = w
        .reservations
        .holder(ResKey::Designation(d.id))
        .and_then(|id| name_of(w, id))
    {
        return format!("{n} has taken it");
    }
    if d.failures > SET_ASIDE_AFTER {
        return format!("set aside after {} failed tries", d.failures);
    }
    if let Some(m) = materials_nowhere(w, d) {
        return m;
    }
    if let Some(owner) = d.owner {
        return match name_of(w, owner) {
            Some(n) => format!("{n}'s own undertaking, not yet begun"),
            None => "the one who took it on is gone".into(),
        };
    }
    let hands: Vec<&Agent> = w
        .agents
        .iter()
        .filter(|a| a.alive && a.species == crate::agents::Species::Person)
        .filter(|a| a.community.is_some())
        .filter(|a| d.community.is_none() || a.community == d.community)
        .collect();
    if hands.is_empty() {
        return match d
            .community
            .and_then(|c| w.communities.iter().find(|x| x.id == c))
        {
            Some(c) => format!("nobody of {} is left to do it", c.name),
            None => "nobody is left to do it".into(),
        };
    }
    // Each hand's first bar, in a fixed order so the line is the same every time it is read.
    let mut bars: [(u32, &str); 5] = [
        (0, "are children"),
        (0, "will not dig into the deep"),
        (0, "will not fell a living tree"),
        (0, "do not know the craft"),
        (0, "gave up on it lately"),
    ];
    let now = w.meta.tick;
    let mut able = 0u32;
    for a in &hands {
        match bar(w, a, d, now) {
            Some(i) => bars[i].0 += 1,
            None => able += 1,
        }
    }
    if able == 0 {
        let parts: Vec<String> = bars
            .iter()
            .filter(|(n, _)| *n > 0)
            .map(|(n, what)| {
                if *n == 1 {
                    format!("1 {}", singular(what))
                } else {
                    format!("{n} {what}")
                }
            })
            .collect();
        return format!("nobody can: {}", parts.join(", "));
    }
    if crate::time::Calendar::rest_time(now, false) {
        return "it waits for morning".into();
    }
    if able == 1 {
        "1 could take it; other work weighs more for now".into()
    } else {
        format!("{able} could take it; other work weighs more for now")
    }
}

/// The first reason this person would pass the order over, as an index into `reason`'s bars,
/// or None when nothing rules them out (the same tests, in the same order, as decide.rs).
fn bar(w: &World, a: &Agent, d: &Designation, now: u64) -> Option<usize> {
    if a.is_child(now) {
        return Some(0);
    }
    if matches!(d.kind, DesignationKind::Mine | DesignationKind::DigStairs)
        && a.beliefs.lean(Topic::DeepDigging) < -0.3
        && crate::underground::in_the_deep(w, d.pos)
    {
        return Some(1);
    }
    if a.work_cooldowns.get(&d.id.0).is_some_and(|t| *t > now) {
        return Some(4);
    }
    if matches!(d.kind, DesignationKind::Chop) && a.beliefs.lean(Topic::CutLivingTree) <= -0.4 {
        return Some(2);
    }
    if let DesignationKind::Craft(r) = d.kind {
        if let Some(t) = crate::work::recipes::craft_of(r) {
            if !a.techniques.knows(t) && !crate::systems::crafts::may_try(w, a, t, d) {
                return Some(3);
            }
        }
    }
    None
}

/// "waiting for 3 stone; none lies anywhere in the valley", when a material still to be
/// brought exists nowhere on the ground (decide.rs: such a site is not a job yet).
fn materials_nowhere(w: &World, d: &Designation) -> Option<String> {
    let (kind, need, want) = d.missing_wants().next()?;
    let on_ground: u32 = w
        .items
        .iter()
        .filter(|it| {
            it.kind == kind
                && want.is_none_or(|m| it.material == m)
                && matches!(it.place, ItemPlace::Ground(_))
        })
        .map(|it| it.count)
        .sum();
    if on_ground >= need.min(1) {
        return None;
    }
    let label = kind.label_for_count(need);
    let what = match want {
        Some(m) => format!("{} {}", w.grid.materials.get(m).name, label),
        None => label.to_string(),
    };
    Some(format!(
        "waiting for {need} {what}; none lies anywhere in the valley"
    ))
}

fn name_of(w: &World, id: crate::ids::AgentId) -> Option<String> {
    w.agents
        .iter()
        .find(|a| a.id == id && a.alive)
        .map(|a| a.name.clone())
}

/// "are children" → "is a child", for a count of one.
fn singular(what: &str) -> &str {
    match what {
        "are children" => "is a child",
        "do not know the craft" => "does not know the craft",
        other => other,
    }
}

/// One order nobody has taken (#292, WP-08 8.6: the inspect-wide list of stuck orders).
#[derive(Clone, Debug, PartialEq)]
pub struct Stuck {
    pub label: String,
    pub reason: String,
    pub pos: crate::ids::TilePos,
    /// Ticks since it was posted.
    pub age: u64,
}

/// Orders not done that nobody holds and that have waited at least `min_age` ticks, oldest
/// first, at most `limit`, each with why it stands untaken (`reason`). For the "Waiting work"
/// panel: the valley-wide answer to "why is nobody doing X?".
pub fn stuck(w: &World, min_age: u64, limit: usize) -> Vec<Stuck> {
    let now = w.meta.tick;
    let mut out: Vec<(u64, u32, Stuck)> = w
        .designations
        .iter()
        .filter(|d| !d.done && now.saturating_sub(d.created) >= min_age)
        .filter(|d| w.reservations.holder(ResKey::Designation(d.id)).is_none())
        .map(|d| {
            let age = now.saturating_sub(d.created);
            (
                age,
                d.id.0,
                Stuck {
                    label: d.label(),
                    reason: reason(w, d),
                    pos: d.pos,
                    age,
                },
            )
        })
        .collect();
    // Oldest first; the order id breaks ties so the list reads the same every time.
    out.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    out.into_iter().take(limit).map(|(_, _, s)| s).collect()
}
