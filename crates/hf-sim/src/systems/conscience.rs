//! Judging a quarrel afterwards (#568, mind/conscience.rs): once an hour, each person with a
//! quarrel still to judge looks for the other; if they see them hurt, sick or grieving, they
//! regret it or rationalise it, by their own tendency, and it changes what they think of the
//! other and how readily they help them. Seen only within sight on their own level, so a person
//! never judges by what they could not know.

use crate::events::{EventKind, Severity};
use crate::mind::conscience::{stance_of, Found, Stance, RATIONALISE_OPINION, REGRET_OPINION};
use crate::mind::feelings::FeelingKind;
use crate::time::TICKS_PER_HOUR;
use crate::world_state::World;

/// How near the other must be for a wound, a sickness or grief to be seen.
pub const SEE_RANGE: i32 = 8;

/// What the other is, as one standing near would see it.
fn found(o: &crate::agents::Agent, now: u64) -> Option<Found> {
    if o.body.ill() >= 0.4 {
        Some(Found::Sick)
    } else if o.injured(now) {
        Some(Found::Hurt)
    } else if o.feelings.of(FeelingKind::Grief) >= 0.5 {
        Some(Found::Grieving)
    } else {
        None
    }
}

pub fn step(w: &mut World) {
    let now = w.meta.tick;
    if !now.is_multiple_of(TICKS_PER_HOUR) {
        return;
    }
    for i in 0..w.agents.len() {
        if !w.agents[i].alive || w.agents[i].conscience.judgements.is_empty() {
            continue;
        }
        w.agents[i].conscience.lapse(now);
        let pending: Vec<_> = w.agents[i].conscience.pending().map(|j| j.other).collect();
        for other in pending {
            let me = w.agents[i].pos;
            let Some(j) = w.agents.iter().position(|o| o.id == other) else {
                continue;
            };
            let o = &w.agents[j];
            if !o.alive
                || o.pos.z != me.z
                || o.pos.dist(me) > SEE_RANGE
                || !crate::mind::perception::line_of_sight(&w.grid, me, o.pos)
            {
                continue;
            }
            let Some(found) = found(o, now) else {
                continue;
            };
            let name = o.name.clone();
            let a = &mut w.agents[i];
            let stance = stance_of(a);
            let opinion = match stance {
                Stance::Regret => REGRET_OPINION,
                Stance::Rationalise => RATIONALISE_OPINION,
            };
            a.relate(other, opinion, 0.0, now);
            a.conscience.judge(other, stance, found, opinion, now);
            let (id, pos) = (a.id, a.pos);
            let (what, because) = match stance {
                Stance::Regret => (
                    format!("regretted quarrelling with {name}"),
                    format!("{name} was {} all along", found.words()),
                ),
                Stance::Rationalise => (
                    format!("told themselves {name} had it coming"),
                    format!(
                        "{name} was {}, but they would not own the quarrel",
                        found.words()
                    ),
                ),
            };
            w.events.push(
                now,
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
