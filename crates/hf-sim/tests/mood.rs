//! #815: a long memory records the tick it was last renewed, not just when it was formed, so
//! mood can later weigh a memory kept fresh above one left to go stale.

use hf_sim::mind::memory::{MemKind, Memories, Record, Subject};

#[test]
fn a_reinforced_memory_records_when() {
    let mut m = Memories::default();
    m.record(Record {
        tick: 0,
        kind: MemKind::Meal,
        subject: Subject::None,
        valence: 0.6,
        salience: 0.8,
        event: None,
    });
    m.consolidate(100, false, false);
    let formed = m.long[0].formed;
    assert_eq!(formed, 100);
    assert_eq!(m.long[0].last, 100);

    m.record(Record {
        tick: 100,
        kind: MemKind::Meal,
        subject: Subject::None,
        valence: 0.6,
        salience: 0.8,
        event: None,
    });
    m.consolidate(500, false, false);
    assert_eq!(
        m.long[0].formed, formed,
        "formed does not move on reinforcement"
    );
    assert_eq!(m.long[0].last, 500, "last moves to when it was reinforced");
}

#[test]
fn an_old_memory_without_last_loads() {
    // Saved by an hf-sim built before #815: the field is absent from the map.
    let old = serde_json::json!({
        "kind": "Meal",
        "subject": "None",
        "valence": 0.5,
        "strength": 0.4,
        "formed": 12345,
        "reinforced": 2,
        "origins": [],
    });
    let mut out = Vec::new();
    let mut ser = rmp_serde::Serializer::new(&mut out).with_struct_map();
    serde::Serialize::serialize(&old, &mut ser).unwrap();
    let m: hf_sim::mind::memory::Memory = rmp_serde::from_slice(&out).unwrap();
    assert_eq!(m.formed, 12345);
    assert_eq!(m.last, 0, "absent from an old save, defaults to 0");
}
