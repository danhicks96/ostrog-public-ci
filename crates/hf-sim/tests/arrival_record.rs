//! A world shown before its first tick is whole (issue #98): the events waiting for the first
//! flush (the valley's past, the arrival) can be written to the chronicle at once, and stepping
//! afterwards neither writes them twice nor changes the world.

use hf_sim::chronicle::Chronicle;
use hf_sim::events::{EventKind, Severity};
use hf_sim::{Session, World};

#[test]
fn the_arrival_is_recorded_before_the_first_tick() {
    let w = World::opening_world(7);
    let waiting = w
        .events
        .pending
        .iter()
        .filter(|e| e.severity > Severity::Trace)
        .count() as i64;
    assert!(waiting > 0);
    let mut early = Session::new(w.clone(), Chronicle::in_memory().unwrap());
    let told = early.write_pending().unwrap();
    assert_eq!(
        early.chronicle.count().unwrap(),
        waiting,
        "all of it, at once"
    );
    assert!(told
        .iter()
        .any(|e| matches!(e.kind, EventKind::Arrived { .. })));
    assert!(told
        .iter()
        .any(|e| matches!(e.kind, EventKind::SetupChosen { .. })));
    // Asking again writes nothing more.
    assert!(early.write_pending().unwrap().is_empty());

    // Stepping on: the same world as one that waited, and every row exactly once.
    let mut late = Session::new(w, Chronicle::in_memory().unwrap());
    for _ in 0..600 {
        early.step().unwrap();
        late.step().unwrap();
    }
    assert_eq!(
        hf_sim::digest::digest(&early.world),
        hf_sim::digest::digest(&late.world)
    );
    assert_eq!(
        early.chronicle.count().unwrap(),
        late.chronicle.count().unwrap()
    );
}

#[test]
fn a_restored_record_is_not_written_twice() {
    // A save taken before the first tick holds the waiting events and the rows already written.
    let w = World::opening_world(3);
    let mut s = Session::new(w, Chronicle::in_memory().unwrap());
    s.write_pending().unwrap();
    let n = s.chronicle.count().unwrap();
    let dir = std::env::temp_dir().join(format!("hf-arrival-record-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    hf_sim::save::write(&s.world, &mut s.chronicle, &dir).unwrap();
    let (world, chronicle) = hf_sim::save::read(&dir).unwrap();
    let mut r = Session::new(world, chronicle);
    assert_eq!(r.chronicle.count().unwrap(), n);
    r.step().unwrap();
    assert!(r.chronicle.count().unwrap() >= n);
    let _ = std::fs::remove_dir_all(&dir);
}
