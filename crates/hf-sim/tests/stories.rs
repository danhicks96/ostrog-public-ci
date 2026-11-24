//! P8 gates (docs/design-peoples.md section 5): every story is a culture's telling of a row of
//! the world's history; one battle is told two ways; stories drift along the map as they are
//! retold, a betrayal naming a real figure who was there; defeats leave grudges; troubles name
//! gods; and a stranger tells their people's stories at the hearth.

use hf_sim::realm::stories::StoryKind;
use hf_sim::World;

fn old_world(seed: u64) -> World {
    let mut s = hf_sim::setup::WorldSetup::new(seed);
    s.realm_years = 1000;
    World::from_setup(&s)
}

#[test]
fn every_story_is_a_telling_of_a_row() {
    let mut w = old_world(31);
    hf_sim::systems::flush::step(&mut w);
    let ids: std::collections::BTreeSet<_> = w.last_events.iter().map(|e| e.id).collect();
    let realm_rows: std::collections::BTreeSet<_> = w
        .last_events
        .iter()
        .filter(|e| matches!(e.kind, hf_sim::events::EventKind::Realm { .. }))
        .map(|e| e.id)
        .collect();
    assert!(!w.realm.stories.is_empty());
    for s in &w.realm.stories {
        assert!(ids.contains(&s.about), "story {} points at no row", s.id);
        assert!(
            realm_rows.contains(&s.about),
            "story {} points at a row that is not the world's",
            s.id
        );
        assert!((s.culture as usize) < w.realm.cultures.len());
    }
}

#[test]
fn one_battle_is_told_two_ways() {
    let w = old_world(32);
    let mut found = false;
    for s in w
        .realm
        .stories
        .iter()
        .filter(|s| matches!(s.kind, StoryKind::Defeat | StoryKind::Betrayal))
    {
        if let Some(other) = w.realm.stories.iter().find(|o| {
            o.about == s.about
                && o.culture != s.culture
                && matches!(
                    o.kind,
                    StoryKind::Victory | StoryKind::Destined | StoryKind::Lament
                )
        }) {
            let a = hf_sim::realm::stories::telling(&w.realm, s);
            let b = hf_sim::realm::stories::telling(&w.realm, other);
            assert_ne!(a, b);
            found = true;
            break;
        }
    }
    assert!(
        found,
        "a battle the winners and the losers tell differently"
    );
}

#[test]
fn stories_drift_and_a_betrayer_was_there() {
    let w = old_world(33);
    let drifted: Vec<_> = w.realm.stories.iter().filter(|s| s.version > 0).collect();
    assert!(
        !drifted.is_empty(),
        "some stories changed as they were retold"
    );
    for s in drifted {
        assert!(matches!(
            s.kind,
            StoryKind::Betrayal | StoryKind::Destined | StoryKind::DrivenOut | StoryKind::Curse
        ));
        if let Some(t) = s.traitor {
            let f = &w.realm.figures[t as usize];
            let people = w.realm.cultures[s.culture as usize].people;
            assert_eq!(f.people, people, "a betrayer of their own people");
            assert!(
                f.born < s.year && f.died.as_ref().is_none_or(|(y, _)| *y >= s.year),
                "alive then"
            );
        }
    }
}

#[test]
fn defeats_leave_grudges_and_troubles_name_gods() {
    let mut grudges = 0;
    let mut gods = 0;
    for seed in 34..38 {
        let w = old_world(seed);
        for c in &w.realm.cultures {
            for g in &c.grudges {
                assert_ne!(g.against, c.people, "no grudge against themselves");
                grudges += 1;
            }
            gods += c.gods.len();
        }
    }
    assert!(grudges > 0 && gods > 0, "{grudges} grudges, {gods} gods");
}

#[test]
fn a_stranger_tells_their_peoples_story_at_the_hearth() {
    let mut w = old_world(38);
    let c = w.realm.stories.last().map(|s| s.culture).expect("stories");
    let id = w.spawn_agent(
        hf_sim::agents::Species::Person,
        hf_sim::ids::TilePos::new(30, 30, 2),
    );
    hf_sim::species::raise_in(&mut w, id, c);
    let a = w.agents.iter().find(|a| a.id == id).unwrap();
    assert!(
        !a.stories.is_empty(),
        "raised knowing their people's stories"
    );
    assert!(a
        .stories
        .iter()
        .all(|s| w.realm.stories.iter().any(|x| x.id == *s && x.culture == c)));
}

#[test]
fn a_story_told_at_the_hearth_is_heard_as_their_people_tell_it() {
    use hf_sim::mind::activity::Activity;
    use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
    let mut w = old_world(39);
    let c = w.realm.stories.last().map(|s| s.culture).expect("stories");
    let cid = w
        .agents
        .iter()
        .find_map(|a| a.community)
        .expect("a community");
    let home = w.communities.iter().find(|x| x.id == cid).unwrap().home;
    if hf_sim::systems::culture::hearth_of(&w, cid).is_none() {
        w.place_structure(
            hf_sim::structures::StructureKind::Hearth,
            home,
            hf_sim::world::Material::STONE_BLOCK,
            true,
        );
    }
    let h = hf_sim::systems::culture::hearth_of(&w, cid).unwrap();
    let listener = w
        .agents
        .iter()
        .find(|a| a.alive && a.community == Some(cid))
        .map(|a| a.id)
        .unwrap();
    // Everyone else is away from the fire tonight.
    for a in w.agents.iter_mut().filter(|a| a.id != listener) {
        a.pos = hf_sim::ids::TilePos::new(h.x + 20, h.y + 20, h.z);
    }
    let stranger = w.spawn_agent(hf_sim::agents::Species::Person, h.offset(1, 0, 0));
    hf_sim::species::raise_in(&mut w, stranger, c);
    for a in w.agents.iter_mut() {
        if a.id == stranger {
            a.community = Some(cid);
        }
        if a.id == listener {
            a.pos = h.offset(-1, 0, 0);
            a.stories.clear();
        }
        if a.id == stranger || a.id == listener {
            a.activity = Activity::Idle;
        }
    }
    let told: Vec<u32> = w
        .agents
        .iter()
        .find(|a| a.id == stranger)
        .unwrap()
        .stories
        .clone();
    assert!(!told.is_empty());
    let day = w.meta.tick / TICKS_PER_DAY + 1;
    w.meta.tick = day * TICKS_PER_DAY + 19 * TICKS_PER_HOUR + TICKS_PER_HOUR / 2;
    hf_sim::systems::culture::step(&mut w);
    let tale = w.events.pending.iter().find_map(|e| match &e.kind {
        hf_sim::events::EventKind::HearthTale {
            teller,
            listeners,
            what,
        } if *teller == stranger => Some((listeners.clone(), what.clone())),
        _ => None,
    });
    let (listeners, what) = tale.expect("the stranger told a story at the hearth");
    assert_eq!(listeners, vec![listener]);
    assert!(what.starts_with("how their people tell it: "), "{what}");
    let heard = &w.agents.iter().find(|a| a.id == listener).unwrap().stories;
    assert_eq!(heard.len(), 1, "one story an evening");
    assert!(told.contains(&heard[0]));
    let st = w.realm.stories.iter().find(|s| s.id == heard[0]).unwrap();
    assert_eq!(st.culture, c, "the stranger's people's own telling");
    assert!(what.ends_with(&hf_sim::realm::stories::telling(&w.realm, st)));
}

/// A hearth of `n` sitters, all of one people; the first holds story S and so do all but
/// `lacking` of the rest. Returns the story told that evening to the circle, if any.
fn hearth_with(
    seed: u64,
    n: usize,
    lacking: usize,
) -> (World, Vec<hf_sim::ids::AgentId>, Option<u32>) {
    use hf_sim::mind::activity::Activity;
    use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
    let mut w = old_world(seed);
    let c = w.realm.stories.last().map(|s| s.culture).expect("stories");
    let cid = w
        .agents
        .iter()
        .find_map(|a| a.community)
        .expect("a community");
    let home = w.communities.iter().find(|x| x.id == cid).unwrap().home;
    if hf_sim::systems::culture::hearth_of(&w, cid).is_none() {
        w.place_structure(
            hf_sim::structures::StructureKind::Hearth,
            home,
            hf_sim::world::Material::STONE_BLOCK,
            true,
        );
    }
    let h = hf_sim::systems::culture::hearth_of(&w, cid).unwrap();
    for a in w.agents.iter_mut() {
        a.pos = hf_sim::ids::TilePos::new(h.x + 20, h.y + 20, h.z);
    }
    let mut ids = Vec::new();
    for _ in 0..n {
        let id = w.spawn_agent(hf_sim::agents::Species::Person, h.offset(1, 0, 0));
        hf_sim::species::raise_in(&mut w, id, c);
        ids.push(id);
    }
    let s = w.agents.iter().find(|a| a.id == ids[0]).unwrap().stories[0];
    for (k, id) in ids.iter().enumerate() {
        let a = w.agents.iter_mut().find(|a| a.id == *id).unwrap();
        a.community = Some(cid);
        a.activity = Activity::Idle;
        a.pos = h.offset(1, 0, 0);
        a.stories.clear();
        if k < n - lacking {
            a.stories.push(s);
        }
    }
    let day = w.meta.tick / TICKS_PER_DAY + 1;
    w.meta.tick = day * TICKS_PER_DAY + 19 * TICKS_PER_HOUR + TICKS_PER_HOUR / 2;
    hf_sim::systems::culture::step(&mut w);
    let told = w.events.pending.iter().find_map(|e| match &e.kind {
        hf_sim::events::EventKind::HearthTale { what, .. }
            if what.starts_with("how their people tell it: ") =>
        {
            Some(s)
        }
        _ => None,
    });
    (w, ids, told)
}

#[test]
fn a_story_most_have_heard_is_not_told_again() {
    // Eight at the fire, seven know it: one newcomer is not enough to tell it to everyone.
    let (_, _, told) = hearth_with(41, 8, 1);
    assert!(told.is_none(), "a story seven of eight know is not told");
    // Four of the eight lack it: it is told, and all four now hold it.
    let (w, ids, told) = hearth_with(41, 8, 4);
    let s = told.expect("a story half the circle lacks is told");
    for id in &ids {
        let a = w.agents.iter().find(|a| a.id == *id).unwrap();
        assert!(a.stories.contains(&s), "everyone now holds it");
    }
}
