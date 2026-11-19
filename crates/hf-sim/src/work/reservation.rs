//! Leases on contested things. Acquisition is all-or-nothing over a sorted key list, nobody
//! ever waits for a lease, and leases expire, so there is no hold-and-wait and no deadlock.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::{AgentId, ItemId, StructureId, TilePos};

use super::designation::DesignationId;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum ResKey {
    Item(ItemId),
    Designation(DesignationId),
    Station(StructureId),
    Tile(TilePos),
    Plant(TilePos),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Lease {
    pub holder: AgentId,
    pub expires: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReservationTable {
    pub leases: BTreeMap<ResKey, Lease>,
    pub expired_total: u64,
}

impl ReservationTable {
    pub fn holder(&self, k: ResKey) -> Option<AgentId> {
        self.leases.get(&k).map(|l| l.holder)
    }

    pub fn is_free_for(&self, k: ResKey, who: AgentId) -> bool {
        match self.leases.get(&k) {
            None => true,
            Some(l) => l.holder == who,
        }
    }

    /// Acquire every key or none. Keys are sorted first (canonical order).
    pub fn acquire_all(&mut self, who: AgentId, keys: &[ResKey], expires: u64) -> bool {
        let mut sorted: Vec<ResKey> = keys.to_vec();
        sorted.sort();
        sorted.dedup();
        if !sorted.iter().all(|k| self.is_free_for(*k, who)) {
            return false;
        }
        for k in sorted {
            self.leases.insert(
                k,
                Lease {
                    holder: who,
                    expires,
                },
            );
        }
        true
    }

    pub fn renew(&mut self, who: AgentId, expires: u64) {
        for l in self.leases.values_mut() {
            if l.holder == who {
                l.expires = expires;
            }
        }
    }

    pub fn release_all(&mut self, who: AgentId) {
        self.leases.retain(|_, l| l.holder != who);
    }

    pub fn release(&mut self, who: AgentId, k: ResKey) {
        if self
            .leases
            .get(&k)
            .map(|l| l.holder == who)
            .unwrap_or(false)
        {
            self.leases.remove(&k);
        }
    }

    /// Drop expired leases; returns how many (a soak metric).
    pub fn sweep(&mut self, now: u64) -> usize {
        let before = self.leases.len();
        self.leases.retain(|_, l| l.expires > now);
        let n = before - self.leases.len();
        self.expired_total += n as u64;
        n
    }

    /// End every lease `who` holds because they are no longer at the work: the sweep's doing, so
    /// counted with the expired ones (`expired_total`, the leases the sweep ended).
    pub fn revoke_all(&mut self, who: AgentId) {
        let before = self.leases.len();
        self.release_all(who);
        self.expired_total += (before - self.leases.len()) as u64;
    }

    /// Everyone holding a lease, in id order.
    pub fn holders(&self) -> std::collections::BTreeSet<AgentId> {
        self.leases.values().map(|l| l.holder).collect()
    }

    pub fn held_by(&self, who: AgentId) -> Vec<ResKey> {
        self.leases
            .iter()
            .filter(|(_, l)| l.holder == who)
            .map(|(k, _)| *k)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_or_nothing_and_expiry() {
        let mut t = ReservationTable::default();
        let a = AgentId(1);
        let b = AgentId(2);
        let k1 = ResKey::Item(ItemId(5));
        let k2 = ResKey::Designation(DesignationId(9));
        assert!(t.acquire_all(a, &[k2, k1], 100));
        assert!(
            !t.acquire_all(b, &[k1, ResKey::Tile(TilePos::new(0, 0, 0))], 100),
            "b must not get k1"
        );
        assert!(
            !t.leases.contains_key(&ResKey::Tile(TilePos::new(0, 0, 0))),
            "nothing partially acquired"
        );
        assert!(t.acquire_all(a, &[k1], 150), "re-acquire own lease");
        assert_eq!(t.sweep(120), 1, "k2 expired at 100, k1 renewed to 150");
        assert_eq!(t.sweep(200), 1);
        assert!(t.leases.is_empty());
    }
}
