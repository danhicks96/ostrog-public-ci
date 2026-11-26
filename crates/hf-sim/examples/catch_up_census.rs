//! #403 (OST-862): does the catch-up say every death, birth, pairing, arrival and departure
//! by name? For each 8-game-hour window of an opening world, the chronicle's told rows of the
//! kinds that change who is in the valley are compared with `summary::changes` (each row must
//! come back as a line, naming its person), and the folded details are checked so that no kind
//! fills more than 3 of the lines.
//! usage: catch_up_census [first_seed last_seed days]   (default: 1 8 100)
//! Prints one line per seed, and the first few misses. Exits non-zero when any row has no line or
//! lacks its person's name; a kind over 3 folded lines is reported, not failed.
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};
use hf_sim::view::summary;
use hf_sim::Session;

const KINDS: [&str; 5] = ["died", "born", "paired", "arrived", "left"];
const WINDOW: u64 = 8 * TICKS_PER_HOUR;

fn main() -> anyhow::Result<()> {
    let a: Vec<u64> = std::env::args()
        .skip(1)
        .filter_map(|x| x.parse().ok())
        .collect();
    let (first, last, days) = match a.as_slice() {
        [f, l, d] => (*f, *l, *d),
        _ => (1, 8, 100),
    };
    let mut bad = 0u64;
    for seed in first..=last {
        let setup = hf_sim::setup::WorldSetup::new(seed);
        let world = hf_sim::world_state::World::from_setup(&setup);
        let mut s = Session::new(world, hf_sim::chronicle::Chronicle::working()?);
        s.write_pending()?;
        let (mut windows, mut rows, mut missing, mut unnamed, mut crowded) =
            (0u64, 0u64, 0u64, 0u64, 0u64);
        let mut shown = 0;
        // Windows start where play starts; the realm's backstory rows come before it.
        let mut from = s.world.meta.tick;
        for _ in 0..days * TICKS_PER_DAY {
            s.step()?;
            let now = s.world.meta.tick;
            if now < from + WINDOW {
                continue;
            }
            s.write_pending()?;
            windows += 1;
            let told = hf_sim::view::known::rows_told_sql(&s.world);
            let list = KINDS.map(|k| format!("'{k}'")).join(",");
            let sql = format!(
                "SELECT id, kind FROM real_history WHERE tick > ?1 AND tick <= ?2 AND kind IN ({list}){told}"
            );
            let want: Vec<(i64, String)> = {
                let mut st = s.chronicle.conn.prepare(&sql)?;
                let v = st
                    .query_map(rusqlite::params![from as i64, now as i64], |r| {
                        Ok((r.get(0)?, r.get(1)?))
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                v
            };
            let got = summary::changes(&s.world, &s.chronicle, from, now)?;
            for (id, kind) in &want {
                rows += 1;
                match got.iter().find(|c| c.history_id == *id) {
                    None => {
                        missing += 1;
                        if shown < 5 {
                            shown += 1;
                            println!("  seed {seed} tick {now}: {kind} row {id} has no line");
                        }
                    }
                    Some(c) => {
                        if let Some(p) = c.agent {
                            let named = s
                                .world
                                .agents
                                .iter()
                                .chain(s.world.overworld.away.iter())
                                .find(|x| x.id == p)
                                .is_none_or(|x| c.line.contains(&x.name));
                            if !named {
                                unnamed += 1;
                            }
                        }
                    }
                }
            }
            // Folded details: the kind of each line is that of its row.
            let details = summary::catch_up_details(&s.world, &s.chronicle, from, now, usize::MAX)?;
            let mut by_kind: std::collections::BTreeMap<String, u32> = Default::default();
            for d in &details {
                let k: String = s.chronicle.conn.query_row(
                    "SELECT kind FROM real_history WHERE id = ?1",
                    [d.history_id],
                    |r| r.get(0),
                )?;
                *by_kind.entry(k).or_default() += 1;
            }
            if by_kind.values().any(|n| *n > 3) {
                crowded += 1;
                if shown < 5 {
                    shown += 1;
                    println!("  seed {seed} tick {now}: folded lines by kind {by_kind:?}");
                }
            }
            from = now;
        }
        println!(
            "seed {seed}: {days} days, {windows} windows, {rows} rows, {missing} missing, {unnamed} unnamed, {crowded} windows with a kind over 3 folded lines"
        );
        bad += missing + unnamed;
    }
    if bad > 0 {
        std::process::exit(1);
    }
    Ok(())
}
