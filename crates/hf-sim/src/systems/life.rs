//! The arc of a life: pairing, births, growing up, aging, and death (old age or starvation).
//! Runs daily at 05:00 per person (striped), with the starvation clock kept per tick stripe.

use rand::Rng;

use crate::agents::{LifeStage, Species};
use crate::events::{EventKind, Severity};
use crate::ids::AgentId;
use crate::mind::{Activity, MemKind, Subject};
use crate::rng::{purpose, stream};
use crate::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use crate::world_state::World;

pub fn step(w: &mut World) {
    step_with_birth_observer(w, &mut |_, _, _| {});
}

/// The ordinary life step, with a read-only report for each paired couple's birth gate.
/// Used by the birth census at this pipeline stage so pairing, prior births and the food
/// reading are exactly the ones the simulation uses. The observer cannot borrow the world.
pub fn step_with_birth_observer(
    w: &mut World,
    observer: &mut impl FnMut(AgentId, AgentId, BirthGate),
) {
    let tick = w.meta.tick;
    starvation(w);
    thirst(w);
    if tick % TICKS_PER_DAY != TICKS_PER_HOUR * 5 {
        return;
    }
    pairing(w);
    births(w, observer);
    aging(w);
    clothes(w);
}

/// Clothes wear a little every day; when they are rags, a set from the store replaces them.
/// A set with a story (legend.rs) is not used up when it is put on: it is worn (held by its
/// wearer, `ItemPlace::Held`), wears as they do, and is kept as rags while there is nothing to
/// replace it. It ends when a new set replaces it, and the feed says what it was.
fn clothes(w: &mut World) {
    use crate::items::{ItemKind, ItemPlace};
    let tick = w.meta.tick;
    let n = w.agents.len();
    for i in 0..n {
        if !w.agents[i].alive || w.agents[i].species != Species::Person {
            continue;
        }
        let who = w.agents[i].id;
        w.agents[i].clothes_wear = (w.agents[i].clothes_wear - 0.01).max(0.0);
        let worn = w
            .items
            .iter()
            .position(|it| it.kind == ItemKind::Clothes && it.place == ItemPlace::Held(who));
        if let Some(s) = worn.and_then(|ix| w.items[ix].story.as_mut()) {
            s.wear = w.agents[i].clothes_wear;
        }
        if w.agents[i].clothes_wear > 0.25 {
            continue;
        }
        let home = w.agents[i]
            .community
            .and_then(|c| w.communities.iter().find(|x| x.id == c).map(|x| x.home))
            .unwrap_or(w.meta.founding_site);
        let set = w
            .items
            .iter()
            .filter(|it| {
                it.kind == ItemKind::Clothes
                    && it.count > 0
                    // 2026-10-30, checkpoint #467: a stored story keeps its real wear; do not
                    // install rags that cold_step treats as unclothed (wear <= 0.2).
                    && (it.count != 1 || it.story.as_ref().is_none_or(|s| s.wear > 0.2))
                    && matches!(it.place, ItemPlace::Ground(p) if p.dist(home) <= 45)
                    // Not a set someone holds a lease on (#177).
                    && w.reservations
                        .is_free_for(crate::work::ResKey::Item(it.id), who)
            })
            .max_by(|a, b| {
                a.quality
                    .partial_cmp(&b.quality)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|it| (it.id, it.quality));
        let Some((id, q)) = set else { continue };
        // Leather and fur keep the cold out better than woven cloth (C34).
        let furred = w.items.iter().find(|it| it.id == id).is_some_and(|it| {
            w.grid.materials.get(it.material).kind == crate::world::material::MaterialKind::Hide
        });
        // Clothes dyed at the loom wear their dye's colour (#286).
        let dyed = w
            .items
            .iter()
            .find(|it| it.id == id)
            .and_then(|it| dyed_colour(it.material));
        // The set they wore, if it had a story, is worn through: what it was, for the feed.
        let mut old = None;
        if let Some(ix) = worn {
            let it = w.items[ix].clone();
            let s = it.story.clone().unwrap_or_default();
            w.consume_item(it.id, 1, "worn out");
            if s.cherished || s.name.is_some() {
                w.agents[i]
                    .thoughts
                    .add(crate::mind::ThoughtKind::LostTreasure, tick, None);
            }
            old = Some((crate::legend::worn_through(w, &it, &s), s));
        }
        // The new set: one with a story is kept on them, one without is used up as before.
        let wear = match w.items.iter().position(|it| it.id == id) {
            Some(ix) if w.items[ix].story.is_some() && w.items[ix].count == 1 => {
                let name = w.agents[i].name.clone();
                let it = &mut w.items[ix];
                it.place = ItemPlace::Held(who);
                let s = it.story.as_mut().unwrap();
                if s.owners.last().map(|o| o.0) != Some(who) {
                    s.owners.push((who, name, tick));
                    s.keeper_uses = 0;
                    s.cherished = false;
                }
                s.wear.clamp(0.0, 1.0)
            }
            _ => {
                w.consume_item(id, 1, "worn");
                1.0
            }
        };
        let a = &mut w.agents[i];
        a.clothes_wear = wear;
        a.furred = furred;
        // In u32: as a u8 add this overflowed for ids from 245 (#161).
        a.appearance.clothing_colour = dyed.unwrap_or((((q * 11.0) as u32 + a.id.0) % 12) as u8);
        let notable = old
            .as_ref()
            .is_some_and(|(_, s)| s.cherished || s.name.is_some() || !s.origin.is_empty());
        let (what, history) = old.map(|(x, _)| x).unwrap_or_default();
        w.events.push(
            tick,
            Some(w.agents[i].pos),
            if notable {
                Severity::Notable
            } else {
                Severity::Minor
            },
            EventKind::NewClothes {
                agent: who,
                tier: crate::items::ItemKind::tier(q),
                worn_through: what,
                history,
            },
        );
    }
}

/// Hunger at zero for three days kills. Counted on the needs stripe (every 20 ticks).
fn starvation(w: &mut World) {
    let tick = w.meta.tick;
    if !tick.is_multiple_of(20) {
        return;
    }
    let mut dead = Vec::new();
    for a in w.agents.iter_mut() {
        if !a.alive || a.species != Species::Person {
            continue;
        }
        if a.needs.hunger <= 0.0 {
            a.starving_ticks += 20;
            // Constitution differs: the frail go in two days, the hardy last six, so a
            // famine thins a settlement instead of erasing it on one day.
            let days = 2.0 + a.genome.constitution() * 4.0;
            if a.starving_ticks as f32 >= TICKS_PER_DAY as f32 * days {
                dead.push((a.id, "starvation".to_string()));
            }
        } else {
            a.starving_ticks = 0;
        }
    }
    for (id, cause) in dead {
        die(w, id, cause);
    }
}

/// Parched for a day and a half kills (#433). Counted on the needs stripe (every 20 ticks).
/// An hour at empty is on record (`Parched`) and remembered as a hardship; a lapse that ends
/// in a drink within the hour leaves nothing.
fn thirst(w: &mut World) {
    let tick = w.meta.tick;
    if !tick.is_multiple_of(20) {
        return;
    }
    let mut dead = Vec::new();
    let mut parched = Vec::new();
    for a in w.agents.iter_mut() {
        if !a.alive || a.species != Species::Person {
            continue;
        }
        if a.needs.thirst <= 0.0 {
            a.needs.parched_ticks += 20;
            if a.needs.parched_ticks as u64 == TICKS_PER_HOUR {
                parched.push((a.id, a.pos));
            }
            if a.needs.parched_ticks as f32
                >= TICKS_PER_DAY as f32 * crate::mind::needs::PARCHED_DAYS
            {
                dead.push(a.id);
            }
        } else {
            a.needs.parched_ticks = 0;
        }
    }
    for (agent, pos) in parched {
        let eid = w.events.push(
            tick,
            Some(pos),
            crate::events::Severity::Major,
            crate::events::EventKind::Parched { agent },
        );
        if let Some(a) = w.agents.iter_mut().find(|a| a.id == agent) {
            a.remember(
                tick,
                crate::mind::MemKind::Hardship,
                crate::mind::Subject::Place(pos),
                -0.8,
                0.8,
                Some(eid),
            );
        }
    }
    for id in dead {
        die(w, id, "thirst".to_string());
    }
}

fn pairing(w: &mut World) {
    let tick = w.meta.tick;
    let adults: Vec<AgentId> = w
        .agents
        .iter()
        .filter(|a| {
            a.alive
                && a.species == Species::Person
                && a.stage(tick) == LifeStage::Adult
                && a.partner.is_none()
                // Nobody takes a new partner while the grief is fresh.
                && !a.thoughts.has(crate::mind::ThoughtKind::LostSomeone)
        })
        .map(|a| a.id)
        .collect();
    // Lookups, not scans: with hundreds of adults the old all-against-all search (each pair
    // looked up by walking the whole list) took tens of milliseconds once a day. Candidates
    // are only those they already know well; relationships iterate by id, as `adults` does.
    let index: std::collections::BTreeMap<AgentId, usize> = w
        .agents
        .iter()
        .enumerate()
        .map(|(k, a)| (a.id, k))
        .collect();
    let single: std::collections::BTreeSet<AgentId> = adults.iter().copied().collect();
    let mut taken: std::collections::BTreeSet<AgentId> = std::collections::BTreeSet::new();
    for &x in &adults {
        if taken.contains(&x) {
            continue;
        }
        let ax = &w.agents[index[&x]];
        // The adult they like most, who likes them back, in the same community, not kin.
        let best = ax
            .relationships
            .keys()
            .copied()
            .filter(|y| *y != x && single.contains(y) && !taken.contains(y))
            .filter(|&y| {
                let ay = &w.agents[index[&y]];
                ay.community == ax.community
                    && ax.mother != Some(y)
                    && ax.father != Some(y)
                    && ay.mother != Some(x)
                    && ay.father != Some(x)
                    && !(ax.mother.is_some() && ax.mother == ay.mother)
                    && ax.opinion_of(y) > 0.15
                    && ay.opinion_of(x) > 0.15
                    // Across kinds only when neither holds other kinds in dread (P9).
                    && (ax.people == ay.people
                        || (ax.beliefs.lean(crate::mind::beliefs::Topic::OtherRaces) > -0.2
                            && ay.beliefs.lean(crate::mind::beliefs::Topic::OtherRaces) > -0.2))
                    && ax
                        .relationships
                        .get(&y)
                        .map(|r| r.familiarity)
                        .unwrap_or(0.0)
                        > 0.2
                    && ay
                        .relationships
                        .get(&x)
                        .map(|r| r.familiarity)
                        .unwrap_or(0.0)
                        > 0.2
            })
            .max_by(|&p, &q| {
                ax.opinion_of(p)
                    .partial_cmp(&ax.opinion_of(q))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        if let Some(y) = best {
            taken.insert(x);
            taken.insert(y);
            let pos = ax.pos;
            for (a, b) in [(x, y), (y, x)] {
                if let Some(agent) = w.agents.iter_mut().find(|z| z.id == a) {
                    agent.partner = Some(b);
                    agent.relate(b, 0.2, 0.2, tick);
                    agent.met(b, tick);
                    agent.remember(tick, MemKind::Company, Subject::Person(b), 0.9, 0.9, None);
                }
            }
            w.events.push(
                tick,
                Some(pos),
                Severity::Major,
                EventKind::Paired { a: x, b: y },
            );
            give_ring(w, x, y);
        }
    }
}

/// A ring from the store, if the jeweller has made one, given by one of a new pair to the
/// other (C34): the giver is the one who thinks more of the other.
pub fn give_ring(w: &mut World, x: AgentId, y: AgentId) {
    use crate::items::{ItemKind, ItemPlace};
    let tick = w.meta.tick;
    let (Some(xi), Some(yi)) = (
        w.agents.iter().position(|a| a.id == x),
        w.agents.iter().position(|a| a.id == y),
    ) else {
        return;
    };
    let home = w.agents[xi]
        .community
        .and_then(|c| w.communities.iter().find(|k| k.id == c).map(|k| k.home))
        .unwrap_or(w.meta.founding_site);
    let Some(id) = w
        .items
        .iter()
        .filter(|it| {
            it.kind == ItemKind::Jewel
                && matches!(it.place, ItemPlace::Ground(p) if p.dist(home) <= 45)
                && w.reservations
                    .is_free_for(crate::work::ResKey::Item(it.id), x)
        })
        .max_by(|a, b| a.quality.total_cmp(&b.quality).then(b.id.cmp(&a.id)))
        .map(|it| it.id)
    else {
        return;
    };
    let (gi, ri) = if w.agents[xi].opinion_of(y) >= w.agents[yi].opinion_of(x) {
        (xi, yi)
    } else {
        (yi, xi)
    };
    let (giver, to) = (w.agents[gi].id, w.agents[ri].id);
    crate::legend::ensure(w, id, None, "of unknown make");
    let to_name = w.agents[ri].name.clone();
    let mut what = String::new();
    if let Some(it) = w.items.iter_mut().find(|it| it.id == id) {
        it.place = ItemPlace::Held(to);
        let noun = crate::legend::noun(it.kind, it.material);
        if let Some(s) = it.story.as_mut() {
            s.owners.push((to, to_name, tick));
            s.deeds.push((tick, "was given at a pairing".into()));
            what = match (&s.name, s.origin.is_empty()) {
                (Some(n), _) => n.clone(),
                (None, false) => format!("{} {}", crate::view::narrate::a_or_an(&noun), s.origin),
                (None, true) => crate::view::narrate::a_or_an(&noun),
            };
        }
    }
    for k in [gi, ri] {
        w.agents[k]
            .thoughts
            .add(crate::mind::ThoughtKind::GivenARing, tick, None);
    }
    w.agents[ri].relate(giver, 0.1, 0.1, tick);
    w.agents[ri].met(giver, tick);
    let at = w.agents[gi].pos;
    w.events.push(
        tick,
        Some(at),
        Severity::Notable,
        EventKind::GaveRing { giver, to, what },
    );
}

/// Fed enough for a child (FIXLOG 2026-11-15, #1195): not an instant reading of hunger at
/// the 05:00 births stripe, which judged a merely peckish dawn as starving before breakfast.
/// "Fed" instead means neither partner has gone hungry long enough to register as starving
/// (`starving_ticks`, reset in `starvation` above whenever hunger rises past zero) and their
/// community holds at least two days' food (the same "secured" measure `decide::secured`
/// uses, design rule 7 / D56). A couple with no community (not yet settled) is judged on
/// hunger alone.
pub fn fed_for_birth(w: &World, pa: &crate::agents::Agent, pb: &crate::agents::Agent) -> bool {
    let days = pa
        .community
        .map(|c| crate::systems::settlement::food_days(w, c));
    pa.starving_ticks == 0 && pb.starving_ticks == 0 && days.is_none_or(|d| d >= 2.0)
}

/// The first gate stopping a couple on this day's existing birth roll. Not saved state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BirthGate {
    Absent,
    Age,
    Kind,
    Fed,
    Cooldown,
    Crowd,
    Dice,
    Ready,
}

impl BirthGate {
    pub const ALL: [Self; 8] = [
        Self::Absent,
        Self::Age,
        Self::Kind,
        Self::Fed,
        Self::Cooldown,
        Self::Crowd,
        Self::Dice,
        Self::Ready,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Age => "age",
            Self::Kind => "kind",
            Self::Fed => "fed",
            Self::Cooldown => "cooldown",
            Self::Crowd => "crowd",
            Self::Dice => "dice",
            Self::Ready => "ready",
        }
    }
}

/// Classify an ordered partner pair without changing the world or any random stream.
/// The local seeded roll is the same roll births uses, including the original crowd factor.
pub fn birth_gate(w: &World, a: AgentId, b: AgentId) -> BirthGate {
    let tick = w.meta.tick;
    let Some(pa) = w.agents.iter().find(|x| x.id == a && x.alive) else {
        return BirthGate::Absent;
    };
    let Some(pb) = w.agents.iter().find(|x| x.id == b && x.alive) else {
        return BirthGate::Absent;
    };
    if pa.stage(tick) != LifeStage::Adult
        || !pa.fertile_years().contains(&pa.age_years(tick))
        || !pb.fertile_years().contains(&pb.age_years(tick))
    {
        return BirthGate::Age;
    }
    if !crate::species::fertile(pa.people, pb.people) {
        return BirthGate::Kind;
    }
    if !fed_for_birth(w, pa, pb) {
        return BirthGate::Fed;
    }
    let last = pa.last_birth.max(pb.last_birth);
    let year = (crate::time::TICKS_PER_YEAR as f32 / w.meta.life_speed) as u64;
    if last > 0 && tick - last < year {
        return BirthGate::Cooldown;
    }
    let mut r = stream(
        w.meta.seed,
        &[
            purpose::AGENT_BIRTH,
            a.0 as u64,
            b.0 as u64,
            tick / TICKS_PER_DAY,
        ],
    );
    let per_day = w.meta.life_speed / (3.0 * crate::time::DAYS_PER_YEAR as f32);
    let community_people = w
        .agents
        .iter()
        .filter(|x| x.alive && x.species == Species::Person && x.community == pa.community)
        .count();
    let crowd = if community_people >= 40 { 0.5 } else { 1.0 };
    let roll = r.gen::<f32>();
    if roll < per_day * crowd {
        BirthGate::Ready
    } else if roll < per_day {
        BirthGate::Crowd
    } else {
        BirthGate::Dice
    }
}

fn births(w: &mut World, observer: &mut impl FnMut(AgentId, AgentId, BirthGate)) {
    let couples: Vec<(AgentId, AgentId)> = w
        .agents
        .iter()
        .chain(w.overworld.away.iter())
        .filter(|a| a.alive && a.partner.is_some())
        .filter_map(|a| a.partner.map(|p| (a.id, p)))
        .filter(|(a, b)| a < b)
        .collect();
    for (a, b) in couples {
        let gate = birth_gate(w, a, b);
        observer(a, b, gate);
        if gate == BirthGate::Ready {
            let _ = w.give_birth(a, b);
        }
    }
}

fn aging(w: &mut World) {
    let tick = w.meta.tick;
    let seed = w.meta.seed;
    let mut dead = Vec::new();
    for a in w.agents.iter() {
        if !a.alive || a.species != Species::Person {
            continue;
        }
        let years = a.age_years(tick) as f32;
        let span = a.lifespan_years();
        if years < span - 8.0 {
            continue;
        }
        // Daily risk rises from (span-8) to certainty around span+8.
        let x = ((years - (span - 8.0)) / 16.0).clamp(0.0, 1.0);
        let per_day = (0.0015 + x * x * 0.06) * w.meta.life_speed;
        let mut r = stream(
            seed,
            &[
                purpose::AGENT_BIRTH,
                0xDEAD,
                a.id.0 as u64,
                tick / TICKS_PER_DAY,
            ],
        );
        if r.gen::<f32>() < per_day {
            dead.push((a.id, "old age".to_string()));
        }
    }
    for (id, cause) in dead {
        die(w, id, cause);
    }
}

/// Death: leases released, carried goods dropped, community membership ended, partner
/// freed, those who knew them remember, and the event recorded with its cause.
pub fn die(w: &mut World, id: AgentId, cause: String) {
    die_at(w, id, cause, true);
}

/// Death away from the valley (lost on the road, lost in the deep): the body is not here, so
/// the death has no place, nobody can walk to it, and the rite is given at home in its absence
/// (#566, FIXLOG 2026-11-02). Before this the body was put back at the tile they left from and
/// buried there as if it had come home.
pub fn die_away(w: &mut World, id: AgentId, cause: String) {
    die_at(w, id, cause, false);
}

/// A person or a kept (herd) animal's death is Major; a wild creature's is Minor when a
/// person saw it (same rule as `accounts::witnessed`'s sight check) and Trace otherwise, so a
/// bat dying unseen in a cave does not read as more important than the valley's own people
/// (#809, FIXLOG 2026-11-15).
fn death_severity(w: &World, dead: usize) -> Severity {
    let a = &w.agents[dead];
    if a.species == Species::Person || a.kept.is_some() {
        return Severity::Major;
    }
    let (pos, tick) = (a.pos, w.meta.tick);
    let seen = w.agents.iter().any(|o| {
        o.alive
            && o.species == Species::Person
            && !o.activity.is_asleep()
            && !o.is_child(tick)
            && o.pos.z == pos.z
            && o.pos.dist(pos) <= crate::accounts::WITNESS_WITHIN
            && crate::mind::perception::line_of_sight(&w.grid, o.pos, pos)
    });
    if seen {
        Severity::Minor
    } else {
        Severity::Trace
    }
}

fn die_at(w: &mut World, id: AgentId, cause: String, body_here: bool) {
    let tick = w.meta.tick;
    let Some(i) = w.agents.iter().position(|a| a.id == id && a.alive) else {
        return;
    };
    let pos = w.agents[i].pos;
    // Alone: none of their own people awake within eight tiles (C68, #229).
    let own = w.agents[i].community;
    let alone = own.is_none()
        || !body_here
        || !w.agents.iter().any(|o| {
            o.alive && o.id != id && o.community == own && o.pos.z == pos.z && o.pos.dist(pos) <= 8
        });
    w.agents[i].died_alone = alone;
    w.agents[i].alive = false;
    w.agents[i].death = Some(cause.clone());
    w.agents[i].activity = Activity::Idle;
    w.agents[i].path.clear();
    w.reservations.release_all(id);
    for it in w.items.iter_mut() {
        if it.place == crate::items::ItemPlace::Carried(id) {
            it.place = crate::items::ItemPlace::Ground(pos);
        }
    }
    let community = w.agents[i].community;
    if let Some(c) = community {
        if let Some(c) = w.communities.iter_mut().find(|x| x.id == c) {
            c.members.retain(|m| *m != id);
        }
    }
    let partner = w.agents[i].partner;
    let name_known: Vec<AgentId> = w
        .agents
        .iter()
        .filter(|o| o.alive && o.relationships.contains_key(&id))
        .map(|o| o.id)
        .collect();
    for o in name_known {
        if let Some(a) = w.agents.iter_mut().find(|a| a.id == o) {
            let close = a.opinion_of(id);
            let sal = (0.4 + close.max(0.0) * 0.6).min(1.0);
            a.remember(
                tick,
                MemKind::Hardship,
                Subject::Person(id),
                -0.6 - close.max(0.0) * 0.3,
                sal,
                None,
            );
            if Some(id) == a.partner {
                a.partner = None;
            }
        }
    }
    let _ = partner;
    w.unpair(id);
    let death = w.events.push(
        tick,
        body_here.then_some(pos),
        death_severity(w, i),
        EventKind::Died {
            agent: id,
            cause: cause.clone(),
        },
    );
    // Those who saw it hold an account of it (#452).
    crate::accounts::witnessed(w, i, death, &cause);
    // Their people lay them to rest by a rite (P7), once someone awake has come to them
    // (C68, #220: before this the rite was done on the tick of the death, by someone who could
    // be asleep across the valley); a person nobody lays to rest within a day begins unrested
    // (P11).
    if w.agents[i].species == crate::agents::Species::Person {
        match community {
            Some(c) => w.rites_due.push(crate::systems::rites::RiteDue {
                dead: id,
                community: c,
                since: tick,
                death,
                doer: None,
                away: !body_here,
            }),
            None => crate::systems::rites::left_unrested(
                w,
                i,
                death,
                "none of their own people",
                !body_here,
            ),
        }
    }
}

/// The cloth colour (view/look.rs, the client's CLOTH_COLOURS order: undyed, brown, rust,
/// madder, ochre, moss, leaf, teal, sky, indigo, violet, slate) a dyestuff gives (#286).
pub fn dyed_colour(m: crate::world::Material) -> Option<u8> {
    use crate::world::Material as M;
    match m {
        M::WOAD => Some(9),
        M::MADDER => Some(3),
        M::WELD => Some(4),
        _ => None,
    }
}
