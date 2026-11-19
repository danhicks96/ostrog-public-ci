//! #455: a beast of the deep woken by a breach comes up because of the noise the valley makes
//! near the way, with a warning first, not on a day fixed at the breach (stirring.rs,
//! docs/design-deep-ways.md).

use hf_sim::events::EventKind;
use hf_sim::save::{self, snapshot::Snapshot};
use hf_sim::stirring::{RISE, WARN};
use hf_sim::time::TICKS_PER_HOUR;
use hf_sim::World;

/// A world whose deep cavern a beast lairs near, breached by one of the valley standing in it.
/// The cavern's index and the one who broke in.
fn breached() -> (World, usize, usize) {
    let mut w = World::from_setup(&hf_sim::setup::WorldSetup::new(52));
    let region = w.meta.setup.as_ref().and_then(|s| s.region).unwrap();
    let b = w
        .realm
        .beasts
        .iter()
        .position(|b| b.layer == "deep_caverns")
        .unwrap();
    w.realm.beasts[b].region = region;
    let k = w.grid.caverns.iter().position(|c| c.deep).unwrap();
    let c = w.grid.caverns[k].clone();
    let i = w
        .agents
        .iter()
        .position(|a| a.alive && a.community.is_some())
        .unwrap();
    w.agents[i].pos = c.centre;
    // They came in from the floor beside them: the breach is in the cavern, not where they stood
    // before the test put them there.
    w.agents[i].from = c.centre;
    hf_sim::underground::breaches(&mut w);
    assert_eq!(w.creatures.stirring.len(), 1, "the breach stirred it");
    w.meta.tick -= w.meta.tick % TICKS_PER_HOUR;
    (w, k, i)
}

fn hour(w: &mut World) {
    w.meta.tick += TICKS_PER_HOUR;
    hf_sim::underground::rise(w);
    hf_sim::stirring::step(w);
}

fn far(w: &mut World, i: usize) {
    let p = w.meta.founding_site;
    w.agents[i].pos = p;
}

#[test]
fn left_quiet_the_beast_does_not_come_up() {
    let (mut w, _, i) = breached();
    far(&mut w, i);
    // The two of the deep's people who came up are not the valley's; move every valley person
    // away from the way.
    for k in 0..w.agents.len() {
        if w.agents[k].community.is_some() {
            far(&mut w, k);
        }
    }
    for _ in 0..(5 * 24) {
        hour(&mut w);
    }
    assert!(
        w.creatures.risen.is_empty(),
        "five quiet days and nothing came"
    );
    assert_eq!(w.creatures.stirring[0].level, 0);
    assert!(!w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Rose { .. } | EventKind::StirBelow { .. })));
}

#[test]
fn noise_at_the_way_brings_a_warning_then_the_beast() {
    let (mut w, k, i) = breached();
    let at = w.grid.caverns[k].centre;
    // Two more of the valley come to stand with the first.
    let more: Vec<usize> = (0..w.agents.len())
        .filter(|&j| j != i && w.agents[j].alive && w.agents[j].community.is_some())
        .take(2)
        .collect();
    for &j in &more {
        w.agents[j].pos = at;
    }
    let mut warned_at = None;
    let mut rose_at = None;
    for h in 0..48 {
        hour(&mut w);
        let ev = |f: fn(&EventKind) -> bool| w.events.pending.iter().any(|e| f(&e.kind));
        if warned_at.is_none() && ev(|k| matches!(k, EventKind::StirBelow { .. })) {
            warned_at = Some(h);
        }
        if rose_at.is_none() && ev(|k| matches!(k, EventKind::Rose { .. })) {
            rose_at = Some(h);
        }
    }
    let (warned, rose) = (warned_at.expect("warned"), rose_at.expect("rose"));
    assert!(
        warned < rose,
        "the warning came first: {warned} then {rose}"
    );
    assert_eq!(
        warned,
        (WARN as usize).div_ceil(3) - 1,
        "three people, three a hour"
    );
    assert_eq!(rose, (RISE as usize).div_ceil(3) - 1);
    // Those who heard it are named, and keep the place as a bad one.
    let heard = w
        .events
        .pending
        .iter()
        .find_map(|e| match &e.kind {
            EventKind::StirBelow { heard, .. } => Some(heard.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(heard.len(), 3);
    let line = hf_sim::view::narrate::line(
        &w,
        w.events
            .pending
            .iter()
            .find(|e| matches!(e.kind, EventKind::StirBelow { .. }))
            .unwrap(),
    );
    assert!(line.starts_with("Something moved below"), "{line}");
    assert!(w.creatures.stirring.is_empty(), "it has come up");
}

#[test]
fn work_at_the_way_is_louder_than_standing_there() {
    let (mut w, _, i) = breached();
    hour(&mut w);
    assert_eq!(w.creatures.stirring[0].level, 1, "one standing: one");
    // Now mining: a work order at hand.
    let pos = w.agents[i].pos;
    let d = hf_sim::systems::settlement::make(&mut w, hf_sim::work::DesignationKind::Mine, pos, 6);
    let did = d.id;
    w.designations.push(d);
    w.agents[i].activity = hf_sim::mind::Activity::Working {
        plan: hf_sim::work::plan::Plan {
            designation: did,
            steps: Vec::new(),
            cursor: 0,
            label: "mining".into(),
        },
    };
    hour(&mut w);
    assert_eq!(w.creatures.stirring[0].level, 4, "one mining: three more");
}

#[test]
fn the_stir_survives_a_save() {
    let (mut w, _, _) = breached();
    hour(&mut w);
    hour(&mut w);
    let bytes = save::encode(&Snapshot::capture(&w)).unwrap();
    let back = save::decode(&bytes).unwrap().restore();
    assert_eq!(back.creatures.stirring, w.creatures.stirring);
    assert_eq!(back.creatures.stirring[0].level, 2);
}
