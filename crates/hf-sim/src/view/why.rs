//! The why of a mood (#292): what a person feels most and about whom, and the thoughts that
//! push their mood the way it leans. For the inspect panel.

use crate::agents::Agent;
use crate::mind::feelings::FeelingKind as F;
use crate::world_state::World;

/// "afraid of Greyjaw", "grieving for Halgar", "glad"; empty when nothing is felt much.
pub fn feeling(w: &World, a: &Agent) -> String {
    let Some(f) = a.feelings.strongest().filter(|f| f.strength >= 0.2) else {
        return String::new();
    };
    let of = f
        .about
        .and_then(|id| w.agents.iter().find(|o| o.id == id))
        .map(|o| o.name.clone());
    match (f.kind, of) {
        (F::Fear, Some(n)) => format!("afraid of {n}"),
        (F::Grief, Some(n)) => format!("grieving for {n}"),
        (F::Anger, Some(n)) => format!("angry with {n}"),
        (F::Joy, Some(n)) => format!("glad of {n}"),
        (F::Awe, Some(n)) => format!("in awe of {n}"),
        (F::Shame, Some(n)) => format!("ashamed before {n}"),
        (k, None) => k.word().to_string(),
    }
}

/// The two thoughts weighing most on the side the mood leans: "slept on the ground, a
/// quarrel".
pub fn because(w: &World, a: &Agent) -> String {
    let lean = a.thoughts.sum(w.meta.tick);
    let mut why: Vec<&crate::mind::Thought> = a
        .thoughts
        .list
        .iter()
        .filter(|t| (t.weight < 0.0) == (lean < 0.0))
        .collect();
    why.sort_by(|x, y| y.weight.abs().total_cmp(&x.weight.abs()));
    why.iter()
        .take(2)
        .map(|t| t.kind.label().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
