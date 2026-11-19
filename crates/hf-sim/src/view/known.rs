//! What the viewer may be told. The seven who came through alone are people from the first
//! tick, but nobody at the landing knows where they are or whether they live
//! (design-organic.md section 4). Until someone of the party finds them, nothing about them
//! reaches the feed, the camera director, the history view or the head count. Their lives are
//! still recorded as they happen, at the tick they happen, so finding them reveals their true
//! condition and the history shows what befell them from then on.
//!
//! Every viewer-facing read model asks here rather than deciding for itself.

use crate::events::Event;
use crate::ids::AgentId;
use crate::items::Item;
use crate::opening::SurvivorState;
use crate::world_state::World;

/// The lost nobody has found yet, in slot order.
pub fn unfound(w: &World) -> Vec<AgentId> {
    w.survivor_pool
        .iter()
        .filter_map(|s| match s.state {
            SurvivorState::Lost {
                agent,
                found: false,
            } => Some(agent),
            _ => None,
        })
        .collect()
}

pub fn is_unfound(w: &World, id: AgentId) -> bool {
    w.survivor_pool
        .iter()
        .any(|s| matches!(s.state, SurvivorState::Lost { agent, found: false } if agent == id))
}

/// May the viewer be told of this event: nobody it is about is one of the unfound lost.
pub fn told(w: &World, e: &Event) -> bool {
    // The wider world after the arrival is far off: nobody here knows what happens there
    // until someone brings the news (realm/live.rs, EventKind::NewsHeard).
    if matches!(e.kind, crate::events::EventKind::Realm { .. }) && e.tick >= w.meta.epoch {
        return false;
    }
    let hidden = unfound(w);
    hidden.is_empty() || !e.participants().iter().any(|a| hidden.contains(a))
}

/// Living people the viewer knows of: every living human except the unfound lost.
pub fn people_known(w: &World) -> usize {
    let hidden = unfound(w);
    w.agents
        .iter()
        .filter(|a| a.alive && a.species == crate::agents::Species::Person)
        .filter(|a| !hidden.contains(&a.id))
        .count()
}

/// A `real_history` filter (to AND into a WHERE clause) leaving out rows about the unfound
/// lost; empty when there are none. Ids are integers, so the clause is built inline.
pub fn rows_told_sql(w: &World) -> String {
    // The wider world's rows after the arrival are not told here either (see `told`).
    let far = format!(
        " AND NOT (kind LIKE 'realm\\_%' ESCAPE '\\' AND tick >= {})",
        w.meta.epoch
    );
    let hidden = unfound(w);
    if hidden.is_empty() {
        return far;
    }
    let ids: Vec<String> = hidden.iter().map(|a| a.0.to_string()).collect();
    format!(
        "{far} AND id NOT IN (SELECT history_id FROM entity_ref WHERE entity_kind = 'agent' \
         AND entity_id IN ({}))",
        ids.join(",")
    )
}

/// May the viewer see this thing: not if its story ties it to one of the unfound lost (the
/// tool a lost person dropped where they died names them, and lying on the map it would tell
/// the viewer of a death nobody at the landing knows of, #107).
pub fn item_told(w: &World, it: &Item) -> bool {
    let Some(s) = it.story.as_ref() else {
        return true;
    };
    let hidden = unfound(w);
    if hidden.is_empty() {
        return true;
    }
    let last_owner = s.owners.last().map(|o| o.0);
    !(last_owner.is_some_and(|o| hidden.contains(&o))
        || s.maker.is_some_and(|m| hidden.contains(&m)))
}

/// One thing a person keeps in mind, in a viewer's words for the inspect panel: "oak logs",
/// "a hearth", "Aelwen", "a food plant", "water". It was the Rust debug form, `Item(ItemId(0))
/// at (106,58,3)` (#178, QA 2026-10-20, C63).
pub fn describe(w: &World, k: &crate::mind::knowledge::Known) -> String {
    use crate::mind::knowledge::Known;
    match k {
        Known::Item(id) => match w.items.iter().find(|it| it.id == *id) {
            Some(it) => crate::items::stack_name(&w.grid.materials, it.kind, it.material),
            None => "something since moved".into(),
        },
        Known::FoodPlant(_) => "a food plant".into(),
        Known::Structure(id) => match w.structures.iter().find(|s| s.id == *id) {
            Some(s) => format!("a {}", words(&format!("{:?}", s.kind))),
            None => "a place since gone".into(),
        },
        Known::Agent(id) => match w.agents.iter().find(|a| a.id == *id) {
            Some(a) if a.species == crate::agents::Species::Person => a.name.clone(),
            Some(a) => format!("a {}", words(&format!("{:?}", a.species))),
            None => "someone".into(),
        },
        Known::Water(_) => "water".into(),
    }
}

/// `CookingPot` as "cooking pot".
fn words(camel: &str) -> String {
    let mut out = String::new();
    for (i, c) in camel.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push(' ');
        }
        out.extend(c.to_lowercase());
    }
    out
}
