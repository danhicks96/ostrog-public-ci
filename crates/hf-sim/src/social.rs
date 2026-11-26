//! The social weave (#295): gossip, favours owed and repaid, and making it up.
//!
//! Talking, people pass on what they think of someone else, and the listener leans a little
//! toward it: a name travels, and a reputation is what most of the valley has come to think. A
//! good turn (tending, teaching) is owed; when the one owed goes hungry and the one who owes
//! has eaten, it is repaid with food. Two who think ill of each other, talking in good spirits,
//! now and then make it up.

use crate::events::{EventKind, Severity};
use crate::ids::AgentId;
use crate::rng::{purpose, stream};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::World;
use rand::Rng;

fn idx(w: &World, id: AgentId) -> Option<usize> {
    w.agents.iter().position(|a| a.id == id && a.alive)
}

/// Recorded family links in this world (OST-605). A birth to a valley-born parent is a
/// third-or-later-generation birth, including children and ancestors who subsequently died.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LineageStats {
    pub living_with_living_grandparent: u32,
    pub births_to_valley_born_parent: u32,
}

/// Read the saved parent links without changing anyone. Only `give_birth` writes these links;
/// founders and incoming strangers have none. Missing parents are unknown, not inferred from
/// age, name, home or an opinion. Each grandchild counts once even with several grandparents.
pub fn lineage_stats(w: &World) -> LineageStats {
    let people: std::collections::BTreeMap<_, _> = w
        .agents
        .iter()
        .filter(|a| a.species == crate::agents::Species::Person)
        .map(|a| (a.id, a))
        .collect();
    let mut stats = LineageStats::default();
    for person in people.values() {
        let parents: Vec<_> = [person.mother, person.father]
            .into_iter()
            .flatten()
            .filter_map(|id| people.get(&id).copied())
            .collect();
        if parents
            .iter()
            .any(|p| p.mother.is_some() || p.father.is_some())
        {
            stats.births_to_valley_born_parent += 1;
        }
        if person.alive
            && parents.iter().any(|p| {
                [p.mother, p.father]
                    .into_iter()
                    .flatten()
                    .any(|id| people.get(&id).is_some_and(|g| g.alive))
            })
        {
            stats.living_with_living_grandparent += 1;
        }
    }
    stats
}

/// What a community thinks of `of`: the mean opinion of those who know them.
pub fn reputation(w: &World, of: AgentId) -> f32 {
    let (mut sum, mut n) = (0.0, 0.0);
    for a in w.agents.iter().filter(|a| a.alive && a.id != of) {
        if let Some(r) = a.relationships.get(&of) {
            sum += r.opinion;
            n += 1.0;
        }
    }
    if n > 0.0 {
        sum / n
    } else {
        0.0
    }
}

pub fn observe(w: &mut World) {
    let tick = w.meta.tick;
    let evs: Vec<EventKind> = w.last_events.iter().map(|e| e.kind.clone()).collect();
    for k in evs {
        match k {
            EventKind::Talked { a, b, warmth } => {
                gossip(w, a, b);
                gossip(w, b, a);
                tell_suspicion(w, a, b);
                tell_suspicion(w, b, a);
                tell_look_alike(w, a, b);
                tell_look_alike(w, b, a);
                tell_how(w, a, b);
                tell_how(w, b, a);
                make_up(w, a, b, warmth);
            }
            EventKind::Tended { healer, patient } => owe(w, patient, healer, tick),
            EventKind::Taught { teacher, pupil, .. } => owe(w, pupil, teacher, tick),
            EventKind::Quarrelled { a, b } | EventKind::Fought { a, b } => {
                kin_take_sides(w, a, b, tick)
            }
            _ => {}
        }
    }
}

/// #1806: a brother who watches his sister quarrel thinks no worse of her. When `a` and `b`
/// quarrel or fight and are not already each other's kin (feuds.rs `kin`: a family's own
/// quarrel takes no side), each living kin of either one, other than the two themselves, who
/// is awake within ten tiles on the same level (the same reach as justice.rs's `witnessed`)
/// takes a little against the other side: this shows before the feud machinery does
/// (feuds.rs, justice.rs `grievance`). At most once per (kin, other side) per day, as
/// `tell`'s cooldown above keys its line.
fn kin_take_sides(w: &mut World, a: AgentId, b: AgentId, tick: u64) {
    if crate::feuds::kin(w, a).contains(&b) {
        return;
    }
    for (side, other) in [(a, b), (b, a)] {
        let Some(si) = idx(w, side) else { continue };
        let (at, z) = (w.agents[si].pos, w.agents[si].pos.z);
        for k in crate::feuds::kin(w, side) {
            if k == side || k == other {
                continue;
            }
            let Some(ki) = idx(w, k) else { continue };
            let ka = &w.agents[ki];
            if ka.pos.z != z || ka.pos.dist(at) > 10 || ka.activity.is_asleep() {
                continue;
            }
            // Keyed by the other side's id (#1806), as `tell`'s 0x4870_0000 key above is kept
            // per teller: one line/change a day per kin toward the other side is enough.
            let key = 0x1806_0000 | (other.0 & 0x0000_ffff);
            if w.agents[ki]
                .work_cooldowns
                .get(&key)
                .is_some_and(|t| *t > tick)
            {
                continue;
            }
            w.agents[ki]
                .work_cooldowns
                .insert(key, tick + TICKS_PER_DAY);
            w.agents[ki].relate(other, -0.1, 0.02, tick);
        }
    }
}

fn owe(w: &mut World, who: AgentId, to: AgentId, tick: u64) {
    if let Some(i) = idx(w, who) {
        let o = &mut w.agents[i].owes;
        if !o.iter().any(|x| x.0 == to) {
            o.push((to, tick));
            if o.len() > 6 {
                o.remove(0);
            }
        }
    }
}

/// `a` tells `b` what they think of the one they feel most strongly about.
fn gossip(w: &mut World, a: AgentId, b: AgentId) {
    let tick = w.meta.tick;
    let (Some(ai), Some(bi)) = (idx(w, a), idx(w, b)) else {
        return;
    };
    let said = w.agents[ai]
        .relationships
        .iter()
        .filter(|(x, r)| **x != b && r.opinion.abs() >= 0.4)
        .max_by(|x, y| {
            x.1.opinion
                .abs()
                .total_cmp(&y.1.opinion.abs())
                .then(y.0.cmp(x.0))
        })
        .map(|(x, r)| (*x, r.opinion));
    if let Some((of, op)) = said {
        // The listener leans toward it, more the more they think of the teller.
        let trust = (0.5 + w.agents[bi].opinion_of(a)).clamp(0.0, 1.5);
        w.agents[bi].relate(of, 0.12 * op * trust, 0.02, tick);
    }
}

/// #322 (2026-10-26 fix-322-rumour): `a` tells `b` who they suspect of letting the deep's
/// people up (societies.rs). The listener comes to suspect them too, the more the more they
/// think of the teller, and thinks the worse of them for it. The suspected are not told, and a
/// member of the society does not take up talk against another.
fn tell_suspicion(w: &mut World, a: AgentId, b: AgentId) {
    let tick = w.meta.tick;
    let (Some(ai), Some(bi)) = (idx(w, a), idx(w, b)) else {
        return;
    };
    let Some((of, sure)) = w.agents[ai]
        .suspects
        .iter()
        .copied()
        .filter(|x| x.0 != b && x.1 >= 0.3)
        .max_by(|x, y| x.1.total_cmp(&y.1).then(y.0.cmp(&x.0)))
    else {
        return;
    };
    if crate::societies::in_it_together(w, b, of) {
        return;
    }
    let trust = (0.5 + w.agents[bi].opinion_of(a)).clamp(0.0, 1.5);
    let had = w.agents[bi]
        .suspects
        .iter()
        .find(|x| x.0 == of)
        .map_or(0.0, |x| x.1);
    crate::societies::suspect(w, b, of, had + 0.4 * sure * trust);
    w.agents[bi].relate(of, -0.15 * sure * trust, 0.02, tick);
}

/// #286: `a` warns `b` of one look-alike plant `a` knows and `b` does not (herb lore, a
/// sickness, seeing one, or being told). Knowledge goes person to person (design rule 3), a
/// plant a talk. Since #487 it is offered only if `a` is willing and taken only if `b` believes
/// them (mind/telling.rs), and the telling is kept with its teller either way.
fn tell_look_alike(w: &mut World, a: AgentId, b: AgentId) {
    let (Some(ai), Some(bi)) = (idx(w, a), idx(w, b)) else {
        return;
    };
    let now = w.meta.tick;
    let told = crate::plants::PlantKind::ALL.iter().copied().find(|k| {
        k.look_alike().is_some()
            && crate::plants::knows_look_alike(&w.agents[ai], *k)
            && !crate::plants::knows_look_alike(&w.agents[bi], *k)
            && !crate::mind::telling::refused_lately(
                &w.agents[bi].told,
                crate::mind::telling::What::LookAlike(*k),
                a,
                now,
            )
    });
    if let Some(k) = told {
        tell(w, ai, bi, crate::mind::telling::What::LookAlike(k));
    }
}

/// The know-how a person explains in talk (#487): crisis techniques and foraging far, not the
/// bench crafts, which are taught at the bench (crafts.rs).
pub const TOLD_HOW: [crate::mind::Technique; 5] = [
    crate::mind::Technique::BeatFlames,
    crate::mind::Technique::CarryWater,
    crate::mind::Technique::Poultice,
    crate::mind::Technique::ForageFar,
    crate::mind::Technique::Shoring,
];

/// #487: `a` explains to `b` one technique `a` knows well enough and `b` does not, now and then
/// (a quarter of talks that could carry one), if willing; `b` takes it if they believe `a`.
fn tell_how(w: &mut World, a: AgentId, b: AgentId) {
    use crate::mind::telling::{refused_lately, What, EXPLAIN_SKILL};
    let (Some(ai), Some(bi)) = (idx(w, a), idx(w, b)) else {
        return;
    };
    let now = w.meta.tick;
    let Some(t) = TOLD_HOW.iter().copied().find(|t| {
        w.agents[ai].techniques.skill(*t) >= EXPLAIN_SKILL
            && !w.agents[bi].techniques.knows(*t)
            && !refused_lately(&w.agents[bi].told, What::Technique(*t), a, now)
    }) else {
        return;
    };
    let mut r = stream(
        w.meta.seed,
        &[purpose::AGENT_SOCIAL, 0x487A, a.0 as u64, b.0 as u64, now],
    );
    if !r.gen_bool(0.25) {
        return;
    }
    tell(w, ai, bi, What::Technique(t));
}

/// One telling from `ai` to `bi` (#487): withheld, refused, misheard or taken, and kept.
fn tell(w: &mut World, ai: usize, bi: usize, what: crate::mind::telling::What) {
    use crate::mind::telling::{remember, trust, willing, Told, Took, What, BELIEVE};
    let now = w.meta.tick;
    let (a, b) = (w.agents[ai].id, w.agents[bi].id);
    let feud = crate::feuds::across(w, a, b);
    if !willing(&w.agents[ai], b, feud) {
        // Said once a day per teller, as the other choice lines are.
        let key = 0x4870_0000;
        if w.agents[ai]
            .work_cooldowns
            .get(&key)
            .is_none_or(|t| *t <= now)
        {
            w.agents[ai].work_cooldowns.insert(key, now + TICKS_PER_DAY);
            let (name, at) = (w.agents[bi].name.clone(), w.agents[ai].pos);
            let because = if feud {
                format!("{name}'s family is at feud with theirs")
            } else {
                format!("they think ill of {name}")
            };
            w.events.push(
                now,
                Some(at),
                Severity::Minor,
                EventKind::Chose {
                    agent: a,
                    what: format!("kept {} to themselves", what.words()),
                    because,
                },
            );
        }
        return;
    }
    let trust = trust(&w.agents[bi], a);
    let took = if trust < BELIEVE {
        Took::Refused
    } else if matches!(what, What::Technique(_)) && {
        let mut r = stream(
            w.meta.seed,
            &[purpose::AGENT_SOCIAL, 0x487B, a.0 as u64, b.0 as u64, now],
        );
        r.gen::<f32>() < crate::mind::telling::MISHEAR
    } {
        Took::Misheard
    } else {
        Took::Taken
    };
    let teller = w.agents[ai].name.clone();
    let l = &mut w.agents[bi];
    match (what, took) {
        (_, Took::Refused) => {}
        (What::LookAlike(k), _) => l.shuns.push(k),
        (What::Technique(t), Took::Misheard) => {
            l.techniques.learn_from(
                t,
                crate::mind::techniques::Learned::Taught,
                crate::mind::telling::MISHEARD_SKILL,
                now,
                &teller,
            );
        }
        (What::Technique(t), Took::Taken) => {
            l.techniques.learn_from(
                t,
                crate::mind::techniques::Learned::Taught,
                crate::mind::telling::TOLD_SKILL,
                now,
                &teller,
            );
        }
    }
    remember(
        &mut l.told,
        Told {
            what,
            from: a,
            tick: now,
            trust,
            took,
        },
    );
}

fn make_up(w: &mut World, a: AgentId, b: AgentId, warmth: f32) {
    let tick = w.meta.tick;
    let (Some(ai), Some(bi)) = (idx(w, a), idx(w, b)) else {
        return;
    };
    let at_odds = w.agents[ai].opinion_of(b) < -0.3 && w.agents[bi].opinion_of(a) < -0.3;
    let spirits = w.agents[ai].mood > 0.3 && w.agents[bi].mood > 0.3;
    if !(at_odds && spirits && warmth > 0.0) {
        return;
    }
    let mut r = stream(
        w.meta.seed,
        &[purpose::AGENT_SOCIAL, 0x295A, a.0 as u64, b.0 as u64, tick],
    );
    if !r.gen_bool(0.2) {
        return;
    }
    w.agents[ai].relate(b, 0.45, 0.1, tick);
    w.agents[bi].relate(a, 0.45, 0.1, tick);
    w.agents[ai].met(b, tick);
    w.agents[bi].met(a, tick);
    let at = w.agents[ai].pos;
    w.events.push(
        tick,
        Some(at),
        Severity::Notable,
        EventKind::MadeUp { a, b },
    );
}

/// Once a day: a good turn owed is repaid when the one owed goes hungry, with a real meal,
/// not the two hunger numbers this used to move, and not across a feud or a camp (carrying it
/// the rest of the way is a separate piece of work).
pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_DAY != 12 * TICKS_PER_HOUR + 41 {
        return;
    }
    for i in 0..w.agents.len() {
        if !w.agents[i].alive || w.agents[i].needs.hunger < 0.6 {
            continue;
        }
        let me = w.agents[i].pos;
        let by = w.agents[i].id;
        let owed: Vec<AgentId> = w.agents[i].owes.iter().map(|x| x.0).collect();
        for to in owed {
            let Some(j) = idx(w, to) else { continue };
            if w.agents[j].needs.hunger >= 0.3 || w.agents[j].pos.dist(me) > 1 {
                continue;
            }
            if crate::feuds::across(w, by, to) || w.agents[i].opinion_of(to) < -0.3 {
                continue;
            }
            // A food item the repayer carries themselves, moved whole or split as hauling
            // does (`systems/work.rs` `Step::PickUp`). Food lying on the ground, such as a
            // store's, is not theirs to hand over.
            let Some(ix) = w.items.iter().position(|it| {
                it.kind.is_food()
                    && match it.place {
                        crate::items::ItemPlace::Carried(id) => id == by,
                        _ => false,
                    }
            }) else {
                continue;
            };
            let moved = if w.items[ix].count > 1 {
                w.items[ix].count -= 1;
                let (kind, quality, material, rot) = (
                    w.items[ix].kind,
                    w.items[ix].quality,
                    w.items[ix].material,
                    w.items[ix].rot,
                );
                let nid = w.alloc_item_id();
                w.items.push(crate::items::Item {
                    id: nid,
                    kind,
                    count: 1,
                    place: crate::items::ItemPlace::Carried(to),
                    quality,
                    material,
                    story: None,
                    rot,
                });
                nid
            } else {
                w.items[ix].place = crate::items::ItemPlace::Carried(to);
                w.items[ix].id
            };
            w.agents[j]
                .knowledge
                .note(crate::mind::knowledge::Known::Item(moved), me, tick, 0);
            w.agents[i].owes.retain(|x| x.0 != to);
            w.agents[j].relate(by, 0.2, 0.05, tick);
            w.events.push(
                tick,
                Some(me),
                Severity::Notable,
                EventKind::Repaid { by, to },
            );
            break;
        }
    }
}
