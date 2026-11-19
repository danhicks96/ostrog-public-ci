//! #297: faith. The deity's answers are read aloud by the one followed in the spirit, heeded by
//! most and scoffed at by one who holds signs are only weather.

use hf_sim::deity::{Answer, Lever, Scope, Trigger};
use hf_sim::events::{EventKind, Severity};
use hf_sim::ids::{AgentId, TilePos};
use hf_sim::justice::Domain;
use hf_sim::mind::beliefs::{Source, Topic};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

/// A party of `n` standing together at (30, 30), the first followed in the spirit.
fn valley(seed: u64, n: usize) -> (World, Vec<AgentId>) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(n);
    w.meta.autoplan = false;
    w.communities[0].leaders[Domain::Spirit as usize] = Some(ids[0]);
    for (k, id) in ids.iter().enumerate() {
        let i = w.agents.iter().position(|a| a.id == *id).unwrap();
        w.agents[i].pos = w.grid.surface(30 + k as i32, 30);
        w.agents[i].activity = hf_sim::mind::activity::Activity::Idle;
    }
    (w, ids)
}

fn idx(w: &World, id: AgentId) -> usize {
    w.agents.iter().position(|a| a.id == id).unwrap()
}

/// Move this tick's pending events to `last_events`, as the flush does.
fn flush(w: &mut World) {
    w.last_events = w.events.drain();
}

#[test]
fn the_deitys_answer_is_read_aloud_heeded_and_scoffed_at() {
    let (mut w, ids) = valley(2972, 4);
    let (reader, doubter) = (ids[0], ids[3]);
    // One holds that signs are only weather; the others hold nothing yet.
    for &id in &ids[1..3] {
        let i = idx(&w, id);
        w.agents[i]
            .beliefs
            .list
            .retain(|b| b.topic != Topic::DeitySigns);
    }
    let d = idx(&w, doubter);
    w.agents[d]
        .beliefs
        .hold(Topic::DeitySigns, -0.8, 0.7, Source::Culture(0), 0);
    // The deity answers the valley's plenty with the cold.
    let tick = w.meta.tick;
    let ev = w.events.push(
        tick,
        None,
        Severity::Trace,
        EventKind::DeityAnswered {
            deity: "Hask".into(),
            trigger: "plenty".into(),
            lever: "the cold".into(),
            place: "the valley".into(),
            strength: -4.0,
            days: 4,
        },
    );
    w.deity_answers.push(Answer {
        trigger: Trigger::Plenty,
        lever: Lever::Cold,
        scope: Scope::Valley,
        strength: -4.0,
        from: tick,
        until: tick + 4 * hf_sim::time::TICKS_PER_DAY,
        event: ev,
    });
    w.meta.engine = hf_sim::engine::Engine::Relentless;
    flush(&mut w);
    hf_sim::faith::observe(&mut w);
    let read = w
        .events
        .pending
        .iter()
        .find_map(|e| match &e.kind {
            EventKind::DeityRead {
                reader: r,
                words,
                good,
                ..
            } if *r == reader => Some((words.clone(), *good)),
            _ => None,
        })
        .expect("the one followed in the spirit read the answer");
    assert!(read.0.starts_with("Hask "), "{}", read.0);
    assert!(!read.1, "the cold is read as a hardness");
    // Next tick it is heard.
    flush(&mut w);
    hf_sim::faith::observe(&mut w);
    assert!(
        w.events.pending.iter().any(
            |e| matches!(e.kind, EventKind::Scoffed { by, at } if by == doubter && at == reader)
        ),
        "the doubter scoffs"
    );
    let d = idx(&w, doubter);
    assert!(w.agents[d].opinion_of(reader) < 0.0);
    // Those who did not doubt took it as the reader holds it, told by them.
    let heeded = ids[1..3].iter().any(|&id| {
        let a = &w.agents[idx(&w, id)];
        a.beliefs.lean(Topic::DeitySigns) > 0.0
            && a.beliefs
                .get(Topic::DeitySigns)
                .unwrap()
                .sources
                .contains(&Source::Person(reader))
    });
    assert!(heeded, "someone heeded the reading");
    assert_eq!(w.faith.signs.len(), 1);
}

#[test]
fn faith_is_a_persons_own() {
    // Inclinations differ between people of one community, and stay the same for one person.
    let (w, ids) = valley(2973, 10);
    let v: Vec<f32> = ids
        .iter()
        .map(|&id| hf_sim::faith::inclination(&w, id))
        .collect();
    assert!(v.iter().any(|x| *x < -0.4) || v.iter().any(|x| *x > 0.4));
    assert!(
        v.iter().cloned().fold(f32::MIN, f32::max) - v.iter().cloned().fold(f32::MAX, f32::min)
            > 0.5
    );
    assert_eq!(v[0], hf_sim::faith::inclination(&w, ids[0]));
}

/// Open a sign read by `reader` to the first community.
fn open_sign(w: &mut World, reader: AgentId, good: bool) {
    let tick = w.meta.tick;
    let event = w.events.push(
        tick,
        None,
        Severity::Notable,
        EventKind::Omen {
            reader,
            of: "a kind season".into(),
            good,
        },
    );
    let community = w.communities[0].id;
    w.faith.signs.push(hf_sim::faith::SignRead {
        reader,
        community,
        good,
        of: "a kind season".into(),
        event,
        until: tick + hf_sim::faith::SIGN_OPEN,
    });
}

/// `who` dies this tick, on record.
fn dies(w: &mut World, who: AgentId) {
    let i = idx(w, who);
    w.agents[i].alive = false;
    let tick = w.meta.tick;
    w.events.push(
        tick,
        None,
        Severity::Major,
        EventKind::Died {
            agent: who,
            cause: "fever".into(),
        },
    );
}

#[test]
fn a_blessing_belied_by_a_death_shakes_faith_and_the_reader() {
    let (mut w, ids) = valley(2974, 4);
    let (reader, wavering, dead) = (ids[0], ids[1], ids[3]);
    for _ in 0..3 {
        open_sign(&mut w, reader, true);
    }
    // The signs were heard long ago; only the death is new.
    w.events.drain();
    let k = idx(&w, wavering);
    w.agents[k]
        .beliefs
        .hold(Topic::DeitySigns, 0.2, 0.45, Source::Culture(0), 0);
    w.agents[k].defers = vec![(reader, [0.0, 0.0, 1.0, 0.0])];
    dies(&mut w, dead);
    flush(&mut w);
    hf_sim::faith::observe(&mut w);
    assert!(w.faith.signs.is_empty(), "each sign settled once");
    let settled = w
        .events
        .pending
        .iter()
        .filter(
            |e| matches!(e.kind, EventKind::SignSettled { good: true, dead: d, .. } if d == dead),
        )
        .count();
    assert_eq!(settled, 3);
    assert_eq!(
        w.faith.record_of(reader),
        (0, 3),
        "kept against the reader's name"
    );
    let k = idx(&w, wavering);
    assert!(w.agents[k].beliefs.lean(Topic::DeitySigns) < -0.08);
    assert!(
        w.agents[k].defers[0].1[2] < 1.0,
        "the reader is followed the less"
    );
    assert!(w.events.pending.iter().any(
        |e| matches!(e.kind, EventKind::FaithTurned { agent, heeds: false } if agent == wavering)
    ));
}

#[test]
fn a_warning_borne_out_moves_a_doubter_toward_the_signs() {
    let (mut w, ids) = valley(2975, 4);
    let (reader, doubter, dead) = (ids[0], ids[1], ids[3]);
    w.events.drain();
    open_sign(&mut w, reader, false);
    let k = idx(&w, doubter);
    w.agents[k]
        .beliefs
        .hold(Topic::DeitySigns, -0.5, 0.5, Source::Culture(0), 0);
    let before = w.agents[k].beliefs.lean(Topic::DeitySigns);
    dies(&mut w, dead);
    flush(&mut w);
    hf_sim::faith::observe(&mut w);
    assert!(w.agents[k].beliefs.lean(Topic::DeitySigns) > before);
    assert!(w.events.pending.iter().any(
        |e| matches!(e.kind, EventKind::SignSettled { good: false, reader: r, .. } if r == reader)
    ));
    // A sign whose days pass with nothing is settled quietly, with no event.
    open_sign(&mut w, reader, false);
    w.events.drain();
    w.last_events.clear();
    w.meta.tick += hf_sim::faith::SIGN_OPEN;
    hf_sim::faith::observe(&mut w);
    assert!(w.faith.signs.is_empty());
    assert!(!w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::SignSettled { .. })));
}

#[test]
fn a_seasons_observance_is_kept_and_then_let_lapse() {
    use hf_sim::mind::activity::Activity;
    use hf_sim::mind::ThoughtKind;
    use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
    let (mut w, ids) = valley(2976, 4);
    let leader = ids[0];
    w.events.drain();
    // Noon of the first day of summer.
    w.meta.tick = w.meta.epoch + 5 * TICKS_PER_DAY + 12 * TICKS_PER_HOUR;
    hf_sim::observances::step(&mut w);
    let came = w
        .events
        .pending
        .iter()
        .find_map(|e| match e.kind {
            EventKind::Observance { by, came, .. } if by == leader => Some(came),
            _ => None,
        })
        .expect("the one followed in the spirit led the observance");
    assert_eq!(came, 3);
    let k = idx(&w, ids[2]);
    assert!(w.agents[k].thoughts.has(ThoughtKind::KeptObservance));
    assert!(w.agents[k].beliefs.lean(Topic::DeitySigns) > 0.0);
    assert_eq!(w.faith.kept.len(), 1);
    // By autumn nobody is followed in the spirit and everyone awake holds signs are only
    // weather; the one who heeds them is asleep.
    w.communities[0].leaders[Domain::Spirit as usize] = None;
    for &id in &ids {
        let i = idx(&w, id);
        w.agents[i]
            .beliefs
            .hold(Topic::DeitySigns, -0.3, 0.5, Source::Culture(0), 0);
    }
    let believer = idx(&w, ids[1]);
    w.agents[believer]
        .beliefs
        .hold(Topic::DeitySigns, 0.8, 0.8, Source::Culture(0), 0);
    w.agents[believer].activity = Activity::Sleeping {
        bed: None,
        until: u64::MAX,
    };
    w.events.drain();
    w.meta.tick += 5 * TICKS_PER_DAY;
    hf_sim::observances::step(&mut w);
    assert!(w.events.pending.iter().any(|e| matches!(
        &e.kind,
        EventKind::ObservanceLapsed { why, .. } if why == "nobody led it"
    )));
    assert!(w.agents[believer]
        .thoughts
        .has(ThoughtKind::ObservanceLapsed));
    assert!(w.faith.kept.is_empty());
    // A lapse is told once: the next season, still unkept, says nothing.
    w.events.drain();
    w.meta.tick += 5 * TICKS_PER_DAY;
    hf_sim::observances::step(&mut w);
    assert!(!w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::ObservanceLapsed { .. })));
}

#[test]
fn a_young_community_with_nobody_followed_keeps_its_observance_led_by_the_most_devout() {
    use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
    // Founders through the portal hold no view on signs, and nobody is followed in the spirit.
    let (mut w, ids) = valley(2985, 6);
    w.communities[0].leaders[Domain::Spirit as usize] = None;
    for &id in &ids {
        let i = idx(&w, id);
        w.agents[i]
            .beliefs
            .list
            .retain(|b| b.topic != Topic::DeitySigns);
    }
    // Somewhere about the place, not standing at home, with ten days of meals in store (a
    // community short of food does not gather, #415).
    let home = w.communities[0].home;
    w.drop_item(hf_sim::items::ItemKind::Meal, 180, home.offset(3, 3, 0));
    for (k, &id) in ids.iter().enumerate() {
        let i = idx(&w, id);
        w.agents[i].pos = w.grid.surface(home.x + 8 + k as i32, home.y + 6);
    }
    let devout = ids
        .iter()
        .copied()
        .max_by(|a, b| {
            hf_sim::faith::inclination(&w, *a)
                .total_cmp(&hf_sim::faith::inclination(&w, *b))
                .then(b.cmp(a))
        })
        .unwrap();
    assert!(hf_sim::faith::inclination(&w, devout) > 0.0);
    w.events.drain();
    let day = w.meta.epoch + 5 * TICKS_PER_DAY;
    // An hour before noon they go home to keep it.
    w.meta.tick = day + 11 * TICKS_PER_HOUR;
    hf_sim::observances::step(&mut w);
    let going = w
        .agents
        .iter()
        .filter(|a| matches!(&a.activity, hf_sim::mind::activity::Activity::Going { to, .. } if to.dist(home) <= 3))
        .count();
    assert_eq!(going, ids.len());
    for &id in &ids {
        let i = idx(&w, id);
        w.agents[i].pos = w.grid.surface(home.x + 1, home.y + 1);
        w.agents[i].activity = hf_sim::mind::activity::Activity::Idle;
    }
    w.meta.tick = day + 12 * TICKS_PER_HOUR;
    hf_sim::observances::step(&mut w);
    let kept = w.events.pending.iter().find_map(|e| match e.kind {
        EventKind::Observance { by, came, .. } => Some((by, came)),
        _ => None,
    });
    assert_eq!(kept, Some((devout, ids.len() as u32 - 1)));
    // Those who kept it now lean toward the signs.
    let i = idx(&w, ids[0]);
    assert!(w.agents[i].beliefs.lean(Topic::DeitySigns) > 0.0);
}
