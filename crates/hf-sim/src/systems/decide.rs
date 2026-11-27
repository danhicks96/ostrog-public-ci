//! Decision: an idle agent scores candidate goals from needs × traits × knowledge and
//! commits to one activity. Targets come only from the agent's own knowledge and must be
//! reachable (checked before scoring). A critical need interrupts a running activity.
//!
//! Phase 3 goals: eat, forage, sleep, talk, wander, loiter. Work goals arrive in Phase 4
//! through the planner; this scorer stays the entry point.

use rand::Rng;

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, StructureId, TilePos};
use crate::items::ItemPlace;
use crate::mind::knowledge::Known;
use crate::mind::needs::Need;
use crate::mind::{Activity, MemKind, Subject};
use crate::rng::{purpose, stream};
use crate::structures::StructureKind;
use crate::time::{Calendar, TICKS_PER_DAY, TICKS_PER_HOUR, TICKS_PER_MINUTE};
use crate::world_state::World;

/// Minimum ticks between re-decisions when nothing finished (oscillation guard).
pub const RETHINK_TICKS: u32 = 200;

/// Knowledge of a structure keeps only its kind's tag (`Known::Structure`'s `b.tag`), so this
/// checks a known tag against `StructureKind::is_sleeping_place` without an unsafe cast
/// (#1081, correction on #1081: three hard-coded `Bed | Tent | Nest` lists here missed the
/// lean-to).
fn tag_is_sleeping_place(tag: u16) -> bool {
    [
        StructureKind::Bed,
        StructureKind::Tent,
        StructureKind::Nest,
        StructureKind::LeanTo,
    ]
    .iter()
    .any(|k| *k as u16 == tag)
}

/// Days of food a community must hold for tomorrow to count as secured (#415).
pub const SECURE_DAYS: f32 = 2.0;
/// Leisure's weight (talk, wander, loiter, the hearth, study) while today's food, tomorrow's
/// or tonight's shelter is not secured (#415): near nothing, not nothing.
pub const LEISURE_UNSECURED: f32 = 0.05;
/// How far from camp one with nothing secured goes before walking back to it (#415).
pub const GROUP_RANGE: i32 = 24;
/// How near camp one with no bed or roof beds down for the night (#415).
pub const SETTLE_RANGE: i32 = 8;

#[derive(Clone, Debug)]
#[allow(dead_code)]
enum Goal {
    /// Go and look for one of the party who is not here.
    Search(u8, TilePos),
    EatItem(crate::ids::ItemId, TilePos),
    Forage(TilePos),
    SleepBed(crate::ids::StructureId, TilePos),
    SleepHere,
    Talk(AgentId, TilePos),
    Wander(TilePos),
    /// Walk with a purpose while something is not secured (#415): back to the group or camp,
    /// or out to where food was found before. The same walk as `Wander`, but not leisure.
    Seek(TilePos),
    /// Search the ground for water without committing to an unlocated source.
    SeekWater(TilePos),
    /// Gather from a food plant for the store, not for their own hunger (#415).
    Stock(TilePos),
    Loiter,
    Work(crate::work::DesignationId),
    /// Get to a fire or under a roof.
    Warm(TilePos),
    /// Take up a fire with a technique they know (or are about to try).
    Respond(TilePos, crate::mind::Technique),
    /// Get away from a fire.
    Flee(TilePos),
    /// Tend someone hurt or sick (they know how and chose to).
    Tend(AgentId, TilePos),
    /// Go after one of a band of strangers (threat.rs).
    Confront(AgentId, TilePos),
    /// Go to a band's camp and tell them they may eat.
    Welcome(u32, TilePos),
    /// Sit over a record or a tool from the old valley and try to learn its craft (C13).
    Study(crate::ids::ItemId, TilePos),
    /// Go after a beast that has hurt people (creatures.rs).
    Avenge(AgentId, TilePos),
    /// Sit at the hearth in the evening, where someone may tell what they remember.
    Gather(TilePos),
    /// Sit by the portal stone and try to understand what is happening there (thin.rs).
    StudyThin(TilePos),
    /// Go to water (a shore, a well) and drink there (#433).
    Drink(TilePos),
}

/// The stable cycle name and the words of a goal, for the Why? lens (#567). Read only when
/// the person is traced, so its lookups cost nothing otherwise.
fn goal_words(w: &World, g: &Goal) -> (&'static str, String) {
    let who = |id: AgentId| {
        w.agents
            .iter()
            .find(|o| o.id == id)
            .map(|o| o.name.clone())
            .unwrap_or_else(|| "someone".to_string())
    };
    match g {
        Goal::Search(_, at) => (
            "search_for_the_lost",
            format!("go looking for one of the lost near {at}"),
        ),
        Goal::EatItem(id, at) => {
            let kind = w
                .items
                .iter()
                .find(|it| it.id == *id)
                .map(|it| it.kind.label())
                .unwrap_or("food");
            ("go_for_meals", format!("eat the {kind} at {at}"))
        }
        Goal::Forage(at) => ("go_for_meals", format!("gather food at {at}")),
        Goal::SleepBed(_, at) => ("sleep", format!("sleep in the bed at {at}")),
        Goal::SleepHere => ("sleep", "sleep where they are".to_string()),
        Goal::Talk(id, _) => ("talk", format!("talk with {}", who(*id))),
        Goal::Wander(at) => ("wander", format!("wander toward {at}")),
        Goal::Seek(at) => ("keep_together", format!("walk with a purpose to {at}")),
        Goal::SeekWater(at) => ("drink", format!("look for drinking water near {at}")),
        Goal::Stock(at) => ("stock_the_store", format!("gather for the store at {at}")),
        Goal::Loiter => ("rest_a_moment", "rest a moment".to_string()),
        Goal::Work(did) => {
            let what = w
                .designations
                .iter()
                .find(|d| d.id == *did)
                .map(|d| d.label())
                .unwrap_or_else(|| "work".to_string());
            ("work", format!("work: {what}"))
        }
        Goal::Warm(at) => ("keep_warm", format!("get warm or under a roof at {at}")),
        Goal::Respond(at, t) => (
            "fight_fire",
            format!("fight the fire at {at} by {}", t.name()),
        ),
        Goal::Flee(at) => ("flee_fire", format!("get away from the fire to {at}")),
        Goal::Tend(id, _) => ("tend", format!("tend {}", who(*id))),
        Goal::Confront(id, _) => ("confront", format!("go after {}", who(*id))),
        Goal::Welcome(_, at) => ("welcome", format!("go out to the strangers' camp at {at}")),
        Goal::Study(_, at) => ("study", format!("study the old thing at {at}")),
        Goal::Avenge(id, _) => ("avenge", format!("go after {}", who(*id))),
        Goal::Gather(at) => ("hearth", format!("sit at the hearth at {at}")),
        Goal::StudyThin(at) => (
            "study_the_stone",
            format!("sit by the portal stone at {at}"),
        ),
        Goal::Drink(at) => ("drink", format!("drink at {at}")),
    }
}

/// How far a regretted quarrel must lean someone toward helping before they go to tend the other
/// though neither healer nor kin (#568).
pub const AMENDS: f32 = 1.15;

/// At most this many people rethink in one tick. When a whole settlement wakes at once,
/// everyone deciding in the same tick made it the slowest of the day (85 ms at 500 people,
/// about half a millisecond each);
/// the rest decide in the ticks after, a few seconds of game time later.
pub const DECISIONS_PER_TICK: usize = 24;

thread_local! {
    static SLEEP_PLACE_WALKS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Full sleeping-place structure walks, for the decision-cost guard (#469).
pub fn sleep_place_walks() -> u64 {
    SLEEP_PLACE_WALKS.with(|c| c.get())
}

const SLEEP_CELL: i32 = 16;
type SleepCell = (i32, i32, i32);

/// Completed sleeping places by local cell and their current claims, built at most once per
/// decision tick (#469, FIXLOG 2026-10-31 c144 review).
#[derive(Default)]
struct SleepingPlaces {
    by_cell: std::collections::BTreeMap<SleepCell, Vec<(usize, StructureId, TilePos)>>,
    claims: std::collections::BTreeMap<StructureId, std::collections::BTreeSet<AgentId>>,
    /// The sleeping places within 40 of each community's home, in structure order, gathered
    /// on the first sleepy decision of one of its people (#469, FIXLOG 2026-11-02).
    homes: std::collections::BTreeMap<crate::ids::CommunityId, Vec<(StructureId, TilePos)>>,
}

impl SleepingPlaces {
    fn build(w: &World) -> Self {
        SLEEP_PLACE_WALKS.with(|c| c.set(c.get() + 1));
        let mut places = Self::default();
        for (order, s) in w.structures.iter().enumerate() {
            if s.progress >= 1.0 && s.kind.is_sleeping_place() {
                places
                    .by_cell
                    .entry(Self::cell(s.pos))
                    .or_default()
                    .push((order, s.id, s.pos));
            }
        }
        for a in &w.agents {
            if let Some(id) = Self::claim_of(&a.activity) {
                places.claims.entry(id).or_default().insert(a.id);
            }
        }
        places
    }

    /// The beds of home: within 40 of the community's home, as its hearths are counted
    /// (perceive.rs `SharedStore`).
    fn home_beds(
        &mut self,
        cid: crate::ids::CommunityId,
        home: TilePos,
    ) -> &[(StructureId, TilePos)] {
        let by_cell = &self.by_cell;
        self.homes.entry(cid).or_insert_with(|| {
            let mut v: Vec<(usize, StructureId, TilePos)> = by_cell
                .values()
                .flatten()
                .filter(|(_, _, p)| p.dist(home) <= 40)
                .copied()
                .collect();
            v.sort_by_key(|(order, _, _)| *order);
            v.into_iter().map(|(_, id, p)| (id, p)).collect()
        })
    }

    fn cell(p: TilePos) -> SleepCell {
        (p.x.div_euclid(SLEEP_CELL), p.y.div_euclid(SLEEP_CELL), p.z)
    }

    fn near(me: TilePos, r: i32) -> impl Iterator<Item = SleepCell> {
        let (x0, x1) = (
            (me.x - r).div_euclid(SLEEP_CELL),
            (me.x + r).div_euclid(SLEEP_CELL),
        );
        let (y0, y1) = (
            (me.y - r).div_euclid(SLEEP_CELL),
            (me.y + r).div_euclid(SLEEP_CELL),
        );
        (y0..=y1).flat_map(move |y| (x0..=x1).map(move |x| (x, y, me.z)))
    }

    fn claim_of(act: &Activity) -> Option<StructureId> {
        match act {
            Activity::Sleeping { bed: Some(id), .. } => Some(*id),
            Activity::Going { then, .. } => match **then {
                Activity::Sleeping { bed: Some(id), .. } => Some(id),
                _ => None,
            },
            _ => None,
        }
    }

    fn is_taken(&self, id: StructureId, by: AgentId) -> bool {
        self.claims
            .get(&id)
            .is_some_and(|claims| claims.iter().any(|who| *who != by))
    }

    fn replace_claim(&mut self, who: AgentId, old: Option<StructureId>, new: Option<StructureId>) {
        if old == new {
            return;
        }
        if let Some(id) = old {
            let empty = self
                .claims
                .get_mut(&id)
                .map(|claims| {
                    claims.remove(&who);
                    claims.is_empty()
                })
                .unwrap_or(false);
            if empty {
                self.claims.remove(&id);
            }
        }
        if let Some(id) = new {
            self.claims.entry(id).or_default().insert(who);
        }
    }
}

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    let seed = w.meta.seed;
    let n = w.agents.len();
    // Round robin: each tick starts where the last one's budget ran out, so everyone waiting
    // is served within a few ticks (a start that only crept forward one place a tick left
    // some waiting hundreds of ticks, and at 500 people some starved beside full stores).
    let start = if n == 0 { 0 } else { w.decide_cursor % n };
    let mut decided = 0usize;
    // The walks chosen this tick, searched together after the loop (#428): see `commit_later`.
    let mut walks: Vec<Walk> = Vec::new();
    // Days of food each community holds (#415), counted once in a tick and only when someone
    // decides in it.
    let mut stores: Option<Vec<(crate::ids::CommunityId, f32)>> = None;
    let mut sleeping_places: Option<SleepingPlaces> = None;
    for k in 0..n {
        let i = (start + k) % n;
        if !w.agents[i].alive {
            continue;
        }
        let a = &w.agents[i];
        if a.species != crate::agents::Species::Person {
            continue;
        }
        // Someone walking out of the valley for good is walked by society::leavers alone; a
        // goal chosen here sent a hurt raider wandering back inland from the edge every time
        // they paused there, so the edge check (every 200 ticks) never caught them.
        if a.leaving && a.community.is_none() {
            continue;
        }
        // In a beast's shape under the full moon they go where the hunger takes them
        // (systems/combat.rs, `shifted`), not where they would choose.
        if w.creatures.shifted.iter().any(|x| x.0 == a.id) {
            continue;
        }
        // In a hard cold they are freezing: warmth drains whenever the temperature sits
        // below their comfort, not only under 2 C, so an awake forager or stocker out in
        // it gets to re-decide before frostbite takes them, whatever they walk toward
        // (#467, FIXLOG 2026-10-29).
        let freezing = w.temp_at(a.pos) < a.species_def().climate.comfort_c[0] - a.hardened
            && !crate::systems::hazards::indoors(w, a.pos)
            && a.needs.warmth < 0.4;
        // Interrupt: critical hunger or exhaustion while doing something non-essential.
        let critical = (a.needs.hunger < 0.08
            && !matches!(
                a.activity,
                Activity::Eating { .. } | Activity::Foraging { .. }
            ))
            || (a.needs.thirst < 0.15 && !matches!(a.activity, Activity::Drinking { .. }))
            || (a.needs.rest < 0.05 && !a.activity.is_asleep())
            || (a.needs.warmth < 0.25 && a.activity.is_asleep())
            || freezing;
        // Wandering was left alone on the belief that when starving it is already the search
        // for food; one who wanders on past food they know of is not searching (#415), so a
        // starving wanderer who knows of food stops and goes to it.
        let interruptible = matches!(a.activity, Activity::Loitering { .. } | Activity::Talking { .. } | Activity::Idle)
            || (matches!(a.activity, Activity::Wandering { .. }) && knows_food(w, a))
            || matches!(&a.activity, Activity::Going { then, .. } if matches!(**then, Activity::Loitering { .. } | Activity::Talking { .. }))
            // A starving sleeper wakes once rested enough, if there is food to go to.
            || (a.activity.is_asleep() && a.needs.hunger < 0.05 && a.needs.rest > 0.4 && knows_food(w, a))
            // A parched sleeper wakes to drink once rested enough (#433).
            || (a.activity.is_asleep() && a.needs.thirst < 0.05 && a.needs.rest > 0.4)
            // A sleeper out in the cold wakes before it kills them.
            || (a.activity.is_asleep() && a.needs.warmth < 0.25)
            // A freezer turns back, but freezing alone must not restart a 300-tick meal at
            // the 200-tick rethink; walking to food remains interruptible (#467, FIXLOG
            // 2026-10-31).
            || (freezing && !a.activity.is_asleep() && !matches!(a.activity, Activity::Eating { .. }));
        if let Some(cv) = a.caravan {
            // Committed to a journey: keep walking to the gathering place, and once there
            // wait for the others (eat only when really hungry, drink when thirsty: #433, one
            // waiting at the gathering place died of thirst there).
            let dry = a.needs.thirst < 0.3;
            if a.needs.hunger > 0.3 && !dry && !matches!(a.activity, Activity::Idle) {
                continue;
            }
            let at_gathering = w
                .overworld
                .caravans
                .iter()
                .find(|c| c.id == cv)
                .map(|c| a.pos.dist(c.depart_tile) <= 2)
                .unwrap_or(false);
            if at_gathering && a.needs.hunger > 0.15 && !dry {
                continue;
            }
        }
        let idle = matches!(a.activity, Activity::Idle);
        let may_rethink = idle || (critical && interruptible && a.activity_ticks > RETHINK_TICKS);
        if !may_rethink {
            continue;
        }
        if idle && a.idle_ticks > 0 {
            w.agents[i].idle_ticks -= 1;
            continue;
        }
        if decided >= DECISIONS_PER_TICK {
            // The first one left waiting is where the next tick begins.
            w.decide_cursor = i;
            break;
        }
        decided += 1;
        let mut r = stream(seed, &[purpose::AGENT_DECISION, a.id.0 as u64, tick]);
        let a_id = a.id;
        let old_claim = SleepingPlaces::claim_of(&a.activity);
        let stores = stores.get_or_insert_with(|| {
            w.communities
                .iter()
                .filter(|c| !c.dissolved)
                .map(|c| (c.id, crate::systems::settlement::food_days(w, c.id)))
                .collect()
        });
        if crate::debuglog::enabled() && !idle {
            let a = &w.agents[i];
            let reason = format!("interrupted by {}", interrupted_by(a, freezing));
            crate::debuglog::abandoned(w, a.id.0, a.activity.name(), &reason);
        }
        // The Why? lens (#567): the traced person's cycle is written down as it is weighed.
        let mut cycle = (w.why_target() == Some(a_id)).then(|| {
            let a = &w.agents[i];
            crate::why::Cycle {
                tick,
                mode: if idle {
                    crate::why::Mode::Deliberate
                } else {
                    crate::why::Mode::Interrupted {
                        was: a.activity.describe(),
                        by: interrupted_by(a, freezing).to_string(),
                    }
                },
                conditions: Vec::new(),
                options: Vec::new(),
                chosen: String::new(),
                because: Vec::new(),
                put_off: Vec::new(),
            }
        });
        let goal = choose(w, i, &mut r, stores, &mut sleeping_places, cycle.as_mut());
        if let Some(g) = &goal {
            observe_knowing(w, i, g);
        }
        if let Some(mut c) = cycle {
            crate::why::explain(&mut c);
            put_off_work(w, i, &mut c);
            if let Some(t) = w.why.as_mut() {
                t.push(c);
            }
        }
        // Make memory-driven choices visible: a strong liking or dislike that shaped the pick.
        if let Some(Goal::EatItem(iid, _)) = &goal {
            if let Some(it) = w.items.iter().find(|it| it.id == *iid) {
                let a = &w.agents[i];
                let liking = a.memories.bias(MemKind::Meal, Subject::Food(it.kind));
                // Said once a day for each food, not at every meal (it was 30% of a 30-year
                // chronicle).
                let key = 0x7000_0000 | it.kind as u32;
                let said = a.work_cooldowns.get(&key).is_some_and(|t| *t > tick);
                if !(0.6..=1.4).contains(&liking) && !said {
                    w.agents[i].work_cooldowns.insert(key, tick + TICKS_PER_DAY);
                    let a = &w.agents[i];
                    // The item's words, not its variant (#215: "went for grainsheaf").
                    let what = format!("went for {}", it.kind.label());
                    let because = if liking > 1.4 {
                        "remembering good meals of it".to_string()
                    } else {
                        "though not fond of it".to_string()
                    };
                    let pos = a.pos;
                    let id = a.id;
                    w.events.push(
                        tick,
                        Some(pos),
                        Severity::Minor,
                        EventKind::Chose {
                            agent: id,
                            what,
                            because,
                        },
                    );
                }
            }
        }
        if let Some(Goal::Talk(who, _)) = &goal {
            let a = &w.agents[i];
            let op = a.opinion_of(*who);
            // Said once a day, not on every decision pass of the walk there (#212: a quarter
            // of a 300-day chronicle was this line, up to 32 times a day; once a friend a day
            // was still 38,000 rows in 600 days of a village of eighty).
            let key = 0x7100_0000;
            let said = a.work_cooldowns.get(&key).is_some_and(|t| *t > tick);
            if op > 0.35 && !said {
                let pos = a.pos;
                let id = a.id;
                w.agents[i].work_cooldowns.insert(key, tick + TICKS_PER_DAY);
                let name = w
                    .agents
                    .iter()
                    .find(|o| o.id == *who)
                    .map(|o| o.name.clone())
                    .unwrap_or_default();
                w.events.push(
                    tick,
                    Some(pos),
                    Severity::Minor,
                    EventKind::Chose {
                        agent: id,
                        what: format!("sought out {name}"),
                        because: "a good friend".into(),
                    },
                );
            }
        }
        if let Ok(name) = std::env::var("HF_TRACE_AGENT") {
            let a = &w.agents[i];
            if a.name == name {
                let foods = a
                    .knowledge
                    .map
                    .keys()
                    .filter(|k| matches!(k, Known::FoodPlant(_)))
                    .count();
                eprintln!("DECIDE {} tick {} hour {} pos {} hunger {:.2} rest {:.2} food_plants {} known {} prev {} -> {:?}", a.name, tick, (tick % 28800) / 1200, a.pos, a.needs.hunger, a.needs.rest, foods, a.knowledge.map.len(), a.activity.name(), goal);
            }
        }
        if let Some(Goal::Confront(raider, _)) = &goal {
            crate::threat::pursue(w, a_id, *raider);
            let pos = w.agents[i].pos;
            w.events.push(
                tick,
                Some(pos),
                Severity::Notable,
                EventKind::Chose {
                    agent: a_id,
                    what: "went after the strangers".to_string(),
                    because: confront_reason(&w.agents[i]),
                },
            );
        }
        if let Some(Goal::Welcome(band, _)) = &goal {
            crate::threat::send_envoy(w, a_id, *band);
            let pos = w.agents[i].pos;
            w.events.push(
                tick,
                Some(pos),
                Severity::Notable,
                EventKind::Chose {
                    agent: a_id,
                    what: "went out to the strangers' camp".to_string(),
                    because: crate::mind::beliefs::phrase(
                        crate::mind::beliefs::Topic::Strangers,
                        -1.0,
                    )
                    .to_string(),
                },
            );
        }
        let a2 = &mut w.agents[i];
        if let Some(Goal::Search(who, at)) = &goal {
            let church = crate::opening::Search {
                by: a_id,
                who: *who,
                at: *at,
                started: w.meta.tick,
                tries: 0,
            };
            if !w
                .searches
                .iter()
                .any(|s| s.by == church.by && s.who == church.who)
            {
                w.searches.push(church);
            }
            let name = w
                .survivor_pool
                .iter()
                .find(|s| s.index == *who)
                .map(|s| crate::opening::slot_name(w.meta.seed, *who, s.rerolls))
                .unwrap_or_else(|| "someone".to_string());
            w.events.push(
                w.meta.tick,
                Some(a2.pos),
                crate::events::Severity::Notable,
                crate::events::EventKind::Chose {
                    agent: a_id,
                    what: format!("set out to look for {name}"),
                    because: "word came they were seen that way".to_string(),
                },
            );
        }
        let a = &mut w.agents[i];
        let gate_walk = matches!(
            goal,
            Some(Goal::Seek(_) | Goal::SeekWater(_)) | Some(Goal::Stock(_)) | Some(Goal::Warm(_))
        );
        let next = match goal {
            Some(Goal::Search(_, at)) => Activity::Going {
                to: at,
                then: Box::new(Activity::Loitering { ticks_left: 120 }),
            },
            Some(Goal::EatItem(id, at)) => Activity::Going {
                to: at,
                then: Box::new(Activity::Eating {
                    item: id,
                    ticks_left: 15 * TICKS_PER_MINUTE as u32,
                }),
            },
            Some(Goal::Drink(at)) => Activity::Going {
                to: at,
                then: Box::new(Activity::Drinking {
                    ticks_left: 2 * TICKS_PER_MINUTE as u32,
                    // A walk off for water is a walk back after it (#433); not in the
                    // evening, when the hearth is where they gather after.
                    back: (a.pos.dist(at) > 4
                        && !crate::systems::culture::HEARTH_HOURS
                            .contains(&((tick % TICKS_PER_DAY) / TICKS_PER_HOUR)))
                    .then_some(a.pos),
                }),
            },
            Some(Goal::Forage(at)) | Some(Goal::Stock(at)) => Activity::Going {
                to: at,
                then: Box::new(Activity::Foraging {
                    plant: at,
                    ticks_left: 8 * TICKS_PER_MINUTE as u32,
                }),
            },
            Some(Goal::SleepBed(id, at)) => Activity::Going {
                to: at,
                then: Box::new(Activity::Sleeping {
                    bed: Some(id),
                    until: tick + (a.traits.sleep_hours() * TICKS_PER_HOUR as f32) as u64,
                }),
            },
            Some(Goal::SleepHere) => Activity::Sleeping {
                bed: None,
                until: tick + (a.traits.sleep_hours() * TICKS_PER_HOUR as f32) as u64,
            },
            Some(Goal::Talk(who, at)) => Activity::Going {
                to: at,
                then: Box::new(Activity::Talking {
                    with: who,
                    ticks_left: 6 * TICKS_PER_MINUTE as u32,
                }),
            },
            Some(Goal::Wander(to)) => Activity::Wandering { to },
            // A walk with a purpose reads as going somewhere, and they think again on arrival.
            Some(Goal::Seek(to) | Goal::SeekWater(to)) => Activity::Going {
                to,
                then: Box::new(Activity::Loitering {
                    ticks_left: TICKS_PER_MINUTE as u32,
                }),
            },
            Some(Goal::StudyThin(to)) => {
                if !w.thin.studying.iter().any(|(x, _)| *x == a_id) {
                    w.thin.studying.push((a_id, tick));
                }
                Activity::Going {
                    to,
                    then: Box::new(Activity::Loitering {
                        ticks_left: TICKS_PER_HOUR as u32,
                    }),
                }
            }
            Some(Goal::Gather(to)) => Activity::Going {
                to,
                then: Box::new(Activity::Loitering {
                    ticks_left: 30 * TICKS_PER_MINUTE as u32,
                }),
            },
            Some(Goal::Warm(to)) => Activity::Going {
                to,
                then: Box::new(Activity::Loitering {
                    ticks_left: TICKS_PER_HOUR as u32,
                }),
            },
            Some(Goal::Work(did)) => start_work(w, i, did, tick),
            Some(Goal::Respond(p, t)) => {
                let kind = match t {
                    crate::mind::Technique::CarryWater => crate::work::DesignationKind::Douse,
                    _ => crate::work::DesignationKind::BeatOut,
                };
                let mut d = crate::systems::settlement::make(w, kind, p, 9);
                d.owner = Some(a_id);
                d.community = w.agents[i].community;
                let did = d.id;
                w.designations.push(d);
                let because = {
                    let ag = &w.agents[i];
                    let b = ag.beliefs.get(crate::mind::beliefs::Topic::Fire);
                    match (ag.techniques.knows(t), b) {
                        (false, _) => "nobody else was doing anything".to_string(),
                        (true, Some(b)) if b.stance < 0.0 => format!(
                            "they knew how, and believe {}",
                            crate::mind::beliefs::phrase(b.topic, b.stance)
                        ),
                        _ => "they knew how".to_string(),
                    }
                };
                // Said once per fire, not each time they go back to it.
                let key = 0x7F1E_0000u32;
                let said = w.agents[i]
                    .work_cooldowns
                    .get(&key)
                    .is_some_and(|t| *t > tick);
                if !said {
                    let at = w.agents[i].pos;
                    w.events.push(
                        tick,
                        Some(at),
                        crate::events::Severity::Notable,
                        crate::events::EventKind::Chose {
                            agent: a_id,
                            what: format!("went to fight the fire by {}", t.name()),
                            because,
                        },
                    );
                }
                w.agents[i]
                    .work_cooldowns
                    .insert(key, tick + 2 * TICKS_PER_HOUR);
                start_work(w, i, did, tick)
            }
            Some(Goal::Avenge(beast, at)) => {
                let mut d = crate::systems::settlement::make(
                    w,
                    crate::work::DesignationKind::Hunt(beast),
                    at,
                    9,
                );
                d.requirements = vec![];
                d.owner = Some(a_id);
                d.community = w.agents[i].community;
                let did = d.id;
                w.designations.push(d);
                let (bname, hurt) = (
                    w.agents
                        .iter()
                        .find(|x| x.id == beast)
                        .map(|x| x.name.clone())
                        .unwrap_or_default(),
                    crate::creatures::wounds_by(w, beast),
                );
                let here = w.agents[i].pos;
                w.events.push(
                    tick,
                    Some(here),
                    Severity::Notable,
                    EventKind::Chose {
                        agent: a_id,
                        what: format!("went after {bname}"),
                        because: format!("it has hurt {hurt} people"),
                    },
                );
                start_work(w, i, did, tick)
            }
            Some(Goal::Study(item, at)) => {
                let mut d = crate::systems::settlement::make(
                    w,
                    crate::work::DesignationKind::Study(item),
                    at,
                    5,
                );
                d.owner = Some(a_id);
                d.community = w.agents[i].community;
                let did = d.id;
                w.designations.push(d);
                start_work(w, i, did, tick)
            }
            Some(Goal::Tend(pid, ppos)) => {
                let mut d = crate::systems::settlement::make(
                    w,
                    crate::work::DesignationKind::Tend(pid),
                    ppos,
                    8,
                );
                d.owner = Some(a_id);
                d.community = w.agents[i].community;
                let did = d.id;
                w.designations.push(d);
                start_work(w, i, did, tick)
            }
            Some(Goal::Flee(to)) => Activity::Going {
                to,
                then: Box::new(Activity::Loitering { ticks_left: 120 }),
            },
            Some(Goal::Confront(_, to)) => Activity::Going {
                to,
                then: Box::new(Activity::Loitering { ticks_left: 40 }),
            },
            Some(Goal::Welcome(_, to)) => Activity::Going {
                to,
                then: Box::new(Activity::Loitering { ticks_left: 200 }),
            },
            Some(Goal::Loiter) | None => Activity::Loitering {
                ticks_left: r.gen_range(3..12) * TICKS_PER_MINUTE as u32,
            },
        };
        commit_later(w, i, next, gate_walk, &mut walks);
        if let Some(places) = sleeping_places.as_mut() {
            let new_claim = SleepingPlaces::claim_of(&w.agents[i].activity);
            places.replace_claim(a_id, old_claim, new_claim);
        }
    }
    finish_walks(w, walks);
}

/// What cut into an activity for a rethink (#567): the same tests as `critical` in `step`.
fn interrupted_by(a: &crate::agents::Agent, freezing: bool) -> &'static str {
    if a.needs.hunger < 0.08 {
        "starving"
    } else if a.needs.thirst < 0.15 {
        "parched"
    } else if a.needs.rest < 0.05 {
        "exhausted"
    } else if a.needs.warmth < 0.25 && a.activity.is_asleep() {
        "cold in their sleep"
    } else if freezing {
        "freezing"
    } else {
        "a pressing need"
    }
}

/// Work this person gave up on for now and walks that found no way (#567), from their own
/// `work_cooldowns`: what they mean to come back to, and when.
fn put_off_work(w: &World, i: usize, c: &mut crate::why::Cycle) {
    let a = &w.agents[i];
    let now = w.meta.tick;
    for (&key, &until) in &a.work_cooldowns {
        if until <= now {
            continue;
        }
        let left = until - now;
        let when = format!(
            "{}h {:02}m from now",
            left / TICKS_PER_HOUR,
            (left % TICKS_PER_HOUR) / TICKS_PER_MINUTE
        );
        if key >> 28 == 0xB {
            let (x, y, z) = ((key >> 17) & 0x7FF, (key >> 6) & 0x7FF, key & 0x3F);
            c.put_off.push(format!(
                "found no way to ({x},{y},{z}); not trying it again for {when}"
            ));
        } else if key >> 28 == 0xD {
            let (x, y, z) = ((key >> 17) & 0x7FF, (key >> 6) & 0x7FF, key & 0x3F);
            c.put_off.push(format!(
                "found no drink standing at ({x},{y},{z}); not going there for water for {when}"
            ));
        } else if key < 0x1000_0000 {
            if let Some(d) = w.designations.iter().find(|d| d.id.0 == key && !d.done) {
                c.put_off.push(format!(
                    "gave up on {} for now; back to it {when}",
                    d.label()
                ));
            }
        }
    }
}

/// How long a walk that found no way is not chosen again (#553, FIXLOG 2026-11-01): an hour.
pub const NO_WAY_TICKS: u64 = TICKS_PER_HOUR;

/// The `work_cooldowns` key for "no way to this tile lately" (#553): the tile packed below
/// the high bits used by the other keys (0xB marks it).
pub fn no_way_key(p: TilePos) -> u32 {
    0xB000_0000 | ((p.x as u32 & 0x7FF) << 17) | ((p.y as u32 & 0x7FF) << 6) | (p.z as u32 & 0x3F)
}

/// A walk committed to in this tick's decisions, its path not yet searched.
struct Walk {
    agent: usize,
    from: TilePos,
    to: TilePos,
    /// A walk the #415 gate ordered: not made across wet ground (`wet`).
    gate: bool,
}

/// `commit`, with the path search left to `finish_walks` (#428, 2026-10-27): the activity is
/// taken up at once, as `commit` does, and the walk is queued in decision order. A path
/// depends only on the grid and the two ends, and nothing in the decision loop changes the
/// grid or where anyone stands, so the searches can run together, across threads, after it.
/// The one difference from `commit`: one who cannot find the way is seen by those deciding
/// after them in the same tick as setting off, until `finish_walks` sets them loitering.
fn commit_later(w: &mut World, i: usize, next: Activity, gate: bool, walks: &mut Vec<Walk>) {
    let target = match &next {
        Activity::Going { to, .. } | Activity::Wandering { to } => Some(*to),
        _ => None,
    };
    let from = w.agents[i].pos;
    match target {
        Some(to) if to != from => {
            walks.push(Walk {
                agent: i,
                from,
                to,
                gate,
            });
            let a = &mut w.agents[i];
            a.activity = next;
            a.activity_ticks = 0;
            a.switches += 1;
        }
        _ => commit(w, i, next),
    }
}

/// Search the queued walks' paths in parallel and apply them in the order they were queued:
/// a path found is followed; a walk with no way through becomes loitering, as in `commit`.
fn finish_walks(w: &mut World, walks: Vec<Walk>) {
    if walks.is_empty() {
        return;
    }
    w.ensure_nav();
    let requests: Vec<(TilePos, TilePos)> = walks.iter().map(|k| (k.from, k.to)).collect();
    let found = crate::par::paths(w, &requests);
    for (walk, path) in walks.into_iter().zip(found) {
        // Safety before everything the gate orders (#415, FIXLOG 2026-10-28): a walk back to
        // camp, under a roof or out to gather for the store is not made across water or flooding
        // ground; they stay where they are instead (seed 1: walking back to camp, eight drowned
        // crossing a spate between them and it).
        let path = path.filter(|p| !(walk.gate && p.iter().any(|t| wet(w, *t))));
        let a = &mut w.agents[walk.agent];
        match path {
            Some(p) => {
                a.path = p;
                a.path_blocked = false;
            }
            None => {
                if crate::debuglog::enabled() {
                    crate::debuglog::abandoned(
                        w,
                        w.agents[walk.agent].id.0,
                        w.agents[walk.agent].activity.name(),
                        "no path",
                    );
                }
                let a = &mut w.agents[walk.agent];
                a.activity = Activity::Loitering { ticks_left: 40 };
                // No way there now (the water over their head between, #553): not the same
                // walk again at the next thought, but after a while, as a normal person waits
                // for the water to go down or goes elsewhere. See `NO_WAY_TICKS`.
                let until = w.meta.tick + NO_WAY_TICKS;
                w.agents[walk.agent]
                    .work_cooldowns
                    .insert(no_way_key(walk.to), until);
            }
        }
    }
}

/// Back to camp is to its edge on the side they come from, `r` tiles out from its heart, not to
/// the hearthstone (#415): a walk back while unsecured is not an evening at the hearth.
fn edge_of_camp(w: &World, home: TilePos, me: TilePos, r: i32) -> TilePos {
    let d = me.dist(home).max(1);
    let k = r.min(d);
    let p = w.grid.surface(
        home.x + (me.x - home.x) * k / d,
        home.y + (me.y - home.y) * k / d,
    );
    if w.grid.walkable(p) {
        p
    } else {
        home
    }
}

/// Water on the way that is not safe ground (#415): a tile under water, or in a rising flood's
/// way (hazards.rs `in_floods_way`, #418). A walk the gate orders does not cross it; a shallow
/// stream in its banks it may (seed 4: refusing those left four out in the cold, re-choosing a
/// walk to the hearth and refused each time, until they froze).
pub fn wet(w: &World, p: TilePos) -> bool {
    crate::systems::hazards::under_water(w, p) || crate::systems::hazards::in_floods_way(w, p)
}

/// Ground the #415 gate never sends anyone onto: water, a tile under water, or, while any of
/// the valley's waters is in spate, ground a flood has covered before (the high-water line).
pub fn unsafe_ground(w: &World, p: TilePos) -> bool {
    w.grid.get(p).is_water()
        || wet(w, p)
        || (w.waters.bodies.iter().any(|b| b.high) && w.waters.flooded_once(p.x, p.y))
}

fn confront_reason(a: &crate::agents::Agent) -> String {
    use crate::mind::beliefs::{phrase, Topic};
    let v = a.beliefs.lean(Topic::Violence);
    let s = a.beliefs.lean(Topic::Strangers);
    if v > s.abs() {
        phrase(Topic::Violence, 1.0).to_string()
    } else if s > 0.0 {
        phrase(Topic::Strangers, 1.0).to_string()
    } else {
        "the stores are ours".to_string()
    }
}

/// Looking for a necessity can reveal it. Only a known destination offers relief on arrival.
fn search_urge(pressure: f32) -> f32 {
    0.4 + pressure * 2.0
}

/// Without a water destination, go look. Sample the visible ground around the person
/// without consulting the water map; a later look can reveal a source.
fn search_for_water(w: &World, i: usize, r: &mut rand_chacha::ChaCha8Rng) -> Option<TilePos> {
    let a = &w.agents[i];
    let nav = w.nav.as_ref()?;
    let radius = super::perceive::sight_radius(w, a, w.meta.tick);
    // Water often collects lower down. Follow a descent at their feet before
    // looking farther along this floor. Only step through an open lip or a
    // stair they are standing on, never through a ceiling to a hidden tile.
    let mut steps = Vec::new();
    crate::nav::neighbours(&w.grid, a.pos, &mut steps);
    if let Some(step) = steps.iter().find(|step| {
        let to = step.to;
        if to.z >= a.pos.z {
            return false;
        }
        let cover = w.grid.get(to.offset(0, 0, 1));
        (cover.shape.connects_down()
            || (matches!(
                cover.shape,
                crate::world::Shape::Open | crate::world::Shape::RampTop
            ) && !cover.is_water()
                && !crate::building::roofed(w, to)))
            && !unsafe_ground(w, to)
    }) {
        return Some(step.to);
    }
    (0..8).find_map(|_| {
        let p = a.pos.offset(
            r.gen_range(-radius..=radius),
            r.gen_range(-radius..=radius),
            0,
        );
        (p != a.pos
            && w.grid.in_bounds(p)
            && w.grid.walkable(p)
            && crate::mind::perception::line_of_sight(&w.grid, a.pos, p)
            && nav.reach(a.pos, p))
        .then_some(p)
    })
}

/// One of a band (threat.rs): they keep to their camp, eat what they carry when they must, and
/// sleep there. Raiding and leaving are given to them by the band, not chosen here.
fn choose_band(w: &World, i: usize, r: &mut rand_chacha::ChaCha8Rng) -> Option<Goal> {
    let a = &w.agents[i];
    let me = a.pos;
    let band = crate::threat::band_of(w, a.id)?;
    let camp = band.camp;
    // Out after food: the band sends them on again at its next step.
    if band.raiders.iter().any(|x| x.0 == a.id) {
        return Some(Goal::Loiter);
    }
    // Thirsty: the nearest water they can walk to (#433), as anyone on the road drinks —
    // chosen by what they know of it (#453 part 4, FIXLOG 2026-10-30): salt never, foul only
    // when parched past desperation.
    if a.needs.thirst < 0.45 {
        let desperate = 1.0 - a.needs.thirst > 0.7;
        if let Some(s) = crate::water::nearest_knowing(w, i, me, 60, &a.knowledge, desperate) {
            return Some(Goal::Drink(s.stand));
        }
    }
    // Hungry with food in hand: eat it. (Foraging again while carrying food piled up stacks a
    // carnivore could not live on: a Skarri band member carried 1,300 of berries, the C43 soak.)
    let carried = w
        .items
        .iter()
        .find(|it| it.kind.is_food() && it.place == ItemPlace::Carried(a.id));
    if 1.0 - a.needs.hunger > 0.55 {
        if let Some(it) = carried {
            return Some(Goal::EatItem(it.id, me));
        }
    }
    // A walk to look for water can wait for food in hand or a tired camper's rest.
    if a.needs.thirst < 0.45
        && a.needs.rest >= 0.3
        && matches!(band.state, crate::threat::BandState::Camped(_))
    {
        if let Some(p) = search_for_water(w, i, r) {
            return Some(Goal::SeekWater(p));
        }
    }
    // Hungry and nothing in hand: forage what they have seen growing near the camp, as anyone
    // on the road would.
    if 1.0 - a.needs.hunger > 0.55 {
        let nav = w.nav.as_ref()?;
        if let Some(p) = a
            .knowledge
            .iter_kind(|k| matches!(k, Known::FoodPlant(_)))
            .filter_map(|(k, _)| match k {
                Known::FoodPlant(p) => Some(*p),
                _ => None,
            })
            .filter(|p| p.dist(camp) <= 25 && nav.reach(me, *p))
            .min_by_key(|p| p.dist(me))
        {
            return Some(Goal::Forage(p));
        }
        // Nothing known growing near the camp: look round it for some (#415), not sit by the
        // fire hungry.
        if matches!(band.state, crate::threat::BandState::Camped(_)) {
            for _ in 0..4 {
                let to = w.grid.surface(
                    (camp.x + r.gen_range(-12..=12)).clamp(1, w.grid.width - 2),
                    (camp.y + r.gen_range(-12..=12)).clamp(1, w.grid.height - 2),
                );
                if w.grid.walkable(to) && nav.reach(me, to) {
                    return Some(Goal::Seek(to));
                }
            }
        }
    }
    if !matches!(band.state, crate::threat::BandState::Camped(_)) {
        return Some(Goal::Loiter);
    }
    if me.dist(camp) > 5 {
        return Some(Goal::Wander(camp));
    }
    if Calendar::rest_time(w.meta.tick, a.species_def().nocturnal()) || a.needs.rest < 0.3 {
        return Some(Goal::SleepHere);
    }
    if r.gen_bool(0.4) {
        let to = w.grid.surface(
            (camp.x + r.gen_range(-4..=4)).clamp(1, w.grid.width - 2),
            (camp.y + r.gen_range(-4..=4)).clamp(1, w.grid.height - 2),
        );
        if w.grid.walkable(to) {
            return Some(Goal::Wander(to));
        }
    }
    Some(Goal::Loiter)
}

/// Strangers camped near the settlement: someone may go after a raider, or go out to tell them
/// they may eat. What they hold about strangers, force and sharing in hunger weights it; nobody
/// is sent.
fn consider_bands(w: &World, i: usize, consider: &mut impl FnMut(f32, Goal)) {
    use crate::mind::beliefs::Topic;
    let a = &w.agents[i];
    let me = a.pos;
    let old_enough = if a.people == crate::species::SpeciesId::HUMAN {
        a.age_years(w.meta.tick) >= 14
    } else {
        !a.is_child(w.meta.tick)
    };
    if a.community.is_none() || !old_enough || a.body.needs_care() {
        return;
    }
    for b in w.bands.iter().filter(|b| b.sighted) {
        let crate::threat::BandState::Camped(_) = b.state else {
            continue;
        };
        let rested = a
            .work_cooldowns
            .get(&crate::threat::confront_key(b.id))
            .map(|t| *t <= w.meta.tick)
            .unwrap_or(true);
        let chasing = b.pursuits.iter().any(|p| p.0 == a.id);
        // Strangers: a positive stance is "strangers bring harm". Violence: positive is "a fist
        // settles things". Once the band has been told it may eat, only the hardest go after it.
        let hostile = if !rested || chasing {
            0.0
        } else if b.welcomed {
            if a.beliefs.lean(Topic::Strangers) > 0.3 {
                0.5 * a.beliefs.weight(Topic::Violence, 1.0)
            } else {
                0.0
            }
        } else {
            a.beliefs.weight(Topic::Strangers, 1.0) * a.beliefs.weight(Topic::Violence, 1.0)
        };
        // A raider out among the stores, close enough to see.
        let out = b
            .raiders
            .iter()
            .filter_map(|(r, _)| w.agents.iter().find(|x| x.id == *r && x.alive))
            .filter(|x| x.pos.z == me.z && x.pos.dist(me) <= 14)
            .min_by_key(|x| x.pos.dist(me));
        if let Some(x) = out {
            consider(1.6 * hostile, Goal::Confront(x.id, x.pos));
        } else if b.taken > 0 && !b.welcomed && a.beliefs.lean(Topic::Violence) > 0.15 {
            // They have taken from us: someone who believes force settles it goes after one
            // they themselves know of — in sight now, or where they remember them being; the
            // band-level `sighted` flag is anyone's glance, not theirs, and the true position
            // is nobody's knowledge (#173; FIXLOG 2026-10-28 fix-173-pursuit-sight).
            if let Some((id, at)) = b
                .members
                .iter()
                .filter_map(|m| {
                    let x = w.agents.iter().find(|x| x.id == *m && x.alive)?;
                    let at = if crate::mind::perception::can_see(&w.grid, me, x.pos) {
                        x.pos
                    } else {
                        a.knowledge.get(Known::Agent(*m))?.pos
                    };
                    (at.dist(me) <= 40).then_some((x.id, at))
                })
                .min_by_key(|(_, at)| at.dist(me))
            {
                consider(0.9 * hostile, Goal::Confront(id, at));
            }
        }
        if !b.welcomed
            && !b.envoys.contains(&a.id)
            && a.beliefs.lean(Topic::Strangers) < -0.1
            && me.dist(b.camp) <= 45
        {
            // "Strangers are welcome" and "in hunger you share" are the negative stances.
            let kind =
                a.beliefs.weight(Topic::Strangers, -1.0) * a.beliefs.weight(Topic::Hunger, -1.0);
            consider(0.75 * kind, Goal::Welcome(b.id, b.camp));
        }
    }
}

fn knows_food(w: &World, a: &crate::agents::Agent) -> bool {
    a.knowledge
        .map
        .keys()
        .any(|k| matches!(k, Known::FoodPlant(_)))
        || w.items.iter().any(|it| {
            it.kind.is_food()
                && (matches!(it.place, ItemPlace::Carried(id) if id == a.id)
                    || a.knowledge.get(Known::Item(it.id)).is_some())
        })
}

/// Plan and lease a designation for agent `i`, or leave it for a while if that fails.
fn start_work(w: &mut World, i: usize, did: crate::work::DesignationId, tick: u64) -> Activity {
    let a_id = w.agents[i].id;
    let planned = w
        .designations
        .iter()
        .find(|d| d.id == did)
        .and_then(|d| crate::work::planner::plan_for(w, a_id, d));
    // (Planned, but someone else holds the station or the stack: busy, not failed.)
    let mut busy = false;
    let acquired = match planned {
        Some(p) => {
            let expires = tick + p.estimate as u64 + 600;
            if w.reservations.acquire_all(a_id, &p.keys, expires) {
                Some(p.plan)
            } else {
                busy = true;
                None
            }
        }
        None => None,
    };
    match acquired {
        Some(plan) => Activity::Working { plan },
        None => {
            // Could not plan or lease it: leave it alone for a while. Only a plan that could
            // not be made counts against the order: one kiln and two orders at it is a wait,
            // and counting each wait as a failure buried the second order (C39 review).
            w.agents[i]
                .work_cooldowns
                .insert(did.0, tick + TICKS_PER_HOUR / 4);
            if !busy {
                if let Some(d) = w.designations.iter_mut().find(|d| d.id == did) {
                    d.failures += 1;
                }
            }
            // An undertaking of their own that they cannot reach is given up, not left; a work
            // in a fey mood is the mood's to end (systems/moods.rs).
            if !busy && !crate::systems::moods::is_fey_work(w, did) {
                w.designations
                    .retain(|d| !(d.id == did && d.owner == Some(a_id)));
            }
            Activity::Loitering { ticks_left: 40 }
        }
    }
}

/// Commit to an activity: plan the path for Going/Wandering, or fall back to loitering.
pub fn commit(w: &mut World, i: usize, next: Activity) {
    let target = match &next {
        Activity::Going { to, .. } => Some(*to),
        Activity::Wandering { to } => Some(*to),
        _ => None,
    };
    if let Some(to) = target {
        if w.agents[i].pos != to && !w.path_agent_to(i, to) {
            if crate::debuglog::enabled() {
                crate::debuglog::abandoned(w, w.agents[i].id.0, next.name(), "no path");
            }
            let a = &mut w.agents[i];
            a.activity = Activity::Loitering { ticks_left: 40 };
            a.activity_ticks = 0;
            a.switches += 1;
            return;
        }
    }
    let a = &mut w.agents[i];
    a.activity = next;
    a.activity_ticks = 0;
    a.switches += 1;
}

/// How far a survivor is from wanting that person's company: 0 seeks them out, 1 keeps away.
/// Combines what they remember about them with what they currently think of them.
fn aversion_flag(a: &crate::agents::Agent, who: crate::ids::AgentId) -> f32 {
    let thinks = -a.opinion_of(who);
    thinks.clamp(0.0, 1.0) * 0.5
}

/// What is not secured for a person (#415, rule 7): their own hunger, their community's
/// stores for tomorrow (under `SECURE_DAYS`, from `days`), tonight's shelter (night two hours
/// off or come, no bed known, no roof overhead).
pub struct Secured {
    /// Water before food (#433): thirst half run down; a drink goal outscores eating at the
    /// same need in `choose`.
    pub thirsty: bool,
    pub hungry: bool,
    pub short: bool,
    pub roofless: bool,
}

impl Secured {
    pub fn all(&self) -> bool {
        !(self.thirsty || self.hungry || self.short || self.roofless)
    }
}

/// `days`: the days of food their community holds, if it has one (`settlement::food_days`).
pub fn secured(w: &World, a: &crate::agents::Agent, days: Option<f32>) -> Secured {
    let now = w.meta.tick;
    let nocturnal = a.species_def().nocturnal();
    let night = Calendar::rest_time(now, nocturnal);
    // A child's stores are their parents' to see to.
    let short = !a.is_child(now) && days.is_some_and(|d| d < SECURE_DAYS);
    // Night is near: from two hours before their rest time until it ends.
    let dusk = night || Calendar::rest_time(now + 2 * TICKS_PER_HOUR, nocturnal);
    let knows_bed = a
        .knowledge
        .iter_kind(|k| matches!(k, Known::Structure(_)))
        .any(|(_, b)| tag_is_sleeping_place(b.tag));
    Secured {
        thirsty: a.needs.thirst < 0.5,
        hungry: a.needs.hunger < 0.5,
        short,
        // A roof over flooding ground is no shelter (#415).
        roofless: dusk
            && !knows_bed
            && (!crate::systems::hazards::indoors(w, a.pos) || unsafe_ground(w, a.pos)),
    }
}

/// Whether everything `secured` asks is secured for agent `i`, counting their community's
/// stores now: for systems outside the decision that call people to leisure (#415).
pub fn is_secured(w: &World, i: usize) -> bool {
    let a = &w.agents[i];
    let days = a
        .community
        .map(|c| crate::systems::settlement::food_days(w, c));
    secured(w, a, days).all()
}

/// How low warmth may fall before the Warm goal opens, in the cold they are in (#467):
/// the old flat 0.4 near their comfort, opening a fifth of a warmth-bar sooner for every
/// 12 C the cold runs below it, capped at 0.6 — in a killing cold they keep much closer
/// to the fire.
fn warm_gate(w: &World, a: &crate::agents::Agent) -> f32 {
    let hard = (a.species_def().climate.comfort_c[0] - a.hardened - w.temp_at(a.pos)).max(0.0);
    (0.4 + (hard / 12.0).min(1.0) * 0.2).min(0.6)
}

/// The food plants and ground food that others are already set on (#553, FIXLOG 2026-11-01):
/// a plant someone is walking to or picking, and how many are going to eat from each stack.
/// Only those a person would know of: their own community, who say where they are off to, and
/// anyone in plain view near them on their level. A normal person does not walk to the bush
/// a neighbour is already picking; on seed 3, 1,315 of 4,124 "nothing to gather" failures in
/// 50 days were a second picker arriving at a bush another had reached first.
/// The census behind #940: record whether the target of a just-chosen Talk, Tend, Forage or
/// EatItem was in the chooser's line of sight at this tick. For Forage and EatItem the "target"
/// is whoever already claimed the plant or item (`spoken_for`); with no claimer there is
/// nothing to see or not see, so nothing is counted.
fn observe_knowing(w: &mut World, i: usize, goal: &Goal) {
    let me = w.agents[i].pos;
    let (watched, target) = match goal {
        Goal::Talk(_, at) => (crate::view::knowing::Watched::Talk, Some(*at)),
        Goal::Tend(_, at) => (crate::view::knowing::Watched::Tend, Some(*at)),
        Goal::Forage(at) => {
            let (plants, _) = spoken_for(w, i);
            (
                crate::view::knowing::Watched::Forage,
                plants
                    .contains(at)
                    .then_some(claimer_pos(w, i, *at, true))
                    .flatten(),
            )
        }
        Goal::EatItem(id, at) => {
            let (_, items) = spoken_for(w, i);
            (
                crate::view::knowing::Watched::EatItem,
                items
                    .contains_key(id)
                    .then_some(claimer_pos(w, i, *at, false))
                    .flatten(),
            )
        }
        _ => return,
    };
    let Some(target) = target else { return };
    let seen = crate::mind::perception::can_see(&w.grid, me, target);
    w.knowing.record(watched, seen);
}

/// Whoever else, known to `i`, is walking to or already picking the plant at `at` (or, for an
/// item, foraging anywhere and about to eat): their current position, the same test
/// `spoken_for` used to find the claim.
fn claimer_pos(w: &World, i: usize, at: TilePos, plant: bool) -> Option<TilePos> {
    let a = &w.agents[i];
    w.agents.iter().enumerate().find_map(|(j, o)| {
        if j == i || !o.alive || o.species != crate::agents::Species::Person {
            return None;
        }
        let known = (a.community.is_some() && o.community == a.community)
            || (o.pos.z == a.pos.z && o.pos.dist(a.pos) <= 8);
        if !known {
            return None;
        }
        let act = match &o.activity {
            Activity::Going { then, .. } => &**then,
            other => other,
        };
        match act {
            Activity::Foraging { plant: p, .. } if plant && *p == at => Some(o.pos),
            Activity::Eating { .. } if !plant => Some(o.pos),
            _ => None,
        }
    })
}

fn spoken_for(
    w: &World,
    i: usize,
) -> (
    std::collections::BTreeSet<TilePos>,
    std::collections::BTreeMap<crate::ids::ItemId, u32>,
) {
    let a = &w.agents[i];
    let mut plants = std::collections::BTreeSet::new();
    let mut items: std::collections::BTreeMap<crate::ids::ItemId, u32> =
        std::collections::BTreeMap::new();
    for (j, o) in w.agents.iter().enumerate() {
        if j == i || !o.alive || o.species != crate::agents::Species::Person {
            continue;
        }
        let known = (a.community.is_some() && o.community == a.community)
            || (o.pos.z == a.pos.z && o.pos.dist(a.pos) <= 8);
        if !known {
            continue;
        }
        let act = match &o.activity {
            Activity::Going { then, .. } => &**then,
            other => other,
        };
        match act {
            Activity::Foraging { plant, .. } => {
                plants.insert(*plant);
            }
            Activity::Eating { item, .. } => *items.entry(*item).or_default() += 1,
            _ => {}
        }
    }
    (plants, items)
}

fn choose(
    w: &World,
    i: usize,
    r: &mut rand_chacha::ChaCha8Rng,
    stores: &[(crate::ids::CommunityId, f32)],
    sleeping_places: &mut Option<SleepingPlaces>,
    mut trace: Option<&mut crate::why::Cycle>,
) -> Option<Goal> {
    if w.agents[i].band.is_some() {
        let g = choose_band(w, i, r);
        if crate::debuglog::enabled() {
            let chose = format!("{g:?}");
            let opts = vec![(chose.clone(), 0.0, true)];
            crate::debuglog::decision(w, w.agents[i].id.0, &chose, opts);
        }
        return g;
    }
    let a = &w.agents[i];
    let me = a.pos;
    let now = w.meta.tick;
    let nav = w.nav.as_ref()?;
    let hunger_p = 1.0 - a.needs.hunger;
    let rest_p = 1.0 - a.needs.rest;
    let social_p = 1.0 - a.needs.social;
    let night = Calendar::rest_time(w.meta.tick, a.species_def().nocturnal());
    // What would a normal person do here? (#415, rule 7, D56, FIXLOG 2026-10-27.) Nobody
    // wanders, loiters, talks or sits about while today's food, tomorrow's food or tonight's
    // shelter is not secured: leisure's weight falls to near nothing until it is, and the
    // walking left is a search with a direction (`Goal::Seek`), toward food or back to the group.
    let young = a.is_child(now);
    let hungry = hunger_p > 0.5;
    let days = a
        .community
        .and_then(|c| stores.iter().find(|s| s.0 == c))
        .map(|s| s.1);
    let Secured {
        thirsty,
        short,
        roofless,
        ..
    } = secured(w, a, days);
    let unsecured = thirsty || hungry || short || roofless;
    // Fresh through the portal, food, warmth and shelter come first (#392, QA 2026-10-26):
    // 1.5 on the day of the arrival, back to 1 as its shock fades; the survival-snap pole
    // runs a third stronger again (#395). The same urge divides leisure down below.
    let survive = crate::systems::strain::survival_urge(a, now);
    // While the arrival shock is fresh, the secured put leisure off too (#395, FIXLOG
    // 2026-11-01): the same urge that weights survival work up divides leisure down,
    // fading back to 1 as the shock does. The unsecured gate stays the stronger of the
    // two.
    let leisure = if unsecured {
        LEISURE_UNSECURED
    } else {
        1.0 / survive
    };
    let mut best: Option<(f32, Goal)> = None;
    // A goal that means walking somewhere is only a choice if the walk can be made: commit
    // turns an unreachable target into a short loiter, and the same choice comes back after
    // it, so a goal that skipped the check could hold someone in place for days. Some goals
    // check for themselves (eating does); this is where every goal passes, so none can miss it.
    let unreachable = |g: &Goal| -> Option<&'static str> {
        match g {
            Goal::Search(_, t)
            | Goal::EatItem(_, t)
            | Goal::Forage(t)
            | Goal::SleepBed(_, t)
            | Goal::Talk(_, t)
            | Goal::Wander(t)
            | Goal::Seek(t)
            | Goal::SeekWater(t)
            | Goal::Stock(t)
            | Goal::Warm(t)
            | Goal::Flee(t)
            | Goal::Confront(_, t)
            | Goal::Welcome(_, t)
            | Goal::Gather(t)
            | Goal::Tend(_, t)
            | Goal::Drink(t)
            | Goal::StudyThin(t) => {
                if !(*t == me || nav.reach(me, *t)) {
                    Some("no way there they know of")
                } else if crate::systems::hazards::in_floods_way(w, *t) {
                    Some("in the flood's way")
                } else if a
                    .work_cooldowns
                    .get(&no_way_key(*t))
                    .is_some_and(|until| *until > now)
                {
                    Some("found no way there lately")
                } else {
                    None
                }
            }
            _ => None,
        }
    };
    let reachable = |g: &Goal| unreachable(g).is_none();
    // Feelings lean the choice, never make it (docs/design-feelings.md step 3): each a
    // multiplier between 0.7 and 1.4 on what the goal already scores.
    let feels = {
        use crate::mind::feelings::FeelingKind as F;
        let f = &a.feelings;
        [
            f.of(F::Fear),
            f.of(F::Grief),
            f.of(F::Anger),
            f.of(F::Joy),
            f.of(F::Awe),
            f.of(F::Shame),
        ]
    };
    let lean = |g: &Goal| -> f32 {
        let [fear, grief, anger, joy, awe, shame] = feels;
        let m = match g {
            Goal::Flee { .. } => 1.0 + 0.4 * fear,
            Goal::Confront { .. } | Goal::Avenge { .. } => 1.0 + 0.4 * anger - 0.3 * fear,
            Goal::Talk { .. } => 1.0 + 0.3 * joy - 0.3 * grief - 0.3 * shame,
            Goal::SleepBed { .. } | Goal::Warm { .. } => 1.0 + 0.2 * grief + 0.1 * fear,
            Goal::StudyThin { .. } | Goal::Study { .. } => 1.0 + 0.4 * awe - 0.2 * fear,
            Goal::Wander { .. } => 1.0 - 0.3 * fear + 0.2 * shame,
            Goal::Work { .. } => 1.0 - 0.15 * grief - 0.15 * shame,
            _ => 1.0,
        };
        m.clamp(0.7, 1.4)
    };
    let gate = |g: &Goal| match g {
        Goal::Talk(..)
        | Goal::Wander(_)
        | Goal::Loiter
        | Goal::Gather(_)
        | Goal::StudyThin(_)
        | Goal::Study(..) => leisure,
        _ => 1.0,
    };
    // Food others are already set on is not a choice (#553): see `spoken_for`.
    let (picked, eaten) = spoken_for(w, i);
    let free = |g: &Goal| match g {
        Goal::Forage(p) | Goal::Stock(p) => !picked.contains(p),
        // Nor a stack someone's work has claimed (a hauler is coming for it): it is carried
        // off before they get there and the meal fails as "food moved".
        Goal::EatItem(id, at)
            if *at != me
                && !w
                    .reservations
                    .is_free_for(crate::work::ResKey::Item(*id), a.id) =>
        {
            false
        }
        Goal::EatItem(id, _) => eaten.get(id).is_none_or(|n| {
            w.items
                .iter()
                .find(|it| it.id == *id)
                .is_some_and(|it| matches!(it.place, ItemPlace::Carried(_)) || it.count > *n)
        }),
        _ => true,
    };
    // The Why? lens (#567): with this person traced, every option is written down with its
    // terms and, when it was not a choice, why not. The checks below are pure, so the choice
    // is the same traced or not.
    let mut best_at: Option<usize> = None;
    // What judging a quarrel afterwards taught them (#568, mind/conscience.rs): a regretted
    // quarrel makes them readier to tend and seek out that person, a rationalised one slower;
    // bounded and fading, 1 for everyone else.
    let learned = |g: &Goal| match g {
        Goal::Tend(id, _) | Goal::Talk(id, _) => a.conscience.help_toward(*id, now),
        _ => 1.0,
    };
    // What their mood does to the weight of an option (#885): the Why? lens's slot for the
    // inner-life lean that #886 onward fills. 1 for every goal for now, so no score changes.
    let spirits = |_g: &Goal| -> f32 { 1.0 };
    // What a disposition does to the weight of an option (#906): the Why? lens's slot for the
    // terms that later slices move out of `base` (`talk_weight`) one at a time.
    // Loiter's floor is the first (#907): the hardest worker now stands about less.
    // Work's weight is the next (#908): withdrawn since a breakdown still zeroes it.
    let work_w = if a.withdrawn_until > now {
        0.0
    } else {
        a.work_weight()
    };
    let temperament = |g: &Goal| -> f32 {
        match g {
            Goal::Loiter => a.loiter_weight(),
            // #1760: a cheerful person is the one who stops for a word.
            Goal::Talk(..) => a.cheer_weight(),
            // #1758: when the stores run short, the hard workers are the first out with a
            // basket.
            Goal::Stock(_) => a.stock_weight(),
            // #1759: the self-reliant sit at the hearth less and wander more; both are
            // leisure goals, survival goals are untouched.
            Goal::Gather(_) => 1.2 - 0.4 * a.independence(),
            Goal::Wander(_) => 0.8 + 0.4 * a.independence(),
            // #1761 on top of #908: the orderly keep the stores and the pot; a haul or a cook
            // order weighs more the tidier they are, times Work's own weight.
            Goal::Work(did) => {
                let order = match w.designations.iter().find(|d| d.id == *did) {
                    Some(d)
                        if matches!(
                            d.kind,
                            crate::work::DesignationKind::Haul | crate::work::DesignationKind::Cook
                        ) =>
                    {
                        a.order_weight()
                    }
                    _ => 1.0,
                };
                work_w * order
            }
            _ => 1.0,
        }
    };
    // Every option weighed, for the debug log (#565); None, and so free, when it is off.
    let mut log: Option<Vec<(f32, bool, String)>> = crate::debuglog::enabled().then(Vec::new);
    let mut consider = |base: f32, g: Goal| {
        let (emotion, held, taught, mood, disposition) = (
            lean(&g),
            gate(&g),
            learned(&g),
            spirits(&g),
            temperament(&g),
        );
        let score = base * emotion * held * taught * mood * disposition;
        // Nor is the gate's ground under water or flooding (#415, safety first).
        let wet = match &g {
            Goal::Seek(t) | Goal::SeekWater(t) | Goal::Stock(t) | Goal::Warm(t) => {
                unsafe_ground(w, *t)
            }
            _ => false,
        };
        if let Some(c) = trace.as_deref_mut() {
            let rejected = unreachable(&g)
                .or(wet.then_some("the ground there is under water"))
                .or((!free(&g)).then_some("someone else is already at it"))
                .or((score <= 0.0).then_some(
                    if matches!(g, Goal::Work(_)) && a.withdrawn_until > now {
                        "off work since they broke down"
                    } else {
                        "weighs nothing now"
                    },
                ));
            let (cycle, what) = goal_words(w, &g);
            if rejected.is_none() && best.as_ref().map(|b| score > b.0).unwrap_or(true) {
                best_at = Some(c.options.len());
            }
            c.options.push(crate::why::Considered {
                cycle,
                what,
                base,
                emotion,
                gate: held,
                learned: taught,
                spirits: mood,
                temperament: disposition,
                score,
                rejected,
                chosen: false,
            });
        }
        if score > 0.0
            && best.as_ref().map(|b| score > b.0).unwrap_or(true)
            && reachable(&g)
            && !wet
            && free(&g)
        {
            if let Some(t) = log.as_mut() {
                t.push((score, true, format!("{g:?}")));
            }
            best = Some((score, g));
        } else if score > 0.0 {
            if let Some(t) = log.as_mut() {
                t.push((score, false, format!("{g:?}")));
            }
        }
    };
    let dist_discount = |to: TilePos| 1.0 / (1.0 + me.dist(to) as f32 / 40.0);
    // The cold they are in, weighed against the walk (#467, FIXLOG 2026-10-29): below
    // their comfort every hour out costs warmth (hazards.rs cold_step), so a long food
    // trip in a hard cold is worth less the farther and colder it is. Current
    // temperature and their own body only — no forecast of tomorrow's weather. The cold
    // is where they are (#268): the heights count colder than the valley floor.
    let cold = (a.species_def().climate.comfort_c[0] - a.hardened - w.temp_at(me)).max(0.0);
    let cold_trip = |to: TilePos| 1.0 / (1.0 + cold * me.dist(to) as f32 / 60.0);
    if !w.bands.is_empty() {
        consider_bands(w, i, &mut consider);
    }

    // Eat: known food items on the ground (or carried), else a known fruiting plant. The
    // newly arrived eat and gather sooner.
    if hunger_p > 0.25 / survive {
        // Eat before bed: at night a half-empty stomach outranks sleep.
        let urgency = hunger_p
            * hunger_p
            * 3.0
            * survive
            * if hunger_p > 0.5 { 2.0 } else { 1.0 }
            * if night && hunger_p > 0.45 { 1.6 } else { 1.0 };
        for it in &w.items {
            if !it.kind.is_food() {
                continue;
            }
            match it.place {
                ItemPlace::Carried(id) if id == a.id => {
                    consider(urgency * 1.3, Goal::EatItem(it.id, me))
                }
                ItemPlace::Ground(p) => {
                    if let Some(b) = a.knowledge.get(Known::Item(it.id)) {
                        // Food lying under water is not a meal anyone goes for (#415): the
                        // moved-camp census found eight starve going down again and again
                        // into a flooded cellar for its stores, turned back by the water
                        // (hazards.rs `drown_step`) each time.
                        if nav.reach(me, p) && !crate::systems::hazards::under_water(w, p) {
                            let liking = a.memories.bias(MemKind::Meal, Subject::Food(it.kind))
                                * crate::systems::culture::food_factor(w, a, it.kind);
                            consider(
                                urgency
                                    * b.confidence
                                    * dist_discount(p)
                                    * (0.6 + it.kind.nutrition_hours() / 10.0)
                                    * liking,
                                Goal::EatItem(it.id, p),
                            );
                        }
                    }
                }
                _ => {}
            }
        }
        for (k, b) in a.knowledge.iter_kind(|k| matches!(k, Known::FoodPlant(_))) {
            if let Known::FoodPlant(p) = k {
                if nav.reach(me, *p) {
                    let skill = 0.8 + a.skills.level(crate::mind::Skill::Foraging) / 40.0;
                    // What happened here last time counts as much as what grows here now.
                    let feel = a.memories.bias(MemKind::Meal, Subject::Place(*p))
                        * a.place_multiplier(*p, now);
                    // Those who learned to keep what they have in hunger go looking sooner.
                    let keep = a.beliefs.weight(crate::mind::beliefs::Topic::Hunger, 1.0);
                    consider(
                        urgency
                            * 0.9
                            * b.confidence
                            * dist_discount(*p)
                            * skill
                            * feel
                            * keep
                            * cold_trip(*p),
                        Goal::Forage(*p),
                    );
                }
            }
        }
    }
    // Drink (#433): water before food. The nearest shore or well they can walk to; a day
    // and a half without it kills, so thirst presses harder than hunger at the same need.
    let thirst_p = 1.0 - a.needs.thirst;
    if thirst_p > 0.3 / survive {
        // Twice eating's weight (#415: water before food), so the walk to water wins over a
        // meal in hand when both are as pressing.
        let urgency = thirst_p
            * thirst_p
            * 6.0
            * survive
            * if thirst_p > 0.5 { 2.0 } else { 1.0 }
            * if night && thirst_p > 0.45 { 1.6 } else { 1.0 };
        // Parched, they will walk twice as far for it. The water is chosen by what they
        // know of it (#453 part 4, FIXLOG 2026-10-30): a pond they remember salt is never
        // worth the walk, one they remember foul only when thirst passes desperation (0.7,
        // the parched mark above) — or nothing they could believe fresh stands in reach.
        let reach = if thirst_p > 0.7 { 120 } else { 60 };
        if let Some(s) =
            crate::water::nearest_knowing(w, i, me, reach, &a.knowledge, thirst_p > 0.7)
        {
            consider(urgency * dist_discount(s.stand), Goal::Drink(s.stand));
        } else if thirst_p > 0.4 {
            if let Some(p) = search_for_water(w, i, r) {
                consider(search_urge(thirst_p) * dist_discount(p), Goal::SeekWater(p));
            }
        }
    }
    // Tomorrow's food (#415): with the stores under two days, those who know where food grows
    // go and gather it even before they are hungry; one not hungry leaves what they gathered
    // on the ground for the store (act.rs).
    if short && !night {
        for (k, b) in a.knowledge.iter_kind(|k| matches!(k, Known::FoodPlant(_))) {
            if let Known::FoodPlant(p) = k {
                if nav.reach(me, *p) {
                    let skill = 0.8 + a.skills.level(crate::mind::Skill::Foraging) / 40.0;
                    consider(
                        0.9 * b.confidence * dist_discount(*p) * skill * cold_trip(*p),
                        Goal::Stock(*p),
                    );
                }
            }
        }
    }
    // Night is near with no bed known and no roof overhead: go under the community's roof if
    // there is one (#415); else the work of shelter weighs more (below).
    if roofless {
        let roof = a
            .community
            .and_then(|c| w.communities.iter().find(|x| x.id == c))
            .and_then(|c| {
                c.house_sites
                    .iter()
                    .filter_map(|o| crate::houses::heart(&crate::houses::plan(w, *o).floor))
                    .filter(|p| {
                        w.grid.walkable(*p)
                            && crate::systems::hazards::indoors(w, *p)
                            && nav.reach(me, *p)
                    })
                    .min_by_key(|p| p.dist(me))
            });
        if let Some(t) = roof {
            consider(1.3 * dist_discount(t).max(0.5), Goal::Warm(t));
        } else if let Some(home) = a
            .community
            .and_then(|c| w.communities.iter().find(|x| x.id == c))
            .map(|c| c.home)
            .filter(|h| me.dist(*h) > SETTLE_RANGE && w.grid.walkable(*h))
        {
            // No roof to go to: bed down at the camp with the others, not out alone.
            consider(1.0, Goal::Seek(edge_of_camp(w, home, me, SETTLE_RANGE / 2)));
        }
    }
    // Keep together while unsecured (#415): one far from their camp with nothing secured walks
    // back toward it rather than on alone.
    if unsecured && !young {
        if let Some(home) = a
            .community
            .and_then(|c| w.communities.iter().find(|x| x.id == c))
            .map(|c| c.home)
        {
            let d = me.dist(home);
            if d > GROUP_RANGE && w.grid.walkable(home) {
                consider(
                    0.5 + d as f32 / 80.0,
                    Goal::Seek(edge_of_camp(w, home, me, GROUP_RANGE / 2)),
                );
            }
        }
    }
    // Cold: get to a lit hearth or indoors before anything else but hunger. The gate is
    // the cold they are in against their own comfort, not a flat 2 C: warmth drains in
    // the whole band below comfort, so the goal opens there too, and the harder the cold,
    // the sooner it opens (#467, FIXLOG 2026-10-29).
    if a.needs.warmth < warm_gate(w, a)
        && w.temp_at(me) < a.species_def().climate.comfort_c[0] - a.hardened
        && !crate::systems::hazards::indoors(w, me)
    {
        let target = w
            .structures
            .iter()
            // A hearth they know of (seen, or told): not every fire in the valley.
            .filter(|s| {
                s.kind == StructureKind::Hearth
                    && s.progress >= 1.0
                    && a.knowledge.get(Known::Structure(s.id)).is_some()
                    && nav.reach(me, s.pos)
            })
            .min_by_key(|s| s.pos.dist(me))
            .map(|s| s.pos.offset(0, 1, 0))
            .filter(|p| w.grid.walkable(*p))
            .or_else(|| {
                // Else the nearest of the community's houses with a roof: a roof is warmth
                // enough.
                a.community
                    .and_then(|c| w.communities.iter().find(|x| x.id == c))
                    .and_then(|c| {
                        c.house_sites
                            .iter()
                            .filter_map(|o| crate::houses::heart(&crate::houses::plan(w, *o).floor))
                            .filter(|p| {
                                w.grid.walkable(*p)
                                    && crate::systems::hazards::indoors(w, *p)
                                    && nav.reach(me, *p)
                            })
                            .min_by_key(|p| p.dist(me))
                    })
            });
        if let Some(t) = target {
            consider(
                (2.5 + (warm_gate(w, a) - a.needs.warmth).max(0.0) * 5.0) * survive,
                Goal::Warm(t),
            );
        } else if a.needs.warmth < 0.25 {
            // No lit hearth or roof they know of: walk back toward the camp and the
            // others; close bodies lose less warmth (hazards.rs huddling). #467.
            if let Some(home) = a
                .community
                .and_then(|c| w.communities.iter().find(|x| x.id == c))
                .map(|c| c.home)
                .filter(|h| w.grid.walkable(*h))
            {
                consider(
                    1.8 * survive,
                    Goal::Seek(edge_of_camp(w, home, me, GROUP_RANGE / 2)),
                );
            }
        }
    }
    // Fire: whoever sees it decides what to do by what they know and what they believe.
    // Nobody is sent; taking it up is their own undertaking (design-organic.md section 3).
    let owns_something = w
        .designations
        .iter()
        .any(|x| !x.done && x.owner == Some(a.id));
    {
        use crate::mind::beliefs::Topic;
        use crate::mind::Technique;
        let near_fire = w
            .fires
            .iter()
            .map(|f| f.pos)
            // A fire is hard to miss: flames and smoke carry a long way across open ground.
            .filter(|p| p.z == me.z && p.dist(me) <= 18)
            .min_by_key(|p| p.dist(me));
        if let Some(fp) = near_fire {
            let run = a.beliefs.weight(Topic::Fire, 1.0);
            let fight = a.beliefs.weight(Topic::Fire, -1.0);
            let knows_beat = a.techniques.knows(Technique::BeatFlames);
            let knows_water = a.techniques.knows(Technique::CarryWater);
            if fp.dist(me) <= 4 {
                let (mut dx, mut dy) = ((me.x - fp.x).signum(), (me.y - fp.y).signum());
                if dx == 0 && dy == 0 {
                    dx = 1;
                    dy = 0;
                }
                let to = w.grid.surface(
                    (me.x + dx * 8).clamp(1, w.grid.width - 2),
                    (me.y + dy * 8).clamp(1, w.grid.height - 2),
                );
                if w.grid.walkable(to) && nav.reach(me, to) {
                    let fear = if knows_beat || knows_water { 0.8 } else { 1.3 };
                    consider(1.5 * run * fear, Goal::Flee(to));
                }
            }
            if !owns_something {
                let open_ground = w.grid.get(fp).shape != crate::world::Shape::Wall;
                if knows_water && crate::systems::settlement::water_source_near(w, fp, 25).is_some()
                {
                    consider(
                        (1.5 + 0.6 * a.techniques.skill(Technique::CarryWater)) * fight,
                        Goal::Respond(fp, Technique::CarryWater),
                    );
                }
                if open_ground {
                    if knows_beat {
                        consider(
                            (1.45 + 0.6 * a.techniques.skill(Technique::BeatFlames)) * fight,
                            Goal::Respond(fp, Technique::BeatFlames),
                        );
                    } else if !knows_water
                        && (a.traits.has(crate::mind::Trait::Curious)
                            || a.traits.has(crate::mind::Trait::Industrious))
                    {
                        // Never done it, but someone has to: they might try.
                        consider(0.9 * fight, Goal::Respond(fp, Technique::BeatFlames));
                    }
                }
            }
        }
    }
    // The hurt and the sick: someone who knows a poultice may choose to tend them, and anyone
    // well may nurse their own family without one (#418: on seed 3 three died of a fever in
    // 23 to 32 days and were never tended).
    let healer = (a.techniques.knows(crate::mind::Technique::Poultice)
        || a.duties.held.contains(&crate::society::Role::Healer))
        && w.items
            .iter()
            .any(|it| it.kind == crate::items::ItemKind::Poultice);
    let kin = |p: &crate::agents::Agent| {
        a.partner == Some(p.id)
            || p.mother == Some(a.id)
            || p.father == Some(a.id)
            || a.mother == Some(p.id)
            || a.father == Some(p.id)
    };
    if !owns_something && !a.body.needs_care() && !a.is_child(now) {
        let patient = w
            .agents
            .iter()
            .filter(|p| {
                p.alive
                    && p.id != a.id
                    // One who regrets a quarrel with them goes to make amends (#568).
                    && (healer
                        || kin(p)
                        || a.conscience.help_toward(p.id, now) > AMENDS)
                    && p.community.is_some()
                    && p.community == a.community
                    && p.body.needs_care()
                    && p.tended_until < now
                    // Up or down a stair as well as across, where a way leads (#439: the sick
                    // below were never looked for from the ground, and a tending was chosen for
                    // the sick no way led to, 825 times on seed 4).
                    && (p.pos.x - me.x).abs().max((p.pos.y - me.y).abs()) <= 25
                    && nav.reach(me, p.pos)
                    && !w
                        .designations
                        .iter()
                        .any(|d| !d.done && d.kind == crate::work::DesignationKind::Tend(p.id))
            })
            .min_by_key(|p| p.pos.dist(me));
        if let Some(p) = patient {
            let skill = a.techniques.skill(crate::mind::Technique::Poultice);
            let family = if kin(p) { 0.4 } else { 0.0 };
            consider(
                (1.1 + 0.5 * skill + family) * dist_discount(p.pos),
                Goal::Tend(p.id, p.pos),
            );
        }
    }
    // Sleep: at night or when tired; prefer a known free bed.
    // Daytime sleep only when genuinely tired (about ten hours awake); at night, readily.
    // The sick lie down by day as well (#418): they spent a tenth of a fever in bed. Not
    // hungry, though: a meal comes first.
    let sick = a.body.ill() >= 0.4 && hunger_p < 0.5;
    // With night coming, no bed and no roof, the tired lie down where they have settled (#415)
    // rather than sit up waiting for the dark. Not in the cold: there they keep to the hearth
    // until night.
    let early = roofless && rest_p > 0.6 && w.temp_at(a.pos) >= 2.0;
    if rest_p > 0.92 || (night && rest_p > 0.15) || sick || early {
        let urgency = rest_p * 2.5
            + if sick {
                1.5
            } else if night {
                0.8
            } else {
                0.0
            };
        let sleeping_places = sleeping_places.get_or_insert_with(|| SleepingPlaces::build(w));
        // The beds of their own home are known to all of it, as its hearths are: a person
        // knows where the village's beds are without having seen each one finished. Before
        // this, beds entered a mind only when finished within 14 tiles or seen, and the house
        // walls hid them, so most rough nights were spent beside a free bed of home nobody
        // had in mind (#469, FIXLOG 2026-11-02).
        let home_beds: Vec<(StructureId, TilePos)> = a
            .community
            .and_then(|c| w.communities.iter().find(|x| x.id == c))
            .map(|c| sleeping_places.home_beds(c.id, c.home).to_vec())
            .unwrap_or_default();
        let mut bed_found = false;
        let mut consider_bed = |id, at| {
            if sleeping_places.is_taken(id, a.id) {
                return false;
            }
            // A bed in a flood's way or on ground under water is no choice: `flee_step` turns
            // the tired back from it, and they never lie down until they collapse. With no
            // safe bed they sleep where they stand.
            if crate::systems::hazards::in_floods_way(w, at) || unsafe_ground(w, at) {
                return false;
            }
            let feel = a.place_multiplier(at, now);
            consider(
                urgency * 1.2 * dist_discount(at) * feel,
                Goal::SleepBed(id, at),
            );
            true
        };
        for (k, b) in a.knowledge.iter_kind(|k| matches!(k, Known::Structure(_))) {
            if let Known::Structure(id) = k {
                if tag_is_sleeping_place(b.tag) && nav.reach(me, b.pos) {
                    bed_found |= consider_bed(*id, b.pos);
                }
            }
        }
        for (id, at) in home_beds {
            if a.knowledge.get(Known::Structure(id)).is_none() && nav.reach(me, at) {
                bed_found |= consider_bed(id, at);
            }
        }
        // Before choosing the ground, use only completed beds in this tick's sight: an
        // off-stripe look must not strand a sleeper beside an unknown bed, and remote or
        // unfinished structures are not choices (#469, FIXLOG 2026-10-31 c144 review).
        if !bed_found {
            let sight = crate::systems::perceive::sight_radius(w, a, now);
            let mut local = Vec::new();
            for cell in SleepingPlaces::near(me, sight) {
                local.extend(
                    sleeping_places
                        .by_cell
                        .get(&cell)
                        .into_iter()
                        .flatten()
                        .copied(),
                );
            }
            local.sort_by_key(|(order, _, _)| *order);
            for (_, id, at) in local {
                if (at.x - me.x).abs().max((at.y - me.y).abs()) <= sight
                    && crate::mind::perception::line_of_sight(&w.grid, me, at)
                    && nav.reach(me, at)
                {
                    bed_found |= consider_bed(id, at);
                }
            }
        }
        // The ground is only for one with no free reachable bed known or in sight (#469).
        if !bed_found {
            consider(urgency * 0.6, Goal::SleepHere);
        }
    }
    // Go see the new baby (#1736, FIXLOG 2026-11-15): kin and friends glad of a birth go to
    // the mother within two days, ahead of the ordinary Talk gate and its social_p>0.3 need
    // (a grandmother at leisure, not pining for company, still calls on a new grandchild).
    // Guards against: a parent's birthday going unmarked by anyone but the two parents, since
    // `choose` otherwise weighs a new mother the same as on any other day.
    const BIRTH_VISIT_WINDOW: u64 = 2 * TICKS_PER_DAY;
    const BIRTH_VISIT_RANGE: i32 = 30;
    for (k, b) in a.knowledge.iter_kind(|k| matches!(k, Known::Agent(_))) {
        if let Known::Agent(id) = k {
            if let Some(o) = w.agents.iter().find(|o| o.id == *id) {
                if !o.alive
                    || o.species != crate::agents::Species::Person
                    || o.activity.is_asleep()
                    || now.saturating_sub(o.last_birth) > BIRTH_VISIT_WINDOW
                    || me.dist(o.pos) > BIRTH_VISIT_RANGE
                    || !nav.reach(me, o.pos)
                {
                    continue;
                }
                // Kin: a parent, a child, or a sibling (sharing a mother or father) of the
                // new mother or father. Friends: anyone else whose opinion of them runs warm.
                let kin = a.mother == Some(*id)
                    || a.father == Some(*id)
                    || a.children.contains(id)
                    || (a.mother.is_some() && a.mother == o.mother)
                    || (a.father.is_some() && a.father == o.father);
                let fondness = (1.0
                    + a.opinion_of(*id) * 0.8
                    + a.memories.attitude(*id) * 0.6
                    + a.relationships
                        .get(id)
                        .map(|r| r.familiarity)
                        .unwrap_or(0.0)
                        * 0.8)
                    .clamp(0.2, 1.8);
                let friend = a.opinion_of(*id) >= 0.6;
                if !kin && !friend {
                    continue;
                }
                if a.memories.aversion_of(*id) + aversion_flag(a, *id) > 0.45
                    || crate::feuds::across(w, a.id, *id)
                {
                    continue;
                }
                // Once talked to since this birth, not again (the acceptance's "a stranger
                // does not" is already kept out above; this keeps kin from circling back).
                let already = a.memories.short.iter().any(|r| {
                    r.tick >= o.last_birth
                        && r.kind == crate::mind::memory::MemKind::Company
                        && r.subject == crate::mind::memory::Subject::Person(*id)
                });
                if already {
                    continue;
                }
                let adj = TilePos::new(o.pos.x, o.pos.y + 1, o.pos.z);
                let target = if w.grid.walkable(adj) { adj } else { o.pos };
                consider(
                    0.9 * fondness * leisure * dist_discount(b.pos),
                    Goal::Talk(*id, target),
                );
            }
        }
    }
    // Talk: someone known nearby who is awake.
    if social_p > 0.3 {
        // Company never outranks a real hunger.
        let urgency = social_p * 1.4 * a.talk_weight() * if hunger_p > 0.5 { 0.4 } else { 1.0 };
        for (k, b) in a.knowledge.iter_kind(|k| matches!(k, Known::Agent(_))) {
            if let Known::Agent(id) = k {
                if let Some(o) = w.agents.iter().find(|o| o.id == *id) {
                    // Not one walking off somewhere (#553, FIXLOG 2026-11-01): by the time
                    // they are reached they are gone, and the talk fails as "they left". One
                    // out for a stroll may be caught up with: a stroller stops to talk
                    // (act.rs), and a settled camp's talk is mostly with them.
                    let walking_off = (o.is_moving() || !o.path.is_empty())
                        && !matches!(o.activity, Activity::Wandering { .. });
                    if o.alive
                        && o.species == crate::agents::Species::Person
                        && !o.activity.is_asleep()
                        && !walking_off
                        && me.dist(o.pos) <= 14
                        && nav.reach(me, o.pos)
                    {
                        let adj = TilePos::new(o.pos.x, o.pos.y + 1, o.pos.z);
                        let target = if w.grid.walkable(adj) { adj } else { o.pos };
                        // People seek out those they know and like; strangers get a look too.
                        // Someone who hurt them recently is not company they will choose.
                        if a.memories.aversion_of(*id) + aversion_flag(a, *id) > 0.45 {
                            continue;
                        }
                        // Nor the other family in a feud (#404, 2026-10-27
                        // fix-404-feuds-in-play): kin keep away from those they are at odds with.
                        if crate::feuds::across(w, a.id, *id) {
                            continue;
                        }
                        let fam = a
                            .relationships
                            .get(id)
                            .map(|r| r.familiarity)
                            .unwrap_or(0.0);
                        let fondness = (1.0
                            + a.opinion_of(*id) * 0.8
                            + a.memories.attitude(*id) * 0.6
                            + fam * 0.8)
                            .clamp(0.2, 1.8);
                        consider(
                            urgency * dist_discount(b.pos) * fondness,
                            Goal::Talk(*id, target),
                        );
                    }
                }
            }
        }
    }
    // Evening: the hearth. A habit of gathering draws people more strongly, and so does having
    // learned to hold the living close; it never outranks a real hunger or tiredness.
    if crate::systems::culture::HEARTH_HOURS.contains(&Calendar::from_tick(w.meta.tick).hour)
        && hunger_p < 0.6
        && rest_p < 0.85
    {
        if let Some(h) = a
            .community
            .and_then(|c| crate::systems::culture::hearth_of(w, c).map(|h| (c, h)))
        {
            let (cid, h) = h;
            const SEATS: [(i32, i32); 8] = [
                (-1, 1),
                (1, 1),
                (0, 2),
                (-2, 0),
                (2, 0),
                (-1, -1),
                (1, -1),
                (0, -2),
            ];
            let k = a.id.0 as usize % SEATS.len();
            let seat = (0..SEATS.len())
                .map(|j| SEATS[(k + j) % SEATS.len()])
                .map(|(dx, dy)| h.offset(dx, dy, 0))
                .find(|p| w.grid.walkable(*p) && nav.reach(me, *p));
            if let Some(seat) = seat {
                let habit = crate::systems::culture::hearth_habit(w, cid);
                let close = a.beliefs.weight(crate::mind::beliefs::Topic::Death, 1.0);
                consider(
                    (0.35 + 0.5 * habit) * a.talk_weight() * close * dist_discount(seat),
                    Goal::Gather(seat),
                );
            }
        }
    }
    // By day, the curious, those it has shown an echo, and those already learning its ways go
    // and sit by the portal stone while the air there is thin and nobody has chosen what to do.
    if w.thin.woke.is_some()
        && !w.thin.quieted
        && w.thin.strength > 0.1
        && (8..18).contains(&Calendar::from_tick(w.meta.tick).hour)
        && hunger_p < 0.5
        && rest_p < 0.7
    {
        let touched = a.traits.has(crate::mind::Trait::Curious)
            || a.techniques.knows(crate::mind::Technique::ThinLore)
            || w.thin.seen.iter().any(|(x, _)| *x == a.id);
        if let Some(s) = crate::thin::stone(w).filter(|_| touched) {
            let seat = [(0, 1), (1, 0), (-1, 0), (0, -1), (1, 1), (-1, 1)]
                .iter()
                .map(|(dx, dy)| s.offset(*dx, *dy, 0))
                .find(|p| w.grid.walkable(*p) && nav.reach(me, *p));
            if let Some(seat) = seat {
                let settled = if w.thin.chosen.is_some() { 0.4 } else { 1.0 };
                consider(0.3 * settled * dist_discount(seat), Goal::StudyThin(seat));
            }
        }
    }
    // Company presses only once the next day is secured (#415).
    let pressing = hunger_p.max(rest_p).max(social_p * leisure);
    // Work: community designations, when nothing personal presses hard. Score by priority,
    // skill fit, trait, distance; materials must be known for build sites.
    let child = a.is_child(w.meta.tick);
    if child {
        // Children keep close to a parent who is away; otherwise they play (wander) and talk.
        if let Some(p) = [a.mother, a.father, a.foster]
            .into_iter()
            .flatten()
            .find_map(|pid| w.agents.iter().find(|o| o.id == pid && o.alive))
        {
            if me.dist(p.pos) > 10 && nav.reach(me, p.pos) && hunger_p < 0.5 {
                consider(0.5, Goal::Seek(TilePos::new(p.pos.x, p.pos.y + 1, p.pos.z)));
            }
        }
    }
    // Go and look: only with word of someone, only when there is time to spare, and never
    // twice for the same person at once. Longing is not a stat; it is why they walk.
    if !night && !child && pressing < 0.55 && hunger_p < 0.5 {
        for rm in &a.heard {
            let still_lost = w.survivor_pool.iter().any(|s| {
                s.index == rm.who
                    && matches!(
                        s.state,
                        crate::opening::SurvivorState::Lost { found: false, .. }
                    )
            });
            if !still_lost {
                continue;
            }
            let already = w.searches.iter().any(|s| s.who == rm.who);
            if already || !nav.reach(me, rm.at) {
                continue;
            }
            let kin = a.memories.bias(
                crate::mind::MemKind::Company,
                crate::mind::Subject::Place(rm.at),
            );
            consider(
                0.55 * dist_discount(rm.at) * (0.8 + a.needs.comfort.min(1.0) * 0.2) * kin,
                Goal::Search(rm.who, rm.at),
            );
        }
    }
    // A beast that has hurt people more than once: those who hold that force settles things,
    // and the hardy, go after it (creatures.rs).
    if !child && !night && !a.body.needs_care() && hunger_p < 0.5 {
        let willing = a.beliefs.lean(crate::mind::beliefs::Topic::Violence) > 0.1
            || a.traits.has(crate::mind::Trait::Hearty);
        if willing {
            for (beast, hurt) in &w.creatures.wounds_by {
                if *hurt < 2 {
                    continue;
                }
                let Some(b) = w
                    .agents
                    .iter()
                    .find(|x| x.id == *beast && x.alive && !x.denned)
                else {
                    continue;
                };
                let hunted = w.designations.iter().any(|d| {
                    !d.done
                        && matches!(d.kind, crate::work::DesignationKind::Hunt(t) if t == *beast)
                });
                if hunted || b.pos.z != me.z || b.pos.dist(me) > 50 || !nav.reach(me, b.pos) {
                    continue;
                }
                consider(
                    0.9 * a.beliefs.weight(crate::mind::beliefs::Topic::Violence, 1.0),
                    Goal::Avenge(*beast, b.pos),
                );
            }
        }
    }
    // Something from the old valley they know of that could teach them a craft (C13): the
    // curious go and sit over it when nothing presses.
    // Not everyone: the curious, and those whose hands already know a craft.
    let inclined = a.traits.has(crate::mind::Trait::Curious)
        || a.skills.level(crate::mind::Skill::Crafting) >= 5.0;
    if inclined && !child && !night && hunger_p < 0.5 && pressing < 0.6 {
        let studying = w.designations.iter().any(|d| {
            !d.done
                && d.owner == Some(a.id)
                && matches!(d.kind, crate::work::DesignationKind::Study(_))
        });
        if !studying {
            for (k, _) in a.knowledge.iter_kind(|k| matches!(k, Known::Item(_))) {
                let Known::Item(id) = k else { continue };
                let Some(it) = w.items.iter().find(|it| it.id == *id) else {
                    continue;
                };
                let ItemPlace::Ground(p) = it.place else {
                    continue;
                };
                let Some(t) = it.story.as_ref().and_then(|s| s.teaches) else {
                    continue;
                };
                let resting = a
                    .work_cooldowns
                    .get(&(0x5700_0000 | (id.0 & 0x00FF_FFFF)))
                    .is_some_and(|until| *until > now);
                if a.techniques.knows(t) || resting || !nav.reach(me, p) {
                    continue;
                }
                let curious = if a.traits.has(crate::mind::Trait::Curious) {
                    1.6
                } else {
                    1.0
                };
                consider(0.25 * curious * dist_discount(p), Goal::Study(*id, p));
            }
        }
    }
    let rest_day = crate::systems::culture::rest_day_for(w, a);
    // An assigned job (#433): a designation is a placed order, not something to stumble on
    // by sight, so working it does not wait on knowing anything else. A lone arrival with
    // nothing yet to perceive (water aside, which does not teach a tool or a bite to eat)
    // could otherwise leave a chop order untouched forever, found when the stump test left
    // its person wandering four hours without once starting it.
    if !child && !rest_day && pressing < 0.75 && hunger_p < 0.55 && !night {
        for d in &w.designations {
            if d.done
                || d.failures > 12
                || !w
                    .reservations
                    .is_free_for(crate::work::ResKey::Designation(d.id), a.id)
            {
                continue;
            }
            if d.community.is_some() && d.community != a.community {
                continue;
            }
            if d.owner.is_some() && d.owner != Some(a.id) {
                continue;
            }
            // Who holds that the deep is not to be dug will not dig into it (P10).
            if matches!(
                d.kind,
                crate::work::DesignationKind::Mine | crate::work::DesignationKind::DigStairs
            ) && a.beliefs.lean(crate::mind::beliefs::Topic::DeepDigging) < -0.3
                && crate::underground::in_the_deep(w, d.pos)
            {
                continue;
            }
            if a.work_cooldowns
                .get(&d.id.0)
                .map(|t| *t > w.meta.tick)
                .unwrap_or(false)
            {
                continue;
            }
            // A build site whose materials do not exist anywhere on the ground is not a job yet.
            if let Some((kind, need, want)) = d.missing_wants().next() {
                let on_ground: u32 = w
                    .items
                    .iter()
                    .filter(|it| {
                        it.kind == kind
                            && want.is_none_or(|m| it.material == m)
                            && matches!(it.place, ItemPlace::Ground(_))
                    })
                    .map(|it| it.count)
                    .sum();
                if on_ground < need.min(1) {
                    continue;
                }
            }
            // Some will not fell a living tree (CutLivingTree, P7).
            if matches!(d.kind, crate::work::DesignationKind::Chop)
                && a.beliefs.lean(crate::mind::beliefs::Topic::CutLivingTree) <= -0.4
            {
                continue;
            }
            // A bench's craft is worked by those who know it (C70, design rule 3), or, when
            // nobody of the community does, tried by its best hand.
            if let crate::work::DesignationKind::Craft(r) = d.kind {
                if let Some(t) = crate::work::recipes::craft_of(r) {
                    if !a.techniques.knows(t) && !crate::systems::crafts::may_try(w, a, t, d) {
                        continue;
                    }
                }
            }
            let fit = match d.kind {
                crate::work::DesignationKind::Mine => {
                    0.6 + a.skills.level(crate::mind::Skill::Mining) / 20.0
                }
                crate::work::DesignationKind::Cook => {
                    0.6 + a.skills.level(crate::mind::Skill::Cooking) / 20.0
                }
                crate::work::DesignationKind::Sow | crate::work::DesignationKind::Harvest => {
                    0.6 + a.skills.level(crate::mind::Skill::Farming) / 20.0
                }
                crate::work::DesignationKind::Haul => {
                    0.5 + a.skills.level(crate::mind::Skill::Hauling) / 20.0 + a.haul_fit()
                }
                _ => 0.6 + a.skills.level(crate::mind::Skill::Construction) / 20.0,
            };
            // #454: keener on the work of their duties (duties.rs).
            let role_fit = crate::duties::fit(&a.duties.held, d.kind);
            let base = (0.5 + d.priority as f32 * 0.12) * role_fit;
            // Shelter, warmth and food are the work that matters while the arrival is fresh,
            // shelter more as night comes with no bed known, food more with the stores under
            // two days (#415). A roof also matters once a hard cold is draining someone's
            // warmth, whether or not they personally have a bed: the same low-warmth
            // threshold `freezing` checks above, not only a homeless dusk (#859, FIXLOG
            // 2026-11-15; before this, ten roofs sat untaken for 207 days through a hard
            // cold because nothing but a homeless worker at dusk ever lifted BuildRoof).
            let cold = a.needs.warmth < 0.5;
            use crate::work::DesignationKind as K;
            let base = match d.kind {
                K::BuildRoof(_) => base * survive * if roofless || cold { 1.6 } else { 1.0 },
                K::BuildWall(_)
                | K::BuildDoor
                | K::BuildStairs(_)
                | K::BuildFloor(_)
                | K::BuildStructure(..)
                | K::Chop => base * survive * if roofless { 1.6 } else { 1.0 },
                K::Sow | K::Harvest | K::Cook => base * survive * if short { 1.4 } else { 1.0 },
                K::Hunt(_) | K::Fish | K::Snare => base * survive * if short { 1.4 } else { 1.0 },
                _ => base,
            };
            // Some go toward a fire and some away from it, by what they believe about fire.
            let belief = if matches!(d.kind, crate::work::DesignationKind::Douse) {
                a.beliefs.weight(crate::mind::beliefs::Topic::Fire, -1.0)
            } else {
                1.0
            };
            // Work in a place they were hurt is work they will put off.
            let score = base
                * fit
                * dist_discount(d.pos)
                * a.place_multiplier(d.pos, now)
                * belief
                * (1.0 - d.failures as f32 * 0.1);
            consider(score, Goal::Work(d.id));
        }
    }
    // Wander: curiosity when nothing presses; searching when hungry and no food is known.
    let food_known = a
        .knowledge
        .map
        .keys()
        .any(|k| matches!(k, Known::FoodPlant(_)))
        || w.items.iter().any(|it| {
            it.kind.is_food()
                && (matches!(it.place, ItemPlace::Carried(id) if id == a.id)
                    || a.knowledge.get(Known::Item(it.id)).is_some())
        });
    let searching = hunger_p > 0.4 && !food_known;
    // One of the lost keeps to the ground around where they came through, unless hunger is
    // bad enough to drive them out into country they do not know.
    let camp = if hunger_p < 0.6 {
        w.survivor_pool.iter().find_map(|s| match s.state {
            crate::opening::SurvivorState::Lost {
                agent,
                found: false,
            } if agent == a.id => Some(w.lost_camp(s.index)),
            _ => None,
        })
    } else {
        None
    };
    let home = a
        .community
        .and_then(|c| w.communities.iter().find(|x| x.id == c))
        .map(|c| c.home);
    if searching {
        // Hungry with no food known: a search with a direction, not a stroll (#415). Back to
        // the camp first, where the stores and the others are; else out to where they ate
        // before; else round the camp, not wherever their feet last stopped.
        let urge = search_urge(hunger_p);
        let back = home
            .or(camp)
            .filter(|h| me.dist(*h) > 12 && w.grid.walkable(*h));
        let fed_here = || {
            let short = a.memories.short.iter().filter_map(|m| match m.subject {
                crate::mind::Subject::Place(p) if m.kind == MemKind::Meal && m.valence > 0.0 => {
                    Some(p)
                }
                _ => None,
            });
            let long = a.memories.long.iter().filter_map(|m| match m.subject {
                crate::mind::Subject::Place(p) if m.kind == MemKind::Meal && m.valence > 0.0 => {
                    Some(p)
                }
                _ => None,
            });
            short
                .chain(long)
                .filter(|p| p.dist(me) > 3 && w.grid.walkable(*p) && nav.reach(me, *p))
                .min_by_key(|p| (p.dist(me), p.x, p.y, p.z))
        };
        if let Some(h) = back {
            consider(urge, Goal::Seek(edge_of_camp(w, h, me, 6)));
        } else if let Some(p) = fed_here() {
            consider(urge, Goal::Seek(p));
        } else {
            // Those who know to range far when food is short go well beyond the usual ground.
            let rad = if a.techniques.knows(crate::mind::Technique::ForageFar) {
                28
            } else {
                12
            };
            let c = camp.or(home).unwrap_or(me);
            for _ in 0..4 {
                let cand = TilePos::new(
                    c.x + r.gen_range(-rad..=rad),
                    c.y + r.gen_range(-rad..=rad),
                    me.z,
                );
                if w.grid.in_bounds(cand) && w.grid.walkable(cand) && nav.reach(me, cand) {
                    consider(urge * r.gen_range(0.6..1.0), Goal::Seek(cand));
                    break;
                }
            }
        }
    } else if pressing < 0.6 && !night {
        // Curiosity when nothing presses: leisure, gated above.
        let wander = 0.35 * a.wander_weight();
        let (cx, cy, rad) = match camp {
            Some(c) => (c.x, c.y, 15),
            None => (me.x, me.y, 12),
        };
        for _ in 0..4 {
            let cand = TilePos::new(
                cx + r.gen_range(-rad..=rad),
                cy + r.gen_range(-rad..=rad),
                me.z,
            );
            if w.grid.in_bounds(cand) && w.grid.walkable(cand) && nav.reach(me, cand) {
                consider(wander * r.gen_range(0.6..1.0), Goal::Wander(cand));
                break;
            }
        }
    }
    consider(0.2 + if night { 0.3 } else { 0.0 }, Goal::Loiter);
    let _ = Need::Comfort;
    if let Some(c) = trace {
        if let Some(k) = best_at {
            c.options[k].chosen = true;
        }
        c.conditions = vec![
            crate::why::Condition {
                name: "hunger",
                value: hunger_p,
                ok: !hungry,
            },
            crate::why::Condition {
                name: "thirst",
                value: thirst_p,
                ok: !thirsty,
            },
            crate::why::Condition {
                name: "tiredness",
                value: rest_p,
                ok: rest_p <= 0.92,
            },
            crate::why::Condition {
                name: "days of food in store",
                value: days.unwrap_or(f32::NAN),
                ok: !short,
            },
            crate::why::Condition {
                name: "night near with no bed or roof",
                value: if roofless { 1.0 } else { 0.0 },
                ok: !roofless,
            },
            crate::why::Condition {
                name: "arrival urge",
                value: survive,
                ok: survive <= 1.0,
            },
            crate::why::Condition {
                name: "leisure weight",
                value: leisure,
                ok: leisure >= 1.0,
            },
        ];
    }
    if let Some(t) = log {
        let chose = best
            .as_ref()
            .map_or("nothing".to_string(), |b| format!("{:?}", b.1));
        let opts = t.into_iter().map(|(sc, ok, g)| (g, sc, ok)).collect();
        crate::debuglog::decision(w, w.agents[i].id.0, &chose, opts);
    }
    best.map(|b| b.1)
}
