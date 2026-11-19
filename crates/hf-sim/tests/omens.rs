//! #297: the one followed in the spirit reads a great event as a sign, and those near feel it.

use hf_sim::events::{EventKind, Severity};
use hf_sim::ids::TilePos;
use hf_sim::justice::Domain;
use hf_sim::mind::feelings::FeelingKind as F;
use hf_sim::world::gen::GenParams;
use hf_sim::World;

#[test]
fn a_warning_read_by_the_one_followed_in_the_spirit_frightens_those_near() {
    let mut w = World::generate(2971, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(3);
    w.meta.autoplan = false;
    let reader = ids[0];
    w.communities[0].leaders[Domain::Spirit as usize] = Some(reader);
    for (k, id) in ids.iter().enumerate() {
        let i = w.agents.iter().position(|a| a.id == *id).unwrap();
        w.agents[i].pos = w.grid.surface(30 + k as i32, 30);
    }
    let tick = w.meta.tick;
    let id = w.events.push(
        tick,
        Some(w.grid.surface(30, 30)),
        Severity::Major,
        EventKind::Besieged {
            site: "Ashby".into(),
            size: 6,
        },
    );
    let e = w
        .events
        .pending
        .iter()
        .find(|e| e.id == id)
        .unwrap()
        .clone();
    w.last_events = vec![e];
    hf_sim::omens::observe(&mut w);
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Omen { good: false, reader: r, .. } if r == reader)));
    let other = w.agents.iter().position(|a| a.id == ids[2]).unwrap();
    assert!(w.agents[other].feelings.of(F::Fear) > 0.0);
}

#[test]
fn a_band_on_the_road_is_read_as_a_warning_before_it_is_seen() {
    use hf_sim::threat::{Band, BandState};
    use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
    let mut w = World::generate(2984, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(3);
    w.meta.autoplan = false;
    let reader = ids[0];
    w.communities[0].leaders[Domain::Spirit as usize] = Some(reader);
    let omens = |w: &mut World, days: u64| -> usize {
        let start = (w.meta.tick / TICKS_PER_DAY + 1) * TICKS_PER_DAY;
        let mut n = 0;
        for h in 0..days * 24 {
            w.meta.tick = start + h * TICKS_PER_HOUR + 53;
            w.last_events.clear();
            w.events.drain();
            hf_sim::omens::observe(w);
            n += w
                .events
                .pending
                .iter()
                .filter(|e| matches!(&e.kind, EventKind::Omen { good: false, of, .. } if of.contains("crows")))
                .count();
        }
        n
    };
    // No band: the birds are only birds.
    assert_eq!(omens(&mut w, 5), 0);
    let tick = w.meta.tick;
    w.bands.push(Band {
        id: 7,
        site: w.overworld.sites[0].id,
        site_name: "Tollmoor".into(),
        members: Vec::new(),
        size: 3,
        camp: TilePos::new(8, 8, 2),
        state: BandState::Approaching,
        formed: tick,
        want: 18,
        taken: 0,
        lost: 0,
        sighted: false,
        welcomed: false,
        pursuits: Vec::new(),
        raiders: Vec::new(),
        envoys: Vec::new(),
        eaten: 0,
        night: 0,
        night_taken: 0,
        remembered: false,
        seen: Vec::new(),
        heard: Vec::new(),
        host: false,
        captives: Vec::new(),
    });
    // On the road: read once, by day, for this band.
    assert_eq!(omens(&mut w, 2), 1);
    assert!(w.faith.foretold.contains(&7));
}
