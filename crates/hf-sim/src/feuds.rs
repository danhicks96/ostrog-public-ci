//! Feuds between families (#404; docs/PLAN-FINISH.md M5, "a feud across two generations").
//!
//! A feud is between two families, not two people. Each side is held by the kin of the one it
//! began with: partner, parents, children, brothers and sisters. It begins when the judging
//! (justice.rs) has set the two families against each other twice within sixty days, whoever of
//! each did the wrong or took it, or at once when one of them was exiled on the other's word or
//! killed or maimed one of the other in a fight (`justice::struck_down`). Holding it, each side
//! thinks the worse of the other and does not seek the other's company (systems/decide.rs).
//!
//! It passes on. A child born to one who holds it holds it too, and so does whoever pairs with
//! one who holds it. When the one a side is named for dies or leaves the valley, the eldest of
//! their children who holds it takes it up, or failing a child the eldest who holds it.
//!
//! It ends with a marriage between the sides; with a reckoning, once each side has made amends
//! to the other; by the judgement of the one followed in settling what is between people, when
//! they judge a case between the sides and belong to neither; when five years pass with no new
//! grievance between them (a quarrel or a fight between the sides is one); or when one side has
//! nobody left in the valley.

use crate::events::{EventKind, Severity};
use crate::ids::{AgentId, CommunityId};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR, TICKS_PER_YEAR};
use crate::World;
use serde::{Deserialize, Serialize};

/// Two grievances between the same two families within this long begin a feud (justice.rs's
/// record).
pub const WITHIN: u64 = 60 * TICKS_PER_DAY;
/// A feud with no new grievance for this long is let go.
pub const FORGOTTEN: u64 = 5 * TICKS_PER_YEAR;
/// The chance the one followed in settling what is between people ends a feud when they judge a
/// case between its sides.
pub const SETTLED_BY_JUDGE: f64 = 0.25;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Feud {
    /// The one each side is named for: who it began with, then whoever took it up.
    pub heads: [AgentId; 2],
    /// Who holds it, on each side.
    pub sides: [Vec<AgentId>; 2],
    pub began: u64,
    /// The last grievance between the sides.
    pub last: u64,
    /// Each side has made amends to the other since it began.
    pub amends: [bool; 2],
}

impl Feud {
    pub fn side_of(&self, id: AgentId) -> Option<usize> {
        (0..2).find(|&s| self.sides[s].contains(&id))
    }
    /// `a` and `b` are on opposite sides: the side `a` is on.
    pub fn between(&self, a: AgentId, b: AgentId) -> Option<usize> {
        let s = self.side_of(a)?;
        (self.side_of(b) == Some(1 - s)).then_some(s)
    }
}

fn living(w: &World, id: AgentId) -> Option<&crate::agents::Agent> {
    w.agents.iter().find(|a| a.id == id && a.alive)
}

/// `id` and their living kin: partner, parents, children, brothers and sisters. The dead have
/// kin too (a killing sets the dead's family against the killer's, #404).
pub fn kin(w: &World, id: AgentId) -> Vec<AgentId> {
    let mut out = vec![id];
    let Some(a) = w.agents.iter().find(|a| a.id == id) else {
        return out;
    };
    let mut near: Vec<AgentId> = Vec::new();
    near.extend(a.partner);
    near.extend(a.mother);
    near.extend(a.father);
    near.extend(a.children.iter().copied());
    for o in w.agents.iter().filter(|o| o.alive && o.id != id) {
        let sibling = (a.mother.is_some() && o.mother == a.mother)
            || (a.father.is_some() && o.father == a.father);
        if sibling {
            near.push(o.id);
        }
    }
    for k in near {
        if !out.contains(&k) && living(w, k).is_some() {
            out.push(k);
        }
    }
    out
}

/// `a` and `b` are on opposite sides of a feud.
pub fn across(w: &World, a: AgentId, b: AgentId) -> bool {
    w.feuds.iter().any(|f| f.between(a, b).is_some())
}

/// The judging set `offender` against `wronged` (justice.rs, after the record row is written):
/// a grievance between their families. `grave` (an exile on the other's word) begins a feud at
/// once; else a second within sixty days between the same families does.
pub fn grievance(
    w: &mut World,
    cid: CommunityId,
    offender: AgentId,
    wronged: AgentId,
    grave: bool,
) {
    let tick = w.meta.tick;
    if let Some(f) = w
        .feuds
        .iter_mut()
        .find(|f| f.between(offender, wronged).is_some())
    {
        f.last = tick;
        return;
    }
    let (ko, kw) = (kin(w, offender), kin(w, wronged));
    if ko.contains(&wronged) {
        // A family's own quarrel is not a feud.
        return;
    }
    let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
        return;
    };
    let times = c
        .record
        .iter()
        .filter(|r| {
            tick.saturating_sub(r.2) < WITHIN
                && ((ko.contains(&r.0) && kw.contains(&r.1))
                    || (kw.contains(&r.0) && ko.contains(&r.1)))
        })
        .count();
    // Each family needs someone still in the valley to hold it (an exile's kin, say).
    let held = |k: &[AgentId]| k.iter().any(|id| here(w, *id));
    if (grave || times >= 2) && held(&ko) && held(&kw) {
        begin(w, offender, wronged);
    }
}

/// The families of `a` and `b` are in a feud.
pub fn begin(w: &mut World, a: AgentId, b: AgentId) {
    let tick = w.meta.tick;
    let (ka, kb) = (kin(w, a), kin(w, b));
    let sides = [
        ka.iter().copied().filter(|x| !kb.contains(x)).collect(),
        kb.iter().copied().filter(|x| !ka.contains(x)).collect(),
    ];
    let f = Feud {
        heads: [a, b],
        sides,
        began: tick,
        last: tick,
        amends: [false, false],
    };
    set_against(w, &f.sides[0].clone(), &f.sides[1].clone());
    let at = living(w, a).or_else(|| living(w, b)).map(|x| x.pos);
    w.feuds.push(f);
    w.events
        .push(tick, at, Severity::Notable, EventKind::Feud { a, b });
}

/// Each of `xs` thinks the worse of each of `ys`, and the other way about.
fn set_against(w: &mut World, xs: &[AgentId], ys: &[AgentId]) {
    let tick = w.meta.tick;
    for (p, q) in [(xs, ys), (ys, xs)] {
        for x in p {
            if let Some(i) = w.agents.iter().position(|a| a.id == *x && a.alive) {
                for y in q {
                    w.agents[i].relate(*y, -0.15, 0.05, tick);
                }
            }
        }
    }
}

/// A case between the sides of a feud was judged (justice.rs): `of` against `by`, `verdict`, by
/// `judge`. Amends made toward a reckoning; the one followed in such things may settle it.
/// True when it ended the feud.
pub fn judged(
    w: &mut World,
    cid: CommunityId,
    judge: AgentId,
    of: AgentId,
    by: AgentId,
    verdict: &str,
) -> bool {
    let tick = w.meta.tick;
    let Some(n) = w.feuds.iter().position(|f| f.between(of, by).is_some()) else {
        return false;
    };
    let s = w.feuds[n].between(of, by).unwrap();
    if verdict == "to make amends" {
        w.feuds[n].amends[s] = true;
    }
    if w.feuds[n].amends == [true, true] {
        end(
            w,
            n,
            "a reckoning: each side has made amends to the other",
            None,
        );
        return true;
    }
    let leader = w
        .communities
        .iter()
        .find(|c| c.id == cid)
        .and_then(|c| c.leaders[crate::justice::Domain::Social as usize]);
    if leader == Some(judge) && w.feuds[n].side_of(judge).is_none() {
        use rand::Rng;
        let mut r = crate::rng::stream(
            w.meta.seed,
            &[
                crate::rng::purpose::AGENT_SOCIAL,
                0x404F,
                judge.0 as u64,
                tick,
            ],
        );
        if r.gen_bool(SETTLED_BY_JUDGE) {
            end(w, n, "settled by the judgement of", Some(judge));
            return true;
        }
    }
    false
}

fn end(w: &mut World, n: usize, how: &str, by: Option<AgentId>) {
    let tick = w.meta.tick;
    let f = w.feuds.remove(n);
    let at = f.heads.iter().find_map(|h| living(w, *h)).map(|a| a.pos);
    w.events.push(
        tick,
        at,
        Severity::Notable,
        EventKind::FeudEnded {
            a: f.heads[0],
            b: f.heads[1],
            how: how.to_string(),
            by,
        },
    );
}

/// The events of the last tick: births and pairings carry a feud on; a marriage between the
/// sides ends it; a quarrel or a fight between them keeps it alive. Once a day, at six in the
/// morning: the dead let go of it, a side's head passes on, and old or empty feuds end.
pub fn observe(w: &mut World) {
    if w.feuds.is_empty() {
        return;
    }
    let tick = w.meta.tick;
    let events: Vec<EventKind> = w.last_events.iter().map(|e| e.kind.clone()).collect();
    for k in events {
        match k {
            EventKind::Born {
                agent,
                mother,
                father,
            } => {
                for f in w.feuds.iter_mut() {
                    let s = f
                        .side_of(mother)
                        .or_else(|| father.and_then(|x| f.side_of(x)));
                    if let (Some(s), None) = (s, f.side_of(agent)) {
                        f.sides[s].push(agent);
                    }
                }
            }
            EventKind::Paired { a, b } => {
                if let Some(n) = w.feuds.iter().position(|f| f.between(a, b).is_some()) {
                    end(w, n, "a marriage between the two families", None);
                }
                let mut joined: Vec<(AgentId, Vec<AgentId>)> = Vec::new();
                for f in w.feuds.iter_mut() {
                    for (x, y) in [(a, b), (b, a)] {
                        if let (Some(s), None) = (f.side_of(x), f.side_of(y)) {
                            f.sides[s].push(y);
                            joined.push((y, f.sides[1 - s].clone()));
                        }
                    }
                }
                for (y, other) in joined {
                    set_against(w, &[y], &other);
                }
            }
            EventKind::Fought { a, b } | EventKind::Quarrelled { a, b } => {
                for f in w.feuds.iter_mut() {
                    if f.between(a, b).is_some() {
                        f.last = tick;
                    }
                }
            }
            _ => {}
        }
    }
    if tick % TICKS_PER_DAY == 6 * TICKS_PER_HOUR + 400 {
        daily(w);
    }
}

/// In the valley: alive and of a community. Those away on a journey still count:
/// `overworld.away` holds them whole while they are off the map, and a short trip must not read
/// as nobody of the side being left.
fn here(w: &World, id: AgentId) -> bool {
    living(w, id).is_some_and(|a| a.community.is_some())
        || w.overworld
            .away
            .iter()
            .any(|a| a.id == id && a.alive && a.community.is_some())
}

fn daily(w: &mut World) {
    let tick = w.meta.tick;
    let mut n = 0;
    while n < w.feuds.len() {
        // The dead let go of it (those away on a journey still hold it).
        let dead = |w: &World, id: AgentId| w.agents.iter().any(|a| a.id == id && !a.alive);
        for s in 0..2 {
            let keep: Vec<AgentId> = w.feuds[n].sides[s]
                .iter()
                .copied()
                .filter(|id| !dead(w, *id))
                .collect();
            w.feuds[n].sides[s] = keep;
        }
        if (0..2).any(|s| !w.feuds[n].sides[s].iter().any(|id| here(w, *id))) {
            end(w, n, "nobody of one side is left in the colony", None);
            continue;
        }
        if tick.saturating_sub(w.feuds[n].last) >= FORGOTTEN {
            end(w, n, "five years without a new grievance, and let go", None);
            continue;
        }
        for s in 0..2 {
            let head = w.feuds[n].heads[s];
            if here(w, head) {
                continue;
            }
            let children: Vec<AgentId> = w
                .agents
                .iter()
                .find(|a| a.id == head)
                .map(|a| a.children.clone())
                .unwrap_or_default();
            let eldest = |pool: &mut dyn Iterator<Item = AgentId>| {
                pool.filter(|id| here(w, *id))
                    .filter_map(|id| living(w, id))
                    .min_by_key(|a| (a.born, a.id))
                    .map(|a| a.id)
            };
            let side = w.feuds[n].sides[s].clone();
            let heir = eldest(&mut side.iter().copied().filter(|id| children.contains(id)))
                .or_else(|| eldest(&mut side.iter().copied()));
            let Some(heir) = heir else { continue };
            w.feuds[n].heads[s] = heir;
            let against = w.feuds[n].heads[1 - s];
            let at = living(w, heir).map(|a| a.pos);
            w.events.push(
                tick,
                at,
                Severity::Notable,
                EventKind::FeudInherited {
                    agent: heir,
                    from: head,
                    against,
                },
            );
        }
        n += 1;
    }
}
