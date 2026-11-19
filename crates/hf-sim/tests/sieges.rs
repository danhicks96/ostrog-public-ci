//! C76: sieges and prisoners (backlog 9.2).

use hf_sim::events::EventKind;
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::World;

/// An opening world four days on, with the first inhabited place beyond the valley holding a
/// grudge enough to send a host. Returns the world and that place's id.
fn aggrieved(seed: u64) -> (World, u32) {
    let mut w = World::opening_world(seed);
    w.meta.autoplan = true;
    for _ in 0..4 * TICKS_PER_DAY {
        w.step();
    }
    let s = w
        .overworld
        .sites
        .iter_mut()
        .find(|s| s.people > 0)
        .expect("an inhabited place");
    s.grudge = hf_sim::threat::GRUDGE_HOST;
    s.food = s.people as f32 * 30.0; // well fed: it comes for the grudge, not for food
    let id = s.id;
    (w, id)
}

fn until_host(w: &mut World, days: u64) -> Option<usize> {
    for _ in 0..days * TICKS_PER_DAY {
        w.step();
        if let Some(k) = w.bands.iter().position(|b| b.host) {
            return Some(k);
        }
    }
    None
}

#[test]
fn a_grudge_sends_a_host_not_a_hungry_band() {
    let (mut w, site) = aggrieved(3);
    let k = until_host(&mut w, 12).expect("a host within twelve days");
    let b = &w.bands[k];
    assert_eq!(b.site, site);
    assert!(b.size >= 4, "a host is bigger than a band: {}", b.size);
    let s = w.overworld.sites.iter().find(|s| s.id == site).unwrap();
    assert_eq!(s.grudge, 0, "spent in the sending");
}

#[test]
fn a_host_carries_off_someone_brought_down_outside_the_wall() {
    let (mut w, _) = aggrieved(3);
    let k = until_host(&mut w, 12).expect("a host");
    let tick = w.meta.tick;
    // Camped now, at noon.
    w.meta.tick = tick - tick % TICKS_PER_DAY + 12 * hf_sim::time::TICKS_PER_HOUR;
    let since = w.meta.tick;
    w.bands[k].state = hf_sim::threat::BandState::Camped(since);
    let camp = w.bands[k].camp;
    let m = w.bands[k].members[0];
    let mi = w.agents.iter().position(|a| a.id == m).unwrap();
    w.agents[mi].pos = camp;
    // One of the valley beside them, out in the open and already badly hurt.
    let vi = w
        .agents
        .iter()
        .position(|a| a.alive && a.community.is_some() && a.band.is_none())
        .unwrap();
    let victim = w.agents[vi].id;
    let beside = w.grid.surface(camp.x + 1, camp.y);
    w.agents[vi].pos = beside;
    hf_sim::systems::health::wound(
        &mut w,
        vi,
        hf_sim::body::Part::LEG_L,
        hf_sim::body::WoundKind::Cut,
        0.7,
        0.0,
        "a test",
        None,
    );
    // Not every one brought down is carried off (half the time): try a few moments.
    for n in 0..12u64 {
        w.meta.tick = since + n * 17;
        hf_sim::siege::besiege(&mut w, k, since);
        if w.overworld.away.iter().any(|a| a.id == victim) {
            break;
        }
    }
    assert!(
        w.overworld.away.iter().any(|a| a.id == victim),
        "carried off out of the valley"
    );
    assert!(w.bands[k].captives.contains(&victim));
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Captured { agent, .. } if agent == victim)));
}

#[test]
fn a_wall_that_holds_lifts_a_siege() {
    let (mut w, _) = aggrieved(3);
    let k = until_host(&mut w, 12).expect("a host");
    let since = w.meta.tick;
    w.bands[k].state = hf_sim::threat::BandState::Camped(since);
    w.meta.tick = since + 4 * TICKS_PER_DAY;
    assert!(!hf_sim::siege::lifts(&w, k, since), "no wall: they stay");
    let c = &mut w.communities[0];
    c.palisade = Some(hf_sim::society::Palisade {
        ring: vec![],
        gate: c.home,
        begun: since,
        stood: Some(since),
        closed: true,
    });
    assert!(hf_sim::siege::lifts(&w, k, since), "the wall held");
}

#[test]
fn one_kept_is_ransomed_by_their_own_people_and_else_taken_in() {
    let (mut w, site) = aggrieved(3);
    let cid = w.communities[0].id;
    let home = w.communities[0].home;
    let tick = w.meta.tick;
    let a = w.spawn_agent(hf_sim::agents::Species::Person, home);
    let i = w.agents.iter().position(|x| x.id == a).unwrap();
    w.agents[i].held = Some((cid, site, tick - 5 * TICKS_PER_DAY));
    hf_sim::siege::daily(&mut w);
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Ransomed { agent, .. } if agent == a)));
    assert!(w.agents[i].leaving && w.agents[i].held.is_none());
    // A second, whose people have nothing to pay with, is in time one of the valley.
    for s in w.overworld.sites.iter_mut().filter(|s| s.id == site) {
        s.food = 0.0;
    }
    let b = w.spawn_agent(hf_sim::agents::Species::Person, home);
    let j = w.agents.iter().position(|x| x.id == b).unwrap();
    w.agents[j].held = Some((cid, site, tick - 11 * TICKS_PER_DAY));
    hf_sim::siege::daily(&mut w);
    assert_eq!(w.agents[j].community, Some(cid), "taken in");
}

#[test]
fn someone_carried_off_may_be_let_go_and_come_home() {
    let (mut w, site) = aggrieved(3);
    let vi = w
        .agents
        .iter()
        .position(|a| a.alive && a.community.is_some())
        .unwrap();
    let who = w.agents[vi].id;
    let a = w.agents.remove(vi);
    w.overworld.away.push(a);
    let tick = w.meta.tick;
    for s in w.overworld.sites.iter_mut().filter(|s| s.id == site) {
        s.captives.push((who, tick - 9 * TICKS_PER_DAY));
    }
    let mut home = false;
    for d in 0..30 {
        w.meta.tick = tick + d * TICKS_PER_DAY;
        hf_sim::siege::daily(&mut w);
        if w.agents.iter().any(|a| a.id == who) {
            home = true;
            break;
        }
    }
    assert!(home, "let go within a month");
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Returned { agent, .. } if agent == who)));
}

/// A camped host at noon with one member at its camp and one of the valley, downed, beside it.
/// Returns (world, band index, victim's index).
fn downed_beside_a_host(captives: usize) -> (World, usize, usize) {
    let (mut w, _) = aggrieved(3);
    let k = until_host(&mut w, 12).expect("a host");
    let tick = w.meta.tick;
    w.meta.tick =
        tick - tick % TICKS_PER_DAY + 2 * TICKS_PER_DAY + 12 * hf_sim::time::TICKS_PER_HOUR;
    let since = w.meta.tick - 2 * TICKS_PER_DAY;
    w.bands[k].state = hf_sim::threat::BandState::Camped(since);
    for n in 0..captives {
        w.bands[k]
            .captives
            .push(hf_sim::ids::AgentId(900_000 + n as u32));
    }
    let camp = w.bands[k].camp;
    let m = w.bands[k].members[0];
    let mi = w.agents.iter().position(|a| a.id == m).unwrap();
    w.agents[mi].pos = camp;
    let vi = w
        .agents
        .iter()
        .position(|a| a.alive && a.community.is_some() && a.band.is_none())
        .unwrap();
    w.agents[vi].pos = w.grid.surface(camp.x + 1, camp.y);
    hf_sim::systems::health::wound(
        &mut w,
        vi,
        hf_sim::body::Part::LEG_L,
        hf_sim::body::WoundKind::Cut,
        0.7,
        0.0,
        "a test",
        None,
    );
    assert!(w.agents[vi].body.needs_care(), "downed");
    (w, k, vi)
}

/// #402 (M5 review, seed 3: one person took 592 wounds from a host in three days): a host that
/// holds all the captives it may take does not strike someone already down.
#[test]
fn a_host_does_not_strike_someone_already_down() {
    let (mut w, k, vi) = downed_beside_a_host(2);
    let victim = w.agents[vi].id;
    let since = match w.bands[k].state {
        hf_sim::threat::BandState::Camped(s) => s,
        _ => unreachable!(),
    };
    let noon = w.meta.tick;
    let before = w.agents[vi].body.wounds.len();
    for n in 0..60u64 {
        w.meta.tick = noon + n * 17;
        hf_sim::siege::besiege(&mut w, k, since);
    }
    let vi = w.agents.iter().position(|a| a.id == victim).unwrap();
    assert_eq!(
        w.agents[vi].body.wounds.len(),
        before,
        "no blow lands on someone already down"
    );
    assert!(!w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Fought { b, .. } if b == victim)));
}

/// #402: someone down beside a host that may still take a captive is carried off, not beaten.
#[test]
fn someone_down_is_carried_off_not_beaten() {
    let (mut w, k, vi) = downed_beside_a_host(0);
    let victim = w.agents[vi].id;
    let since = match w.bands[k].state {
        hf_sim::threat::BandState::Camped(s) => s,
        _ => unreachable!(),
    };
    let noon = w.meta.tick;
    let before = w.agents[vi].body.wounds.len();
    for n in 0..12u64 {
        w.meta.tick = noon + n * 17;
        hf_sim::siege::besiege(&mut w, k, since);
        if w.overworld.away.iter().any(|a| a.id == victim) {
            break;
        }
    }
    let a = w
        .overworld
        .away
        .iter()
        .find(|a| a.id == victim)
        .expect("carried off");
    assert_eq!(a.body.wounds.len(), before, "carried off without a blow");
}

/// Step until the host is leaving; every event kind recorded on the way.
fn until_leaving(w: &mut World, k: usize) -> Vec<EventKind> {
    let mut seen = Vec::new();
    for _ in 0..TICKS_PER_DAY {
        w.step();
        seen.extend(w.last_events.iter().map(|e| e.kind.clone()));
        if w.bands
            .get(k)
            .is_none_or(|b| b.state == hf_sim::threat::BandState::Leaving)
        {
            break;
        }
    }
    seen
}

/// #402 (M5 review: all ten `siege_lifted` rows were hosts that took all they came for): a host
/// that went home sated is not a siege lifted.
#[test]
fn a_host_that_took_what_it_came_for_is_not_a_siege_lifted() {
    let (mut w, _) = aggrieved(3);
    let k = until_host(&mut w, 12).expect("a host");
    let since = w.meta.tick;
    w.bands[k].state = hf_sim::threat::BandState::Camped(since);
    w.bands[k].taken = w.bands[k].want;
    let seen = until_leaving(&mut w, k);
    assert!(seen
        .iter()
        .any(|e| matches!(e, EventKind::HostSated { .. })));
    assert!(!seen
        .iter()
        .any(|e| matches!(e, EventKind::SiegeLifted { .. })));
    let line = seen
        .iter()
        .find(|e| matches!(e, EventKind::HostSated { .. }))
        .cloned()
        .unwrap();
    let ev = hf_sim::events::Event {
        id: hf_sim::ids::EventId(0),
        tick: w.meta.tick,
        at: None,
        severity: hf_sim::events::Severity::Major,
        kind: line,
    };
    let text = hf_sim::view::narrate::line(&w, &ev);
    assert!(text.contains("had what it came for"), "{text}");
}

/// #402: a siege the wall held is lifted, and the record says so.
#[test]
fn a_siege_the_wall_held_says_the_wall_held() {
    let (mut w, _) = aggrieved(3);
    let k = until_host(&mut w, 12).expect("a host");
    let since = w.meta.tick - 4 * TICKS_PER_DAY;
    w.bands[k].state = hf_sim::threat::BandState::Camped(since);
    w.bands[k].taken = 0;
    let c = &mut w.communities[0];
    c.palisade = Some(hf_sim::society::Palisade {
        ring: vec![],
        gate: c.home,
        begun: since,
        stood: Some(since),
        closed: true,
    });
    let seen = until_leaving(&mut w, k);
    assert!(
        seen.iter()
            .any(|e| matches!(e, EventKind::SiegeLifted { why, .. } if why == "the wall held")),
        "{seen:?}"
    );
    assert!(!seen
        .iter()
        .any(|e| matches!(e, EventKind::HostSated { .. })));
}

/// #274: a host batters the wall nearest its camp; a timber wall comes down in two days, one of
/// granite holds four, and the deep's gabbro five. The broken stretch is on record and the
/// builders are set to mend it.
#[test]
fn harder_walls_stand_longer_against_a_siege() {
    use hf_sim::world::{Material, Tile};
    let (mut w, _) = aggrieved(3);
    let k = until_host(&mut w, 12).expect("a host");
    let tick = w.meta.tick;
    let since = tick - tick % TICKS_PER_DAY;
    w.bands[k].state = hf_sim::threat::BandState::Camped(since);
    let camp = w.bands[k].camp;
    let ring: Vec<_> = (2..5)
        .map(|dx| w.grid.surface(camp.x + dx, camp.y))
        .collect();
    let base = w.clone();
    for (stone, of, days) in [
        (false, Material::PINE, 2u64),
        (true, Material::GRANITE, 4),
        (true, Material::GABBRO, 5),
    ] {
        let mut w = base.clone();
        for q in &ring {
            w.grid.set(
                *q,
                Tile::solid(if stone {
                    Material::STONE_BLOCK
                } else {
                    Material::WOOD
                }),
            );
            w.built_of.insert(*q, of);
        }
        let c = &mut w.communities[0];
        c.palisade = Some(hf_sim::society::Palisade {
            ring: ring.clone(),
            gate: ring[2],
            begun: since,
            stood: Some(since),
            closed: true,
        });
        assert_eq!(hf_sim::siege::batter_days(&w, ring[0]), days, "{of:?}");
        let mut fell = None;
        for d in 1..=6u64 {
            w.meta.tick = since + d * TICKS_PER_DAY + 12 * hf_sim::time::TICKS_PER_HOUR;
            hf_sim::siege::batter(&mut w, k, since);
            if ring
                .iter()
                .any(|q| w.grid.get(*q).shape != hf_sim::world::Shape::Wall)
            {
                fell = Some(d);
                break;
            }
        }
        assert_eq!(fell, Some(days), "{of:?}");
        assert!(!w.communities[0].palisade.as_ref().unwrap().closed);
        assert!(w
            .events
            .pending
            .iter()
            .any(|e| matches!(e.kind, EventKind::WallBroken { .. })));
        let broken = *ring
            .iter()
            .find(|q| w.grid.get(**q).shape != hf_sim::world::Shape::Wall)
            .unwrap();
        assert!(w
            .designations
            .iter()
            .any(|d| d.pos == broken
                && matches!(d.kind, hf_sim::work::DesignationKind::BuildWall(_))));
    }
}
