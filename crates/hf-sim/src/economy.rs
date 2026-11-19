//! Conservation ledger: every item creation and destruction, by cause. The soak harness
//! checks that the live inventory equals created minus destroyed, per kind.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::items::ItemKind;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Ledger {
    pub created: BTreeMap<String, u64>,
    pub destroyed: BTreeMap<String, u64>,
    pub by_cause: BTreeMap<String, i64>,
}

impl Ledger {
    pub fn created(&mut self, kind: ItemKind, n: u32, cause: &str) {
        *self.created.entry(format!("{kind:?}")).or_default() += n as u64;
        *self.by_cause.entry(format!("+{cause}")).or_default() += n as i64;
    }
    pub fn destroyed(&mut self, kind: ItemKind, n: u32, cause: &str) {
        *self.destroyed.entry(format!("{kind:?}")).or_default() += n as u64;
        *self.by_cause.entry(format!("-{cause}")).or_default() += n as i64;
    }
    /// Expected live count per kind.
    pub fn expected(&self) -> BTreeMap<String, i64> {
        let mut m = BTreeMap::new();
        for (k, v) in &self.created {
            *m.entry(k.clone()).or_default() += *v as i64;
        }
        for (k, v) in &self.destroyed {
            *m.entry(k.clone()).or_default() -= *v as i64;
        }
        m
    }
}

/// Live counts per kind from the item list.
pub fn live_counts(items: &[crate::items::Item]) -> BTreeMap<String, i64> {
    let mut m = BTreeMap::new();
    for it in items {
        *m.entry(format!("{:?}", it.kind)).or_default() += it.count as i64;
    }
    m
}
