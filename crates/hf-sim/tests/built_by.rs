//! #292 item 2 (WP-08 slice 8.6): the block panel says who built what stands on a tile, from
//! the chronicle's built record, without the Built event (Trace) entering real history.

use hf_sim::chronicle::Chronicle;
use hf_sim::events::{Event, EventKind, Severity};
use hf_sim::ids::{EventId, StructureId, TilePos};
use hf_sim::world::gen::GenParams;
use hf_sim::world::Material;
use hf_sim::World;

#[test]
fn who_built_a_wall_is_named_while_it_stands() {
    let mut w = World::generate(2921, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(1);
    let name = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .name
        .clone();
    let p = TilePos::new(33, 33, 2);
    let mut c = Chronicle::in_memory().unwrap();
    let before = c.count().unwrap();
    let tick = w.meta.epoch + 3 * hf_sim::time::TICKS_PER_DAY;
    c.record(&[Event {
        id: EventId(77),
        tick,
        at: Some(p),
        severity: Severity::Trace,
        kind: EventKind::Built {
            agent: ids[0],
            structure: StructureId(5),
            at: p,
        },
    }])
    .unwrap();
    // One wall is not news: real history is unchanged.
    assert_eq!(c.count().unwrap(), before);
    assert_eq!(c.built_at(p).unwrap(), Some((tick, ids[0])));
    // Nothing built stands there yet in this world: nothing is said.
    assert_eq!(hf_sim::view::built::line(&w, &c, p), None);
    w.built_of.insert(p, Material::WOOD);
    let line = hf_sim::view::built::line(&w, &c, p).unwrap();
    assert!(
        line.starts_with(&format!("Built by {name}, Y1 day 4")),
        "{line}"
    );
    // Another tile has no record.
    assert_eq!(c.built_at(TilePos::new(1, 1, 2)).unwrap(), None);
}
