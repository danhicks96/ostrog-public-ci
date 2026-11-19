//! C77: leadership granted by deeds seen, and justice (#298, #360).

use hf_sim::events::EventKind;
use hf_sim::ids::{ItemId, TilePos};
use hf_sim::justice::{self, Domain};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::world::gen::GenParams;
use hf_sim::world::{Material, Tile};
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
    // All together, awake, adults.
    for (k, &i) in idx.iter().enumerate() {
        w.agents[i].pos = w.grid.surface(30 + k as i32, 30);
        w.agents[i].activity = hf_sim::mind::Activity::Idle;
    }
    (w, idx)
}

/// Run one event through the deed observer as if it had just happened.
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
    justice::observe(w);
}

fn at_hour(w: &mut World, day: u64, h: u64) {
    w.meta.tick = day * TICKS_PER_DAY + h * TICKS_PER_HOUR + 300;
    justice::step(w);
}

fn count(w: &World, f: impl Fn(&EventKind) -> bool) -> usize {
    w.events.pending.iter().filter(|e| f(&e.kind)).count()
}

#[test]
fn a_healer_seen_tending_is_followed_in_settling_what_is_between_people() {
    let (mut w, p) = meadow(7701, 5);
    let healer = w.agents[p[0]].id;
    for &k in &p[1..5] {
        let patient = w.agents[k].id;
        for _ in 0..2 {
            happen(&mut w, EventKind::Tended { healer, patient });
        }
    }
    at_hour(&mut w, 1, 17);
    assert_eq!(
        w.communities[0].leaders[Domain::Social as usize],
        Some(healer)
    );
    assert_eq!(
        count(
            &w,
            |k| matches!(k, EventKind::Followed { agent, .. } if *agent == healer)
        ),
        1
    );
    // Deeds not renewed fade: two months on, nobody has done anything, and nobody is followed.
    for d in 2..70 {
        at_hour(&mut w, d, 17);
    }
    assert_eq!(w.communities[0].leaders[Domain::Social as usize], None);
}

#[test]
fn a_third_fight_by_one_nobody_likes_is_exile() {
    let (mut w, p) = meadow(7702, 5);
    let (off, vic) = (p[0], p[1]);
    let who = w.agents[off].id;
    let victim = w.agents[vic].id;
    // Nobody likes them (exile is for one the judge does not like, C77 soaks).
    let tick0 = w.meta.tick;
    for &k in &p {
        if k != off {
            w.agents[k].relate(who, -0.8, 0.5, tick0);
        }
    }
    let site_grudge = |w: &World| w.overworld.sites.iter().map(|s| s.grudge).sum::<u32>();
    let before = site_grudge(&w);
    let mut verdicts = Vec::new();
    for d in 1..=3u64 {
        w.meta.tick = d * TICKS_PER_DAY + 12 * TICKS_PER_HOUR;
        let at = w.agents[off].pos;
        justice::offence(&mut w, off, Some(victim), "starting a fight", at);
        at_hour(&mut w, d, 18);
        for e in &w.events.pending {
            if let EventKind::Judged { of, verdict, .. } = &e.kind {
                if *of == who && !verdicts.contains(verdict) {
                    verdicts.push(verdict.clone());
                }
            }
        }
        w.events.pending.clear();
    }
    assert!(verdicts.contains(&"exiled".to_string()), "{verdicts:?}");
    assert_eq!(w.agents[off].community, None);
    assert!(w.agents[off].leaving);
    if w.overworld.sites.iter().any(|s| s.people > 0) {
        assert_eq!(
            site_grudge(&w),
            before + 1,
            "the place they went to holds it"
        );
    }
}

#[test]
fn an_offence_nobody_saw_can_be_laid_on_the_wrong_one_and_costs_the_judge() {
    let (mut w, p) = meadow(7703, 5);
    let (off, finder, scapegoat, friend) = (p[0], p[1], p[2], p[3]);
    // Everyone asleep: nobody sees the tantrum.
    for &i in &p {
        w.agents[i].activity = hf_sim::mind::Activity::Sleeping {
            bed: None,
            until: u64::MAX,
        };
    }
    let (sid, oid) = (w.agents[scapegoat].id, w.agents[off].id);
    let tick = w.meta.tick;
    w.agents[finder].relate(sid, -0.9, 0.5, tick);
    w.agents[finder].relate(oid, 0.9, 0.5, tick);
    w.agents[friend].relate(sid, 0.9, 0.5, tick);
    let at = w.agents[off].pos;
    // A broken stool is made a case of only some of the time: at whichever moment it is.
    for k in 0..200 {
        w.meta.tick += k;
        justice::offence(&mut w, off, None, "breaking the table", at);
        if !w.communities[0].accused.is_empty() {
            break;
        }
    }
    let acc = w.communities[0].accused.clone();
    assert_eq!(acc.len(), 1);
    assert_eq!(acc[0].of, sid, "laid on the one the finder likes least");
    assert_eq!(acc[0].truly, oid);
    for &i in &p {
        w.agents[i].activity = hf_sim::mind::Activity::Idle;
    }
    // Whoever judges has the friend's following before.
    for &k in &p {
        let to = w.agents[k].id;
        justice::credit(&mut w, friend, to, Domain::Social, 1.0);
    }
    at_hour(&mut w, 1, 18);
    let judge = w
        .events
        .pending
        .iter()
        .find_map(|e| match &e.kind {
            EventKind::Judged {
                judge,
                of,
                wrongly: true,
                ..
            } if *of == sid => Some(*judge),
            _ => None,
        })
        .expect("judged, wrongly");
    let left = w.agents[friend]
        .defers
        .iter()
        .find(|x| x.0 == judge)
        .map(|x| x.1[Domain::Social as usize])
        .unwrap_or(0.0);
    assert!(
        left < 0.6,
        "the wronged one's friend follows the judge less: {left}"
    );
}

#[test]
fn two_in_the_record_against_each_other_twice_are_in_a_feud() {
    let (mut w, p) = meadow(7704, 4);
    let (a, b) = (p[0], p[1]);
    let (ai, bi) = (w.agents[a].id, w.agents[b].id);
    for d in 1..=3u64 {
        w.meta.tick = d * TICKS_PER_DAY + 12 * TICKS_PER_HOUR;
        let (x, v) = if d % 2 == 1 { (a, bi) } else { (b, ai) };
        let at = w.agents[x].pos;
        justice::offence(&mut w, x, Some(v), "starting a fight", at);
        at_hour(&mut w, d, 18);
    }
    assert_eq!(count(&w, |k| matches!(k, EventKind::Feud { .. })), 1);
}

#[test]
fn a_liked_one_who_fights_three_times_makes_amends_and_stays() {
    let (mut w, p) = meadow(7705, 5);
    let (off, vic) = (p[0], p[1]);
    let (who, victim) = (w.agents[off].id, w.agents[vic].id);
    let tick0 = w.meta.tick;
    for &k in &p[1..] {
        w.agents[k].relate(who, 0.8, 0.5, tick0);
    }
    for d in 1..=3u64 {
        w.meta.tick = d * TICKS_PER_DAY + 12 * TICKS_PER_HOUR;
        let at = w.agents[off].pos;
        justice::offence(&mut w, off, Some(victim), "starting a fight", at);
        at_hour(&mut w, d, 18);
    }
    assert_eq!(count(&w, |k| matches!(k, EventKind::Exiled { .. })), 0);
    assert!(w.agents[off].community.is_some());
}

/// #404: 52 accusations and no exile in the M5 soaks. Setting a fire is one of the gravest
/// offences: always taken up, and exile even for one everybody likes.
#[test]
fn a_fire_set_in_a_rage_is_exile_even_for_one_who_is_liked() {
    let (mut w, p) = meadow(7706, 5);
    let off = p[0];
    let who = w.agents[off].id;
    let tick0 = w.meta.tick;
    for &k in &p[1..] {
        w.agents[k].relate(who, 0.8, 0.5, tick0);
    }
    w.meta.tick = TICKS_PER_DAY + 12 * TICKS_PER_HOUR;
    let at = w.agents[off].pos;
    justice::offence(&mut w, off, None, justice::ARSON, at);
    assert_eq!(w.communities[0].accused.len(), 1, "always taken up");
    at_hour(&mut w, 1, 18);
    assert_eq!(
        count(
            &w,
            |k| matches!(k, EventKind::Judged { of, verdict, .. } if *of == who && verdict == "exiled")
        ),
        1
    );
    assert_eq!(count(&w, |k| matches!(k, EventKind::Exiled { .. })), 1);
    assert_eq!(w.agents[off].community, None);
}

/// #1247: the seers of a deed were anyone of the community, awake, on the same level and
/// within 10 tiles — no line of sight, so a miner in the next gallery credited the cook.
/// D48 says "a person who sees another build...": a watcher behind a wall earns no deference.
#[test]
fn a_deed_behind_a_wall_earns_no_deference() {
    let (mut w, p) = meadow(7707, 3);
    let cook = w.agents[p[0]].id;
    let z = w.agents[p[0]].pos.z;
    let at = w.agents[p[0]].pos;
    // Watcher in the open, 5 tiles off: sees the deed.
    w.agents[p[1]].pos = TilePos::new(at.x + 5, at.y, z);
    // Watcher behind a wall 5 tiles off: cannot see it.
    w.agents[p[2]].pos = TilePos::new(at.x, at.y + 5, z);
    w.grid.set(
        TilePos::new(at.x, at.y + 2, z),
        Tile::solid(Material::GRANITE),
    );

    happen(
        &mut w,
        EventKind::Cooked {
            agent: cook,
            item: ItemId(0),
        },
    );

    let open_watcher = w.agents[p[1]].id;
    let walled_watcher = w.agents[p[2]].id;
    let defers_to_cook = |w: &World, who: hf_sim::ids::AgentId| {
        w.agents
            .iter()
            .find(|a| a.id == who)
            .and_then(|a| a.defers.iter().find(|x| x.0 == cook))
            .map(|x| x.1[Domain::Provision as usize])
            .unwrap_or(0.0)
    };
    assert!(
        defers_to_cook(&w, open_watcher) > 0.0,
        "a watcher in the open should credit the cook"
    );
    assert_eq!(
        defers_to_cook(&w, walled_watcher),
        0.0,
        "a watcher behind a wall should not credit the cook"
    );
}

/// #1807: a verdict against someone, other than a pardon, leaves their family cool toward
/// whoever accused them. The accused's mother's opinion of the accuser falls by about 0.1 and
/// an unrelated bystander's does not change; after a pardon the mother's does not change.
#[test]
fn kin_of_the_judged_hold_it_against_the_accuser() {
    let (mut w, p) = meadow(7707, 5);
    let (off, vic, mom, stranger) = (p[0], p[1], p[2], p[3]);
    let (who, victim) = (w.agents[off].id, w.agents[vic].id);
    let mother_id = w.agents[mom].id;
    w.agents[off].mother = Some(mother_id);
    let tick0 = w.meta.tick;
    // Nobody likes the offender, so a first offence is "to make amends", not pardoned.
    for &k in &p {
        if k != off {
            w.agents[k].relate(who, -0.8, 0.5, tick0);
        }
    }
    let before_mom = w.agents[mom].opinion_of(victim);
    let before_stranger = w.agents[stranger].opinion_of(victim);
    w.meta.tick = TICKS_PER_DAY + 12 * TICKS_PER_HOUR;
    let at = w.agents[off].pos;
    justice::offence(&mut w, off, Some(victim), "starting a fight", at);
    at_hour(&mut w, 1, 18);
    assert_eq!(
        count(
            &w,
            |k| matches!(k, EventKind::Judged { of, verdict, .. } if *of == who && verdict == "to make amends")
        ),
        1,
        "set up as a first, disliked offence"
    );
    let after_mom = w.agents[mom].opinion_of(victim);
    assert!(
        after_mom < before_mom - 0.05,
        "the mother holds the verdict against the accuser: {before_mom} -> {after_mom}"
    );
    assert!(
        (before_mom - after_mom - 0.1).abs() < 0.03,
        "about 0.1 (#1807): {before_mom} -> {after_mom}"
    );
    assert_eq!(
        w.agents[stranger].opinion_of(victim),
        before_stranger,
        "an unrelated member is unaffected"
    );

    // A pardon changes nothing.
    let (mut w2, p2) = meadow(7708, 5);
    let (off2, vic2, mom2) = (p2[0], p2[1], p2[2]);
    let (who2, victim2) = (w2.agents[off2].id, w2.agents[vic2].id);
    let mother_id2 = w2.agents[mom2].id;
    w2.agents[off2].mother = Some(mother_id2);
    // Everyone likes the offender, so a first offence is pardoned.
    let tick1 = w2.meta.tick;
    for &k in &p2 {
        if k != off2 {
            w2.agents[k].relate(who2, 0.8, 0.5, tick1);
        }
    }
    let before_mom2 = w2.agents[mom2].opinion_of(victim2);
    w2.meta.tick = TICKS_PER_DAY + 12 * TICKS_PER_HOUR;
    let at2 = w2.agents[off2].pos;
    justice::offence(&mut w2, off2, Some(victim2), "starting a fight", at2);
    at_hour(&mut w2, 1, 18);
    assert_eq!(
        count(
            &w2,
            |k| matches!(k, EventKind::Judged { of, verdict, .. } if *of == who2 && verdict == "pardoned")
        ),
        1,
        "set up as a first, liked offence"
    );
    assert_eq!(
        w2.agents[mom2].opinion_of(victim2),
        before_mom2,
        "a pardon leaves the mother's opinion unchanged"
    );
}
