//! #454: a role is the work someone has been doing (duties.rs), not one of each handed to the
//! best hand by skill (docs/design-institutions.md).

use hf_sim::duties::{self, DUTY_MINUTES, KINDS};
use hf_sim::events::EventKind;
use hf_sim::mind::Skill;
use hf_sim::society::Role;
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_YEAR};
use hf_sim::work::DesignationKind;
use hf_sim::World;

fn party(seed: u64) -> World {
    let mut w = World::opening_world(seed);
    for a in w.agents.iter_mut() {
        a.duties = Default::default();
        a.role = None;
    }
    w
}

fn people(w: &World) -> Vec<usize> {
    (0..w.agents.len())
        .filter(|&i| w.agents[i].alive && w.agents[i].community.is_some())
        .collect()
}

fn work(w: &mut World, i: usize, kind: DesignationKind, minutes: u32) {
    for _ in 0..minutes {
        duties::worked(w, i, kind);
    }
}

#[test]
fn the_one_who_has_been_cooking_is_the_cook_not_the_best_cook_who_never_did() {
    let mut w = party(4541);
    let p = people(&w);
    let (cook, skilled) = (p[0], p[1]);
    w.agents[skilled].skills.practise(Skill::Cooking, 50.0);
    work(&mut w, cook, DesignationKind::Cook, 300);
    duties::daily(&mut w);
    assert_eq!(w.agents[cook].role, Some(Role::Cook));
    assert_ne!(
        w.agents[skilled].role,
        Some(Role::Cook),
        "skill alone gives no role"
    );
    assert!(
        p.iter().skip(2).all(|&i| w.agents[i].role.is_none()),
        "nobody who has done no work holds a role"
    );
    assert!(w.events.pending.iter().any(
        |e| matches!(&e.kind, EventKind::RoleTaken { agent, role } if *agent == w.agents[cook].id && role == "cook")
    ));
}

#[test]
fn a_role_can_be_shared_and_one_person_can_hold_two_duties() {
    let mut w = party(4542);
    let p = people(&w);
    work(&mut w, p[0], DesignationKind::Mine, 400);
    work(&mut w, p[1], DesignationKind::Mine, 300);
    work(&mut w, p[1], DesignationKind::Haul, 200);
    duties::daily(&mut w);
    assert_eq!(w.agents[p[0]].role, Some(Role::Miner));
    assert_eq!(w.agents[p[1]].role, Some(Role::Miner), "two miners");
    assert_eq!(
        w.agents[p[1]].duties.held,
        vec![Role::Miner, Role::Hauler],
        "and one of them hauls too"
    );
    // Their duties weigh their choice of work, as the role did.
    let held = &w.agents[p[1]].duties.held;
    assert!(duties::fit(held, DesignationKind::Haul) > 1.0);
    assert!(duties::fit(held, DesignationKind::Mine) > 1.0);
    assert!(duties::fit(held, DesignationKind::Cook) < 1.0);
    assert_eq!(duties::fit(&[], DesignationKind::Cook), 1.0);
}

#[test]
fn a_duty_nobody_keeps_doing_fades_and_passes_to_whoever_takes_it_up() {
    let mut w = party(4543);
    let p = people(&w);
    let (cook, other) = (p[0], p[1]);
    work(&mut w, cook, DesignationKind::Cook, 600);
    duties::daily(&mut w);
    assert_eq!(w.agents[cook].role, Some(Role::Cook));
    // The cook stops; someone else cooks a couple of hours a day.
    let mut days = 0;
    while w.agents[cook].role.is_some() {
        w.meta.tick += TICKS_PER_DAY;
        work(&mut w, other, DesignationKind::Cook, 120);
        duties::daily(&mut w);
        days += 1;
        assert!(days < 60, "the duty never faded");
    }
    assert_eq!(
        w.agents[other].role,
        Some(Role::Cook),
        "the one who took it up"
    );
    assert!(w.events.pending.iter().any(
        |e| matches!(&e.kind, EventKind::RoleLeft { agent, .. } if *agent == w.agents[cook].id)
    ));
}

#[test]
fn in_an_opening_roles_come_only_from_work_done() {
    let mut w = party(4544);
    for _ in 0..(3 * TICKS_PER_DAY) {
        w.step();
    }
    let p = people(&w);
    assert!(
        p.iter()
            .any(|&i| w.agents[i].duties.minutes.iter().any(|&m| m > 0)),
        "work at orders is counted"
    );
    for &i in &p {
        let a = &w.agents[i];
        if let Some(r) = a.role {
            let k = KINDS.iter().position(|x| *x == r).unwrap();
            assert!(
                a.duties.minutes[k] >= DUTY_MINUTES,
                "{} holds {r:?} with {} minutes of it",
                a.name,
                a.duties.minutes[k]
            );
        }
    }
}

/// #602: in an opening people gather food from the first day, and those who gather most are
/// known for it; the hurt are tended, and whoever tends them comes to hold it as a duty.
#[test]
fn those_who_gather_and_tend_hold_those_duties() {
    let mut w = party(4545);
    let p = people(&w);
    let mut tends: std::collections::BTreeMap<hf_sim::ids::AgentId, u32> = Default::default();
    for day in 0..4u64 {
        // Each morning someone is hurt and needs nursing.
        let k = p[(day as usize) % 2];
        hf_sim::systems::health::wound(
            &mut w,
            k,
            hf_sim::body::Part::ARM_L,
            hf_sim::body::WoundKind::Cut,
            0.6,
            0.4,
            "in a test",
            None,
        );
        for _ in 0..TICKS_PER_DAY {
            w.step();
            for e in &w.last_events {
                if let EventKind::Tended { healer, .. } = e.kind {
                    *tends.entry(healer).or_default() += 1;
                }
            }
        }
    }
    assert!(
        p.iter()
            .any(|&i| w.agents[i].duties.held.contains(&Role::Forager)),
        "someone gathers enough to be known for it"
    );
    let (healer, n) = tends
        .iter()
        .max_by_key(|(id, n)| (**n, std::cmp::Reverse(id.0)))
        .map(|(id, n)| (*id, *n))
        .expect("the hurt were tended");
    let i = w.agents.iter().position(|a| a.id == healer).unwrap();
    assert!(n >= 2, "tended {n} times");
    assert!(
        w.agents[i].duties.held.contains(&Role::Healer),
        "{} tended {n} times and holds {:?}",
        w.agents[i].name,
        w.agents[i].duties.held
    );
}

#[test]
fn tending_and_gathering_fade_when_they_stop() {
    let mut w = party(4546);
    let p = people(&w);
    duties::credit(&mut w, p[0], Role::Healer, duties::TEND_CREDIT * 3);
    duties::credit(&mut w, p[0], Role::Hauler, 3000);
    duties::credit(&mut w, p[1], Role::Forager, duties::FORAGE_CREDIT * 12);
    duties::daily(&mut w);
    assert!(
        w.agents[p[0]].duties.held.contains(&Role::Healer),
        "a healer though they haul far more"
    );
    assert!(w.agents[p[1]].duties.held.contains(&Role::Forager));
    // Gathering takes nobody off other work; a healer is keener to tend.
    let held = w.agents[p[1]].duties.held.clone();
    assert_eq!(held, vec![Role::Forager]);
    assert_eq!(duties::fit(&held, DesignationKind::Cook), 1.0);
    assert!(duties::fit(&[Role::Healer], DesignationKind::Tend(w.agents[p[2]].id)) > 1.0);
    for _ in 0..10 {
        duties::daily(&mut w);
    }
    assert!(!w.agents[p[0]].duties.held.contains(&Role::Healer));
    assert!(!w.agents[p[1]].duties.held.contains(&Role::Forager));
}

#[test]
fn a_child_who_forages_holds_no_duty_until_of_age() {
    let mut w = party(1582);
    w.meta.tick = 30 * TICKS_PER_YEAR;
    let i = people(&w)[0];
    w.agents[i].age_scale = 1.0;
    w.agents[i].born = (w.meta.tick - 6 * TICKS_PER_YEAR) as i64;
    let k = KINDS.iter().position(|r| *r == Role::Forager).unwrap();
    w.agents[i].duties.minutes = vec![0; KINDS.len()];
    w.agents[i].duties.minutes[k] = DUTY_MINUTES * 3;
    duties::daily(&mut w);
    assert_eq!(w.agents[i].role, None, "a 6-year-old holds no role");
    assert!(w.agents[i].duties.held.is_empty());
    assert!(!w.events.pending.iter().any(
        |e| matches!(&e.kind, EventKind::RoleTaken { agent, .. } if *agent == w.agents[i].id)
    ));
    assert!(!w
        .events
        .pending
        .iter()
        .any(|e| matches!(&e.kind, EventKind::RoleLeft { agent, .. } if *agent == w.agents[i].id)));
    // Sixteen: the minutes kept counting, so the role comes the next morning.
    w.agents[i].born = (w.meta.tick - 16 * TICKS_PER_YEAR) as i64;
    w.agents[i].duties.minutes.iter_mut().for_each(|m| *m *= 2);
    duties::daily(&mut w);
    assert_eq!(w.agents[i].role, Some(Role::Forager));
}
