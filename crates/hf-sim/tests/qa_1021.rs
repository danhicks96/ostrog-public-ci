//! QA round 2026-10-21 (C68): guards for the simulation fixes, each against a filed issue.

use hf_sim::ids::TilePos;
use hf_sim::items::{ItemKind, ItemPlace};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn meadow(seed: u64, people: usize) -> World {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    w.found_party(people);
    w.ensure_nav();
    w
}

fn storied(
    w: &mut World,
    kind: ItemKind,
    at: TilePos,
    name: &str,
    made: u64,
) -> hf_sim::ids::ItemId {
    let id = w.create_lone_item(kind, 1, ItemPlace::Ground(at), 0.9, "test");
    w.items.iter_mut().find(|it| it.id == id).unwrap().story =
        Some(Box::new(hf_sim::legend::Story {
            maker_name: "the hunt".into(),
            name: Some(name.into()),
            made,
            ..Default::default()
        }));
    id
}

#[test]
fn word_of_the_valley_carries_one_treasure_at_most_and_not_always() {
    // #232: every band knew every named thing and its tile, and carried them all off in one
    // visit. Word of at most one reaches a band, by its fame, and often of none.
    let mut w = meadow(23201, 3);
    let home = w.communities[0].home;
    let tick = w.meta.tick;
    // Made two seasons ago: a thing in its first season is not heard of at all (#249).
    let made = tick.saturating_sub(60 * TICKS_PER_DAY);
    for k in 0..5 {
        storied(
            &mut w,
            ItemKind::Figurine,
            home.offset(3 + k, 4, 0),
            &format!("Figure {k}"),
            made,
        );
    }
    let new = hf_sim::legend::Story {
        name: Some("New".into()),
        made: tick,
        ..Default::default()
    };
    assert_eq!(
        hf_sim::threat::fame(&new, tick),
        0.0,
        "not in its first season"
    );
    assert_eq!(
        hf_sim::threat::heard_of(&w, home).len(),
        5,
        "five renowned things lie there"
    );
    let mut heard = 0;
    for t in 0..40u64 {
        let word = hf_sim::threat::word_of(&w, home, 1, tick + t * TICKS_PER_DAY);
        assert!(
            word.len() <= 1,
            "a band hears of one thing at most: {word:?}"
        );
        heard += word.len();
    }
    assert!(heard >= 1, "word gets out now and then");
    assert!(
        heard < 40,
        "but not to every band: {heard} of 40 heard of a new-made figure"
    );
}

#[test]
fn fame_grows_with_age_and_deeds() {
    use hf_sim::legend::Story;
    let year = hf_sim::time::TICKS_PER_YEAR;
    let new = Story {
        name: Some("Oldbiter".into()),
        made: 10 * year,
        ..Default::default()
    };
    let old = Story {
        deeds: vec![(0, "felled the great oak".into())],
        made: 8 * year,
        ..new.clone()
    };
    let now = 10 * year;
    assert!(hf_sim::threat::fame(&new, now) < hf_sim::threat::fame(&old, now));
    assert!(hf_sim::threat::fame(&old, now) <= 0.9);
}

fn camped_band(seed: u64, camp: TilePos, welcomed: bool) -> World {
    use hf_sim::agents::Species;
    use hf_sim::threat::{Band, BandState};
    let mut w = meadow(seed, 3);
    w.meta.autoplan = false;
    for a in w.agents.iter_mut() {
        a.needs.hunger = 1.0;
        a.needs.rest = 1.0;
    }
    let id = 1;
    let members: Vec<_> = (0..2)
        .map(|k| {
            let m = w.spawn_agent(Species::Person, camp.offset(k, 0, 0));
            w.agents.iter_mut().find(|a| a.id == m).unwrap().band = Some(id);
            m
        })
        .collect();
    let tick = w.meta.tick;
    w.bands.push(Band {
        id,
        site: w.overworld.sites[0].id,
        site_name: "Tollmoor".into(),
        members,
        size: 2,
        camp,
        state: BandState::Camped(tick),
        formed: tick,
        want: 12,
        taken: 0,
        lost: 0,
        sighted: true,
        welcomed,
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
    w.meta.tick = (tick / TICKS_PER_DAY + 1) * TICKS_PER_DAY + 23 * TICKS_PER_HOUR;
    w
}

fn targets(w: &World) -> Vec<hf_sim::ids::ItemId> {
    w.bands
        .iter()
        .flat_map(|b| b.raiders.iter().map(|r| r.1))
        .collect()
}

#[test]
fn guests_told_they_may_eat_leave_the_treasures_alone() {
    // #232: a welcomed pair walked out with the colony's only named figure and its trophy.
    let camp = TilePos::new(8, 8, 2);
    for welcomed in [false, true] {
        let mut w = camped_band(23202, camp, welcomed);
        let home = w.communities[0].home;
        let made = w.meta.tick;
        let crown = storied(
            &mut w,
            ItemKind::Jewel,
            home.offset(6, 6, 0),
            "the Antler Crown",
            made,
        );
        w.bands[0].heard = vec![(crown, home.offset(6, 6, 0))];
        if welcomed {
            // By day, as guests take openly.
            w.meta.tick += 11 * TICKS_PER_HOUR;
        }
        let mut went = false;
        for _ in 0..TICKS_PER_HOUR * 2 {
            w.step();
            went |= targets(&w).contains(&crown);
        }
        assert_eq!(
            went, !welcomed,
            "welcomed {welcomed}: went for the crown {went}"
        );
    }
}

// Moving camp (C60) after the QA round: #208, #209, #210, #213, #214.

fn big_valley(seed: u64) -> (World, hf_sim::ids::CommunityId) {
    let mut w = World::generate(seed, GenParams::default());
    w.flatten(2, 2, 120, 120, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(60, 60, 2);
    w.meta.tick = TICKS_PER_DAY * 30 + TICKS_PER_HOUR * 8;
    w.rebuild_caches();
    let ids = w.found_party(8);
    let cid = w
        .agents
        .iter()
        .find(|a| a.id == ids[0])
        .unwrap()
        .community
        .unwrap();
    w.meta.autoplan = true;
    w.ensure_nav();
    (w, cid)
}

fn hard_times(w: &mut World, cid: hf_sim::ids::CommunityId) {
    let tick = w.meta.tick;
    let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
    for (k, what) in [
        "Ana died of starvation",
        "strangers took from the store",
        "the rock came down",
        "Bren died of the cold",
    ]
    .iter()
    .enumerate()
    {
        c.hardships.push((
            tick - (4 - k as u64) * TICKS_PER_DAY,
            what.to_string(),
            hf_sim::ids::EventId(k as u32 + 1),
        ));
    }
}

fn regard_all(w: &mut World, cid: hf_sim::ids::CommunityId, opinion: f32) {
    let ids: Vec<_> = w
        .agents
        .iter()
        .filter(|a| a.community == Some(cid))
        .map(|a| a.id)
        .collect();
    for a in w.agents.iter_mut().filter(|a| a.community == Some(cid)) {
        for o in &ids {
            if *o != a.id {
                a.relationships.entry(*o).or_default().opinion = opinion;
            }
        }
    }
}

fn moves(w: &World, cid: hf_sim::ids::CommunityId) -> usize {
    w.projects
        .iter()
        .filter(|p| {
            p.community == cid && matches!(p.kind, hf_sim::project::ProjectKind::Move { .. })
        })
        .count()
}

#[test]
fn a_proposal_answers_its_hardships() {
    // #208: the same three raids drove three proposals twenty days apart.
    let (mut w, cid) = big_valley(20801);
    hard_times(&mut w, cid);
    regard_all(&mut w, cid, 0.6);
    hf_sim::systems::moving::consider(&mut w, cid);
    assert_eq!(moves(&w, cid), 1, "the hard times move them");
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    assert!(
        c.hardships.is_empty(),
        "the proposal answered them: {:?}",
        c.hardships
    );
    assert_eq!(c.last_move_proposal, w.meta.tick);
    // Twenty-one days on, with nothing new, and the project list long gone past fifty.
    w.meta.tick += 21 * TICKS_PER_DAY;
    w.projects.clear();
    hf_sim::systems::moving::consider(&mut w, cid);
    assert_eq!(moves(&w, cid), 0, "no new hardships, no second proposal");
}

#[test]
fn the_cooldown_lives_on_the_community() {
    // #208, the second hole: the Move record could fall out of the capped project list.
    let (mut w, cid) = big_valley(20802);
    hard_times(&mut w, cid);
    regard_all(&mut w, cid, 0.6);
    hf_sim::systems::moving::consider(&mut w, cid);
    assert_eq!(moves(&w, cid), 1);
    w.projects.clear();
    w.meta.tick += 5 * TICKS_PER_DAY;
    hard_times(&mut w, cid);
    hf_sim::systems::moving::consider(&mut w, cid);
    assert_eq!(
        moves(&w, cid),
        0,
        "not twice in twenty days, whatever the project list holds"
    );
}

#[test]
fn this_is_home_counts_the_years_lived_here() {
    // #209: the hold was computed from age, so every grown arrival had it on day one.
    let (mut w, cid) = big_valley(20901);
    hard_times(&mut w, cid);
    // Nobody thinks anything of the proposer, nobody is unhappy: only the hold decides.
    regard_all(&mut w, cid, 0.0);
    for a in w.agents.iter_mut() {
        a.mood = 0.0;
    }
    let year = hf_sim::time::TICKS_PER_YEAR;
    // All of them forty years old, here a month.
    for a in w.agents.iter_mut().filter(|a| a.community == Some(cid)) {
        a.born = w.meta.tick as i64 - 40 * year as i64;
    }
    hf_sim::systems::moving::consider(&mut w, cid);
    let stayed_home = w
        .events
        .drain()
        .iter()
        .filter(|e| matches!(&e.kind, hf_sim::events::EventKind::Chose { what, because, .. } if what == "stayed" && because == "this is home"))
        .count();
    assert_eq!(
        stayed_home, 0,
        "nobody a month in the valley stays for 'this is home'"
    );
    // The same people, here twenty-five years: it holds them, and says so.
    let (mut w, cid) = big_valley(20901);
    w.meta.tick += 30 * year;
    hard_times(&mut w, cid);
    regard_all(&mut w, cid, 0.0);
    let tick = w.meta.tick;
    for a in w.agents.iter_mut() {
        a.mood = 0.0;
        if a.community == Some(cid) {
            a.came = tick.saturating_sub(25 * year);
        }
    }
    hf_sim::systems::moving::consider(&mut w, cid);
    let evs = w.events.drain();
    let all: Vec<String> = evs
        .iter()
        .map(|e| hf_sim::view::narrate::line(&w, e))
        .collect();
    let stayed_home = all.iter().filter(|l| l.contains("this is home")).count();
    assert!(stayed_home >= 1, "those long here stay for it: {all:?}");
}

#[test]
fn food_given_openly_is_no_hardship() {
    // #213: a welcomed band's taking counted as a raid and split the village.
    let (mut w, cid) = big_valley(21301);
    let home = w.communities.iter().find(|c| c.id == cid).unwrap().home;
    for open in [true, false] {
        let tick = w.meta.tick;
        w.events.push(
            tick,
            Some(home.offset(10, 0, 0)),
            hf_sim::events::Severity::Minor,
            hf_sim::events::EventKind::Raided {
                band: 7 + open as u32,
                what: "grain".into(),
                count: 4,
                open,
            },
        );
        w.last_events = w.events.drain();
        hf_sim::systems::moving::observe(&mut w);
    }
    let c = w.communities.iter().find(|c| c.id == cid).unwrap();
    assert_eq!(
        c.hardships.len(),
        1,
        "only the night's taking counts: {:?}",
        c.hardships
    );
}

#[test]
fn the_feed_gets_one_line_for_the_answers() {
    // #214: twenty Minor answers on one tick reached the screen as none.
    let (mut w, cid) = big_valley(21401);
    hard_times(&mut w, cid);
    regard_all(&mut w, cid, 0.6);
    hf_sim::systems::moving::consider(&mut w, cid);
    let evs = w.events.drain();
    let line = evs
        .iter()
        .find(|e| matches!(e.kind, hf_sim::events::EventKind::Answered { .. }))
        .expect("one line sums the answers");
    assert!(line.severity >= hf_sim::events::Severity::Notable);
    let text = hf_sim::view::narrate::line(&w, line);
    assert!(text.contains("went with"), "{text}");
    assert!(!text.contains('{'), "{text}");
}

#[test]
fn a_party_leaving_does_not_end_the_community_as_a_departure() {
    // #210: `left_for` was set when seven of forty-eight left, so the forty-one who later died
    // were recorded as having left the valley.
    let (mut w, cid) = big_valley(21001);
    hard_times(&mut w, cid);
    // Most think nothing of the proposer; the proposer and one friend go.
    regard_all(&mut w, cid, -0.5);
    for a in w.agents.iter_mut() {
        a.mood = 0.0;
    }
    let ids: Vec<_> = w
        .agents
        .iter()
        .filter(|a| a.community == Some(cid))
        .map(|a| a.id)
        .collect();
    for a in w.agents.iter_mut().filter(|a| a.community == Some(cid)) {
        a.relationships.entry(ids[0]).or_default().opinion =
            if a.id == ids[1] { 0.9 } else { -0.5 };
        a.relationships.entry(ids[1]).or_default().opinion =
            if a.id == ids[0] { 0.9 } else { -0.5 };
    }
    // No flat place in the valley to go to, and a settlement of the world known: they leave.
    w.overworld.sites[0].known_by.push(cid);
    for p in w.communities.iter_mut() {
        p.home = TilePos::new(4, 4, 2);
    }
    hf_sim::systems::moving::consider(&mut w, cid);
    let evs = w.events.drain();
    if evs
        .iter()
        .any(|e| matches!(e.kind, hf_sim::events::EventKind::LeftTheValley { .. }))
    {
        let c = w.communities.iter().find(|c| c.id == cid).unwrap();
        assert!(
            c.left_for.is_none(),
            "most stayed: the community did not leave"
        );
    }
}

#[test]
fn the_dead_are_laid_to_rest_by_someone_who_came_to_them() {
    // #220: the rite was given on the tick of the death by someone asleep across the valley.
    use hf_sim::events::EventKind;
    let mut w = meadow(22001, 4);
    w.meta.autoplan = false;
    let ids: Vec<_> = w
        .agents
        .iter()
        .filter(|a| a.community.is_some())
        .map(|a| a.id)
        .collect();
    let far = TilePos::new(50, 50, 2);
    let dead = ids[0];
    let i = w.agents.iter().position(|a| a.id == dead).unwrap();
    w.agents[i].pos = far;
    for a in w.agents.iter_mut() {
        a.needs.hunger = 1.0;
        a.needs.rest = 1.0;
    }
    let t0 = w.meta.tick;
    hf_sim::systems::life::die(&mut w, dead, "a fever".into());
    let first = w.events.drain();
    assert!(
        !first
            .iter()
            .any(|e| matches!(e.kind, EventKind::LaidToRest { .. })),
        "no rite on the tick of the death"
    );
    let mut laid = None;
    for _ in 0..TICKS_PER_DAY {
        w.step();
        if let Some(e) = w
            .last_events
            .iter()
            .find(|e| matches!(e.kind, EventKind::LaidToRest { agent, .. } if agent == dead))
        {
            laid = Some((e.tick, e.kind.clone()));
            break;
        }
    }
    let (t, kind) = laid.expect("laid to rest within the day");
    assert!(t > t0 + 60, "it took a walk: {} ticks", t - t0);
    let EventKind::LaidToRest { by, in_absence, .. } = kind else {
        unreachable!()
    };
    assert!(!in_absence);
    let b = w.agents.iter().find(|a| a.id == by).unwrap();
    assert!(
        b.pos.dist(far) <= 2,
        "{} stood by the body, at {:?}",
        b.name,
        b.pos
    );
}

#[test]
fn seeking_out_a_friend_is_said_once_a_day() {
    // #212: "sought out <friend>" was logged on every decision pass while walking there.
    use hf_sim::events::EventKind;
    let mut w = meadow(21201, 2);
    w.meta.autoplan = false;
    let ids: Vec<_> = w
        .agents
        .iter()
        .filter(|a| a.community.is_some())
        .map(|a| a.id)
        .collect();
    for a in w.agents.iter_mut() {
        a.needs.hunger = 1.0;
        a.needs.rest = 1.0;
        for o in &ids {
            if *o != a.id {
                a.relationships.entry(*o).or_default().opinion = 0.9;
            }
        }
    }
    let i = w.agents.iter().position(|a| a.id == ids[1]).unwrap();
    w.agents[i].pos = TilePos::new(50, 50, 2);
    let mut sought = 0;
    for _ in 0..TICKS_PER_HOUR * 6 {
        w.step();
        sought += w
            .last_events
            .iter()
            .filter(|e| matches!(&e.kind, EventKind::Chose { agent, what, .. } if *agent == ids[0] && what.starts_with("sought out")))
            .count();
    }
    assert!(sought <= 1, "said {sought} times in six hours");
}

// The records and their words: #227, #228, #215, #226, #222, #223, #233, #211.

#[test]
fn a_name_the_valley_has_given_is_not_given_again() {
    // #227: two oak figures by one maker were both Ironmourner.
    use hf_sim::ids::ItemId;
    let first = hf_sim::legend::tool_name(1, ItemId(7), ItemKind::Figurine, &[]);
    let second = hf_sim::legend::tool_name(
        1,
        ItemId(7),
        ItemKind::Figurine,
        std::slice::from_ref(&first),
    );
    assert_ne!(first, second);
    assert!(!second.is_empty());
}

#[test]
fn a_hearth_tale_names_three_listeners_and_counts_the_rest() {
    // #228: a line of forty names ran to six rows.
    use hf_sim::events::{Event, EventKind, Severity};
    use hf_sim::ids::EventId;
    let w = meadow(22801, 8);
    let ids: Vec<_> = w
        .agents
        .iter()
        .filter(|a| a.community.is_some())
        .map(|a| a.id)
        .collect();
    let e = Event {
        id: EventId(1),
        tick: w.meta.tick,
        at: None,
        severity: Severity::Notable,
        kind: EventKind::HearthTale {
            teller: ids[0],
            listeners: ids[1..].to_vec(),
            what: "how the valley was found".into(),
        },
    };
    let line = hf_sim::view::narrate::line(&w, &e);
    assert!(line.contains("and 4 others listened"), "{line}");
    assert!(line.matches(", ").count() <= 3, "{line}");
}

#[test]
fn items_are_named_in_words_not_variants() {
    // #215: "carried 8 grainsheaf", "went for grainsheaf".
    assert_eq!(
        hf_sim::systems::society::kind_name(ItemKind::GrainSheaf),
        "grain"
    );
    assert!(!hf_sim::systems::society::kind_name(ItemKind::GrainSheaf).contains("sheaf"));
    // #233: "took up Saewen's clothe".
    assert_eq!(
        hf_sim::legend::noun(ItemKind::Clothes, hf_sim::world::Material::WOOD),
        "clothes"
    );
}

#[test]
fn a_customs_narrative_is_told_once() {
    // #226: the record printed a custom's narrative in the line and again as its detail.
    use hf_sim::events::{Event, EventKind, Severity};
    use hf_sim::ids::{CommunityId, EventId};
    let e = Event {
        id: EventId(1),
        tick: 0,
        at: None,
        severity: Severity::Notable,
        kind: EventKind::CustomFormed {
            community: CommunityId(0),
            custom: 0,
            label: "gather at the hearth at dusk".into(),
            origin: EventId(0),
            narrative: "Since the night ...".into(),
        },
    };
    assert_eq!(hf_sim::view::narrate::detail(&e), None);
}

#[test]
fn a_site_page_tells_each_story_once_with_its_year_and_curses_are_rare() {
    // #222: every lament became a curse, and the same curse was printed four times.
    use hf_sim::chronicle::Chronicle;
    use hf_sim::realm::stories::StoryKind;
    use hf_sim::Session;
    let w = hf_sim::World::from_setup(&hf_sim::setup::WorldSetup::new(1));
    let mut s = Session::new(w, Chronicle::in_memory().unwrap());
    s.write_pending().unwrap();
    let (w, c) = (&s.world, &s.chronicle);
    let r = &w.realm;
    let curses = r
        .stories
        .iter()
        .filter(|st| st.kind == StoryKind::Curse)
        .count();
    let laments = r
        .stories
        .iter()
        .filter(|st| st.kind == StoryKind::Lament)
        .count();
    assert!(
        curses < laments,
        "{curses} curses against {laments} laments"
    );
    // A curse is for a loss worth one (#246: a curse for one person lost), and one lost is
    // told in the singular.
    assert!(r
        .stories
        .iter()
        .all(|st| st.kind != StoryKind::Curse || st.lost >= 5));
    if let Some(one) = r
        .stories
        .iter()
        .find(|st| st.kind == StoryKind::Lament && st.lost == 1)
    {
        let line = hf_sim::realm::stories::telling(r, one);
        assert!(line.contains("one of the"), "{line}");
        assert!(!line.contains("1 of the"), "{line}");
    }
    let mut pages_with_stories = 0;
    for id in 0..r.sites.len().min(40) as u32 {
        let text = hf_sim::view::legends::page(w, c, &format!("site:{id}"));
        let said: Vec<&str> = text.lines().filter(|l| l.contains(" say: ")).collect();
        if said.is_empty() {
            continue;
        }
        pages_with_stories += 1;
        let mut seen = std::collections::BTreeSet::new();
        for l in &said {
            assert!(seen.insert(*l), "site {id} repeats: {l}");
            assert!(
                l.contains("before the portal") || l.contains("after the portal"),
                "{l}"
            );
        }
    }
    assert!(pages_with_stories >= 3, "{pages_with_stories}");
}

#[test]
fn battle_deaths_are_one_cause() {
    // #223: the deaths tally had 704 "fell at <place>" keys.
    use hf_sim::realm::sites::RealmEventKind;
    let m = hf_sim::realm::generate(1, &hf_sim::realm::RealmParams::default());
    let (rl, events) = hf_sim::realm::sites::run(1, &m, 1000);
    let _ = rl;
    let fell: Vec<&String> = events
        .iter()
        .filter_map(|e| match &e.kind {
            RealmEventKind::Died { cause, .. } if cause.starts_with("fell") => Some(cause),
            _ => None,
        })
        .collect();
    assert!(
        fell.iter().all(|c| c.as_str() == "fell in battle"),
        "{:?}",
        fell.first()
    );
}

#[test]
fn a_dropped_order_gives_back_what_it_took_on_open_ground() {
    // #233: materials given back landed on the order's tile (rock, for a Mine order) at 0.5.
    use hf_sim::work::{DesignationKind, Requirement};
    use hf_sim::world::{Material, Tile};
    let mut w = meadow(23302, 2);
    let rock = TilePos::new(40, 40, 2);
    w.grid.set(rock, Tile::solid(Material::GRANITE));
    w.rebuild_caches();
    let mut d = hf_sim::systems::settlement::make(&mut w, DesignationKind::Mine, rock, 5);
    d.requirements = vec![Requirement {
        kind: ItemKind::Plank,
        needed: 4,
        delivered: 3,
        material: Material::OAK,
        want: None,
        quality: 0.9,
    }];
    w.give_back(vec![d]);
    let it = w
        .items
        .iter()
        .find(|it| it.kind == ItemKind::Plank && it.count == 3)
        .expect("the planks came back");
    let ItemPlace::Ground(p) = it.place else {
        panic!("on the ground")
    };
    assert!(w.grid.walkable(p), "on open ground, not in the rock: {p:?}");
    assert!(p.dist(rock) <= 3);
    assert!(
        (it.quality - 0.9).abs() < 1e-3,
        "at the quality brought: {}",
        it.quality
    );
    assert_eq!(it.material, Material::OAK);
}

#[test]
fn the_pasts_deity_does_not_answer_the_same_stretch_every_five_years() {
    // #211: one hamlet's hard stretch was answered seven times running, "and it came to pass".
    let mut setup = hf_sim::setup::WorldSetup::new(1);
    setup.history_years = 300;
    let w = hf_sim::World::from_setup(&setup);
    let a = &w.history.answers;
    assert!(!a.is_empty(), "the deity answered something in 300 years");
    for x in a {
        let same_soon = a
            .iter()
            .filter(|y| {
                y.site == x.site
                    && y.trigger == x.trigger
                    && y.year < x.year
                    && x.year - y.year <= 20
            })
            .count();
        assert!(
            same_soon < 2,
            "site {} answered for {:?} {} times in twenty years",
            x.site,
            x.trigger,
            same_soon + 1
        );
    }
    // And the past's words: no "1 years".
    assert_eq!(hf_sim::deep::ago(1), "the year before the portal");
}

#[test]
fn an_echo_stands_for_one_of_the_valleys_own_who_died_alone() {
    // #229: no echo in 2,100 soak days; the lost are found alive within weeks, so "a lost
    // survivor who died" never happened. Now one who died with none of their own near is
    // echoed too.
    use hf_sim::events::EventKind;
    let mut w = World::opening_world(3);
    let s = hf_sim::thin::stone(&w).unwrap();
    w.ensure_nav();
    // Every lost survivor found at once, so only the valley's own dead can be echoed.
    for sl in w.survivor_pool.iter_mut() {
        if let hf_sim::opening::SurvivorState::Lost { agent, .. } = sl.state {
            sl.state = hf_sim::opening::SurvivorState::Lost { agent, found: true };
        }
    }
    // The stone awake and at full strength, left open.
    w.thin.woke = Some(w.meta.tick);
    w.thin.strength = 1.0;
    w.thin.chosen = Some((w.agents[0].id, false));
    // One of the community dies far from the rest.
    let who = w
        .agents
        .iter()
        .find(|a| a.community.is_some() && a.alive)
        .map(|a| a.id)
        .unwrap();
    let i = w.agents.iter().position(|a| a.id == who).unwrap();
    w.agents[i].pos = TilePos::new(s.x + 40, s.y, s.z);
    hf_sim::systems::life::die(&mut w, who, "froze".into());
    assert!(w.agents[i].died_alone, "none of their own were near");
    let mut seen = None;
    for _ in 0..20 * TICKS_PER_DAY {
        w.step();
        if let Some(e) = w
            .last_events
            .iter()
            .find(|e| matches!(e.kind, EventKind::EchoSeen { .. }))
        {
            seen = Some(e.kind.clone());
            break;
        }
    }
    let EventKind::EchoSeen {
        of, cause, known, ..
    } = seen.expect("an echo within twenty days")
    else {
        unreachable!()
    };
    assert_eq!(of, w.agents[i].name);
    assert_eq!(cause, "froze");
    assert!(!known);
    assert!(w.thin.echoed.contains(&who));
}

#[test]
fn the_director_looks_at_a_fire_nobody_stands_in() {
    // #219: an event cell with nobody in it scored a quarter, so a strike in the open was
    // not worth a look until someone stood in it.
    use hf_sim::events::{Event, EventKind, Severity};
    use hf_sim::ids::EventId;
    use hf_sim::view::director::Director;
    let tick = 1000;
    let fire = TilePos::new(80, 28, 3);
    let store = TilePos::new(20, 20, 3);
    let mut d = Director::default();
    d.observe(
        tick,
        &[
            Event {
                id: EventId(1),
                tick,
                at: Some(fire),
                severity: Severity::Major,
                kind: EventKind::FireStarted {
                    at: fire,
                    cause: "lightning".into(),
                },
            },
            Event {
                id: EventId(2),
                tick,
                at: Some(store),
                severity: Severity::Notable,
                kind: EventKind::Crafted {
                    agent: hf_sim::ids::AgentId(1),
                    kind: ItemKind::Plank,
                    tier: 1,
                },
            },
        ],
    );
    // Two busy people at the store, nobody at the fire.
    let people = [(store, true), (store.offset(1, 0, 0), true)];
    let s = d.suggest(tick + 1, store, &people);
    assert_eq!(s.at, fire, "{}: {}", s.reason, s.score);
}
