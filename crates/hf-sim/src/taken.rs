//! The taken (#322, 2026-10-26 fix-322-search): a child carried down into the deep
//! (societies.rs) is gone after. The morning the way is found open, the child's parents and
//! those close to them gather at the way down and go below after it, off the map as a caravan
//! is (`overworld.away`). Some days on they come up again: with the child, changed by the dark;
//! hurt; short one of their number, lost below; or with nothing found. A child not gone after,
//! or not found, grows up below among the people who took it, and is seen again, grown and one
//! of them, when that people next comes up into the valley.
//!
//! The grown child comes home (#322, 2026-10-28 round-17): one grown below, now and then, comes
//! up of their own accord to seek their family (`come_up`), and walks to a parent, else a brother
//! or sister, else the old home. Their parents' grief is stirred. Met, they are known or not, by
//! how long ago they were taken and who meets them; known, they are taken back into the
//! community or turn back below, and either way the old accusation is stirred: they name the one
//! who let their people up, whom their parent accuses (justice.rs), and their parents think the
//! better of anyone wrongly accused at the time. Not known, they turn back. Each meeting is on
//! record (`SoughtKin`, `KinMet`). FIXLOG 2026-10-28 "the grown child never came home".

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, CommunityId, TilePos};
use crate::mind::feelings::FeelingKind;
use crate::mind::{Activity, MemKind, Subject, ThoughtKind, Trait};
use crate::rng::{purpose, stream};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::World;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// How long the party waits at the way down for the last of them.
const GATHERING: u64 = 6 * TICKS_PER_HOUR;
/// Never more than four go down.
const PARTY: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stage {
    /// The party walks to the way down.
    Gathering,
    /// Below, after the child.
    Down,
    /// Nobody went, or they came back without it: the child grows up below.
    Below,
    /// Brought back, or seen again grown.
    Done,
    /// Appended (round-17): grown, up from below to seek their family.
    Seeking,
}

/// A child carried below, and what is done about it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Taken {
    pub child: AgentId,
    pub community: CommunityId,
    /// The deep's settlement it was taken to.
    pub site: String,
    pub cavern: usize,
    pub taken: u64,
    pub party: Vec<AgentId>,
    /// Where the party gathers: the valley's side of the way down.
    pub stand: TilePos,
    pub since: u64,
    pub back_at: u64,
    pub stage: Stage,
    /// Who opened the way the night the child was taken (societies.rs `Taking`).
    #[serde(default)]
    pub opener: Option<AgentId>,
    /// Who was accused of it at the time.
    #[serde(default)]
    pub accused: Vec<AgentId>,
    /// Seeking, met and turned back: walking back to the way down.
    #[serde(default)]
    pub going_back: bool,
}

/// How a search below ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    BroughtBack,
    /// One of the party lost below; the rest come up without the child.
    Lost,
    /// They come up hurt, without the child.
    Hurt,
    NothingFound,
}

impl Outcome {
    pub fn word(self) -> &'static str {
        match self {
            Outcome::BroughtBack => "brought back",
            Outcome::Lost => "lost",
            Outcome::Hurt => "hurt",
            Outcome::NothingFound => "nothing found",
        }
    }
}

/// The valley's side of a cavern's way down.
fn stand_of(w: &World, cavern: usize) -> Option<TilePos> {
    let c = w.grid.caverns.get(cavern)?;
    let b = c.breach?;
    let s = [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .iter()
        .map(|(dx, dy)| b.offset(*dx, *dy, 0))
        .filter(|p| !c.contains(*p) && w.grid.walkable(*p))
        .min_by_key(|p| (p.x, p.y));
    Some(s.unwrap_or(b))
}

/// The morning a taking is found (societies.rs `dawn`): the child's parents, and up to four in
/// all with those who think well of them or of the child, set out for the way down. None of
/// the society that did it go. With no parent here to go, nobody does.
pub fn gone(w: &mut World, tk: &crate::societies::Taking) {
    let tick = w.meta.tick;
    let cid = tk.community;
    let able = |a: &crate::agents::Agent| {
        a.alive
            && a.community == Some(cid)
            && a.species == crate::agents::Species::Person
            && !a.is_child(tick)
            && !a.injured(tick)
            && a.caravan.is_none()
            && a.id != tk.opener
            && !tk.members.contains(&a.id)
    };
    let mut party: Vec<AgentId> = w
        .agents
        .iter()
        .filter(|a| able(a) && a.children.contains(&tk.child))
        .map(|a| a.id)
        .collect();
    party.sort();
    let stand = stand_of(w, tk.cavern).unwrap_or(w.grid.caverns[tk.cavern].centre);
    let mut rec = Taken {
        child: tk.child,
        community: cid,
        site: tk.site.clone(),
        cavern: tk.cavern,
        taken: tk.opened,
        party: Vec::new(),
        stand,
        since: tick,
        back_at: 0,
        stage: Stage::Below,
        opener: Some(tk.opener),
        accused: tk.accused.clone(),
        going_back: false,
    };
    if party.is_empty() {
        w.gone_below.push(rec);
        return;
    }
    let close = |a: &crate::agents::Agent| {
        party
            .iter()
            .map(|p| a.opinion_of(*p))
            .fold(a.opinion_of(tk.child), f32::max)
    };
    let mut friends: Vec<(f32, AgentId)> = w
        .agents
        .iter()
        .filter(|a| able(a) && !party.contains(&a.id) && close(a) >= 0.4)
        .map(|a| (close(a), a.id))
        .collect();
    friends.sort_by(|x, y| y.0.total_cmp(&x.0).then(x.1.cmp(&y.1)));
    party.truncate(PARTY);
    let room = PARTY - party.len();
    party.extend(friends.into_iter().take(room).map(|f| f.1));
    for id in &party {
        if let Some(i) = w.agents.iter().position(|a| a.id == *id) {
            w.reservations.release_all(*id);
            w.agents[i].path.clear();
            if w.path_agent_to(i, stand) {
                w.agents[i].activity = Activity::Wandering { to: stand };
                w.agents[i].activity_ticks = 0;
            }
        }
    }
    rec.party = party;
    rec.stage = Stage::Gathering;
    w.gone_below.push(rec);
}

/// Every 200 ticks: the party gathers and goes down; a party due comes up. Every tick, one
/// grown below and seeking their family is walked to them and meets them; once a day, one grown
/// below may come up to seek them.
pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if w.gone_below.is_empty() {
        return;
    }
    for k in 0..w.gone_below.len() {
        if w.gone_below[k].stage == Stage::Seeking {
            seeking(w, k);
        }
    }
    if tick % TICKS_PER_DAY == 9 * TICKS_PER_HOUR {
        seek(w);
    }
    if !tick.is_multiple_of(200) {
        return;
    }
    for k in 0..w.gone_below.len() {
        match w.gone_below[k].stage {
            Stage::Gathering => gather(w, k),
            Stage::Down if tick >= w.gone_below[k].back_at => {
                let o = roll(w, k);
                settle(w, k, o);
            }
            _ => {}
        }
    }
}

fn gather(w: &mut World, k: usize) {
    let tick = w.meta.tick;
    let rec = w.gone_below[k].clone();
    let mut all_there = true;
    for id in &rec.party {
        let Some(i) = w.agents.iter().position(|a| a.id == *id && a.alive) else {
            continue;
        };
        if w.agents[i].pos.dist(rec.stand) > 2 {
            all_there = false;
            let a = &w.agents[i];
            let heading = matches!(a.activity, Activity::Wandering { to } if to == rec.stand);
            if !heading
                && matches!(
                    a.activity,
                    Activity::Idle | Activity::Loitering { .. } | Activity::Wandering { .. }
                )
                && w.path_agent_to(i, rec.stand)
            {
                w.agents[i].activity = Activity::Wandering { to: rec.stand };
                w.agents[i].activity_ticks = 0;
            }
        }
    }
    if !all_there && tick.saturating_sub(rec.since) < GATHERING {
        return;
    }
    // Those at the way go down; the rest stay.
    let mut gone = Vec::new();
    for id in &rec.party {
        let Some(i) = w.agents.iter().position(|a| a.id == *id && a.alive) else {
            continue;
        };
        if w.agents[i].pos.dist(rec.stand) > 2 {
            continue;
        }
        w.reservations.release_all(*id);
        let at = w.agents[i].pos;
        for it in w.items.iter_mut() {
            if it.place == crate::items::ItemPlace::Carried(*id) {
                it.place = crate::items::ItemPlace::Ground(at);
            }
        }
        let mut a = w.agents.remove(i);
        a.activity = Activity::Idle;
        a.activity_ticks = 0;
        a.path.clear();
        w.overworld.away.push(a);
        gone.push(*id);
    }
    let r = &mut w.gone_below[k];
    r.party = gone.clone();
    if gone.is_empty() {
        r.stage = Stage::Below;
        return;
    }
    let mut rng = stream(
        w.meta.seed,
        &[purpose::AGENT_SOCIAL, 0x3223, rec.child.0 as u64, rec.taken],
    );
    r.stage = Stage::Down;
    r.since = tick;
    r.back_at = tick + rng.gen_range(2..=5) * TICKS_PER_DAY;
    w.events.push(
        tick,
        Some(rec.stand),
        Severity::Major,
        EventKind::WentDownAfter {
            agents: gone,
            child: rec.child,
            to: rec.site.clone(),
        },
    );
}

/// How it goes below, in the style of a rite's outcome (#338): the more who went, the likelier
/// the child is found; some parties lose one of their own, some come up hurt.
pub fn roll(w: &World, k: usize) -> Outcome {
    let rec = &w.gone_below[k];
    let mut r = stream(
        w.meta.seed,
        &[purpose::AGENT_SOCIAL, 0x3222, rec.child.0 as u64, rec.taken],
    );
    let n = rec.party.len().min(PARTY) as f32;
    let p_back = 0.15 + 0.1 * n;
    let p_lost = 0.15;
    let p_hurt = 0.2;
    let roll: f32 = r.gen();
    if roll < p_back {
        Outcome::BroughtBack
    } else if roll < p_back + p_lost {
        Outcome::Lost
    } else if roll < p_back + p_lost + p_hurt {
        Outcome::Hurt
    } else {
        Outcome::NothingFound
    }
}

/// Off the map and back onto it at `at`.
fn up(w: &mut World, id: AgentId, at: TilePos) -> bool {
    let Some(ix) = w.overworld.away.iter().position(|a| a.id == id) else {
        return false;
    };
    let mut a = w.overworld.away.remove(ix);
    a.pos = at;
    a.from = at;
    a.path.clear();
    a.activity = Activity::Idle;
    a.activity_ticks = 0;
    a.needs.hunger = a.needs.hunger.min(0.5);
    w.agents.push(a);
    true
}

/// The party comes up again at the way down, and it ends as `o` says.
pub fn settle(w: &mut World, k: usize, o: Outcome) {
    let tick = w.meta.tick;
    let rec = w.gone_below[k].clone();
    // The last to join is the one lost, when one is.
    let lost = (o == Outcome::Lost).then(|| *rec.party.last().unwrap());
    for id in &rec.party {
        up(w, *id, rec.stand);
    }
    if let Some(l) = lost {
        crate::systems::life::die_away(w, l, "lost in the deep".into());
    }
    let first = rec.party.iter().copied().find(|id| Some(*id) != lost);
    let by = first.unwrap_or(rec.party[0]);
    let eid = w.events.push(
        tick,
        Some(rec.stand),
        Severity::Major,
        EventKind::CameUpFromBelow {
            child: rec.child,
            by,
            outcome: o.word().to_string(),
            lost,
            from: rec.site.clone(),
        },
    );
    let back: Vec<AgentId> = rec
        .party
        .iter()
        .copied()
        .filter(|id| Some(*id) != lost)
        .collect();
    match o {
        Outcome::BroughtBack => {
            if up(w, rec.child, rec.stand) {
                let ci = w.agents.len() - 1;
                let c = &mut w.agents[ci];
                // Changed by it: afraid of the dark from now on.
                c.thoughts.add(ThoughtKind::BroughtBack, tick, Some(eid));
                if !c.traits.has(Trait::FearsTheDark) {
                    c.traits.list.push(Trait::FearsTheDark);
                    c.traits.list.sort();
                }
                c.feelings
                    .feel(FeelingKind::Fear, None, 0.7, tick, Some(eid));
            }
            for i in 0..w.agents.len() {
                let a = &mut w.agents[i];
                if !a.alive {
                    continue;
                }
                if a.children.contains(&rec.child) || back.contains(&a.id) {
                    a.feelings
                        .list
                        .retain(|f| !(f.kind == FeelingKind::Grief && f.about == Some(rec.child)));
                    a.feelings
                        .feel(FeelingKind::Joy, Some(rec.child), 0.9, tick, Some(eid));
                }
            }
            w.gone_below[k].stage = Stage::Done;
        }
        Outcome::Hurt | Outcome::Lost | Outcome::NothingFound => {
            let hurt: Vec<AgentId> = if o == Outcome::Hurt {
                back.iter().copied().take(2).collect()
            } else {
                Vec::new()
            };
            for id in &back {
                if let Some(a) = w.agents.iter_mut().find(|a| a.id == *id) {
                    a.remember(tick, MemKind::Hardship, Subject::None, -0.6, 0.9, Some(eid));
                    if hurt.contains(id) {
                        a.injured_until = tick + 2 * TICKS_PER_DAY;
                        a.thoughts.add(ThoughtKind::Hurt, tick, Some(eid));
                    }
                }
            }
            w.gone_below[k].stage = Stage::Below;
        }
    }
    // Home again, the child with them.
    let home = w
        .communities
        .iter()
        .find(|c| c.id == rec.community && !c.dissolved)
        .map(|c| c.home);
    if let Some(home) = home {
        let child = (o == Outcome::BroughtBack).then_some(rec.child);
        for id in back.iter().copied().chain(child) {
            if let Some(i) = w.agents.iter().position(|a| a.id == id && a.alive) {
                if w.path_agent_to(i, home) {
                    w.agents[i].activity = Activity::Wandering { to: home };
                }
            }
        }
    }
}

/// #322: the deep's people of `site` come up into the valley (underground.rs `breaches`,
/// societies.rs `let_up`): any child taken to them years ago and grown among them is seen with
/// them, and known by their kin.
pub fn among_them(w: &mut World, site: &str, at: TilePos) {
    let tick = w.meta.tick;
    for k in 0..w.gone_below.len() {
        let rec = w.gone_below[k].clone();
        if rec.stage != Stage::Below || rec.site != site {
            continue;
        }
        let Some(a) = w.overworld.away.iter().find(|a| a.id == rec.child) else {
            continue;
        };
        if !a.alive || a.is_child(tick) {
            continue;
        }
        let years = ((tick.saturating_sub(rec.taken)) / crate::time::TICKS_PER_YEAR) as u32;
        w.gone_below[k].stage = Stage::Done;
        let eid = w.events.push(
            tick,
            Some(at),
            Severity::Major,
            EventKind::GrownBelow {
                agent: rec.child,
                from: rec.site.clone(),
                years,
            },
        );
        for a in w.agents.iter_mut() {
            if a.alive && a.children.contains(&rec.child) {
                a.feelings
                    .feel(FeelingKind::Grief, Some(rec.child), 0.6, tick, Some(eid));
                a.feelings
                    .feel(FeelingKind::Awe, Some(rec.child), 0.5, tick, Some(eid));
            }
        }
    }
}

/// Chance a day that one grown below comes up to seek their family.
pub const SEEKS_A_DAY: f64 = 1.0 / 60.0;
/// Days a seeker looks before turning back.
const SEEKS_FOR: u64 = 2 * TICKS_PER_DAY;

/// Once a day: anyone taken below, grown and not seen since, may come up to seek their family.
fn seek(w: &mut World) {
    let tick = w.meta.tick;
    let day = tick / TICKS_PER_DAY;
    for k in 0..w.gone_below.len() {
        let rec = &w.gone_below[k];
        if rec.stage != Stage::Below {
            continue;
        }
        let grown = w
            .overworld
            .away
            .iter()
            .any(|a| a.id == rec.child && a.alive && !a.is_child(tick));
        if !grown {
            continue;
        }
        let mut r = stream(
            w.meta.seed,
            &[purpose::AGENT_SOCIAL, 0x3224, rec.child.0 as u64, day],
        );
        if r.gen_bool(SEEKS_A_DAY) {
            come_up(w, k);
        }
    }
}

/// Those of the valley a grown child would look for, nearest kin first: their parents, then
/// their brothers and sisters, living and of the community.
fn kin_of(w: &World, child: AgentId, cid: CommunityId) -> Vec<(AgentId, bool)> {
    let c = w
        .agents
        .iter()
        .chain(w.overworld.away.iter())
        .find(|a| a.id == child);
    let (mother, father) = c.map(|c| (c.mother, c.father)).unwrap_or((None, None));
    let here = |a: &crate::agents::Agent| a.alive && a.community == Some(cid) && a.id != child;
    let mut out: Vec<(AgentId, bool)> = w
        .agents
        .iter()
        .filter(|a| here(a) && a.children.contains(&child))
        .map(|a| (a.id, true))
        .collect();
    out.extend(
        w.agents
            .iter()
            .filter(|a| {
                here(a)
                    && !a.children.contains(&child)
                    && ((mother.is_some() && a.mother == mother)
                        || (father.is_some() && a.father == father))
            })
            .map(|a| (a.id, false)),
    );
    out
}

/// One grown below comes up at the way down to seek their family; their parents' grief is
/// stirred.
pub fn come_up(w: &mut World, k: usize) {
    let tick = w.meta.tick;
    let rec = w.gone_below[k].clone();
    if !up(w, rec.child, rec.stand) {
        return;
    }
    let i = w.agents.len() - 1;
    // A stranger here now, walked by nobody's choice but this (decide.rs passes over the
    // leaving) until they are taken back or turn back.
    if let Some(c) = w.communities.iter_mut().find(|c| c.id == rec.community) {
        c.members.retain(|m| *m != rec.child);
    }
    w.agents[i].community = None;
    w.agents[i].leaving = true;
    let r = &mut w.gone_below[k];
    r.stage = Stage::Seeking;
    r.since = tick;
    r.back_at = tick + SEEKS_FOR;
    r.going_back = false;
    let years = (tick.saturating_sub(rec.taken) / crate::time::TICKS_PER_YEAR) as u32;
    let eid = w.events.push(
        tick,
        Some(rec.stand),
        Severity::Major,
        EventKind::SoughtKin {
            agent: rec.child,
            from: rec.site.clone(),
            years,
        },
    );
    for a in w.agents.iter_mut() {
        if a.alive && a.children.contains(&rec.child) {
            a.feelings
                .feel(FeelingKind::Grief, Some(rec.child), 0.5, tick, Some(eid));
        }
    }
    head_for_kin(w, k);
}

/// Walk the seeker toward the nearest of their kin, else the old home.
fn head_for_kin(w: &mut World, k: usize) {
    let rec = w.gone_below[k].clone();
    let Some(i) = w.agents.iter().position(|a| a.id == rec.child && a.alive) else {
        return;
    };
    let at = w.agents[i].pos;
    let to = kin_of(w, rec.child, rec.community)
        .into_iter()
        .filter_map(|(id, _)| w.agents.iter().find(|a| a.id == id).map(|a| a.pos))
        .min_by_key(|p| (p.dist(at), p.x, p.y))
        .or_else(|| {
            w.communities
                .iter()
                .find(|c| c.id == rec.community && !c.dissolved)
                .map(|c| c.home)
        });
    let Some(to) = to else { return };
    if matches!(w.agents[i].activity, Activity::Wandering { to: t } if t.dist(to) <= 2)
        && !w.agents[i].path.is_empty()
    {
        return;
    }
    if w.path_agent_to(i, to) {
        w.agents[i].activity = Activity::Wandering { to };
        w.agents[i].activity_ticks = 0;
    }
}

/// How meeting their family goes for one grown below.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Meeting {
    /// Known, and taken back into the community.
    TakenBack,
    /// Known, and they turn back below.
    KnownTurnedBack,
    /// Not known; they turn back below.
    NotKnown,
}

impl Meeting {
    pub fn word(self) -> &'static str {
        match self {
            Meeting::TakenBack => "taken back",
            Meeting::KnownTurnedBack => "known, and turned back below",
            Meeting::NotKnown => "not known, and turned back below",
        }
    }
}

/// Every tick while one grown below is seeking: walked toward their kin; within three tiles of
/// one of them, or with the time up, they meet; once turned back, walked to the way down and
/// gone below again.
fn seeking(w: &mut World, k: usize) {
    let tick = w.meta.tick;
    let rec = w.gone_below[k].clone();
    let Some(i) = w.agents.iter().position(|a| a.id == rec.child && a.alive) else {
        w.gone_below[k].stage = Stage::Done;
        return;
    };
    let at = w.agents[i].pos;
    if rec.going_back {
        if at.dist(rec.stand) <= 1 || tick >= rec.back_at {
            down_again(w, k);
        } else if w.agents[i].path.is_empty() && w.path_agent_to(i, rec.stand) {
            w.agents[i].activity = Activity::Wandering { to: rec.stand };
        }
        return;
    }
    let met = kin_of(w, rec.child, rec.community)
        .into_iter()
        .filter_map(|(id, parent)| {
            let a = w.agents.iter().find(|a| a.id == id)?;
            (a.pos.z == at.z && a.pos.dist(at) <= 3).then_some((id, parent))
        })
        .min_by_key(|(id, parent)| (!*parent, *id));
    if met.is_none() && tick < rec.back_at {
        if w.agents[i].path.is_empty() || tick.is_multiple_of(200) {
            head_for_kin(w, k);
        }
        return;
    }
    let m = meeting(w, k, met);
    meet(w, k, met.map(|x| x.0), m);
}

/// Whether a grown child met by `met` (a parent when true) is known, and whether they stay:
/// a parent knows them the more the fewer years since, a brother or sister less often, and
/// nobody met nobody knows.
pub fn meeting(w: &World, k: usize, met: Option<(AgentId, bool)>) -> Meeting {
    let rec = &w.gone_below[k];
    let Some((_, parent)) = met else {
        return Meeting::NotKnown;
    };
    let years = (w.meta.tick.saturating_sub(rec.taken) / crate::time::TICKS_PER_YEAR) as f64;
    let mut r = stream(
        w.meta.seed,
        &[purpose::AGENT_SOCIAL, 0x3225, rec.child.0 as u64, rec.taken],
    );
    let known = if parent {
        (0.9 - 0.02 * years).max(0.5)
    } else {
        0.4
    };
    if !r.gen_bool(known) {
        return Meeting::NotKnown;
    }
    let stays = if parent { 0.7 } else { 0.5 };
    if r.gen_bool(stays) {
        Meeting::TakenBack
    } else {
        Meeting::KnownTurnedBack
    }
}

/// The grown child meets `by` (or nobody), and it goes as `m` says.
pub fn meet(w: &mut World, k: usize, by: Option<AgentId>, m: Meeting) {
    let tick = w.meta.tick;
    let rec = w.gone_below[k].clone();
    let Some(i) = w.agents.iter().position(|a| a.id == rec.child && a.alive) else {
        return;
    };
    let at = w.agents[i].pos;
    let eid = w.events.push(
        tick,
        Some(at),
        Severity::Major,
        EventKind::KinMet {
            agent: rec.child,
            by,
            outcome: m.word().to_string(),
        },
    );
    let parents: Vec<usize> = (0..w.agents.len())
        .filter(|&j| w.agents[j].alive && w.agents[j].children.contains(&rec.child))
        .collect();
    match m {
        Meeting::TakenBack => {
            w.agents[i].leaving = false;
            w.agents[i].path.clear();
            w.agents[i].activity = Activity::Idle;
            w.join_community(rec.child, rec.community);
            for &j in &parents {
                let a = &mut w.agents[j];
                a.feelings
                    .list
                    .retain(|f| !(f.kind == FeelingKind::Grief && f.about == Some(rec.child)));
                a.feelings
                    .feel(FeelingKind::Joy, Some(rec.child), 0.9, tick, Some(eid));
            }
            w.gone_below[k].stage = Stage::Done;
        }
        Meeting::KnownTurnedBack | Meeting::NotKnown => {
            if m == Meeting::KnownTurnedBack {
                for &j in &parents {
                    w.agents[j].feelings.feel(
                        FeelingKind::Grief,
                        Some(rec.child),
                        0.8,
                        tick,
                        Some(eid),
                    );
                }
            }
            let r = &mut w.gone_below[k];
            r.going_back = true;
            r.back_at = tick + TICKS_PER_DAY;
            if w.path_agent_to(i, rec.stand) {
                w.agents[i].activity = Activity::Wandering { to: rec.stand };
            }
        }
    }
    if m != Meeting::NotKnown {
        stir(w, &rec, at);
    }
}

/// Known, the grown child names the one who let their people up that night: their parent (the
/// first, else whoever knew them) accuses them, if they are still of the community, and their
/// parents think the better of anyone wrongly accused at the time.
fn stir(w: &mut World, rec: &Taken, at: TilePos) {
    let tick = w.meta.tick;
    let cid = rec.community;
    let parents: Vec<AgentId> = w
        .agents
        .iter()
        .filter(|a| a.alive && a.community == Some(cid) && a.children.contains(&rec.child))
        .map(|a| a.id)
        .collect();
    if let Some(opener) = rec.opener.filter(|o| {
        w.agents
            .iter()
            .any(|a| a.id == *o && a.alive && a.community == Some(cid))
    }) {
        for &p in &parents {
            crate::societies::suspect(w, p, opener, 0.9);
        }
        let by = parents.iter().copied().min().unwrap_or(rec.child);
        crate::justice::accuse(w, cid, by, opener, opener, crate::justice::DEEP_GATE, at);
    }
    for wrong in rec.accused.iter().filter(|a| Some(**a) != rec.opener) {
        for &p in &parents {
            if let Some(j) = w.agents.iter().position(|a| a.id == p) {
                w.agents[j].relate(*wrong, 0.3, 0.05, tick);
            }
        }
    }
}

/// Turned back, at the way down: below again, for good.
fn down_again(w: &mut World, k: usize) {
    let rec = w.gone_below[k].clone();
    if let Some(i) = w.agents.iter().position(|a| a.id == rec.child && a.alive) {
        w.reservations.release_all(rec.child);
        let mut a = w.agents.remove(i);
        a.activity = Activity::Idle;
        a.activity_ticks = 0;
        a.path.clear();
        a.leaving = false;
        w.overworld.away.push(a);
    }
    w.gone_below[k].stage = Stage::Done;
    w.gone_below[k].going_back = false;
}
