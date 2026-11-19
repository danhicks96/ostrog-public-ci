//! Phase 10 gates: a caravan leaves the map for a reason and comes back with an outcome
//! that lands as items, memories and history; a caravan that is lost leaves history and
//! dead people, not a gap; expedition memories consolidate at sleep with provenance.

use hf_sim::chronicle::Chronicle;
use hf_sim::events::{EventKind, Severity};
use hf_sim::expedition::{CaravanState, Purpose, SiteKind};
use hf_sim::ids::TilePos;
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::mind::MemKind;
use hf_sim::mind::Skill;
use hf_sim::plants::{Plant, PlantKind};
use hf_sim::systems::expedition::form_caravan;
use hf_sim::time::Season;
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::world::gen::GenParams;
use hf_sim::{Session, World};

fn meadow(seed: u64) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    for i in 0..120 {
        w.plants.push(Plant::new(
            TilePos::new(4 + (i * 7) % 56, 4 + (i * 11) % 56, 2),
            PlantKind::BerryBush,
            1.0,
            i as u32,
        ));
    }
    w.meta.tick = hf_sim::time::TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    w
}

fn founded(seed: u64) -> (World, hf_sim::ids::CommunityId) {
    let mut w = meadow(seed);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.found_party(10);
    let cid = w.communities[0].id;
    // A surplus of logs at home so the trade purpose is honest.
    w.drop_item(ItemKind::Log, 40, TilePos::new(32, 30, 2));
    w.ensure_nav();
    (w, cid)
}

#[test]
fn caravan_returns_with_outcome() {
    let (w, cid) = founded(1001);
    let mut s = Session::new(w, Chronicle::in_memory().unwrap());
    let site = s
        .world
        .overworld
        .sites
        .iter()
        .find(|st| st.wants.contains(&ItemKind::Log))
        .expect("a site that wants logs")
        .clone();
    let id = form_caravan(
        &mut s.world,
        cid,
        Purpose::Trade {
            site: site.id,
            carry: ItemKind::Log,
        },
    )
    .expect("caravan forms");
    let party = s.world.overworld.caravans[0].members.clone();
    assert!(party.len() >= 2);
    let mut left = false;
    let mut returned = None;
    for _ in 0..TICKS_PER_DAY * 16 {
        s.step().unwrap();
        for e in &s.world.last_events {
            match &e.kind {
                EventKind::CaravanLeft { caravan, .. } if *caravan == id => left = true,
                EventKind::CaravanReturned {
                    caravan,
                    received,
                    site: at,
                    ..
                } if *caravan == id => returned = Some((received.clone(), at.clone())),
                _ => {}
            }
        }
        if returned.is_some() {
            break;
        }
    }
    assert!(left, "the party left the map");
    let (received, at) = returned.expect("the party came back");
    assert_eq!(at, site.name);
    let c = &s.world.overworld.caravans[0];
    assert_eq!(c.state, CaravanState::Done);
    assert!(
        c.log.iter().any(|l| l.contains("traded")),
        "the trade happened: {:?}",
        c.log
    );
    // People are back on the map, the same people, with a memory of the journey.
    for m in &c.members {
        let a = s
            .world
            .agents
            .iter()
            .find(|a| a.id == *m)
            .expect("member is on the map again");
        assert!(a.alive);
        assert!(
            a.memories
                .short
                .iter()
                .any(|r| r.kind == MemKind::Discovery && r.event.is_some())
                || a.memories
                    .long
                    .iter()
                    .any(|mm| mm.kind == MemKind::Discovery),
            "{} remembers the journey",
            a.name
        );
    }
    // Goods arrived as real items; the site is now known; the chronicle holds the line.
    let got: Vec<&str> = received.split(", ").collect();
    assert!(!received.is_empty(), "something was brought back");
    for g in &got {
        let kind = g.split_once(' ').unwrap().1;
        assert!(
            s.world
                .items
                .iter()
                .any(|it| it.kind.label() == kind && matches!(it.place, ItemPlace::Ground(_))),
            "{kind} exists on the map"
        );
    }
    assert!(s
        .world
        .overworld
        .site(site.id)
        .unwrap()
        .known_by
        .contains(&cid));
    let rows = s.chronicle.recent(Severity::Major, 200).unwrap();
    assert!(rows.iter().any(|r| r.2 == "caravan_returned"));
    assert!(rows.iter().any(|r| r.2 == "caravan_left"));
    assert!(s.world.communities[0]
        .discoveries
        .iter()
        .any(|d| d.0.contains(&site.name)));
}

#[test]
fn caravan_lost_leaves_history() {
    // A far, dangerous site with no food on the road: some party, under some seed, is lost.
    let mut lost_seen = false;
    for seed in 2001..2012u64 {
        let (mut w, cid) = founded(seed);
        let sid = w.overworld.sites[0].id;
        {
            let st = &mut w.overworld.sites[0];
            st.danger = 1.0;
            st.days = 12;
        }
        let mut s = Session::new(w, Chronicle::in_memory().unwrap());
        let Some(id) = form_caravan(&mut s.world, cid, Purpose::Scout { site: sid }) else {
            continue;
        };
        // No rations: strand them and they starve.
        s.world.overworld.caravans[0].cargo.clear();
        s.world.overworld.caravans[0].rations = 0;
        let party = s.world.overworld.caravans[0].members.clone();
        let mut lost = false;
        for _ in 0..TICKS_PER_DAY * 40 {
            s.step().unwrap();
            if s.world.last_events.iter().any(
                |e| matches!(&e.kind, EventKind::CaravanLost { caravan, .. } if *caravan == id),
            ) {
                lost = true;
                break;
            }
            if matches!(s.world.overworld.caravans[0].state, CaravanState::Done) {
                break;
            }
        }
        if !lost {
            continue;
        }
        lost_seen = true;
        // Every member is dead on the record with the cause, none is still "away".
        for m in &party {
            let a = s
                .world
                .agents
                .iter()
                .find(|a| a.id == *m)
                .expect("member recorded");
            assert!(!a.alive);
            assert_eq!(a.death.as_deref(), Some("lost on the road"));
        }
        assert!(s.world.overworld.away.is_empty());
        let rows = s.chronicle.recent(Severity::Major, 400).unwrap();
        assert!(rows.iter().any(|r| r.2 == "caravan_lost"));
        let died = rows.iter().filter(|r| r.2 == "died").count();
        assert!(
            died >= party.len(),
            "deaths recorded: {died} of {}",
            party.len()
        );
        break;
    }
    assert!(lost_seen, "no caravan was lost under eleven seeds");
}

#[test]
fn expedition_memories_consolidate() {
    let (w, cid) = founded(3001);
    let mut s = Session::new(w, Chronicle::in_memory().unwrap());
    let sid = s.world.overworld.sites[0].id;
    s.world.overworld.sites[0].days = 2;
    s.world.overworld.sites[0].danger = 0.0;
    let id = form_caravan(&mut s.world, cid, Purpose::Scout { site: sid }).unwrap();
    let mut back = None;
    for _ in 0..TICKS_PER_DAY * 10 {
        s.step().unwrap();
        if let Some(e) = s.world.last_events.iter().find(
            |e| matches!(&e.kind, EventKind::CaravanReturned { caravan, .. } if *caravan == id),
        ) {
            back = Some(e.id);
            break;
        }
    }
    let eid = back.expect("the scouts came back");
    let members = s.world.overworld.caravans[0].members.clone();
    // Sleep on it: a few more days, then the long-term memory carries the event id (four,
    // not two, since C71: with the night the cold half of the day (#256) a sleeper on the
    // ground wakes from the cold and a full night's sleep comes later).
    s.world.run(TICKS_PER_DAY * 4);
    for m in &members {
        // They may already be off on the next journey; the person is the same either way.
        let a = s
            .world
            .agents
            .iter()
            .chain(s.world.overworld.away.iter())
            .find(|a| a.id == *m)
            .unwrap();
        let long = a
            .memories
            .long
            .iter()
            .find(|mm| mm.kind == MemKind::Discovery && mm.origins.contains(&eid));
        assert!(
            long.is_some(),
            "{} consolidated the journey with provenance",
            a.name
        );
    }
    let rows = s.chronicle.recent(Severity::Major, 200).unwrap();
    assert!(rows
        .iter()
        .any(|r| r.0 as u64 == eid.0 as u64 || r.2 == "caravan_returned"));
}

#[test]
fn the_road_log_names_every_incident_kind() {
    // #1584: a staged caravan on a site with danger 1.0 for 12 days; every log line it writes is a
    // road incident the census knows, or a line about the site itself.
    let (mut w, cid) = founded(2001);
    let sid = w.overworld.sites[0].id;
    {
        let st = &mut w.overworld.sites[0];
        st.danger = 1.0;
        st.days = 12;
    }
    let mut s = Session::new(w, Chronicle::in_memory().unwrap());
    form_caravan(&mut s.world, cid, Purpose::Scout { site: sid }).expect("caravan forms");
    for _ in 0..TICKS_PER_DAY * 40 {
        s.step().unwrap();
    }
    let log = s.world.overworld.caravans[0].log.clone();
    assert!(!log.is_empty(), "the road wrote nothing");
    for line in &log {
        assert!(
            hf_sim::expedition::road_incident(line).is_some()
                || line.starts_with("at ")
                || line.starts_with("set out")
                || line.starts_with("reached "),
            "unclassified road line: {line}"
        );
    }
}

/// Steps a staged scouting caravan from `start_day` over a `days`-day road with no danger, to a
/// `kind` site, and returns (food gathered while the calendar read a season where nothing
/// grows, gathering log lines, whether a member's Foraging rose).
fn staged_road(seed: u64, start_day: u64, kind: SiteKind) -> (u32, Vec<String>, bool) {
    let (mut w, cid) = founded(seed);
    w.meta.tick = start_day * TICKS_PER_DAY + hf_sim::time::TICKS_PER_HOUR * 8;
    let sid = w.overworld.sites[0].id;
    {
        let st = &mut w.overworld.sites[0];
        st.danger = 0.0;
        st.days = 4;
        st.kind = kind;
    }
    let mut s = Session::new(w, Chronicle::in_memory().unwrap());
    form_caravan(&mut s.world, cid, Purpose::Scout { site: sid }).expect("caravan forms");
    let food = |s: &Session| -> u32 {
        s.world.overworld.caravans[0]
            .cargo
            .iter()
            .filter(|(k, _)| matches!(k, ItemKind::Berries | ItemKind::Root))
            .map(|c| c.1)
            .sum()
    };
    let total_skill = |s: &Session| -> f32 {
        let ids = s.world.overworld.caravans[0].members.clone();
        s.world
            .overworld
            .away
            .iter()
            .filter(|a| ids.contains(&a.id))
            .map(|a| a.skills.level(Skill::Foraging))
            .sum()
    };
    let mut off_season = 0;
    let (mut prev, mut skill_rose) = (0, false);
    let mut base = None;
    for _ in 0..TICKS_PER_DAY * 14 {
        s.step().unwrap();
        if s.world.overworld.caravans[0].members.is_empty() {
            break;
        }
        base.get_or_insert_with(|| total_skill(&s));
        let now = food(&s);
        let season = hf_sim::time::Calendar::from_tick(s.world.meta.tick).season;
        if now > prev && !matches!(season, Season::Summer | Season::Autumn) {
            off_season += now - prev;
        }
        prev = now;
        skill_rose |= total_skill(&s) > base.unwrap();
    }
    let lines = s.world.overworld.caravans[0]
        .log
        .iter()
        .filter(|l| l.contains("gathered"))
        .cloned()
        .collect();
    (off_season, lines, skill_rose)
}

#[test]
fn nothing_is_gathered_on_a_winter_road() {
    // #1587: winter (days 15-19 of the 20-day year) with no danger: no food is added to the cargo.
    for seed in 2001..2013 {
        let (off, _, _) = staged_road(seed, 15, SiteKind::Forest);
        assert_eq!(off, 0, "seed {seed}: food appeared on a winter road");
    }
}

#[test]
fn a_summer_forest_road_feeds_the_party() {
    // #1587: summer (days 5-9) to a Forest site: some party gathers, names who, and practises.
    let mut gathered = false;
    for seed in 2001..2013 {
        let (_, lines, rose) = staged_road(seed, 5, SiteKind::Forest);
        if !lines.is_empty() {
            gathered = true;
            assert!(
                lines[0].contains("gathered") && lines[0].contains("by the way"),
                "unnamed: {}",
                lines[0]
            );
            assert!(rose, "seed {seed}: gatherer's Foraging did not rise");
        }
    }
    assert!(gathered, "no summer forest road gathered anything");
}
