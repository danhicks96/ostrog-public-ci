//! A loss that stays (OST-215): the widowed carry a weight that eases over weeks, not a
//! seven-day thought.

use hf_sim::events::{EventKind, Severity};
use hf_sim::ids::TilePos;
use hf_sim::mind::ThoughtKind;
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

/// Two partners; the first dies. Returns the world and the widow's id.
fn widowed() -> (World, hf_sim::ids::AgentId) {
    let mut w = World::generate(1801, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(2);
    let (dead, wid) = (ids[0], ids[1]);
    for a in w.agents.iter_mut() {
        if a.id == dead {
            a.partner = Some(wid);
        }
        if a.id == wid {
            a.partner = Some(dead);
            a.relationships.entry(dead).or_default().opinion = 0.9;
        }
    }
    let tick = w.meta.tick;
    let id = w.events.push(
        tick,
        None,
        Severity::Major,
        EventKind::Died {
            agent: dead,
            cause: "fever".into(),
        },
    );
    let e = w
        .events
        .pending
        .iter()
        .find(|e| e.id == id)
        .unwrap()
        .clone();
    w.last_events = vec![e];
    hf_sim::systems::strain::observe(&mut w);
    (w, wid)
}

/// The Mourning weight the widow carries `days` after the death (0 when not held).
fn mourning_at(w: &mut World, who: hf_sim::ids::AgentId, days: u64) -> f32 {
    // Days count from the death tick, which is the world's first tick here.
    let died = w.agents.iter().find(|a| a.id == who).unwrap().bereaved[0].since;
    let day0 = died - died % TICKS_PER_DAY;
    w.meta.tick = day0 + days * TICKS_PER_DAY + 450;
    let now = w.meta.tick;
    hf_sim::systems::feel::step(w);
    w.agents
        .iter()
        .find(|a| a.id == who)
        .unwrap()
        .thoughts
        .list
        .iter()
        .find(|t| t.kind == ThoughtKind::Mourning && t.until > now)
        .map(|t| t.weight)
        .unwrap_or(0.0)
}

#[test]
fn a_widow_still_mourns_after_twenty_days_and_not_after_sixty() {
    let (mut w, wid) = widowed();
    assert!(mourning_at(&mut w, wid, 20) <= -0.1);
    assert_eq!(mourning_at(&mut w, wid, 60), 0.0);
}

#[test]
fn a_loss_eases_as_it_ages() {
    let (mut w, wid) = widowed();
    let early = mourning_at(&mut w, wid, 1);
    let later = mourning_at(&mut w, wid, 30);
    assert!(early < later && later < 0.0, "{early} then {later}");
}

#[test]
fn a_grown_child_mourns_a_parent_more_briefly_than_a_widow_a_partner() {
    use hf_sim::mind::bereavement::{Bereavement, Tie};
    let id = hf_sim::ids::AgentId(1);
    let at = |tie| {
        Bereavement {
            who: id,
            tie,
            bond: 0.9,
            since: 0,
        }
        .weight(30 * TICKS_PER_DAY)
    };
    assert!(at(Tie::Partner) < at(Tie::Parent));
}
