//! The deity (design-organic.md section 8): the storyteller, with motives and triggers.
//!
//! A deity fires nothing. Once a day it looks at the world: the settlement in the valley at full
//! detail and the places beyond it at the grain of their food stores. When something it cares
//! about is true (plenty grown idle, a hard stretch, a killing, a long quiet, a neighbour starving
//! or fat), it answers by leaning one of the world's own mechanisms for some days: a place's
//! harvest, the cold, dry storms, sickness, the herds, the boldness of wolves. Generators read
//! those leans through `lean()`. A raid still comes from people who are hungry; the deity only
//! made their harvest fail. Every answer is kept with the trigger and day that caused it, and
//! recorded as a Trace event, so History can explain it while the feed never shows it.
//!
//! The three deities are the three old Engine modes, so saves keep their choice:
//! Steady is Aun, who keeps the balance; Relentless is Hask, who tests the strong; Whimsical is
//! Mirre, who is amused.

use crate::engine::Engine;
use crate::events::{EventKind, Severity};
use crate::rng::{purpose, stream};
use crate::time::{Season, TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::World;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Motive {
    /// Lift what is low, lower what is high.
    Balance,
    /// Press the strong; leave the broken be.
    Test,
    /// Whatever would be interesting.
    Amusement,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Trigger {
    /// The settlement has more than enough and has had for days.
    Plenty,
    /// The settlement is in a hard stretch: hunger, sickness, deaths.
    Hardship,
    /// Someone was killed by another's hand.
    Killing,
    /// Nothing has happened for days.
    LongQuiet,
    /// A place beyond the valley has gone hungry.
    SiteHunger,
    /// A place beyond the valley is fat with stores.
    SiteFat,
}

impl Trigger {
    pub fn label(self) -> &'static str {
        match self {
            Trigger::Plenty => "plenty",
            Trigger::Hardship => "a hard stretch",
            Trigger::Killing => "a killing",
            Trigger::LongQuiet => "a long quiet",
            Trigger::SiteHunger => "hunger",
            Trigger::SiteFat => "full stores",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Lever {
    /// Multiplier on a place's harvest (threat.rs sites_daily).
    Harvest,
    /// Degrees added to the temperature (negative is colder).
    Cold,
    /// Multiplier on dry thunder (hazards.rs).
    DryStorms,
    /// Multiplier on catching a winter sickness (health.rs).
    Sickness,
    /// Multiplier on wild herds breeding (animals.rs).
    Herds,
    /// Above 1, wolves are hungry sooner (animals.rs).
    WolfHunger,
}

impl Lever {
    pub fn label(self) -> &'static str {
        match self {
            Lever::Harvest => "the harvest",
            Lever::Cold => "the cold",
            Lever::DryStorms => "dry storms",
            Lever::Sickness => "sickness",
            Lever::Herds => "the herds",
            Lever::WolfHunger => "the wolves",
        }
    }
    /// Levers that add (degrees) rather than multiply.
    fn additive(self) -> bool {
        self == Lever::Cold
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Scope {
    /// The valley the settlement is in.
    Valley,
    /// A place beyond it (expedition.rs Site id).
    Site(u32),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Answer {
    pub trigger: Trigger,
    pub lever: Lever,
    pub scope: Scope,
    /// Multiplier, or degrees for Cold.
    pub strength: f32,
    pub from: u64,
    pub until: u64,
    /// The Trace event that records it.
    pub event: crate::ids::EventId,
}

/// What the deity has been noticing, day to day.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Watch {
    pub plenty_days: u32,
    pub hard_days: u32,
    pub last_major: u64,
    pub last_killing: u64,
    pub killing_seen: u64,
    /// Who struck the last killing blow, if the record knows (P11).
    #[serde(default)]
    pub last_killer: Option<(crate::ids::AgentId, crate::ids::AgentId)>,
}

pub struct Deity {
    pub name: &'static str,
    pub title: &'static str,
    pub motive: Motive,
}

pub fn of(engine: Engine) -> Deity {
    match engine {
        Engine::Steady => Deity {
            name: "Aun",
            title: "who keeps the balance",
            motive: Motive::Balance,
        },
        Engine::Relentless => Deity {
            name: "Hask",
            title: "who tests the strong",
            motive: Motive::Test,
        },
        Engine::Whimsical => Deity {
            name: "Mirre",
            title: "who is amused",
            motive: Motive::Amusement,
        },
    }
}

/// What each deity notices and how it answers, for the new-game screen.
pub fn about(engine: Engine) -> &'static str {
    match engine {
        Engine::Steady => {
            "Notices a place grown fat, or one stuck in a long hard stretch, and leans its \
             harvest the other way. Valleys under Aun even out over the years."
        }
        Engine::Relentless => {
            "Notices whoever is doing well, a place well peopled or long untroubled, and \
             answers with dry storms and fever. The strong are tested; the broken are let be."
        }
        Engine::Whimsical => {
            "Notices a place where nothing has happened for too long, and leans on whatever \
             would be interesting, kind or cruel."
        }
    }
}

/// How hard the deity currently leans `lever` over `scope`: the product of active multipliers
/// (1.0 when none), or for Cold the sum of degrees (0.0 when none).
pub fn lean(w: &World, lever: Lever, scope: Scope) -> f32 {
    let now = w.meta.tick;
    let active = w
        .deity_answers
        .iter()
        .filter(|a| a.lever == lever && a.scope == scope && a.from <= now && now < a.until);
    if lever.additive() {
        active.map(|a| a.strength).sum()
    } else {
        active.map(|a| a.strength).product()
    }
}

/// Every tick: notice what happened (cheap). At 04:00: decide.
pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    for e in &w.last_events {
        if e.severity >= Severity::Major {
            w.deity_watch.last_major = e.tick;
        }
        if let EventKind::Died { cause, agent } = &e.kind {
            if cause.contains("fight") || cause.contains("killed") || cause.contains("stranger") {
                w.deity_watch.last_killing = e.tick;
                // The one whose blow it was, if a wound of the dead names them.
                let by = w
                    .agents
                    .iter()
                    .find(|a| a.id == *agent)
                    .and_then(|a| a.body.wounds.iter().rev().find_map(|wd| wd.by));
                if let Some(by) = by {
                    w.deity_watch.last_killer = Some((by, *agent));
                }
            }
        }
    }
    if tick % TICKS_PER_DAY != 4 * TICKS_PER_HOUR + 11 || w.since_party() < TICKS_PER_DAY {
        return;
    }
    // Forget answers long over.
    w.deity_answers
        .retain(|a| a.until + 60 * TICKS_PER_DAY > tick);
    let day = tick / TICKS_PER_DAY;
    let mut r = stream(w.meta.seed, &[purpose::DEITY, day]);
    let deity = of(w.meta.engine);
    let noticed = notice(w);
    for (trigger, scope) in noticed {
        answer(w, &deity, trigger, scope, &mut r);
    }
}

/// What is true today that a deity might answer.
fn notice(w: &mut World) -> Vec<(Trigger, Scope)> {
    let tick = w.meta.tick;
    let mut out = Vec::new();
    let people: Vec<&crate::agents::Agent> = w
        .agents
        .iter()
        .filter(|a| a.alive && a.community.is_some())
        .collect();
    if !people.is_empty() {
        let n = people.len() as f32;
        let food: u32 = w
            .items
            .iter()
            .filter(|it| it.kind.is_food())
            .map(|it| it.count)
            .sum();
        let starving = people.iter().filter(|a| a.needs.hunger < 0.15).count();
        let sick = people.iter().filter(|a| a.body.illness.is_some()).count();
        let t = crate::engine::tension(w);
        // Plenty is about food and how people are, not about building still to do.
        let plenty = food as f32 / n >= 6.0
            && crate::engine::strain(w) < 0.3
            && starving * 10 <= people.len()
            && sick == 0;
        let hard = t > 0.5 || starving * 4 >= people.len() || sick * 3 >= people.len();
        let wt = &mut w.deity_watch;
        wt.plenty_days = if plenty { wt.plenty_days + 1 } else { 0 };
        wt.hard_days = if hard { wt.hard_days + 1 } else { 0 };
        if wt.plenty_days >= 3 {
            out.push((Trigger::Plenty, Scope::Valley));
        }
        if wt.hard_days >= 2 {
            out.push((Trigger::Hardship, Scope::Valley));
        }
        if wt.last_killing > wt.killing_seen {
            wt.killing_seen = wt.last_killing;
            out.push((Trigger::Killing, Scope::Valley));
        }
        if tick.saturating_sub(wt.last_major.max(w.meta.epoch)) > 5 * TICKS_PER_DAY {
            out.push((Trigger::LongQuiet, Scope::Valley));
        }
    }
    for s in &w.overworld.sites {
        if s.people == 0 {
            continue;
        }
        if s.hungry_days >= 3 {
            out.push((Trigger::SiteHunger, Scope::Site(s.id)));
        } else if s.food > s.people as f32 * 25.0 {
            out.push((Trigger::SiteFat, Scope::Site(s.id)));
        }
    }
    out
}

/// Is this trigger already being answered, or was it answered in the last three days (so the
/// deity does not pile on)? Valley triggers are answered over other scopes too (Hask answers
/// the valley's plenty over a neighbour's harvest), so for them any scope counts.
fn answering(w: &World, trigger: Trigger, scope: Scope) -> bool {
    let now = w.meta.tick;
    let valley = matches!(
        trigger,
        Trigger::Plenty | Trigger::Hardship | Trigger::Killing | Trigger::LongQuiet
    );
    w.deity_answers.iter().any(|a| {
        a.trigger == trigger && (valley || a.scope == scope) && now < a.until + 3 * TICKS_PER_DAY
    })
}

fn answer(
    w: &mut World,
    d: &Deity,
    trigger: Trigger,
    scope: Scope,
    r: &mut rand_chacha::ChaCha8Rng,
) {
    // Once Hask notices hardship, let go of existing pressure even if it is too
    // soon to grant another reprieve from sickness.
    if d.motive == Motive::Test && trigger == Trigger::Hardship {
        let now = w.meta.tick;
        for a in &mut w.deity_answers {
            if a.trigger == Trigger::Plenty && a.until > now {
                a.until = now;
            }
        }
    }
    if answering(w, trigger, scope) {
        return;
    }
    let winter = w.calendar().season == Season::Winter;
    // (lever, scope, strength, days)
    let mut acts: Vec<(Lever, Scope, f32, u64)> = Vec::new();
    match (d.motive, trigger) {
        // Aun lifts what is low and lowers what is high.
        (Motive::Balance, Trigger::Hardship) => {
            acts.push((Lever::Herds, Scope::Valley, 1.8, 8));
            acts.push((Lever::Sickness, Scope::Valley, 0.5, 6));
        }
        (Motive::Balance, Trigger::Plenty) if w.deity_watch.plenty_days >= 6 => {
            acts.push((Lever::DryStorms, Scope::Valley, 2.0, 4));
        }
        (Motive::Balance, Trigger::SiteHunger) => acts.push((Lever::Harvest, scope, 1.5, 10)),
        (Motive::Balance, Trigger::SiteFat) => acts.push((Lever::Harvest, scope, 0.8, 8)),
        (Motive::Balance, Trigger::Killing) => acts.push((Lever::Cold, Scope::Valley, -3.0, 5)),
        // Hask presses the strong: when the valley has plenty, a neighbour's harvest fails (and
        // hungry people come), the cold bites, and in winter the wolves grow bold. When the valley
        // is broken it lets go of everything it was pressing.
        (Motive::Test, Trigger::Plenty) => {
            if let Some(site) = nearest_inhabited(w, r) {
                acts.push((Lever::Harvest, Scope::Site(site), 0.3, 12));
            }
            acts.push((Lever::Cold, Scope::Valley, -4.0, 4));
            if winter {
                acts.push((Lever::WolfHunger, Scope::Valley, 2.0, 5));
            }
        }
        (Motive::Test, Trigger::Hardship) => {
            acts.push((Lever::Sickness, Scope::Valley, 0.7, 5));
        }
        (Motive::Test, Trigger::Killing) => {
            acts.push((Lever::Cold, Scope::Valley, -5.0, 6));
            acts.push((Lever::WolfHunger, Scope::Valley, 2.0, 6));
            // And the killer is cursed with the Beast-hunger (P11).
            if let Some((by, of)) = w.deity_watch.last_killer.take() {
                let dead = w
                    .agents
                    .iter()
                    .find(|a| a.id == of)
                    .map(|a| a.name.clone())
                    .unwrap_or_default();
                if let Some(i) = w.agents.iter().position(|a| {
                    a.id == by && a.alive && a.species == crate::agents::Species::Person
                }) {
                    crate::conditions::take(
                        w,
                        i,
                        crate::conditions::ConditionKind::BeastHunger,
                        &format!("for the killing of {dead}, by {}'s will", d.name),
                        None,
                    );
                }
            }
        }
        (Motive::Test, Trigger::SiteFat) => acts.push((Lever::Harvest, scope, 0.7, 10)),
        // Mirre likes a long quiet least, and answers anything with whatever comes to hand.
        (Motive::Amusement, Trigger::LongQuiet | Trigger::Plenty | Trigger::Hardship)
            if trigger == Trigger::LongQuiet || r.gen_bool(0.4) =>
        {
            let days = r.gen_range(2..7);
            match r.gen_range(0..5) {
                0 => acts.push((Lever::DryStorms, Scope::Valley, 3.0, days)),
                1 => acts.push((Lever::Cold, Scope::Valley, -6.0, days)),
                2 => acts.push((Lever::Herds, Scope::Valley, 2.0, days)),
                3 => acts.push((Lever::Sickness, Scope::Valley, 1.8, days)),
                _ => {
                    if let Some(site) = nearest_inhabited(w, r) {
                        acts.push((Lever::Harvest, Scope::Site(site), 0.4, days + 4));
                    }
                }
            }
        }
        (Motive::Amusement, Trigger::SiteHunger | Trigger::SiteFat) if r.gen_bool(0.3) => {
            acts.push((
                Lever::Harvest,
                scope,
                r.gen_range(0.4..1.6),
                r.gen_range(3..10),
            ));
        }
        _ => {}
    }
    let tick = w.meta.tick;
    for (lever, scope, strength, days) in acts {
        let place = match scope {
            Scope::Valley => "the valley".to_string(),
            Scope::Site(id) => w
                .overworld
                .sites
                .iter()
                .find(|s| s.id == id)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "a far place".into()),
        };
        let event = w.events.push(
            tick,
            None,
            Severity::Trace,
            EventKind::DeityAnswered {
                deity: d.name.to_string(),
                trigger: trigger.label().to_string(),
                lever: lever.label().to_string(),
                place,
                strength,
                days: days as u32,
            },
        );
        w.deity_answers.push(Answer {
            trigger,
            lever,
            scope,
            strength,
            from: tick,
            until: tick + days * TICKS_PER_DAY,
            event,
        });
    }
}

/// An inhabited place beyond the valley, nearer ones likelier (fewer travel days).
fn nearest_inhabited(w: &World, r: &mut rand_chacha::ChaCha8Rng) -> Option<u32> {
    let sites: Vec<&crate::expedition::Site> =
        w.overworld.sites.iter().filter(|s| s.people > 0).collect();
    if sites.is_empty() {
        return None;
    }
    let total: f32 = sites.iter().map(|s| 1.0 / s.days.max(1) as f32).sum();
    let mut x = r.gen_range(0.0..total);
    for s in &sites {
        x -= 1.0 / s.days.max(1) as f32;
        if x <= 0.0 {
            return Some(s.id);
        }
    }
    sites.last().map(|s| s.id)
}

impl World {
    pub fn lean(&self, lever: Lever, scope: Scope) -> f32 {
        lean(self, lever, scope)
    }
}
