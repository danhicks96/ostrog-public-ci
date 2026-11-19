//! The history view: eras (per game-year counts), the major record with its detail, and
//! every living custom traced to the real event it grew from. Everything here is read from
//! `real_history` rows or from recorded customs; nothing is generated.

use anyhow::Result;

use crate::chronicle::Chronicle;
use crate::events::{Event, EventKind, Severity};
use crate::ids::EventId;
use crate::time::{Calendar, TICKS_PER_YEAR};
use crate::world_state::World;

/// (history id, tick, headline, detail)
pub type MajorLine = (i64, u64, String, Option<String>);

#[derive(Clone, Debug)]
pub struct Era {
    /// Which year. From 0 up, the colony's own years (0 is the HUD's Year 1); below 0, the
    /// valley's past: -1 - n is n whole years before the portal, as `deep::ago` counts them.
    pub year: i64,
    /// The year as the viewer reads it ("Year 1", "150 years before the portal").
    pub label: String,
    /// (kind, count), most frequent first.
    pub counts: Vec<(String, i64)>,
}

/// When a recorded tick was, as the viewer reads it: the valley's past counted back from the
/// portal (`deep::ago`), the colony's own time on the HUD's calendar.
pub fn when(w: &World, tick: u64) -> String {
    let epoch = w.meta.epoch;
    if tick < epoch {
        crate::deep::ago((epoch - tick) / TICKS_PER_YEAR)
    } else {
        let cal = Calendar::from_tick(tick - epoch);
        format!(
            "Y{} day {} {:02}:{:02}",
            cal.year + 1,
            cal.day_of_year + 1,
            cal.hour,
            cal.minute
        )
    }
}

/// Per-year counts of recorded events the viewer may be told of (view::known), oldest year
/// first: the valley's past by years before the portal, then the colony's years.
pub fn eras(w: &World, c: &Chronicle) -> Result<Vec<Era>> {
    let sql = format!(
        "SELECT CASE WHEN tick < ?2 THEN -1 - (?2 - tick) / ?1 ELSE (tick - ?2) / ?1 END, \
         kind, COUNT(*) FROM real_history WHERE 1{} GROUP BY 1, 2 ORDER BY 1, 3 DESC",
        crate::view::known::rows_told_sql(w)
    );
    let mut st = c.conn.prepare(&sql)?;
    let rows = st
        .query_map(
            rusqlite::params![TICKS_PER_YEAR as i64, w.meta.epoch as i64],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            },
        )?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut out: Vec<Era> = Vec::new();
    for (y, k, n) in rows {
        match out.last_mut() {
            Some(e) if e.year == y => e.counts.push((k, n)),
            _ => out.push(Era {
                year: y,
                label: era_label(y),
                counts: vec![(k, n)],
            }),
        }
    }
    Ok(out)
}

fn era_label(year: i64) -> String {
    if year >= 0 {
        return format!("Year {}", year + 1);
    }
    let ago = crate::deep::ago((-1 - year) as u64);
    let mut ch = ago.chars();
    match ch.next() {
        Some(f) => f.to_uppercase().chain(ch).collect(),
        None => ago,
    }
}

/// A recorded event rebuilt from its row: the text is produced from the stored fields.
pub fn event_from_row(id: i64, tick: u64, detail: &str) -> Option<Event> {
    let kind: EventKind = serde_json::from_str(detail).ok()?;
    Some(Event {
        id: EventId(id as u32),
        tick,
        at: None,
        severity: Severity::Major,
        kind,
    })
}

/// (history id, tick, headline, detail) for the most recent notable rows the viewer may be
/// told of (view::known), latest first by when they happened: the valley's past sits below
/// the arrival however late in the making of the world it was written.
pub fn major(w: &World, c: &Chronicle, limit: usize) -> Result<Vec<MajorLine>> {
    let sql = format!(
        "SELECT id, tick, detail FROM real_history WHERE severity >= ?1{} \
         ORDER BY tick DESC, id DESC LIMIT ?2",
        crate::view::known::rows_told_sql(w)
    );
    let mut st = c.conn.prepare(&sql)?;
    let rows = st
        .query_map(
            rusqlite::params![Severity::Notable as i64, limit as i64],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)? as u64,
                    r.get::<_, String>(2)?,
                ))
            },
        )?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows
        .into_iter()
        .filter_map(|(id, tick, detail)| {
            let e = event_from_row(id, tick, &detail)?;
            Some((
                id,
                tick,
                crate::view::narrate::line(w, &e),
                crate::view::narrate::detail(&e),
            ))
        })
        .collect())
}

#[derive(Clone, Debug)]
pub struct CustomOrigin {
    pub community: String,
    pub label: String,
    pub narrative: String,
    pub intensity: f32,
    pub holders: usize,
    pub origin_tick: u64,
    /// The real event's own line, from its stored row; None only if the row is missing,
    /// which the audit test treats as a defect.
    pub origin_line: Option<String>,
}

/// "Why does this community do that": each living custom with the recorded event behind it.
pub fn custom_origins(w: &World, c: &Chronicle) -> Result<Vec<CustomOrigin>> {
    let mut out = Vec::new();
    for cu in w.customs.iter().filter(|c| !c.lapsed) {
        let community = w
            .communities
            .iter()
            .find(|x| x.id == cu.community)
            .map(|x| x.name.clone())
            .unwrap_or_default();
        let origin_line = match c.row(cu.origin)? {
            Some((id, tick, detail)) => {
                event_from_row(id, tick, &detail).map(|e| crate::view::narrate::line(w, &e))
            }
            None => None,
        };
        out.push(CustomOrigin {
            community,
            label: cu.label(),
            narrative: cu.narrative.clone(),
            intensity: cu.intensity,
            holders: cu.adopted_by.len(),
            origin_tick: cu.origin_tick,
            origin_line,
        });
    }
    Ok(out)
}

/// The history panel as the viewer reads it (#203, QA 2026-10-20): it opened on "501 years
/// before the portal: settlements founded 8" and a row per year of the wider world before the
/// colony's own record. Now the colony's years come first, newest first; the valley's own past
/// after, newest first; and the wider world last, one line per century naming its great events.
#[derive(Clone, Debug, Default)]
pub struct Overview {
    pub colony: Vec<Era>,
    pub valley: Vec<Era>,
    /// One per century before the portal, newest first; `year` is the century's newest year.
    pub world: Vec<Century>,
}

#[derive(Clone, Debug)]
pub struct Century {
    pub year: i64,
    /// "100 to 199 years before the portal", or "Up to 99 years before the portal".
    pub label: String,
    /// (kind, count), most frequent first.
    pub counts: Vec<(String, i64)>,
    /// The century's great events in words, oldest first (realms rising and falling, wars),
    /// at most `NAMED`; `more` is how many others there were.
    pub named: Vec<String>,
    pub more: usize,
}

pub const NAMED: usize = 4;

/// Which hundred years before the portal an era year (below 0, -1 - n for n whole years
/// before it) is in: 0 for the year it opened up to 99 years before.
pub fn century_of(year: i64) -> i64 {
    ((-1 - year) / 100).max(0)
}

pub fn overview(w: &World, c: &Chronicle) -> Result<Overview> {
    let mut o = Overview::default();
    for e in eras(w, c)? {
        if e.year >= 0 {
            o.colony.push(e);
            continue;
        }
        let (world, valley): (Vec<_>, Vec<_>) = e
            .counts
            .into_iter()
            .partition(|(k, _)| k.starts_with("realm_"));
        if !valley.is_empty() {
            o.valley.push(Era {
                year: e.year,
                label: e.label.clone(),
                counts: valley,
            });
        }
        if world.is_empty() {
            continue;
        }
        let cy = century_of(e.year);
        if o.world.last().map(|c| century_of(c.year)) != Some(cy) {
            o.world.push(Century {
                year: e.year,
                label: if cy == 0 {
                    "Up to 99 years before the portal".to_string()
                } else {
                    format!("{} to {} years before the portal", cy * 100, cy * 100 + 99)
                },
                counts: Vec::new(),
                named: Vec::new(),
                more: 0,
            });
        }
        let cen = o.world.last_mut().expect("pushed");
        cen.year = cen.year.max(e.year);
        for (k, n) in world {
            match cen.counts.iter_mut().find(|(x, _)| *x == k) {
                Some(p) => p.1 += n,
                None => cen.counts.push((k, n)),
            }
        }
    }
    for cen in &mut o.world {
        cen.counts.sort_by_key(|c| std::cmp::Reverse(c.1));
    }
    // The great events of the wider world, by name (the issue's "Keizou founded", "Tezaang
    // rose"): its Major rows, which are realms and wars, never a single settler's birth.
    let sql = format!(
        "SELECT id, tick, detail FROM real_history WHERE tick < ?1 AND severity >= ?2 \
         AND substr(kind, 1, 6) = 'realm_'{} ORDER BY tick, id",
        crate::view::known::rows_told_sql(w)
    );
    let epoch = w.meta.epoch;
    let mut st = c.conn.prepare(&sql)?;
    let rows = st
        .query_map(
            rusqlite::params![epoch as i64, Severity::Major as i64],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)? as u64,
                    r.get::<_, String>(2)?,
                ))
            },
        )?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    for (id, tick, detail) in rows {
        let year = -1 - ((epoch - tick) / TICKS_PER_YEAR) as i64;
        let cy = century_of(year);
        let Some(cen) = o.world.iter_mut().find(|c| century_of(c.year) == cy) else {
            continue;
        };
        if cen.named.len() >= NAMED {
            cen.more += 1;
            continue;
        }
        if let Some(e) = event_from_row(id, tick, &detail) {
            cen.named.push(crate::view::narrate::line(w, &e));
        }
    }
    o.colony.reverse();
    o.valley.reverse();
    o.world.reverse();
    Ok(o)
}

/// A year's or a century's counts in words, the kinds worth a line only ("born 3, died 1",
/// "settlements founded 12, realms rose 2"); empty when none is. The wider world's many
/// births and deaths are in the Legends browser, not here.
pub fn summary(counts: &[(String, i64)]) -> String {
    let mut parts: Vec<(&str, i64)> = Vec::new();
    for (k, n) in counts {
        let word = match k.as_str() {
            "born" => "born",
            "died" | "old_died" => "died",
            "arrived" => "arrived",
            "left" => "left",
            "joined" => "joined",
            "turned_away" => "turned away",
            "built" => "built",
            "split" => "split",
            "trade" => "trade",
            "custom_formed" => "custom formed",
            "caravan_returned" => "caravan returned",
            "caravan_lost" => "caravan lost",
            "hunted" => "hunted",
            "cave_in" => "cave in",
            "collapse" => "collapse",
            "paired" => "paired",
            "old_founded" => "founded",
            "old_trouble" => "trouble",
            "old_ended" => "left empty",
            "old_answered" => "answered",
            "lived" => "lives lived",
            // The wider world's years (realm/, M3); its many births and deaths
            // are in the Legends browser, not here.
            "realm_founded" => "settlements founded",
            "realm_ended" => "settlements emptied",
            "realm_polity_founded" => "realms rose",
            "realm_polity_ended" => "realms fell",
            "realm_succeeded" => "new rulers",
            "realm_plague" => "plague",
            "realm_famine" => "famine",
            _ => continue,
        };
        match parts.iter_mut().find(|(w, _)| *w == word) {
            Some(p) => p.1 += n,
            None => parts.push((word, *n)),
        }
    }
    let parts: Vec<String> = parts.iter().map(|(w, n)| format!("{w} {n}")).collect();
    parts.join(", ")
}

/// #322: a person's story as the chronicle has it, oldest first: every recorded line that
/// names them, narrated from its row. For a child taken below this reads the pact, the night,
/// the taking, the finding, the going down after them and how it ended, in that order.
pub fn story_of(w: &World, c: &Chronicle, agent: crate::ids::AgentId) -> Result<Vec<String>> {
    let mut st = c.conn.prepare_cached(
        "SELECT r.id, r.tick, r.detail FROM real_history r JOIN entity_ref e ON e.history_id = r.id \
         WHERE e.entity_kind = 'agent' AND e.entity_id = ?1 ORDER BY r.tick, r.id",
    )?;
    let rows = st
        .query_map(rusqlite::params![agent.0 as i64], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)? as u64,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows
        .into_iter()
        .filter_map(|(id, tick, detail)| {
            let e = event_from_row(id, tick, &detail)?;
            Some(crate::view::narrate::line(w, &e))
        })
        .collect())
}

/// The world's history before the portal year by year, for the reel at a new game's start
/// (#561): (realm year, that year's notable lines, oldest first), only years with a line. The
/// same rows the History panel and Legends read (real_history before the arrival, notable and
/// above, what the viewer may be told); a realm year is counted back from the arrival as the
/// realm's own events were written (realm/chronicle.rs `tick_of`).
pub fn reel(w: &World, c: &Chronicle) -> Result<Vec<(i64, Vec<String>)>> {
    let sql = format!(
        "SELECT id, tick, detail FROM real_history WHERE tick < ?1 AND severity >= ?2{} \
         ORDER BY tick, id",
        crate::view::known::rows_told_sql(w)
    );
    let epoch = w.meta.epoch;
    let last = if w.realm.arrived > 0 {
        w.realm.arrived
    } else {
        w.realm.year
    };
    let mut st = c.conn.prepare(&sql)?;
    let rows = st
        .query_map(
            rusqlite::params![epoch as i64, Severity::Notable as i64],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)? as u64,
                    r.get::<_, String>(2)?,
                ))
            },
        )?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut out: Vec<(i64, Vec<String>)> = Vec::new();
    for (id, tick, detail) in rows {
        let back = (epoch - tick).div_ceil(TICKS_PER_YEAR) as i64;
        let year = last - back + 1;
        let Some(e) = event_from_row(id, tick, &detail) else {
            continue;
        };
        let line = crate::view::narrate::line(w, &e);
        match out.last_mut() {
            Some((y, v)) if *y == year => v.push(line),
            _ => out.push((year, vec![line])),
        }
    }
    Ok(out)
}
