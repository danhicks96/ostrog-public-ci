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
use rand::Rng;

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

fn a_lesson_talk(w: &mut World, a: AgentId, b: AgentId, misheard: bool) {
    use hf_sim::rng::{purpose, stream};
    let tick = (1..=1000)
        .map(|h| w.meta.tick + h * TICKS_PER_HOUR)
        .find(|&tick| {
            let mut offer = stream(
                w.meta.seed,
                &[purpose::AGENT_SOCIAL, 0x1E55, a.0 as u64, b.0 as u64, tick],
            );
            let mut hearing = stream(
                w.meta.seed,
                &[purpose::AGENT_SOCIAL, 0x487B, a.0 as u64, b.0 as u64, tick],
            );
            offer.gen_bool(0.25)
                && (hearing.gen::<f32>() < hf_sim::mind::telling::MISHEAR) == misheard
        })
        .expect("a seeded talk with the requested hearing outcome");
    w.meta.tick = tick - TICKS_PER_HOUR;
    talk(w, a, b, 1);
}

#[test]
fn a_lesson_is_told_and_kept_with_its_teller() {
    use hf_sim::mind::{Evidence, ObservationKind, OutcomeKind};
    use hf_sim::save::{decode, encode, snapshot::Snapshot};
    let run = |threads| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                let mut digests = Vec::new();
                for feud in [false, true] {
                    let (mut w, ids) = camp(4879, 2);
                    let (a, b) = (ids[0], ids[1]);
                    let (ai, bi) = (idx(&w, a), idx(&w, b));
                    if feud {
                        hf_sim::feuds::begin(&mut w, a, b);
                        assert!(hf_sim::feuds::across(&w, a, b));
                    }
                    // Isolate willingness across a feud from the listener's ordinary trust gate.
                    w.agents[ai].relationships.clear();
                    w.agents[bi].relationships.clear();
                    set_opinion(&mut w, a, b, 0.8, 0.8);
                    set_opinion(&mut w, b, a, 0.8, 0.8);
                    let at = TilePos::new(40, 40, 2);
                    let tick = w.meta.tick;
                    w.agents[ai].learning.learn(
                        ObservationKind::Cold,
                        OutcomeKind::Death,
                        Some(at),
                        true,
                        tick,
                        None,
                    );
                    w.agents[ai].learning.lessons[0].confidence = 0.7;
                    a_lesson_talk(&mut w, a, b, false);
                    let lesson = w.agents[bi].learning.lesson(
                        ObservationKind::Cold,
                        OutcomeKind::Death,
                        Some(at),
                    );
                    if feud {
                        assert!(
                            w.agents[bi].learning.lessons.is_empty(),
                            "the feud blocks the telling"
                        );
                    } else {
                        let lesson = lesson.expect("the listener holds the teller's lesson");
                        assert!(lesson.confidence > 0.0 && lesson.confidence <= 0.42);
                        assert!(w.agents[bi]
                            .told
                            .iter()
                            .any(|t| t.from == a && t.took == Took::Taken));
                        assert!(w.agents[bi]
                            .learning
                            .observations
                            .iter()
                            .any(|o| o.kind == ObservationKind::Cold
                                && o.pos == at
                                && o.evidence == Evidence::ToldBy(a)));
                    }
                    // Stage a complete tick before saving, including pending fixture events.
                    w.step();
                    let mut back = decode(&encode(&Snapshot::capture(&w)).unwrap())
                        .unwrap()
                        .restore();
                    assert_eq!(hf_sim::digest::digest(&w), hf_sim::digest::digest(&back));
                    w.run(100);
                    back.run(100);
                    assert_eq!(hf_sim::digest::digest(&w), hf_sim::digest::digest(&back));
                    digests.push(hf_sim::digest::digest(&w));
                }
                digests
            })
    };
    assert_eq!(run(1), run(4));
}

#[test]
fn a_misheard_lesson_keeps_the_teller_but_points_one_tile_away() {
    use hf_sim::mind::{Evidence, ObservationKind, OutcomeKind};
    let (mut w, ids) = camp(4880, 2);
    let (a, b) = (ids[0], ids[1]);
    let ai = idx(&w, a);
    set_opinion(&mut w, b, a, 0.8, 0.8);
    let at = TilePos::new(40, 40, 2);
    let tick = w.meta.tick;
    w.agents[ai].learning.learn(
        ObservationKind::Cold,
        OutcomeKind::Death,
        Some(at),
        true,
        tick,
        None,
    );
    w.agents[ai].learning.lessons[0].confidence = 0.7;
    a_lesson_talk(&mut w, a, b, true);
    let listener = &w.agents[idx(&w, b)];
    assert!(listener
        .learning
        .lesson(ObservationKind::Cold, OutcomeKind::Death, Some(at))
        .is_none());
    let lesson = listener
        .learning
        .lessons
        .iter()
        .find(|l| l.cue == ObservationKind::Cold)
        .expect("a misheard warning");
    let heard = lesson.pos.unwrap();
    assert_eq!(heard.dist(at), 1);
    assert!(lesson.confidence <= 0.42);
    assert!(listener
        .learning
        .observations
        .iter()
        .any(|o| o.pos == heard && o.evidence == Evidence::ToldBy(a)));
    assert!(listener
        .told
        .iter()
        .any(|t| t.from == a && t.took == Took::Misheard));
}

#[test]
fn a_misheard_warning_does_not_replace_a_lesson_already_held() {
    use hf_sim::mind::{ObservationKind, OutcomeKind};
    let (mut w, ids) = camp(4882, 2);
    let (a, b) = (ids[0], ids[1]);
    let (ai, bi) = (idx(&w, a), idx(&w, b));
    let at = TilePos::new(40, 40, 2);
    let heard = at.offset(1, 0, 0);
    let tick = w.meta.tick;
    for (i, pos, confidence) in [(ai, at, 0.7), (bi, heard, 0.9)] {
        w.agents[i].learning.learn(
            ObservationKind::Cold,
            OutcomeKind::Death,
            Some(pos),
            true,
            tick,
            None,
        );
        w.agents[i].learning.lessons[0].confidence = confidence;
    }
    set_opinion(&mut w, b, a, 0.8, 0.8);
    a_lesson_talk(&mut w, a, b, true);
    let lesson = w.agents[bi]
        .learning
        .lesson(ObservationKind::Cold, OutcomeKind::Death, Some(heard))
        .unwrap();
    assert_eq!(
        lesson.confidence, 0.9,
        "hearsay must not weaken the held lesson"
    );
    assert_eq!(lesson.last_tested, tick);
    assert!(w.agents[bi]
        .told
        .iter()
        .any(|t| t.from == a && t.took == Took::Misheard));
}

#[test]
fn a_refused_lesson_does_not_spread_and_is_not_pressed_again() {
    use hf_sim::mind::{ObservationKind, OutcomeKind};
    let (mut w, ids) = camp(4881, 3);
    let (a, b, c) = (ids[0], ids[1], ids[2]);
    let ai = idx(&w, a);
    let at = TilePos::new(40, 40, 2);
    let tick = w.meta.tick;
    w.agents[ai].learning.learn(
        ObservationKind::Cold,
        OutcomeKind::Death,
        Some(at),
        true,
        tick,
        None,
    );
    w.agents[ai].learning.lessons[0].confidence = 0.7;
    set_opinion(&mut w, b, a, -0.6, 0.0);
    set_opinion(&mut w, c, b, 0.8, 0.8);
    a_lesson_talk(&mut w, a, b, false);
    talk(&mut w, a, b, 40);
    let listener = &w.agents[idx(&w, b)];
    let refused: Vec<_> = listener
        .told
        .iter()
        .filter(|t| matches!(t.what, What::Lesson { .. }) && t.from == a)
        .collect();
    assert_eq!(refused.len(), 1);
    assert_eq!(refused[0].took, Took::Refused);
    assert!(listener.learning.lessons.is_empty());
    talk(&mut w, b, c, 40);
    assert!(w.agents[idx(&w, c)].learning.lessons.is_empty());
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
