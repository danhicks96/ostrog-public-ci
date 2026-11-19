//! #562: those of the valley in another place, as the world map shows them (view::away): where
//! they are, what they are doing and how it goes, from what the simulation holds.

use hf_sim::ids::TilePos;
use hf_sim::mind::feelings::FeelingKind;
use hf_sim::realm::sites::{RSite, SiteKind};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::view::away::{list, Reaction};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

/// A small valley of six with a peopled place they know, which is settlement 0 of the world at
/// region 777; someone has gone there on their own.
fn one_away() -> (World, hf_sim::ids::AgentId, u32) {
    let mut w = World::generate(3471, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.rebuild_caches();
    w.found_party(6);
    w.meta.autoplan = false;
    w.ensure_nav();
    let cid = w.communities[0].id;
    for a in w.agents.iter_mut().filter(|a| a.community == Some(cid)) {
        a.traits.list = vec![hf_sim::mind::Trait::Curious];
    }
    w.realm.sites.push(RSite {
        id: w.realm.sites.len() as u32,
        people: 0,
        region: 777,
        name: "Ivouth".into(),
        kind: SiteKind::Village,
        founded: 0,
        parent: None,
        population: 40.0,
        peak: 40.0,
        ended: None,
        minority: None,
        culture: 0,
        deep: false,
    });
    let rs = w.realm.sites.len() as u32 - 1;
    // Only one peopled place, so the journey goes there.
    let mut sid = None;
    for s in w.overworld.sites.iter_mut() {
        if s.people > 0 && sid.is_none() {
            s.known_by.push(cid);
            s.realm_site = Some(rs);
            s.grudge = 0;
            s.hungry_days = 0;
            s.trades.clear();
            sid = Some(s.id);
        } else {
            s.known_by.retain(|c| *c != cid);
        }
    }
    let sid = sid.expect("a peopled place");
    for d in 15..800u64 {
        w.meta.tick = d * TICKS_PER_DAY + 7 * TICKS_PER_HOUR + 23;
        hf_sim::journeys::step(&mut w);
        if let Some(a) = w.overworld.away.first() {
            let id = a.id;
            return (w, id, sid);
        }
    }
    panic!("nobody set out");
}

#[test]
fn someone_away_on_their_own_is_at_the_place_on_the_map() {
    let (w, who, sid) = one_away();
    let rows = list(&w);
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    let site = w.overworld.sites.iter().find(|s| s.id == sid).unwrap();
    assert_eq!(r.agent, who);
    assert_eq!(r.place, site.name);
    assert_eq!(r.region, Some(777), "the place's world region");
    assert!(
        r.line.starts_with(&format!("at {}", site.name)),
        "{}",
        r.line
    );
    assert!(r.line.contains("back in"), "{}", r.line);
    assert!(
        matches!(r.reaction, Reaction::Among | Reaction::Working),
        "{:?}",
        r.reaction
    );
}

#[test]
fn a_place_holding_a_grudge_threatens_and_grief_shows() {
    let (mut w, who, sid) = one_away();
    w.overworld
        .sites
        .iter_mut()
        .find(|s| s.id == sid)
        .unwrap()
        .grudge = 2;
    let r = &list(&w)[0];
    assert_eq!(r.reaction, Reaction::Threatened);
    assert!(r.line.contains("grudge"), "{}", r.line);

    // No grudge, and grief for someone at home is what they feel most.
    w.overworld
        .sites
        .iter_mut()
        .find(|s| s.id == sid)
        .unwrap()
        .grudge = 0;
    let (other, other_name) = w
        .agents
        .iter()
        .find(|a| a.community.is_some())
        .map(|a| (a.id, a.name.clone()))
        .unwrap();
    let tick = w.meta.tick;
    let a = w.overworld.away.iter_mut().find(|a| a.id == who).unwrap();
    a.feelings
        .feel(FeelingKind::Grief, Some(other), 0.6, tick, None);
    let r = &list(&w)[0];
    assert_eq!(r.reaction, Reaction::Grieving);
    assert_eq!(
        r.feeling.as_deref(),
        Some(format!("grieving for {other_name}").as_str())
    );
}

#[test]
fn a_captive_where_the_valley_has_never_been_has_no_place_on_the_map() {
    let (mut w, who, sid) = one_away();
    // Turn the journey into a carrying-off to a place the valley does not know.
    let tick = w.meta.tick;
    let cid = w.communities[0].id;
    w.overworld.away[0].journey = None;
    let s = w.overworld.sites.iter_mut().find(|s| s.id == sid).unwrap();
    s.known_by.retain(|c| *c != cid);
    s.captives.push((who, tick - 3 * TICKS_PER_DAY));
    let r = &list(&w)[0];
    assert_eq!(r.reaction, Reaction::Threatened);
    assert_eq!(r.place, "");
    assert_eq!(r.region, None, "not on the map");
    assert!(
        r.line.starts_with(
            "held captive at a place the valley does not know, carried off 3 days ago"
        ),
        "{}",
        r.line
    );
}

#[test]
fn nobody_away_is_an_empty_list_and_the_list_changes_nothing() {
    let (w, _, _) = one_away();
    let before = format!("{:?}", w.overworld.away);
    let _ = list(&w);
    assert_eq!(before, format!("{:?}", w.overworld.away));
    let mut home = w.clone();
    home.overworld.away.clear();
    assert!(list(&home).is_empty());
}
