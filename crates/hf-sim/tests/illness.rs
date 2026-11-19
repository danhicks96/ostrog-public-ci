//! #328: named sicknesses, each with its cause in what people did and its own course.

use hf_sim::body::{Illness, IllnessKind as K};
use hf_sim::events::EventKind;
use hf_sim::ids::TilePos;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn party(n: usize) -> (World, Vec<usize>) {
    let mut w = World::generate(3281, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    let ids = w.found_party(n);
    w.meta.autoplan = false;
    let idx = ids
        .iter()
        .map(|id| w.agents.iter().position(|a| a.id == *id).unwrap())
        .collect();
    (w, idx)
}

/// Hours of health alone, everyone fed and resting nowhere in particular.
fn hours(w: &mut World, n: u64) {
    for _ in 0..n {
        for a in w.agents.iter_mut() {
            a.needs.hunger = 1.0;
        }
        let t = w.meta.tick;
        w.meta.tick = (t / TICKS_PER_HOUR + 1) * TICKS_PER_HOUR + 600;
        hf_sim::systems::health::step(w);
    }
}

fn sick(w: &mut World, i: usize, k: K) {
    let t = w.meta.tick;
    w.agents[i].body.illness = Some(Illness {
        severity: 0.35,
        since: t,
        kind: k,
        treated: None,
    });
}

#[test]
fn a_sickness_is_named_in_the_record() {
    let (mut w, p) = party(2);
    hf_sim::systems::health::catch(&mut w, p[0], K::Flux);
    assert!(w.events.pending.iter().any(|e| matches!(
        &e.kind,
        EventKind::FellIll { what, .. } if what == "the flux"
    )));
}

#[test]
fn a_fever_had_once_is_not_had_again() {
    let (mut w, p) = party(2);
    let i = p[0];
    sick(&mut w, i, K::Fever);
    // Tended rightly after its two days, it breaks.
    hours(&mut w, 50);
    if let Some(ill) = w.agents[i].body.illness.as_mut() {
        ill.treated = Some(true);
        ill.severity = ill.severity.min(0.5);
    }
    hours(&mut w, 24 * 6);
    assert!(w.agents[i].body.illness.is_none(), "it broke");
    assert!(w.agents[i].immune.contains(&K::Fever));
    hf_sim::systems::health::catch(&mut w, i, K::Fever);
    assert!(w.agents[i].body.illness.is_none(), "not again");
}

#[test]
fn the_wasting_never_wholly_lifts() {
    let (mut w, p) = party(2);
    let i = p[0];
    sick(&mut w, i, K::Wasting);
    if let Some(ill) = w.agents[i].body.illness.as_mut() {
        ill.treated = Some(true);
    }
    hours(&mut w, 24 * 10);
    let ill = w.agents[i].body.illness.expect("still there");
    assert!(ill.severity >= 0.25);
}

#[test]
fn the_flux_turns_only_with_the_right_care() {
    let (mut w, p) = party(3);
    let (a, b) = (p[0], p[1]);
    sick(&mut w, a, K::Flux);
    sick(&mut w, b, K::Flux);
    w.agents[a].body.illness.as_mut().unwrap().treated = Some(true);
    w.agents[b].body.illness.as_mut().unwrap().treated = Some(false);
    hours(&mut w, 48);
    let (sa, sb) = (w.agents[a].body.ill(), w.agents[b].body.ill());
    assert!(sa < sb, "the right care turns it: {sa} against {sb}");
    assert!(sb > 0.35, "a wrong one does not: {sb}");
}

#[test]
fn the_flux_comes_from_the_dead_left_lying() {
    let (mut w, p) = party(8);
    let dead = p[7];
    let did = w.agents[dead].id;
    hf_sim::systems::life::die(&mut w, did, "a test".into());
    let at = w.agents[dead].pos;
    for &k in &p[..7] {
        w.agents[k].pos = at.offset(1, 0, 0);
    }
    // A day on, nobody has buried them.
    let mut caught = 0;
    for d in 1..40u64 {
        w.meta.tick = d * TICKS_PER_DAY + 5 * TICKS_PER_HOUR + 600;
        for r in w.rites_due.iter_mut() {
            r.since = 0;
        }
        w.events.pending.clear();
        hf_sim::systems::health::step(&mut w);
        caught += w
            .events
            .pending
            .iter()
            .filter(|e| matches!(&e.kind, EventKind::FellIll { what, .. } if what == "the flux"))
            .count();
    }
    assert!(caught > 0, "nobody took the flux beside the unburied");
}

#[test]
fn the_sick_lie_down_and_their_family_nurse_them_without_a_poultice() {
    // #418: on seed 3 three died of a fever in 23 to 32 days, a tenth of it in bed and never
    // tended, with healers of their own people near for most of it.
    let (mut w, p) = party(2);
    let (a, b) = (p[0], p[1]);
    let (ia, ib) = (w.agents[a].id, w.agents[b].id);
    w.agents[a].partner = Some(ib);
    w.agents[b].partner = Some(ia);
    w.items
        .retain(|it| it.kind != hf_sim::items::ItemKind::Poultice);
    sick(&mut w, a, K::Fever);
    w.agents[a].body.illness.as_mut().unwrap().severity = 0.5;
    let mut nursed = 0;
    let mut lying = 0;
    for _ in 0..48 {
        for _ in 0..TICKS_PER_HOUR {
            w.step();
            for e in &w.last_events {
                if let EventKind::Tended { healer, patient } = e.kind {
                    nursed += (healer == ib && patient == ia) as u32;
                }
            }
        }
        let h = (w.meta.tick % TICKS_PER_DAY) / TICKS_PER_HOUR;
        if (8..20).contains(&h) && w.agents[a].activity.is_asleep() {
            lying += 1;
        }
    }
    assert!(nursed >= 1, "their partner nursed them");
    assert!(
        lying >= 12,
        "they lay down by day: {lying} of 24 daylight hours"
    );
}
