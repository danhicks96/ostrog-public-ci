//! "While you were away": counts and the major lines since a tick, from the chronicle.

use anyhow::Result;

use crate::chronicle::Chronicle;
use crate::events::Severity;
use crate::world_state::World;

pub struct Summary {
    pub from_tick: u64,
    pub to_tick: u64,
    pub counts: Vec<(String, i64)>,
    /// (history_id, tick, line json) of major events, newest first.
    pub major: Vec<(i64, u64, String)>,
}

impl Summary {
    /// The counts in words, the kinds worth a line only ("died 2, born 1"); routine work kinds
    /// (hauled, harvested) are left out. `counts` itself is unchanged for existing consumers.
    pub fn counts_in_words(&self) -> String {
        crate::view::history::summary(&self.counts)
    }
}

/// Only rows the viewer may be told of (view::known).
pub fn since(
    w: &World,
    c: &Chronicle,
    from_tick: u64,
    to_tick: u64,
    max_major: usize,
) -> Result<Summary> {
    since_i64(w, c, from_tick as i64, to_tick as i64, max_major)
}

/// `since` over `(from, to]` with signed bounds, so a span can start before tick zero and still
/// take a row at tick zero.
fn since_i64(
    w: &World,
    c: &Chronicle,
    from_tick: i64,
    to_tick: i64,
    max_major: usize,
) -> Result<Summary> {
    let told = crate::view::known::rows_told_sql(w);
    let mut st = c.conn.prepare(&format!("SELECT kind, COUNT(*) FROM real_history WHERE tick > ?1 AND tick <= ?2{told} GROUP BY kind ORDER BY COUNT(*) DESC"))?;
    let counts = st
        .query_map(rusqlite::params![from_tick, to_tick], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut st2 = c.conn.prepare(&format!("SELECT id, tick, detail FROM real_history WHERE tick > ?1 AND tick <= ?2 AND severity >= ?3{told} ORDER BY id DESC LIMIT ?4"))?;
    let major = st2
        .query_map(
            rusqlite::params![
                from_tick,
                to_tick,
                Severity::Notable as i64,
                max_major as i64
            ],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)? as u64,
                    r.get::<_, String>(2)?,
                ))
            },
        )?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(Summary {
        from_tick: from_tick.max(0) as u64,
        to_tick: to_tick.max(0) as u64,
        counts,
        major,
    })
}

/// The kinds that change who is in the valley or what they are, most weighty first (#1443).
/// There is no recorded "parted" row yet; add it here, at its weight, when one exists.
const CHANGE_KINDS: [&str; 12] = [
    "died",
    "born",
    "paired",
    "arrived",
    "joined",
    "left",
    "exiled",
    "captured",
    "returned",
    "broke",
    "craft_lost",
    "role",
];

/// One life event of the catch-up card: who, the row it comes from, when, and its line.
pub struct Change {
    /// The first person the row names (a pairing names the first of the two); `None` for a row
    /// about no one in particular (a craft lost to a community).
    pub agent: Option<crate::ids::AgentId>,
    pub history_id: i64,
    pub tick: u64,
    pub line: String,
}

/// #1443: the life events between two ticks (from, to], the ones to tell first on coming back:
/// deaths, births, pairings, arrivals, and so on through `CHANGE_KINDS`, ordered by that list,
/// then by tick. Every line is narrated from a recorded row (D9), and only rows the viewer may
/// be told of (view::known) are read. A role counts only the first time the person took it.
/// Not wired into the card yet (#1445).
pub fn changes(w: &World, c: &Chronicle, from: u64, to: u64) -> Result<Vec<Change>> {
    changes_i64(w, c, from as i64, to as i64)
}

fn changes_i64(w: &World, c: &Chronicle, from: i64, to: i64) -> Result<Vec<Change>> {
    let kinds: Vec<String> = CHANGE_KINDS.iter().map(|k| format!("'{k}'")).collect();
    let sql = format!(
        "SELECT id, tick, kind, detail FROM real_history WHERE tick > ?1 AND tick <= ?2 \
         AND kind IN ({}) AND (kind <> 'role' OR NOT EXISTS (SELECT 1 FROM real_history p \
         WHERE p.kind = 'role' AND p.detail = real_history.detail AND p.id < real_history.id))\
         {} ORDER BY tick, id",
        kinds.join(","),
        crate::view::known::rows_told_sql(w)
    );
    let mut st = c.conn.prepare(&sql)?;
    let rows = st
        .query_map(rusqlite::params![from, to], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)? as u64,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut out: Vec<(usize, Change)> = rows
        .into_iter()
        .filter_map(|(id, tick, kind, detail)| {
            let e = crate::view::history::event_from_row(id, tick, &detail)?;
            let rank = CHANGE_KINDS.iter().position(|k| *k == kind)?;
            Some((
                rank,
                Change {
                    agent: e.participants().first().copied(),
                    history_id: id,
                    tick,
                    line: crate::view::narrate::line(w, &e),
                },
            ))
        })
        .collect();
    out.sort_by_key(|(rank, ch)| (*rank, ch.tick, ch.history_id));
    Ok(out.into_iter().map(|(_, ch)| ch).collect())
}

/// One notable detail in Catch Up. The line and anchor come from the newest history row;
/// the span records the event times, which can be out of order relative to history IDs.
#[derive(Debug, PartialEq, Eq)]
pub struct CatchUpDetail {
    pub history_id: i64,
    pub anchor_tick: u64,
    pub earliest_tick: u64,
    pub latest_tick: u64,
    pub count: usize,
    pub line: String,
}

/// Read the viewer-visible Notable+ rows in `(from, to]` for Catch Up, folding repeat keys
/// before limiting the number of distinct details. Named changes and raw markers are separate.
pub fn catch_up_details(
    w: &World,
    c: &Chronicle,
    from: u64,
    to: u64,
    limit: usize,
) -> Result<Vec<CatchUpDetail>> {
    catch_up_details_i64(w, c, from as i64, to as i64, limit)
}

fn catch_up_details_i64(
    w: &World,
    c: &Chronicle,
    from: i64,
    to: i64,
    limit: usize,
) -> Result<Vec<CatchUpDetail>> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    let mut st = c.conn.prepare(&format!(
        "SELECT id, tick, detail FROM real_history WHERE tick > ?1 AND tick <= ?2 \
         AND severity >= ?3{} ORDER BY id DESC",
        crate::view::known::rows_told_sql(w)
    ))?;
    let mut rows = st.query(rusqlite::params![from, to, Severity::Notable as i64])?;
    let mut details: Vec<CatchUpDetail> = Vec::new();
    let mut by_key = std::collections::BTreeMap::<String, usize>::new();
    while let Some(row) = rows.next()? {
        let id = row.get::<_, i64>(0)?;
        let tick = row.get::<_, i64>(1)? as u64;
        let detail: String = row.get(2)?;
        let Some(event) = crate::view::history::event_from_row(id, tick, &detail) else {
            continue;
        };
        let key = crate::view::fold::key(&event);
        if let Some(&index) = key.as_ref().and_then(|key| by_key.get(key)) {
            let detail = &mut details[index];
            detail.earliest_tick = detail.earliest_tick.min(tick);
            detail.latest_tick = detail.latest_tick.max(tick);
            detail.count += 1;
            continue;
        }

        // New keys below the cap cannot displace an earlier group: rows arrive in descending
        // ID order. Keep scanning so every repeat of a retained key contributes to its span.
        if details.len() == limit {
            continue;
        }
        if let Some(key) = key {
            by_key.insert(key, details.len());
        }
        details.push(CatchUpDetail {
            history_id: id,
            anchor_tick: tick,
            earliest_tick: tick,
            latest_tick: tick,
            count: 1,
            line: crate::view::narrate::line(w, &event),
        });
    }
    Ok(details)
}

/// The Catch Up card: a headline in words, the named changes, then the folded details. Each
/// part comes from its own read model; the raw `Summary::counts` are untouched.
pub struct CatchUpCard {
    /// Whether there is news: a named change or a notable detail. Counts alone are not news.
    pub has_news: bool,
    pub headline: String,
    pub changes: Vec<String>,
    pub details: Vec<String>,
}

impl CatchUpCard {
    /// The card as the client shows it, in reading order: the headline, the named changes,
    /// then the folded details. A part with nothing to say is left out. A folded line says how
    /// many times it happened.
    pub fn bbcode(&self) -> String {
        let mut out = String::new();
        if !self.headline.is_empty() {
            out.push_str(&self.headline);
            out.push_str("\n\n");
        }
        if !self.changes.is_empty() {
            out.push_str("[b]What changed[/b]\n");
            for l in &self.changes {
                out.push_str(l);
                out.push('\n');
            }
            out.push('\n');
        }
        if !self.details.is_empty() {
            out.push_str("[b]Also noted[/b]\n");
            for l in &self.details {
                out.push_str(l);
                out.push('\n');
            }
        }
        out
    }
}

/// Build the Catch Up card for `(from, to]`: the counts in words, the life events by name, and
/// at most `detail_limit` folded details. Only rows the viewer may be told of are read.
pub fn catch_up_card(
    w: &World,
    c: &Chronicle,
    from: u64,
    to: u64,
    detail_limit: usize,
) -> Result<CatchUpCard> {
    catch_up_card_between(w, c, from as i64, to as i64, detail_limit)
}

/// The card for rows after `from` up to and including `to`, with signed bounds so a span that
/// starts at tick zero can take a row at tick zero (`from` is then -1).
pub fn catch_up_card_between(
    w: &World,
    c: &Chronicle,
    from: i64,
    to: i64,
    detail_limit: usize,
) -> Result<CatchUpCard> {
    let sum = since_i64(w, c, from, to, 0)?;
    let named = changes_i64(w, c, from, to)?;
    let named_ids: Vec<i64> = named.iter().map(|ch| ch.history_id).collect();
    let changes: Vec<String> = named.into_iter().map(|ch| ch.line).collect();
    // A row already told by name is not told again below it.
    let details: Vec<String> =
        catch_up_details_i64(w, c, from, to, detail_limit + named_ids.len())?
            .into_iter()
            .filter(|d| !named_ids.contains(&d.history_id))
            .take(detail_limit)
            .map(|d| {
                if d.count > 1 {
                    format!("{} (x{})", d.line, d.count)
                } else {
                    d.line
                }
            })
            .collect();
    Ok(CatchUpCard {
        has_news: !changes.is_empty() || !details.is_empty(),
        headline: sum.counts_in_words(),
        changes,
        details,
    })
}

/// The spans the viewer can ask Catch Up for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatchUpSpan {
    PreviousDay,
    CurrentDay,
    PreviousSeason,
    CurrentSeason,
}

impl CatchUpSpan {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "previous_day" => Some(Self::PreviousDay),
            "current_day" => Some(Self::CurrentDay),
            "previous_season" => Some(Self::PreviousSeason),
            "current_season" => Some(Self::CurrentSeason),
            _ => None,
        }
    }
}

/// The rows after `.0` up to and including `.1` that a span covers at tick `now` in a world whose
/// calendar starts at `epoch`. A current span runs from its first tick (inclusive) to now; a
/// previous span ends on the tick before the current one starts, so the new day or season's
/// boundary row belongs only to the current span. `None` when there is no previous span: at the
/// start there is no invented earlier day or season, and nothing underflows.
pub fn catch_up_span_bounds(epoch: u64, now: u64, span: CatchUpSpan) -> Option<(i64, i64)> {
    use crate::time::{DAYS_PER_SEASON, TICKS_PER_DAY};
    let unit = match span {
        CatchUpSpan::PreviousDay | CatchUpSpan::CurrentDay => TICKS_PER_DAY,
        CatchUpSpan::PreviousSeason | CatchUpSpan::CurrentSeason => TICKS_PER_DAY * DAYS_PER_SEASON,
    };
    let rel = now.saturating_sub(epoch);
    let start = epoch + rel / unit * unit;
    match span {
        CatchUpSpan::CurrentDay | CatchUpSpan::CurrentSeason => {
            Some((start as i64 - 1, now as i64))
        }
        CatchUpSpan::PreviousDay | CatchUpSpan::PreviousSeason => {
            if start < epoch + unit {
                return None;
            }
            Some((start as i64 - unit as i64 - 1, start as i64 - 1))
        }
    }
}

/// The Catch Up card for a span ending now; `None` for a previous span that does not exist yet.
pub fn catch_up_card_span(
    w: &World,
    c: &Chronicle,
    span: CatchUpSpan,
    detail_limit: usize,
) -> Result<Option<CatchUpCard>> {
    let Some((from, to)) = catch_up_span_bounds(w.meta.epoch, w.tick(), span) else {
        return Ok(None);
    };
    catch_up_card_between(w, c, from, to, detail_limit).map(Some)
}
