//! #351 (WP-08 slice 8.5): the World panel is a view over knowledge, never over truth.

use hf_sim::chronicle::Chronicle;
use hf_sim::view::{known_world, legends};
use hf_sim::World;

#[test]
fn the_valley_knows_only_what_it_has_heard_and_says_what_it_does_not() {
    let mut w = World::opening_world(1);
    let c = Chronicle::in_memory().unwrap();
    for s in w.overworld.sites.iter_mut() {
        s.known_by.clear();
    }
    let name = w.overworld.sites[0].name.clone();
    let page = known_world::page(&w);
    assert!(
        !page.contains(&name),
        "a place nobody has heard of was named: {page}"
    );
    assert!(page.contains("Of anywhere else, nothing is known."));
    // Heard of: named, with who knows it.
    let cid = w.communities[0].id;
    let cname = w.communities[0].name.clone();
    w.overworld.sites[0].known_by.push(cid);
    let page = known_world::page(&w);
    assert!(page.contains(&name) && page.contains(&cname), "{page}");
    // The lost: a count, never names.
    let lost = hf_sim::view::known::unfound(&w);
    if !lost.is_empty() {
        assert!(page.contains(&format!("{} of those who came through", lost.len())));
        for id in &lost {
            let n = &w.agents.iter().find(|a| a.id == *id).unwrap().name;
            assert!(!page.contains(n.as_str()), "a lost person was named: {n}");
        }
    }
    // Reached from the Legends world page, and routed there.
    assert!(legends::page(&w, &c, "world:0").contains("[url=known:0]"));
    assert_eq!(legends::page(&w, &c, "known:0"), page);
}

#[test]
fn the_page_does_not_leak_what_the_valley_cannot_know() {
    let mut w = World::opening_world(1);
    for s in w.overworld.sites.iter_mut() {
        s.known_by.clear();
    }
    // Someone on a journey who has died out there: the valley does not know it yet.
    let ix = w.agents.iter().position(|a| a.community.is_some()).unwrap();
    let mut a = w.agents.remove(ix);
    let name = a.name.clone();
    a.journey = Some((w.meta.tick + 1000, "Tollmoor".into(), "to see it".into()));
    a.alive = false;
    w.overworld.away.push(a);
    let page = known_world::page(&w);
    assert!(
        page.contains(&format!("{name} gone to Tollmoor")),
        "a death on the road leaked: {page}"
    );
    // A sighted band from a place nobody here knows: not named by it, nor counted to the head.
    let site = w.overworld.sites[0].clone();
    w.bands.push(hf_sim::threat::Band {
        id: 9,
        site: site.id,
        site_name: site.name.clone(),
        members: Vec::new(),
        size: 7,
        camp: w.meta.founding_site,
        state: hf_sim::threat::BandState::Approaching,
        formed: w.meta.tick,
        want: 10,
        taken: 0,
        lost: 0,
        sighted: true,
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
    let page = known_world::page(&w);
    assert!(
        !page.contains(&site.name),
        "an unknown home was named: {page}"
    );
    assert!(
        page.contains("A band of strangers, a dozen or so, coming"),
        "{page}"
    );
    assert!(
        !page.contains(" 7 "),
        "the band was counted to the head: {page}"
    );
    // Once the place is known, it is named.
    let cid = w.communities[0].id;
    w.overworld.sites[0].known_by.push(cid);
    assert!(known_world::page(&w).contains(&format!("A band from {}", site.name)));
}
