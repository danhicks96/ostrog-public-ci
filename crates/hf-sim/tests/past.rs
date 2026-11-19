//! Lived pasts (design-organic.md section 1, decision D18): adults arrive having lived, their
//! lives are recorded history before the arrival, and the same upbringing can send siblings
//! opposite ways.

use hf_sim::agents::Species;
use hf_sim::chronicle::Chronicle;
use hf_sim::events::EventKind;
use hf_sim::opening::{identity, PARTY};
use hf_sim::past::{self, Formative, Topic};
use hf_sim::world::gen::GenParams;
use hf_sim::{Session, World};

#[test]
fn past_is_recorded_history() {
    let w = World::opening_world(7);
    let epoch = w.meta.epoch;
    assert!(epoch > 0, "an opening world starts at its arrival");
    assert_eq!(
        w.meta.tick,
        epoch + 7 * hf_sim::time::TICKS_PER_HOUR,
        "they step through at seven"
    );
    let mut s = Session::new(w, Chronicle::in_memory().unwrap());
    s.step().unwrap();
    let rows = s.chronicle.counts_between(0, epoch).unwrap();
    let lived: i64 = rows
        .iter()
        .filter(|(k, _)| k == "lived")
        .map(|(_, n)| *n)
        .sum();
    // Before the arrival: the survivors' own lives, the valley's past (deep.rs) and the wider
    // world's (realm/, M3). Nothing else is recorded there.
    let others: i64 = rows
        .iter()
        .filter(|(k, _)| k != "lived" && !k.starts_with("old_") && !k.starts_with("realm_"))
        .map(|(_, n)| *n)
        .sum();
    assert!(lived > PARTY as i64 * 3, "ten lives leave rows: {lived}");
    assert_eq!(
        others, 0,
        "nothing but lives and the valley's past before the arrival"
    );
    // Every person standing in the world carries their own past, as row ids, and each one is
    // a real row that narrates back to a line about them.
    let w = &s.world;
    for a in w.agents.iter().filter(|a| a.species == Species::Person) {
        assert!(!a.past.is_empty(), "{} arrived with no past", a.name);
        assert!(!a.home.is_empty());
        for id in &a.past {
            let row = s.chronicle.row(*id).unwrap();
            let (hid, tick, detail) = row.expect("the past is in the chronicle");
            assert!(
                tick < epoch,
                "{} remembers something after the arrival",
                a.name
            );
            let e = hf_sim::view::history::event_from_row(hid, tick, &detail).unwrap();
            match &e.kind {
                EventKind::Lived { name, .. } => assert_eq!(name, &a.name),
                k => panic!("a past row that is not a life: {k:?}"),
            }
            let line = hf_sim::view::narrate::line(w, &e);
            assert!(line.contains(&a.name), "the line names them: {line}");
            assert!(line.contains("before the arrival") || line.starts_with("At the portal"));
        }
    }
}

#[test]
fn ages_match_the_lives_they_lived() {
    for seed in 0..20 {
        let w = World::opening_world(seed);
        for a in w.agents.iter().filter(|a| a.species == Species::Person) {
            assert!(a.born >= 0, "born before the world's clock");
            let age = a.age_years(w.meta.tick) as u32;
            assert!((16..=50).contains(&age), "{} is {age}", a.name);
        }
    }
}

/// The reference case: two children of a parent who drank. Over many villages, some pairs of
/// siblings answer the same way and some opposite ways; neither outcome is fixed.
#[test]
fn siblings_diverge() {
    let mut pairs = 0;
    let mut opposite = 0;
    for seed in 0..400u64 {
        let hs = past::households(seed);
        let lives: Vec<_> = (0..PARTY as u8)
            .map(|i| past::life(seed, i, 0, &identity(seed, i, 0).traits))
            .collect();
        for i in 0..PARTY {
            for j in i + 1..PARTY {
                if hs[i] != hs[j] {
                    continue;
                }
                let drank_i = lives[i]
                    .events
                    .iter()
                    .any(|e| matches!(e.what, Formative::ParentDrank { .. }));
                if !drank_i {
                    continue;
                }
                if let (Some(a), Some(b)) =
                    (lives[i].stance(Topic::Drink), lives[j].stance(Topic::Drink))
                {
                    pairs += 1;
                    if (a < 0.0) != (b < 0.0) {
                        opposite += 1;
                    }
                }
            }
        }
    }
    assert!(
        pairs >= 30,
        "enough sibling pairs grew up with drink: {pairs}"
    );
    let share = opposite as f32 / pairs as f32;
    assert!(
        (0.15..=0.85).contains(&share),
        "siblings sometimes go opposite ways, not always: {opposite} of {pairs}"
    );
}

/// Someone who showed them another way (a kind elder) makes rejecting a parent's drinking more
/// likely, and never certain.
#[test]
fn past_shapes_but_does_not_decide() {
    let (mut with_n, mut with_reject, mut without_n, mut without_reject) = (0, 0, 0, 0);
    for seed in 0..600u64 {
        for i in 0..PARTY as u8 {
            let l = past::life(seed, i, 0, &identity(seed, i, 0).traits);
            let Some(s) = l.stance(Topic::Drink) else {
                continue;
            };
            let elder = l
                .events
                .iter()
                .any(|e| matches!(e.what, Formative::KindElder { .. }));
            if elder {
                with_n += 1;
                with_reject += (s < 0.0) as i32;
            } else {
                without_n += 1;
                without_reject += (s < 0.0) as i32;
            }
        }
    }
    let a = with_reject as f32 / with_n.max(1) as f32;
    let b = without_reject as f32 / without_n.max(1) as f32;
    assert!(with_n > 50 && without_n > 50, "{with_n} {without_n}");
    assert!(
        a > b + 0.1,
        "a counter-model shifts the answer: {a:.2} vs {b:.2}"
    );
    assert!(
        a < 0.97 && b > 0.03,
        "but never decides it: {a:.2} vs {b:.2}"
    );
}

#[test]
fn rerolling_keeps_the_family() {
    let seed = 11;
    let hs = past::households(seed);
    for i in 0..PARTY as u8 {
        let a = past::life(seed, i, 0, &identity(seed, i, 0).traits);
        let b = past::life(seed, i, 3, &identity(seed, i, 3).traits);
        assert_eq!(a.household, hs[i as usize]);
        assert_eq!(
            b.household, a.household,
            "a reroll is a different sibling, same house"
        );
        assert_eq!(a.home, b.home, "and the same village");
    }
}

#[test]
fn epoch_zero_unchanged() {
    let w = World::generate(3, GenParams::small());
    assert_eq!(w.meta.epoch, 0);
    assert_eq!(w.meta.tick, 0);
    assert_eq!(w.since_arrival(), 0);
    assert!(w.agents.iter().all(|a| a.past.is_empty()));
}

#[test]
fn the_calendar_counts_from_the_arrival() {
    let w = World::opening_world(5);
    let c = w.calendar();
    assert_eq!(c.year, 0, "the first year here is year 1");
    assert_eq!(c.day_of_year, 0);
}
