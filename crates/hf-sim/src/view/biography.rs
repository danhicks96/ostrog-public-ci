//! #1453: a person's life as the chronicle has it, without the routine. `history::story_of`
//! returns every row naming them (thousands for an old hand: talk, nightly suspicions); this
//! keeps the rows that make a life: everything Notable or above, plus the few minor kinds
//! that mark one (the first of each role taken, a craft learned, a ring given). Every line is
//! narrated from a recorded `real_history` row, never invented (D9).

use anyhow::Result;

use crate::chronicle::Chronicle;
use crate::events::{EventKind, Severity};
use crate::ids::AgentId;
use crate::world_state::World;

#[derive(Clone, Debug, PartialEq)]
pub struct BioLine {
    /// The `real_history` row this line is told from.
    pub history_id: i64,
    pub tick: u64,
    pub line: String,
}

/// The story of `agent`, oldest first: rows of Notable severity or above, plus the first
/// `RoleTaken` of each role, a craft learned (taught, watched, or worked out) and a ring given.
pub fn biography(w: &World, c: &Chronicle, agent: AgentId) -> Result<Vec<BioLine>> {
    let mut st = c.conn.prepare_cached(
        "SELECT r.id, r.tick, r.severity, r.detail FROM real_history r \
         JOIN entity_ref e ON e.history_id = r.id \
         WHERE e.entity_kind = 'agent' AND e.entity_id = ?1 ORDER BY r.tick, r.id",
    )?;
    let rows = st
        .query_map(rusqlite::params![agent.0 as i64], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)? as u64,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut roles: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for (id, tick, severity, detail) in rows {
        let Some(e) = crate::view::history::event_from_row(id, tick, &detail) else {
            continue;
        };
        let marks_a_life = match &e.kind {
            EventKind::RoleTaken { role, .. } => {
                let first = !roles.contains(role);
                if first {
                    roles.push(role.clone());
                }
                first
            }
            EventKind::Taught { pupil, .. } => *pupil == agent,
            EventKind::LearnedByWatching { agent: a, .. } => *a == agent,
            EventKind::CraftTried { success, .. } => *success,
            EventKind::GaveRing { .. } => true,
            _ => false,
        };
        if severity >= Severity::Notable as i64 || marks_a_life {
            out.push(BioLine {
                history_id: id,
                tick,
                line: crate::view::narrate::line(w, &e),
            });
        }
    }
    Ok(out)
}
