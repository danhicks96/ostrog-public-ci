//! #1791: a smashed bed should wrong its owner, not nobody.

use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::mind::Trait;
use hf_sim::structures::{Structure, StructureKind};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

/// A, B and a bed B owns, A within 6 tiles of the bed, B at `b_dist` tiles from it. Runs
/// tantrums until A smashes the bed (A is leant toward tantrums by the Industrious trait and
/// the absence of any other breakable furniture), then returns the world as of that break.
fn setup(seed: u64, b_dist: i32) -> (World, hf_sim::ids::AgentId, hf_sim::ids::AgentId, bool) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(2);
    let (a_id, b_id) = (ids[0], ids[1]);
    w.meta.autoplan = false;

    let bed_pos = TilePos::new(30, 30, 2);
    let sid = w.alloc_structure_id();
    w.structures.push(Structure {
        id: sid,
        kind: StructureKind::Bed,
        pos: bed_pos,
        material: Material::WOOD,
        progress: 1.0,
        active: false,
        condition: 1.0,
        owner: Some(b_id),
        quality: 0.5,
    });

    let ai = w.agents.iter().position(|a| a.id == a_id).unwrap();
    let bi = w.agents.iter().position(|a| a.id == b_id).unwrap();
    w.agents[ai].pos = TilePos::new(30, 32, 2); // 2 tiles from the bed: within the tantrum's 6.
    w.agents[ai].traits.list = vec![Trait::Industrious];
    w.agents[bi].pos = TilePos::new(30, 30 + b_dist, 2);
    w.agents[bi].activity = hf_sim::mind::Activity::Idle;

    let mut broke_it = false;
    for d in 0..200u64 {
        w.agents[ai].stress = 1.0;
        w.agents[ai].last_break = 0;
        w.agents[ai].needs.hunger = 1.0;
        w.agents[ai].activity = hf_sim::mind::Activity::Idle;
        w.agents[ai].pos = TilePos::new(30, 32, 2);
        w.agents[bi].activity = hf_sim::mind::Activity::Idle;
        w.meta.tick = (d + 1) * 3 * TICKS_PER_DAY + 12 * TICKS_PER_HOUR + 900;
        w.events.pending.clear();
        hf_sim::systems::strain::step(&mut w);
        if w.events
            .pending
            .iter()
            .any(|e| matches!(&e.kind, EventKind::FurnitureBroken { agent, .. } if *agent == a_id))
        {
            broke_it = true;
            break;
        }
        // The bed may already be gone without naming A if something else broke it first;
        // in that case there is nothing left to test, so bail out loudly rather than looping.
        if !w.structures.iter().any(|s| s.id == sid) {
            break;
        }
    }
    (w, a_id, b_id, broke_it)
}

#[test]
fn a_smashed_bed_wrongs_its_owner_and_they_accuse() {
    // `offence` takes up a broken bed only a third of the time (justice.rs): try a handful of
    // seeds rather than relying on one lucky roll.
    let mut accused = false;
    let mut last_opinion = 0.0;
    for seed in 9101..9121u64 {
        let (w, a_id, b_id, broke_it) = setup(seed, 4);
        assert!(broke_it, "A never smashed the bed within 200 tries");
        let bi = w.agents.iter().position(|a| a.id == b_id).unwrap();
        last_opinion = w.agents[bi].opinion_of(a_id);
        assert!(
            last_opinion <= -0.19,
            "B's opinion of A should have fallen by about 0.2, got {last_opinion}"
        );
        if w.events.pending.iter().any(
            |e| matches!(&e.kind, EventKind::Accused { by, of, .. } if *by == b_id && *of == a_id),
        ) {
            accused = true;
            break;
        }
    }
    assert!(
        accused,
        "B should have accused A at least once in 20 tries (last opinion {last_opinion})"
    );
}

#[test]
fn a_smashed_bed_far_from_its_owner_wrongs_nobody() {
    let (w, a_id, b_id, broke_it) = setup(9102, 30);
    assert!(broke_it, "A never smashed the bed within 200 tries");
    let bi = w.agents.iter().position(|a| a.id == b_id).unwrap();
    let opinion = w.agents[bi].opinion_of(a_id);
    assert!(
        opinion.abs() < 0.001,
        "B did not see it, so B's opinion of A should be unchanged, got {opinion}"
    );
    assert!(
        !w.events
            .pending
            .iter()
            .any(|e| matches!(&e.kind, EventKind::Accused { by, .. } if *by == b_id)),
        "B did not see it, so B should not be the accuser"
    );
}
