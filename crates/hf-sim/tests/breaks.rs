//! #332: breakdowns of many kinds, each its own thing, and each person breaking their own way.

use hf_sim::events::{BreakExplanation, Event, EventKind, Severity};
use hf_sim::ids::{AgentId, EventId, TilePos};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;
use std::collections::BTreeMap;

/// Everyone at the end of their rope, broken once a day for `days` days: (agent, kind) rows.
fn breaks(seed: u64, n: usize, days: u64) -> (World, Vec<(hf_sim::ids::AgentId, String)>) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    w.found_party(n);
    w.meta.autoplan = false;
    let mut out = Vec::new();
    for d in 0..days {
        for a in w.agents.iter_mut() {
            a.stress = 1.0;
            a.last_break = 0;
            a.needs.hunger = 1.0;
            a.activity = hf_sim::mind::Activity::Idle;
        }
        w.meta.tick = (d + 1) * 3 * TICKS_PER_DAY + 12 * TICKS_PER_HOUR + 900;
        w.events.pending.clear();
        hf_sim::systems::strain::step(&mut w);
        for e in &w.events.pending {
            if let EventKind::Broke { agent, kind, .. } = &e.kind {
                out.push((*agent, kind.clone()));
            }
        }
    }
    (w, out)
}

#[test]
fn a_valley_breaks_in_many_ways_and_each_person_in_their_own() {
    let (_, rows) = breaks(3321, 20, 12);
    let kinds: std::collections::BTreeSet<&str> = rows.iter().map(|r| r.1.as_str()).collect();
    assert!(kinds.len() >= 5, "kinds seen: {kinds:?}");
    // Each person's most usual way is most of their breaks.
    let mut per: BTreeMap<hf_sim::ids::AgentId, BTreeMap<String, u32>> = BTreeMap::new();
    for (a, k) in &rows {
        *per.entry(*a).or_default().entry(k.clone()).or_default() += 1;
    }
    let (top, all): (u32, u32) = per.values().fold((0, 0), |(t, n), m| {
        (t + m.values().max().unwrap(), n + m.values().sum::<u32>())
    });
    assert!(top * 10 >= all * 4, "own way {top} of {all}");
}

#[test]
fn a_binge_eats_what_lies_near() {
    let mut w = World::generate(3322, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    w.found_party(6);
    w.meta.autoplan = false;
    let at = w.agents.iter().find(|a| a.alive).unwrap().pos;
    w.drop_item(hf_sim::items::ItemKind::Berries, 10, at);
    let food = |w: &World| -> u32 {
        w.items
            .iter()
            .filter(|it| it.kind.is_food())
            .map(|it| it.count)
            .sum()
    };
    let before = food(&w);
    // Everyone a glutton, at the end of their rope, beside the berries.
    for a in w.agents.iter_mut() {
        a.traits.list = vec![hf_sim::mind::Trait::Gourmet];
        a.pos = at;
    }
    let mut binged = false;
    for d in 0..10u64 {
        for a in w.agents.iter_mut() {
            a.stress = 1.0;
            a.last_break = 0;
        }
        w.meta.tick = (d + 1) * 3 * TICKS_PER_DAY + 12 * TICKS_PER_HOUR + 900;
        w.events.pending.clear();
        hf_sim::systems::strain::step(&mut w);
        binged |= w
            .events
            .pending
            .iter()
            .any(|e| matches!(&e.kind, EventKind::Broke { kind, .. } if kind == "a binge"));
    }
    assert!(binged);
    assert!(food(&w) < before, "the food went");
}

/// A lone, settled pair; `a` is at the end of their rope with nothing else against them.
fn pair_at_the_end_of_a_rope(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    w.found_party(2);
    w.meta.autoplan = false;
    w.meta.tick = 3 * TICKS_PER_DAY + 12 * TICKS_PER_HOUR + 900;
    w.events.pending.clear();
    let a = &mut w.agents[0];
    a.stress = 1.0;
    a.last_break = 0;
    a.thoughts.list.clear();
    a.feelings.list.clear();
    a.needs = hf_sim::mind::needs::Needs {
        hunger: 1.0,
        rest: 1.0,
        warmth: 1.0,
        social: 1.0,
        comfort: 1.0,
        thirst: 1.0,
        parched_ticks: 0,
    };
    a.activity = hf_sim::mind::Activity::Idle;
    w.agents[1].stress = 0.0;
    w
}

fn only_break(w: &World, who: AgentId) -> &Event {
    let mut found = w
        .events
        .pending
        .iter()
        .filter(|e| matches!(&e.kind, EventKind::Broke { agent, .. } if *agent == who));
    let event = found.next().expect("the forced breakdown was recorded");
    assert!(found.next().is_none(), "only one breakdown was recorded");
    event
}

fn anger_about_other(w: &mut World) -> (AgentId, AgentId) {
    let (a, b) = (w.agents[0].id, w.agents[1].id);
    let tick = w.meta.tick;
    w.agents[0].feelings.feel(
        hf_sim::mind::feelings::FeelingKind::Anger,
        Some(b),
        0.7,
        tick,
        None,
    );
    (a, b)
}

#[test]
fn a_break_names_the_person_it_is_over() {
    let mut w = pair_at_the_end_of_a_rope(3323);
    let (a, b) = anger_about_other(&mut w);
    hf_sim::systems::strain::step(&mut w);
    let event = only_break(&w, a);
    assert_eq!(event.participants(), [a]);
    assert!(matches!(
        &event.kind,
        EventKind::Broke {
            explanation: Some(BreakExplanation::Known { about: Some(about), .. }),
            ..
        } if *about == b
    ));
    let expected = format!("It was over {}.", w.agents[1].name);
    assert_eq!(
        hf_sim::view::narrate::contextual_detail(&w, event),
        Some(expected)
    );
}

#[test]
fn a_break_with_no_cause_says_so() {
    let mut w = pair_at_the_end_of_a_rope(3324);
    let a = w.agents[0].id;
    hf_sim::systems::strain::step(&mut w);
    let event = only_break(&w, a);
    assert!(matches!(
        &event.kind,
        EventKind::Broke {
            explanation: Some(BreakExplanation::Unknown),
            ..
        }
    ));
    assert_eq!(
        hf_sim::view::narrate::contextual_detail(&w, event).as_deref(),
        Some("No one could name the cause.")
    );
}

#[test]
fn an_explanation_preserves_event_ids_and_observer_state() {
    let run = |with_explanation: bool| {
        let mut w = pair_at_the_end_of_a_rope(3323);
        let (a, b) = anger_about_other(&mut w);
        // Remove possible collateral events: no hostile target, fuel, furniture or food.
        w.agents[0].relationships.clear();
        w.agents[0].beliefs = Default::default();
        w.agents[0].came = w.meta.tick;
        w.structures.clear();
        w.items.clear();
        let pos = w.agents[0].pos;
        // Out of reach of a fight, which anger at someone close now brings (OST-549).
        w.agents[1].pos = pos.offset(9, 0, 0);
        w.agents[1].activity = hf_sim::mind::Activity::Idle;
        w.agents[1].techniques = Default::default();
        let tick = w.meta.tick;
        let next = w.events.next_id;

        hf_sim::systems::strain::step(&mut w);
        assert_eq!(
            w.events.pending.len(),
            1,
            "no second event or collateral event"
        );
        let event = only_break(&w, a);
        assert_eq!(
            (event.id, event.tick, event.at),
            (EventId(next), tick, Some(pos))
        );
        assert_eq!(event.severity, Severity::Major);
        assert_eq!(event.kind_name(), "broke");
        assert_eq!(event.participants(), [a]);
        if !with_explanation {
            let EventKind::Broke { explanation, .. } = &mut w.events.pending[0].kind else {
                unreachable!()
            };
            *explanation = None;
        }

        let followup = w.events.push(
            tick,
            Some(pos),
            Severity::Notable,
            EventKind::Doused { agent: a, at: pos },
        );
        assert_eq!(followup, EventId(next + 1));
        hf_sim::systems::flush::step(&mut w);
        hf_sim::systems::culture::observe(&mut w);
        hf_sim::systems::strain::observe(&mut w);
        hf_sim::systems::feel::observe(&mut w);
        hf_sim::systems::learn::observe(&mut w);
        assert!(matches!(
            w.culture_log.as_slice(),
            [(
                _,
                _,
                EventKind::Broke {
                    explanation: None,
                    ..
                }
            )]
        ));
        let learned = w
            .agents
            .iter()
            .find(|a| a.id == b)
            .unwrap()
            .techniques
            .knows(hf_sim::mind::Technique::CarryWater);

        // Match the serialized state after removing only the permitted narration difference.
        let EventKind::Broke { explanation, .. } = &mut w.last_events[0].kind else {
            unreachable!()
        };
        *explanation = None;
        (learned, hf_sim::digest::digest(&w), w.culture_log)
    };

    let (annotated_learned, annotated_digest, annotated_log) = run(true);
    let (baseline_learned, baseline_digest, baseline_log) = run(false);
    assert_eq!(annotated_log, baseline_log, "culture log without masking");
    assert_eq!(annotated_learned, baseline_learned);
    assert_eq!(annotated_digest, baseline_digest);
}

#[test]
fn legacy_break_payloads_have_no_explanation() {
    #[derive(serde::Serialize)]
    enum LegacyKind {
        Broke { agent: AgentId, kind: String },
    }
    #[derive(serde::Serialize)]
    struct LegacyEvent {
        id: EventId,
        tick: u64,
        at: Option<TilePos>,
        severity: Severity,
        kind: LegacyKind,
    }

    let old_json = r#"{"Broke":{"agent":7,"kind":"a daze"}}"#;
    let from_chronicle: EventKind = serde_json::from_str(old_json).unwrap();
    assert_eq!(serde_json::to_string(&from_chronicle).unwrap(), old_json);
    assert!(matches!(
        &from_chronicle,
        EventKind::Broke {
            explanation: None,
            ..
        }
    ));

    let old_event = LegacyEvent {
        id: EventId(8),
        tick: 12,
        at: Some(TilePos::new(3, 4, 0)),
        severity: Severity::Major,
        kind: LegacyKind::Broke {
            agent: AgentId(7),
            kind: "a daze".into(),
        },
    };
    // Save format 2 writes named-field MessagePack. Use the old event's actual field
    // layout here, not a new event initialized with an explicit None.
    let mut bytes = Vec::new();
    serde::Serialize::serialize(
        &old_event,
        &mut rmp_serde::Serializer::new(&mut bytes).with_struct_map(),
    )
    .unwrap();
    let decoded: Event = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(
        (decoded.id, decoded.tick, decoded.at),
        (old_event.id, old_event.tick, old_event.at)
    );
    assert_eq!(decoded.severity, old_event.severity);
    assert_eq!(decoded.kind, from_chronicle);
}

#[test]
fn recorded_explanations_round_trip_in_save_and_chronicle() {
    use hf_sim::save::{self, snapshot::Snapshot};

    let mut w = pair_at_the_end_of_a_rope(3323);
    let (a, b) = (w.agents[0].id, w.agents[1].id);
    let pos = w.agents[0].pos;
    let tick = w.meta.tick;
    let mut chronicle = hf_sim::chronicle::Chronicle::in_memory().unwrap();
    for (explanation, wire) in [
        (
            BreakExplanation::Known {
                about: Some(b),
                cause: "anger".into(),
            },
            serde_json::json!({"Known": {"about": b.0, "cause": "anger"}}),
        ),
        (
            BreakExplanation::Known {
                about: None,
                cause: "from hunger".into(),
            },
            serde_json::json!({"Known": {"about": null, "cause": "from hunger"}}),
        ),
        (BreakExplanation::Unknown, serde_json::json!("Unknown")),
    ] {
        let id = w.events.push(
            tick,
            Some(pos),
            Severity::Major,
            EventKind::Broke {
                agent: a,
                kind: "a daze".into(),
                explanation: Some(explanation),
            },
        );
        let event = w.events.pending.last().unwrap();
        chronicle.record(std::slice::from_ref(event)).unwrap();
        let (_, _, detail) = chronicle.row(id).unwrap().unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&detail).unwrap(),
            serde_json::json!({"Broke": {"agent": a.0, "kind": "a daze", "explanation": wire}})
        );
        assert_eq!(
            serde_json::from_str::<EventKind>(&detail).unwrap(),
            event.kind
        );
    }

    let bytes = save::encode(&Snapshot::capture(&w)).unwrap();
    assert_eq!(
        save::decode(&bytes).unwrap().events.pending,
        w.events.pending
    );

    hf_sim::systems::flush::step(&mut w);
    hf_sim::systems::culture::observe(&mut w);
    let bytes = save::encode(&Snapshot::capture(&w)).unwrap();
    let loaded = save::decode(&bytes).unwrap();
    assert_eq!(&*loaded.last_events, &w.last_events);
    assert_eq!(&*loaded.culture_log, &w.culture_log);
    assert_eq!(loaded.culture_log.len(), 3);
    for (logged, full) in loaded.culture_log.iter().zip(loaded.last_events.iter()) {
        assert_eq!((logged.0, logged.1), (full.id, full.tick));
        assert_eq!(
            logged.2,
            EventKind::Broke {
                agent: a,
                kind: "a daze".into(),
                explanation: None,
            }
        );
        assert!(matches!(
            &full.kind,
            EventKind::Broke {
                explanation: Some(_),
                ..
            }
        ));
    }
}

#[test]
fn an_unfound_subject_does_not_hide_the_break_or_leak_the_explanation() {
    use hf_sim::opening::{SurvivorSlot, SurvivorState};
    use hf_sim::view::{history, known, narrate};

    let mut w = pair_at_the_end_of_a_rope(3323);
    let (a, b) = (w.agents[0].id, w.agents[1].id);
    let actor = w.agents[0].name.clone();
    w.agents[1].name = "Unfound person".into();
    let event = Event {
        id: EventId(8),
        tick: w.meta.tick,
        at: Some(w.agents[0].pos),
        severity: Severity::Major,
        kind: EventKind::Broke {
            agent: a,
            kind: "a daze".into(),
            explanation: Some(BreakExplanation::Known {
                about: Some(b),
                cause: "hidden explanation".into(),
            }),
        },
    };
    let headline = format!("{actor} broke down: a daze.");
    assert_eq!(narrate::line(&w, &event), headline);
    assert_eq!(
        narrate::contextual_detail(&w, &event).as_deref(),
        Some("It was over Unfound person.")
    );
    w.survivor_pool.push(SurvivorSlot {
        index: 4,
        rerolls: 0,
        state: SurvivorState::Lost {
            agent: b,
            found: false,
        },
        past_events: Vec::new(),
    });

    assert!(known::told(&w, &event));
    assert_eq!(event.participants(), [a]);
    assert_eq!(narrate::line(&w, &event), headline);
    assert_eq!(narrate::detail(&event), None);
    assert_eq!(narrate::contextual_detail(&w, &event), None);

    let mut chronicle = hf_sim::chronicle::Chronicle::in_memory().unwrap();
    chronicle.record(std::slice::from_ref(&event)).unwrap();
    let shown = history::major(&w, &chronicle, 5).unwrap();
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].2, headline);
    assert_eq!(shown[0].3, None);
}

/// OST-241: the mood a person has `ticks` after a break day, with or without the break; the
/// needs pass rebuilds mood from its terms every stripe, so only a lasting term can show.
fn mood_after_break_day(broke: bool) -> (f32, bool) {
    let mut w = World::generate(3323, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    w.found_party(4);
    w.meta.autoplan = false;
    let who = w.agents[0].id;
    for a in w.agents.iter_mut() {
        a.stress = 0.0;
        a.last_break = 0;
        a.needs.hunger = 1.0;
        a.activity = hf_sim::mind::Activity::Idle;
    }
    w.meta.tick = 3 * TICKS_PER_DAY + 12 * TICKS_PER_HOUR + 900;
    if broke {
        w.agents[0].stress = 1.0;
    }
    w.events.pending.clear();
    hf_sim::systems::strain::step(&mut w);
    let did_break = w
        .events
        .pending
        .iter()
        .any(|e| matches!(&e.kind, EventKind::Broke { agent, .. } if *agent == who));
    // One game hour on, needs pass every tick (it only touches its own stripe each time).
    for _ in 0..TICKS_PER_HOUR {
        w.meta.tick += 1;
        for a in w.agents.iter_mut() {
            a.needs.hunger = 1.0;
            a.needs.rest = 1.0;
            a.needs.social = 1.0;
            a.needs.comfort = 1.0;
            a.needs.thirst = 1.0;
            a.needs.warmth = 1.0;
        }
        hf_sim::systems::needs::step(&mut w);
    }
    (w.agents[0].mood, did_break)
}

#[test]
fn the_cost_of_a_break_lasts_beyond_a_minute() {
    let (with, did_break) = mood_after_break_day(true);
    let (without, none) = mood_after_break_day(false);
    assert!(did_break, "the stressed person did not break");
    assert!(!none, "the calm person broke");
    assert!(
        without - with >= 0.15,
        "an hour after a break mood {with} vs {without} without it"
    );
}

/// The kind of break one forced breakdown picks, for a person carrying `feel` (OST-549).
fn kind_with(seed: u64, feel: Option<(hf_sim::mind::feelings::FeelingKind, f32)>) -> String {
    let mut w = pair_at_the_end_of_a_rope(seed);
    // The other person is far away, so a fight is not on offer.
    w.agents[1].pos = TilePos::new(55, 55, 2);
    let (a, b) = (w.agents[0].id, w.agents[1].id);
    let tick = w.meta.tick;
    if let Some((k, s)) = feel {
        w.agents[0].feelings.feel(k, Some(b), s, tick, None);
    }
    hf_sim::systems::strain::step(&mut w);
    match &only_break(&w, a).kind {
        EventKind::Broke { kind, .. } => kind.clone(),
        _ => unreachable!(),
    }
}

#[test]
fn grief_breaks_into_a_daze() {
    use hf_sim::mind::feelings::FeelingKind::Grief;
    let kinds: Vec<String> = (0..20)
        .map(|s| kind_with(9000 + s, Some((Grief, 0.8))))
        .collect();
    let daze = kinds.iter().filter(|k| k.as_str() == "a daze").count();
    assert!(
        daze * 2 >= kinds.len(),
        "daze {daze} of {}: {kinds:?}",
        kinds.len()
    );
}

#[test]
fn without_a_cause_the_trait_bag_still_picks() {
    // No feeling: the kinds are spread as the bag leaves them, not mostly one.
    let kinds: Vec<String> = (0..20).map(|s| kind_with(9000 + s, None)).collect();
    let daze = kinds.iter().filter(|k| k.as_str() == "a daze").count();
    assert!(daze * 2 < kinds.len(), "{kinds:?}");
}
