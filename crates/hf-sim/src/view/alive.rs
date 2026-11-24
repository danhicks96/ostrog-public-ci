//! The alive report (WP-10, epic #1531): how a run feels, read from the world and nothing else.
//! `AliveTally` is fed once an hour of game time by the caller (the soak and the alive probe;
//! `observe_hour`, and `observe_events` with each tick's events for the grief keys) and `report`
//! turns what it has seen into named keys. Each later alive slice adds its own
//! samples to the tally and its own keys to `report`; the keys are `(name, value)` rows so a
//! caller prints or compares them without knowing which slice added them.
//!
//! Read-only (D7): no rng, no wall clock, no map with an unstable order, and nothing here
//! changes what anyone does, so no digest moves.

use std::collections::{BTreeMap, BTreeSet};

use crate::agents::{Agent, Species};
use crate::events::{Event, EventKind, Severity};
use crate::ids::{AgentId, CommunityId, TilePos};
use crate::mind::learning::ObservationKind;
use crate::mind::Activity;
use crate::time::TICKS_PER_DAY;
use crate::world_state::World;

/// A mood at or above this counts as "high": 0.9 of the way to the +1.00 clamp.
const MOOD_HIGH: f32 = 0.9;
/// A mood at or above this is counted as sitting on the +1.00 clamp (`systems/needs.rs` clamps
/// to +-1). 0.995 is the lowest value that prints as +1.00 at two decimals, so it is what a
/// reader of the view sees as "at the clamp" even when the stored mood is a hair under 1.0.
const MOOD_CLAMPED: f32 = 0.995;
/// An hour's spread is recorded only when at least this many people were sampled: the s.d. of
/// fewer than five moods says little about whether people in one room differ.
const SPREAD_MIN_PEOPLE: usize = 5;
/// A person's activity mix is read (`day_distance`, `act_entropy_sd`) only once they have been
/// sampled this many hours: 2,000 hours is 83 game days (about four game years of 20 days,
/// docs/TIME.md), long enough for a mix to be a habit rather than one season's luck. The audit's
/// per-person table used the same cut, so the numbers stay comparable with it.
const ACT_MIN_HOURS: u32 = 2_000;

/// A tie at or above this opinion is counted as sitting on the +1.00 clamp (`Agent::relate`
/// clamps to +-1): 0.995 is the lowest value that prints as +1.00, as for `MOOD_CLAMPED`.
const TIE_SATURATED: f32 = 0.995;
/// A tie at or above this opinion is a close one, the kind that should fade without contact
/// (#1491): 0.95 is 19/20 of the way to the clamp, the audit's cut for "close".
const TIE_CLOSE: f32 = 0.95;
/// The Major events that name a person, which "nothing important is missed for ever"
/// (docs/NORTH-STAR.md) is read against (OST-004). Any other Major event (a raid, a collapse, ...)
/// is left out here.
const GLANCE_MAJOR_KINDS: [&str; 7] = [
    "died", "captured", "broke", "exiled", "born", "paired", "returned",
];
/// How close the camera's suggestion must come to a watched event's place, in tiles, to count as
/// having shown it: Chebyshev distance in the plane, on the same level.
const GLANCE_MAJOR_REACH: i32 = 12;
/// How long the camera has to show a watched event, in ticks: one game hour, counted from the
/// event's own tick. A sample taken later than this can no longer show it.
const GLANCE_MAJOR_WINDOW: u64 = 1_200;
/// Which `ObservationKind`s a `cause` string's lesson would have to warn about to count as
/// heeded (#1823, FIXLOG 2026-11-15 #1823): "drowned" (as in "drowned through the ice") reads a
/// lesson about water depth, current or cold; "thirst" and "froze" read one about cold; any other
/// cause has no cue, so `lesson_ignored` cannot be read for it (not "no lesson existed" from "a
/// lesson existed and was not heeded": there is nothing to tell those apart for a cause not in
/// this table).
const CAUSE_CUES: &[(&str, &[ObservationKind])] = &[
    (
        "drowned",
        &[
            ObservationKind::WaterDepth,
            ObservationKind::WaterCurrent,
            ObservationKind::Cold,
        ],
    ),
    ("thirst", &[ObservationKind::Cold]),
    ("froze", &[ObservationKind::Cold]),
];

/// The confidence a lesson about `cause` must reach to count as held (#1823): 0.5, the point the
/// issue draws between a hunch (`learn`'s starting 0.2 or 0.35) and something acted on.
const LESSON_HELD: f32 = 0.5;

/// The cues a death whose cause contains `cause` (a substring match, as "drowned through the ice"
/// contains "drowned") would have been warned about, or `&[]` when the cause matches no row of
/// `CAUSE_CUES`.
fn cues_for(cause: &str) -> &'static [ObservationKind] {
    CAUSE_CUES
        .iter()
        .find(|(key, _)| cause.contains(key))
        .map_or(&[], |(_, cues)| cues)
}

/// A tie with no contact for this long is frozen: 100 game days = 2,880,000 ticks (28,800 a
/// day, docs/TIME.md), which is five game years of 20 days. The audit used the same cut.
const TIE_FROZEN_TICKS: u64 = 100 * TICKS_PER_DAY;
/// How many of a person's strongest ties `ties_saturated` and `ties_hostile` read: the audit's
/// "top-5 ties", the few people someone is closest to or most set against.
const TIES_TOP: usize = 5;

/// How long a bereavement is followed: 60 game days = 1,728,000 ticks (28,800 a day, docs/TIME.md;
/// three game years of 20 days). It is the rubric's upper end for the return ("back within 0.10 of
/// baseline between 10 and 60 days", #1488 M2), so a person still low when the bereavement is this
/// old is "never"; one still low and younger is not read yet (#1492, FIXLOG second round).
const GRIEF_WINDOW: u64 = 60 * TICKS_PER_DAY;
/// `grief_drop_2d` reads the first 2 game days after the death: 57,600 ticks, the rubric's window
/// for the largest drop (#1488 M2: ">= 0.40 within 2 days").
const GRIEF_EARLY: u64 = 2 * TICKS_PER_DAY;
/// `grief_drop_10d` reads the drop 10 game days after the death: 288,000 ticks (#1488 M2).
const GRIEF_MARK: u64 = 10 * TICKS_PER_DAY;
/// The day-10 sample is the first one taken from the mark on, and only if it comes within this
/// long: the hourly look can be missing (the person was away), and a sample a day late is not the
/// day-10 sample. One game day = 28,800 ticks.
const GRIEF_MARK_SLACK: u64 = TICKS_PER_DAY;
/// A bereaved person is back when they stand no more than this far below their baseline: the
/// rubric's "within 0.10 of baseline" (a tenth of the -1..+1 mood range's width of 2, 5%).
const GRIEF_BACK: f32 = 0.10;

/// The mood terms are read once per 10 game days (OST-904): 288,000 ticks at 28,800 a day.
const TERMS_EVERY: u64 = 10 * TICKS_PER_DAY;
/// A mood below this counts toward `mood_below_m03`.
const MOOD_LOW: f32 = -0.3;

/// How many `Activity` names there are, and the order a person's hour counts are kept in. The
/// order is `Activity::name()`'s: idle, going, eating, foraging, sleeping, talking, loitering,
/// wandering, working, drinking. `slot` is an exhaustive match, so a new `Activity` variant
/// stops this file compiling until it is given a place here.
const ACTIVITIES: usize = 10;
const IDLE: usize = 0;
const GOING: usize = 1;
const SLEEPING: usize = 4;
const TALKING: usize = 5;
const LOITERING: usize = 6;
const WORKING: usize = 8;

/// Which of a person's ten counters an activity is added to.
fn slot(a: &Activity) -> usize {
    match a {
        Activity::Idle => IDLE,
        Activity::Going { .. } => GOING,
        Activity::Eating { .. } => 2,
        Activity::Foraging { .. } => 3,
        Activity::Sleeping { .. } => SLEEPING,
        Activity::Talking { .. } => TALKING,
        Activity::Loitering { .. } => LOITERING,
        Activity::Wandering { .. } => 7,
        Activity::Working { .. } => WORKING,
        Activity::Drinking { .. } => 9,
    }
}

/// What the client draws for each activity (OST-867). Index = the activity's counter slot, so
/// the rows are in `slot` order. `still` is the frame shown when the person is not moving and
/// `moving` the one shown while walking; `plain_standing` is true when `still` is the plain
/// standing loop, which a watcher cannot tell from doing nothing. Read from
/// `godot/scripts/render/entity_layer.gd` (the pose match in `"person"`) and
/// `crates/hf-godot/src/lib.rs::person_pose`. A fight, a fresh wound and the dead override all
/// of these; a person who carries something while moving is drawn with the carry cycle.
pub struct Drawn {
    pub activity: &'static str,
    pub still: &'static str,
    pub moving: &'static str,
    pub plain_standing: bool,
}

pub const READABLE: [Drawn; ACTIVITIES] = [
    Drawn {
        activity: "idle",
        still: "stand loop with a glance",
        moving: "walk",
        plain_standing: true,
    },
    Drawn {
        activity: "going",
        still: "stand loop with a glance",
        moving: "walk",
        plain_standing: true,
    },
    Drawn {
        activity: "eating",
        still: "eating cycle",
        moving: "walk",
        plain_standing: false,
    },
    Drawn {
        activity: "foraging",
        still: "stoop cycle",
        moving: "walk",
        plain_standing: false,
    },
    Drawn {
        activity: "sleeping",
        still: "sleep, breathing and stirring",
        moving: "walk",
        plain_standing: false,
    },
    Drawn {
        activity: "talking",
        still: "talking cycle",
        moving: "walk",
        plain_standing: false,
    },
    Drawn {
        activity: "loitering",
        still: "stand loop with a glance",
        moving: "walk",
        plain_standing: true,
    },
    Drawn {
        activity: "wandering",
        still: "stand loop with a glance",
        moving: "walk",
        plain_standing: true,
    },
    Drawn {
        activity: "working",
        still: "see `work_drawn`; stand loop when not at the work step",
        moving: "walk, or carry cycle",
        plain_standing: false,
    },
    Drawn {
        activity: "drinking",
        still: "drinking cycle",
        moving: "walk",
        plain_standing: false,
    },
];

/// What the client draws for this activity.
pub fn drawn(a: &Activity) -> &'static Drawn {
    &READABLE[slot(a)]
}

/// The frame and the emote the client shows at the work step of a designation of this kind
/// (the `"person"` pose-2 branch and the bridge's `agent_row` code). Every kind is listed so that
/// a new one must be given a place here.
pub fn work_drawn(kind: &crate::work::DesignationKind) -> (&'static str, &'static str) {
    use crate::work::DesignationKind as K;
    match kind {
        K::Chop => ("swing", "tool"),
        K::Mine | K::DigStairs | K::DigEarth | K::DigChannel => ("swing", "sledgehammer"),
        K::BuildWall(_)
        | K::BuildDoor
        | K::BuildStairs(_)
        | K::BuildFloor(_)
        | K::BuildRoof(_)
        | K::BuildStructure(..) => ("tap", "hammer"),
        K::Craft(_) => ("tap", "saw"),
        K::Cook => ("tap", "vessel"),
        K::Sow | K::Harvest => ("stoop", "tool"),
        K::BeatOut => ("stoop", "bucket"),
        K::Douse => ("stoop", "bucket"),
        K::Tend(_) => ("tend cycle", "poultice"),
        K::Hunt(_) => ("stoop", "weapon"),
        K::Study(_) => ("stoop", "record"),
        K::ClearRoof
        | K::Haul
        | K::Engrave
        | K::Fish
        | K::Snare
        | K::Tame(_)
        | K::FetchWater
        | K::Uproot => ("stoop", "none"),
    }
}

/// What the run has shown so far. Plain vectors, kept in the order the hours were observed.
#[derive(Clone, Debug, PartialEq)]
pub struct AliveTally {
    /// Every sampled person's mood at every observed hour.
    mood: Vec<f32>,
    /// The population s.d. of moods at each observed hour that sampled enough people.
    spread: Vec<f32>,
    /// Per game day: (sum of moods, people sampled), for the mean mood by day.
    by_day_mean: BTreeMap<u64, (f32, u32)>,
    /// Per person: the hours they were sampled in each activity, indexed by `slot` (#1490).
    activity_hours: BTreeMap<AgentId, [u32; ACTIVITIES]>,
    /// The sampled hours a person needs before the per-person activity keys read them.
    min_hours: u32,
    /// What the latest `observe_ties` saw (#1491); all zero until it is called.
    ties: TieCounts,
    /// (hostile, top-five) tie counts added up over the `observe_ties` calls, the one tie key
    /// that is a run's sum rather than the latest look (#1491: `ties_hostile`). It holds at most
    /// one look per tick: see `last_look`.
    hostile_pool: (u32, u32),
    /// The tick of the latest `observe_ties` and the counts it added to `hostile_pool`, so that
    /// a second look at the same tick can take that look's counts out again before it adds its
    /// own (#1491, FIXLOG 2026-11-14 #1491 second fix). `None` until the first look.
    last_look: Option<(u64, (u32, u32))>,
    /// Every bereavement followed so far, open or closed, in the order they were opened (#1492).
    grief: Vec<GriefTrack>,
    /// Each sampled person's mood minus the mean mood of the rest of their community at the latest
    /// `observe_hour` (#1492); a person alone in their community has none. Replaced each hour.
    last_rel: BTreeMap<AgentId, f32>,
    /// The tick of the latest `observe_hour` (#1492): how old a bereavement is at the latest look,
    /// whether or not the person was sampled at it (away, dead and out of any community are not).
    last_hour: u64,
    /// (community, cause) of every death `observe_events` has seen, so a later death with a cause
    /// already seen in the same community counts toward `cause_repeat` (#1823). A death with no
    /// community does not add here and is never a repeat.
    causes_seen: BTreeSet<(CommunityId, String)>,
    /// Deaths counted so far whose cause had already killed someone in the same community
    /// (#1823: `cause_repeat`).
    cause_repeat: u32,
    /// Deaths counted so far where the victim held a lesson (confidence >= 0.5) about the cause
    /// but it was not heeded (#1823: `lesson_ignored`).
    lesson_ignored: u32,
    /// Sums of the four mood terms (`systems::needs::mood_terms`) over every sampled person at
    /// each 10-day look, and how many person-looks that is (OST-904).
    term_sums: [f64; 4],
    term_n: u32,
    /// The 10-day window (tick over `TERMS_EVERY`) of the latest look, so each window is read once.
    terms_window: Option<u64>,
    /// Watched Major events (OST-004) still inside `GLANCE_MAJOR_WINDOW`, in the order seen.
    major_watch: Vec<MajorWatch>,
    /// Per kind in `GLANCE_MAJOR_KINDS`: (missed, all) of the watched events whose hour has run
    /// out. They are folded in here once settled, so a long run keeps no per-event record.
    major_counts: BTreeMap<&'static str, (u32, u32)>,
}

/// One Major event being watched for `glance_missed_major`.
#[derive(Clone, Debug, PartialEq)]
struct MajorWatch {
    kind: &'static str,
    at_tick: u64,
    /// The event's place; with `None` the event always settles as missed.
    at: Option<TilePos>,
    shown: bool,
}

impl Default for AliveTally {
    fn default() -> Self {
        Self::with_min_hours(ACT_MIN_HOURS)
    }
}

/// The counts of the latest `observe_ties`: `top` ties in everyone's top five, `saturated` of
/// them; `all` ties to someone the world holds (alive or dead, in the valley or away); the
/// `frozen` ties and the longest gap of any close tie in whole days; `to_dead` of the `all`
/// ties that are to someone found dead.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct TieCounts {
    top: u32,
    saturated: u32,
    all: u32,
    frozen: u32,
    close_max_days: u64,
    to_dead: u32,
}

impl TieCounts {
    fn share(&self, part: u32, whole: u32) -> f64 {
        if whole == 0 {
            return 0.0;
        }
        f64::from(part) / f64::from(whole)
    }
}

/// One bereavement being followed (#1492): who lost whom, when, how they stood against their
/// community the hour before, and how they have stood at each hour since.
#[derive(Clone, Debug, PartialEq)]
struct GriefTrack {
    who: AgentId,
    dead: AgentId,
    at_tick: u64,
    /// `last_rel` of `who` at the latest `observe_hour` before the death was seen.
    baseline: f32,
    /// (ticks since the death, `last_rel`) at each hour looked at, in time order, from the death
    /// to `GRIEF_WINDOW` after it.
    curve: Vec<(u64, f32)>,
}

impl GriefTrack {
    /// Days from the death until the person is back: the first look after the first one more than
    /// `GRIEF_BACK` below baseline at which they are no more than that far below it. Someone who
    /// was never that far below reads 0.0 days. Someone who left and is not back reads `None`
    /// (not read yet) until the bereavement is `GRIEF_WINDOW` old at `latest`, the tick of the
    /// latest look, and 60.0 from then on (the rubric's "60 if never"): the curve stops at 60
    /// days, so they cannot come back later in it. A younger one may still come back, so counting
    /// it as 60 would push the median to "never" for every recent death at the end of a run
    /// (FIXLOG 2026-11-15 #1492, second round).
    fn days_back(&self, latest: u64) -> Option<f32> {
        let mut left = false;
        for &(since, rel) in &self.curve {
            if self.baseline - rel > GRIEF_BACK {
                left = true;
            } else if left {
                return Some(since as f32 / TICKS_PER_DAY as f32);
            }
        }
        if !left {
            return Some(0.0);
        }
        let aged = latest.saturating_sub(self.at_tick) >= GRIEF_WINDOW;
        aged.then_some((GRIEF_WINDOW / TICKS_PER_DAY) as f32)
    }
}

/// Named keys and their values, in a fixed order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AliveReport {
    pub rows: Vec<(&'static str, f64)>,
}

impl AliveReport {
    /// The value of one key, if the report has it.
    pub fn get(&self, key: &str) -> Option<f64> {
        self.rows.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
    }
}

impl AliveTally {
    /// An empty tally whose per-person keys (`day_distance`, `act_entropy_sd`) read only people
    /// sampled at least `min_hours` hours. `default()` uses 2,000; a test with a short run lowers it.
    pub fn with_min_hours(min_hours: u32) -> Self {
        AliveTally {
            mood: Vec::new(),
            spread: Vec::new(),
            by_day_mean: BTreeMap::new(),
            activity_hours: BTreeMap::new(),
            min_hours,
            ties: TieCounts::default(),
            hostile_pool: (0, 0),
            last_look: None,
            grief: Vec::new(),
            last_rel: BTreeMap::new(),
            last_hour: 0,
            causes_seen: BTreeSet::new(),
            cause_repeat: 0,
            lesson_ignored: 0,
            term_sums: [0.0; 4],
            term_n: 0,
            terms_window: None,
            major_watch: Vec::new(),
            major_counts: BTreeMap::new(),
        }
    }

    /// Sample this hour: the mood of every living person who belongs to a community, and,
    /// when five or more were sampled, the s.d. of those moods; and which activity each of them
    /// is in, kept per person (#1490).
    pub fn observe_hour(&mut self, w: &World) {
        let people: Vec<_> = w
            .agents
            .iter()
            .filter(|a| a.alive && a.species == Species::Person && a.community.is_some())
            .collect();
        for a in &people {
            self.activity_hours.entry(a.id).or_default()[slot(&a.activity)] += 1;
        }
        self.observe_grief(w.meta.tick, &people);
        let window = w.meta.tick / TERMS_EVERY;
        if self.terms_window != Some(window) {
            self.terms_window = Some(window);
            for a in &people {
                let t = crate::systems::needs::mood_terms(a, w.meta.tick);
                for (sum, x) in self.term_sums.iter_mut().zip(t) {
                    *sum += f64::from(x);
                }
                self.term_n += 1;
            }
        }
        let moods: Vec<f32> = people.iter().map(|a| a.mood).collect();
        if moods.is_empty() {
            return;
        }
        let day = self
            .by_day_mean
            .entry(w.meta.tick / TICKS_PER_DAY)
            .or_default();
        day.0 += moods.iter().sum::<f32>();
        day.1 += moods.len() as u32;
        if moods.len() >= SPREAD_MIN_PEOPLE {
            self.spread.push(std_dev(&moods));
        }
        self.mood.extend(moods);
    }

    /// Look at the ties of every living person who belongs to a community (the people the other
    /// keys sample) and keep what is seen, replacing the last look: the tie keys describe the
    /// world at the latest call, except `ties_hostile`, which adds the calls up (#1491; FIXLOG
    /// 2026-11-14 #1491). The caller decides when to look (the end of a run and every 30 days),
    /// and on a run whose length is a multiple of 30 days the two are the same tick. A look at
    /// the same tick as the last one therefore replaces that look's counts in the pool, so the
    /// pool holds at most one look per tick, the latest; a look at an earlier tick than the last
    /// one (not expected) is added like any other.
    pub fn observe_ties(&mut self, w: &World) {
        // Who the world holds, and whether they live: everyone in the valley and everyone off the
        // map (a journey, a captive, a taken child; `w.overworld.away` keeps them whole and alive,
        // and someone who dies away is put back in `w.agents` first, systems/expedition.rs). A
        // tie to an id found in neither is to no one the world can say is dead, so it is read as
        // neither dead nor living (FIXLOG 2026-11-14 #1491 fix). No one is in both lists, so the
        // order of the chain below decides nothing; were someone in both, `w.agents` (inserted
        // last) would win.
        let mut alive_of: BTreeMap<AgentId, bool> = BTreeMap::new();
        for a in w.overworld.away.iter().chain(w.agents.iter()) {
            alive_of.insert(a.id, a.alive);
        }
        let now = w.meta.tick;
        let mut c = TieCounts::default();
        let mut hostile = 0;
        // Whose ties are read: the people `observe_hour` samples, so only those in `w.agents`.
        // A colonist away on a journey or held captive is in `w.overworld.away`, and their own
        // ties are not read while they are away (they are still the other end of someone's
        // tie, above); FIXLOG 2026-11-14 #1491 second fix.
        for a in w
            .agents
            .iter()
            .filter(|a| a.alive && a.species == Species::Person && a.community.is_some())
        {
            // The map is in id order, and the sort is stable, so equally strong ties rank the
            // lower id first.
            let mut opinions = Vec::new();
            for (other, r) in &a.relationships {
                opinions.push(r.opinion);
                if let Some(alive) = alive_of.get(other) {
                    c.all += 1;
                    c.to_dead += u32::from(!alive);
                }
                let gap = now.saturating_sub(r.last_contact);
                if r.opinion >= TIE_CLOSE {
                    c.close_max_days = c.close_max_days.max(gap / TICKS_PER_DAY);
                    c.frozen += u32::from(gap >= TIE_FROZEN_TICKS);
                }
            }
            opinions.sort_by(|x, y| y.abs().total_cmp(&x.abs()));
            for o in opinions.into_iter().take(TIES_TOP) {
                c.top += 1;
                c.saturated += u32::from(o >= TIE_SATURATED);
                hostile += u32::from(o < 0.0);
            }
        }
        // A second look at the tick of the last one takes that look's counts out first, so a run
        // whose end falls on a 30-day look does not weigh its last sample twice (FIXLOG
        // 2026-11-14 #1491 second fix). `last_look` always holds what the latest call added, so
        // the subtraction cannot underflow.
        if let Some((tick, (h, t))) = self.last_look {
            if tick == now {
                self.hostile_pool.0 -= h;
                self.hostile_pool.1 -= t;
            }
        }
        self.hostile_pool.0 += hostile;
        self.hostile_pool.1 += c.top;
        self.last_look = Some((now, (hostile, c.top)));
        self.ties = c;
    }

    /// Read this tick's events and open a bereavement for each living, sampled person close to
    /// someone who died (#1492; FIXLOG 2026-11-15 #1492). `last_rel` at the latest `observe_hour`
    /// is their baseline, so call this after the hour before the death and before the hours
    /// after it. The bereaved are the dead person's partner, mother, father and children, a link
    /// read from either end: `die` clears the survivor's `partner` but not the dead person's, so
    /// a partner can only be found from the dead person's record, and a child's mother or father
    /// is found from the child's. Who is not bereaved: siblings, grandparents, friends, anyone
    /// with no baseline (no hour seen yet, no community or away at the last hour, alone in their
    /// community), anyone not alive and in `w.agents` now (a person away is not looked at), and
    /// anyone at all when the dead is an animal. The same death fed twice opens one bereavement
    /// per bereaved person; a person who lost two people has two.
    pub fn observe_events(&mut self, w: &World, events: &[Event]) {
        for e in events {
            let EventKind::Died { agent, cause } = &e.kind else {
                continue;
            };
            let dead = *agent;
            let record = w.agents.iter().find(|a| a.id == dead);
            if record.is_some_and(|d| d.species != Species::Person) {
                continue;
            }
            self.observe_death_cause(record, cause);
            let mut kin: BTreeSet<AgentId> = BTreeSet::new();
            if let Some(d) = record {
                kin.extend(
                    d.partner
                        .into_iter()
                        .chain(d.mother)
                        .chain(d.father)
                        .chain(d.children.iter().copied()),
                );
            }
            kin.extend(
                w.agents
                    .iter()
                    .filter(|a| {
                        a.partner == Some(dead)
                            || a.mother == Some(dead)
                            || a.father == Some(dead)
                            || a.children.contains(&dead)
                    })
                    .map(|a| a.id),
            );
            for who in kin {
                let alive = w.agents.iter().any(|a| a.id == who && a.alive);
                let Some(&baseline) = self.last_rel.get(&who) else {
                    continue;
                };
                let known = self.grief.iter().any(|t| t.who == who && t.dead == dead);
                if who == dead || !alive || known {
                    continue;
                }
                self.grief.push(GriefTrack {
                    who,
                    dead,
                    at_tick: e.tick,
                    baseline,
                    curve: Vec::new(),
                });
            }
        }
    }

    /// `cause_repeat` and `lesson_ignored` (#1823, FIXLOG 2026-11-15 #1823), for one death whose
    /// victim is `record` (still in `w.agents`, `alive == false`; absent for a death the world
    /// does not hold). `cause_repeat` counts a death whose cause has already killed someone in
    /// the same community; a death with no community is never a repeat and is never recorded as
    /// a cause seen. `lesson_ignored` counts a death whose victim held a lesson (confidence
    /// `>=` 0.5) whose cue is one `cues_for(cause)` lists, read whether or not the cause repeats.
    fn observe_death_cause(&mut self, record: Option<&Agent>, cause: &str) {
        if let Some(community) = record.and_then(|d| d.community) {
            let seen = (community, cause.to_string());
            if !self.causes_seen.insert(seen) {
                self.cause_repeat += 1;
            }
        }
        let cues = cues_for(cause);
        if !cues.is_empty()
            && record.is_some_and(|d| {
                d.learning
                    .lessons
                    .iter()
                    .any(|l| l.confidence >= LESSON_HELD && cues.contains(&l.cue))
            })
        {
            self.lesson_ignored += 1;
        }
    }

    /// The hourly part of the grief keys (#1492): work out each sampled person's mood against the
    /// mean of the rest of their community, keep it as `last_rel`, and add it to the curve of every
    /// bereavement of theirs that is no more than `GRIEF_WINDOW` old. The rest of the community is
    /// the other sampled people with the same community id (the dead, people away, people with no
    /// community and animals are not sampled); someone alone in theirs has no mean to be measured
    /// against, so no sample. A look at the tick of the curve's last point replaces it; a look at
    /// an earlier tick, or before the death, is not added to any curve. `last_hour` keeps the
    /// latest tick looked at, for the age of a bereavement in `grief_keys`.
    fn observe_grief(&mut self, now: u64, people: &[&Agent]) {
        self.last_hour = self.last_hour.max(now);
        let mut sums: BTreeMap<CommunityId, (f64, u32)> = BTreeMap::new();
        for a in people {
            if let Some(c) = a.community {
                let s = sums.entry(c).or_default();
                s.0 += f64::from(a.mood);
                s.1 += 1;
            }
        }
        self.last_rel = people
            .iter()
            .filter_map(|a| {
                let (sum, n) = sums[&a.community?];
                if n < 2 {
                    return None;
                }
                let rest = (sum - f64::from(a.mood)) / f64::from(n - 1);
                Some((a.id, (f64::from(a.mood) - rest) as f32))
            })
            .collect();
        for t in &mut self.grief {
            let Some(since) = now.checked_sub(t.at_tick).filter(|s| *s <= GRIEF_WINDOW) else {
                continue;
            };
            let Some(&rel) = self.last_rel.get(&t.who) else {
                continue;
            };
            match t.curve.last_mut() {
                Some(last) if last.0 == since => last.1 = rel,
                Some(last) if last.0 > since => {}
                _ => t.curve.push((since, rel)),
            }
        }
    }

    /// Watch this tick's Major events of the kinds in `GLANCE_MAJOR_KINDS` (OST-004) for the
    /// `glance_missed_major` keys. Call it once a tick with the events the viewer may be shown;
    /// events of another kind or severity are ignored. The caller decides which events the camera
    /// could have shown: the tally counts what it is given.
    pub fn observe_major_events(&mut self, events: &[Event]) {
        for e in events {
            if e.severity != Severity::Major {
                continue;
            }
            let kind = e.kind_name();
            if !GLANCE_MAJOR_KINDS.contains(&kind) {
                continue;
            }
            self.major_watch.push(MajorWatch {
                kind,
                at_tick: e.tick,
                at: e.at,
                shown: false,
            });
        }
    }

    /// Feed one camera sample (OST-004): the tick it was taken and the place the director
    /// suggested at it. A watched event is marked shown when the sample is within
    /// `GLANCE_MAJOR_REACH` tiles of its place on the same level **and** no more than
    /// `GLANCE_MAJOR_WINDOW` ticks after the event; a later sample, or one taken before the event
    /// existed, shows nothing. Then every event older than the window settles into the counts,
    /// shown or missed. An event with no place is never marked, so it settles missed.
    pub fn observe_glance(&mut self, tick: u64, suggestion_at: TilePos) {
        for m in self.major_watch.iter_mut().filter(|m| !m.shown) {
            let Some(p) = m.at else { continue };
            let age = tick.checked_sub(m.at_tick);
            let in_time = age.is_some_and(|a| a <= GLANCE_MAJOR_WINDOW);
            let close = p.z == suggestion_at.z
                && (p.x - suggestion_at.x)
                    .abs()
                    .max((p.y - suggestion_at.y).abs())
                    <= GLANCE_MAJOR_REACH;
            if in_time && close {
                m.shown = true;
            }
        }
        let (settled, watching): (Vec<_>, Vec<_>) = std::mem::take(&mut self.major_watch)
            .into_iter()
            .partition(|m| tick.saturating_sub(m.at_tick) > GLANCE_MAJOR_WINDOW);
        self.major_watch = watching;
        for m in settled {
            let c = self.major_counts.entry(m.kind).or_default();
            c.1 += 1;
            c.0 += u32::from(!m.shown);
        }
    }

    /// Per kind in `GLANCE_MAJOR_KINDS`, `(kind, missed, all)` over the settled watched events:
    /// the raw counts behind the `glance_missed_major` shares.
    pub fn glance_major_counts(&self) -> Vec<(&'static str, u32, u32)> {
        GLANCE_MAJOR_KINDS
            .iter()
            .map(|&k| {
                let (missed, all) = self.major_counts.get(k).copied().unwrap_or((0, 0));
                (k, missed, all)
            })
            .collect()
    }

    /// Watched events still inside their hour: not in any count yet, so a run that ends now
    /// leaves them out.
    pub fn glance_major_pending(&self) -> usize {
        self.major_watch.len()
    }

    /// (`glance_missed_major`, `glance_major_n`, the share per kind): see `report`.
    fn glance_keys(&self) -> (f64, f64, Vec<f64>) {
        let share = |missed: u32, all: u32| {
            if all == 0 {
                0.0
            } else {
                f64::from(missed) / f64::from(all)
            }
        };
        let counts = self.glance_major_counts();
        let (missed, all) = counts
            .iter()
            .fold((0, 0), |a, &(_, m, n)| (a.0 + m, a.1 + n));
        let by_kind = counts.iter().map(|&(_, m, n)| share(m, n)).collect();
        (share(missed, all), f64::from(all), by_kind)
    }

    /// (`grief_drop_2d`, `grief_drop_10d`, `grief_days`, `grief_n`): see `report`.
    fn grief_keys(&self) -> (f64, f64, f64, f64) {
        let (mut early, mut mark, mut days) = (Vec::new(), Vec::new(), Vec::new());
        for t in self.grief.iter().filter(|t| !t.curve.is_empty()) {
            let drop = |rel: f32| t.baseline - rel;
            let worst = t
                .curve
                .iter()
                .filter(|(since, _)| *since <= GRIEF_EARLY)
                .map(|(_, rel)| drop(*rel))
                .max_by(f32::total_cmp);
            early.extend(worst);
            let at_mark = t.curve.iter().find(|(since, _)| *since >= GRIEF_MARK);
            mark.extend(
                at_mark
                    .filter(|(since, _)| *since < GRIEF_MARK + GRIEF_MARK_SLACK)
                    .map(|(_, rel)| drop(*rel)),
            );
            days.extend(t.days_back(self.last_hour));
        }
        for v in [&mut early, &mut mark, &mut days] {
            v.sort_by(f32::total_cmp);
        }
        (
            median(&early),
            median(&mark),
            median(&days),
            self.grief.len() as f64,
        )
    }

    /// The keys. An empty tally gives every key as 0.0, so the rows keep their shape.
    ///
    /// - `mood_median`, `mood_p10`: the median and the 10th percentile (the sample at rank
    ///   floor(0.1 * (n - 1)) once sorted) of all sampled moods.
    /// - `mood_hi`, `mood_lo`, `mood_clamped`: the share of samples at >= 0.9, below 0, and
    ///   >= 0.995.
    /// - `mood_spread`: the median over the observed hours of that hour's s.d.
    /// - `mood_p25`, `mood_p75`: the 25th and 75th percentiles (rank floor(q * (n - 1))) of all
    ///   sampled moods; `mood_below_m03`: the share below -0.3 (OST-904).
    /// - `mood_term_needs`, `mood_term_recent`, `mood_term_long`, `mood_term_thoughts`: the mean
    ///   of each term of `systems::needs::mood_terms` over every sampled person at the first look
    ///   of each 10-day window (OST-904); 0.0 before any look.
    ///
    /// The activity keys (#1490; FIXLOG 2026-11-14 #1490) read the same sampled people. "Waking
    /// hours" are all sampled person-hours except `sleeping`.
    /// - `standing_share`, `unreadable_share`, `work_share`, `talk_share`: the share of waking
    ///   hours in idle + loitering; idle + loitering + going; working; talking. 0.0 when nobody
    ///   was sampled awake.
    /// - `day_distance`: over people sampled at least `min_hours` hours, each person's share
    ///   vector is their hours in each of the ten activities (sleeping included) over their own
    ///   sampled hours; the colony's mean day is the mean of those vectors (each person counts
    ///   once); a person's distance is half the L1 distance from it, in points (0 to 100); the key
    ///   is the median of the distances (the mean of the two middle ones for an even count).
    /// - `act_entropy_sd`: over the same people, the population s.d. (divide by n) of each
    ///   person's Shannon entropy in bits (log base 2) of their share vector.
    ///
    /// The tie keys (#1491; FIXLOG 2026-11-14 #1491) are all 0.0 before `observe_ties` is called
    /// or when nobody has a tie. All but `ties_hostile` read the latest `observe_ties` only
    /// (the rubric reads saturation "at the last sample", frozen ties "at the end"); a person's
    /// top five are their five ties of greatest `opinion.abs()` (equal strength: lower id first;
    /// fewer than five: all).
    /// - `ties_saturated`: the share of everyone's top-five ties, pooled, at opinion >= 0.995.
    /// - `ties_hostile`: the share of top-five ties below 0, pooled over the `observe_ties`
    ///   calls, one look per tick (the rubric's key counts every sample it takes; a second look
    ///   at the same tick replaces the first in the pool).
    /// - `ties_frozen`: the count of all ties at opinion >= 0.95 whose last contact is 100 or
    ///   more days (2,880,000 ticks) before the tick; `ties_frozen_max_days`: the longest gap
    ///   since contact of any tie at opinion >= 0.95, frozen or not, in whole days (floor).
    /// - `ties_to_dead_share`: of all ties to someone the world holds (in `w.agents`, or away in
    ///   `w.overworld.away`), the share to someone found with `alive == false`. A tie to an id
    ///   found nowhere is neither dead nor counted.
    ///
    /// The grief keys (#1492; FIXLOG 2026-11-15 #1492) come from the bereavements `observe_events`
    /// opened (who counts as bereaved: see there) and are all 0.0 before any is. A bereavement's
    /// baseline is the person's mood minus the mean mood of the rest of their community (the other
    /// sampled people with the same community id; the dead, people away, people with no community
    /// and animals are not sampled) at the latest `observe_hour` before the death was fed; its
    /// curve is the same figure at each later `observe_hour` for 60 days (1,728,000 ticks, the
    /// last look counted if it is at exactly that age). A person alone in their community has no
    /// figure, so no point on the curve. A bereaved person away, dead or out of any community at a
    /// look has no point for it: the curve keeps what it has. The drop at a point is baseline
    /// minus that figure, so a mood above baseline is a negative drop.
    /// The death-cause keys (#1823, FIXLOG 2026-11-15 #1823) come from every death `observe_events`
    /// has seen (whether or not it opens a bereavement) and are counts, not shares.
    /// - `cause_repeat`: deaths whose cause had already killed someone in the same community.
    /// - `lesson_ignored`: deaths where the victim held a lesson (`confidence >= 0.5`) whose cue
    ///   `cues_for` maps the cause to, so the two can tell "a lesson existed and was ignored" from
    ///   "there was nothing to learn".
    /// - `grief_n`: the bereavements opened (one per bereaved person per death), including any
    ///   with no point yet; each key below reads the bereavements that have what it needs, so it
    ///   can rest on fewer than `grief_n`.
    /// - `grief_drop_2d`: the median over bereavements with a point at or before 2 days (57,600
    ///   ticks) of their largest drop among those points.
    /// - `grief_drop_10d`: the median of the drop at the first point at or after 10 days (288,000
    ///   ticks) and before 11 days, over bereavements that have one.
    /// - `grief_days`: the median over bereavements with a point of the days (fractional: ticks
    ///   over 28,800) from the death to the first point no more than 0.10 below baseline after the
    ///   first point more than 0.10 below it; 0.0 if no point was ever more than 0.10 below
    ///   baseline. One who was and has not come back is left out of the median until the
    ///   bereavement is 60 days old at the latest `observe_hour` (whether or not the person was
    ///   sampled at it), and reads 60.0 from then on, as does one who died or is away: a recent
    ///   death is not "never" yet, as `grief_drop_10d` leaves out the ones with no day-10 point.
    ///   Medians of an even count are the mean of the two middle values.
    ///
    /// The glance keys (OST-004) read the Major events `observe_major_events` was fed against the
    /// samples `observe_glance` was fed, and are all 0.0 until an event has settled.
    /// - `glance_major_n`: the watched `died`, `captured`, `broke`, `exiled`, `born`, `paired` and
    ///   `returned` events whose hour has run out. Events still inside it are not counted
    ///   (`glance_major_pending`).
    /// - `glance_missed_major`: of those, the share never shown: no sample came within 12 tiles
    ///   of the event's place, on the same level, no later than 1,200 ticks (one game hour) after
    ///   the event. An event with no place is always missed.
    /// - `glance_missed_major_died`, `..._captured`, `..._broke`, `..._exiled`, `..._born`,
    ///   `..._paired`, `..._returned`: the same share for one kind.
    pub fn report(&self) -> AliveReport {
        let mut sorted = self.mood.clone();
        sorted.sort_by(f32::total_cmp);
        let share = |hit: &dyn Fn(f32) -> bool| {
            if sorted.is_empty() {
                return 0.0;
            }
            sorted.iter().filter(|m| hit(**m)).count() as f64 / sorted.len() as f64
        };
        let mut spread = self.spread.clone();
        spread.sort_by(f32::total_cmp);
        let (standing, unreadable, work, talk) = self.waking_shares();
        let (day_distance, act_entropy_sd) = self.person_mix_keys();
        let (grief_2d, grief_10d, grief_days, grief_n) = self.grief_keys();
        let (glance_missed, glance_n, glance_by_kind) = self.glance_keys();
        let mut rows = vec![
            ("mood_median", median(&sorted)),
            ("mood_p10", percentile10(&sorted)),
            ("mood_hi", share(&|m| m >= MOOD_HIGH)),
            ("mood_lo", share(&|m| m < 0.0)),
            ("mood_clamped", share(&|m| m >= MOOD_CLAMPED)),
            ("mood_spread", median(&spread)),
            ("mood_p25", quantile(&sorted, 25)),
            ("mood_p75", quantile(&sorted, 75)),
            ("mood_below_m03", share(&|m| m < MOOD_LOW)),
            ("mood_term_needs", self.term_mean(0)),
            ("mood_term_recent", self.term_mean(1)),
            ("mood_term_long", self.term_mean(2)),
            ("mood_term_thoughts", self.term_mean(3)),
            ("standing_share", standing),
            ("unreadable_share", unreadable),
            ("work_share", work),
            ("talk_share", talk),
            ("day_distance", day_distance),
            ("act_entropy_sd", act_entropy_sd),
            (
                "ties_saturated",
                self.ties.share(self.ties.saturated, self.ties.top),
            ),
            (
                "ties_hostile",
                self.ties.share(self.hostile_pool.0, self.hostile_pool.1),
            ),
            ("ties_frozen", f64::from(self.ties.frozen)),
            ("ties_frozen_max_days", self.ties.close_max_days as f64),
            (
                "ties_to_dead_share",
                self.ties.share(self.ties.to_dead, self.ties.all),
            ),
            ("grief_drop_2d", grief_2d),
            ("grief_drop_10d", grief_10d),
            ("grief_days", grief_days),
            ("grief_n", grief_n),
            ("cause_repeat", f64::from(self.cause_repeat)),
            ("lesson_ignored", f64::from(self.lesson_ignored)),
            ("glance_major_n", glance_n),
            ("glance_missed_major", glance_missed),
        ];
        const GLANCE_KEYS: [&str; 7] = [
            "glance_missed_major_died",
            "glance_missed_major_captured",
            "glance_missed_major_broke",
            "glance_missed_major_exiled",
            "glance_missed_major_born",
            "glance_missed_major_paired",
            "glance_missed_major_returned",
        ];
        rows.extend(GLANCE_KEYS.into_iter().zip(glance_by_kind));
        AliveReport { rows }
    }

    /// The mean of mood term `i` over the 10-day looks; 0.0 when there is none.
    fn term_mean(&self, i: usize) -> f64 {
        if self.term_n == 0 {
            return 0.0;
        }
        self.term_sums[i] / f64::from(self.term_n)
    }

    /// (standing, unreadable, work, talk) as shares of every sampled person's waking hours.
    fn waking_shares(&self) -> (f64, f64, f64, f64) {
        let mut total = [0u64; ACTIVITIES];
        for hours in self.activity_hours.values() {
            for (t, h) in total.iter_mut().zip(hours) {
                *t += u64::from(*h);
            }
        }
        let waking = total.iter().sum::<u64>() - total[SLEEPING];
        if waking == 0 {
            return (0.0, 0.0, 0.0, 0.0);
        }
        let share = |hours: u64| hours as f64 / waking as f64;
        (
            share(total[IDLE] + total[LOITERING]),
            share(total[IDLE] + total[LOITERING] + total[GOING]),
            share(total[WORKING]),
            share(total[TALKING]),
        )
    }

    /// (`day_distance`, `act_entropy_sd`) over the people sampled at least `min_hours` hours;
    /// both 0.0 when none were. The per-person arithmetic is f64; the median and s.d. helpers
    /// take f32, which is far finer than a key printed to a point or a hundredth of a bit.
    fn person_mix_keys(&self) -> (f64, f64) {
        let mixes: Vec<[f64; ACTIVITIES]> = self
            .activity_hours
            .values()
            .filter(|h| h.iter().sum::<u32>() >= self.min_hours)
            .map(|h| {
                let n = f64::from(h.iter().sum::<u32>());
                h.map(|c| f64::from(c) / n)
            })
            .collect();
        if mixes.is_empty() {
            return (0.0, 0.0);
        }
        let mut mean = [0.0f64; ACTIVITIES];
        for m in &mixes {
            for (a, b) in mean.iter_mut().zip(m) {
                *a += b / mixes.len() as f64;
            }
        }
        let mut distance: Vec<f32> = mixes
            .iter()
            .map(|m| {
                let l1: f64 = m.iter().zip(&mean).map(|(a, b)| (a - b).abs()).sum();
                (l1 / 2.0 * 100.0) as f32
            })
            .collect();
        distance.sort_by(f32::total_cmp);
        let entropy: Vec<f32> = mixes
            .iter()
            .map(|m| {
                -m.iter()
                    .filter(|p| **p > 0.0)
                    .map(|p| p * p.log2())
                    .sum::<f64>() as f32
            })
            .collect();
        (median(&distance), f64::from(std_dev(&entropy)))
    }
}

/// Population standard deviation (divide by n, not n - 1: these are everyone present).
fn std_dev(v: &[f32]) -> f32 {
    let n = v.len() as f32;
    let mean = v.iter().sum::<f32>() / n;
    (v.iter().map(|x| (x - mean) * (x - mean)).sum::<f32>() / n).sqrt()
}

/// Median of an already sorted slice; 0.0 when empty.
fn median(sorted: &[f32]) -> f64 {
    match sorted.len() {
        0 => 0.0,
        n if n % 2 == 1 => sorted[n / 2] as f64,
        n => (sorted[n / 2 - 1] as f64 + sorted[n / 2] as f64) / 2.0,
    }
}

/// The 10th percentile of an already sorted slice; 0.0 when empty.
fn percentile10(sorted: &[f32]) -> f64 {
    match sorted.len() {
        0 => 0.0,
        n => sorted[(n - 1) / 10] as f64,
    }
}

/// The `pct`th percentile of an already sorted slice: the sample at rank floor(pct * (n - 1) /
/// 100); 0.0 when empty.
fn quantile(sorted: &[f32], pct: usize) -> f64 {
    match sorted.len() {
        0 => 0.0,
        n => sorted[(n - 1) * pct / 100] as f64,
    }
}
