//! Carry out the committed activity. Movement happens in `movement`; this system handles
//! arrival (Going → then), timed activities, and their world effects.

use crate::events::{EventKind, Severity};
use crate::items::{ItemKind, ItemPlace};
use crate::mind::knowledge::Known;
use crate::mind::{Activity, MemKind, Skill, Subject};
use crate::time::TICKS_PER_HOUR;
use crate::world_state::World;
use std::collections::BTreeMap;

/// A quarrel between two who each think this ill of the other can come to blows (#404): the
/// quarrel itself costs 0.15, so a pair already at odds.
pub const BLOWS_AT: f32 = -0.15;

/// Above this fullness a forager was gathering for the store, not for their own hunger (#415):
/// decide.rs sends people out to eat below 0.75, or 0.83 on the day of the arrival.
pub const STORE_GATHER_HUNGER: f32 = 0.85;

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    let n = w.agents.len();
    // A sleeping person checks whether their bed is shared. Index the sleepers once instead
    // of scanning every agent for every sleeper (#428 follow-up; FIXLOG 2026-11-08).
    let mut sleeping_beds = sleeping_bed_counts(w);
    for i in 0..n {
        if !w.agents[i].alive {
            continue;
        }
        w.agents[i].activity_ticks += 1;
        if w.agents[i].species != crate::agents::Species::Person {
            // Animals: a finished walk is simply over.
            if let Activity::Wandering { .. } = &w.agents[i].activity {
                if w.agents[i].path.is_empty() && !w.agents[i].is_moving() {
                    w.agents[i].set_activity(Activity::Idle);
                }
            }
            continue;
        }
        let act = w.agents[i].activity.clone();
        match act {
            Activity::Idle => {}
            Activity::Working { .. } => {
                crate::systems::work::run(w, i);
            }
            Activity::Going { to, then } => {
                let a = &w.agents[i];
                if a.path_blocked {
                    crate::systems::hazards::path_failed(w, i);
                    fail(w, i, "path blocked");
                } else if a.path.is_empty() && !a.is_moving() {
                    if a.pos == to || a.pos.dist(to) <= 1 {
                        let a = &mut w.agents[i];
                        a.activity = *then;
                        a.activity_ticks = 0;
                    } else {
                        crate::systems::hazards::path_failed(w, i);
                        fail(w, i, "did not arrive");
                    }
                }
            }
            Activity::Wandering { .. } => {
                let a = &w.agents[i];
                if a.path_blocked || (a.path.is_empty() && !a.is_moving()) {
                    finish(w, i, 20);
                }
            }
            Activity::Drinking { ticks_left, back } => {
                if ticks_left <= 1 {
                    if crate::water::drink(w, i) {
                        // Back to where they were, the work or the company they left for it
                        // (#433: one who walked off to the water stayed there).
                        let here = w.agents[i].pos;
                        match back.filter(|b| b.dist(here) > 3 && w.grid.walkable(*b)) {
                            Some(b) if w.path_agent_to(i, b) => {
                                let a = &mut w.agents[i];
                                a.activity = Activity::Going {
                                    to: b,
                                    then: Box::new(Activity::Idle),
                                };
                                a.activity_ticks = 0;
                            }
                            _ => finish(w, i, 10),
                        }
                    } else {
                        // FIXLOG 2026-11-03 #608: a stand where no drink was had is not
                        // offered to them again for a while (water.rs `dry_key`), so they go
                        // to other water instead of standing there thirsty until they die.
                        let here = w.agents[i].pos;
                        let until = w.meta.tick + crate::water::DRY_STAND_TICKS;
                        w.agents[i]
                            .work_cooldowns
                            .insert(crate::water::dry_key(here), until);
                        fail(w, i, "no water here");
                    }
                } else if let Activity::Drinking { ticks_left, .. } = &mut w.agents[i].activity {
                    *ticks_left -= 1;
                }
            }
            Activity::Loitering { ticks_left } => {
                if ticks_left <= 1 {
                    finish(w, i, 0);
                } else if let Activity::Loitering { ticks_left } = &mut w.agents[i].activity {
                    *ticks_left -= 1;
                }
            }
            Activity::Eating { item, ticks_left } => {
                let Some(ix) = w.items.iter().position(|it| it.id == item) else {
                    let a = &mut w.agents[i];
                    a.knowledge.forget(Known::Item(item));
                    fail(w, i, "food gone");
                    continue;
                };
                let here = w.agents[i].pos;
                let near = match w.items[ix].place {
                    ItemPlace::Ground(p) => p.dist(here) <= 1,
                    ItemPlace::Carried(id) | ItemPlace::Held(id) => id == w.agents[i].id,
                };
                if !near {
                    w.agents[i].knowledge.doubt(Known::Item(item));
                    fail(w, i, "food moved");
                    continue;
                }
                if ticks_left <= 1 {
                    let kind = w.items[ix].kind;
                    let quality = w.items[ix].quality;
                    w.consume_item(item, 1, "eat");
                    if !w.items.iter().any(|it| it.id == item) {
                        w.agents[i].knowledge.forget(Known::Item(item));
                    }
                    let a = &mut w.agents[i];
                    // What a food is worth to them depends on what they are (P4).
                    let gain = kind.nutrition_hours() / crate::mind::needs::HUNGER_HOURS
                        * a.species_def().diet_factor(kind);
                    a.needs.hunger = (a.needs.hunger + gain).min(1.0);
                    a.needs.comfort = (a.needs.comfort + quality * 0.1).min(1.0);
                    a.last_meal = Some(tick);
                    let ev = EventKind::Ate {
                        agent: a.id,
                        item: Some(item),
                        quality,
                    };
                    // A meal is not history: the systems that care see it this tick (thoughts,
                    // memory), but it is not kept (it was 42% of a 30-year chronicle).
                    let eid = w.events.push(tick, Some(here), Severity::Trace, ev);
                    // How the meal felt: its quality, and relief if they were very hungry.
                    let valence = ((quality - 0.5) * 1.6 + if gain > 0.5 { 0.2 } else { 0.0 })
                        .clamp(-1.0, 1.0);
                    let a = &mut w.agents[i];
                    let salience = 0.3 + (1.0 - a.needs.hunger).min(0.5);
                    a.remember(
                        tick,
                        MemKind::Meal,
                        Subject::Food(kind),
                        valence,
                        salience,
                        Some(eid),
                    );
                    finish(w, i, 10);
                } else if let Activity::Eating { ticks_left, .. } = &mut w.agents[i].activity {
                    *ticks_left -= 1;
                }
            }
            Activity::Foraging { plant, ticks_left } => {
                let Some(px) = w
                    .plants
                    .iter()
                    .position(|p| p.pos == plant && p.alive && p.fruit > 0.3)
                else {
                    w.agents[i].knowledge.forget(Known::FoodPlant(plant));
                    fail(w, i, "nothing to gather");
                    continue;
                };
                if ticks_left <= 1 {
                    let kind = w.plants[px].kind;
                    w.plants[px].fruit = 0.0;
                    // A look-alike (#286): taken for the food it resembles, and some eaten on
                    // the spot.
                    if let Some((_, harm)) = kind.look_alike() {
                        ate_look_alike(w, i, plant, kind, harm);
                        continue;
                    }
                    // What the plant gives, from its row of data (plants.rs `lore`, #286).
                    let lore = kind.lore();
                    let (item_kind, base) = match lore.gives {
                        crate::plants::Gives::Food(k) => (k, lore.yields),
                        crate::plants::Gives::Crop(k, _) => (k, 2),
                        _ => (ItemKind::Root, 2),
                    };
                    let skill = w.agents[i].skills.level(Skill::Foraging);
                    let count = base + (skill / 5.0) as u32;
                    let me = w.agents[i].id;
                    // One who gathered for the store before they were hungry (#415, decide.rs)
                    // leaves it on the ground there for the haulers, not in their stomach.
                    let for_store = w.agents[i].needs.hunger > STORE_GATHER_HUNGER;
                    let here_tile = w.agents[i].pos;
                    let id = w.create_item(
                        item_kind,
                        count,
                        if for_store {
                            ItemPlace::Ground(here_tile)
                        } else {
                            ItemPlace::Carried(me)
                        },
                        0.5 + skill / 40.0,
                        "forage",
                    );
                    let a = &mut w.agents[i];
                    a.skills.practise(Skill::Foraging, 8.0 / 60.0);
                    a.knowledge.forget(Known::FoodPlant(plant));
                    let here = a.pos;
                    w.events.push(
                        tick,
                        Some(here),
                        Severity::Minor,
                        EventKind::Foraged {
                            agent: me,
                            at: plant,
                            item: id,
                        },
                    );
                    // A forage done counts toward gathering as a duty (#602, duties.rs).
                    crate::duties::credit(
                        w,
                        i,
                        crate::society::Role::Forager,
                        crate::duties::FORAGE_CREDIT,
                    );
                    w.agents[i].remember(
                        tick,
                        MemKind::Meal,
                        Subject::Place(plant),
                        0.3,
                        0.3,
                        None,
                    );
                    if for_store {
                        let seen = w.meta.tick;
                        w.agents[i].knowledge.note(
                            Known::Item(id),
                            here_tile,
                            seen,
                            item_kind as u16,
                        );
                        finish(w, i, 10);
                    } else {
                        // Go straight to eating what was gathered.
                        let a = &mut w.agents[i];
                        a.activity = Activity::Eating {
                            item: id,
                            ticks_left: 10 * 20,
                        };
                        a.activity_ticks = 0;
                    }
                } else if let Activity::Foraging { ticks_left, .. } = &mut w.agents[i].activity {
                    *ticks_left -= 1;
                }
            }
            Activity::Sleeping { bed, until } => {
                if let Some(bid) = bed {
                    // Someone else in the bed? (first come keeps it)
                    let other = sleeping_beds
                        .get(&bid)
                        .is_some_and(|sleepers| *sleepers > 1);
                    if other && w.agents[i].activity_ticks < 2 {
                        // Get up and re-decide (their next known free bed), not sleep on the
                        // spot beside the taken bed (#469, FIXLOG 2026-10-29).
                        remove_sleeping_bed(&mut sleeping_beds, bid);
                        fail(w, i, "the bed was taken");
                        continue;
                    }
                }
                let night =
                    crate::time::Calendar::rest_time(tick, w.agents[i].species_def().nocturnal());
                let rested = w.agents[i].needs.rest >= 0.99;
                let slept = w.agents[i].activity_ticks as u64;
                // Nobody gets up in the dark for nothing (#587, rule 7, FIXLOG 2026-11-02):
                // before this a rested sleeper rose after eight hours whatever the hour, so
                // one who lay down tired at 19:00 was up at 03:00 to loiter and eat in the
                // dark, tired again by 19:00 the next day, and the day stayed three hours
                // off the clock. At night they stay abed until it ends, unless real hunger or
                // thirst gets them up: asleep, thirst falls a quarter as fast, so a night's
                // sleep from 20:00 leaves most people below 0.3 by 04:00, which is not yet a
                // reason to get up in the dark.
                let needs_up = {
                    let n = &w.agents[i].needs;
                    n.hunger < 0.15 || n.thirst < 0.15
                };
                let done = (tick >= until && !night)
                    || (rested && slept > TICKS_PER_HOUR && (!night || needs_up))
                    || tick >= until + TICKS_PER_HOUR * 6;
                if done {
                    if let Some(bid) = bed {
                        remove_sleeping_bed(&mut sleeping_beds, bid);
                    }
                    let a = &mut w.agents[i];
                    let hours = a.activity_ticks as f32 / TICKS_PER_HOUR as f32;
                    let in_bed = bed.is_some();
                    let here_p = a.pos;
                    a.remember(
                        tick,
                        MemKind::Rest,
                        Subject::Place(here_p),
                        if in_bed { 0.4 } else { -0.3 },
                        0.25,
                        None,
                    );
                    let anxious = a.traits.has(crate::mind::Trait::Cautious);
                    let curious = a.traits.has(crate::mind::Trait::Curious);
                    let settled = a.memories.consolidate(tick, anxious, curious);
                    if settled > 0 {
                        let id = a.id;
                        w.events.push(
                            tick,
                            Some(here_p),
                            Severity::Trace,
                            EventKind::Dreamt {
                                agent: id,
                                consolidated: settled,
                            },
                        );
                    }
                    let a = &mut w.agents[i];
                    if std::env::var("HF_TRACE_SLEEP").is_ok() {
                        eprintln!(
                            "SLEEP {} tick {} until {} ticks {} rest {:.2} hunger {:.2}",
                            a.name, tick, until, a.activity_ticks, a.needs.rest, a.needs.hunger
                        );
                    }
                    a.last_sleep = tick;
                    let id = a.id;
                    let here = a.pos;
                    w.events.push(
                        tick,
                        Some(here),
                        Severity::Minor,
                        EventKind::Slept {
                            agent: id,
                            hours,
                            in_bed,
                        },
                    );
                    finish(w, i, 20);
                }
            }
            Activity::Talking { with, ticks_left } => {
                let other = w.agents.iter().position(|o| o.id == with);
                let near = other
                    .map(|j| {
                        w.agents[j].pos.dist(w.agents[i].pos) <= 2
                            && !w.agents[j].activity.is_asleep()
                    })
                    .unwrap_or(false);
                if !near {
                    fail(w, i, "they left");
                    continue;
                }
                let j = other.unwrap();
                // Face each other; the partner joins if free.
                let (pi, pj) = (w.agents[i].pos, w.agents[j].pos);
                w.agents[i].facing = crate::agents::Agent::facing_toward(pi, pj);
                if matches!(
                    w.agents[j].activity,
                    Activity::Idle | Activity::Loitering { .. } | Activity::Wandering { .. }
                ) {
                    w.agents[j].activity = Activity::Talking {
                        with: w.agents[i].id,
                        ticks_left,
                    };
                    w.agents[j].activity_ticks = 0;
                    w.agents[j].facing = crate::agents::Agent::facing_toward(pj, pi);
                    w.agents[j].path.clear();
                }
                if ticks_left <= 1 {
                    // A talk sours into a quarrel when either side thinks little of the other
                    // or is in a foul mood; the chance is small between friends.
                    let (ai_id, aj_id) = (w.agents[i].id, w.agents[j].id);
                    let ill_will = (-w.agents[i].opinion_of(aj_id)).max(0.0)
                        + (-w.agents[j].opinion_of(ai_id)).max(0.0);
                    let foul = (-w.agents[i].mood).max(0.0) + (-w.agents[j].mood).max(0.0);
                    let mut rq = crate::rng::stream(
                        w.meta.seed,
                        &[
                            crate::rng::purpose::AGENT_SOCIAL,
                            ai_id.0 as u64,
                            aj_id.0 as u64,
                            tick,
                        ],
                    );
                    let quarrel_chance = (0.02 + ill_will * 0.35 + foul * 0.2).min(0.8);
                    if rand::Rng::gen::<f32>(&mut rq) < quarrel_chance {
                        for (x, y) in [(i, j), (j, i)] {
                            let other = w.agents[y].id;
                            let a = &mut w.agents[x];
                            a.needs.social = (a.needs.social + 0.1).min(1.0);
                            a.relate(other, -0.15, 0.05, tick);
                            a.met(other, tick);
                            a.remember(
                                tick,
                                MemKind::Company,
                                Subject::Person(other),
                                -0.7,
                                0.6,
                                None,
                            );
                            // OST-241: the Quarrel thought (added from the event below) is the
                            // lasting cost; a direct mood write lasted one needs pass.
                        }
                        let here = w.agents[i].pos;
                        let qid = w.events.push(
                            tick,
                            Some(here),
                            Severity::Notable,
                            EventKind::Quarrelled { a: ai_id, b: aj_id },
                        );
                        // Each will judge it once they see how the other really was (#568).
                        w.agents[i].conscience.quarrelled(aj_id, Some(qid), tick);
                        w.agents[j].conscience.quarrelled(ai_id, Some(qid), tick);
                        finish(w, i, 60);
                        if matches!(w.agents[j].activity, Activity::Talking { .. }) {
                            finish(w, j, 60);
                        }
                        // #404 (2026-10-27 fix-404-feuds): between two who bear each other ill
                        // will, half the time it comes to blows, started by whoever thinks the
                        // worse of the other: a fight, judged, and between two families a
                        // grievance toward a feud.
                        let (oij, oji) =
                            (w.agents[i].opinion_of(aj_id), w.agents[j].opinion_of(ai_id));
                        if oij < BLOWS_AT && oji < BLOWS_AT && rand::Rng::gen_bool(&mut rq, 0.5) {
                            let (x, y) = if oij <= oji { (i, aj_id) } else { (j, ai_id) };
                            crate::systems::strain::fight(w, x, y, Some(qid), &mut rq);
                        }
                        continue;
                    }
                    let warmth = 0.5 + w.agents[i].skills.level(Skill::Social) / 40.0;
                    for k in [i, j] {
                        let a = &mut w.agents[k];
                        a.needs.social = (a.needs.social + 0.35).min(1.0);
                        a.skills.practise(Skill::Social, 0.1);
                    }
                    let (ai, aj) = (w.agents[i].id, w.agents[j].id);
                    w.events.push(
                        tick,
                        Some(pi),
                        Severity::Minor,
                        EventKind::Talked {
                            a: ai,
                            b: aj,
                            warmth,
                        },
                    );
                    let v = (warmth - 0.45) * 2.0;
                    let v = if v > 0.0 {
                        v * w.agents[i]
                            .beliefs
                            .weight(crate::mind::beliefs::Topic::Trust, -1.0)
                    } else {
                        v
                    };
                    crate::systems::beliefs::share(w, i, j, tick);
                    crate::lore::on_talk(w, i, j);
                    crate::accounts::on_talk(w, i, j);
                    w.agents[i].remember(
                        tick,
                        MemKind::Company,
                        Subject::Person(aj),
                        v,
                        0.35 + warmth * 0.3,
                        None,
                    );
                    w.agents[j].remember(
                        tick,
                        MemKind::Company,
                        Subject::Person(ai),
                        v,
                        0.35 + warmth * 0.3,
                        None,
                    );
                    w.agents[i].relate(aj, v * 0.12, 0.08, tick);
                    w.agents[j].relate(ai, v * 0.12, 0.08, tick);
                    w.agents[i].met(aj, tick);
                    w.agents[j].met(ai, tick);
                    finish(w, i, 40);
                    if matches!(w.agents[j].activity, Activity::Talking { .. }) {
                        finish(w, j, 40);
                    }
                } else if let Activity::Talking { ticks_left, .. } = &mut w.agents[i].activity {
                    *ticks_left -= 1;
                }
            }
        }
    }
}

fn sleeping_bed_counts(w: &World) -> BTreeMap<crate::ids::StructureId, usize> {
    let mut counts = BTreeMap::new();
    for a in &w.agents {
        if let Activity::Sleeping { bed: Some(bid), .. } = a.activity {
            *counts.entry(bid).or_insert(0) += 1;
        }
    }
    counts
}

fn remove_sleeping_bed(
    counts: &mut BTreeMap<crate::ids::StructureId, usize>,
    bid: crate::ids::StructureId,
) {
    if let Some(count) = counts.get_mut(&bid) {
        *count -= 1;
        if *count == 0 {
            counts.remove(&bid);
        }
    }
}

/// `i` ate some of a look-alike plant at `at`, taking it for food (#286): they sicken, and
/// from now on they know it and leave it (`plants::knows_look_alike`; design rule 3, knowledge
/// belongs to people). The sickness is as bad as the plant is.
fn ate_look_alike(
    w: &mut World,
    i: usize,
    at: crate::ids::TilePos,
    kind: crate::plants::PlantKind,
    harm: f32,
) {
    let tick = w.meta.tick;
    let what = format!("a poisoning from {}", kind.label());
    crate::systems::health::catch_at(w, i, crate::body::IllnessKind::Poisoned, harm, &what);
    let a = &mut w.agents[i];
    a.knowledge.forget(Known::FoodPlant(at));
    if !a.shuns.contains(&kind) {
        a.shuns.push(kind);
    }
    a.remember(tick, MemKind::Meal, Subject::Place(at), -0.6, 0.6, None);
    // Those awake who saw it (within eight tiles on the level, in sight, as learning by
    // watching works) learn the plant too; others hear of it by talk (social.rs).
    let me = w.agents[i].pos;
    let grid = &w.grid;
    for b in w.agents.iter_mut() {
        if b.alive
            && b.species == crate::agents::Species::Person
            && b.pos.z == me.z
            && b.pos.dist(me) <= 8
            && !b.activity.is_asleep()
            && !b.shuns.contains(&kind)
            && crate::mind::perception::line_of_sight(grid, b.pos, me)
        {
            b.shuns.push(kind);
        }
    }
    finish(w, i, 10);
}

fn finish(w: &mut World, i: usize, idle: u32) {
    let a = &mut w.agents[i];
    a.activity = Activity::Idle;
    a.activity_ticks = 0;
    a.idle_ticks = idle;
    a.path.clear();
}

fn fail(w: &mut World, i: usize, reason: &str) {
    let a = &mut w.agents[i];
    let goal = a.activity.name().to_string();
    a.activity = Activity::Idle;
    a.activity_ticks = 0;
    a.idle_ticks = 30;
    a.path.clear();
    a.path_blocked = false;
    let id = a.id;
    let here = a.pos;
    w.events.push(
        w.meta.tick,
        Some(here),
        Severity::Trace,
        EventKind::PlanFailed {
            agent: id,
            goal,
            reason: reason.into(),
        },
    );
}

#[cfg(test)]
mod tests {
    use super::{remove_sleeping_bed, sleeping_bed_counts};
    use crate::agents::Species;
    use crate::ids::{StructureId, TilePos};
    use crate::mind::Activity;
    use crate::world::gen::GenParams;
    use crate::World;

    #[test]
    fn sleeping_bed_index_tracks_conflicts_without_an_agent_scan() {
        let mut w = World::generate(428, GenParams::small());
        let bed = StructureId(77);
        let first = w.spawn_agent(Species::Person, TilePos::new(4, 4, 1));
        let second = w.spawn_agent(Species::Person, TilePos::new(5, 4, 1));
        for id in [first, second] {
            let i = w.agents.iter().position(|a| a.id == id).unwrap();
            w.agents[i].activity = Activity::Sleeping {
                bed: Some(bed),
                until: 100,
            };
        }

        let mut counts = sleeping_bed_counts(&w);
        assert_eq!(counts.get(&bed), Some(&2));
        assert!(counts.get(&bed).is_some_and(|sleepers| *sleepers > 1));
        remove_sleeping_bed(&mut counts, bed);
        assert_eq!(counts.get(&bed), Some(&1));
        assert!(counts.get(&bed).is_none_or(|sleepers| *sleepers <= 1));
    }
}
