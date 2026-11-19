//! The Engine (others call this a storyteller). It does not fire events: it decides how hard
//! the *existing* generators lean on the community, and every number it produces comes from
//! recorded state. See docs/design-emergence.md.

use hf_sim::engine::{self, Engine};
use hf_sim::world::gen::GenParams;
use hf_sim::world_state::World;

const TICKS_PER_DAY: u64 = 28_800;

#[test]
fn the_engine_is_deterministic() {
    let a = engine::rates(Engine::Whimsical, 0.5, 100, 7);
    let b = engine::rates(Engine::Whimsical, 0.5, 100, 7);
    assert_eq!(
        format!("{a:?}"),
        format!("{b:?}"),
        "same state, same answer"
    );
}

#[test]
fn the_engines_are_different_engines() {
    let steady = engine::rates(Engine::Steady, 0.6, 100, 7);
    let relentless = engine::rates(Engine::Relentless, 0.6, 100, 7);
    let whimsical = engine::rates(Engine::Whimsical, 0.6, 100, 7);
    let all = [steady.arrivals, relentless.arrivals, whimsical.arrivals];
    assert!(
        all.iter().any(|v| (v - steady.arrivals).abs() > 0.01),
        "the choice has to change something: {all:?}"
    );
    // Whatever they do, nobody gets a free pass: rates stay within written bounds.
    for r in [steady, relentless, whimsical] {
        assert!(
            (0.0..=2.0).contains(&r.arrivals),
            "arrivals stay sane: {}",
            r.arrivals
        );
        assert!(
            (0.0..=2.0).contains(&r.welcome),
            "welcome stays sane: {}",
            r.welcome
        );
    }
}

#[test]
fn hard_times_close_the_door_under_a_relentless_engine() {
    let calm = engine::rates(Engine::Relentless, 0.1, 100, 7).arrivals;
    let grim = engine::rates(Engine::Relentless, 0.9, 100, 7).arrivals;
    assert!(
        grim < calm,
        "when things are bad, fewer strangers arrive: calm {calm} grim {grim}"
    );
}

#[test]
fn the_steady_engine_keeps_its_temper() {
    let lo = engine::rates(Engine::Steady, 0.1, 100, 7).arrivals;
    let hi = engine::rates(Engine::Steady, 0.9, 100, 7).arrivals;
    assert!(
        (hi - lo).abs() < 0.25,
        "steady is steady whatever happens: lo {lo} hi {hi}"
    );
}

#[test]
fn the_whimsical_engine_swings() {
    let mut lo = f32::MAX;
    let mut hi = f32::MIN;
    for day in 0..20u64 {
        let r = engine::rates(Engine::Whimsical, 0.5, day * TICKS_PER_DAY, 7).arrivals;
        lo = lo.min(r);
        hi = hi.max(r);
    }
    assert!(hi - lo > 0.5, "whimsical varies day to day: {lo} .. {hi}");
}

#[test]
fn tension_reads_the_colonys_actual_state() {
    // A comfortable world reads calm; the same world with everyone starving does not.
    let mut w = World::generate(31, GenParams::small());
    w.found_party(6);
    let calm = engine::tension(&w);
    for a in w.agents.iter_mut() {
        if a.species == hf_sim::agents::Species::Person {
            a.needs.hunger = 0.02;
            a.needs.rest = 0.02;
            a.needs.comfort = 0.0;
        }
    }
    let grim = engine::tension(&w);
    assert!(
        calm < grim,
        "hunger and exhaustion raise tension: {calm} -> {grim}"
    );
    assert!((0.0..=1.0).contains(&grim), "tension is a fraction: {grim}");
}

#[test]
fn the_engine_fires_no_events() {
    // The whole point: an Engine may change how often things happen, never what happened.
    let mut w = World::generate(33, GenParams::small());
    w.meta.engine = Engine::Relentless;
    let mut seen: Vec<String> = Vec::new();
    for _ in 0..TICKS_PER_DAY * 4 {
        w.step();
        for e in &w.last_events {
            seen.push(e.kind_name().to_string());
        }
    }
    assert!(!seen.is_empty(), "the world did something in four days");
    // Every event comes from a system acting; the Engine module itself never records one.
    // (An allowlist of kinds went stale the moment fire and floods arrived, so the check is on
    // the Engine's own source instead.)
    let src = include_str!("../src/engine.rs");
    for needle in ["EventKind", "events.", "pending.push", "emit("] {
        assert!(
            !src.contains(needle),
            "the Engine must not record events, found `{needle}` in engine.rs"
        );
    }
}

#[test]
fn what_the_engine_decides_travels_with_the_save() {
    let mut w = World::generate(37, GenParams::small());
    w.meta.engine = Engine::Whimsical;
    let back = hf_sim::save::snapshot::Snapshot::capture(&w).restore();
    assert_eq!(back.meta.engine, Engine::Whimsical, "the choice is kept");
}
