//! Observe the normal pipeline at its life stage, without installing simulation state.

use std::collections::BTreeMap;

use hf_sim::events::EventKind;
use hf_sim::systems::life::step_with_birth_observer;
use hf_sim::World;

#[derive(Default)]
pub struct BirthCounts {
    pub gates: BTreeMap<&'static str, u64>,
    pub births: u64,
}

pub fn observed_tick(w: &mut World, counts: &mut BirthCounts) {
    w.meta.tick += 1;
    // Read the live pipeline rather than maintaining a second list of systems. In particular,
    // births must be observed after pairing and after any earlier couple's child was born.
    for (name, run) in hf_sim::systems::PIPELINE {
        if *name == "life" {
            step_with_birth_observer(w, &mut |_, _, gate| {
                *counts.gates.entry(gate.name()).or_default() += 1;
            });
        } else {
            run(w);
        }
    }
    if hf_sim::debuglog::enabled() {
        hf_sim::debuglog::after_tick(w);
    }
    counts.births += w
        .last_events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::Born { .. }))
        .count() as u64;
}
