//! What an inhabitant believes exists. Agents choose targets ONLY from here (no omniscience).
//! Entries are stamped with when they were last confirmed; stale entries are retried with
//! lower confidence and removed when found wrong.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::ids::{AgentId, ItemId, StructureId, TilePos};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Known {
    Item(ItemId),
    /// A food plant identified by its tile (plants do not move).
    FoodPlant(TilePos),
    Structure(StructureId),
    Agent(AgentId),
    /// Water accessible at this tile.
    Water(TilePos),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Belief {
    pub pos: TilePos,
    pub seen_tick: u64,
    /// Short descriptor the planner can filter on (item kind index, structure kind index ...).
    /// On a `Known::Water` belief, the quality as perceived (floods.rs `TAG_WATER_*`); saved
    /// beliefs from before the tags load as 0, fresh.
    #[serde(default)]
    pub tag: u16,
    /// Confidence 0..1, drops when the belief fails.
    pub confidence: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Knowledge {
    pub map: BTreeMap<Known, Belief>,
    /// Retired 2026-11 (#1268): place feeling lives in `memory::place_multiplier`; kept so old
    /// saves load. Do not write.
    #[serde(default, skip_serializing)]
    pub place_affect: BTreeMap<TilePos, f32>,
    /// The things of each budget in order of when they were last seen, oldest first (ties by
    /// key), so a full mind finds what to forget without walking itself (C55: the walk was 67%
    /// of a fed 500-person profile). Built on first need, kept in step by every change below;
    /// not saved, rebuilt from `map` after a load.
    #[serde(skip)]
    by_age: Option<[BTreeSet<(u64, Known)>; 3]>,
}

/// What a person keeps in mind: up to CAP things, shared by three budgets (loose things on
/// the ground, people, places: structures, food plants, water). When the mind is full, the
/// oldest thing of a kind that is over its own budget is forgotten, so a field of unhauled
/// sheaves cannot push the settlement's hearth out of mind (with one shared budget it did,
/// and people froze looking for it: 119 froze on seed 3 in the C21 soak). A kind may use
/// room the others leave free.
pub const ITEM_CAP: usize = 128;
pub const PEOPLE_CAP: usize = 64;
pub const PLACE_CAP: usize = 192;
pub const CAP: usize = ITEM_CAP + PEOPLE_CAP + PLACE_CAP;

fn budget(k: &Known) -> usize {
    match k {
        Known::Item(_) => 0,
        Known::Agent(_) => 1,
        Known::FoodPlant(_) | Known::Structure(_) | Known::Water(_) => 2,
    }
}

const BUDGETS: [usize; 3] = [ITEM_CAP, PEOPLE_CAP, PLACE_CAP];

impl Knowledge {
    pub fn note(&mut self, k: Known, pos: TilePos, tick: u64, tag: u16) {
        // One walk of the map, not two, and the age order left alone when the key's place in
        // it does not change (seen twice in one look: a stockpile's things are both in sight
        // and known to members). #205: noting was the largest part of a 500-person look.
        let belief = Belief {
            pos,
            seen_tick: tick,
            tag,
            confidence: 1.0,
        };
        let before = match self.map.entry(k) {
            std::collections::btree_map::Entry::Occupied(mut o) => {
                Some(std::mem::replace(o.get_mut(), belief).seen_tick)
            }
            std::collections::btree_map::Entry::Vacant(v) => {
                v.insert(belief);
                None
            }
        };
        if before == Some(tick) {
            return;
        }
        if let Some(ages) = self.by_age.as_mut() {
            let c = budget(&k);
            if let Some(t) = before {
                ages[c].remove(&(t, k));
            }
            ages[c].insert((tick, k));
        }
        if self.map.len() > CAP {
            // Full: some kind is over its budget (the budgets sum to CAP). Forget the oldest
            // of the kind just seen if it is over, else of a kind that is (first key on a tie).
            let map = &self.map;
            let ages = self.by_age.get_or_insert_with(|| {
                let mut a: [BTreeSet<(u64, Known)>; 3] = Default::default();
                for (key, b) in map {
                    a[budget(key)].insert((b.seen_tick, *key));
                }
                a
            });
            let mine = budget(&k);
            let over = if ages[mine].len() > BUDGETS[mine] {
                mine
            } else {
                (0..3).find(|&c| ages[c].len() > BUDGETS[c]).unwrap_or(mine)
            };
            if let Some(&(t, key)) = ages[over].first() {
                ages[over].remove(&(t, key));
                self.map.remove(&key);
            }
        }
    }

    pub fn forget(&mut self, k: Known) {
        self.remove(k);
    }

    pub fn doubt(&mut self, k: Known) {
        if let Some(b) = self.map.get_mut(&k) {
            b.confidence *= 0.5;
            if b.confidence < 0.2 {
                self.remove(k);
            }
        }
    }

    fn remove(&mut self, k: Known) {
        if let Some(b) = self.map.remove(&k) {
            if let Some(ages) = self.by_age.as_mut() {
                ages[budget(&k)].remove(&(b.seen_tick, k));
            }
        }
    }

    pub fn get(&self, k: Known) -> Option<&Belief> {
        self.map.get(&k)
    }

    pub fn iter_kind<'a>(
        &'a self,
        f: impl Fn(&Known) -> bool + 'a,
    ) -> impl Iterator<Item = (&'a Known, &'a Belief)> + 'a {
        self.map.iter().filter(move |(k, _)| f(k))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: i32) -> TilePos {
        TilePos { x, y: 0, z: 0 }
    }

    /// The walk C55 replaced, kept as the reference the ordered index must agree with.
    fn note_by_walking(m: &mut BTreeMap<Known, Belief>, k: Known, pos: TilePos, tick: u64) {
        let e = m.entry(k).or_insert(Belief {
            pos,
            seen_tick: tick,
            tag: 0,
            confidence: 1.0,
        });
        e.seen_tick = tick;
        e.confidence = 1.0;
        if m.len() > CAP {
            let mut count = [0usize; 3];
            let mut oldest: [Option<(Known, u64)>; 3] = [None; 3];
            for (key, b) in m.iter() {
                let c = budget(key);
                count[c] += 1;
                if oldest[c].is_none_or(|(_, t)| b.seen_tick < t) {
                    oldest[c] = Some((*key, b.seen_tick));
                }
            }
            let mine = budget(&k);
            let over = if count[mine] > BUDGETS[mine] {
                mine
            } else {
                (0..3).find(|&c| count[c] > BUDGETS[c]).unwrap_or(mine)
            };
            if let Some((key, _)) = oldest[over] {
                m.remove(&key);
            }
        }
    }

    #[test]
    fn forgetting_by_age_forgets_what_the_walk_forgot() {
        use rand::{Rng, SeedableRng};
        let mut r = rand_chacha::ChaCha8Rng::seed_from_u64(55);
        let mut k = Knowledge::default();
        let mut m: BTreeMap<Known, Belief> = BTreeMap::new();
        for step in 0..40_000u64 {
            let id = r.gen_range(0..900u32);
            let key = match r.gen_range(0..4) {
                0 => Known::Item(ItemId(id)),
                1 => Known::Agent(AgentId(id)),
                2 => Known::Structure(StructureId(id)),
                _ => Known::FoodPlant(at(id as i32)),
            };
            // Ticks repeat, so ties by key are exercised.
            let tick = step / 7;
            match r.gen_range(0..10) {
                0 => {
                    k.forget(key);
                    m.remove(&key);
                }
                1 => {
                    k.doubt(key);
                    k.doubt(key);
                    k.doubt(key);
                    m.remove(&key);
                }
                _ => {
                    k.note(key, at(1), tick, 0);
                    note_by_walking(&mut m, key, at(1), tick);
                }
            }
            if step % 500 == 0 {
                let a: Vec<_> = k.map.iter().map(|(x, b)| (*x, b.seen_tick)).collect();
                let b: Vec<_> = m.iter().map(|(x, b)| (*x, b.seen_tick)).collect();
                assert_eq!(a, b, "step {step}");
            }
        }
        // A loaded mind (no index) agrees too.
        let mut loaded = Knowledge {
            map: k.map.clone(),
            ..Default::default()
        };
        for i in 0..300u32 {
            loaded.note(Known::Agent(AgentId(5000 + i)), at(1), 10_000, 0);
            note_by_walking(&mut m, Known::Agent(AgentId(5000 + i)), at(1), 10_000);
        }
        let a: Vec<_> = loaded.map.keys().copied().collect();
        let b: Vec<_> = m.keys().copied().collect();
        assert_eq!(a, b);
    }

    fn count(k: &Knowledge, c: usize) -> usize {
        k.map.keys().filter(|x| budget(x) == c).count()
    }

    #[test]
    fn a_pile_of_things_does_not_push_a_place_out_of_mind() {
        let mut k = Knowledge::default();
        k.note(Known::Structure(StructureId(7)), at(1), 0, 0);
        for i in 0..1000u32 {
            k.note(Known::Item(ItemId(i)), at(2), 10 + i as u64, 0);
        }
        assert!(k.get(Known::Structure(StructureId(7))).is_some());
        assert_eq!(k.map.len(), CAP);
        // The newest things are the ones kept.
        assert!(k.get(Known::Item(ItemId(999))).is_some());
        assert!(k.get(Known::Item(ItemId(0))).is_none());
    }

    #[test]
    fn things_seen_later_take_room_only_up_to_their_budget() {
        let mut k = Knowledge::default();
        for i in 0..(CAP as u32 + 5) {
            k.note(Known::Structure(StructureId(i)), at(1), i as u64, 0);
        }
        assert_eq!(k.map.len(), CAP);
        assert!(k.get(Known::Structure(StructureId(0))).is_none());
        for i in 0..1000u32 {
            k.note(Known::Item(ItemId(i)), at(2), 1000 + i as u64, 0);
        }
        // Items stop at their budget; places keep the room nobody else is using.
        assert_eq!(count(&k, 0), ITEM_CAP);
        assert_eq!(count(&k, 2), CAP - ITEM_CAP);
        assert!(count(&k, 2) >= PLACE_CAP);
        // The places kept are the most recently seen.
        assert!(k
            .get(Known::Structure(StructureId(CAP as u32 + 4)))
            .is_some());
    }
}
