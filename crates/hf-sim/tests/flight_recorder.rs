//! Real unattended decision, route attempt, blocked route and a later choice.
use hf_sim::chronicle::Chronicle;
use hf_sim::ids::{EventId, TilePos};
use hf_sim::items::ItemKind;
use hf_sim::mind::{knowledge::Known, Activity};
use hf_sim::systems::{act, decide};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::{debuglog, digest, Session, World};
use serde_json::Value;
use std::fs;

type DurableRows = Vec<Option<(i64, u64, String)>>;

fn run() -> (String, DurableRows, u32) {
    let mut w = World::generate(5532, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(20, 20, 2);
    w.meta.tick = TICKS_PER_DAY * 30 + 10 * TICKS_PER_HOUR;
    w.meta.autoplan = false;
    w.rebuild_caches();
    let id = w.found_party(1)[0];
    w.items.retain(|it| !it.kind.is_food());
    w.designations.clear();
    w.ensure_nav();
    let i = w.agents.iter().position(|a| a.id == id).unwrap();
    let position = w.grid.surface(20, 20);
    let first = w.grid.surface(24, 20);
    let second = w.grid.surface(20, 25);
    let a = &mut w.agents[i];
    a.pos = position;
    a.from = position;
    a.path.clear();
    a.activity = Activity::Idle;
    a.idle_ticks = 0;
    a.needs.hunger = 0.3;
    a.needs.thirst = 1.0;
    a.needs.rest = 1.0;
    a.needs.social = 1.0;
    a.needs.warmth = 1.0;
    let add_food = |w: &mut World, pos| {
        w.drop_item(ItemKind::Berries, 5, pos);
        let it = w.items.last().unwrap().id;
        w.agents[i]
            .knowledge
            .note(Known::Item(it), pos, w.meta.tick, ItemKind::Berries as u16);
        it
    };
    let dropped = add_food(&mut w, first);
    let mut session = Session::new(w, Chronicle::in_memory().unwrap());
    session.write_pending().unwrap();
    decide::step(&mut session.world);
    assert!(matches!(session.world.agents[i].activity, Activity::Going { to, .. } if to == first));
    // Deterministically exercise the real act-stage failure path, as if the route
    // had become blocked after the route search. The recorder does not cause it.
    session.world.agents[i].path_blocked = true;
    act::step(&mut session.world);
    session.world.items.retain(|it| it.id != dropped);
    session.world.agents[i]
        .knowledge
        .forget(Known::Item(dropped));
    add_food(&mut session.world, second);
    session.step().unwrap();
    session.world.agents[i].idle_ticks = 0;
    decide::step(&mut session.world);
    assert!(matches!(session.world.agents[i].activity, Activity::Going { to, .. } if to == second));
    let last = session.chronicle.max_event_id().unwrap().unwrap();
    let records = (0..=last)
        .map(|n| session.chronicle.row(EventId(n)).unwrap())
        .collect();
    (digest::digest(&session.world), records, id.0)
}

#[test]
fn unattended_choices_and_failures_are_readable_after_close_without_changing_state() {
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("capture");
    let plain = run();
    assert!(!dest.exists());
    debuglog::configure_recorder(dest.clone(), Vec::new()).unwrap();
    let recorded = run();
    debuglog::disable();
    assert_eq!(
        plain, recorded,
        "world and ordered durable chronicle both stay unchanged"
    );
    let rows: Vec<Value> = fs::read_to_string(dest.join("trace-0000000000.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .filter(|row: &Value| row["agent"] == plain.2)
        .collect();
    let search = |kind: &str, after: usize| {
        rows.iter()
            .enumerate()
            .skip(after)
            .find(|(_, row)| row["t"] == kind)
            .unwrap()
            .0
    };
    let first_choice = search("decide", 0);
    let first_attempt = search("path_attempt", first_choice + 1);
    let failure = search("plan_failed", first_attempt + 1);
    let retry = search("decide", failure + 1);
    let next_attempt = search("path_attempt", retry + 1);
    assert_eq!(rows[failure]["reason"], "path blocked");
    assert_ne!(rows[first_attempt]["to"], rows[next_attempt]["to"]);
    assert_eq!(rows[next_attempt]["outcome"], "found");
}
