//! Beliefs moving in the world (mind/beliefs.rs): talk spreads them, outcomes confirm or shake
//! them. Nothing here decides anything; it only changes what people hold, with the source.

use crate::events::EventKind;

use crate::mind::beliefs::{Source, Topic};
use crate::World;

/// After a good talk between agents `i` and `j`, each may pass on the belief they hold most
/// strongly. The listener weighs it by how much they think of the teller.
pub fn share(w: &mut World, i: usize, j: usize, tick: u64) {
    for (t, l) in [(i, j), (j, i)] {
        let Some(b) = w.agents[t].beliefs.strongest().cloned() else {
            continue;
        };
        if b.stance.abs() * b.confidence < 0.2 {
            continue;
        }
        let teller = w.agents[t].id;
        let trust = (0.5 + w.agents[l].opinion_of(teller)).clamp(0.0, 1.0);
        w.agents[l]
            .beliefs
            .hear(b.topic, b.stance, teller, trust, tick);
    }
    // E16 practical warnings follow the same conversation, but the teller still chooses whether
    // to help. A friend shares; a rival may withhold or distort the location. The listener gets a
    // person-scoped record with the teller attached rather than a colony-wide fact.
    for (t, l) in [(i, j), (j, i)] {
        let teller = w.agents[t].id;
        let listener = w.agents[l].id;
        let Some((_, problem)) = w.agents[t].problems.strongest_open() else {
            continue;
        };
        let problem = problem.clone();
        let opinion = w.agents[t].opinion_of(listener);
        let trust = (0.5 + w.agents[l].opinion_of(teller)).clamp(0.0, 1.0);
        if (-0.65..-0.2).contains(&opinion) {
            // Resentment can be passive: they want the other person to discover it the hard way.
            continue;
        }
        let misleading = opinion < -0.65;
        let pos = if misleading {
            let x = (problem.pos.x + 1).clamp(1, w.grid.width - 2);
            // FIXLOG 2026-10-29 feat/e16-social-knowledge: clamping must not collapse a
            // distorted warning back onto the true location at the right edge.
            let x = if x == problem.pos.x {
                (problem.pos.x - 1).clamp(1, w.grid.width - 2)
            } else {
                x
            };
            w.grid.surface(x, problem.pos.y.clamp(1, w.grid.height - 2))
        } else {
            problem.pos
        };
        w.agents[l].problems.hear(crate::mind::HeardProblem {
            teller,
            kind: problem.kind,
            pos,
            urgency: problem.urgency,
            trust,
            tick,
            trigger: problem.trigger,
        });
    }
    // Word of the lost passes on too, a little less exact each time it is told.
    for (t, l) in [(i, j), (j, i)] {
        let teller = w.agents[t].id;
        let news: Vec<crate::opening::Rumour> = w.agents[t]
            .heard
            .iter()
            .filter(|r| !w.agents[l].heard.iter().any(|x| x.who == r.who))
            .copied()
            .collect();
        let fam = w.agents[l]
            .relationships
            .get(&teller)
            .map(|r| r.familiarity)
            .unwrap_or(0.0);
        let blur = ((1.0 - fam) * 5.0).round() as i32;
        for (k, r) in news.into_iter().enumerate() {
            let dx = ((tick as i32 + k as i32 * 7) % (2 * blur + 1)) - blur;
            let dy = ((tick as i32 / 3 + k as i32 * 5) % (2 * blur + 1)) - blur;
            let at = w.grid.surface(
                (r.at.x + dx).clamp(1, w.grid.width - 2),
                (r.at.y + dy).clamp(1, w.grid.height - 2),
            );
            w.agents[l].hear(crate::opening::Rumour {
                who: r.who,
                at,
                heard: r.heard,
                from: Some(teller),
            });
        }
    }
    // A witnessed account travels the same way, unless its holder withholds it. The
    // listener's copy keeps the carrier and the read, blurs the place by how well they
    // know the teller, and every fifth retelling loses the read's detail (7.5, #508).
    for (t, l) in [(i, j), (j, i)] {
        let news: Vec<crate::mind::witnessed::Witnessed> = w.agents[t]
            .witnessed
            .iter()
            .filter(|x| x.tells && !w.agents[l].witnessed.iter().any(|y| y.same_account(x)))
            .copied()
            .collect();
        let fam = w.agents[l]
            .relationships
            .get(&w.agents[t].id)
            .map(|r| r.familiarity)
            .unwrap_or(0.0);
        let blur = ((1.0 - fam) * 3.0).round() as i32;
        for (k, mut x) in news.into_iter().enumerate() {
            let dx = ((tick as i32 / 5 + k as i32 * 3) % (2 * blur + 1)) - blur;
            let dy = ((tick as i32 / 7 + k as i32) % (2 * blur + 1)) - blur;
            // #778: blur x/y only — the account keeps its level. Projecting with
            // grid.surface() would move a refusal born underground onto the surface
            // above it, and anything acting on Witnessed.at would act there (FIXLOG 2026-11-13).
            x.at = crate::ids::TilePos {
                x: (x.at.x + dx).clamp(1, w.grid.width - 2),
                y: (x.at.y + dy).clamp(1, w.grid.height - 2),
                z: x.at.z,
            };
            if (tick / crate::time::TICKS_PER_DAY + k as u64).is_multiple_of(5) {
                x.read = crate::mind::witnessed::WitnessRead::UnexplainedDread;
            }
            x.tick = tick;
            x.tells = true;
            crate::mind::witnessed::remember(&mut w.agents[l].witnessed, x);
        }
    }
    // A held fragment travels the same way, unless its holder withholds it (7.6, #508).
    // Familiarity blurs which one it was; every fifth retelling loses that altogether.
    // A copy that now disagrees with one the listener already holds of the same fragment
    // is kept beside it — the record holds both, and no one arbitrates.
    for (t, l) in [(i, j), (j, i)] {
        let tid = w.agents[t].id;
        let lid = w.agents[l].id;
        let fam = w.agents[l]
            .relationships
            .get(&tid)
            .map(|r| r.familiarity)
            .unwrap_or(0.0);
        let news: Vec<crate::mind::fragment::HeldFragment> = w.agents[t]
            .fragments
            .iter()
            .filter(|x| x.tells)
            .copied()
            .collect();
        for (k, mut x) in news.into_iter().enumerate() {
            if let Some(m) = x.mark {
                if (tick / crate::time::TICKS_PER_DAY + k as u64).is_multiple_of(5) {
                    x.mark = None;
                } else {
                    let blur = ((1.0 - fam) * 2.0).round() as i32;
                    let span = crate::mind::fragment::HeldFragment::span(x.what) as i32;
                    let d = ((tick as i32 / 5 + k as i32 * 3) % (2 * blur + 1)) - blur;
                    x.mark = Some((m as i32 + d).clamp(0, span - 1) as u8);
                }
            }
            // Same fragment, same telling: nothing new. Different telling: both are kept.
            if w.agents[l]
                .fragments
                .iter()
                .any(|y| y.identity(lid) == x.identity(tid) && y.mark == x.mark)
            {
                continue;
            }
            // A copy of the woken original points at it; a copy of a copy keeps the chain.
            if x.origin.is_none() {
                x.origin = Some((tid, x.tick));
            }
            x.learned = crate::mind::fragment::Learned::Taught;
            x.tick = tick;
            x.tells = true;
            crate::mind::fragment::remember(&mut w.agents[l].fragments, x);
        }
    }
}

pub(crate) fn near(w: &World, at: crate::ids::TilePos, r: i32) -> Vec<usize> {
    w.agents
        .iter()
        .enumerate()
        .filter(|(_, a)| {
            a.alive
                && a.species == crate::agents::Species::Person
                && a.pos.z == at.z
                && a.pos.dist(at) <= r
                && !a.activity.is_asleep()
        })
        .map(|(k, _)| k)
        .collect()
}

/// After the flush: what just happened bears on what people believe.
pub fn observe(w: &mut World) {
    let tick = w.meta.tick;
    let events: Vec<_> = w
        .last_events
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                EventKind::Doused { .. } | EventKind::Burned { .. } | EventKind::Died { .. }
            )
        })
        .cloned()
        .collect();
    for e in events {
        let src = Source::Event(e.id);
        match &e.kind {
            // Water on a fire, and it went out: fire is something you fight. Most for the one
            // who threw it, some for those who saw.
            EventKind::Doused { agent, at } => {
                for k in near(w, *at, 8) {
                    let force = if w.agents[k].id == *agent { 0.3 } else { 0.12 };
                    w.agents[k]
                        .beliefs
                        .experience(Topic::Fire, -1.0, force, src, tick);
                }
            }
            // A building lost to fire: fire is something you run from.
            EventKind::Burned { at, .. } => {
                for k in near(w, *at, 10) {
                    w.agents[k]
                        .beliefs
                        .experience(Topic::Fire, 1.0, 0.15, src, tick);
                }
            }
            // Someone starved: in hunger you keep what you have. Felt by their community.
            EventKind::Died { agent, cause } if cause == "starvation" => {
                let community = w
                    .agents
                    .iter()
                    .find(|a| a.id == *agent)
                    .and_then(|a| a.community);
                if let Some(c) = community {
                    let members: Vec<usize> = w
                        .agents
                        .iter()
                        .enumerate()
                        .filter(|(_, a)| a.alive && a.community == Some(c))
                        .map(|(k, _)| k)
                        .collect();
                    for k in members {
                        w.agents[k]
                            .beliefs
                            .experience(Topic::Hunger, 1.0, 0.2, src, tick);
                    }
                }
            }
            _ => {}
        }
    }
}

/// What a people held when they came through (the wizard's ideology): seeded into everyone
/// standing in the world, sourced to the event that records the choice, free to drift after.
impl World {
    pub fn seed_ideology(&mut self, held: &[(Topic, f32)], choice: crate::ids::EventId) {
        let tick = self.meta.tick;
        for a in self.agents.iter_mut().filter(|a| {
            a.alive && a.species == crate::agents::Species::Person && a.community.is_some()
        }) {
            for (t, s) in held {
                // A shared belief joins what their own life taught them rather than erasing it.
                let own = a.beliefs.get(*t).map(|b| (b.stance, b.confidence));
                let (stance, conf) = match own {
                    Some((o, c)) => ((o * c + s * 0.6) / (c + 0.6), (c + 0.6).min(0.9)),
                    None => (*s, 0.6),
                };
                a.beliefs
                    .hold(*t, stance, conf, Source::Ideology(choice), tick);
            }
        }
    }
}
