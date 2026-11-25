//! Drinking water (#433): where people can drink, and whether the water serves.
//!
//! Fresh water is a body of the valley's surveyed waters (a stream, a lake, a pond, #393) where
//! it still stands, or a well. Flood water spread over the land and water standing in the
//! workings (let in from the rock or a broken-in pond) is muddy and does not serve. In a hard
//! frost the open water is ice (as fishing has it, below -2 C): then a well serves, and so does
//! a hearth, where snow and ice are melted to drink.

use crate::ids::TilePos;
use crate::structures::StructureKind;
use crate::World;
use rand::Rng;

/// Below this the open water is ice (fish.rs reads the same).
pub const ICE_C: f32 = -2.0;
/// Drinks a carried vessel holds (a bucket or a pot of water).
pub const VESSEL: u32 = 6;
/// The most a household's barrel holds, in drinks.
pub const STORE_MAX: u32 = 60;
/// Water nearer home than this and no store is kept: they walk to it.
pub const STORE_BEYOND: i32 = 6;

/// The chance a drink from a fouled tile brings the flux, before the Sickness deity lever
/// leans it. One foul drink is a day's unlucky living near a corpse (health.rs daily rolls:
/// 0.05 the flux near the dead lying, 0.2 a sickness close by) condensed into the moment of
/// drinking.
const FLUX_CHANCE: f32 = 0.2;

/// A community's water barrel: a finished barrel within four tiles of home.
pub fn barrel(w: &World, home: TilePos) -> Option<TilePos> {
    w.structures
        .iter()
        .filter(|s| s.kind == StructureKind::Barrel && s.progress >= 1.0 && s.pos.dist(home) <= 4)
        .min_by_key(|s| (s.pos.dist(home), s.pos))
        .map(|s| s.pos)
}

/// Whether someone standing at `at` can drink from `source` (#608, FIXLOG 2026-11-03): on it or
/// beside it (diagonals too, as a barrel or a well is reached), at its level or one level above
/// it (a lake lies a level under its bank). Every stand `spots` offers holds this, and every
/// check of whether a drink can be had beside a structure reads it, so the place a person is
/// sent to drink and the place a drink is taken cannot disagree. `TilePos::dist` counts a level
/// as four, so `dist <= 1` refused a stand one level up: a whole town stood on a wall beside its
/// full barrel until it died of thirst.
pub fn reaches(at: TilePos, source: TilePos) -> bool {
    (at.x - source.x).abs() <= 1
        && (at.y - source.y).abs() <= 1
        && (0..=1).contains(&(at.z - source.z))
}

/// How long a stand where a drink failed is not offered to that person again (#608): a quarter
/// of a day, long enough to drink somewhere else and come back. See `dry_key`.
pub const DRY_STAND_TICKS: u64 = crate::time::TICKS_PER_DAY / 4;

/// The `work_cooldowns` key for "no drink was had standing here lately" (#608): the tile packed
/// as `decide::no_way_key` packs it, 0xD marking it.
pub fn dry_key(p: TilePos) -> u32 {
    0xD000_0000 | ((p.x as u32 & 0x7FF) << 17) | ((p.y as u32 & 0x7FF) << 6) | (p.z as u32 & 0x3F)
}

/// A place to drink: where to stand, and what is drunk from there.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Spot {
    pub stand: TilePos,
    pub source: TilePos,
    pub well: bool,
}

/// A spot, checked (#608): its stand reaches its source, or the stand is a place where no drink
/// can be had and whoever is sent there stands thirsty.
fn spot(stand: TilePos, source: TilePos, well: bool) -> Spot {
    debug_assert!(
        reaches(stand, source),
        "a stand at {stand} does not reach {source} (#608)"
    );
    Spot {
        stand,
        source,
        well,
    }
}

/// Whether the open water is frozen over, valley-wide: the sky's snow-or-rain call and the
/// hearth-water offers stay on the valley reading (#268's lapse is a per-tile rule).
pub fn frozen(w: &World) -> bool {
    w.weather.temperature_c < ICE_C
}

/// Whether the water at `p` is iced over (#268): a tarn on the heights freezes in a cold
/// the valley floor does not.
pub fn frozen_at(w: &World, p: TilePos) -> bool {
    w.temp_at(p) < ICE_C
}

/// Whether `p` is fresh water to drink from now: a tile of a surveyed body that still holds
/// water (not dried by drought), not flood water, not in the workings, and not iced over.
pub fn fresh(w: &World, p: TilePos) -> bool {
    let t = w.grid.get(p);
    t.is_water()
        && !w.flooded.contains(&p)
        && !w.waters.left.contains_key(&p)
        && !w.dug.contains(&p)
        && !frozen_at(w, p)
}

/// Somewhere beside `p` a person can stand to drink: a walkable tile next to it, at its level
/// or one above (a lake lies a level under the grass of its shore).
fn stand_by(w: &World, p: TilePos) -> Option<TilePos> {
    for dz in [1, 0] {
        for (dx, dy) in [(0, 1), (1, 0), (0, -1), (-1, 0)] {
            let q = p.offset(dx, dy, dz);
            if w.grid.in_bounds(q)
                && w.grid.walkable(q)
                && !w.grid.get(q).is_water()
                && !w.grid.get(q.offset(0, 0, -1)).is_water()
            {
                debug_assert!(reaches(q, p));
                return Some(q);
            }
        }
    }
    None
}

/// Every place to drink within `radius` of `near`, nearest first by the straight line: the
/// shores of the valley's waters where they serve, and finished wells.
pub fn spots(w: &World, near: TilePos, radius: i32) -> Vec<Spot> {
    let mut out: Vec<(i32, Spot)> = Vec::new();
    for s in w
        .structures
        .iter()
        .filter(|s| s.kind == StructureKind::Well && s.progress >= 1.0)
    {
        let d = s.pos.dist(near);
        if d <= radius {
            if let Some(stand) = stand_by(w, s.pos).or(Some(s.pos)) {
                out.push((d, spot(stand, s.pos, true)));
            }
        }
    }
    // Frozen over, a hearth melts snow and ice (#433): what anyone does in a hard winter.
    if frozen(w) {
        for s in w
            .structures
            .iter()
            .filter(|s| s.kind == StructureKind::Hearth && s.progress >= 1.0)
        {
            let d = s.pos.dist(near);
            if d <= radius {
                if let Some(stand) = stand_by(w, s.pos) {
                    out.push((d, spot(stand, s.pos, false)));
                }
            }
        }
    }
    if !frozen(w) {
        for b in w.waters.bodies.iter() {
            // Every water tile of it, not only its surveyed shore: in a drought the shore dries
            // and the water's edge is what was its middle.
            for &p in &b.tiles {
                let d = p.dist(near);
                if d > radius || !fresh(w, p) {
                    continue;
                }
                if let Some(stand) = stand_by(w, p) {
                    // A river in spate is drunk from only where the bank stands above any
                    // flood yet seen: nobody goes down into a flood to drink.
                    if b.high && w.waters.flooded_once(stand.x, stand.y) {
                        continue;
                    }
                    out.push((d, spot(stand, p, false)));
                }
            }
        }
    }
    // A household's store, while it holds water, and only when there is no other water to
    // drink at within a short walk clear of any flood (#433: drunk from by all whose way
    // passed it, the store of seed 4 stood empty when its stream flooded).
    let water_near = out
        .iter()
        .any(|(d, s)| *d <= 12 && !crate::systems::hazards::in_floods_way(w, s.stand));
    if !water_near {
        for c in w.communities.iter().filter(|c| !c.dissolved && c.water > 0) {
            if let Some(b) = barrel(w, c.home) {
                let d = b.dist(near);
                if d <= radius {
                    if let Some(stand) = stand_by(w, b) {
                        out.push((d, spot(stand, b, false)));
                    }
                }
            }
        }
    }
    out.sort_by_key(|(d, s)| (*d, s.source));
    out.into_iter().map(|(_, s)| s).collect()
}

/// The nearest place to drink within `radius` of `from` that can be walked to.
pub fn nearest(w: &World, from: TilePos, radius: i32) -> Option<Spot> {
    let nav = w.nav.as_ref()?;
    // The nearest few by the straight line, then on: a lake beyond a cliff is near and
    // unreachable, and the next shore that can be walked to may be a little farther.
    candidates(w, from, radius)
        .into_iter()
        .find(|s| s.stand == from || nav.reach(from, s.stand))
}

/// The places to drink within `radius` of `from`, nearest first by the straight line, the
/// nearest 48 of them with each stand tile once, none in a flood's way (#418).
fn candidates(w: &World, from: TilePos, radius: i32) -> Vec<Spot> {
    let mut tried = std::collections::BTreeSet::new();
    spots(w, from, radius)
        .into_iter()
        .filter(|s| tried.insert(s.stand))
        // Not into a flood's way (#418): the next bank clear of it serves.
        .filter(|s| !crate::systems::hazards::in_floods_way(w, s.stand))
        .take(48)
        .collect()
}

/// What `knowledge` believes of a spot's water, as a `TAG_WATER_*` value (#453, epic #477
/// part 4): the belief of the spot's own tile, else of a tile of the same body of water.
/// A foul belief anywhere taints the whole body for them — one foul shore noted is the
/// pond remembered foul. None where the water was never perceived.
fn believed_tag(w: &World, knowledge: &crate::mind::knowledge::Knowledge, s: &Spot) -> Option<u16> {
    use crate::mind::knowledge::Known;
    let mut tag = knowledge.get(Known::Water(s.source)).map(|b| b.tag);
    if tag == Some(crate::floods::TAG_WATER_FOUL) {
        return tag;
    }
    let body = crate::floods::body_at(w, s.source)?;
    for (k, b) in knowledge.iter_kind(|k| matches!(k, Known::Water(_))) {
        if let Known::Water(p) = k {
            if *p == s.source {
                continue;
            }
            // Same column of the same surveyed body (body_at matches by column).
            let same = crate::floods::body_at(w, *p).is_some_and(|ob| std::ptr::eq(ob, body));
            if !same {
                continue;
            }
            // Foul taints the whole body for them; else the first belief by key order.
            match (tag, b.tag) {
                (_, crate::floods::TAG_WATER_FOUL) => return Some(b.tag),
                (None, t) => tag = Some(t),
                _ => {}
            }
        }
    }
    tag
}

/// The nearest place to drink within `radius` of `from`, chosen by what `knowledge`
/// believes of each water (#453, epic #477 part 4, FIXLOG 2026-10-30): a spot whose water is
/// known salt is never worth the walk; one known foul is shunned unless `desperate` — thirst
/// past the parched mark (decide.rs), or nothing they could believe fresh within reach.
/// Water never perceived is assumed fresh: they can be wrong, and part 3's flux roll, ground
/// truth at drink time, is the punishment.
///
/// A stand where agent `i` lately failed to drink (`dry_key`, #608) is passed over: they go to
/// the next water, not back to stand where no drink was had.
pub fn nearest_knowing(
    w: &World,
    i: usize,
    from: TilePos,
    radius: i32,
    knowledge: &crate::mind::knowledge::Knowledge,
    desperate: bool,
) -> Option<Spot> {
    let nav = w.nav.as_ref()?;
    let now = w.meta.tick;
    let cooldowns = &w.agents[i].work_cooldowns;
    // The lesson survives an evicted water tag. Cache its body once, not for every shore.
    let lessons: Vec<_> = w.agents[i]
        .learning
        .lessons
        .iter()
        .filter_map(|lesson| {
            if lesson.cue != crate::mind::ObservationKind::FoulWater
                || lesson.outcome != crate::mind::OutcomeKind::Illness
            {
                return None;
            }
            let pos = lesson.pos?;
            Some((
                pos,
                crate::floods::body_at(w, pos),
                lesson.confidence.clamp(0.0, 1.0),
            ))
        })
        .collect();
    let illness = |s: &Spot| {
        if lessons.is_empty() || s.well || !w.grid.get(s.source).is_water() {
            return 0.0;
        }
        let body = crate::floods::body_at(w, s.source);
        lessons
            .iter()
            .filter(|(pos, learned_body, _)| {
                *pos == s.source
                    || body
                        .zip(*learned_body)
                        .is_some_and(|(a, b)| std::ptr::eq(a, b))
            })
            .map(|(_, _, confidence)| *confidence)
            .fold(0.0f32, f32::max)
    };
    let spots: Vec<_> = candidates(w, from, radius)
        .into_iter()
        .filter(|s| cooldowns.get(&dry_key(s.stand)).is_none_or(|t| *t <= now))
        .map(|s| {
            let confidence = illness(&s);
            (s, confidence)
        })
        .collect();
    let tag = |s: &Spot| believed_tag(w, knowledge, s);
    let salt = |s: &Spot| tag(s) == Some(crate::floods::TAG_WATER_SALT);
    let foul = |s: &Spot, confidence: f32| {
        tag(s) == Some(crate::floods::TAG_WATER_FOUL) || confidence > 0.2
    };
    // Nothing they could believe fresh in reach is its own desperation: foul water or none.
    let desperate = desperate
        || !spots
            .iter()
            .any(|(s, confidence)| !salt(s) && !foul(s, *confidence));
    let thirst = (1.0 - w.agents[i].needs.thirst).clamp(0.0, 1.0);
    spots
        .into_iter()
        .filter(|(s, confidence)| {
            !salt(s)
                && (!foul(s, *confidence)
                    || if *confidence > 0.0 {
                        thirst > 0.7 + 0.25 * confidence
                    } else {
                        desperate
                    })
        })
        .map(|(s, _)| s)
        .find(|s| s.stand == from || nav.reach(from, s.stand))
}

/// Whether someone standing at `at` can drink there now: fresh water or a well beside them.
pub fn can_drink_at(w: &World, at: TilePos) -> bool {
    (-1..=0).any(|dz| {
        [(0, 0), (0, 1), (1, 0), (0, -1), (-1, 0)]
            .iter()
            .any(|(dx, dy)| {
                let q = at.offset(*dx, *dy, dz);
                w.grid.in_bounds(q)
                    && (fresh(w, q)
                        || w.structures.iter().any(|s| {
                            s.kind == StructureKind::Well && s.progress >= 1.0 && s.pos == q
                        }))
            })
    }) || w.structures.iter().any(|s| {
        s.progress >= 1.0
            && reaches(at, s.pos)
            && (s.kind == StructureKind::Well || (s.kind == StructureKind::Hearth && frozen(w)))
    })
}

/// Agent `i` drinks from what is beside them (#433): fresh water or a well, else a
/// household's barrel with water in it. True if they drank.
pub fn drink(w: &mut World, i: usize) -> bool {
    let at = w.agents[i].pos;
    if !can_drink_at(w, at) {
        let store = w.communities.iter().position(|c| {
            !c.dissolved && c.water > 0 && barrel(w, c.home).is_some_and(|b| reaches(at, b))
        });
        let Some(k) = store else {
            return false;
        };
        w.communities[k].water -= 1;
    }
    // Where they drank is water they know of (#433): a lake a level under its bank was not
    // seen by the eye's scan of their own level.
    let source = (-1..=0).find_map(|dz| {
        [(0, 1), (1, 0), (0, -1), (-1, 0)]
            .iter()
            .map(|(dx, dy)| at.offset(*dx, *dy, dz))
            .find(|q| w.grid.in_bounds(*q) && fresh(w, *q))
    });
    let tick = w.meta.tick;
    let was_ill = w.agents[i].body.illness.is_some();
    // Tagged with what the drink taught them (#453 part 4): a fouled tile tasted is
    // remembered foul.
    let tag = source.map(|p| {
        if w.waters.foul.contains_key(&p) {
            crate::floods::TAG_WATER_FOUL
        } else {
            crate::floods::quality(w, p)
                .map(|q| q.tag())
                .unwrap_or(crate::floods::TAG_WATER_FRESH)
        }
    });
    {
        let a = &mut w.agents[i];
        a.needs.thirst = 1.0;
        a.needs.parched_ticks = 0;
        if let (Some(p), Some(tag)) = (source, tag) {
            a.knowledge
                .note(crate::mind::knowledge::Known::Water(p), p, tick, tag);
        }
    }
    // FIXLOG 2026-10-30 #453 (epic #477, part 3): a drink from a fouled tile can bring the
    // flux. The foul map is read first — a salt or brackish tile fouls the same as fresh, and
    // salt's own effects are part 4. Wells and barrel stores never reach here: `source` is a
    // tile of open water, and they are none. The roll is once per drink, keyed by agent and
    // day like health.rs's daily rolls, and the Sickness lever leans it.
    if let Some(p) = source {
        let mut illness_event = None;
        if w.waters.foul.contains_key(&p) {
            let sickness = w.lean(crate::deity::Lever::Sickness, crate::deity::Scope::Valley);
            let mut r = crate::rng::stream(
                w.seed(),
                &[
                    crate::rng::purpose::DRINK_FLUX,
                    w.agents[i].id.0 as u64,
                    tick / crate::time::TICKS_PER_DAY,
                ],
            );
            if r.gen::<f32>() < FLUX_CHANCE * sickness {
                let pending = w.events.pending.len();
                crate::systems::health::catch_at(
                    w,
                    i,
                    crate::body::IllnessKind::Flux,
                    0.35,
                    "the flux from foul water",
                );
                illness_event = w.events.pending[pending..].iter().find_map(|e| {
                    matches!(
                        e.kind,
                        crate::events::EventKind::FellIll { agent, .. }
                            if agent == w.agents[i].id
                    )
                    .then_some(e.id)
                });
            }
        }
        use crate::mind::{Evidence, Observation, ObservationKind, OutcomeKind};
        let a = &mut w.agents[i];
        if let Some(event) = illness_event {
            a.learning.observe(Observation {
                tick,
                pos: p,
                kind: ObservationKind::FoulWater,
                strength: 1.0,
                evidence: Evidence::Direct,
                event: Some(event),
                full: true,
                reading: None,
            });
            a.learning.learn(
                ObservationKind::FoulWater,
                OutcomeKind::Illness,
                Some(p),
                true,
                tick,
                Some(event),
            );
        } else if !was_ill
            && a.learning.lessons.iter().any(|lesson| {
                lesson.cue == ObservationKind::FoulWater
                    && lesson.outcome == OutcomeKind::Illness
                    && lesson.pos == Some(p)
            })
        {
            // No new illness is counter-evidence only to a lesson already held. An already
            // sick person cannot test whether this particular drink would sicken them.
            a.learning.learn(
                ObservationKind::FoulWater,
                OutcomeKind::Illness,
                Some(p),
                false,
                tick,
                None,
            );
        }
    }
    true
}

/// The planner's part (systems/settlement.rs): a household whose water is more than a few
/// steps from home keeps a day's water in a barrel by home, filled a vessel at a time by
/// whoever takes the order, while it has a bucket, a pot or a skin (a hide) to carry it in.
pub fn plan_store(w: &mut World, cid: crate::ids::CommunityId, home: TilePos) {
    let Some(far) = crate::wells::water_within(w, home, 40) else {
        return;
    };
    // Water near home that floods (#433: in seed 4 every bank within reach lay in the
    // flood's way for a day at a time, and all were parched): keep water in as well.
    let floods =
        (-8..=8).any(|dy| (-8..=8).any(|dx| w.waters.flooded_once(home.x + dx, home.y + dy)));
    if far <= STORE_BEYOND && !floods {
        return;
    }
    let posted = |w: &World, k: crate::work::DesignationKind| {
        w.designations
            .iter()
            .filter(|d| !d.done && d.kind == k && d.pos.dist(home) <= 6)
            .count()
    };
    let Some(b) = barrel(w, home) else {
        let wood = crate::world::Material::WOOD;
        if posted(
            w,
            crate::work::DesignationKind::BuildStructure(StructureKind::Barrel, wood),
        ) == 0
        {
            if let Some(p) =
                crate::systems::settlement::find_flat_area(w, home.offset(2, 2, 0), 1, 1, 0)
            {
                let d = crate::systems::settlement::make(
                    w,
                    crate::work::DesignationKind::BuildStructure(StructureKind::Barrel, wood),
                    p,
                    6,
                );
                w.designations.push(d);
            }
        }
        return;
    };
    let here = crate::systems::settlement::Here::of(w, cid, home);
    let vessel = w.items.iter().any(|it| {
        matches!(
            it.kind,
            crate::items::ItemKind::Bucket
                | crate::items::ItemKind::Vessel
                // A hide sewn up is a water skin.
                | crate::items::ItemKind::Hide
                | crate::items::ItemKind::Leather
        ) && here.holds_item(it)
    });
    if !vessel {
        return;
    }
    let Some(c) = w.communities.iter().find(|c| c.id == cid) else {
        return;
    };
    // A day's water: four drinks a head.
    let want = (c.members.len() as u32 * 4).clamp(VESSEL, STORE_MAX);
    let open = posted(w, crate::work::DesignationKind::FetchWater) as u32;
    if c.water + open * VESSEL < want && open < 2 {
        let mut d =
            crate::systems::settlement::make(w, crate::work::DesignationKind::FetchWater, b, 5);
        d.community = Some(cid);
        w.designations.push(d);
    }
}
