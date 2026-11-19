//! The seven nobody has found (issues #85, #93). What happens to them is recorded when it
//! happens, but until someone of the party finds them the viewer is told nothing of it: not in
//! the feed, not by the camera director, not in the away summary, not in the head count.

use hf_sim::chronicle::Chronicle;
use hf_sim::events::{EventKind, Severity};
use hf_sim::ids::AgentId;
use hf_sim::opening::{PARTY, STARTING};
use hf_sim::view::{director, history, known, summary};
use hf_sim::{Session, World};

fn about(e: &hf_sim::events::Event, who: &[AgentId]) -> bool {
    e.participants().iter().any(|a| who.contains(a))
}

#[test]
fn nothing_about_the_unfound_reaches_the_viewer_at_the_arrival() {
    let mut hurt = 0;
    for seed in [3u64, 7] {
        let w = World::opening_world(seed);
        let lost = known::unfound(&w);
        assert_eq!(lost.len(), PARTY - STARTING, "seed {seed}: seven unfound");
        let epoch = w.meta.epoch;
        // The arrival's events, as the feed would take them: told, from the arrival on.
        for e in &w.events.pending {
            if about(e, &lost) {
                if matches!(e.kind, EventKind::Wounded { .. }) {
                    hurt += 1;
                }
                assert!(
                    !known::told(&w, e),
                    "seed {seed}: the viewer is told {:?} about one nobody has found",
                    e.kind
                );
            } else if e.tick >= epoch && e.severity >= Severity::Minor {
                assert!(known::told(&w, e), "seed {seed}: {:?} is withheld", e.kind);
            }
        }
        assert_eq!(
            known::people_known(&w),
            STARTING,
            "seed {seed}: the head count"
        );
        assert_eq!(
            director::people(&w).len(),
            STARTING,
            "seed {seed}: the camera weighs only the three"
        );

        // The record keeps the truth; the viewer's read models leave it out.
        let mut s = Session::new(w, Chronicle::in_memory().unwrap());
        s.step().unwrap();
        let rows: i64 = s
            .chronicle
            .conn
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM entity_ref WHERE entity_kind = 'agent' AND entity_id IN ({})",
                    lost.iter()
                        .map(|a| a.0.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                ),
                [],
                |r| r.get(0),
            )
            .unwrap();
        let wounds = s
            .world
            .last_events
            .iter()
            .filter(|e| about(e, &lost) && e.severity > Severity::Trace)
            .count() as i64;
        assert_eq!(rows, wounds, "seed {seed}: what befell them is recorded");
        let sum = summary::since(&s.world, &s.chronicle, 0, s.world.tick(), 400).unwrap();
        for (id, tick, detail) in &sum.major {
            let e = history::event_from_row(*id, *tick, detail).unwrap();
            assert!(!about(&e, &lost), "seed {seed}: the away summary tells it");
        }
        let counted: i64 = sum.counts.iter().map(|(_, n)| n).sum();
        assert_eq!(
            counted,
            s.chronicle.count().unwrap() - rows,
            "seed {seed}: the summary's counts leave them out"
        );
    }
    assert!(
        hurt >= 2,
        "both seeds have someone hurt coming through: {hurt}"
    );
}

#[test]
fn found_they_are_told_of() {
    let mut w = World::opening_world(3);
    let lost = known::unfound(&w);
    let wound = w
        .events
        .pending
        .iter()
        .find(|e| matches!(e.kind, EventKind::Wounded { .. }) && about(e, &lost))
        .cloned()
        .expect("seed 3 has one hurt coming through");
    let id = wound.participants()[0];
    let slot = w
        .survivor_pool
        .iter()
        .find(|s| {
            matches!(s.state, hf_sim::opening::SurvivorState::Lost { agent, .. } if agent == id)
        })
        .map(|s| s.index)
        .unwrap();
    let at = w.agents.iter().find(|a| a.id == id).unwrap().pos;
    assert!(!known::told(&w, &wound));
    let out = hf_sim::opening::resolve_search(&mut w, slot, at);
    assert!(out.found);
    assert!(
        known::told(&w, &wound),
        "found, what befell them can be told"
    );
    assert!(!known::is_unfound(&w, id));
    assert_eq!(known::people_known(&w), STARTING + usize::from(out.alive));
}

#[test]
fn a_thing_tied_to_an_unfound_person_is_not_shown_until_they_are_found() {
    // The tool a lost person drops where they die names them (#107); staged directly.
    let mut w = World::opening_world(3);
    let lost = known::unfound(&w)[0];
    let name = w.agents.iter().find(|a| a.id == lost).unwrap().name.clone();
    let p = w.meta.founding_site.offset(2, 0, 0);
    let id = w.create_item(
        hf_sim::items::ItemKind::Tool,
        1,
        hf_sim::items::ItemPlace::Ground(p),
        0.5,
        "test",
    );
    let it = w.items.iter_mut().find(|it| it.id == id).unwrap();
    it.story = Some(Box::new(hf_sim::legend::Story {
        maker_name: "Halgar".into(),
        owners: vec![(lost, name.clone(), 0)],
        ..Default::default()
    }));
    let it = w.items.iter().find(|it| it.id == id).unwrap().clone();
    assert!(!known::item_told(&w, &it));
    assert!(hf_sim::legend::ground_note(&w, p).is_none());
    // Once found, it is theirs to be told of.
    for s in w.survivor_pool.iter_mut() {
        if let hf_sim::opening::SurvivorState::Lost { agent, found } = &mut s.state {
            if *agent == lost {
                *found = true;
            }
        }
    }
    assert!(known::item_told(&w, &it));
    assert!(hf_sim::legend::ground_note(&w, p).is_some());
}
