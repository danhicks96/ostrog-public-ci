//! Blows, nerve, and the valley standing together (C75).
//!
//! One blow is one roll: a weapon lands more often and cuts deeper, armour turns some and
//! blunts the rest. Nerve is worked out when it is needed, not stored: a person's lean on
//! violence, the company at hand, a spear in the hand, pain, and a cautious nature. Whoever
//! has the nerve stands; whoever has not runs for home.
use crate::agents::Species;
use crate::body::{Part, WoundKind};
use crate::ids::AgentId;
use crate::mind::beliefs::Topic;
use crate::world_state::World;
use rand::Rng;

/// How hurt someone is: the sum of their open wounds' severity. A body that is not a skeleton
/// is as hurt as it is near to too little of itself to live (#423, body.rs `Form::harm`), or its
/// head's wounds if more: at 1 it is felled.
pub fn battered(w: &World, i: usize) -> f32 {
    let b = &w.agents[i].body;
    let wounds: f32 = b.wounds.iter().map(|x| x.severity).sum();
    match b.form {
        Some(f) => f.harm().max(wounds),
        None => wounds,
    }
}

/// Whether a hunter `i` takes the beast `j` (#423): a skeleton at a stroke, as ever; a body that
/// is not one only if the hunter's blows, up to three, leave too little of it to live. One that
/// is left alive gets away.
pub fn hunted_down(w: &mut World, i: usize, j: usize) -> bool {
    if w.agents[j].body.form.is_none() {
        return true;
    }
    let mut r = crate::rng::stream(
        w.meta.seed,
        &[
            crate::rng::purpose::AGENT_WORK,
            0x4232,
            w.agents[i].id.0 as u64,
            w.meta.tick,
        ],
    );
    for _ in 0..3 {
        blow(w, i, j, &mut r, "on the hunt");
        if battered(w, j) >= 1.0 {
            return true;
        }
    }
    false
}

/// Nerve, 0 to 1. Above 0.4 a person will stand by a neighbour; under 0.25 they run.
pub fn nerve(w: &World, i: usize) -> f32 {
    let a = &w.agents[i];
    if a.species != Species::Person {
        return 1.0 - battered(w, i).min(1.0);
    }
    let company = w
        .agents
        .iter()
        .filter(|o| {
            o.alive
                && o.id != a.id
                && o.species == Species::Person
                && o.community.is_some()
                && o.community == a.community
                && o.pos.z == a.pos.z
                && o.pos.dist(a.pos) <= 6
        })
        .count()
        .min(4) as f32;
    let armed = if crate::systems::arms::edge(w, i).is_some() {
        0.15
    } else {
        0.0
    };
    let cautious = if a.traits.has(crate::mind::Trait::Cautious) {
        0.15
    } else {
        0.0
    };
    (0.45 + 0.35 * a.beliefs.lean(Topic::Violence) + 0.08 * company + armed
        - 0.6 * battered(w, i).min(1.0)
        - cautious)
        .clamp(0.0, 1.0)
}

/// Someone of fighting age, awake and whole enough to stand.
pub fn can_stand(w: &World, i: usize) -> bool {
    let a = &w.agents[i];
    a.alive
        && a.species == Species::Person
        && a.age_years(w.meta.tick) >= 14
        && !a.activity.is_asleep()
        && !a.body.needs_care()
}

/// `y` strikes at `x`. Returns true if the blow landed. `why` is the cause for the feed.
pub fn blow(w: &mut World, y: usize, x: usize, r: &mut impl Rng, why: &str) -> bool {
    let other: AgentId = w.agents[y].id;
    let beast = w.agents[y].species != Species::Person;
    let edge = if beast {
        0.0
    } else {
        crate::systems::arms::edge(w, y).map(|e| e.1).unwrap_or(0.0)
    };
    let guard = crate::systems::arms::guard(w, x)
        .map(|g| g.1)
        .unwrap_or(0.0);
    if !r.gen_bool((0.65 + 0.15 * edge as f64).min(0.95)) {
        return false;
    }
    if edge > 0.0 {
        crate::systems::arms::struck_with(w, y, false);
    }
    if guard > 0.0 {
        crate::systems::arms::took_blow(w, x);
        if r.gen_bool((0.3 * guard as f64).min(0.5)) {
            return false; // turned by the armour
        }
    }
    let part = [
        Part::HEAD,
        Part::TORSO,
        Part::ARM_L,
        Part::ARM_R,
        Part::LEG_L,
    ][r.gen_range(0..5)];
    let kind = if beast || edge > 0.0 || r.gen_bool(0.4) {
        WoundKind::Cut
    } else {
        WoundKind::Bruise
    };
    let base = if beast { 0.3 } else { 0.2 };
    let sev = (base + r.gen::<f32>() * 0.4) * (1.0 + 0.5 * edge) * (1.0 - 0.5 * guard.min(1.0));
    // Size (#422): a heavy body hits harder and takes more to hurt, by mass to the half power
    // against the human mean (size.rs `might`). Two people of middling size are as before; a
    // bear's blow is about twice a person's, a hare's a quarter, and a person's blow on a bear
    // goes half as deep.
    let now = w.meta.tick;
    let sev = sev * crate::size::might(w.agents[y].body_kg(now))
        / crate::size::might(w.agents[x].body_kg(now));
    // A blow on a part already hurt goes deeper (#402, M5 review 2026-10-26: one person took 592
    // blows, each a fresh wound no deeper than the first, and lived). Half of what is still open
    // on the part is added, so blow after blow to the body or the head reaches the organs and
    // bleeds inside (health.rs `wound`, the layers of C74).
    let open: f32 = w.agents[x]
        .body
        .wounds
        .iter()
        .filter(|wd| wd.part == part)
        .map(|wd| wd.severity)
        .sum();
    let sev = sev + 0.5 * open;
    let cause = if edge > 0.0 {
        format!("by a spear {why}")
    } else {
        why.to_string()
    };
    // Beasts are not doctored: their wounds are kept on the body only to tell how hurt they
    // are (health.rs runs for people alone).
    crate::systems::health::wound(w, x, part, kind, sev, 0.0, &cause, Some(other));
    true
}

/// A beast has gone for someone at `victim`. Everyone near with the nerve for it stands with
/// them and strikes; those without it run. The beast is felled when hurt enough, and goes off
/// hurt otherwise. Returns (who stood, felled).
pub fn stand_against(
    w: &mut World,
    beast: usize,
    victim: usize,
    r: &mut impl Rng,
) -> (Vec<AgentId>, bool) {
    let at = w.agents[beast].pos;
    let near: Vec<usize> = (0..w.agents.len())
        .filter(|&k| {
            // The one just bitten fights back hurt (the bite already makes them need care,
            // which kept every lone victim from standing in the C75 soaks).
            let able = if k == victim {
                let a = &w.agents[k];
                a.alive && a.age_years(w.meta.tick) >= 14 && !a.activity.is_asleep()
            } else {
                can_stand(w, k)
            };
            able && w.agents[k].pos.z == at.z && w.agents[k].pos.dist(at) <= 6
        })
        .collect();
    let mut stood = Vec::new();
    for k in near {
        let n = nerve(w, k);
        // The one gone for has least choice: they stand on less.
        let bar = if k == victim { 0.3 } else { 0.4 };
        if n >= bar {
            stood.push(k);
        } else if n < 0.25 && k != victim {
            run_home(w, k);
        }
    }
    let mut felled = false;
    for &k in &stood {
        if !w.agents[beast].alive {
            break;
        }
        // Go in close first: the blow is struck from beside the beast.
        // A neighbour in a beast's shape is hurt like anyone (health.rs takes it from there);
        // only a true beast is felled here, and its hide taken.
        if blow(w, k, beast, r, "fighting off a beast")
            && w.agents[beast].species != Species::Person
            && battered(w, beast) >= 1.0
        {
            felled = true;
            crate::creatures::slain(w, k, beast);
            let name = w.agents[k].name.clone();
            // A swarm is scattered, a soft body broken up, a chain cut to pieces (#423).
            let how = w.agents[beast].body.form.map_or("killed", |f| f.end());
            crate::systems::life::die(w, w.agents[beast].id, format!("{how} by {name}"));
        }
    }
    let ids = stood.iter().map(|&k| w.agents[k].id).collect();
    (ids, felled)
}

/// Off home, at a run, away from whatever it is.
pub fn run_home(w: &mut World, i: usize) {
    let home = w.agents[i]
        .community
        .and_then(|c| w.communities.iter().find(|x| x.id == c))
        .map(|x| x.home);
    if let Some(h) = home {
        if w.agents[i].pos.dist(h) > 2 && w.path_agent_to(i, h) {
            w.agents[i].activity = crate::mind::Activity::Wandering { to: h };
            w.agents[i].activity_ticks = 0;
        }
    }
}

/// C75: the Beast-hunger's shape. Until dawn each shifted person hunts the nearest person out
/// alone; on reaching them they tear at them once, and whoever stands fights back (wounding a
/// neighbour, who will be themselves again by morning). One attack a night.
pub fn shifted(w: &mut World) {
    let tick = w.meta.tick;
    if w.creatures.shifted.is_empty() {
        return;
    }
    let list = std::mem::take(&mut w.creatures.shifted);
    let mut keep = Vec::new();
    for (id, until) in list {
        let Some(i) = w.agents.iter().position(|a| a.id == id && a.alive) else {
            continue;
        };
        if tick >= until {
            continue; // themselves again
        }
        let me = w.agents[i].pos;
        let prey = (0..w.agents.len())
            .filter(|&k| {
                let o = &w.agents[k];
                k != i
                    && o.alive
                    && o.species == Species::Person
                    && o.pos.z == me.z
                    && o.pos.dist(me) <= 30
                    && !crate::systems::hazards::indoors(w, o.pos)
                    && !w.creatures.shifted.iter().any(|x| x.0 == o.id)
            })
            .min_by_key(|&k| (w.agents[k].pos.dist(me), w.agents[k].id));
        let Some(j) = prey else {
            keep.push((id, until));
            continue;
        };
        let at = w.agents[j].pos;
        if at.dist(me) > 1 {
            if tick.is_multiple_of(20) && w.path_agent_to(i, at) {
                w.agents[i].activity = crate::mind::Activity::Wandering { to: at };
                w.agents[i].activity_ticks = 0;
            }
            keep.push((id, until));
            continue;
        }
        let mut r = crate::rng::stream(
            w.meta.seed,
            &[crate::rng::purpose::THREATS, 0xBEA5, id.0 as u64, tick],
        );
        let sev = r.gen_range(0.3..0.6);
        let part = [Part::ARM_L, Part::ARM_R, Part::LEG_L, Part::TORSO][r.gen_range(0..4)];
        crate::systems::health::wound(
            w,
            j,
            part,
            WoundKind::Cut,
            sev,
            0.15,
            "torn by a beast in the night",
            Some(id),
        );
        let (stood, felled) = stand_against(w, i, j, &mut r);
        let victim = w.agents[j].id;
        w.events.push(
            tick,
            Some(at),
            crate::events::Severity::Major,
            crate::events::EventKind::Attacked {
                animal: id,
                agent: victim,
                driven_off: !stood.is_empty(),
            },
        );
        if !stood.is_empty() {
            w.events.push(
                tick,
                Some(at),
                crate::events::Severity::Major,
                crate::events::EventKind::StoodAgainst {
                    beast: id,
                    by: stood,
                    felled,
                },
            );
        }
        // Spent for the night: off into the dark until dawn, and themselves again.
    }
    w.creatures.shifted = keep;
}
