//! A beast is told by the name it had when a line's event happened (#486, FIXLOG 2026-11-03):
//! a bear wounded and then named for the fight is "A bear" in the wound's line, told then or
//! told again later, and by its name in lines after it earned it.

use hf_sim::agents::Species;
use hf_sim::events::{Event, EventKind, Severity};
use hf_sim::ids::{AgentId, EventId, TilePos};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn wounded(agent: AgentId, tick: u64) -> Event {
    Event {
        id: EventId(1),
        tick,
        at: None,
        severity: Severity::Minor,
        kind: EventKind::Wounded {
            agent,
            part: "right arm".into(),
            kind: "cut".into(),
            cause: "fighting off a beast".into(),
        },
    }
}

#[test]
fn a_wound_line_names_the_beast_as_it_was_then() {
    let mut w = World::generate(1, GenParams::small());
    let id = w.spawn_agent(Species::Bear, TilePos::new(20, 20, 2));
    let i = w.agents.iter().position(|a| a.id == id).unwrap();
    let early = wounded(id, 1000);
    let told_then = hf_sim::view::narrate::line(&w, &early);
    assert_eq!(
        told_then,
        "A bear took a cut to the right arm fighting off a beast."
    );
    // Named for the fight a little after.
    w.agents[i].name = "Longback".into();
    w.agents[i].named_at = Some(1200);
    assert_eq!(
        hf_sim::view::narrate::line(&w, &early),
        told_then,
        "told again, the earlier line is the line that was shown"
    );
    assert_eq!(
        hf_sim::view::narrate::line(&w, &wounded(id, 1300)),
        "Longback took a cut to the right arm fighting off a beast."
    );
}
