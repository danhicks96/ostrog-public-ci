//! Minds under strain: thoughts from what happened, stress from a mood that stays low,
//! and breaks when it stacks up (a tantrum, withdrawal, wandering off, a fight). Breaks
//! are events others witness and remember.

use rand::Rng;

use crate::agents::{Agent, Species};
use crate::events::{BreakExplanation, EventKind, Severity};
use crate::ids::AgentId;
use crate::mind::thoughts::ThoughtKind;
use crate::mind::{Activity, MemKind, Subject};
use crate::rng::{purpose, stream};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::world_state::World;

/// Thoughts from the events of this tick. Runs after flush, like culture::observe.
pub fn observe(w: &mut World) {
    let tick = w.meta.tick;
    let events = w.last_events.clone();
    for e in &events {
        let src = Some(e.id);
        match &e.kind {
            EventKind::Ate { agent, quality, .. } if *quality >= 0.6 => {
                add(w, *agent, ThoughtKind::AteWell, src);
            }
            EventKind::Slept {
                agent,
                hours,
                in_bed,
            } => {
                let in_bed = *in_bed;
                // #274: a bed in a house of fine stone is a better night.
                if *hours >= 6.0 && in_bed {
                    let fine = w
                        .agents
                        .iter()
                        .find(|a| a.id == *agent)
                        .is_some_and(|a| crate::houses::fine_stone(w, a.pos).is_some());
                    if fine {
                        add(w, *agent, ThoughtKind::FineHouse, src);
                    }
                }
                if *hours >= 6.0 {
                    add(
                        w,
                        *agent,
                        if in_bed {
                            ThoughtKind::SleptInBed
                        } else {
                            ThoughtKind::SleptRough
                        },
                        src,
                    );
                }
            }
            EventKind::Quarrelled { a, b } => {
                add(w, *a, ThoughtKind::Quarrel, src);
                add(w, *b, ThoughtKind::Quarrel, src);
            }
            EventKind::Talked { a, b, warmth } if *warmth > 0.6 => {
                add(w, *a, ThoughtKind::GoodTalk, src);
                add(w, *b, ThoughtKind::GoodTalk, src);
            }
            EventKind::Tended { patient, .. } => add(w, *patient, ThoughtKind::Tended, src),
            EventKind::NewClothes { agent, .. } => add(w, *agent, ThoughtKind::NewClothes, src),
            EventKind::Crafted { agent, tier, .. } if *tier >= 3 => {
                add(w, *agent, ThoughtKind::MadeSomething, src);
            }
            EventKind::Paired { a, b } => {
                add(w, *a, ThoughtKind::Wedding, src);
                add(w, *b, ThoughtKind::Wedding, src);
            }
            EventKind::Born { mother, father, .. } => {
                add(w, *mother, ThoughtKind::NewChild, src);
                if let Some(f) = father {
                    add(w, *f, ThoughtKind::NewChild, src);
                }
            }
            EventKind::CaravanReturned { community, .. } => {
                let members: Vec<AgentId> = w
                    .agents
                    .iter()
                    .filter(|a| a.alive && a.community == Some(*community))
                    .map(|a| a.id)
                    .collect();
                for m in members {
                    add(w, m, ThoughtKind::PartyHome, src);
                }
            }
            EventKind::Died { agent, .. } => {
                // Kin grieve; those who saw it carry it a while.
                let dead = w.agents.iter().find(|a| a.id == *agent).cloned();
                if let Some(d) = dead {
                    let mut kin: Vec<AgentId> = Vec::new();
                    kin.extend(d.partner);
                    kin.extend(d.mother);
                    kin.extend(d.father);
                    kin.extend(d.children.iter().copied());
                    for k in kin {
                        if let Some(i) = w.agents.iter().position(|a| a.id == k && a.alive) {
                            w.agents[i]
                                .thoughts
                                .add(ThoughtKind::LostSomeone, tick, src);
                            // How heavy grief lies depends on what earlier losses taught them.
                            let g = w.agents[i]
                                .beliefs
                                .weight(crate::mind::beliefs::Topic::Death, 1.0);
                            if let Some(t) = w.agents[i]
                                .thoughts
                                .list
                                .iter_mut()
                                .rev()
                                .find(|t| t.kind == ThoughtKind::LostSomeone)
                            {
                                t.weight *= g;
                            }
                            w.agents[i].remember(
                                tick,
                                MemKind::Hardship,
                                Subject::Person(*agent),
                                -0.9,
                                1.0,
                                src,
                            );
                        }
                    }
                    let at = d.pos;
                    let witnesses: Vec<AgentId> = w
                        .agents
                        .iter()
                        .filter(|a| {
                            a.alive
                                && a.id != *agent
                                && a.species == Species::Person
                                && a.pos.dist(at) <= 10
                                && a.pos.z == at.z
                        })
                        .map(|a| a.id)
                        .collect();
                    for wit in witnesses {
                        add(w, wit, ThoughtKind::SawDeath, src);
                    }
                }
            }
            EventKind::Broke { agent, .. } => {
                let at = e.at;
                if let Some(at) = at {
                    let witnesses: Vec<AgentId> = w
                        .agents
                        .iter()
                        .filter(|a| {
                            a.alive
                                && a.id != *agent
                                && a.species == Species::Person
                                && a.pos.dist(at) <= 8
                                && a.pos.z == at.z
                                && !a.activity.is_asleep()
                        })
                        .map(|a| a.id)
                        .collect();
                    for wit in witnesses {
                        // #237: someone who broke down lately is not set off again by seeing
                        // another (the cascade was a quarter of all stress).
                        let lately = w.agents.iter().find(|a| a.id == wit).is_some_and(|a| {
                            a.last_break != 0
                                && tick.saturating_sub(a.last_break) < 2 * TICKS_PER_DAY
                        });
                        if !lately {
                            add(w, wit, ThoughtKind::SawBreak, src);
                        }
                        if let Some(i) = w.agents.iter().position(|a| a.id == wit) {
                            w.agents[i].remember(
                                tick,
                                MemKind::Company,
                                Subject::Person(*agent),
                                -0.5,
                                0.7,
                                src,
                            );
                            w.agents[i].relate(*agent, -0.1, 0.05, tick);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn add(w: &mut World, who: AgentId, kind: ThoughtKind, src: Option<crate::ids::EventId>) {
    let tick = w.meta.tick;
    if let Some(a) = w.agents.iter_mut().find(|a| a.id == who && a.alive) {
        a.thoughts.add(kind, tick, src);
    }
}

/// The arrival (#392, QA 2026-10-26; polarized #395): survivors came through the portal at
/// mood 0 and stress 0, as if nothing had happened. Each now carries a thought of it that
/// weighs on mood over the first days: three to four and a half days at roughly -0.4 to
/// -0.8, shorter and lighter the calmer (a calm of 1 shakes it in two and a half days),
/// heavier for the cautious, shorter for the easygoing; and a spike of stress at once,
/// highest in the least calm. The shock is polarized by temperament (#395): the least calm
/// are the panic tail and tip straight over their break threshold (in ten, one or two are
/// non-functional at first), while the most calm snap to the survival pole instead
/// (`survival_urge`). Every survivor also carries a dread of the missing, the rest of the
/// party, unaccounted for.
pub fn arrival_shock(a: &mut crate::agents::Agent, now: u64) {
    use crate::mind::Trait;
    let calm = a.genome.calm();
    // FIXLOG 2026-11-01, branch 482-cave-mouths: survival gains must not erase the first-day shock.
    let mut weight = -0.55 * (1.2 - 0.4 * calm);
    let mut days = 3.5 * (1.3 - 0.6 * calm);
    if a.traits.has(Trait::Cautious) {
        weight *= 1.2;
    }
    if a.traits.has(Trait::Easygoing) {
        days *= 0.8;
    }
    a.thoughts
        .add_shaped(ThoughtKind::Arrival, weight, days, now);
    // The rest of the party is not here (#395, FIXLOG 2026-11-01): seven of the ten
    // came through somewhere else, and that dread sits in the background of the first
    // days the way the shock does, easing as a standing thought eases.
    a.thoughts.add(ThoughtKind::Missing, now, None);
    // The shock is a spike of stress at the arrival, highest in the least calm, which then
    // eases as any stress does; the thought itself is left out of the hourly build (`step`),
    // or a whole party broke in the first days (a first try: 6 to 18 breaks a seed in ten
    // days against 0 to 2).
    a.stress = (a.stress + 0.05 + 0.15 * (1.0 - calm)).min(1.0);
    // The panic tail (#395, FIXLOG 2026-11-01): the least calm go straight over their
    // break threshold, and the first hourly `step` below breaks them — freaking out,
    // freezing, wandering off, refusing work — as the first hour of an abrupt stranding
    // does. The threshold is their own, the same one `step` breaks people at; the spike,
    // not the thought, does it, so the calmer are untouched (the whole party breaking was
    // tried for #392 and reverted).
    if calm <= PANIC_LINE {
        let threshold = threshold(a);
        a.stress = a.stress.max((threshold + 0.05).min(1.0));
    }
}

/// At or below this temperament a survivor is the panic tail of the arrival shock
/// (#395, FIXLOG 2026-11-01): the spike tips them over their break threshold at once.
/// About one or two of ten.
pub const PANIC_LINE: f32 = 0.15;

/// While the arrival is fresh, the things that keep them alive come first (#392): 1.5 on the
/// day they came through, falling to 1 as the arrival thought fades; the survival-snap pole
/// of the shock runs a third stronger again (#395, FIXLOG 2026-11-01). Read by
/// `decide::choose` as a multiplier on eating, foraging, getting warm and the work of
/// shelter and food, and as the divisor that puts leisure off while the shock is fresh.
pub fn survival_urge(a: &crate::agents::Agent, now: u64) -> f32 {
    // The snap pole is a feature of the shock, so it fades with it. Unscaled it was a
    // permanent 1.35x on survival work for the steadiest, long after the arrival, and it
    // outvoted dispositions in settled people's choices (FIXLOG 2026-11-02 #548).
    let fresh = a.thoughts.left(ThoughtKind::Arrival, now);
    let snap = if snap(a) { fresh } else { 0.0 };
    (1.0 + 0.5 * fresh) * (1.0 + SNAP * snap)
}

/// How much harder the survival-snap pole's urgency runs (#395).
pub const SNAP: f32 = 0.35;
/// At or above this temperament a survivor is the survival-snap pole of the party
/// (#395, FIXLOG 2026-11-01): composed enough to lock onto water, shelter, warmth and
/// food at once and drag the rest.
pub const SNAP_LINE: f32 = 0.85;

/// The survival-snap pole of the bootstrap shock (#395, FIXLOG 2026-11-01): the steadiest
/// of the party.
fn snap(a: &crate::agents::Agent) -> bool {
    a.genome.calm() >= SNAP_LINE
}

/// Hourly: standing thoughts from states, stress from mood, breaks from stress.
/// The share of the bad thoughts' gross weight that builds stress whatever the mood (#237).
pub const HARD_SHARE: f32 = 0.2;
/// The extra hourly stress ease from a markedly calmer person nearby (#395).
pub const STEADY: f32 = 0.002;
/// Throttle key for asking after the missing (#395, FIXLOG 2026-11-01), once a day per
/// person.
const WONDER_KEY: u32 = 0x7200_0000;

/// What a breakdown was over: the strongest hard feeling (fear, grief, anger, shame) at
/// 0.3 or more, with its object; else the heaviest bad thought; else a need near empty; else
/// nothing anyone can name. Read-only: it changes nothing about how a person breaks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cause {
    Feeling(crate::mind::feelings::FeelingKind, Subject),
    Thought(ThoughtKind, Subject),
    Need(&'static str),
    Unknown,
}

pub fn break_cause(a: &crate::agents::Agent, now: u64) -> Cause {
    use crate::mind::feelings::FeelingKind as F;
    let hard = a
        .feelings
        .list
        .iter()
        .filter(|f| matches!(f.kind, F::Fear | F::Grief | F::Anger | F::Shame) && f.strength >= 0.3)
        .max_by(|x, y| x.strength.total_cmp(&y.strength));
    if let Some(f) = hard {
        let subject = f.about.map_or(Subject::None, Subject::Person);
        return Cause::Feeling(f.kind, subject);
    }
    let heaviest = a
        .thoughts
        .list
        .iter()
        .filter(|t| t.weight < 0.0 && t.kind != ThoughtKind::Arrival && t.until > now)
        .min_by(|x, y| {
            a.thoughts
                .sum_of(x.kind, now)
                .total_cmp(&a.thoughts.sum_of(y.kind, now))
        });
    if let Some(t) = heaviest {
        return Cause::Thought(t.kind, Subject::None);
    }
    let n = &a.needs;
    let low = [
        (n.hunger, "hunger"),
        (n.thirst, "thirst"),
        (n.rest, "weariness"),
        (n.warmth, "cold"),
        (n.social, "loneliness"),
        (n.comfort, "discomfort"),
    ]
    .into_iter()
    .filter(|(v, _)| *v < 0.2)
    .min_by(|x, y| x.0.total_cmp(&y.0));
    match low {
        Some((_, name)) => Cause::Need(name),
        None => Cause::Unknown,
    }
}

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_HOUR != 900 {
        return;
    }
    let seed = w.meta.seed;
    let n = w.agents.len();
    let mut breaks: Vec<usize> = Vec::new();
    for i in 0..n {
        if !w.agents[i].alive || w.agents[i].species != Species::Person {
            continue;
        }
        // A steadier presence nearby eases the strain (#395, FIXLOG 2026-11-01, the
        // calm side of contagion): while the arrival dread is still on them, an awake
        // person of notably more even temperament within a few tiles steadies them a
        // little, every hour they are there. It is a bootstrap phenomenon, so it ends
        // with the Missing thought and never touches worlds without an arrival.
        let steadied = {
            let me = &w.agents[i];
            let calm = me.genome.calm();
            me.thoughts.has(ThoughtKind::Missing)
                && w.agents.iter().any(|o| {
                    o.alive
                        && o.id != me.id
                        && o.species == Species::Person
                        && !o.activity.is_asleep()
                        && o.pos.z == me.pos.z
                        && o.pos.dist(me.pos) <= 8
                        && o.genome.calm() >= calm + 0.3
                })
        };
        let a = &mut w.agents[i];
        a.thoughts.prune(tick);
        if a.needs.hunger < 0.2 {
            a.thoughts.add(ThoughtKind::Hungry, tick, None);
        }
        // #879: a person dying of thirst read as content (mood fed only on hunger, rest,
        // social and comfort). Same gate as Hungry.
        if a.needs.thirst < 0.2 {
            a.thoughts.add(ThoughtKind::Thirsty, tick, None);
        }
        if a.body.worst_wound() >= 0.3 {
            a.thoughts.add(ThoughtKind::Hurt, tick, None);
        }
        if a.body.ill() >= 0.3 {
            a.thoughts.add(ThoughtKind::Sick, tick, None);
        }
        // Stress builds from hardship (the bad thoughts, hunger, pain) and eases with calm
        // and a good mood; a run of bad days breaks people, one bad day does not.
        // #237: stress builds from the net of what a person carries, the good offsetting the
        // bad, plus a fifth of the bad on its own (a hard thing still weighs on a good day).
        // From the gross bad alone, a contented village broke once a head every four days:
        // the build always beat the ease, and someone sleeping rough carried it for good.
        let gross: f32 = a
            .thoughts
            .list
            .iter()
            .filter(|t| {
                t.weight < 0.0 && !matches!(t.kind, ThoughtKind::Arrival | ThoughtKind::Broke)
            })
            .map(|t| -t.weight)
            .sum::<f32>();
        // The arrival's stress came all at once (`arrival_shock`); its thought weighs on mood.
        // OST-241: the `Broke` thought weighs on mood only; as stress it would feed the next break.
        let net = a.thoughts.sum(tick)
            - a.thoughts.sum_of(ThoughtKind::Arrival, tick)
            - a.thoughts.sum_of(ThoughtKind::Broke, tick);
        let hardship: f32 = ((-net).max(0.0) + HARD_SHARE * gross).min(2.0);
        let calm = a.genome.calm();
        let build = hardship * 0.03 * (1.3 - calm);
        let ease =
            0.005 * (0.5 + calm) * (1.0 + a.mood.max(0.0)) + if steadied { STEADY } else { 0.0 };
        a.stress = (a.stress + build - ease).clamp(0.0, 1.0);
        let threshold = threshold(a);
        let child = a.is_child(tick);
        let rested = a.last_break == 0 || tick - a.last_break > TICKS_PER_DAY * 2;
        if a.stress >= threshold && rested && !child {
            breaks.push(i);
        }
    }
    for i in breaks {
        let mut r = stream(
            seed,
            &[purpose::AGENT_SOCIAL, 0xB4EA, w.agents[i].id.0 as u64, tick],
        );
        let who = w.agents[i].id;
        let here = w.agents[i].pos;
        // Someone nearby they think little of makes it a fight; otherwise temperament picks.
        let target = w
            .agents
            .iter()
            .filter(|o| {
                o.alive
                    && o.id != who
                    && o.species == Species::Person
                    && o.pos.dist(here) <= 8
                    && o.pos.z == here.z
                    && !o.activity.is_asleep()
            })
            .filter(|o| w.agents[i].opinion_of(o.id) < -0.2)
            .map(|o| o.id)
            .next();
        let roll: f32 = r.gen();
        let industrious = w.agents[i].traits.has(crate::mind::Trait::Industrious);
        let solitary = w.agents[i].traits.has(crate::mind::Trait::Solitary);
        let kind = if let Some(t) = target {
            // Whether a break turns on someone depends in part on what they learned about force;
            // one who holds firmly that you never raise a hand does not (P11: the restless dead
            // add to strain, and a village of the gentle came to blows in the threats gates).
            let lean = w.agents[i]
                .beliefs
                .lean(crate::mind::beliefs::Topic::Violence);
            let fight = if lean <= -0.6 {
                0.0
            } else {
                0.6 * w.agents[i]
                    .beliefs
                    .weight(crate::mind::beliefs::Topic::Violence, 1.0)
            };
            if roll < fight {
                Break::Fight(t)
            } else {
                Break::Tantrum
            }
        } else {
            // #332: a bag of ways to break, weighted by who they are, and each person leaning
            // to one way of their own (by who they are, so the same person breaks alike).
            let a = &w.agents[i];
            let t = |x| a.traits.has(x);
            use crate::mind::Trait as T;
            let lonely = a.relationships.values().all(|r| r.opinion < 0.3);
            let violent = a.beliefs.lean(crate::mind::beliefs::Topic::Violence) > 0.4;
            // One who came through within the day has nowhere to walk out to (#395,
            // FIXLOG 2026-11-01: the panic tail of the arrival breaks into wandering,
            // withdrawal or a daze, not out of the valley for good).
            let arrived_today = tick.saturating_sub(a.came) < TICKS_PER_DAY;
            let mut wt = [
                3.0 + if industrious { 2.0 } else { 0.0 },        // tantrum
                3.0,                                              // withdraw
                2.0 + if solitary { 3.0 } else { 0.0 },           // wander
                1.0 + if t(T::Gourmet) { 3.0 } else { 0.0 },      // binge
                1.0 + if t(T::Cautious) { 2.0 } else { 0.0 },     // daze
                if lonely && !arrived_today { 0.6 } else { 0.0 }, // walk out
                if violent { 0.4 } else { 0.0 },                  // fire
            ];
            let own = (a.id.0 as usize).wrapping_mul(2_654_435_761) % 5;
            wt[own] += 3.0;
            let total: f32 = wt.iter().sum();
            let mut x = roll * total;
            let mut k = 0;
            while k < wt.len() - 1 && x >= wt[k] {
                x -= wt[k];
                k += 1;
            }
            match k {
                0 => Break::Tantrum,
                1 => Break::Withdraw,
                2 => Break::Wander,
                3 => Break::Binge,
                4 => Break::Daze,
                5 => Break::WalkOut,
                _ => Break::Firestart,
            }
        };
        let label = match kind {
            Break::Tantrum => "a tantrum",
            Break::Withdraw => "withdrawal",
            Break::Wander => "wandering off",
            Break::Fight(_) => "a fight",
            Break::Binge => "a binge",
            Break::Daze => "a daze",
            Break::WalkOut => "walking out",
            Break::Firestart => "setting a fire",
        };
        let explanation = match break_cause(&w.agents[i], tick) {
            Cause::Feeling(k, Subject::Person(about)) => BreakExplanation::Known {
                about: Some(about),
                cause: k.word().to_string(),
            },
            Cause::Feeling(k, _) => BreakExplanation::Known {
                about: None,
                cause: format!("from {}", k.word()),
            },
            Cause::Thought(k, _) => BreakExplanation::Known {
                about: None,
                cause: format!("because they were {}", k.label()),
            },
            Cause::Need(n) => BreakExplanation::Known {
                about: None,
                cause: format!("from {n}"),
            },
            Cause::Unknown => BreakExplanation::Unknown,
        };
        {
            let a = &mut w.agents[i];
            a.stress = 0.15;
            a.last_break = tick;
            // OST-241: a direct mood write was rebuilt away a game-minute later; the cost is
            // now the `Broke` thought added with the event below (see docs/history/FIXLOG.md).
        }
        // Whatever they were carrying for a job goes down where they stand, or the job's
        // materials would be in their hands for ever.
        for it in w.items.iter_mut() {
            if it.place == crate::items::ItemPlace::Carried(who) && !it.kind.is_food() {
                it.place = crate::items::ItemPlace::Ground(here);
            }
        }
        let eid = w.events.push(
            tick,
            Some(here),
            Severity::Major,
            EventKind::Broke {
                agent: who,
                kind: label.into(),
                // Do not consume a second event ID: some simulation draws use event IDs.
                explanation: Some(explanation),
            },
        );
        w.agents[i].remember(tick, MemKind::Hardship, Subject::None, -0.6, 0.8, Some(eid));
        w.agents[i]
            .thoughts
            .add(ThoughtKind::Broke, tick, Some(eid));
        match kind {
            Break::Tantrum => {
                // Break the nearest piece of furniture, if any is close.
                if let Some((sid, skind, spos, sowner)) = w
                    .structures
                    .iter()
                    .filter(|s| {
                        s.pos.dist(here) <= 6
                            && s.pos.z == here.z
                            && s.progress >= 1.0
                            && s.kind.breakable_in_anger()
                    })
                    .min_by_key(|s| s.pos.dist(here))
                    .map(|s| (s.id, s.kind, s.pos, s.owner))
                {
                    w.structures.retain(|s| s.id != sid);
                    let label = format!("{skind:?}").to_lowercase();
                    // #1791: an owner who is not the breaker, and saw it, is wronged by it, not
                    // just a stranger finding wreckage; the thing is named theirs.
                    let owner_ix = sowner
                        .filter(|&o| o != who)
                        .and_then(|o| w.agents.iter().position(|a| a.id == o));
                    let what = match owner_ix {
                        Some(oi) => format!("{}'s {}", w.agents[oi].name, label),
                        None => label.clone(),
                    };
                    let eid = w.events.push(
                        tick,
                        Some(spos),
                        Severity::Notable,
                        EventKind::FurnitureBroken { agent: who, what },
                    );
                    let victim = owner_ix.filter(|&oi| {
                        // The same `seen` test `offence` applies for an unnamed accuser:
                        // alive, awake, same z, within 8 tiles of the breaking.
                        let o = &w.agents[oi];
                        o.alive
                            && o.pos.z == spos.z
                            && o.pos.dist(spos) <= 8
                            && !o.activity.is_asleep()
                    });
                    if let Some(oi) = victim {
                        w.agents[oi].relate(who, -0.2, 0.0, tick);
                        w.agents[oi].remember(
                            tick,
                            MemKind::Hardship,
                            Subject::Person(who),
                            -0.5,
                            0.8,
                            Some(eid),
                        );
                    }
                    crate::justice::offence(
                        w,
                        i,
                        victim.map(|oi| w.agents[oi].id),
                        &format!("breaking the {label}"),
                        spos,
                    );
                }
                w.reservations.release_all(who);
                w.agents[i].activity = Activity::Loitering {
                    ticks_left: TICKS_PER_HOUR as u32,
                };
                w.agents[i].activity_ticks = 0;
                w.agents[i].path.clear();
            }
            Break::Binge => {
                // Everything to eat within six tiles, up to four helpings.
                let mut left = 4u32;
                let near: Vec<(crate::ids::ItemId, u32)> = w
                    .items
                    .iter()
                    .filter(|it| {
                        it.kind.is_food()
                            && matches!(it.place, crate::items::ItemPlace::Ground(p) if p.dist(here) <= 6)
                    })
                    .map(|it| (it.id, it.count))
                    .collect();
                for (id, n) in near {
                    if left == 0 {
                        break;
                    }
                    let k = n.min(left);
                    w.consume_item(id, k, "eaten in a binge");
                    left -= k;
                }
                w.agents[i].needs.hunger = 1.0;
                w.reservations.release_all(who);
                w.agents[i].activity = Activity::Loitering {
                    ticks_left: TICKS_PER_HOUR as u32,
                };
                w.agents[i].activity_ticks = 0;
                w.agents[i].path.clear();
            }
            Break::Daze => {
                w.reservations.release_all(who);
                w.agents[i].withdrawn_until = tick + 2 * TICKS_PER_DAY;
                w.agents[i].activity = Activity::Loitering {
                    ticks_left: TICKS_PER_DAY as u32,
                };
                w.agents[i].activity_ticks = 0;
                w.agents[i].path.clear();
            }
            Break::WalkOut => {
                // Nobody holds them here: out of the valley, for good (society::leavers).
                w.reservations.release_all(who);
                w.agents[i].community = None;
                w.agents[i].leaving = true;
            }
            Break::Firestart => {
                let name = w.agents[i].name.clone();
                let spot = (1..=4)
                    .flat_map(|r| [(r, 0), (-r, 0), (0, r), (0, -r)].into_iter())
                    .map(|(dx, dy)| here.offset(dx, dy, 0))
                    .find(|p| crate::systems::hazards::fuel_at(w, *p) > 0);
                if let Some(p) = spot {
                    // #404 (2026-10-27 fix-404-feuds): one of the gravest offences, judged.
                    if w.ignite(p, &format!("set by {name} in a rage")) {
                        crate::justice::offence(w, i, None, crate::justice::ARSON, p);
                    }
                }
                w.reservations.release_all(who);
                w.agents[i].activity = Activity::Loitering {
                    ticks_left: TICKS_PER_HOUR as u32,
                };
                w.agents[i].activity_ticks = 0;
            }
            Break::Withdraw => {
                w.reservations.release_all(who);
                w.agents[i].withdrawn_until = tick + TICKS_PER_DAY;
                w.agents[i].activity = Activity::Loitering {
                    ticks_left: TICKS_PER_HOUR as u32 * 2,
                };
                w.agents[i].activity_ticks = 0;
                w.agents[i].path.clear();
            }
            Break::Wander => {
                w.reservations.release_all(who);
                w.agents[i].withdrawn_until = tick + TICKS_PER_DAY;
                // Somewhere far from everyone.
                let g = &w.grid;
                let far = crate::ids::TilePos::new(
                    (here.x + r.gen_range(-30..=30)).clamp(2, g.width - 3),
                    (here.y + r.gen_range(-30..=30)).clamp(2, g.height - 3),
                    here.z,
                );
                let far = g.surface(far.x, far.y);
                if g.walkable(far) && w.path_agent_to(i, far) {
                    w.agents[i].activity = Activity::Wandering { to: far };
                } else {
                    w.agents[i].activity = Activity::Loitering {
                        ticks_left: TICKS_PER_HOUR as u32 * 2,
                    };
                }
                w.agents[i].activity_ticks = 0;
            }
            Break::Fight(t) => fight(w, i, t, Some(eid), &mut r),
        }
    }
    // While the missing are still out there, the party is heard asking after them
    // (#395, FIXLOG 2026-11-01): a named "where are they", once a day per person, for
    // the chronicle the viewer reads. The dread itself is the `Missing` thought, seeded
    // at the arrival and decaying away with these first days; only community members ask
    // aloud.
    let lost: Vec<String> = w
        .survivor_pool
        .iter()
        .filter(|s| {
            matches!(
                s.state,
                crate::opening::SurvivorState::Lost { found: false, .. }
            )
        })
        .map(|s| crate::opening::slot_name(w.meta.seed, s.index, s.rerolls))
        .collect();
    if !lost.is_empty() {
        let askers: Vec<AgentId> = w
            .agents
            .iter()
            .filter(|a| {
                a.alive
                    && a.species == Species::Person
                    && a.community.is_some()
                    && a.thoughts.has(ThoughtKind::Missing)
                    && a.work_cooldowns.get(&WONDER_KEY).is_none_or(|t| *t <= tick)
            })
            .map(|a| a.id)
            .collect();
        for id in askers {
            let pos = w.agents.iter().find(|a| a.id == id).map(|a| a.pos);
            let who = &lost[id.0 as usize % lost.len()];
            w.events.push(
                tick,
                pos,
                Severity::Minor,
                EventKind::Chose {
                    agent: id,
                    what: format!("wondered where {who} is"),
                    because: "the rest of the party is not here".into(),
                },
            );
            if let Some(a) = w.agents.iter_mut().find(|a| a.id == id) {
                a.work_cooldowns.insert(WONDER_KEY, tick + TICKS_PER_DAY);
            }
        }
    }
}

/// `i` starts a fight with `t` (C77: an offence, judged in the evening): both are bruised and
/// think the worse of each other. A break that turns on someone, or a quarrel between two who
/// bear each other ill will that comes to blows (systems/act.rs, #404).
pub fn fight(
    w: &mut World,
    i: usize,
    t: AgentId,
    eid: Option<crate::ids::EventId>,
    r: &mut impl Rng,
) {
    let tick = w.meta.tick;
    let who = w.agents[i].id;
    let here = w.agents[i].pos;
    crate::justice::offence(w, i, Some(t), "starting a fight", here);
    if let Some(j) = w.agents.iter().position(|o| o.id == t) {
        for (x, y) in [(i, j), (j, i)] {
            let other = w.agents[y].id;
            w.agents[x].relate(other, -0.3, 0.1, tick);
            w.agents[x].remember(
                tick,
                MemKind::Company,
                Subject::Person(other),
                -0.8,
                0.9,
                eid,
            );
            struck(w, y, x, r);
        }
        w.reservations.release_all(who);
        w.reservations.release_all(t);
        for k in [i, j] {
            w.agents[k].activity = Activity::Loitering {
                ticks_left: TICKS_PER_HOUR as u32 / 2,
            };
            w.agents[k].activity_ticks = 0;
            w.agents[k].path.clear();
        }
        w.events.push(
            tick,
            Some(here),
            Severity::Major,
            EventKind::Fought { a: who, b: t },
        );
    }
}

/// `y` strikes `x` in a fight between people of the valley. Mostly a bruise; now and then a
/// cut from whatever is to hand or a broken bone; a spear if `y` carries one and holds that
/// force settles things. A blow on a part already hurt goes deeper (as `combat::blow`, #402).
/// The wound names who struck it and that it was struck in anger (#404): a death or a lost limb
/// from it is a killing or a maiming (health.rs, `justice::struck_down`).
fn struck(w: &mut World, y: usize, x: usize, r: &mut impl Rng) {
    use crate::body::{Part, WoundKind};
    let by = w.agents[y].id;
    let before = w.agents[x].body.wounds.len();
    let armed = crate::systems::arms::edge(w, y).is_some()
        && w.agents[y]
            .beliefs
            .lean(crate::mind::beliefs::Topic::Violence)
            > 0.4;
    if armed {
        crate::systems::combat::blow(w, y, x, r, "in a fight");
    } else {
        let part = [Part::HEAD, Part::TORSO, Part::ARM_L, Part::ARM_R][r.gen_range(0..4)];
        let roll: f32 = r.gen();
        let (kind, sev, bleeding) = if roll < 0.8 {
            (WoundKind::Bruise, 0.15 + r.gen::<f32>() * 0.2, 0.0)
        } else if roll < 0.95 {
            (WoundKind::Cut, 0.2 + r.gen::<f32>() * 0.3, 0.1)
        } else {
            (WoundKind::Break, 0.4 + r.gen::<f32>() * 0.2, 0.0)
        };
        let open: f32 = w.agents[x]
            .body
            .wounds
            .iter()
            .filter(|wd| wd.part == part)
            .map(|wd| wd.severity)
            .sum();
        crate::systems::health::wound(
            w,
            x,
            part,
            kind,
            sev + 0.5 * open,
            bleeding,
            "in a fight",
            Some(by),
        );
    }
    if w.agents[x].body.wounds.len() > before {
        if let Some(wd) = w.agents[x].body.wounds.last_mut() {
            wd.in_anger = true;
        }
    }
}

enum Break {
    Tantrum,
    Withdraw,
    Wander,
    Fight(AgentId),
    /// #332: eats whatever food lies near.
    Binge,
    /// #332: sits staring for a day, work let go.
    Daze,
    /// #332: one with nobody close walks out of the valley for good.
    WalkOut,
    /// #332: one who holds that force settles things sets something alight.
    Firestart,
}

/// The stress at which this person breaks: the calmer, the higher.
pub fn threshold(a: &Agent) -> f32 {
    0.55 + a.genome.calm() * 0.35
}
