//! The trade census (#1330, under #1329): `examples/trade_census.rs`, included here so the
//! tests call the same `Census` the example prints from. The scenario is built by hand, not
//! read back from the code, so the shares and "nothing to show" count below are worked out on
//! paper first.

#[path = "../examples/trade_census.rs"]
mod trade_census;

use hf_sim::agents::Species;
use hf_sim::events::{Event, EventKind, Severity};
use hf_sim::expedition::{Caravan, CaravanState, Edge, Purpose, Site, SiteKind};
use hf_sim::ids::{AgentId, CommunityId, TilePos};
use hf_sim::items::ItemKind;
use hf_sim::world::gen::GenParams;
use hf_sim::World;
use trade_census::Census;

/// A person of `community`, named, standing at home.
fn person(w: &mut World, community: CommunityId, name: &str) -> AgentId {
    let id = w.spawn_agent(Species::Person, TilePos::new(30, 30, 2));
    let a = w.agents.iter_mut().find(|a| a.id == id).unwrap();
    a.name = name.to_string();
    a.community = Some(community);
    id
}

/// A flat meadow with one community and two sites: Tollmoor (grudge 0) and Saltmere (grudge 2).
fn setup() -> (World, CommunityId) {
    let mut w = World::generate(1330, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.tick = 0;
    w.rebuild_caches();
    let cid = w.found_community(TilePos::new(30, 30, 2));
    // Clear the pre-generated sites (`expedition::SITE_NAMES`, ids 0..9) so our ids 1 and 2
    // are the only Tollmoor and Saltmere `w.overworld.site` finds, not a generated site that
    // happens to share the id with grudge 0.
    w.overworld.sites.clear();
    w.overworld.sites.push(Site {
        id: 1,
        name: "Tollmoor".into(),
        kind: SiteKind::Village,
        edge: Edge::North,
        days: 2,
        offers: Default::default(),
        wants: Vec::new(),
        danger: 0.0,
        known_by: Vec::new(),
        people: 10,
        food: 10.0,
        hungry_days: 0,
        last_band: 0,
        grudge: 0,
        captives: Vec::new(),
        realm_site: None,
        stores: Default::default(),
        store_material: Default::default(),
        trades: Vec::new(),
        exchanges: Vec::new(),
    });
    w.overworld.sites.push(Site {
        id: 2,
        name: "Saltmere".into(),
        kind: SiteKind::Village,
        edge: Edge::South,
        days: 2,
        offers: Default::default(),
        wants: Vec::new(),
        danger: 0.0,
        known_by: Vec::new(),
        people: 10,
        food: 10.0,
        hungry_days: 0,
        last_band: 0,
        grudge: 2,
        captives: Vec::new(),
        realm_site: None,
        stores: Default::default(),
        store_material: Default::default(),
        trades: Vec::new(),
        exchanges: Vec::new(),
    });
    (w, cid)
}

/// A `CaravanFormed` + its `Caravan` in `w.overworld.caravans`, as `expedition.rs` leaves it the
/// tick it fires, trading `carry` at `site`, led by `leader`.
fn form(w: &mut World, id: u32, cid: CommunityId, leader: AgentId, site: u32, carry: ItemKind) {
    w.overworld.caravans.push(Caravan {
        id,
        community: cid,
        members: vec![leader],
        purpose: Purpose::Trade { site, carry },
        cargo: Vec::new(),
        materials: Default::default(),
        qualities: Default::default(),
        rations: 0,
        lost: 0,
        state: CaravanState::Forming,
        formed: 0,
        departed: 0,
        depart_tile: TilePos::new(30, 30, 2),
        finished: 0,
        log: Vec::new(),
    });
    w.last_events.push(Event {
        id: hf_sim::ids::EventId(id),
        tick: 0,
        at: None,
        severity: Severity::Major,
        kind: EventKind::CaravanFormed {
            caravan: id,
            community: cid,
            leader,
            members: 1,
            purpose: "to trade".into(),
        },
    });
}

/// A `CaravanReturned` with `received` ("" for "nothing to show").
fn returned(w: &mut World, id: u32, cid: CommunityId, leader: AgentId, received: &str) {
    w.last_events.push(Event {
        id: hf_sim::ids::EventId(1000 + id),
        tick: 0,
        at: None,
        severity: Severity::Major,
        kind: EventKind::CaravanReturned {
            caravan: id,
            community: cid,
            leader,
            members: 1,
            days: 4,
            site: "Tollmoor".into(),
            received: received.into(),
            story: "went and came back".into(),
        },
    });
}

/// Mordric leads 3 of 4 trips, all to Tollmoor; Halgar leads 1, to Saltmere, whose grudge is 2
/// at departure. By hand: leader share 3/4 = 75%, site share 3/4 = 75% (Tollmoor), 1 grudge
/// trip. One return brings nothing, so "nothing to show" is 1 of 2 returns.
#[test]
fn tallies_leader_and_site_share_and_grudge_trips() {
    let (mut w, cid) = setup();
    let mordric = person(&mut w, cid, "Mordric");
    let halgar = person(&mut w, cid, "Halgar");
    let mut census = Census::default();

    form(&mut w, 1, cid, mordric, 1, ItemKind::GrainSheaf);
    census.note(&w);
    w.last_events.clear();
    form(&mut w, 2, cid, mordric, 1, ItemKind::GrainSheaf);
    census.note(&w);
    w.last_events.clear();
    form(&mut w, 3, cid, mordric, 1, ItemKind::GrainSheaf);
    census.note(&w);
    w.last_events.clear();
    form(&mut w, 4, cid, halgar, 2, ItemKind::GrainSheaf);
    census.note(&w);
    w.last_events.clear();

    returned(&mut w, 1, cid, mordric, "40 grain");
    census.note(&w);
    w.last_events.clear();
    returned(&mut w, 4, cid, halgar, "");
    census.note(&w);
    w.last_events.clear();

    let name = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .unwrap()
        .name
        .clone();
    let cc = census.by_community.get(&name).expect("community tallied");

    assert_eq!(cc.trips, 4);
    assert_eq!(*cc.by_leader.get("Mordric").unwrap_or(&0), 3);
    assert!(
        (cc.leader_share() - 0.75).abs() < 1e-6,
        "leader share: {}",
        cc.leader_share()
    );
    assert!(
        (cc.site_share() - 0.75).abs() < 1e-6,
        "site share: {}",
        cc.site_share()
    );
    assert_eq!(
        cc.grudge_trips, 1,
        "only the Saltmere trip left at grudge >= 2"
    );
    assert_eq!(cc.returns, 2);
    assert_eq!(
        cc.nothing, 1,
        "Halgar's empty-handed return is the one nothing-to-show"
    );
}
