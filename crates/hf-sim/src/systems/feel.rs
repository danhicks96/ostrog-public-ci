//! Appraisal (docs/design-feelings.md, step 1): each event of the last tick is felt by those it
//! touches, by kind and by their temperament; every hour feelings fade, and the strong ones
//! are carried as thoughts, so mood and stress read them.

use crate::events::EventKind;
use crate::ids::{AgentId, TilePos};
use crate::mind::feelings::{gain, FeelingKind as F};
use crate::mind::ThoughtKind;
use crate::time::TICKS_PER_HOUR;
use crate::world_state::World;

fn idx(w: &World, id: AgentId) -> Option<usize> {
    w.agents.iter().position(|a| a.id == id && a.alive)
}

/// Those of the valley awake within `d` of `at`.
fn near(w: &World, at: TilePos, d: i32) -> Vec<usize> {
    (0..w.agents.len())
        .filter(|&k| {
            let a = &w.agents[k];
            a.alive
                && a.species == crate::agents::Species::Person
                && a.community.is_some()
                && a.pos.z == at.z
                && a.pos.dist(at) <= d
                && !a.activity.is_asleep()
        })
        .collect()
}

/// Those of the valley awake within `d` of `at` who could actually see it happen (#856: fear
/// from a death, an attack or a fight needs sight, not only radius — a frightened neighbour
/// behind a wall who saw nothing is how fear spreads without a cause the watcher can see).
fn seen_near(w: &World, at: TilePos, d: i32) -> Vec<usize> {
    near(w, at, d)
        .into_iter()
        .filter(|&k| crate::mind::perception::can_see(&w.grid, w.agents[k].pos, at))
        .collect()
}

fn feel(w: &mut World, i: usize, k: F, about: Option<AgentId>, s: f32, src: crate::ids::EventId) {
    let tick = w.meta.tick;
    let g = gain(&w.agents[i], k);
    w.agents[i].feelings.feel(k, about, s * g, tick, Some(src));
}

pub fn observe(w: &mut World) {
    let evs: Vec<crate::events::Event> = w.last_events.clone();
    for e in &evs {
        let who = e.participants();
        let at = e.at;
        match &e.kind {
            EventKind::Died { agent, .. } => {
                // Grief in those who held them dear; a little fear in those who saw.
                let dead = *agent;
                let mourners: Vec<(usize, f32)> = (0..w.agents.len())
                    .filter(|&k| w.agents[k].alive && w.agents[k].id != dead)
                    .map(|k| (k, w.agents[k].opinion_of(dead)))
                    .filter(|(_, o)| *o > 0.3)
                    .collect();
                for (k, o) in mourners {
                    feel(w, k, F::Grief, Some(dead), 0.3 + 0.5 * o, e.id);
                }
                if let Some(p) = at {
                    for k in seen_near(w, p, 8) {
                        feel(w, k, F::Fear, None, 0.15, e.id);
                    }
                }
            }
            EventKind::Attacked { animal, agent, .. } => {
                if let Some(i) = idx(w, *agent) {
                    feel(w, i, F::Fear, Some(*animal), 0.7, e.id);
                }
                if let Some(p) = at {
                    for k in seen_near(w, p, 8) {
                        feel(w, k, F::Fear, Some(*animal), 0.35, e.id);
                    }
                }
            }
            EventKind::Fought { a, b } => {
                let stranger = |x: AgentId| {
                    w.agents
                        .iter()
                        .find(|o| o.id == x)
                        .is_some_and(|o| o.band.is_some())
                };
                let foe = if stranger(*b) {
                    Some(*b)
                } else if stranger(*a) {
                    Some(*a)
                } else {
                    None
                };
                if let (Some(f), Some(p)) = (foe, at) {
                    for k in seen_near(w, p, 10) {
                        // The bold are angered, the rest afraid.
                        let bold = w.agents[k]
                            .beliefs
                            .lean(crate::mind::beliefs::Topic::Violence)
                            > 0.2;
                        feel(
                            w,
                            k,
                            if bold { F::Anger } else { F::Fear },
                            Some(f),
                            0.35,
                            e.id,
                        );
                    }
                }
            }
            EventKind::Quarrelled { a, b } => {
                if let Some(i) = idx(w, *a) {
                    feel(w, i, F::Anger, Some(*b), 0.4, e.id);
                }
                if let Some(j) = idx(w, *b) {
                    feel(w, j, F::Anger, Some(*a), 0.4, e.id);
                }
            }
            EventKind::Broke { agent, .. } => {
                if let Some(i) = idx(w, *agent) {
                    feel(w, i, F::Shame, None, 0.5, e.id);
                }
            }
            EventKind::FireStarted { .. } => {
                if let Some(p) = at {
                    for k in near(w, p, 12) {
                        feel(w, k, F::Fear, None, 0.4, e.id);
                    }
                }
            }
            _ => {
                let kn = e.kind_name();
                let (k, s, d) = match kn {
                    "born" | "joined" | "recovered" | "returned" | "came_back" | "paired" => {
                        (F::Joy, 0.35, 12)
                    }
                    "raided" | "strangers_sighted" | "besieged" | "captured" | "rose" => {
                        (F::Fear, 0.3, 20)
                    }
                    "breached" | "cavern_found" | "shifted" => (F::Awe, 0.35, 12),
                    _ => continue,
                };
                // Those it is about feel it most; those near, some.
                let them: Vec<usize> = who.iter().filter_map(|x| idx(w, *x)).collect();
                for p in them {
                    feel(w, p, k, None, s * 1.5, e.id);
                }
                if let Some(p) = at {
                    for n in near(w, p, d) {
                        feel(w, n, k, None, s, e.id);
                    }
                }
            }
        }
    }
}

/// Step 2 (docs/design-feelings.md section 4): each hour, someone awake within six tiles who
/// thinks well of a person (above 0.2) takes a fifth of that person's strongest feeling, about
/// the same thing, by their own temperament.
fn contagion(w: &mut World) {
    let tick = w.meta.tick;
    let mut catch: Vec<(usize, F, Option<AgentId>, f32)> = Vec::new();
    for i in 0..w.agents.len() {
        let a = &w.agents[i];
        if !a.alive || a.activity.is_asleep() || a.community.is_none() {
            continue;
        }
        let Some(f) = a.feelings.strongest().copied() else {
            continue;
        };
        if f.strength < 0.3 {
            continue;
        }
        for (k, o) in w.agents.iter().enumerate() {
            if k == i
                || !o.alive
                || o.activity.is_asleep()
                || o.community != a.community
                || o.pos.z != a.pos.z
                || o.pos.dist(a.pos) > 6
                || o.opinion_of(a.id) <= 0.2
            {
                continue;
            }
            catch.push((k, f.kind, f.about, f.strength * 0.2));
        }
    }
    for (k, kind, about, s) in catch {
        // Never more than the one it came from: s is a fifth of theirs, so a listener tops out
        // at what they were given, not above it.
        let g = gain(&w.agents[k], kind);
        let cap = s * 5.0;
        if w.agents[k].feelings.of(kind) >= cap * 0.5 {
            continue;
        }
        w.agents[k].feelings.feel(kind, about, s * g, tick, None);
    }
}

/// Every hour: feelings pass between those who care for each other, fade, and the strong ones
/// are carried as thoughts.
pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_HOUR != 450 {
        return;
    }
    contagion(w);
    for a in w.agents.iter_mut().filter(|a| a.alive) {
        a.feelings.fade_hour();
        // A loss that stays (OST-215): the open bereavements weigh as one Mourning thought,
        // refreshed hourly; a loss under the floor is dropped.
        if !a.bereaved.is_empty() {
            a.bereaved
                .retain(|b| b.weight(tick).abs() >= crate::mind::bereavement::FLOOR);
            let m = crate::mind::bereavement::total(&a.bereaved, tick);
            if m < 0.0 {
                a.thoughts
                    .add_shaped(ThoughtKind::Mourning, m, 1.0 / 12.0, tick);
            }
        }
        for (k, t) in [
            (F::Fear, ThoughtKind::Afraid),
            (F::Grief, ThoughtKind::Grieving),
            (F::Anger, ThoughtKind::Angry),
            (F::Joy, ThoughtKind::Glad),
            (F::Awe, ThoughtKind::Awed),
            (F::Shame, ThoughtKind::Ashamed),
        ] {
            // Only a strong feeling is carried as a thought (round 7 soaks: from 0.3, with fear
            // passing between friends, breakdowns rose twentyfold).
            if a.feelings.of(k) >= 0.5 {
                a.thoughts.add(t, tick, None);
            }
        }
    }
}
