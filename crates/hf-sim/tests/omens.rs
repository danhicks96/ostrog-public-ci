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

fn band(id: u32, members: Vec<hf_sim::ids::AgentId>, tick: u64, w: &World) -> hf_sim::threat::Band {
    use hf_sim::threat::{Band, BandState};
    Band {
        id,
        site: w.overworld.sites[0].id,
        site_name: "Tollmoor".into(),
        members,
        size: 1,
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
    }
}

/// OST-1312: birds are read only when someone outdoors and devout sees them rise.
#[test]
fn birds_are_read_only_when_someone_sees_them() {
    use hf_sim::mind::beliefs::{Source, Topic};
    use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
    let mut w = World::generate(2984, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    let ids = w.found_party(4);
    w.meta.autoplan = false;
    let at = |w: &World, id| w.agents.iter().position(|a| a.id == id).unwrap();
    let (devout, walker) = (ids[0], ids[3]);
    for id in &ids[1..3] {
        let i = at(&w, *id);
        w.agents[i].pos = w.grid.surface(2, 2);
    }
    let d = at(&w, devout);
    w.agents[d].pos = w.grid.surface(30, 30);
    w.agents[d]
        .beliefs
        .hold(Topic::DeitySigns, 0.8, 0.7, Source::Culture(0), 0);
    let wi = at(&w, walker);
    w.agents[wi].pos = w.grid.surface(55, 30);
    w.agents[wi].community = None;
    w.agents[wi].band = Some(7);
    let tick = w.meta.tick;
    let b = band(7, vec![walker], tick, &w);
    w.bands.push(b);
    let day = |w: &mut World| -> Vec<hf_sim::ids::AgentId> {
        let start = (w.meta.tick / TICKS_PER_DAY + 1) * TICKS_PER_DAY;
        let mut out = Vec::new();
        for h in 0..24 {
            w.meta.tick = start + h * TICKS_PER_HOUR + 53;
            w.last_events.clear();
            w.events.drain();
            hf_sim::omens::observe(w);
            for e in &w.events.pending {
                if let EventKind::Omen { reader, of, .. } = &e.kind {
                    if of.contains("crows") {
                        out.push(*reader);
                    }
                }
            }
        }
        out
    };
    // 25 tiles off, outdoors, devout: one omen, naming them.
    assert_eq!(day(&mut w), vec![devout]);
    // The same band with everyone indoors: none.
    w.faith.foretold.clear();
    for k in 0..w.agents.len() {
        if w.agents[k].band.is_none() {
            let p = w.agents[k].pos;
            w.roofs.insert(
                p,
                hf_sim::building::Roof {
                    material: hf_sim::world::Material::THATCH,
                    quality: 0.7,
                    built: 0,
                    snow: 0.0,
                },
            );
        }
    }
    assert!(day(&mut w).is_empty());
}
