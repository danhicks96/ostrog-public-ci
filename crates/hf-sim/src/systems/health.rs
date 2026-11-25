//! Wounds bleed, clot and heal; cuts can turn bad; illness grows from infection or spreads
//! through winter quarters and fades with rest; age brings ailments. Death here has a
//! cause from the body. Healing is faster in a bed and faster still when tended.

use rand::Rng;

use crate::agents::Species;
use crate::body::{Ailment, Illness, Part, Wound, WoundKind};
use crate::events::{EventKind, Severity};
use crate::mind::Activity;
use crate::rng::{purpose, stream};
use crate::time::{Calendar, Season, TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::world_state::World;

pub fn step(w: &mut World) {
    let tick = w.meta.tick;
    if tick % TICKS_PER_HOUR == 600 {
        hourly(w);
        forms(w);
    }
    if tick % TICKS_PER_DAY == TICKS_PER_HOUR * 5 + 600 {
        daily(w);
    }
    // In the morning, when the valley is up (#405, FIXLOG 2026-10-27): at half past five,
    // with the daily pass, the healers were asleep or elsewhere.
    if tick % TICKS_PER_DAY == TICKS_PER_HOUR * 10 + 600 {
        limbs(w);
    }
}

/// Inflict a wound and record it. `cause` is plain words for the feed. `by` is who struck the
/// blow, when known (a person's attacker, or the attacking beast's own agent id) — it is kept
/// on the `Wound` and on the victim's memory of it (#872: a wound used to be remembered only as
/// "something bad here", with no event and no attacker, because the memory was recorded before
/// the `Wounded` event existed and never looked at `by`).
#[allow(clippy::too_many_arguments)]
pub fn wound(
    w: &mut World,
    i: usize,
    part: Part,
    kind: WoundKind,
    severity: f32,
    bleeding: f32,
    cause: &str,
    by: Option<crate::ids::AgentId>,
) {
    let tick = w.meta.tick;
    // A body that is not a skeleton is not wounded as limbs are (#423).
    if let Some(f) = w.agents[i].body.form {
        formed_wound(w, i, f, kind, severity, cause);
        return;
    }
    // The harm lands on a part this body has (the review of C38-C44: every wound named a human
    // part, so a body without legs could break one). A part it lacks becomes one doing the
    // same work, chosen by the plan's wound weights from the agent's own stream.
    let plan = w.agents[i].species_def().body_plan();
    let part = if plan.has(part) {
        part
    } else {
        let role = crate::plan::biped()
            .parts
            .iter()
            .find(|d| d.id == part)
            .and_then(|d| d.roles.first().copied());
        let mut r = crate::rng::stream(
            w.meta.seed,
            &[
                crate::rng::purpose::AGENT_WORK,
                0x9A27,
                w.agents[i].id.0 as u64,
                tick,
                part.0 as u64,
            ],
        );
        match role {
            Some(role) if plan.parts.iter().any(|d| d.roles.contains(&role)) => {
                plan.random_part(&mut r, crate::plan::PartFilter::Role(role))
            }
            _ => plan.random_part(&mut r, crate::plan::PartFilter::Any),
        }
    };
    let a = &mut w.agents[i];
    let severity = severity.clamp(0.05, 1.0);
    let tissue = crate::body::Tissue::of(kind, severity, part);
    a.body.wounds.push(Wound {
        part,
        kind,
        severity,
        // Deep inside, it bleeds whatever the blow (C74).
        bleeding: if tissue == crate::body::Tissue::Organ {
            bleeding.max(0.4)
        } else {
            bleeding
        }
        .clamp(0.0, 1.0),
        infected: false,
        since: tick,
        by,
        tissue,
        treated: None,
        in_anger: false,
    });
    a.injured_until = a.injured_until.max(tick + TICKS_PER_HOUR * 6);
    let who = a.id;
    let at = a.pos;
    // #872: the event is pushed first so the memory can point at it, and the memory about the
    // place carries the event id instead of standing alone as a nameless hardship.
    let eid = w.events.push(
        tick,
        Some(at),
        if severity >= 0.5 {
            Severity::Major
        } else {
            Severity::Notable
        },
        EventKind::Wounded {
            agent: who,
            part: part.label().into(),
            kind: kind.label().into(),
            cause: cause.to_string(),
        },
    );
    let a = &mut w.agents[i];
    a.remember(
        tick,
        crate::mind::MemKind::Hardship,
        crate::mind::Subject::Place(at),
        -(0.3 + severity * 0.5),
        0.6 + severity * 0.3,
        Some(eid),
    );
    // A second, separate record about whoever struck the blow (#872): without it the victim's
    // memory of being hurt had no bearing on how they felt about their attacker.
    if let Some(striker) = by {
        a.remember(
            tick,
            crate::mind::MemKind::Hardship,
            crate::mind::Subject::Person(striker),
            -(0.3 + severity * 0.5),
            0.6 + severity * 0.3,
            Some(eid),
        );
    }
}

/// How much of a whole body that is not a skeleton a blow of full severity takes (#423): a
/// quarter of a swarm's members; two fifths of a soft body. A blow on a chain's segments cuts
/// one through at `SEVER` and two at `SEVER_TWO`; a lighter one glances off.
pub const SWARM_PER_BLOW: f32 = 0.25;
pub const SOFT_PER_BLOW: f32 = 0.4;
pub const SEVER: f32 = 0.3;
pub const SEVER_TWO: f32 = 0.75;

/// A harm on a body that is not a skeleton (#423). It lands on a part of its plan by the plan's
/// weights. A swarm loses members and a soft body some of its mass, by the blow's severity, and
/// neither bleeds; a chain loses segments to a blow that cuts deep enough, and a blow to its
/// head is a wound there as on any head, without bleeding. What it lost is on record.
fn formed_wound(
    w: &mut World,
    i: usize,
    f: crate::body::Form,
    kind: WoundKind,
    severity: f32,
    cause: &str,
) {
    use crate::plan::Kind;
    let tick = w.meta.tick;
    let severity = severity.clamp(0.05, 1.0);
    let id = w.agents[i].id;
    let at = w.agents[i].pos;
    // Several blows can land in one tick: what is left and the wounds so far tell them apart.
    let mut r = stream(
        w.meta.seed,
        &[
            purpose::AGENT_WORK,
            0x4231,
            id.0 as u64,
            tick,
            f.units as u64,
            w.agents[i].body.wounds.len() as u64,
        ],
    );
    let plan = f.kind.plan();
    let part = plan.random_part(&mut r, crate::plan::PartFilter::Any);
    let lost = match f.kind {
        Kind::Swarm => (severity * SWARM_PER_BLOW * f.whole as f32).ceil() as u16,
        Kind::Soft => ((severity * SOFT_PER_BLOW * f.whole as f32).round() as u16).max(1),
        Kind::Segmented if part == Part::SEGMENT => {
            u16::from(severity >= SEVER) + u16::from(severity >= SEVER_TWO)
        }
        _ => 0,
    }
    .min(f.units);
    let a = &mut w.agents[i];
    a.injured_until = a.injured_until.max(tick + TICKS_PER_HOUR * 6);
    if f.kind == Kind::Segmented && part == Part::HEAD {
        a.body.wounds.push(Wound {
            part,
            kind,
            severity,
            bleeding: 0.0,
            infected: false,
            since: tick,
            by: None,
            tissue: crate::body::Tissue::of(kind, severity, part),
            treated: None,
            in_anger: false,
        });
        w.events.push(
            tick,
            Some(at),
            Severity::Notable,
            EventKind::Wounded {
                agent: id,
                part: part.label().into(),
                kind: kind.label().into(),
                cause: cause.to_string(),
            },
        );
        return;
    }
    if lost == 0 {
        return;
    }
    let left = f.units - lost;
    if let Some(form) = a.body.form.as_mut() {
        form.units = left;
    }
    // A soft body that can split, cut deep enough that what the blow took could live by itself,
    // is cut in two: the piece lives on (#423, the split ability). Not past `SPLIT_CAP` of its
    // kind in the valley, so blows on blows do not fill it.
    let species = w.agents[i].species;
    let split = f.kind == Kind::Soft
        && plan.can(crate::plan::Ability::Split)
        && matches!(kind, WoundKind::Cut | WoundKind::Puncture)
        && lost >= f.least()
        && w.agents
            .iter()
            .filter(|o| o.alive && o.species == species)
            .count()
            < SPLIT_CAP;
    w.events.push(
        tick,
        Some(at),
        Severity::Notable,
        EventKind::Thinned {
            agent: id,
            part: part.label().into(),
            lost: lost as u32,
            left: left as u32,
            cause: cause.to_string(),
        },
    );
    if split {
        divide(w, i, lost);
    }
}

/// No more than this many of a kind of soft body come of splitting (#423).
pub const SPLIT_CAP: usize = 8;

/// The piece cut from the soft body `i` lives on beside it (#423): as old as it, as fed, whole at
/// the same size, so it grows back to it while fed.
fn divide(w: &mut World, i: usize, piece: u16) {
    let tick = w.meta.tick;
    let a = &w.agents[i];
    let (parent, species, at, born, hunger) = (a.id, a.species, a.pos, a.born, a.needs.hunger);
    let Some(f) = a.body.form else { return };
    let to = [(0, 1), (1, 0), (0, -1), (-1, 0)]
        .iter()
        .map(|(dx, dy)| at.offset(*dx, *dy, 0))
        .find(|q| w.grid.walkable(*q))
        .unwrap_or(at);
    let id = w.spawn_agent(species, to);
    let Some(j) = w.agents.iter().rposition(|o| o.id == id) else {
        return;
    };
    let b = &mut w.agents[j];
    b.born = born;
    b.needs.hunger = hunger;
    b.body.form = Some(crate::body::Form {
        kind: f.kind,
        units: piece,
        whole: f.whole,
    });
    w.events.push(
        tick,
        Some(at),
        Severity::Notable,
        EventKind::Divided {
            agent: parent,
            into: id,
        },
    );
}

/// Fed enough to grow back what it lost (#423).
pub const FED: f32 = 0.5;
/// What a unit grown back costs of a full belly.
pub const REGROW_COST: f32 = 0.02;

/// A swarm or soft body this empty is starving: it loses itself a piece at a time (#423).
pub const STARVED: f32 = 0.02;

/// Hourly, the beasts whose bodies are not skeletons (#423). Their wear is what their kind
/// suffers, not a sickness of limbs and organs: a starving swarm loses a member every four
/// hours, and a starving soft body two hundredths of itself an hour; a soft body out of the
/// damp (above ground, with no water within two tiles) dries by a hundredth an hour. One left
/// with too little of itself to live as one creature dies of it, or of the blows that did it.
/// One fed that can regrow grows back what it lost.
fn forms(w: &mut World) {
    use crate::plan::Kind;
    let tick = w.meta.tick;
    let mut ends: Vec<(crate::ids::AgentId, String)> = Vec::new();
    for i in 0..w.agents.len() {
        let a = &w.agents[i];
        let Some(mut f) = a.body.form else { continue };
        if !a.alive {
            continue;
        }
        let starving = a.needs.hunger <= STARVED;
        let mut lost = 0u16;
        let mut why = "";
        match f.kind {
            Kind::Swarm if starving && (tick / TICKS_PER_HOUR).is_multiple_of(4) => {
                lost = 1;
                why = "starving";
            }
            Kind::Soft => {
                if starving {
                    lost += 2;
                    why = "starving";
                }
                if dry(w, a.pos) {
                    lost += 1;
                    why = if starving { "starving" } else { "drying out" };
                }
            }
            _ => {}
        }
        // Fed, a body that can regrow grows back what it lost (#423): a swarm breeds back a
        // member every six hours, a soft body a hundredth an hour, a chain a segment a day,
        // each at a fiftieth of a full belly.
        let hour = tick / TICKS_PER_HOUR;
        let regrow = lost == 0
            && f.units < f.whole
            && !f.spent()
            && a.needs.hunger >= FED
            && f.kind.plan().can(crate::plan::Ability::Regrow)
            && match f.kind {
                Kind::Swarm => hour.is_multiple_of(6),
                Kind::Soft => true,
                _ => hour.is_multiple_of(24),
            };
        if regrow {
            f.units += 1;
            let a = &mut w.agents[i];
            a.needs.hunger = (a.needs.hunger - REGROW_COST).max(0.0);
            a.body.form = Some(f);
            continue;
        }
        let a = &w.agents[i];
        let (id, at) = (a.id, a.pos);
        let lost = lost.min(f.units);
        if lost > 0 {
            f.units -= lost;
            w.agents[i].body.form = Some(f);
            w.events.push(
                tick,
                Some(at),
                Severity::Trace,
                EventKind::Thinned {
                    agent: id,
                    part: f.kind.plan().parts[0].id.label().into(),
                    lost: lost as u32,
                    left: f.units as u32,
                    cause: why.to_string(),
                },
            );
        }
        if f.spent() {
            let cause = if lost > 0 {
                format!("{}, {why}", f.end())
            } else {
                f.end().to_string()
            };
            ends.push((id, cause));
        }
    }
    for (id, cause) in ends {
        crate::systems::life::die(w, id, cause);
    }
}

/// Above ground, with no water within two tiles: a soft body dries there (#423).
pub fn dry(w: &World, p: crate::ids::TilePos) -> bool {
    if crate::underground::below(w, p) {
        return false;
    }
    for dy in -2..=2 {
        for dx in -2..=2 {
            for dz in -1..=0 {
                let q = p.offset(dx, dy, dz);
                if w.grid.in_bounds(q) && w.grid.get(q).is_water() {
                    return false;
                }
            }
        }
    }
    true
}

fn in_bed(a: &crate::agents::Agent) -> bool {
    matches!(a.activity, Activity::Sleeping { bed: Some(_), .. })
}

fn hourly(w: &mut World) {
    let tick = w.meta.tick;
    let seed = w.meta.seed;
    let n = w.agents.len();
    let mut dead: Vec<(usize, &'static str)> = Vec::new();
    for i in 0..n {
        if !w.agents[i].alive || w.agents[i].species != Species::Person {
            continue;
        }
        if w.agents[i].body.wounds.is_empty() && w.agents[i].body.illness.is_none() {
            // Blood comes back slowly once whole.
            let a = &mut w.agents[i];
            a.blood_up(0.01);
            continue;
        }
        let resting = in_bed(&w.agents[i]);
        let tended = w.agents[i].tended_until > tick;
        let mut r = stream(
            seed,
            &[purpose::AGENT_BIRTH, 0x4EA1, w.agents[i].id.0 as u64, tick],
        );
        let weak_lungs = w.agents[i].body.ailments.contains(&Ailment::WeakLungs);
        let mut healed: Vec<Wound> = Vec::new();
        {
            let a = &mut w.agents[i];
            let heal = 0.012 * if resting { 2.0 } else { 1.0 } * if tended { 1.5 } else { 1.0 };
            let mut blood_loss = 0.0;
            for wd in a.body.wounds.iter_mut() {
                blood_loss += wd.bleeding * 0.02;
                let stitched = wd.treated == Some(crate::body::Treatment::Stitched);
                if wd.tissue == crate::body::Tissue::Organ && !stitched {
                    // Bleeding inside does not clot by itself (C74).
                } else if wd.kind != WoundKind::Break {
                    wd.bleeding = (wd.bleeding - 0.25).max(0.0);
                } else {
                    wd.bleeding = (wd.bleeding - 0.1).max(0.0);
                }
                // An infected wound barely heals until it is cleaned; a deep one heals slower
                // than a graze, and treatment hastens it (C74).
                let treated = if wd.treated.is_some() { 1.4 } else { 1.0 };
                let rate = if wd.infected { heal * 0.1 } else { heal } * wd.tissue.heal() * treated;
                wd.severity -= rate;
            }
            a.body.blood = (a.body.blood - blood_loss).max(0.0);
            for wd in a.body.wounds.iter().filter(|w| w.severity <= 0.0) {
                healed.push(*wd);
            }
            a.body.wounds.retain(|w| w.severity > 0.0);
            // Hurt people tire faster.
            if a.body.hurt() {
                a.needs.rest = (a.needs.rest - 0.015).max(0.0);
            }
        }
        for wd in healed {
            let a = &mut w.agents[i];
            if wd.kind == WoundKind::Break || wd.infected {
                a.body.scars = a.body.scars.saturating_add(1);
            }
            // A break nobody set heals crooked: the part is scarred for good (C74).
            if wd.kind == WoundKind::Break && wd.treated != Some(crate::body::Treatment::Splinted) {
                a.body
                    .parts
                    .entry(wd.part)
                    .or_insert(crate::body::PartState::Scarred);
            }
            let who = a.id;
            let at = a.pos;
            w.events.push(
                tick,
                Some(at),
                Severity::Minor,
                EventKind::Healed {
                    agent: who,
                    part: wd.part.label().into(),
                },
            );
        }
        // Illness runs its course hour by hour.
        let mut fell_ill = false;
        let mut recovered = false;
        let mut recovered_from = "";
        {
            let a = &mut w.agents[i];
            let infected = a.body.wounds.iter().any(|w| w.infected);
            if infected && a.body.illness.is_none() {
                a.body.illness = Some(Illness {
                    severity: 0.3,
                    since: tick,
                    kind: crate::body::IllnessKind::WoundRot,
                    treated: None,
                });
                fell_ill = true;
            }
            if let Some(ill) = a.body.illness.as_mut() {
                // Each sickness its own course (#328); untended infection outruns rest; a
                // clean wound and a bed turn it round.
                let hours = tick.saturating_sub(ill.since) / crate::time::TICKS_PER_HOUR;
                let mut delta = ill.kind.course(hours, infected);
                // The right care turns it; a wrong one does nothing (#328).
                if ill.treated == Some(true) {
                    delta -= 0.006;
                }
                if resting {
                    delta -= 0.004;
                }
                if tended {
                    delta -= 0.005;
                }
                if weak_lungs && delta < 0.0 {
                    delta *= 0.5;
                }
                if a.needs.hunger < 0.2 {
                    delta += 0.003;
                }
                ill.severity += delta;
                // The wasting never wholly lifts (#328).
                if ill.kind == crate::body::IllnessKind::Wasting {
                    ill.severity = ill.severity.max(0.25);
                }
                if ill.severity <= 0.0 {
                    let k = ill.kind;
                    a.body.illness = None;
                    recovered = true;
                    if k.leaves_immune() && !a.immune.contains(&k) {
                        a.immune.push(k);
                    }
                    recovered_from = k.name();
                }
            }
        }
        let (who, at) = (w.agents[i].id, w.agents[i].pos);
        if fell_ill {
            w.events.push(
                tick,
                Some(at),
                Severity::Notable,
                EventKind::FellIll {
                    agent: who,
                    what: crate::body::IllnessKind::WoundRot.name().into(),
                },
            );
        }
        if recovered {
            w.events.push(
                tick,
                Some(at),
                Severity::Notable,
                EventKind::Recovered {
                    agent: who,
                    what: recovered_from.into(),
                },
            );
        }
        // Death from the body.
        let b = &w.agents[i].body;
        if b.blood < 0.3 {
            dead.push((i, "bled out"));
        } else if b.ill() >= 1.0 {
            // A look-alike eaten in error kills as a poisoning, not a fever (#286).
            let poisoned = b
                .illness
                .is_some_and(|ill| ill.kind == crate::body::IllnessKind::Poisoned);
            dead.push((i, if poisoned { "poisoning" } else { "fever" }));
        }
        let _ = r.gen::<f32>();
    }
    for (i, cause) in dead {
        let id = w.agents[i].id;
        // #404 (2026-10-27 fix-404-feuds-in-play): a death from a wound struck in a fight is a
        // killing at the striker's hand, taken up while the dead are still among their kin.
        if let Some(by) = killed_by(&w.agents[i], cause) {
            crate::justice::struck_down(w, by, id, None);
        }
        crate::systems::life::die(w, id, cause.into());
    }
}

/// Who struck the wound `a` is dying of, if it was struck in a fight between people of the
/// valley (#404): bleeding out, the wound that bleeds most; a fever, a wound gone bad.
pub fn killed_by(a: &crate::agents::Agent, cause: &str) -> Option<crate::ids::AgentId> {
    let wounds = a
        .body
        .wounds
        .iter()
        .filter(|wd| wd.in_anger && wd.by.is_some());
    let wd = match cause {
        "bled out" => wounds
            .filter(|wd| wd.bleeding > 0.0)
            .max_by(|x, y| x.bleeding.total_cmp(&y.bleeding)),
        "fever"
            if a.body.illness.as_ref().map(|i| i.kind)
                == Some(crate::body::IllnessKind::WoundRot) =>
        {
            wounds
                .filter(|wd| wd.infected)
                .max_by(|x, y| x.severity.total_cmp(&y.severity))
        }
        _ => None,
    };
    wd.and_then(|wd| wd.by)
}

/// #284: a limb whose wound has gone bad, with the rot deep in them, is taken by one who knows
/// healing to save them; a limb lost is in time replaced by a made one, by one who knows
/// carpentry, from a plank or a log.
pub fn limbs(w: &mut World) {
    use crate::body::{Part, PartState};
    let tick = w.meta.tick;
    let limbs = [Part::ARM_L, Part::ARM_R, Part::LEG_L, Part::LEG_R];
    for i in 0..w.agents.len() {
        let a = &w.agents[i];
        if !a.alive || a.species != Species::Person {
            continue;
        }
        let Some(cid) = a.community else { continue };
        let at = a.pos;
        let who = a.id;
        // A healer of their community who is awake comes to them, wherever they are (#405:
        // only one within twelve tiles on the same level would do, and of eight mornings in
        // four 200-day openings when a limb had rotted that far, one had such a healer to hand).
        let healer = |w: &World| {
            (0..w.agents.len()).find(|&k| {
                let h = &w.agents[k];
                k != i
                    && h.alive
                    && h.community == Some(cid)
                    && !h.activity.is_asleep()
                    && (h.techniques.knows(crate::mind::Technique::HerbLore)
                        || h.skills.level(crate::mind::Skill::Cooking) >= 4.0)
            })
        };
        // Taking a limb.
        let rotten = a
            .body
            .wounds
            .iter()
            .find(|wd| wd.infected && limbs.contains(&wd.part) && wd.severity >= 0.4)
            .map(|wd| (wd.part, wd.by.filter(|_| wd.in_anger)));
        if let (Some((part, struck_by)), true) = (rotten, a.body.ill() >= 0.6) {
            if let Some(h) = healer(w) {
                let by = w.agents[h].id;
                let b = &mut w.agents[i].body;
                b.wounds.retain(|wd| wd.part != part);
                b.parts.insert(part, PartState::Lost);
                b.blood = (b.blood - 0.15).max(0.35);
                if let Some(ill) = b.illness.as_mut() {
                    ill.severity = (ill.severity - 0.45).max(0.02);
                }
                let ev = w.events.push(
                    tick,
                    Some(at),
                    Severity::Major,
                    EventKind::LimbTaken {
                        agent: who,
                        by,
                        part: part.label().into(),
                    },
                );
                part_story(
                    &mut w.agents[i],
                    part,
                    tick,
                    crate::partlog::Change::Lost,
                    ev,
                    "taken to save them",
                );
                // #404 (2026-10-27 fix-404-feuds-in-play): lost to a wound struck in a fight.
                if let Some(s) = struck_by {
                    crate::justice::struck_down(w, s, who, Some(part.label()));
                }
                continue;
            }
        }
        // Replacing one.
        let lost = w.agents[i]
            .body
            .parts
            .iter()
            .find(|(_, st)| matches!(st, PartState::Lost))
            .map(|(p, _)| *p);
        let Some(part) = lost else { continue };
        let Some(c) = (0..w.agents.len()).find(|&k| {
            let h = &w.agents[k];
            k != i
                && h.alive
                && h.community == Some(cid)
                && h.techniques.knows(crate::mind::Technique::Carpentry)
        }) else {
            continue;
        };
        let wood = w
            .items
            .iter()
            .find(|it| {
                matches!(
                    it.kind,
                    crate::items::ItemKind::Plank | crate::items::ItemKind::Log
                ) && matches!(it.place, crate::items::ItemPlace::Ground(_))
            })
            .map(|it| it.id);
        let Some(wood) = wood else { continue };
        w.consume_item(wood, 1, "a made limb");
        let what = if part == Part::LEG_L || part == Part::LEG_R {
            "a wooden leg"
        } else {
            "a wooden hand"
        };
        w.agents[i]
            .body
            .parts
            .insert(part, PartState::Replaced(what.to_string()));
        let by = w.agents[c].id;
        let ev = w.events.push(
            tick,
            Some(at),
            Severity::Notable,
            EventKind::LimbMade {
                agent: who,
                by,
                what: what.into(),
            },
        );
        part_story(
            &mut w.agents[i],
            part,
            tick,
            crate::partlog::Change::Replaced,
            ev,
            what,
        );
    }
}

/// Keep the story of a lost or replaced arm on its hand (partlog.rs), pointing at the chronicle
/// event. Only arms for now: the foot and the rest of the tree follow.
fn part_story(
    a: &mut crate::agents::Agent,
    part: Part,
    tick: u64,
    change: crate::partlog::Change,
    ev: crate::ids::EventId,
    note: &str,
) {
    use crate::partlog::{Entry, PartKey, Region, Side, Spot};
    let side = if part == Part::ARM_L {
        Side::Left
    } else if part == Part::ARM_R {
        Side::Right
    } else {
        return;
    };
    a.part_log.record(Entry {
        part: PartKey::new(side, Region::Hand, Spot::Whole),
        tick,
        change,
        event: Some(ev),
        note: note.to_string(),
    });
}

fn daily(w: &mut World) {
    let tick = w.meta.tick;
    let seed = w.meta.seed;
    let season = Calendar::from_tick(tick).season;
    let cold = matches!(season, Season::Winter | Season::Autumn);
    let n = w.agents.len();
    // Wounds turned bad with no healer left (cascade.rs, #319).
    let mut festered: Vec<(
        crate::ids::AgentId,
        crate::ids::TilePos,
        crate::ids::EventId,
    )> = Vec::new();
    // Infection of untreated cuts.
    for i in 0..n {
        if !w.agents[i].alive || w.agents[i].species != Species::Person {
            continue;
        }
        let tended = w.agents[i].tended_until > tick;
        let mut r = stream(
            seed,
            &[
                purpose::AGENT_BIRTH,
                0x4EA2,
                w.agents[i].id.0 as u64,
                tick / TICKS_PER_DAY,
            ],
        );
        let lost_lore = w.agents[i]
            .community
            .and_then(|c| w.cascades.healerless.get(&c))
            .copied();
        let a = &mut w.agents[i];
        for wd in a.body.wounds.iter_mut() {
            if matches!(wd.kind, WoundKind::Cut | WoundKind::Puncture)
                && !wd.infected
                && wd.severity >= 0.3
            {
                // A stitched wound stays clean; a deep one festers more (C74).
                let chance = if wd.treated == Some(crate::body::Treatment::Stitched) {
                    0.01
                } else if tended {
                    0.04
                } else if wd.tissue >= crate::body::Tissue::Muscle {
                    0.2
                } else {
                    0.15
                };
                if r.gen::<f32>() < chance {
                    wd.infected = true;
                    // Undressed, where the last healer died: the cascade's next step.
                    if let (Some(row), None) = (lost_lore, wd.treated) {
                        if !festered.iter().any(|f| f.0 == a.id) {
                            festered.push((a.id, a.pos, row));
                        }
                    }
                }
            }
        }
        // Ailments of age.
        let years = a.age_years(tick);
        if years >= a.ailing_years().0 && r.gen::<f32>() < 0.06 && a.body.ailments.len() < 2 {
            let pick = [Ailment::BadBack, Ailment::WeakLungs, Ailment::DimEyes][r.gen_range(0..3)];
            if !a.body.ailments.contains(&pick) {
                a.body.ailments.push(pick);
                let who = a.id;
                let at = a.pos;
                w.events.push(
                    tick,
                    Some(at),
                    Severity::Notable,
                    EventKind::Ailing {
                        agent: who,
                        what: pick.label().into(),
                    },
                );
            }
        }
    }
    for (agent, at, cause) in festered {
        w.events.push(
            tick,
            Some(at),
            Severity::Notable,
            EventKind::WoundFestered { agent, cause },
        );
    }
    // An ill wind (#278) brings sickness in any season, and anyone may catch it.
    let ill_wind = crate::spells::now(w) == Some(crate::spells::SpellKind::IllWind);
    // Sicknesses caught (#328): a cough or a fever from the sick close by (the cough in the
    // cold months only); the winter cough by the weak in winter; the flux near the dead left
    // lying a day; the wasting in the old and the long-hungry. None that one has had and is
    // guarded against.
    use crate::body::IllnessKind as K;
    let sick: Vec<(crate::ids::TilePos, K)> = w
        .agents
        .iter()
        .filter(|a| a.alive && a.body.ill() > 0.2)
        .filter_map(|a| a.body.illness.map(|i| (a.pos, i.kind)))
        .filter(|(_, k)| matches!(k, K::WinterCough | K::Fever))
        .collect();
    let lying: Vec<crate::ids::TilePos> = w
        .rites_due
        .iter()
        .filter(|r| !r.away && tick.saturating_sub(r.since) >= TICKS_PER_DAY)
        .filter_map(|r| w.agents.iter().find(|a| a.id == r.dead).map(|a| a.pos))
        .collect();
    // A deity may lean it (deity.rs).
    let sickness = w.lean(crate::deity::Lever::Sickness, crate::deity::Scope::Valley);
    let mut newly: Vec<(usize, K)> = Vec::new();
    for i in 0..n {
        let a = &w.agents[i];
        if !a.alive || a.species != Species::Person || a.body.illness.is_some() {
            continue;
        }
        let mut r = stream(
            seed,
            &[
                purpose::AGENT_BIRTH,
                0x4EA3,
                a.id.0 as u64,
                tick / TICKS_PER_DAY,
            ],
        );
        let roll: f32 = r.gen();
        // Weak from hunger in a failed winter (cascade.rs, #319): what goes round takes them
        // twice as readily.
        let weakened = crate::cascade::weak_now(w, a);
        let sickness = if weakened { sickness * 2.0 } else { sickness };
        let near = sick
            .iter()
            .filter(|(p, k)| {
                p.dist(a.pos) <= 8 && (cold || ill_wind || *k == K::Fever) && !a.immune.contains(k)
            })
            .min_by_key(|(p, _)| p.dist(a.pos))
            .map(|x| x.1);
        let weak = weakened || a.needs.hunger < 0.25 || a.age_years(tick) >= a.ailing_years().1;
        let caught = if let Some(k) = near {
            let c = if in_bed(a) { 0.35 } else { 0.2 };
            (roll < c * sickness).then_some(k)
        } else if lying.iter().any(|p| p.dist(a.pos) <= 6) {
            (roll < 0.05 * sickness).then_some(K::Flux)
        } else if weak && cold && season == Season::Winter {
            (roll < 0.015 * sickness).then_some(K::WinterCough)
        } else if a.age_years(tick) >= a.ailing_years().1 && a.needs.hunger < 0.15 {
            (roll < 0.01 * sickness).then_some(K::Wasting)
        } else if ill_wind {
            // An ill wind (#278) brings a fever to anyone, in any season.
            (roll < 0.02 * sickness).then_some(K::Fever)
        } else {
            None
        };
        if let Some(k) = caught {
            newly.push((i, k));
        }
    }
    for (i, k) in newly {
        catch(w, i, k);
        // The weak who caught it: the lean winter's next step, holding the weakening's row.
        let id = w.agents[i].id;
        if let (Some(&cause), true) = (
            w.cascades.weakened.get(&id),
            w.agents[i]
                .body
                .illness
                .is_some_and(|ill| ill.kind == k && ill.since == tick)
                && crate::cascade::weak_now(w, &w.agents[i]),
        ) {
            let at = w.agents[i].pos;
            w.events.push(
                tick,
                Some(at),
                Severity::Notable,
                EventKind::SickenedWeak {
                    agent: id,
                    what: k.name().to_string(),
                    cause,
                },
            );
        }
    }
}

/// `i` takes sickness `k` (#328), unless guarded against it.
pub fn catch(w: &mut World, i: usize, k: crate::body::IllnessKind) {
    let severity = if k == crate::body::IllnessKind::Wasting {
        0.3
    } else {
        0.35
    };
    catch_at(w, i, k, severity, k.name());
}

/// `i` takes sickness `k` at `severity`, unless guarded against it or already ill.
pub fn catch_at(w: &mut World, i: usize, k: crate::body::IllnessKind, severity: f32, what: &str) {
    let tick = w.meta.tick;
    let a = &mut w.agents[i];
    if a.body.illness.is_some() || a.immune.contains(&k) {
        return;
    }
    a.body.illness = Some(Illness {
        severity,
        since: tick,
        kind: k,
        treated: None,
    });
    let who = a.id;
    let at = a.pos;
    w.events.push(
        tick,
        Some(at),
        Severity::Notable,
        EventKind::FellIll {
            agent: who,
            what: what.into(),
        },
    );
}
