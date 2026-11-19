//! Leadership and justice (C77, backlog 6.7 and 8.5; issues #298, #360, #281, #322).
//!
//! Leadership is granted, not held (docs/design-leadership.md): someone who sees a person do a
//! valued deed credits them with deference in that deed's domain, and it fades. A community
//! follows, in each domain, whoever its people defer to most, once that is enough to matter.
//! Different people can lead in different domains, and a crisis's deeds swap who is followed.
//!
//! Justice: a fight or a tantrum is an offence. Someone who saw it, or was hurt by it, accuses
//! the offender; an offence nobody saw is laid at the door of whoever the finder likes least
//! nearby, which can be wrong. Each evening the one followed in the social domain (or the
//! eldest) judges: a first offence by someone liked is pardoned, else amends are made; a third
//! fight within forty days by one the judge does not like is exile, and so is any of the gravest
//! offences (setting a fire, letting the deep's people up; #404, #322). The exiled go to a place
//! beyond the valley, which holds it against the valley. A wrong verdict costs the judge the
//! following of the wronged one's friends. Two families set against each other by the judging
//! twice within sixty days, or one of them exiled on the other's word, are in a feud (feuds.rs,
//! #404). A killing or a maiming in a fight is the gravest of all: the families are in a feud
//! at once, and the one who struck is judged for it (`struck_down`).

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, CommunityId, TilePos};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::World;
use serde::{Deserialize, Serialize};

/// The four domains a person can be followed in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain {
    Provision = 0,
    Protection = 1,
    Spirit = 2,
    Social = 3,
}

impl Domain {
    pub fn word(self) -> &'static str {
        match self {
            Domain::Provision => "in feeding and housing them",
            Domain::Protection => "in keeping them safe",
            Domain::Spirit => "in the rites and the unseen",
            Domain::Social => "in settling what is between people",
        }
    }
}

const ALL: [Domain; 4] = [
    Domain::Provision,
    Domain::Protection,
    Domain::Spirit,
    Domain::Social,
];

/// Deference enough, summed over a community, to be followed.
pub const FOLLOWED_AT: f32 = 1.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Accusation {
    pub by: AgentId,
    pub of: AgentId,
    /// Who really did it (the record knows; the accuser may not).
    pub truly: AgentId,
    pub what: String,
    pub at: u64,
}

/// Food found short in a household's chest with nobody watching (#1796): what the chest's
/// keeper has yet to find out. The record knows who really took it (`truly`); the household
/// does not, until a member comes within reach of the chest (`find_missing_food`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissingFood {
    pub units: u32,
    pub truly: AgentId,
    pub at: TilePos,
    pub tick: u64,
}

fn idx(w: &World, id: AgentId) -> Option<usize> {
    w.agents.iter().position(|a| a.id == id && a.alive)
}

/// `who` credits `to` with deference in `d`.
pub fn credit(w: &mut World, who: usize, to: AgentId, d: Domain, amount: f32) {
    if w.agents[who].id == to {
        return;
    }
    let defers = &mut w.agents[who].defers;
    match defers.iter_mut().find(|x| x.0 == to) {
        Some(x) => x.1[d as usize] = (x.1[d as usize] + amount).min(3.0),
        None => {
            let mut v = [0.0; 4];
            v[d as usize] = amount;
            defers.push((to, v));
            if defers.len() > 8 {
                // Forget the one they think least of.
                let k = defers
                    .iter()
                    .enumerate()
                    .min_by(|a, b| {
                        let (sa, sb) = (a.1 .1.iter().sum::<f32>(), b.1 .1.iter().sum::<f32>());
                        sa.total_cmp(&sb)
                    })
                    .map(|x| x.0)
                    .unwrap();
                defers.remove(k);
            }
        }
    }
}

/// `who` takes back deference from `to` in `d` (#297): a blessing they read was belied.
pub fn discredit(w: &mut World, who: usize, to: AgentId, d: Domain, amount: f32) {
    if let Some(x) = w.agents[who].defers.iter_mut().find(|x| x.0 == to) {
        x.1[d as usize] = (x.1[d as usize] - amount).max(0.0);
    }
}

/// Everyone of `doer`'s community awake within ten tiles, who actually had line of sight to
/// the deed, saw it (D48, #1247: a miner in the next gallery does not credit the cook).
fn witnessed(w: &mut World, doer: AgentId, d: Domain, amount: f32, also: Option<AgentId>) {
    let Some(i) = idx(w, doer) else { return };
    let (at, cid) = (w.agents[i].pos, w.agents[i].community);
    if cid.is_none() {
        return;
    }
    let seers: Vec<usize> = (0..w.agents.len())
        .filter(|&k| {
            let a = &w.agents[k];
            k != i
                && a.alive
                && a.community == cid
                && a.pos.z == at.z
                && a.pos.dist(at) <= 10
                && !a.activity.is_asleep()
                && crate::mind::perception::can_see(&w.grid, a.pos, at)
        })
        .collect();
    for k in seers {
        let more = if Some(w.agents[k].id) == also {
            2.0
        } else {
            1.0
        };
        credit(w, k, doer, d, amount * more);
    }
    // The one the deed was for credits it whether or not they were awake to see it.
    if let Some(b) = also.and_then(|b| idx(w, b)) {
        if w.agents[b].pos.dist(at) > 10 || w.agents[b].activity.is_asleep() {
            credit(w, b, doer, d, amount * 2.0);
        }
    }
}

/// The deeds of the last tick, credited by those who saw them.
pub fn observe(w: &mut World) {
    let events: Vec<EventKind> = w.last_events.iter().map(|e| e.kind.clone()).collect();
    for k in events {
        match k {
            EventKind::Built { agent, .. } => witnessed(w, agent, Domain::Provision, 0.05, None),
            EventKind::Harvested { agent, .. } => {
                witnessed(w, agent, Domain::Provision, 0.02, None)
            }
            EventKind::Cooked { agent, .. } => witnessed(w, agent, Domain::Provision, 0.03, None),
            EventKind::Hunted { agent, .. } => witnessed(w, agent, Domain::Provision, 0.1, None),
            EventKind::StoodAgainst { by, .. } => {
                for a in by {
                    witnessed(w, a, Domain::Protection, 0.4, None);
                }
            }
            EventKind::Fought { a, b } => {
                // Standing up to a stranger, not a quarrel at home.
                let stranger = |x: AgentId| {
                    w.agents
                        .iter()
                        .find(|o| o.id == x)
                        .is_some_and(|o| o.band.is_some())
                };
                if stranger(b) {
                    witnessed(w, a, Domain::Protection, 0.3, None);
                } else if stranger(a) {
                    witnessed(w, b, Domain::Protection, 0.3, None);
                }
            }
            EventKind::LaidToRest { by, .. } => witnessed(w, by, Domain::Spirit, 0.3, None),
            EventKind::Tended { healer, patient } => {
                witnessed(w, healer, Domain::Social, 0.2, Some(patient))
            }
            EventKind::Taught { teacher, pupil, .. } => {
                witnessed(w, teacher, Domain::Social, 0.2, Some(pupil))
            }
            EventKind::Welcomed { agent, .. } => witnessed(w, agent, Domain::Social, 0.2, None),
            _ => {}
        }
    }
}

/// An offence: `offender` did `what` at `at`, to `victim` if anyone. Called where it happens
/// (systems/strain.rs: a fight, a tantrum).
pub fn offence(w: &mut World, offender: usize, victim: Option<AgentId>, what: &str, at: TilePos) {
    let Some(cid) = w.agents[offender].community else {
        return;
    };
    let truly = w.agents[offender].id;
    let tick = w.meta.tick;
    // Not everyone makes a case of a broken stool: a third of the time for anything short of a
    // fight (C77 soaks: 240 to 475 judgements a run soured the valley into splitting).
    let mut r = crate::rng::stream(
        w.meta.seed,
        &[
            crate::rng::purpose::AGENT_SOCIAL,
            0x0FFE,
            truly.0 as u64,
            tick,
        ],
    );
    use rand::Rng;
    if !what.contains("fight") && !grave(what) && !r.gen_bool(0.35) {
        return;
    }
    let near = |w: &World, d: i32| -> Vec<usize> {
        (0..w.agents.len())
            .filter(|&k| {
                let a = &w.agents[k];
                k != offender
                    && a.alive
                    && a.community == Some(cid)
                    && a.pos.z == at.z
                    && a.pos.dist(at) <= d
            })
            .collect()
    };
    let seen: Vec<usize> = near(w, 8)
        .into_iter()
        .filter(|&k| !w.agents[k].activity.is_asleep())
        .collect();
    let (by, of) = if let Some(v) = victim {
        (v, truly)
    } else if let Some(&k) = seen.iter().min_by_key(|&&k| (w.agents[k].pos.dist(at), k)) {
        (w.agents[k].id, truly)
    } else {
        // Nobody saw: half the time whoever finds it lays it on the one near they like least
        // (D48, lay_blame below).
        if r.gen_bool(0.5) {
            return;
        }
        let around = near(w, 14);
        let Some(&f) = around
            .iter()
            .min_by_key(|&&k| (w.agents[k].pos.dist(at), k))
        else {
            return;
        };
        let finder = w.agents[f].id;
        lay_blame(w, finder, truly, what, at);
        return;
    };
    accuse(w, cid, by, of, truly, what, at);
}

/// D48: with no witness, the one who finds a wrong (`finder`, standing at `at`) lays it on
/// whoever of their community within 14 tiles they like least — not necessarily `truly`, who
/// really did it. Used by `offence`'s nobody-saw branch and by a household keeper who finds
/// food short in the chest (#1796). Does not re-check `truly`'s whereabouts: call this only
/// where the caller has already ruled out a witness, since it never counts the finder as one
/// (that is `offence`'s `seen` test, which this intentionally skips).
///
/// The one blamed is accused as usual (`accuse`), and the finder's opinion of them alone
/// sours: `relate(blamed, -0.3)` and a `Hardship` memory of them. The finder's opinion of
/// `truly` (the real offender) never moves here unless `truly` is the one blamed.
pub fn lay_blame(
    w: &mut World,
    finder: AgentId,
    truly: AgentId,
    what: &str,
    at: TilePos,
) -> Option<AgentId> {
    let fi = idx(w, finder)?;
    let cid = w.agents[fi].community?;
    let suspects: Vec<usize> = (0..w.agents.len())
        .filter(|&k| {
            let a = &w.agents[k];
            a.alive && a.community == Some(cid) && a.id != finder && a.pos.dist(at) <= 14
        })
        .collect();
    let &s = suspects.iter().min_by(|&&x, &&y| {
        let (ox, oy) = (
            w.agents[fi].opinion_of(w.agents[x].id),
            w.agents[fi].opinion_of(w.agents[y].id),
        );
        ox.total_cmp(&oy).then(x.cmp(&y))
    })?;
    let blamed = w.agents[s].id;
    let tick = w.meta.tick;
    let ev = accuse(w, cid, finder, blamed, truly, what, at);
    w.agents[fi].remember(
        tick,
        crate::mind::MemKind::Hardship,
        crate::mind::Subject::Person(blamed),
        -0.4,
        0.5,
        ev,
    );
    w.agents[fi].relate(blamed, -0.3, 0.1, tick);
    Some(blamed)
}

/// A household keeper's chest is short by `units`: nobody saw who took it (#1796). Recorded on
/// `cid`'s community (the household is not yet its own record; the whole settlement stands in
/// for it) against the real taker `truly`, capped at the last 4 unfound shortages.
pub fn report_missing_food(
    w: &mut World,
    cid: CommunityId,
    truly: AgentId,
    units: u32,
    at: TilePos,
) {
    let tick = w.meta.tick;
    let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) else {
        return;
    };
    c.missing_food.push(MissingFood {
        units,
        truly,
        at,
        tick,
    });
    if c.missing_food.len() > 4 {
        c.missing_food.remove(0);
    }
}

/// An unfound shortage older than this many ticks is given up on (#1796): nobody checked the
/// chest in time to ever lay blame for it.
const MISSING_FOOD_STALE: u64 = 10 * TICKS_PER_DAY;

/// A member of a household standing this close to its chest, awake, can find it short (#1796).
const MISSING_FOOD_FIND_RANGE: i32 = 3;

/// Once a day: any community member who comes within reach of a chest with an unfound shortage
/// finds it short and lays blame by D48's rule (#1796), as its own entry rather than through
/// `offence` (whose `seen` test would count the finder standing at the chest as a witness and
/// correctly name the true taker — see `lay_blame`'s doc comment).
pub fn find_missing_food(w: &mut World) {
    let tick = w.meta.tick;
    let cids: Vec<CommunityId> = w.communities.iter().map(|c| c.id).collect();
    for cid in cids {
        let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
            continue;
        };
        if c.missing_food.is_empty() {
            continue;
        }
        let entries = c.missing_food.clone();
        let mut kept = Vec::new();
        for entry in entries {
            if tick.saturating_sub(entry.tick) > MISSING_FOOD_STALE {
                continue;
            }
            let found = (0..w.agents.len()).find(|&k| {
                let a = &w.agents[k];
                a.alive
                    && a.community == Some(cid)
                    && a.pos.z == entry.at.z
                    && a.pos.dist(entry.at) <= MISSING_FOOD_FIND_RANGE
                    && !a.activity.is_asleep()
            });
            match found {
                Some(k) => {
                    let finder = w.agents[k].id;
                    lay_blame(w, finder, entry.truly, "missing food", entry.at);
                }
                None => kept.push(entry),
            }
        }
        if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
            c.missing_food = kept;
        }
    }
}

/// #322: letting the deep's people up through a way down (societies.rs). Always taken up, and
/// a conviction is exile.
pub const DEEP_GATE: &str = "let the deep's people up";
/// #404: setting a fire in a rage (systems/strain.rs). Always taken up, and a conviction is exile.
pub const ARSON: &str = "setting a fire";

/// #404: a death from a wound struck in a fight (systems/health.rs). Always taken up, and a
/// conviction is exile.
pub const KILLING: &str = "a killing";
/// #404: a limb lost to a wound struck in a fight. Always taken up, and a conviction is exile.
pub const MAIMING: &str = "maiming";

/// The gravest offences: always taken up, and a conviction is exile, liked or not (#322, #404).
pub fn grave(what: &str) -> bool {
    what == DEEP_GATE || what == ARSON || what == KILLING || what == MAIMING
}

/// `victim` dies (`lost` None) or loses the part `lost` of a wound `striker` gave them in a fight
/// between people of the valley (systems/health.rs; #404). Called while the dead are still
/// living, so their kin can be found. On record; the two families are in a feud at once
/// (feuds.rs); and it is taken up for the evening's judging: the dead's nearest kin accuses, or
/// failing kin the one they named before they died; the maimed accuse for themselves.
pub fn struck_down(w: &mut World, striker: AgentId, victim: AgentId, lost: Option<&str>) {
    let tick = w.meta.tick;
    let of_valley =
        |a: &crate::agents::Agent| a.species == crate::agents::Species::Person && a.band.is_none();
    let Some(si) = w.agents.iter().position(|a| a.id == striker) else {
        return;
    };
    let Some(vi) = w.agents.iter().position(|a| a.id == victim) else {
        return;
    };
    if striker == victim || !of_valley(&w.agents[si]) || !of_valley(&w.agents[vi]) {
        return;
    }
    let at = w.agents[vi].pos;
    match lost {
        None => w.events.push(
            tick,
            Some(at),
            Severity::Major,
            EventKind::Killing {
                agent: victim,
                by: striker,
            },
        ),
        Some(part) => w.events.push(
            tick,
            Some(at),
            Severity::Major,
            EventKind::Maiming {
                agent: victim,
                by: striker,
                part: part.to_string(),
            },
        ),
    };
    // The dead hold no feud: one is taken up only when they leave kin in the valley.
    let kin_left = lost.is_some()
        || crate::feuds::kin(w, victim).iter().any(|k| {
            *k != victim
                && w.agents
                    .iter()
                    .any(|a| a.id == *k && a.alive && a.community.is_some())
        });
    let cid = w.agents[vi].community.or(w.agents[si].community);
    if let (Some(cid), true) = (cid, kin_left) {
        crate::feuds::grievance(w, cid, striker, victim, true);
    }
    // Judged where the one who struck lives, if they are still of a community.
    let (Some(scid), true) = (w.agents[si].community, w.agents[si].alive) else {
        return;
    };
    let by = match lost {
        Some(_) => victim,
        None => crate::feuds::kin(w, victim)
            .into_iter()
            .filter(|k| *k != victim && *k != striker)
            .filter_map(|k| w.agents.iter().find(|a| a.id == k && a.alive))
            .filter(|a| a.community.is_some())
            .min_by_key(|a| (a.pos.dist(at), a.id))
            .map(|a| a.id)
            .unwrap_or(victim),
    };
    let what = if lost.is_some() { MAIMING } else { KILLING };
    accuse(w, scid, by, striker, striker, what, at);
}

/// `by` accuses `of` of `what` (done in truth by `truly`): on the community's list for the
/// evening's judging, and on record.
pub fn accuse(
    w: &mut World,
    cid: CommunityId,
    by: AgentId,
    of: AgentId,
    truly: AgentId,
    what: &str,
    at: TilePos,
) -> Option<crate::ids::EventId> {
    let tick = w.meta.tick;
    if by == of {
        return None;
    }
    if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
        c.accused.push(Accusation {
            by,
            of,
            truly,
            what: what.to_string(),
            at: tick,
        });
    }
    Some(w.events.push(
        tick,
        Some(at),
        Severity::Notable,
        EventKind::Accused {
            by,
            of,
            what: what.to_string(),
        },
    ))
}

/// Once a day: deference fades and the followed are worked out; at six in the evening the
/// day's accusations are judged.
pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_DAY == 17 * TICKS_PER_HOUR + 300 {
        follow(w);
    }
    if tick % TICKS_PER_DAY == 18 * TICKS_PER_HOUR + 300 {
        judge_all(w);
    }
    if tick.is_multiple_of(TICKS_PER_DAY) {
        find_missing_food(w);
    }
}

fn follow(w: &mut World) {
    let tick = w.meta.tick;
    for a in w.agents.iter_mut() {
        for x in a.defers.iter_mut() {
            for v in x.1.iter_mut() {
                *v *= 0.95;
            }
        }
        a.defers.retain(|x| x.1.iter().sum::<f32>() > 0.01);
    }
    let cids: Vec<CommunityId> = w
        .communities
        .iter()
        .filter(|c| !c.dissolved)
        .map(|c| c.id)
        .collect();
    for cid in cids {
        let mut sums: std::collections::BTreeMap<AgentId, [f32; 4]> = Default::default();
        for a in w
            .agents
            .iter()
            .filter(|a| a.alive && a.community == Some(cid))
        {
            for (to, v) in &a.defers {
                let e = sums.entry(*to).or_insert([0.0; 4]);
                for d in 0..4 {
                    e[d] += v[d];
                }
            }
        }
        for d in ALL {
            let best = sums
                .iter()
                .filter(|(id, v)| {
                    v[d as usize] >= FOLLOWED_AT
                        && w.agents
                            .iter()
                            .any(|a| a.id == **id && a.alive && a.community == Some(cid))
                })
                .max_by(|a, b| {
                    a.1[d as usize]
                        .total_cmp(&b.1[d as usize])
                        .then(b.0.cmp(a.0))
                })
                .map(|(id, _)| *id);
            let c = w.communities.iter_mut().find(|c| c.id == cid).unwrap();
            if c.leaders[d as usize] != best {
                c.leaders[d as usize] = best;
                if let Some(agent) = best {
                    let at = w.agents.iter().find(|a| a.id == agent).map(|a| a.pos);
                    w.events.push(
                        tick,
                        at,
                        Severity::Notable,
                        EventKind::Followed {
                            agent,
                            domain: d.word().to_string(),
                        },
                    );
                }
            }
        }
    }
}

/// Who judges in `cid`: the one followed in the social domain, else the eldest adult; never
/// the accused.
fn judge_of(w: &World, cid: CommunityId, not: AgentId) -> Option<usize> {
    let c = w.communities.iter().find(|c| c.id == cid)?;
    if let Some(l) = c.leaders[Domain::Social as usize].filter(|l| *l != not) {
        if let Some(i) = idx(w, l) {
            return Some(i);
        }
    }
    let tick = w.meta.tick;
    (0..w.agents.len())
        .filter(|&k| {
            let a = &w.agents[k];
            a.alive && a.community == Some(cid) && a.id != not && !a.is_child(tick)
        })
        .max_by_key(|&k| {
            (
                w.agents[k].age_years(tick),
                std::cmp::Reverse(w.agents[k].id),
            )
        })
}

fn judge_all(w: &mut World) {
    let tick = w.meta.tick;
    let cids: Vec<CommunityId> = w.communities.iter().map(|c| c.id).collect();
    for cid in cids {
        let pending = std::mem::take(
            &mut w
                .communities
                .iter_mut()
                .find(|c| c.id == cid)
                .unwrap()
                .accused,
        );
        for acc in pending {
            let Some(oi) = idx(w, acc.of) else { continue };
            if w.agents[oi].community != Some(cid) {
                continue;
            }
            let Some(ji) = judge_of(w, cid, acc.of) else {
                continue;
            };
            let judge = w.agents[ji].id;
            let prior = w
                .communities
                .iter()
                .find(|c| c.id == cid)
                .map(|c| {
                    c.record
                        .iter()
                        .filter(|r| r.0 == acc.of && tick.saturating_sub(r.2) < 40 * TICKS_PER_DAY)
                        .count()
                })
                .unwrap_or(0);
            let liked = w.agents[ji].opinion_of(acc.of) >= 0.0;
            // Exile is for one who keeps starting fights and whom the judge does not like (C77
            // soaks: 17 to 28 exiles a run when any third offence was enough).
            let violent = acc.what.contains("fight");
            // #322 (2026-10-26 fix-322-reckoning): letting the deep up is exile, liked or not;
            // #404 (2026-10-27 fix-404-feuds): so is setting a fire.
            let grave = grave(&acc.what);
            let verdict = if grave || (prior >= 2 && violent && !liked) {
                "exiled"
            } else if prior == 0 && liked {
                "pardoned"
            } else {
                "to make amends"
            };
            let wrongly = acc.of != acc.truly;
            let at = w.agents[oi].pos;
            w.events.push(
                tick,
                Some(at),
                Severity::Notable,
                EventKind::Judged {
                    judge,
                    of: acc.of,
                    what: acc.what.clone(),
                    verdict: verdict.to_string(),
                    wrongly,
                },
            );
            if let Some(c) = w.communities.iter_mut().find(|c| c.id == cid) {
                c.record.push((acc.of, acc.by, tick));
            }
            match verdict {
                "to make amends" => {
                    // Made good: the grudge between them eases.
                    w.agents[oi].relate(acc.by, 0.2, 0.1, tick);
                    if let Some(b) = idx(w, acc.by) {
                        w.agents[b].relate(acc.of, 0.3, 0.1, tick);
                    }
                }
                "exiled" => {
                    if acc.what == DEEP_GATE && acc.of == acc.truly {
                        // The one who did it, convicted: what they were part of comes out.
                        crate::societies::exposed(w, cid, acc.of);
                    }
                    exile(w, oi)
                }
                _ => {}
            }
            if wrongly {
                // The wronged hold it against judge and accuser, and their friends follow the
                // judge less (a verdict's legitimacy is in who still defers after it).
                w.agents[oi].relate(judge, -0.4, 0.1, tick);
                w.agents[oi].relate(acc.by, -0.4, 0.1, tick);
                let friends: Vec<usize> = (0..w.agents.len())
                    .filter(|&k| {
                        w.agents[k].alive
                            && w.agents[k].community == Some(cid)
                            && w.agents[k].opinion_of(acc.of) > 0.3
                    })
                    .collect();
                for k in friends {
                    if let Some(x) = w.agents[k].defers.iter_mut().find(|x| x.0 == judge) {
                        x.1[Domain::Social as usize] *= 0.5;
                    }
                }
            }
            // #1807 (2026-11-16): a verdict against the accused, other than a pardon, leaves
            // their family cool toward whoever accused them, whatever the feud rules do later.
            if verdict != "pardoned" {
                for k in crate::feuds::kin(w, acc.of) {
                    if k == acc.of || k == acc.by {
                        continue;
                    }
                    if let Some(ki) = idx(w, k) {
                        w.agents[ki].relate(acc.by, -0.1, 0.02, tick);
                    }
                }
            }
            // #404 (2026-10-27 fix-404-feuds): a grievance between families, not only the pair.
            if !crate::feuds::judged(w, cid, judge, acc.of, acc.by, verdict) {
                crate::feuds::grievance(w, cid, acc.of, acc.by, verdict == "exiled");
            }
        }
    }
}

/// Out of the community and the valley, to the nearest inhabited place beyond it, which holds
/// it against the valley (siege.rs grudges).
fn exile(w: &mut World, i: usize) {
    let tick = w.meta.tick;
    let who = w.agents[i].id;
    let at = w.agents[i].pos;
    w.agents[i].community = None;
    w.agents[i].leaving = true;
    let to = w
        .overworld
        .sites
        .iter_mut()
        .filter(|s| s.people > 0)
        .min_by_key(|s| (s.days, s.id))
        .map(|s| {
            s.people += 1;
            s.grudge += 1;
            s.name.clone()
        })
        .unwrap_or_else(|| "the wilds".to_string());
    w.events.push(
        tick,
        Some(at),
        Severity::Major,
        EventKind::Exiled { agent: who, to },
    );
}
