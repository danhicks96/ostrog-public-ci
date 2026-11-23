//! Needs decay. Striped: each agent updates once every 20 ticks (one game-minute).

use crate::events::{EventKind, Severity};
use crate::world_state::World;

pub const STRIPE: u32 = 20;

/// A grazing animal burns food at half a waking person's rate for its size: it does no work,
/// and grazes through the day and the night (#422).
pub const ANIMAL_BURN: f32 = 0.5;

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    let stripe = (tick % STRIPE as u64) as usize;
    let mut events = Vec::new();
    // Heat dries a body faster (#433): half as fast again above 25 C, twice above 30.
    // Reckoned where the person stands (#268): the heights run hotter than the valley floor.
    let valley = w.weather.temperature_c;
    let base = w
        .grid
        .ground_z(w.meta.founding_site.x, w.meta.founding_site.y);
    for (i, a) in w.agents.iter_mut().enumerate() {
        if !a.alive || i % STRIPE as usize != stripe {
            continue;
        }
        let heat = if crate::thermal::lapse_c(valley, base, a.pos.z) > 30.0 {
            2.0
        } else if crate::thermal::lapse_c(valley, base, a.pos.z) > 25.0 {
            1.5
        } else {
            1.0
        };
        if a.species != crate::agents::Species::Person {
            // A grazing animal gets hungry by its size (#422): a grown deer at about six tenths
            // of a waking person's rate, a hare at a twentieth. It eats at the plants
            // (systems/animals.rs `graze_bite`).
            // The beasts whose bodies are not skeletons feed too (#423).
            let feeds =
                crate::size::grazes(a.species) || crate::plan::beast_plan(a.species).is_some();
            if feeds && !a.denned {
                let rate = ANIMAL_BURN * crate::genetics::hunger_scale(a.body_kg(tick));
                a.needs.burn(STRIPE, rate);
            }
            continue;
        }
        let was_starving = a.needs.starving();
        // Thirst (#433): hard work and a walk dry a body as heat does.
        let toil = match a.activity {
            crate::mind::Activity::Working { .. } => 1.25,
            _ if a.is_moving() => 1.1,
            _ => 1.0,
        };
        a.needs.dry(STRIPE, heat * toil, a.activity.is_asleep());
        let was_exhausted = a.needs.exhausted();
        if a.activity.is_asleep() {
            let in_bed = matches!(
                a.activity,
                crate::mind::Activity::Sleeping { bed: Some(_), .. }
            );
            a.needs.sleep(STRIPE, in_bed, a.hunger_scale());
        } else {
            let work = if a.is_moving() { 1.1 } else { 1.0 };
            // A bigger body gets hungry faster (genetics.rs, P1).
            a.needs
                .decay_awake(STRIPE, a.traits.hunger_rate() * work * a.hunger_scale());
            // Sociable people get lonely faster, solitary people slower (#947: `.max(0.0)`
            // threw away every rate below 1, so only the sociable half of people differed).
            let extra = (a.social_rate() - 1.0) * STRIPE as f32
                / crate::time::TICKS_PER_HOUR as f32
                / crate::mind::needs::SOCIAL_HOURS;
            a.needs.social = (a.needs.social - extra).clamp(0.0, 1.0);
        }
        // Starving means empty with no meal in progress; a last bite on the way down is not
        // news, and neither is the walk to a bite that is in sight (FIXLOG 2026-10-29 #469
        // correction: an exhausted walker woke at 5% beside a chronically picked-over bush and
        // was flagged while crossing to the fruiting one he had chosen).
        let feeding = matches!(
            a.activity,
            crate::mind::Activity::Eating { .. } | crate::mind::Activity::Foraging { .. }
        ) || matches!(
            &a.activity,
            crate::mind::Activity::Going { then, .. }
                if matches!(
                    **then,
                    crate::mind::Activity::Eating { .. } | crate::mind::Activity::Foraging { .. }
                )
        );
        if a.needs.starving() && !was_starving && !feeding {
            if std::env::var("HF_TRACE_STARVE").is_ok() {
                eprintln!(
                    "STARVE {} tick {} hour {} act {} ticks {} known {} idle {}",
                    a.name,
                    tick,
                    (tick % 28800) / 1200,
                    a.activity.name(),
                    a.activity_ticks,
                    a.knowledge.map.len(),
                    a.idle_ticks
                );
            }
            events.push((a.pos, EventKind::Starving { agent: a.id }));
        }
        if a.needs.exhausted() && !was_exhausted {
            events.push((a.pos, EventKind::Exhausted { agent: a.id }));
        }
    }
    for (pos, k) in events {
        let eid = w.events.push(tick, Some(pos), Severity::Major, k.clone());
        if let EventKind::Starving { agent } | EventKind::Exhausted { agent } = k {
            if let Some(a) = w.agents.iter_mut().find(|a| a.id == agent) {
                a.remember(
                    tick,
                    crate::mind::MemKind::Hardship,
                    crate::mind::Subject::Place(pos),
                    -0.8,
                    0.8,
                    Some(eid),
                );
            }
        }
    }
    // Children learn by watching: a little practice in whatever a nearby working adult does.
    let watching: Vec<(usize, crate::mind::Skill)> = w
        .agents
        .iter()
        .enumerate()
        .filter(|(i, a)| a.alive && a.is_child(tick) && *i % STRIPE as usize == stripe)
        .filter_map(|(i, a)| {
            w.agents
                .iter()
                .find(|o| {
                    o.alive
                        && o.id != a.id
                        && o.pos.dist(a.pos) <= 3
                        && matches!(
                            o.activity,
                            crate::mind::Activity::Working { .. }
                                | crate::mind::Activity::Foraging { .. }
                        )
                })
                .map(|o| {
                    (
                        i,
                        match &o.activity {
                            crate::mind::Activity::Foraging { .. } => crate::mind::Skill::Foraging,
                            _ => crate::mind::Skill::Construction,
                        },
                    )
                })
        })
        .collect();
    for (i, sk) in watching {
        w.agents[i].skills.practise(sk, 0.01);
    }
    // Mood: needs plus the tone of recent memories; recomputed on the stripe.
    for (i, a) in w.agents.iter_mut().enumerate() {
        if !a.alive || i % STRIPE as usize != stripe {
            continue;
        }
        a.mood = mood_terms(a, tick).iter().sum::<f32>().clamp(-1.0, 1.0);
    }
}

/// The four terms a person's mood is the clamped sum of, in order: needs (already weighted by
/// 1.2), recent memories, long memories, held thoughts (OST-904). The mood block of `step` sums
/// these, and the alive report reads them, so the two cannot drift.
///
/// thirst and warmth never reached mood, so a person dying of thirst read as content as
/// one who had just eaten.
pub fn mood_terms(a: &crate::agents::Agent, now: u64) -> [f32; 4] {
    let needs = (a.needs.hunger
        + a.needs.rest
        + a.needs.social
        + a.needs.comfort
        + a.needs.thirst
        + a.needs.warmth)
        / 6.0
        - 0.5;
    let recent: f32 = a
        .memories
        .short
        .iter()
        .rev()
        .take(10)
        .map(|r| r.valence * r.salience)
        .sum::<f32>()
        / 4.0;
    let long: f32 = a
        .memories
        .long
        .iter()
        .map(|m| m.valence * m.strength)
        .sum::<f32>()
        / 8.0;
    let thoughts = a.thoughts.sum(now) * 0.8;
    [needs * 1.2, recent, long, thoughts]
}
