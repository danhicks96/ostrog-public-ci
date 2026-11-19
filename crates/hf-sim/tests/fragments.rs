//! #508 (epic #508, WP-07 7.6): the ancient-fragment potential surfaces exactly one
//! bounded piece of the old world in its carrier — never a system, never a reason. It is
//! taught in talk with blur, withheld in silence, lost with its last carrier, and when two
//! copies of one fragment disagree, both are kept: a contested record (mind/fragment.rs).

use hf_sim::events::EventKind;
use hf_sim::mind::fragment::{HeldFragment, Learned};
use hf_sim::resonance::{self, Potential, Resonance};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

/// A flat meadow with two people standing together: `carrier` (one sitting from waking)
/// and `other`. Returns their indices.
fn pair(seed: u64) -> (World, usize, usize) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.tick = 7 * hf_sim::time::TICKS_PER_DAY + hf_sim::time::TICKS_PER_HOUR * 8;
    let ids = w.found_party(2);
    let c = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let n = w.agents.iter().position(|a| a.id == ids[1]).unwrap();
    w.agents[c].resonance = Some(Resonance {
        potential: Potential::AncientFragment,
        sat: resonance::WAKES_AFTER,
        woke: None,
        refused: 0,
        first_manifestation: None,
    });
    (w, c, n)
}

/// Wake the carrier at the stone: the world sees the unsettled coming-back, the carrier
/// comes away holding the fragment.
fn wake(w: &mut World, c: usize) -> HeldFragment {
    let before = w.events.pending.len();
    resonance::sat_by_stone(w, c);
    let news = &w.events.pending[before..];
    assert_eq!(
        news.len(),
        1,
        "the waking surfaces as the one quiet event, nothing naming the content"
    );
    assert!(
        matches!(news[0].kind, EventKind::CameBackUnsettled { .. }),
        "the world sees the murmuring, not the fragment: {:?}",
        news[0].kind
    );
    assert_eq!(
        w.agents[c].fragments.len(),
        1,
        "exactly one bounded fragment"
    );
    w.agents[c].fragments[0]
}

#[test]
fn the_woken_carrier_holds_exactly_one_bounded_fragment() {
    let (mut w, c, _) = pair(1);
    let f = wake(&mut w, c);
    assert!(f.origin.is_none(), "this IS the original");
    assert_eq!(f.learned, Learned::Woke);
    assert_eq!(f.tick, w.meta.tick);
    let m = f.mark.expect("a woken fragment names which one it is");
    assert!(
        m < HeldFragment::span(f.what),
        "the mark is inside the kind's range"
    );
    let cautious = w.agents[c]
        .dispositions
        .map(|d| d.caution > 0.75)
        .unwrap_or(false);
    assert_eq!(
        f.tells, !cautious,
        "only the very cautious keep it to themselves"
    );
    // Bounded means bounded: the record names no system, no mechanism, no potential.
    let debug = format!("{:?}", f);
    for word in [
        "Potential",
        "Resonance",
        "AncientFragment",
        "mechanism",
        "system",
    ] {
        assert!(
            !debug.contains(word),
            "{word} leaked into the fragment record: {debug}"
        );
    }
}

#[test]
fn the_fragment_draw_is_deterministic_for_seed_and_carrier() {
    let (w1, c1, _) = pair(5);
    let (w2, c2, _) = pair(5);
    let mut a = w1;
    let mut b = w2;
    let fa = wake(&mut a, c1);
    let fb = wake(&mut b, c2);
    assert_eq!(fa.what, fb.what, "same seed, same carrier, same fragment");
    assert_eq!(fa.mark, fb.mark, "same seed, same carrier, same mark");
}

#[test]
fn a_fragment_survives_save_and_load_unchanged() {
    let (mut w, c, _) = pair(9);
    wake(&mut w, c);
    let bytes = hf_sim::save::encode(&hf_sim::save::snapshot::Snapshot::capture(&w)).unwrap();
    let back = hf_sim::save::decode(&bytes).unwrap().restore();
    assert_eq!(
        back.agents[c].fragments, w.agents[c].fragments,
        "held, not rerolled"
    );
}

#[test]
fn a_fragment_is_taught_in_talk_and_travels_with_its_origin() {
    let (mut w, c, _) = pair(3);
    let f = wake(&mut w, c);
    w.agents[c].fragments[0].tells = true;
    let ids = w.found_party(1);
    let l = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    assert!(w.agents[l].fragments.is_empty());
    let tick = w.tick();
    hf_sim::systems::beliefs::share(&mut w, c, l, tick);
    let heard = &w.agents[l].fragments;
    assert_eq!(heard.len(), 1, "the fragment passed on as one thing");
    assert_eq!(heard[0].what, f.what, "the kind travels whole");
    assert_eq!(
        heard[0].origin,
        Some((w.agents[c].id, f.tick)),
        "the copy points at the woken original"
    );
    assert_eq!(heard[0].learned, Learned::Taught);
    assert!(heard[0].tells, "a heard story may be told on");
}

#[test]
fn a_close_listener_hears_it_exactly() {
    let (mut w, c, _) = pair(4);
    let f = wake(&mut w, c);
    w.agents[c].fragments[0].tells = true;
    let ids = w.found_party(1);
    let l = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let cid = w.agents[c].id;
    w.agents[l].relationships.insert(
        cid,
        hf_sim::mind::memory::Relation {
            opinion: 0.0,
            familiarity: 1.0,
            last_contact: 0,
            met: 0,
        },
    );
    let tick = w.tick();
    hf_sim::systems::beliefs::share(&mut w, c, l, tick);
    assert_eq!(
        w.agents[l].fragments[0].mark, f.mark,
        "a close telling loses nothing"
    );
}

#[test]
fn a_strangers_telling_blurs_which_one_it_was() {
    for seed in 1..=8 {
        let (mut w, c, _) = pair(seed);
        let f = wake(&mut w, c);
        let Some(m) = f.mark else {
            continue;
        };
        if m < 2 {
            continue;
        }
        w.agents[c].fragments[0].tells = true;
        let ids = w.found_party(1);
        let l = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
        let tick = w.tick();
        hf_sim::systems::beliefs::share(&mut w, c, l, tick);
        // At the harness tick a stranger's telling drifts two marks down (blur 2 at
        // familiarity 0; deterministic — the guard pins the behaviour).
        assert_eq!(
            w.agents[l].fragments[0].mark,
            Some(m - 2),
            "seed {seed}: a stranger hears it a little wrong"
        );
        return;
    }
    panic!("no seed in 1..=8 gave a mark worth blurring");
}

#[test]
fn every_fifth_retelling_loses_which_one_it_was() {
    let (mut w, c, _) = pair(6);
    let f = wake(&mut w, c);
    assert!(f.mark.is_some());
    w.agents[c].fragments[0].tells = true;
    let ids = w.found_party(1);
    let l = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    // The fifth-day cadence: the kind survives, the mark does not.
    hf_sim::systems::beliefs::share(&mut w, c, l, 5 * hf_sim::time::TICKS_PER_DAY);
    let heard = &w.agents[l].fragments;
    assert_eq!(heard.len(), 1, "the fragment itself still travels");
    assert_eq!(heard[0].what, f.what, "the kind is not lost");
    assert_eq!(heard[0].mark, None, "which one it was is gone");
}

#[test]
fn a_withheld_fragment_does_not_travel() {
    let (mut w, c, _) = pair(7);
    wake(&mut w, c);
    w.agents[c].fragments[0].tells = false;
    let ids = w.found_party(1);
    let l = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    let tick = w.tick();
    hf_sim::systems::beliefs::share(&mut w, c, l, tick);
    assert!(
        w.agents[l].fragments.is_empty(),
        "a withheld fragment stays with its holder — and dies there"
    );
}

#[test]
fn two_drifted_copies_of_one_fragment_are_both_kept() {
    for seed in 1..=8 {
        let (mut w, c, _) = pair(seed);
        let f = wake(&mut w, c);
        let Some(m) = f.mark else {
            continue;
        };
        if m < 2 {
            continue;
        }
        w.agents[c].fragments[0].tells = true;
        let ids = w.found_party(1);
        let l = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
        let tick1 = w.tick();
        hf_sim::systems::beliefs::share(&mut w, c, l, tick1);
        let first = w.agents[l].fragments[0];
        assert_eq!(
            first.mark,
            Some(m - 2),
            "seed {seed}: the first hearing drifts two"
        );
        // The carrier tells it again, differently (a stranger's second hearing shifts it
        // differently). Both copies stand: no arbiter decides between them. The carrier's
        // own list is trimmed back to the original so the second telling is one voice.
        w.agents[c].fragments.truncate(1);
        hf_sim::systems::beliefs::share(&mut w, c, l, tick1 + 5);
        let held = &w.agents[l].fragments;
        assert_eq!(held.len(), 2, "seed {seed}: the two tellings are both kept");
        assert_ne!(held[0].mark, held[1].mark, "and they disagree");
        for x in held {
            assert_eq!(
                x.origin,
                Some((w.agents[c].id, f.tick)),
                "one fragment, two records"
            );
        }
        return;
    }
    panic!("no seed in 1..=8 gave a mark worth contesting");
}

#[test]
fn the_held_fragment_list_is_capped_and_drops_the_oldest() {
    let (mut w, c, _) = pair(11);
    wake(&mut w, c);
    let tick = w.meta.tick;
    let base = w.agents[c].fragments[0];
    for i in 0..(hf_sim::mind::fragment::FRAGMENT_CAP as u64 + 2) {
        hf_sim::mind::fragment::remember(
            &mut w.agents[c].fragments,
            hf_sim::mind::fragment::HeldFragment {
                what: base.what,
                mark: Some(
                    (i % hf_sim::mind::fragment::HeldFragment::span(base.what) as u64) as u8,
                ),
                origin: Some((hf_sim::ids::AgentId(i as u32), tick)),
                learned: hf_sim::mind::fragment::Learned::Taught,
                tick: tick + i,
                tells: true,
            },
        );
    }
    let held = &w.agents[c].fragments;
    assert_eq!(
        held.len(),
        hf_sim::mind::fragment::FRAGMENT_CAP,
        "the list never grows past the cap"
    );
    assert!(
        !held.iter().any(|x| x.origin.is_none()),
        "the woken original was the oldest and fell off"
    );
    assert_eq!(
        held.last().unwrap().origin,
        Some((
            hf_sim::ids::AgentId((hf_sim::mind::fragment::FRAGMENT_CAP as u64 + 1) as u32),
            tick
        )),
        "the newest telling survives"
    );
}
