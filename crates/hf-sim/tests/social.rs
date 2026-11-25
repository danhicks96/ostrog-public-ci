//! #295: gossip carries what people think of someone; good turns are repaid; the estranged
//! make it up.

use hf_sim::events::{EventKind, Severity};
use hf_sim::ids::{AgentId, TilePos};
use hf_sim::items::{Item, ItemKind, ItemPlace};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

/// A meal `who` carries, for the real-food-sharing tests.
fn carry_meal(w: &mut World, who: AgentId) -> hf_sim::ids::ItemId {
    let id = w.alloc_item_id();
    w.items.push(Item {
        id,
        kind: ItemKind::Meal,
        count: 1,
        place: ItemPlace::Carried(who),
        quality: 0.8,
        material: hf_sim::world::Material::AIR,
        story: None,
        rot: 0.0,
    });
    id
}

fn party(n: usize) -> (World, Vec<usize>) {
    let mut w = World::generate(2951, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 10;
    w.rebuild_caches();
    let ids = w.found_party(n);
    w.meta.autoplan = false;
    let idx = ids
        .iter()
        .map(|id| w.agents.iter().position(|a| a.id == *id).unwrap())
        .collect();
    (w, idx)
}

fn happen(w: &mut World, kind: EventKind) {
    let tick = w.meta.tick;
    let id = w.events.push(tick, None, Severity::Minor, kind);
    let e = w
        .events
        .pending
        .iter()
        .find(|e| e.id == id)
        .unwrap()
        .clone();
    w.last_events = vec![e];
    hf_sim::social::observe(w);
}

#[test]
fn solitary_get_lonely_slower() {
    // #947: the `.max(0.0)` on the sociability extra threw away every rate below 1, so a
    // solitary person's social need fell exactly as fast as a sociable one's.
    let (mut w, p) = party(2);
    for (i, sociability) in [(p[0], 0.1_f32), (p[1], 0.5_f32)] {
        w.agents[i].dispositions = Some(hf_sim::mind::Dispositions {
            sociability,
            ..Default::default()
        });
        w.agents[i].needs.social = 1.0;
        w.agents[i].activity = hf_sim::mind::Activity::Idle;
    }
    for _ in 0..12 * TICKS_PER_HOUR {
        w.meta.tick += 1;
        hf_sim::systems::needs::step(&mut w);
    }
    let (solitary, average) = (w.agents[p[0]].needs.social, w.agents[p[1]].needs.social);
    assert!(
        solitary - average >= 0.15,
        "the solitary one stays lonelier for longer: solitary {solitary}, average {average}"
    );
}

#[test]
fn a_bad_name_travels_by_talk() {
    let (mut w, p) = party(4);
    let (teller, listener, subject) = (w.agents[p[0]].id, w.agents[p[1]].id, w.agents[p[2]].id);
    let t = w.meta.tick;
    w.agents[p[0]].relate(subject, -0.9, 0.5, t);
    w.agents[p[1]].relate(teller, 0.5, 0.5, t);
    let before = w.agents[p[1]].opinion_of(subject);
    for _ in 0..5 {
        happen(
            &mut w,
            EventKind::Talked {
                a: teller,
                b: listener,
                warmth: 0.5,
            },
        );
    }
    let after = w.agents[p[1]].opinion_of(subject);
    assert!(after < before - 0.2, "{before} -> {after}");
    assert!(hf_sim::social::reputation(&w, subject) < 0.0);
}

#[test]
fn repaying_a_good_turn_moves_a_real_meal() {
    let (mut w, p) = party(3);
    let (healer, patient) = (w.agents[p[0]].id, w.agents[p[1]].id);
    happen(&mut w, EventKind::Tended { healer, patient });
    assert!(w.agents[p[1]].owes.iter().any(|x| x.0 == healer));
    w.agents[p[0]].needs.hunger = 0.1;
    w.agents[p[1]].needs.hunger = 0.9;
    w.agents[p[0]].pos = w.agents[p[1]].pos;
    w.items.clear();
    let meal = carry_meal(&mut w, patient);
    w.meta.tick = TICKS_PER_DAY + 12 * TICKS_PER_HOUR + 41;
    hf_sim::social::step(&mut w);
    assert_eq!(
        w.items.iter().find(|it| it.id == meal).map(|it| it.place),
        Some(ItemPlace::Carried(healer)),
        "the meal moved from the giver to the one who was owed"
    );
    assert_eq!(
        w.agents[p[1]].needs.hunger, 0.9,
        "no hunger number is set directly"
    );
    assert_eq!(
        w.agents[p[0]].needs.hunger, 0.1,
        "no hunger number is set directly"
    );
    assert!(w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Repaid { by, to } if by == patient && to == healer)));
}

#[test]
fn no_good_turn_is_repaid_across_a_feud() {
    let (mut w, p) = party(3);
    let (healer, patient) = (w.agents[p[0]].id, w.agents[p[1]].id);
    happen(&mut w, EventKind::Tended { healer, patient });
    hf_sim::feuds::begin(&mut w, healer, patient);
    w.agents[p[0]].needs.hunger = 0.1;
    w.agents[p[1]].needs.hunger = 0.9;
    w.agents[p[0]].pos = w.agents[p[1]].pos;
    w.items.clear();
    let meal = carry_meal(&mut w, patient);
    w.meta.tick = TICKS_PER_DAY + 12 * TICKS_PER_HOUR + 41;
    hf_sim::social::step(&mut w);
    assert_eq!(
        w.items.iter().find(|it| it.id == meal).map(|it| it.place),
        Some(ItemPlace::Carried(patient)),
        "nothing is shared across a feud"
    );
    assert!(!w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Repaid { .. })));
}

#[test]
fn with_no_food_to_hand_nothing_is_repaid() {
    let (mut w, p) = party(3);
    let (healer, patient) = (w.agents[p[0]].id, w.agents[p[1]].id);
    happen(&mut w, EventKind::Tended { healer, patient });
    w.agents[p[0]].needs.hunger = 0.1;
    w.agents[p[1]].needs.hunger = 0.9;
    w.agents[p[0]].pos = w.agents[p[1]].pos;
    w.items.clear();
    w.meta.tick = TICKS_PER_DAY + 12 * TICKS_PER_HOUR + 41;
    hf_sim::social::step(&mut w);
    assert!(
        w.agents[p[1]].owes.iter().any(|x| x.0 == healer),
        "the debt stands"
    );
    assert!(!w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Repaid { .. })));
}

#[test]
fn no_meal_jumps_across_the_camp() {
    let (mut w, p) = party(3);
    let (healer, patient) = (w.agents[p[0]].id, w.agents[p[1]].id);
    happen(&mut w, EventKind::Tended { healer, patient });
    w.agents[p[0]].needs.hunger = 0.1;
    w.agents[p[1]].needs.hunger = 0.9;
    let here = w.agents[p[1]].pos;
    w.agents[p[0]].pos = TilePos::new(here.x + 10, here.y, here.z);
    w.items.clear();
    let meal = carry_meal(&mut w, patient);
    w.meta.tick = TICKS_PER_DAY + 12 * TICKS_PER_HOUR + 41;
    hf_sim::social::step(&mut w);
    assert_eq!(
        w.items.iter().find(|it| it.id == meal).map(|it| it.place),
        Some(ItemPlace::Carried(patient)),
        "no meal jumps 10 tiles across the camp"
    );
    assert!(!w
        .events
        .pending
        .iter()
        .any(|e| matches!(e.kind, EventKind::Repaid { .. })));
    // At 1 tile, the same debt is repaid with a real meal.
    w.agents[p[0]].pos = TilePos::new(here.x + 1, here.y, here.z);
    hf_sim::social::step(&mut w);
    assert_eq!(
        w.items.iter().find(|it| it.id == meal).map(|it| it.place),
        Some(ItemPlace::Carried(healer)),
        "at 1 tile the meal is shared"
    );
}

#[test]
fn two_at_odds_in_good_spirits_sometimes_make_it_up() {
    let (mut w, p) = party(2);
    let (a, b) = (w.agents[p[0]].id, w.agents[p[1]].id);
    let mut made = false;
    for k in 0..60u64 {
        let t = w.meta.tick + k;
        w.meta.tick = t;
        w.agents[p[0]].relate(b, -2.0, 0.1, t);
        w.agents[p[0]].relate(b, 0.6, 0.0, t);
        w.agents[p[1]].relate(a, -2.0, 0.1, t);
        w.agents[p[1]].relate(a, 0.6, 0.0, t);
        w.agents[p[0]].mood = 0.6;
        w.agents[p[1]].mood = 0.6;
        happen(&mut w, EventKind::Talked { a, b, warmth: 0.4 });
        if w.events
            .pending
            .iter()
            .any(|e| matches!(e.kind, EventKind::MadeUp { .. }))
        {
            made = true;
            break;
        }
    }
    assert!(made);
}

/// #1131 (2026-11-15 fix-1131-met-stamp-real-contact): hearing someone talked about moves
/// `last_contact` ("last heard of") but must not stamp `met`, the in-person-only marker.
#[test]
fn gossip_does_not_count_as_meeting() {
    let (mut w, p) = party(3);
    let (teller, listener, subject) = (w.agents[p[0]].id, w.agents[p[1]].id, w.agents[p[2]].id);
    let t0 = w.meta.tick;
    // The teller feels strongly about the absent subject, so gossip has something to say.
    w.agents[p[0]].relate(subject, -0.9, 0.5, t0);
    assert_eq!(
        w.agents[p[1]].relationships.get(&subject).map(|r| r.met),
        None
    );
    for k in 0..10u64 {
        w.meta.tick = t0 + k * TICKS_PER_DAY;
        happen(
            &mut w,
            EventKind::Talked {
                a: teller,
                b: listener,
                warmth: 0.5,
            },
        );
    }
    let r = w.agents[p[1]]
        .relationships
        .get(&subject)
        .expect("gossip should have created a relation entry for the absent subject");
    assert!(
        r.last_contact > t0,
        "last_contact (last heard of) should move on gossip: {}",
        r.last_contact
    );
    assert_eq!(
        r.met, 0,
        "hearing about the subject must not stamp met (real contact only): {}",
        r.met
    );
}

/// #1131 (2026-11-15 fix-1131-met-stamp-real-contact): a finished talk is real, in-person
/// contact and must stamp `met` on both sides.
#[test]
fn talk_stamps_met() {
    use hf_sim::mind::Activity;

    let (mut w, p) = party(2);
    let (a, b) = (w.agents[p[0]].id, w.agents[p[1]].id);
    let t = w.meta.tick;
    assert_eq!(w.agents[p[0]].relationships.get(&b).map(|r| r.met), None);
    assert_eq!(w.agents[p[1]].relationships.get(&a).map(|r| r.met), None);

    // Put them side by side, mid-talk, with its last tick (so act::step finishes it now),
    // and in good odour with each other so it cannot sour into a quarrel instead.
    w.agents[p[0]].pos = w.agents[p[1]].pos;
    w.agents[p[0]].relate(b, 0.9, 0.5, t);
    w.agents[p[1]].relate(a, 0.9, 0.5, t);
    w.agents[p[0]].mood = 0.5;
    w.agents[p[1]].mood = 0.5;
    w.agents[p[0]].activity = Activity::Talking {
        with: b,
        ticks_left: 1,
    };
    w.agents[p[1]].activity = Activity::Talking {
        with: a,
        ticks_left: 1,
    };
    hf_sim::systems::act::step(&mut w);

    assert_eq!(
        w.agents[p[0]].relationships.get(&b).map(|r| r.met),
        Some(t),
        "a should have met b after a finished talk"
    );
    assert_eq!(
        w.agents[p[1]].relationships.get(&a).map(|r| r.met),
        Some(t),
        "b should have met a after a finished talk"
    );
}

fn distant_pair(opinion: f32) -> (World, Vec<usize>) {
    let (mut w, p) = party(2);
    let other = w.agents[p[1]].id;
    let tick = w.meta.tick;
    w.agents[p[0]].relationships.clear();
    w.agents[p[0]].dispositions = Some(hf_sim::mind::Dispositions {
        warmth: 0.5,
        ..Default::default()
    });
    w.agents[p[0]].partner = None;
    w.agents[p[0]].mother = None;
    w.agents[p[0]].father = None;
    w.agents[p[0]].children.clear();
    w.agents[p[0]].relate(other, opinion, 0.5, tick);
    w.agents[p[0]].met(other, tick);
    (w, p)
}

fn distant_days(w: &mut World, first: u64, last: u64) {
    for day in first..=last {
        w.meta.tick = day * TICKS_PER_DAY + 12 * TICKS_PER_HOUR + 41;
        hf_sim::social::step(w);
    }
}

#[test]
fn ties_fade_without_meeting() {
    let (mut w, p) = distant_pair(1.0);
    let other = w.agents[p[1]].id;
    let met = w.agents[p[0]].relationships[&other].met;
    distant_days(&mut w, 1, 40);
    let r = &w.agents[p[0]].relationships[&other];
    assert!(r.opinion < 0.6 && r.opinion > 0.0, "{}", r.opinion);
    assert_eq!(r.met, met);
    assert_eq!(r.last_contact, met, "fading is not contact");
    assert_eq!(r.familiarity, 0.5, "familiarity is a later slice");
}

#[test]
fn kin_fade_to_a_floor_not_to_nothing() {
    for kind in ["partner", "mother", "father", "child", "sibling"] {
        let (mut w, p) = distant_pair(1.0);
        let other = w.agents[p[1]].id;
        match kind {
            "partner" => w.agents[p[0]].partner = Some(other),
            "mother" => w.agents[p[0]].mother = Some(other),
            "father" => w.agents[p[0]].father = Some(other),
            "child" => w.agents[p[0]].children.push(other),
            "sibling" => {
                let parent = AgentId(u32::MAX);
                w.agents[p[0]].mother = Some(parent);
                w.agents[p[1]].mother = Some(parent);
            }
            _ => unreachable!(),
        }
        distant_days(&mut w, 1, 200);
        let opinion = w.agents[p[0]].opinion_of(other);
        assert!((0.3..0.4).contains(&opinion), "{kind}: {opinion}");
    }
}

#[test]
fn a_grudge_cools_untended() {
    let (mut w, p) = distant_pair(-0.8);
    let other = w.agents[p[1]].id;
    distant_days(&mut w, 1, 60);
    let opinion = w.agents[p[0]].opinion_of(other);
    assert!(opinion > -0.4 && opinion < 0.0, "{opinion}");
}

#[test]
fn the_dead_are_not_faded() {
    let (mut w, p) = distant_pair(1.0);
    let other = w.agents[p[1]].id;
    w.agents[p[1]].alive = false;
    w.agents[p[1]].death = Some("old age".into());
    distant_days(&mut w, 1, 200);
    assert_eq!(w.agents[p[0]].opinion_of(other), 1.0);
    // The dead person's own ties are not changed either.
    let tick = w.meta.tick;
    let living = w.agents[p[0]].id;
    w.agents[p[1]].relate(living, -0.8, 0.5, tick);
    distant_days(&mut w, 201, 260);
    assert_eq!(w.agents[p[1]].opinion_of(living), -0.8);
}

#[test]
fn hearsay_does_not_restart_the_fade_grace() {
    let (mut w, p) = distant_pair(1.0);
    let other = w.agents[p[1]].id;
    let met = 12 * TICKS_PER_HOUR + 41;
    w.agents[p[0]].met(other, met);
    w.meta.tick = met + 3 * TICKS_PER_DAY;
    hf_sim::social::step(&mut w);
    assert_eq!(w.agents[p[0]].opinion_of(other), 1.0);
    for day in 1..=40 {
        let tick = day * TICKS_PER_DAY + 12 * TICKS_PER_HOUR + 41;
        w.agents[p[0]].relate(other, 0.0, 0.0, tick);
        distant_days(&mut w, day, day);
    }
    assert!(w.agents[p[0]].opinion_of(other) < 0.6);
    let tick = w.meta.tick;
    w.agents[p[0]].met(other, tick);
    let opinion = w.agents[p[0]].opinion_of(other);
    distant_days(&mut w, 41, 42);
    assert_eq!(w.agents[p[0]].opinion_of(other), opinion);
}

#[test]
fn a_departed_living_person_is_not_treated_as_dead() {
    let (mut w, p) = distant_pair(1.0);
    let other = w.agents[p[1]].id;
    // Leaving and exile keep the person here for history, with no death recorded.
    w.agents[p[1]].alive = false;
    assert!(w.agents[p[1]].death.is_none());
    distant_days(&mut w, 1, 40);
    assert!(w.agents[p[0]].opinion_of(other) < 0.6);
}

#[test]
fn ties_to_travelling_kin_fade_as_living_ties() {
    let (mut w, p) = distant_pair(1.0);
    let other = w.agents[p[1]].id;
    let parent = AgentId(u32::MAX);
    w.agents[p[0]].father = Some(parent);
    w.agents[p[1]].father = Some(parent);
    let traveller = w.agents.remove(p[1]);
    w.overworld.away.push(traveller);
    distant_days(&mut w, 1, 200);
    let opinion = w.agents[p[0]].opinion_of(other);
    assert!((0.3..0.4).contains(&opinion), "{opinion}");
}

#[test]
fn fading_ties_resume_and_match_on_one_and_four_threads() {
    fn run(threads: usize, resume: bool) -> String {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                let (mut w, p) = distant_pair(1.0);
                let other = w.agents[p[1]].id;
                distant_days(&mut w, 1, 20);
                assert!(w.agents[p[0]].opinion_of(other) < 1.0);
                if resume {
                    use hf_sim::save::{decode, encode, snapshot::Snapshot};
                    w = decode(&encode(&Snapshot::capture(&w)).unwrap())
                        .unwrap()
                        .restore();
                }
                distant_days(&mut w, 21, 40);
                hf_sim::digest::digest(&w)
            })
    }
    let expected = run(1, false);
    assert_eq!(expected, run(4, false));
    assert_eq!(expected, run(1, true));
    assert_eq!(expected, run(4, true));
}
