//! #295: gossip carries what people think of someone; good turns are repaid; the estranged
//! make it up.

use hf_sim::events::{Event, EventKind, Severity};
use hf_sim::ids::{AgentId, TilePos};
use hf_sim::items::{Item, ItemKind, ItemPlace};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn court_event(id: u32, tick: u64, kind: EventKind) -> Event {
    Event {
        id: hf_sim::ids::EventId(id),
        tick,
        at: None,
        severity: Severity::Minor,
        kind,
    }
}

#[test]
fn courtship_stats_from_a_built_log() {
    let (a, b) = (AgentId(1), AgentId(2));
    let mut events = vec![
        court_event(
            1,
            0,
            EventKind::Arrived {
                agent: a,
                community: None,
            },
        ),
        court_event(
            2,
            0,
            EventKind::Arrived {
                agent: b,
                community: None,
            },
        ),
    ];
    for (index, day) in [0, 1, 4, 8].into_iter().enumerate() {
        let (a, b) = if index % 2 == 0 { (a, b) } else { (b, a) };
        events.push(court_event(
            index as u32 + 3,
            day * TICKS_PER_DAY,
            EventKind::Talked { a, b, warmth: 0.5 },
        ));
    }
    events.push(court_event(
        7,
        9 * TICKS_PER_DAY,
        EventKind::Paired { a, b },
    ));
    let stats = hf_sim::social::courtship_stats(&events);
    assert_eq!(stats.pairs, 1);
    assert_eq!(stats.timed_pairs, 1);
    assert_eq!(stats.days_median, 9.0);
    assert_eq!(stats.talks_median, 4.0);
    assert_eq!(stats.first_ten_days, 1);
    assert_eq!(stats.unknown_arrival, 0);
}

#[test]
fn courtship_quartiles_exclude_pairings_with_no_recorded_talk() {
    let mut events = Vec::new();
    for pair in 0..4 {
        events.push(court_event(
            pair + 1,
            0,
            EventKind::Talked {
                a: AgentId(2 * pair + 1),
                b: AgentId(2 * pair + 2),
                warmth: 0.5,
            },
        ));
    }
    for (pair, day) in [2, 4, 8, 10].into_iter().enumerate() {
        events.push(court_event(
            pair as u32 + 5,
            day * TICKS_PER_DAY,
            EventKind::Paired {
                a: AgentId(2 * pair as u32 + 1),
                b: AgentId(2 * pair as u32 + 2),
            },
        ));
    }
    events.push(court_event(
        9,
        20 * TICKS_PER_DAY,
        EventKind::Paired {
            a: AgentId(50),
            b: AgentId(51),
        },
    ));
    let stats = hf_sim::social::courtship_stats(&events);
    assert_eq!(
        (stats.pairs, stats.timed_pairs, stats.missing_talk),
        (5, 4, 1)
    );
    assert_eq!(
        (stats.days_p25, stats.days_median, stats.days_p75),
        (3.5, 6.0, 8.5)
    );
    assert_eq!(stats.talks_median, 1.0);
    assert_eq!(stats.unknown_arrival, 5);
}

/// A meal `who` carries, for the real-food-sharing tests.
fn carry_meal(w: &mut World, who: AgentId) -> hf_sim::ids::ItemId {
    let id = w.alloc_item_id();
    w.items.push(Item {
        id,
        kind: ItemKind::Meal,
        count: 1,
        place: ItemPlace::Carried(who),
        quality: 0.8,
        material: hf_sim::world::Material::AIR,
        story: None,
        rot: 0.0,
    });
    id
}

fn party(n: usize) -> (World, Vec<usize>) {
    let mut w = World::generate(2951, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 10;
    w.rebuild_caches();
    let ids = w.found_party(n);
    w.meta.autoplan = false;
    let idx = ids
        .iter()
        .map(|id| w.agents.iter().position(|a| a.id == *id).unwrap())
        .collect();
    (w, idx)
}

fn happen(w: &mut World, kind: EventKind) {
    let tick = w.meta.tick;
    let id = w.events.push(tick, None, Severity::Minor, kind);
    let e = w
        .events
        .pending
        .iter()
        .find(|e| e.id == id)
        .unwrap()
        .clone();
    w.last_events = vec![e];
    hf_sim::social::observe(w);
}

#[test]
fn solitary_get_lonely_slower() {
    // #947: the `.max(0.0)` on the sociability extra threw away every rate below 1, so a
    // solitary person's social need fell exactly as fast as a sociable one's.
    let (mut w, p) = party(2);
    for (i, sociability) in [(p[0], 0.1_f32), (p[1], 0.5_f32)] {
        w.agents[i].dispositions = Some(hf_sim::mind::Dispositions {
            sociability,
            ..Default::default()
        });
        w.agents[i].needs.social = 1.0;
        w.agents[i].activity = hf_sim::mind::Activity::Idle;
    }
    for _ in 0..12 * TICKS_PER_HOUR {
        w.meta.tick += 1;
        hf_sim::systems::needs::step(&mut w);
    }
    let (solitary, average) = (w.agents[p[0]].needs.social, w.agents[p[1]].needs.social);
    assert!(
        solitary - average >= 0.15,
        "the solitary one stays lonelier for longer: solitary {solitary}, average {average}"
    );
}

#[test]
fn a_bad_name_travels_by_talk() {
    let (mut w, p) = party(4);
    let (teller, listener, subject) = (w.agents[p[0]].id, w.agents[p[1]].id, w.agents[p[2]].id);
    let t = w.meta.tick;
    w.agents[p[0]].relate(subject, -0.9, 0.5, t);
    w.agents[p[1]].relate(teller, 0.5, 0.5, t);
    let before = w.agents[p[1]].opinion_of(subject);
    for _ in 0..5 {
        happen(
            &mut w,
            EventKind::Talked {
                a: teller,
                b: listener,
                warmth: 0.5,
            },
        );
    }
    let after = w.agents[p[1]].opinion_of(subject);
    assert!(after < before - 0.2, "{before} -> {after}");
    assert!(hf_sim::social::reputation(&w, subject) < 0.0);
}

#[test]
fn repaying_a_good_turn_moves_a_real_meal() {
    let (mut w, p) = party(3);
    let (healer, patient) = (w.agents[p[0]].id, w.agents[p[1]].id);
    happen(&mut w, EventKind::Tended { healer, patient });
    assert!(w.agents[p[1]].owes.iter().any(|x| x.0 == healer));
    w.agents[p[0]].needs.hunger = 0.1;
    w.agents[p[1]].needs.hunger = 0.9;
    w.agents[p[0]].pos = w.agents[p[1]].pos;
    w.items.clear();
    let meal = carry_meal(&mut w, patient);
    w.meta.tick = TICKS_PER_DAY + 12 * TICKS_PER_HOUR + 41;
    hf_sim::social::step(&mut w);
    assert_eq!(
        w.items.iter().find(|it| it.id == meal).map(|it| it.place),
        Some(ItemPlace::Carried(healer)),
        "the meal moved from the giver to the one who was owed"
    );
    assert_eq!(
        w.agents[p[1]].needs.hunger, 0.9,
        "no hunger number is set directly"
    );
    assert_eq!(
        w.agents[p[0]].needs.hunger, 0.1,
        "no hunger number is set directly"
    );
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Repaid { by, to } if by == patient && to == healer)));
}

#[test]
fn no_good_turn_is_repaid_across_a_feud() {
    let (mut w, p) = party(3);
    let (healer, patient) = (w.agents[p[0]].id, w.agents[p[1]].id);
    happen(&mut w, EventKind::Tended { healer, patient });
    hf_sim::feuds::begin(&mut w, healer, patient);
    w.agents[p[0]].needs.hunger = 0.1;
    w.agents[p[1]].needs.hunger = 0.9;
    w.agents[p[0]].pos = w.agents[p[1]].pos;
    w.items.clear();
    let meal = carry_meal(&mut w, patient);
    w.meta.tick = TICKS_PER_DAY + 12 * TICKS_PER_HOUR + 41;
    hf_sim::social::step(&mut w);
    assert_eq!(
        w.items.iter().find(|it| it.id == meal).map(|it| it.place),
        Some(ItemPlace::Carried(patient)),
        "nothing is shared across a feud"
    );
    assert!(!w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Repaid { .. })));
}

#[test]
fn with_no_food_to_hand_nothing_is_repaid() {
    let (mut w, p) = party(3);
    let (healer, patient) = (w.agents[p[0]].id, w.agents[p[1]].id);
    happen(&mut w, EventKind::Tended { healer, patient });
    w.agents[p[0]].needs.hunger = 0.1;
    w.agents[p[1]].needs.hunger = 0.9;
    w.agents[p[0]].pos = w.agents[p[1]].pos;
    w.items.clear();
    w.meta.tick = TICKS_PER_DAY + 12 * TICKS_PER_HOUR + 41;
    hf_sim::social::step(&mut w);
    assert!(
        w.agents[p[1]].owes.iter().any(|x| x.0 == healer),
        "the debt stands"
    );
    assert!(!w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Repaid { .. })));
}

#[test]
fn no_meal_jumps_across_the_camp() {
    let (mut w, p) = party(3);
    let (healer, patient) = (w.agents[p[0]].id, w.agents[p[1]].id);
    happen(&mut w, EventKind::Tended { healer, patient });
    w.agents[p[0]].needs.hunger = 0.1;
    w.agents[p[1]].needs.hunger = 0.9;
    let here = w.agents[p[1]].pos;
    w.agents[p[0]].pos = TilePos::new(here.x + 10, here.y, here.z);
    w.items.clear();
    let meal = carry_meal(&mut w, patient);
    w.meta.tick = TICKS_PER_DAY + 12 * TICKS_PER_HOUR + 41;
    hf_sim::social::step(&mut w);
    assert_eq!(
        w.items.iter().find(|it| it.id == meal).map(|it| it.place),
        Some(ItemPlace::Carried(patient)),
        "no meal jumps 10 tiles across the camp"
    );
    assert!(!w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Repaid { .. })));
    // At 1 tile, the same debt is repaid with a real meal.
    w.agents[p[0]].pos = TilePos::new(here.x + 1, here.y, here.z);
    hf_sim::social::step(&mut w);
    assert_eq!(
        w.items.iter().find(|it| it.id == meal).map(|it| it.place),
        Some(ItemPlace::Carried(healer)),
        "at 1 tile the meal is shared"
    );
}

#[test]
fn two_at_odds_in_good_spirits_sometimes_make_it_up() {
    let (mut w, p) = party(2);
    let (a, b) = (w.agents[p[0]].id, w.agents[p[1]].id);
    let mut made = false;
    for k in 0..60u64 {
        let t = w.meta.tick + k;
        w.meta.tick = t;
        w.agents[p[0]].relate(b, -2.0, 0.1, t);
        w.agents[p[0]].relate(b, 0.6, 0.0, t);
        w.agents[p[1]].relate(a, -2.0, 0.1, t);
        w.agents[p[1]].relate(a, 0.6, 0.0, t);
        w.agents[p[0]].mood = 0.6;
        w.agents[p[1]].mood = 0.6;
        happen(&mut w, EventKind::Talked { a, b, warmth: 0.4 });
        if w.events
            .pending
            .iter()
            .any(|e| matches!(e.kind, EventKind::MadeUp { .. }))
        {
            made = true;
            break;
        }
    }
    assert!(made);
}

/// #1131 (2026-11-15 fix-1131-met-stamp-real-contact): hearing someone talked about moves
/// `last_contact` ("last heard of") but must not stamp `met`, the in-person-only marker.
#[test]
fn gossip_does_not_count_as_meeting() {
    let (mut w, p) = party(3);
    let (teller, listener, subject) = (w.agents[p[0]].id, w.agents[p[1]].id, w.agents[p[2]].id);
    let t0 = w.meta.tick;
    // The teller feels strongly about the absent subject, so gossip has something to say.
    w.agents[p[0]].relate(subject, -0.9, 0.5, t0);
    assert_eq!(
        w.agents[p[1]].relationships.get(&subject).map(|r| r.met),
        None
    );
    for k in 0..10u64 {
        w.meta.tick = t0 + k * TICKS_PER_DAY;
        happen(
            &mut w,
            EventKind::Talked {
                a: teller,
                b: listener,
                warmth: 0.5,
            },
        );
    }
    let r = w.agents[p[1]]
        .relationships
        .get(&subject)
        .expect("gossip should have created a relation entry for the absent subject");
    assert!(
        r.last_contact > t0,
        "last_contact (last heard of) should move on gossip: {}",
        r.last_contact
    );
    assert_eq!(
        r.met, 0,
        "hearing about the subject must not stamp met (real contact only): {}",
        r.met
    );
}

/// #1131 (2026-11-15 fix-1131-met-stamp-real-contact): a finished talk is real, in-person
/// contact and must stamp `met` on both sides.
#[test]
fn talk_stamps_met() {
    use hf_sim::mind::Activity;

    let (mut w, p) = party(2);
    let (a, b) = (w.agents[p[0]].id, w.agents[p[1]].id);
    let t = w.meta.tick;
    assert_eq!(w.agents[p[0]].relationships.get(&b).map(|r| r.met), None);
    assert_eq!(w.agents[p[1]].relationships.get(&a).map(|r| r.met), None);

    // Put them side by side, mid-talk, with its last tick (so act::step finishes it now),
    // and in good odour with each other so it cannot sour into a quarrel instead.
    w.agents[p[0]].pos = w.agents[p[1]].pos;
    w.agents[p[0]].relate(b, 0.9, 0.5, t);
    w.agents[p[1]].relate(a, 0.9, 0.5, t);
    w.agents[p[0]].mood = 0.5;
    w.agents[p[1]].mood = 0.5;
    w.agents[p[0]].activity = Activity::Talking {
        with: b,
        ticks_left: 1,
    };
    w.agents[p[1]].activity = Activity::Talking {
        with: a,
        ticks_left: 1,
    };
    hf_sim::systems::act::step(&mut w);

    assert_eq!(
        w.agents[p[0]].relationships.get(&b).map(|r| r.met),
        Some(t),
        "a should have met b after a finished talk"
    );
    assert_eq!(
        w.agents[p[1]].relationships.get(&a).map(|r| r.met),
        Some(t),
        "b should have met a after a finished talk"
    );
}
