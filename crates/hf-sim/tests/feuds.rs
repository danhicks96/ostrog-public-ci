//! #404: feuds between families, passed to children, and ended (docs/PLAN-FINISH.md M5, "a feud
//! across two generations").

use hf_sim::events::EventKind;
use hf_sim::ids::{AgentId, TilePos};
use hf_sim::justice;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

fn meadow(seed: u64, n: usize) -> (World, Vec<usize>) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(30, 30, 2);
    w.meta.tick = TICKS_PER_HOUR * 10;
    w.rebuild_caches();
    let ids = w.found_party(n);
    w.meta.autoplan = false;
    let idx: Vec<usize> = ids
        .iter()
        .map(|id| w.agents.iter().position(|a| a.id == *id).unwrap())
        .collect();
    for (k, &i) in idx.iter().enumerate() {
        w.agents[i].pos = w.grid.surface(30 + k as i32, 30);
        w.agents[i].activity = hf_sim::mind::Activity::Idle;
    }
    (w, idx)
}

/// `child` is `parent`'s child.
fn parent_of(w: &mut World, parent: usize, child: usize) {
    let (p, c) = (w.agents[parent].id, w.agents[child].id);
    w.agents[child].mother = Some(p);
    w.agents[parent].children.push(c);
}

/// `x` starts a fight with `v` at noon on day `d`, judged that evening.
fn fight(w: &mut World, d: u64, x: usize, v: AgentId) {
    w.meta.tick = d * TICKS_PER_DAY + 12 * TICKS_PER_HOUR;
    let at = w.agents[x].pos;
    justice::offence(w, x, Some(v), "starting a fight", at);
    w.meta.tick = d * TICKS_PER_DAY + 18 * TICKS_PER_HOUR + 300;
    justice::step(w);
}

fn count(w: &World, f: impl Fn(&EventKind) -> bool) -> usize {
    w.events.pending.iter().filter(|e| f(&e.kind)).count()
}

/// Run one event through the feud observer as if it had just happened.
fn happen(w: &mut World, kind: EventKind) {
    let tick = w.meta.tick;
    let id = w
        .events
        .push(tick, None, hf_sim::events::Severity::Notable, kind);
    let e = w
        .events
        .pending
        .iter()
        .find(|e| e.id == id)
        .unwrap()
        .clone();
    w.last_events = vec![e];
    hf_sim::feuds::observe(w);
}

/// The morning's pass over the feuds on day `d`.
fn morning(w: &mut World, d: u64) {
    w.meta.tick = d * TICKS_PER_DAY + 6 * TICKS_PER_HOUR + 400;
    w.last_events.clear();
    hf_sim::feuds::observe(w);
}

#[test]
fn two_fights_between_the_same_two_families_are_a_feud_between_the_families() {
    let (mut w, p) = meadow(4041, 5);
    // p0 and p2 are mother and son; p1 is of another family.
    parent_of(&mut w, p[0], p[2]);
    let (a, b, son) = (w.agents[p[0]].id, w.agents[p[1]].id, w.agents[p[2]].id);
    fight(&mut w, 1, p[0], b);
    assert_eq!(count(&w, |k| matches!(k, EventKind::Feud { .. })), 0);
    // A different one of the same family, against the same other family, a week on.
    fight(&mut w, 8, p[1], son);
    assert_eq!(count(&w, |k| matches!(k, EventKind::Feud { .. })), 1);
    assert_eq!(w.feuds.len(), 1);
    let f = &w.feuds[0];
    let s = f.side_of(a).expect("the mother holds it");
    assert_eq!(f.side_of(son), Some(s), "and her son with her");
    assert_eq!(f.side_of(b), Some(1 - s));
    // Holding it, they think the worse of the other side.
    let ai = p[0];
    assert!(w.agents[ai].opinion_of(b) < 0.0);
}

#[test]
fn a_fight_within_one_family_is_not_a_feud() {
    let (mut w, p) = meadow(4042, 4);
    parent_of(&mut w, p[0], p[1]);
    let (a, b) = (w.agents[p[0]].id, w.agents[p[1]].id);
    fight(&mut w, 1, p[0], b);
    fight(&mut w, 2, p[1], a);
    fight(&mut w, 3, p[0], b);
    assert!(w.feuds.is_empty());
}

#[test]
fn exile_on_anothers_word_sets_the_exiles_family_against_the_accusers() {
    let (mut w, p) = meadow(4043, 5);
    parent_of(&mut w, p[0], p[2]);
    let (a, b, son) = (w.agents[p[0]].id, w.agents[p[1]].id, w.agents[p[2]].id);
    let at = w.agents[p[0]].pos;
    w.meta.tick = TICKS_PER_DAY + 12 * TICKS_PER_HOUR;
    let cid = w.agents[p[0]].community.unwrap();
    justice::accuse(&mut w, cid, b, a, a, justice::DEEP_GATE, at);
    w.meta.tick = TICKS_PER_DAY + 18 * TICKS_PER_HOUR + 300;
    justice::step(&mut w);
    assert_eq!(count(&w, |k| matches!(k, EventKind::Exiled { .. })), 1);
    assert_eq!(w.feuds.len(), 1, "one exile is enough");
    let f = w.feuds[0].clone();
    assert_eq!(f.between(son, b), Some(f.side_of(son).unwrap()));
    // The exile is gone; the son takes the feud up the next morning.
    morning(&mut w, 2);
    assert_eq!(w.feuds[0].heads[f.side_of(son).unwrap()], son);
}

/// The M5 gate: a feud begun by the parents is held by their children once the parents are gone.
#[test]
fn a_feud_begun_by_parents_is_held_by_their_children_after_the_parents_are_gone() {
    let (mut w, p) = meadow(4044, 7);
    // Two families: p0 with her daughter p2, p1 with his son p3; p4 to p6 are of neither.
    parent_of(&mut w, p[0], p[2]);
    parent_of(&mut w, p[1], p[3]);
    let ids: Vec<AgentId> = p.iter().map(|&i| w.agents[i].id).collect();
    fight(&mut w, 1, p[0], ids[1]);
    fight(&mut w, 2, p[1], ids[0]);
    assert_eq!(w.feuds.len(), 1);
    // A child born to one of them afterwards is born into it.
    let baby = ids[5];
    w.meta.tick = 3 * TICKS_PER_DAY;
    happen(
        &mut w,
        EventKind::Born {
            agent: baby,
            mother: ids[2],
            father: None,
        },
    );
    let s0 = w.feuds[0].side_of(ids[0]).unwrap();
    assert_eq!(w.feuds[0].side_of(baby), Some(s0));
    // Whoever pairs with one who holds it takes it up.
    happen(
        &mut w,
        EventKind::Paired {
            a: ids[3],
            b: ids[6],
        },
    );
    assert_eq!(w.feuds[0].side_of(ids[6]), Some(1 - s0));
    // Both parents die.
    for k in [p[0], p[1]] {
        w.agents[k].alive = false;
        w.agents[k].death = Some("old age".into());
    }
    morning(&mut w, 4);
    assert_eq!(w.feuds.len(), 1, "the feud outlives those who began it");
    let f = w.feuds[0].clone();
    assert_eq!(f.side_of(ids[0]), None, "the dead let go of it");
    assert_eq!(f.heads[s0], ids[2], "the daughter has taken it up");
    assert_eq!(f.heads[1 - s0], ids[3], "and the son on the other side");
    assert_eq!(
        count(&w, |k| matches!(k, EventKind::FeudInherited { .. })),
        2
    );
    // The children of the feuding parents fight it on: a judged case between them keeps it alive.
    fight(&mut w, 30, p[2], ids[3]);
    assert_eq!(w.feuds.len(), 1);
    assert_eq!(w.feuds[0].last, w.meta.tick);
}

#[test]
fn a_marriage_between_the_two_families_ends_the_feud() {
    let (mut w, p) = meadow(4045, 6);
    parent_of(&mut w, p[0], p[2]);
    parent_of(&mut w, p[1], p[3]);
    let ids: Vec<AgentId> = p.iter().map(|&i| w.agents[i].id).collect();
    fight(&mut w, 1, p[0], ids[1]);
    fight(&mut w, 2, p[1], ids[0]);
    assert_eq!(w.feuds.len(), 1);
    happen(
        &mut w,
        EventKind::Paired {
            a: ids[2],
            b: ids[3],
        },
    );
    assert!(w.feuds.is_empty());
    assert_eq!(
        count(
            &w,
            |k| matches!(k, EventKind::FeudEnded { how, .. } if how.contains("marriage"))
        ),
        1
    );
}

#[test]
fn a_feud_nobody_feeds_is_let_go_after_five_years() {
    let (mut w, p) = meadow(4046, 5);
    let ids: Vec<AgentId> = p.iter().map(|&i| w.agents[i].id).collect();
    fight(&mut w, 1, p[0], ids[1]);
    fight(&mut w, 2, p[1], ids[0]);
    assert_eq!(w.feuds.len(), 1);
    let last = w.feuds[0].last / TICKS_PER_DAY;
    let years = hf_sim::feuds::FORGOTTEN / TICKS_PER_DAY;
    morning(&mut w, last + years - 1);
    assert_eq!(w.feuds.len(), 1);
    morning(&mut w, last + years + 1);
    assert!(w.feuds.is_empty());
}

#[test]
fn amends_made_by_each_side_to_the_other_are_a_reckoning() {
    let (mut w, p) = meadow(4047, 5);
    let ids: Vec<AgentId> = p.iter().map(|&i| w.agents[i].id).collect();
    // Pardoned the first time each; a second offence within forty days is amends.
    fight(&mut w, 1, p[0], ids[1]);
    fight(&mut w, 2, p[1], ids[0]);
    assert_eq!(w.feuds.len(), 1);
    // Whoever judges is the eldest (nobody is followed), so the judge cannot settle it.
    fight(&mut w, 3, p[0], ids[1]);
    assert_eq!(w.feuds.len(), 1);
    fight(&mut w, 4, p[1], ids[0]);
    assert!(w.feuds.is_empty());
    assert_eq!(
        count(
            &w,
            |k| matches!(k, EventKind::FeudEnded { how, .. } if how.contains("reckoning"))
        ),
        1
    );
}

/// Fights were too rare in play for any feud (two in 200 days of seed 4, M5 review): a quarrel
/// between two who bear each other ill will can come to blows, and the fight is judged.
#[test]
fn a_quarrel_between_two_who_bear_each_other_ill_will_can_come_to_blows() {
    let (mut w, p) = meadow(4048, 4);
    let (i, j) = (p[0], p[1]);
    let (a, b) = (w.agents[i].id, w.agents[j].id);
    let t = w.meta.tick;
    w.agents[i].relate(b, -0.9, 0.5, t);
    w.agents[j].relate(a, -0.9, 0.5, t);
    w.agents[j].pos = w.agents[i].pos.offset(1, 0, 0);
    let mut blows = false;
    for k in 0..400 {
        w.meta.tick = t + k;
        w.agents[i].activity = hf_sim::mind::Activity::Talking {
            with: b,
            ticks_left: 1,
        };
        w.agents[j].activity = hf_sim::mind::Activity::Idle;
        w.events.pending.clear();
        hf_sim::systems::act::step(&mut w);
        if count(&w, |k| matches!(k, EventKind::Fought { .. })) > 0 {
            blows = true;
            break;
        }
    }
    assert!(blows);
    assert_eq!(w.communities[0].accused.len(), 1, "the fight is taken up");
}

/// `by` gave `i` a wound in a fight: `bleeding` per hour, infected or not, on `part`.
fn struck_in_a_fight(
    w: &mut World,
    i: usize,
    by: AgentId,
    part: hf_sim::body::Part,
    bleeding: f32,
    infected: bool,
) {
    hf_sim::systems::health::wound(
        w,
        i,
        part,
        hf_sim::body::WoundKind::Cut,
        0.5,
        bleeding,
        "in a fight",
        Some(by),
    );
    let wd = w.agents[i].body.wounds.last_mut().unwrap();
    wd.in_anger = true;
    wd.infected = infected;
}

/// A fight's wounds name who struck them, and that it was in anger (#404).
#[test]
fn a_fights_wounds_name_who_struck_them() {
    let (mut w, p) = meadow(4049, 3);
    let (a, b) = (w.agents[p[0]].id, w.agents[p[1]].id);
    w.agents[p[1]].pos = w.agents[p[0]].pos.offset(1, 0, 0);
    let mut r = hf_sim::rng::stream(4049, &[1, 2, 3]);
    hf_sim::systems::strain::fight(&mut w, p[0], b, None, &mut r);
    let on_b = w.agents[p[1]].body.wounds.last().expect("b was struck");
    assert_eq!((on_b.by, on_b.in_anger), (Some(a), true));
    let on_a = w.agents[p[0]]
        .body
        .wounds
        .last()
        .expect("a was struck back");
    assert_eq!((on_a.by, on_a.in_anger), (Some(b), true));
}

/// A death from a wound struck in a fight is a killing: the families are in a feud at once, the
/// dead's kin accuses, the killer is exiled, and each side passes to the next generation (#404).
#[test]
fn a_killing_in_a_fight_is_a_feud_at_once_and_exile_and_the_children_take_it_up() {
    let (mut w, p) = meadow(4050, 6);
    // p0 and her son p2; p1 and his daughter p3; p4 and p5 are neither.
    parent_of(&mut w, p[0], p[2]);
    parent_of(&mut w, p[1], p[3]);
    let ids: Vec<AgentId> = p.iter().map(|&i| w.agents[i].id).collect();
    w.meta.tick = 2 * TICKS_PER_DAY + 9 * TICKS_PER_HOUR + 600;
    struck_in_a_fight(&mut w, p[0], ids[1], hf_sim::body::Part::TORSO, 1.0, false);
    w.agents[p[0]].body.blood = 0.31;
    hf_sim::systems::health::step(&mut w);
    assert!(!w.agents[p[0]].alive, "she bled out");
    assert_eq!(
        count(
            &w,
            |k| matches!(k, EventKind::Killing { agent, by } if *agent == ids[0] && *by == ids[1])
        ),
        1
    );
    assert_eq!(w.feuds.len(), 1, "a feud at once, before any judging");
    let f = w.feuds[0].clone();
    let s = f.side_of(ids[2]).expect("her son holds it");
    assert_eq!(f.side_of(ids[1]), Some(1 - s));
    assert_eq!(f.side_of(ids[3]), Some(1 - s), "and the killer's daughter");
    assert_eq!(f.side_of(ids[4]), None);
    let acc = &w.communities[0].accused;
    assert_eq!(acc.len(), 1);
    assert_eq!(
        (acc[0].by, acc[0].of, acc[0].what.as_str()),
        (ids[2], ids[1], justice::KILLING),
        "her son brings it to judgement"
    );
    w.meta.tick = 2 * TICKS_PER_DAY + 18 * TICKS_PER_HOUR + 300;
    justice::step(&mut w);
    assert_eq!(
        count(
            &w,
            |k| matches!(k, EventKind::Exiled { agent, .. } if *agent == ids[1])
        ),
        1
    );
    assert_eq!(w.feuds.len(), 1, "still one feud");
    morning(&mut w, 3);
    let f = &w.feuds[0];
    assert!(f.heads.contains(&ids[2]) && f.heads.contains(&ids[3]));
    assert_eq!(
        count(&w, |k| matches!(k, EventKind::FeudInherited { .. })),
        2,
        "each side's child takes it up"
    );
}

/// A limb lost to a wound struck in a fight is a maiming: a feud, and the maimed accuse (#404).
#[test]
fn a_limb_lost_to_a_wound_struck_in_a_fight_is_a_maiming() {
    let (mut w, p) = meadow(4051, 4);
    parent_of(&mut w, p[1], p[3]);
    let ids: Vec<AgentId> = p.iter().map(|&i| w.agents[i].id).collect();
    w.meta.tick = 2 * TICKS_PER_DAY + 5 * TICKS_PER_HOUR + 600;
    struck_in_a_fight(&mut w, p[0], ids[1], hf_sim::body::Part::ARM_L, 0.0, true);
    w.agents[p[0]].body.illness = Some(hf_sim::body::Illness {
        severity: 0.7,
        since: 0,
        kind: hf_sim::body::IllnessKind::WoundRot,
        treated: None,
    });
    w.agents[p[2]].techniques.learn(
        hf_sim::mind::Technique::HerbLore,
        hf_sim::mind::techniques::Learned::Studied,
        0.8,
        0,
    );
    hf_sim::systems::health::limbs(&mut w);
    assert_eq!(count(&w, |k| matches!(k, EventKind::LimbTaken { .. })), 1);
    assert_eq!(
        count(
            &w,
            |k| matches!(k, EventKind::Maiming { agent, by, .. } if *agent == ids[0] && *by == ids[1])
        ),
        1
    );
    assert_eq!(w.feuds.len(), 1);
    let acc = &w.communities[0].accused;
    assert_eq!(
        (acc[0].by, acc[0].of, acc[0].what.as_str()),
        (ids[0], ids[1], justice::MAIMING)
    );
}

/// A death from a wound not struck in anger (a beast's, a stranger's, one struck in defence) is
/// no killing (#404).
#[test]
fn a_death_from_a_wound_not_struck_in_anger_is_no_killing() {
    let (mut w, p) = meadow(4052, 3);
    let b = w.agents[p[1]].id;
    w.meta.tick = 2 * TICKS_PER_DAY + 9 * TICKS_PER_HOUR + 600;
    struck_in_a_fight(&mut w, p[0], b, hf_sim::body::Part::TORSO, 1.0, false);
    w.agents[p[0]].body.wounds.last_mut().unwrap().in_anger = false;
    w.agents[p[0]].body.blood = 0.31;
    hf_sim::systems::health::step(&mut w);
    assert!(!w.agents[p[0]].alive);
    assert_eq!(count(&w, |k| matches!(k, EventKind::Killing { .. })), 0);
    assert!(w.feuds.is_empty());
    assert!(w.communities[0].accused.is_empty());
}

/// Lonely, with only one other to talk to: they seek them out, unless the two are on opposite
/// sides of a feud (#404: kin keep away from the other family).
#[test]
fn one_in_a_feud_does_not_seek_out_the_other_family_for_company() {
    let talked = |feud: bool| {
        let (mut w, p) = meadow(4053, 2);
        let (a, b) = (w.agents[p[0]].id, w.agents[p[1]].id);
        w.agents[p[1]].pos = w.agents[p[0]].pos.offset(3, 0, 0);
        if feud {
            hf_sim::feuds::begin(&mut w, a, b);
        }
        let mut n = 0;
        for _ in 0..TICKS_PER_HOUR * 3 {
            w.agents[p[0]].needs.social = 0.0;
            w.step();
            if matches!(w.agents[p[0]].activity, hf_sim::mind::Activity::Talking { with, .. } if with == b)
            {
                n += 1;
            }
        }
        n
    };
    assert!(talked(false) > 0, "the control: lonely, they go to talk");
    assert_eq!(talked(true), 0);
}

/// Run one event through the social observer (social.rs), as `happen` does for the feud one.
fn happen_social(w: &mut World, kind: EventKind) {
    let tick = w.meta.tick;
    let id = w
        .events
        .push(tick, None, hf_sim::events::Severity::Notable, kind);
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

/// #1806: a brother who watches his sister quarrel thinks no worse of her; kin who see a
/// quarrel or a fight between two who are not already each other's kin take a little against
/// the other side.
#[test]
fn kin_side_with_their_own() {
    let (mut w, p) = meadow(5001, 4);
    let (ai, bi, broi, stri) = (p[0], p[1], p[2], p[3]);
    // A and the brother share a mother (feuds::kin); nobody else here is kin of A or B.
    let mother = AgentId(90001);
    w.agents[ai].mother = Some(mother);
    w.agents[broi].mother = Some(mother);
    let (a, b) = (w.agents[ai].id, w.agents[bi].id);
    // The brother is awake 5 tiles from A; a stranger stands right beside him.
    w.agents[broi].pos = w.grid.surface(35, 30);
    w.agents[stri].pos = w.agents[broi].pos;

    let before_bro = w.agents[broi].opinion_of(b);
    let before_stranger = w.agents[stri].opinion_of(b);
    happen_social(&mut w, EventKind::Quarrelled { a, b });
    let after_bro = w.agents[broi].opinion_of(b);
    assert!(
        after_bro < before_bro - 0.05,
        "the brother takes a little against the other side: {before_bro} -> {after_bro}"
    );
    assert!(
        (before_bro - after_bro - 0.1).abs() < 0.03,
        "about 0.1 (#1806): {before_bro} -> {after_bro}"
    );
    assert_eq!(
        w.agents[stri].opinion_of(b),
        before_stranger,
        "a stranger standing beside him is unchanged"
    );

    // A second quarrel the same day changes the brother's opinion no further.
    happen_social(&mut w, EventKind::Quarrelled { a, b });
    assert_eq!(w.agents[broi].opinion_of(b), after_bro);
}

/// A feud's only holder of one side going off on a journey must not read as "nobody of
/// one side is left in the valley" — they are still of the community, just off the map
/// (overworld.away), and still hold the feud (feuds.rs's own comment at `daily`).
#[test]
fn a_feud_survives_a_journey() {
    let (mut w, p) = meadow(4060, 2);
    let (a, b) = (w.agents[p[0]].id, w.agents[p[1]].id);
    hf_sim::feuds::begin(&mut w, a, b);
    assert_eq!(w.feuds.len(), 1);

    // a is the only holder of their side; send them off on a 6-day journey, as journeys.rs
    // does: moved whole out of w.agents and into overworld.away with `journey` set.
    let ai = w.agents.iter().position(|x| x.id == a).unwrap();
    let mut agent = w.agents.remove(ai);
    let depart = w.meta.tick;
    let until = depart + 6 * TICKS_PER_DAY;
    agent.journey = Some((until, "Greywater".to_string(), "to see it".to_string()));
    w.overworld.away.push(agent);

    // The daily feud check runs while they are away; the feud must still be running.
    for d in 1..=5 {
        morning(&mut w, d);
        assert_eq!(
            w.feuds.len(),
            1,
            "day {d}: the feud survives while they are away"
        );
    }
    assert_eq!(count(&w, |k| matches!(k, EventKind::FeudEnded { .. })), 0);
    assert_eq!(
        count(&w, |k| matches!(k, EventKind::FeudInherited { .. })),
        0
    );

    // They come back; still holding it, as before.
    let ix = w.overworld.away.iter().position(|x| x.id == a).unwrap();
    let agent = w.overworld.away.remove(ix);
    w.agents.push(agent);
    morning(&mut w, 6);
    assert_eq!(w.feuds.len(), 1);
    assert_eq!(w.feuds[0].heads[w.feuds[0].side_of(a).unwrap()], a);
}
