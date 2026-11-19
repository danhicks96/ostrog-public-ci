//! #487: people teach, tell, and change from what they hear. Know-how passes in talk only from
//! one who knows it well enough and is willing, to one who believes them; it is kept with its
//! teller, can be refused or misheard, does not pass through someone who refused it, and changes
//! what the learner later weighs.

use hf_sim::events::{Event, EventKind, Severity};
use hf_sim::ids::{AgentId, EventId, TilePos};
use hf_sim::items::ItemKind;
use hf_sim::mind::techniques::Learned;
use hf_sim::mind::telling::{Took, What};
use hf_sim::mind::{Activity, Dispositions, Technique};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn camp(seed: u64, n: usize) -> (World, Vec<AgentId>) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(20, 20, 2);
    w.meta.tick = TICKS_PER_DAY * 30 + TICKS_PER_HOUR * 10;
    w.rebuild_caches();
    let ids = w.found_party(n);
    w.items.retain(|it| !it.kind.is_food());
    let at = w.grid.surface(22, 18);
    w.drop_item(ItemKind::Meal, 90, at);
    w.ensure_nav();
    for a in w.agents.iter_mut() {
        a.needs.hunger = 0.95;
        a.needs.thirst = 0.95;
        a.needs.rest = 0.9;
        a.needs.social = 0.95;
        a.needs.warmth = 1.0;
        a.activity = Activity::Idle;
        a.idle_ticks = 0;
        a.techniques = Default::default();
        a.relationships.clear();
        a.traits = Default::default();
        a.dispositions = Some(Dispositions::default());
    }
    (w, ids)
}

fn idx(w: &World, id: AgentId) -> usize {
    w.agents.iter().position(|a| a.id == id).unwrap()
}

/// `a` and `b` finish a talk, `times` times an hour apart.
fn talk(w: &mut World, a: AgentId, b: AgentId, times: usize) {
    for _ in 0..times {
        w.meta.tick += TICKS_PER_HOUR;
        w.last_events = vec![Event {
            id: EventId(0),
            tick: w.meta.tick,
            at: None,
            severity: Severity::Minor,
            kind: EventKind::Talked { a, b, warmth: 0.6 },
        }];
        hf_sim::social::observe(w);
    }
}

fn knows(w: &World, who: AgentId, t: Technique) -> bool {
    w.agents[idx(w, who)].techniques.knows(t)
}

fn set_opinion(w: &mut World, of: AgentId, about: AgentId, opinion: f32, fam: f32) {
    let i = idx(w, of);
    let t = w.meta.tick;
    w.agents[i].relate(about, opinion, fam, t);
}

fn teacher(w: &mut World, who: AgentId, t: Technique) {
    let i = idx(w, who);
    let now = w.meta.tick;
    w.agents[i].techniques.learn(t, Learned::Past, 0.7, now);
}

#[test]
fn a_trusted_friend_is_taught_and_the_teaching_is_kept_with_its_teller() {
    let (mut w, ids) = camp(4871, 2);
    let (a, b) = (ids[0], ids[1]);
    teacher(&mut w, a, Technique::ForageFar);
    set_opinion(&mut w, b, a, 0.6, 0.5);
    talk(&mut w, a, b, 40);
    let l = &w.agents[idx(&w, b)];
    let t = l
        .told
        .iter()
        .find(|t| t.what == What::Technique(Technique::ForageFar))
        .expect("told");
    assert_eq!(t.from, a);
    assert_ne!(t.took, Took::Refused);
    assert!(l.techniques.knows(Technique::ForageFar));
    let k = l
        .techniques
        .list
        .iter()
        .find(|k| k.technique == Technique::ForageFar)
        .unwrap();
    assert_eq!(k.learned, Learned::Taught);
    assert_eq!(k.from, w.agents[idx(&w, a)].name);
    assert!(
        k.skill <= hf_sim::mind::telling::TOLD_SKILL,
        "told is less than watched"
    );
}

#[test]
fn the_same_word_is_taken_by_a_communal_listener_and_refused_by_a_self_reliant_one() {
    let mut took = Vec::new();
    for independence in [0.1, 0.9] {
        let (mut w, ids) = camp(4872, 2);
        let (a, b) = (ids[0], ids[1]);
        teacher(&mut w, a, Technique::ForageFar);
        set_opinion(&mut w, b, a, 0.1, 0.2);
        let i = idx(&w, b);
        w.agents[i].dispositions = Some(Dispositions {
            independence,
            ..Dispositions::default()
        });
        talk(&mut w, a, b, 40);
        let l = &w.agents[idx(&w, b)];
        took.push((
            l.told.first().map(|t| t.took),
            l.techniques.knows(Technique::ForageFar),
        ));
    }
    assert!(took[0].1, "communal listener takes it: {took:?}");
    assert_eq!(took[1], (Some(Took::Refused), false), "{took:?}");
}

#[test]
fn one_who_thinks_ill_of_the_listener_withholds_unless_warm() {
    let mut learned = Vec::new();
    for warmth in [0.5, 0.9] {
        let (mut w, ids) = camp(4873, 2);
        let (a, b) = (ids[0], ids[1]);
        teacher(&mut w, a, Technique::ForageFar);
        set_opinion(&mut w, b, a, 0.6, 0.5);
        set_opinion(&mut w, a, b, -0.6, 0.5);
        let i = idx(&w, a);
        w.agents[i].dispositions = Some(Dispositions {
            warmth,
            ..Dispositions::default()
        });
        talk(&mut w, a, b, 40);
        let kept = w.events.pending.iter().any(|e| {
            matches!(&e.kind, EventKind::Chose { agent, what, .. }
                if *agent == a && what.starts_with("kept foraging far afield to themselves"))
        });
        learned.push((knows(&w, b, Technique::ForageFar), kept));
    }
    assert_eq!(
        learned[0],
        (false, true),
        "withheld, and said why: {learned:?}"
    );
    assert!(learned[1].0, "a warm teller tells anyway: {learned:?}");
}

#[test]
fn what_one_refused_does_not_pass_through_them() {
    let (mut w, ids) = camp(4874, 3);
    let (a, c, d) = (ids[0], ids[1], ids[2]);
    teacher(&mut w, a, Technique::ForageFar);
    set_opinion(&mut w, c, a, -0.2, 0.1);
    set_opinion(&mut w, d, c, 0.8, 0.8);
    talk(&mut w, a, c, 40);
    assert!(!knows(&w, c, Technique::ForageFar));
    assert!(w.agents[idx(&w, c)]
        .told
        .iter()
        .all(|t| t.took == Took::Refused));
    talk(&mut w, c, d, 40);
    assert!(
        !knows(&w, d, Technique::ForageFar),
        "no source, no knowledge"
    );
    // A refused telling is not pressed again by the same teller for days.
    let offers = w.agents[idx(&w, c)].told.len();
    assert!(offers <= 2, "{offers} offers in 40 hours");
}

#[test]
fn a_poultice_taught_in_talk_makes_tending_a_choice_later() {
    let run = |taught: bool| {
        let (mut w, ids) = camp(4875, 3);
        let (a, b, sick) = (ids[0], ids[1], ids[2]);
        teacher(&mut w, a, Technique::Poultice);
        set_opinion(&mut w, b, a, 0.7, 0.6);
        if taught {
            talk(&mut w, a, b, 60);
            assert!(knows(&w, b, Technique::Poultice), "taught");
        }
        let bi = idx(&w, b);
        let si = idx(&w, sick);
        let near = w.agents[bi].pos.offset(3, 0, 0);
        w.agents[si].pos = w.grid.surface(near.x, near.y);
        w.agents[si].body.illness = Some(hf_sim::body::Illness {
            severity: 0.5,
            since: 0,
            kind: Default::default(),
            treated: None,
        });
        let p = w.agents[bi].pos.offset(-2, 0, 0);
        let p = w.grid.surface(p.x, p.y);
        w.drop_item(ItemKind::Poultice, 3, p);
        let ai = idx(&w, a);
        w.agents[ai].pos = w.grid.surface(50, 50);
        w.agents[bi].activity = Activity::Idle;
        w.agents[bi].idle_ticks = 0;
        w.trace_why(Some(b));
        hf_sim::systems::decide::step(&mut w);
        let c = w.why.as_ref().unwrap().latest().cloned();
        c.map(|c| c.options.iter().any(|o| o.cycle == "tend"))
            .unwrap_or(false)
    };
    assert!(
        !run(false),
        "without the poultice lore a stranger is not theirs to tend"
    );
    assert!(run(true), "taught, they weigh tending the sick");
}
