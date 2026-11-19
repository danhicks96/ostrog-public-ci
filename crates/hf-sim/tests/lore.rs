//! Lore (issue #50 part 3, C14): tales of how the old valley's hamlets ended, held with who told
//! them; they drift as they pass; neighbours remember their own; ruins nobody has explained are
//! guessed at. History stays what happened.

use hf_sim::agents::Species;
use hf_sim::events::EventKind;
use hf_sim::lore::{Claim, Tale};
use hf_sim::mind::Traits;
use hf_sim::World;

fn site_with_kin(w: &World) -> Option<(u32, String)> {
    w.history.sites.iter().find_map(|s| {
        let why = &s.ended.as_ref()?.1;
        why.strip_prefix("the last of them went to ")
            .map(|n| (s.id, n.to_string()))
    })
}

#[test]
fn tales_drift_as_they_pass_and_keep_their_tellers() {
    let mut w = World::opening_world(11);
    let ids = w.found_party(8);
    let site = w.history.sites[0].id;
    let why = w.history.sites[0].ended.clone().unwrap().1;
    let truth = Claim::truth(&why);
    let i0 = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    w.agents[i0].tales.push(Tale {
        site,
        claim: truth.clone(),
        teller: None,
        how: "their family's memory".into(),
        heard: 0,
    });
    let idx: Vec<usize> = ids
        .iter()
        .map(|id| w.agents.iter().position(|a| a.id == *id).unwrap())
        .collect();
    for round in 0..40 {
        for k in 0..idx.len() {
            let j = (k + 1 + round % 3) % idx.len();
            hf_sim::lore::on_talk(&mut w, idx[k], idx[j]);
            w.meta.tick += 1;
        }
    }
    let holders: Vec<&hf_sim::agents::Agent> = idx.iter().map(|&i| &w.agents[i]).collect();
    let told: Vec<&Tale> = holders
        .iter()
        .flat_map(|a| a.tales.iter())
        .filter(|t| t.teller.is_some())
        .collect();
    assert!(told.len() >= 4, "it passed around: {}", told.len());
    assert!(told.iter().all(|t| t.how.starts_with("told by")));
    assert!(
        told.iter().any(|t| t.claim != truth),
        "somewhere along the way it changed"
    );
    // What happened is unchanged.
    assert_eq!(w.history.sites[0].ended.clone().unwrap().1, why);
    let lines: Vec<String> = w
        .events
        .pending
        .iter()
        .filter(|e| matches!(e.kind, EventKind::TaleTold { changed: true, .. }))
        .map(|e| hf_sim::view::narrate::line(&w, e))
        .collect();
    assert!(lines.iter().any(|l| l.starts_with("In ")), "{lines:?}");
}

#[test]
fn neighbours_remember_their_own() {
    let mut found = 0;
    for seed in [7u64, 11, 19, 3] {
        let mut w = World::opening_world(seed);
        if site_with_kin(&w).is_none() {
            continue;
        }
        let p = w.meta.founding_site;
        let ids: Vec<_> = (0..24).map(|_| w.spawn_agent(Species::Person, p)).collect();
        hf_sim::lore::newcomers(&mut w, &ids);
        for id in &ids {
            let a = w.agents.iter().find(|a| a.id == *id).unwrap();
            for t in &a.tales {
                if t.how.starts_with("their family's memory") {
                    // Remembered in the place that hamlet's last people went to.
                    let why = &w
                        .history
                        .sites
                        .iter()
                        .find(|s| s.id == t.site)
                        .unwrap()
                        .ended
                        .as_ref()
                        .unwrap()
                        .1;
                    let place = why.trim_start_matches("the last of them went to ");
                    assert!(t.how.contains(place), "{} / {why}", t.how);
                    assert!(t.true_to(&w), "a family remembers it true");
                    found += 1;
                }
            }
        }
    }
    assert!(found >= 2, "{found}");
}

#[test]
fn a_ruin_nobody_explained_is_guessed_at() {
    let mut w = World::opening_world(11);
    let s = w.history.sites[0].clone();
    let ids = w.found_party(3);
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    w.agents[i].pos = s.centre;
    w.agents[i].tales.clear();
    w.meta.tick += hf_sim::time::TICKS_PER_HOUR - w.meta.tick % hf_sim::time::TICKS_PER_HOUR;
    hf_sim::lore::step(&mut w);
    let a = &w.agents[i];
    let t = a.tales.iter().find(|t| t.site == s.id).expect("a guess");
    // #1817: seed 11's site 0 truly ended by moving away, but has graves nearby (as any
    // long-lived hamlet does), so the settler's guess reads the graves as sickness — wrongly,
    // which is the point: a guess, not the truth.
    assert_eq!(t.claim, Claim::Fever);
    assert_eq!(t.how, "the graves there");
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(&e.kind, EventKind::RuinWondered { site, .. } if *site == s.name)));
}

#[test]
fn an_old_hamlet_with_many_graves_reads_as_sickness() {
    // #1817: a hillside of graves reads as sickness before "the young left", and still
    // reads as a curse to a fearful settler regardless of the graves.
    let mut w = World::opening_world(11);
    let s = w.history.sites[0].clone();
    w.history.sites[0].ended = Some((s.founded.saturating_sub(1), "the young left".into()));
    w.history.graves.clear();
    for k in 0..3 {
        w.history.graves.push(hf_sim::deep::Grave {
            pos: s.centre.offset(k, 0, 0),
            figure: k as u32,
        });
    }
    let ids = w.found_party(3);
    let i = w.agents.iter().position(|a| a.id == ids[0]).unwrap();
    w.agents[i].pos = s.centre;
    w.agents[i].tales.clear();
    w.agents[i].traits = Traits { list: Vec::new() };
    w.meta.tick += hf_sim::time::TICKS_PER_HOUR - w.meta.tick % hf_sim::time::TICKS_PER_HOUR;
    hf_sim::lore::step(&mut w);
    let t = w.agents[i]
        .tales
        .iter()
        .find(|t| t.site == s.id)
        .expect("a guess");
    assert_eq!(t.claim, Claim::Fever, "the graves say sickness");
    assert_eq!(t.how, "the graves there");

    // Control: the same site with no graves guesses the young simply left.
    let mut open = World::opening_world(11);
    open.history.sites[0].ended = Some((s.founded.saturating_sub(1), "the young left".into()));
    open.history.graves.clear();
    let ids = open.found_party(3);
    let i = open.agents.iter().position(|a| a.id == ids[0]).unwrap();
    open.agents[i].pos = s.centre;
    open.agents[i].tales.clear();
    open.agents[i].traits = Traits { list: Vec::new() };
    open.meta.tick += hf_sim::time::TICKS_PER_HOUR - open.meta.tick % hf_sim::time::TICKS_PER_HOUR;
    hf_sim::lore::step(&mut open);
    let t = open.agents[i]
        .tales
        .iter()
        .find(|t| t.site == s.id)
        .expect("a guess");
    assert_eq!(t.claim, Claim::YoungLeft, "no graves to read");

    // A fearful settler at the graves still says curse.
    let mut fearful = World::opening_world(11);
    fearful.history.sites[0].ended = Some((s.founded.saturating_sub(1), "the young left".into()));
    fearful.history.graves.clear();
    for k in 0..3 {
        fearful.history.graves.push(hf_sim::deep::Grave {
            pos: s.centre.offset(k, 0, 0),
            figure: k as u32,
        });
    }
    let ids = fearful.found_party(3);
    let i = fearful.agents.iter().position(|a| a.id == ids[0]).unwrap();
    fearful.agents[i].pos = s.centre;
    fearful.agents[i].tales.clear();
    fearful.agents[i].traits = Traits {
        list: vec![hf_sim::mind::Trait::Cautious],
    };
    fearful.meta.tick +=
        hf_sim::time::TICKS_PER_HOUR - fearful.meta.tick % hf_sim::time::TICKS_PER_HOUR;
    hf_sim::lore::step(&mut fearful);
    let t = fearful.agents[i]
        .tales
        .iter()
        .find(|t| t.site == s.id)
        .expect("a guess");
    assert_eq!(t.claim, Claim::Curse, "fearful says curse regardless");
}

#[test]
fn a_band_stops_at_its_peoples_graves() {
    for seed in [7u64, 11, 19, 3] {
        let mut w = World::opening_world(seed);
        let Some((sid, place)) = site_with_kin(&w) else {
            continue;
        };
        let c = w.history.sites.iter().find(|s| s.id == sid).unwrap().centre;
        let m = w.spawn_agent(Species::Person, c);
        w.bands.push(hf_sim::threat::Band {
            id: 99,
            site: 0,
            site_name: place.clone(),
            members: vec![m],
            size: 1,
            camp: c,
            state: hf_sim::threat::BandState::Camped(w.meta.tick),
            formed: w.meta.tick,
            want: 6,
            taken: 0,
            lost: 0,
            sighted: true,
            welcomed: false,
            pursuits: vec![],
            raiders: vec![],
            envoys: vec![],
            eaten: 0,
            night: 0,
            night_taken: 0,
            remembered: false,
            seen: Vec::new(),
            heard: Vec::new(),
            host: false,
            captives: Vec::new(),
        });
        w.meta.tick += hf_sim::time::TICKS_PER_HOUR - w.meta.tick % hf_sim::time::TICKS_PER_HOUR;
        hf_sim::lore::step(&mut w);
        let line = w
            .events
            .pending
            .iter()
            .find(|e| matches!(e.kind, EventKind::BandRemembered { .. }))
            .map(|e| hf_sim::view::narrate::line(&w, e))
            .expect("they stopped at the graves");
        assert!(
            line.contains(&place) && line.contains("their people came from there"),
            "{line}"
        );
        assert!(w.bands.iter().find(|b| b.id == 99).unwrap().remembered);
        return;
    }
    panic!("no seed had a hamlet whose people went to a neighbour");
}
