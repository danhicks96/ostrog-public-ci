//! Three-tier chronicle on SQLite, pattern ported from the earlier prototype.
//!
//! `real_history` is append-only and immutable (schema triggers reject UPDATE/DELETE).
//! `taught_history` holds what communities tell each other, versioned, allowed to drift.
//! Era summaries are aggregates over real rows. Every narrative line produced anywhere in the
//! game carries a `real_history.id`; the audit test checks that no line lacks one.

use anyhow::Result;
use rusqlite::{params, Connection};

use crate::events::{Event, Severity};

/// (history_id, community, version, narrative, real kind, real tick)
pub type TaughtRow = (i64, i64, i64, String, String, u64);

pub struct Chronicle {
    pub conn: Connection,
    /// The working file of a disk-backed chronicle (`working`), removed when it is dropped.
    path: Option<std::path::PathBuf>,
}

/// Working chronicles opened by this process, for unique file names.
static WORKING: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

impl Drop for Chronicle {
    fn drop(&mut self) {
        if let Some(p) = self.path.take() {
            // Close the file before removing it.
            if let Ok(mem) = Connection::open_in_memory() {
                drop(std::mem::replace(&mut self.conn, mem));
            }
            for suffix in ["", "-wal", "-shm"] {
                let _ = std::fs::remove_file(format!("{}{suffix}", p.display()));
            }
        }
    }
}

pub const SCHEMA_VERSION: i64 = 1;

impl Chronicle {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        let c = Chronicle { conn, path: None };
        c.init_schema()?;
        Ok(c)
    }

    pub fn in_memory() -> Result<Self> {
        Self::open(":memory:")
    }

    /// A chronicle kept in a working file in the system temporary folder rather than in
    /// memory: a long game's history runs to hundreds of thousands of rows (about 230 bytes
    /// each in memory), and on disk only a small page cache stays resident. The file is
    /// removed when the chronicle is dropped; saves copy it (save/mod.rs).
    pub fn working() -> Result<Self> {
        let dir = std::env::temp_dir().join("ostrog");
        std::fs::create_dir_all(&dir)?;
        // Leftovers from a process that did not exit cleanly, a day or more old.
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let old = e
                    .metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.elapsed().ok())
                    .is_some_and(|age| age.as_secs() > 86_400);
                if old {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        let n = WORKING.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = dir.join(format!("chronicle-{}-{n}.sqlite", std::process::id()));
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
        }
        let mut c = Self::open(path.to_str().unwrap_or("ostrog-chronicle.sqlite"))?;
        c.conn.execute_batch("PRAGMA cache_size = -8192;")?;
        c.path = Some(path);
        Ok(c)
    }

    /// The working file, for a disk-backed chronicle.
    pub fn path(&self) -> Option<&std::path::Path> {
        self.path.as_deref()
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS real_history (
                id        INTEGER PRIMARY KEY,
                event_id  INTEGER NOT NULL UNIQUE,
                tick      INTEGER NOT NULL,
                severity  INTEGER NOT NULL,
                kind      TEXT NOT NULL,
                x INTEGER, y INTEGER, z INTEGER,
                detail    TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS real_history_tick ON real_history(tick);
            CREATE INDEX IF NOT EXISTS real_history_kind ON real_history(kind);
            CREATE TRIGGER IF NOT EXISTS real_history_no_update BEFORE UPDATE ON real_history
                BEGIN SELECT RAISE(ABORT, 'real history is immutable'); END;
            CREATE TRIGGER IF NOT EXISTS real_history_no_delete BEFORE DELETE ON real_history
                BEGIN SELECT RAISE(ABORT, 'real history is immutable'); END;
            CREATE TABLE IF NOT EXISTS entity_ref (
                history_id INTEGER NOT NULL REFERENCES real_history(id),
                entity_kind TEXT NOT NULL,
                entity_id  INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS entity_ref_entity ON entity_ref(entity_kind, entity_id);
            CREATE TABLE IF NOT EXISTS taught_history (
                id          INTEGER PRIMARY KEY,
                history_id  INTEGER NOT NULL REFERENCES real_history(id),
                community   INTEGER NOT NULL,
                version     INTEGER NOT NULL,
                narrative   TEXT NOT NULL,
                intensity   REAL NOT NULL
            );
            -- Who built what where (#292 item 2, WP-08 8.6): one row per finished wall, door,
            -- floor or structure. Its own table, not real_history: the Built event is Trace
            -- (one wall is not news), and the history views count every real_history row.
            CREATE TABLE IF NOT EXISTS built_record (
                tick INTEGER NOT NULL,
                x INTEGER NOT NULL, y INTEGER NOT NULL, z INTEGER NOT NULL,
                agent INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS built_record_at ON built_record(x, y, z);
            CREATE TABLE IF NOT EXISTS era_summary (
                id INTEGER PRIMARY KEY,
                from_tick INTEGER NOT NULL, to_tick INTEGER NOT NULL,
                kind TEXT NOT NULL, count INTEGER NOT NULL
            );
            "#,
        )?;
        self.conn.execute(
            "INSERT OR IGNORE INTO meta(key, value) VALUES ('schema_version', ?1)",
            params![SCHEMA_VERSION.to_string()],
        )?;
        Ok(())
    }

    /// Re-apply the additive schema after copying a save into a fresh working chronicle.
    /// SQLite backup replaces the destination contents, so tables introduced after an older
    /// save was written must be recreated without disturbing the save's existing rows.
    pub(crate) fn ensure_schema(&self) -> Result<()> {
        self.init_schema()
    }

    /// Record a batch of events in one transaction. Trace-severity events are not persisted.
    pub fn record(&mut self, events: &[Event]) -> Result<usize> {
        let tx = self.conn.transaction()?;
        let mut n = 0;
        {
            let mut ins = tx.prepare_cached(
                "INSERT INTO real_history(event_id, tick, severity, kind, x, y, z, detail) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            )?;
            let mut eref = tx.prepare_cached(
                "INSERT INTO entity_ref(history_id, entity_kind, entity_id) VALUES (?1,'agent',?2)",
            )?;
            let mut kref = tx.prepare_cached(
                "INSERT INTO entity_ref(history_id, entity_kind, entity_id) VALUES (?1,?2,?3)",
            )?;
            let mut built = tx.prepare_cached(
                "INSERT INTO built_record(tick, x, y, z, agent) VALUES (?1,?2,?3,?4,?5)",
            )?;
            for e in events {
                if let crate::events::EventKind::Built { agent, at, .. } = &e.kind {
                    built.execute(params![e.tick, at.x, at.y, at.z, agent.0])?;
                }
                if e.severity == Severity::Trace {
                    continue;
                }
                let detail = serde_json::to_string(&e.kind)?;
                let (x, y, z) = match e.at {
                    Some(p) => (Some(p.x), Some(p.y), Some(p.z)),
                    None => (None, None, None),
                };
                ins.execute(params![
                    e.id.0,
                    e.tick,
                    e.severity as i64,
                    e.kind_name(),
                    x,
                    y,
                    z,
                    detail
                ])?;
                let hid = tx.last_insert_rowid();
                for a in e.participants() {
                    eref.execute(params![hid, a.0])?;
                }
                for (kind, id) in e.refs() {
                    kref.execute(params![hid, kind, id as i64])?;
                }
                n += 1;
            }
        }
        tx.commit()?;
        Ok(n)
    }

    /// Who last built what stands at `p`, and when (#292 item 2): the newest `built_record` row
    /// for the tile. None for things that stood before the record began or were never built.
    pub fn built_at(&self, p: crate::ids::TilePos) -> Result<Option<(u64, crate::ids::AgentId)>> {
        let mut st = self.conn.prepare_cached(
            "SELECT tick, agent FROM built_record WHERE x = ?1 AND y = ?2 AND z = ?3 \
             ORDER BY rowid DESC LIMIT 1",
        )?;
        let mut rows = st.query(params![p.x, p.y, p.z])?;
        Ok(match rows.next()? {
            Some(r) => Some((
                r.get::<_, i64>(0)? as u64,
                crate::ids::AgentId(r.get::<_, i64>(1)? as u32),
            )),
            None => None,
        })
    }

    /// The real_history row id of a recorded event, if it was persisted.
    pub fn history_id(&self, event_id: crate::ids::EventId) -> Result<Option<i64>> {
        let mut st = self
            .conn
            .prepare_cached("SELECT id FROM real_history WHERE event_id = ?1")?;
        let mut rows = st.query(params![event_id.0])?;
        Ok(rows.next()?.map(|r| r.get(0)).transpose()?)
    }

    /// A community's telling of a real event: what they say, how strongly, which version.
    pub fn teach(
        &mut self,
        history_id: i64,
        community: i64,
        version: i64,
        narrative: &str,
        intensity: f64,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO taught_history(history_id, community, version, narrative, intensity) VALUES (?1,?2,?3,?4,?5)",
            params![history_id, community, version, narrative, intensity],
        )?;
        Ok(())
    }

    /// Taught rows with the real event they refer to: (history_id, community, version, narrative, real kind, real tick).
    pub fn taught(&self, limit: usize) -> Result<Vec<TaughtRow>> {
        let mut st = self.conn.prepare_cached(
            "SELECT t.history_id, t.community, t.version, t.narrative, r.kind, r.tick FROM taught_history t JOIN real_history r ON r.id = t.history_id ORDER BY t.id DESC LIMIT ?1",
        )?;
        let rows = st
            .query_map(params![limit as i64], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get::<_, i64>(5)? as u64,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// The stored row of an event: (history id, tick, detail json).
    pub fn row(&self, event_id: crate::ids::EventId) -> Result<Option<(i64, u64, String)>> {
        let mut st = self
            .conn
            .prepare_cached("SELECT id, tick, detail FROM real_history WHERE event_id = ?1")?;
        let mut rows = st.query(params![event_id.0])?;
        Ok(rows
            .next()?
            .map(|r| -> rusqlite::Result<(i64, u64, String)> {
                Ok((r.get(0)?, r.get::<_, i64>(1)? as u64, r.get(2)?))
            })
            .transpose()?)
    }

    /// Taught rows whose real event is missing (must be zero: taught history is a telling
    /// of a real event, never free-standing).
    pub fn orphan_taught(&self) -> Result<i64> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM taught_history t LEFT JOIN real_history r ON r.id = t.history_id WHERE r.id IS NULL",
            [],
            |r| r.get(0),
        )?)
    }

    /// Write one era row (per-kind count over a tick span). Idempotent per (span, kind).
    pub fn write_era(&self, from_tick: u64, to_tick: u64, kind: &str, count: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM era_summary WHERE from_tick = ?1 AND to_tick = ?2 AND kind = ?3",
            params![from_tick as i64, to_tick as i64, kind],
        )?;
        self.conn.execute(
            "INSERT INTO era_summary(from_tick, to_tick, kind, count) VALUES (?1,?2,?3,?4)",
            params![from_tick as i64, to_tick as i64, kind, count],
        )?;
        Ok(())
    }

    /// Per-kind counts of recorded events in a tick span.
    pub fn counts_between(&self, from_tick: u64, to_tick: u64) -> Result<Vec<(String, i64)>> {
        let mut st = self.conn.prepare_cached(
            "SELECT kind, COUNT(*) FROM real_history WHERE tick >= ?1 AND tick < ?2 GROUP BY kind",
        )?;
        let rows = st
            .query_map(params![from_tick as i64, to_tick as i64], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn count(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM real_history", [], |r| r.get(0))?)
    }

    /// The newest event id recorded, if any.
    pub fn max_event_id(&self) -> Result<Option<u32>> {
        Ok(self
            .conn
            .query_row("SELECT MAX(event_id) FROM real_history", [], |r| {
                r.get::<_, Option<i64>>(0)
            })?
            .map(|v| v as u32))
    }

    pub fn max_tick(&self) -> Result<Option<i64>> {
        Ok(self
            .conn
            .query_row("SELECT MAX(tick) FROM real_history", [], |r| r.get(0))?)
    }

    /// Force the WAL into the main file so the on-disk file is self-contained for hashing.
    pub fn checkpoint(&self) -> Result<()> {
        self.conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }

    /// Recent rows for the history view: (history_id, tick, kind, detail json).
    pub fn recent(
        &self,
        min_severity: Severity,
        limit: usize,
    ) -> Result<Vec<(i64, u64, String, String)>> {
        let mut st = self.conn.prepare_cached(
            "SELECT id, tick, kind, detail FROM real_history WHERE severity >= ?1 ORDER BY id DESC LIMIT ?2",
        )?;
        let rows = st
            .query_map(params![min_severity as i64, limit as i64], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)? as u64,
                    r.get(2)?,
                    r.get(3)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// The viewer's favourited people for this save, stored in the chronicle's `meta` table
    /// so they survive save and load without touching the world snapshot (#413).
    pub fn favourites(&self) -> std::collections::BTreeSet<crate::ids::AgentId> {
        let raw: Option<String> = self
            .conn
            .query_row("SELECT value FROM meta WHERE key = 'favourites'", [], |r| {
                r.get(0)
            })
            .ok();
        raw.unwrap_or_default()
            .split(',')
            .filter_map(|s| s.trim().parse::<u32>().ok())
            .map(crate::ids::AgentId)
            .collect()
    }

    /// Persist the viewer's favourited people for this save in the chronicle's `meta` table.
    pub fn set_favourites(
        &self,
        favs: &std::collections::BTreeSet<crate::ids::AgentId>,
    ) -> Result<()> {
        let val: String = favs
            .iter()
            .map(|id| id.0.to_string())
            .collect::<Vec<_>>()
            .join(",");
        self.conn.execute(
            "INSERT INTO meta(key, value) VALUES ('favourites', ?1) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![val],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{EventKind, EventSink};

    #[test]
    fn real_history_is_immutable() {
        let mut c = Chronicle::in_memory().unwrap();
        let mut sink = EventSink::default();
        sink.push(
            5,
            None,
            Severity::Major,
            EventKind::WorldCreated { seed: 1 },
        );
        c.record(&sink.drain()).unwrap();
        assert!(c
            .conn
            .execute("UPDATE real_history SET tick = 9", [])
            .is_err());
        assert!(c.conn.execute("DELETE FROM real_history", []).is_err());
        assert_eq!(c.count().unwrap(), 1);
    }
}
