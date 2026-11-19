//! #568: regret and rationalising. After a quarrel a person who later sees the other was in
//! fact hurt, sick or grieving judges what they did. Warm, cautious, communal people regret it
//! and lean toward helping the other; cold, self-reliant ones rationalise it and lean away. The
//! lean is a named, bounded, fading term in their later decisions (the Why? lens, #567).

use hf_sim::ids::{AgentId, TilePos};
use hf_sim::items::ItemKind;
use hf_sim::mind::conscience::{Stance, HELP_MAX, HELP_MIN, LEANING_DAYS, REGRET_HELP};
use hf_sim::mind::{Activity, Dispositions};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::World;

/// Two of a party on a flat meadow at ten on day 30, fed, rested and not lonely; the second
/// sick in bed-worthy measure, three tiles from the first.
fn pair(seed: u64) -> (World, AgentId, AgentId) {
    let mut w = World::generate(seed, GenParams::small());
    w.flatten(2, 2, 60, 60, 1);
    w.plants.clear();
    w.meta.founding_site = TilePos::new(20, 20, 2);
    w.meta.tick = TICKS_PER_DAY * 30 + TICKS_PER_HOUR * 10;
    w.rebuild_caches();
    let ids = w.found_party(2);
    w.items.retain(|it| !it.kind.is_food());
    let at = w.grid.surface(22, 18);
    w.drop_item(ItemKind::Meal, 60, at);
    w.ensure_nav();
    for a in w.agents.iter_mut() {
        a.needs.hunger = 0.95;
        a.needs.thirst = 0.95;
        a.needs.rest = 0.9;
        a.needs.social = 0.95;
        a.needs.warmth = 1.0;
        a.stress = 0.0;
        a.activity = Activity::Idle;
        a.idle_ticks = 0;
    }
    let (a, b) = (ids[0], ids[1]);
    let ia = idx(&w, a);
    let ib = idx(&w, b);
    let near = w.agents[ia].pos.offset(3, 0, 0);
    w.agents[ib].pos = w.grid.surface(near.x, near.y);
    w.agents[ib].from = w.agents[ib].pos;
    w.agents[ib].body.illness = Some(hf_sim::body::Illness {
        severity: 0.5,
        since: 0,
        kind: Default::default(),
        treated: None,
    });
    (w, a, b)
}

fn idx(w: &World, id: AgentId) -> usize {
    w.agents.iter().position(|a| a.id == id).unwrap()
}

fn temper(w: &mut World, who: AgentId, warm: bool) {
    let i = idx(w, who);
    let v = if warm { 0.9 } else { 0.1 };
    w.agents[i].dispositions = Some(Dispositions {
        warmth: v,
        caution: v,
        independence: 1.0 - v,
        ..Dispositions::default()
    });
}

/// The quarrel happened an hour ago; now, on the hour, the conscience step runs.
fn quarrel_and_judge(w: &mut World, a: AgentId, b: AgentId) {
    let i = idx(w, a);
    let t = w.meta.tick;
    w.agents[i]
        .conscience
        .quarrelled(b, Some(hf_sim::ids::EventId(4242)), t - TICKS_PER_HOUR);
    hf_sim::systems::conscience::step(w);
}

/// The tend option as the Why? lens recorded it, if it was weighed.
fn tend_option(w: &mut World, a: AgentId) -> Option<hf_sim::why::Considered> {
    w.trace_why(Some(a));
    hf_sim::systems::decide::step(w);
    let c = w.why.as_ref()?.latest()?.clone();
    c.options.into_iter().find(|o| o.cycle == "tend")
}

#[test]
fn the_same_outcome_leaves_a_warm_and_a_cold_person_in_different_states() {
    let mut out = Vec::new();
    for warm in [true, false] {
        let (mut w, a, b) = pair(5681);
        temper(&mut w, a, warm);
        let before = w.agents[idx(&w, a)].opinion_of(b);
        quarrel_and_judge(&mut w, a, b);
        let me = &w.agents[idx(&w, a)];
        let j = me.conscience.judgements[0];
        let (_, stance, found) = j.judged.expect("judged on seeing them sick");
        assert_eq!(
            j.cause,
            Some(hf_sim::ids::EventId(4242)),
            "linked to the quarrel"
        );
        assert_eq!(found, hf_sim::mind::conscience::Found::Sick);
        out.push((
            stance,
            me.opinion_of(b) - before,
            me.conscience.help_toward(b, w.meta.tick),
        ));
        // What changed is recorded with the judgement, not only as an event line.
        assert_eq!(j.help, me.conscience.help_toward(b, w.meta.tick));
        assert!(w.events.pending.iter().any(|e| matches!(
            &e.kind,
            hf_sim::events::EventKind::Chose { agent, .. } if *agent == a
        )));
    }
    assert_eq!(out[0].0, Stance::Regret);
    assert_eq!(out[1].0, Stance::Rationalise);
    assert!(out[0].1 > 0.0 && out[1].1 < 0.0, "{out:?}");
    assert!(out[0].2 > 1.0 && out[1].2 < 1.0, "{out:?}");
}

#[test]
fn a_healer_who_regrets_weighs_tending_the_other_more_than_a_control() {
    let run = |quarrel: Option<bool>| {
        let (mut w, a, b) = pair(5682);
        temper(&mut w, a, quarrel.unwrap_or(true));
        let i = idx(&w, a);
        // A healer is whoever's recent work makes it their duty (#454), not a handed-out role.
        w.agents[i].duties.held = vec![hf_sim::society::Role::Healer];
        let p = w.agents[i].pos.offset(-2, 0, 0);
        let p = w.grid.surface(p.x, p.y);
        w.drop_item(ItemKind::Poultice, 3, p);
        if quarrel.is_some() {
            quarrel_and_judge(&mut w, a, b);
        }
        tend_option(&mut w, a).expect("a healer weighs tending the sick")
    };
    let control = run(None);
    let regret = run(Some(true));
    let rational = run(Some(false));
    assert_eq!(control.learned, 1.0);
    assert!((regret.learned - REGRET_HELP).abs() < 1e-6, "{regret:?}");
    assert!(rational.learned < 1.0, "{rational:?}");
    // Same situation, same base: only the learned term differs.
    assert!((regret.base - control.base).abs() < 1e-6);
    assert!(regret.score > control.score && control.score > rational.score);
}

#[test]
fn one_who_regrets_goes_to_make_amends_though_neither_healer_nor_kin() {
    let run = |quarrel: bool| {
        let (mut w, a, b) = pair(5683);
        temper(&mut w, a, true);
        if quarrel {
            quarrel_and_judge(&mut w, a, b);
        }
        tend_option(&mut w, a)
    };
    assert!(run(false).is_none(), "a stranger is not theirs to tend");
    let amends = run(true).expect("regret makes the sick one theirs to tend");
    assert!(amends.learned > hf_sim::systems::decide::AMENDS);
}

#[test]
fn nothing_is_judged_without_seeing_the_other_and_the_quarrel_lapses() {
    let (mut w, a, b) = pair(5684);
    temper(&mut w, a, true);
    let ib = idx(&w, b);
    let far = w.grid.surface(55, 55);
    w.agents[ib].pos = far;
    quarrel_and_judge(&mut w, a, b);
    let i = idx(&w, a);
    assert_eq!(
        w.agents[i].conscience.pending().count(),
        1,
        "still to judge"
    );
    assert_eq!(w.agents[i].conscience.help_toward(b, w.meta.tick), 1.0);
    w.meta.tick += TICKS_PER_DAY;
    hf_sim::systems::conscience::step(&mut w);
    assert!(
        w.agents[i].conscience.judgements.is_empty(),
        "lapsed unjudged"
    );
}

#[test]
fn the_lean_fades_and_cannot_run_away() {
    let (mut w, a, b) = pair(5685);
    temper(&mut w, a, true);
    for _ in 0..12 {
        quarrel_and_judge(&mut w, a, b);
        w.meta.tick += TICKS_PER_HOUR;
    }
    let i = idx(&w, a);
    let now = w.meta.tick;
    let help = w.agents[i].conscience.help_toward(b, now);
    assert!((HELP_MIN..=HELP_MAX).contains(&help), "{help}");
    assert!(w.agents[i].conscience.judgements.len() <= hf_sim::mind::conscience::JUDGEMENT_CAP);
    let faded = w.agents[i]
        .conscience
        .help_toward(b, now + LEANING_DAYS * TICKS_PER_DAY);
    assert_eq!(faded, 1.0);
}
