//! The debug decision log (#565): JSON Lines whose header is enough to rebuild the run; off, it
//! writes nothing; it changes nothing in the run; and "why did agent N do X at tick T, and what
//! else could they have done" can be answered from the file alone. The log is process-wide
//! state, so everything is one test.

use hf_sim::time::TICKS_PER_DAY;
use hf_sim::world::gen::GenParams;
use hf_sim::World;
use serde_json::Value;
use std::path::Path;

const SEED: u64 = 77;

fn build(seed: u64, small: bool, agents: usize) -> World {
    let mut w = World::generate(
        seed,
        if small {
            GenParams::small()
        } else {
            GenParams::default()
        },
    );
    w.found_party(agents);
    w
}

fn run(ticks: u64) -> String {
    let mut w = build(SEED, true, 12);
    for _ in 0..ticks {
        w.step();
    }
    hf_sim::digest::digest(&w)
}

fn lines(path: &Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("not JSON ({e}): {l}")))
        .collect()
}

fn settings() -> Vec<(String, String)> {
    vec![
        ("small".into(), "true".into()),
        ("agents".into(), "12".into()),
    ]
}

#[test]
fn log_header_off_means_nothing_digest_unchanged_and_the_log_answers_why() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("debug.jsonl");
    let ticks = TICKS_PER_DAY + 200;

    // Off: nothing is written.
    assert!(!hf_sim::debuglog::enabled());
    let off = run(ticks);
    assert!(!file.exists(), "a run with the log off wrote a file");

    // On: the same digest, and a file.
    hf_sim::debuglog::configure(file.clone(), settings(), "test --seed 77".into());
    let on = run(ticks);
    hf_sim::debuglog::disable();
    assert_eq!(off, on, "the log changed the run");
    let recs = lines(&file);

    // The header is the first object and carries what a rerun needs.
    let h = &recs[0];
    assert_eq!(h["t"], "header");
    assert_eq!(h["seed"], SEED);
    assert!(!h["commit"].as_str().unwrap().is_empty());
    assert_eq!(h["game_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(h["command_line"], "test --seed 77");
    assert_eq!(h["settings"]["small"], "true");
    assert_eq!(h["settings"]["agents"], "12");
    for k in ["region", "history_years", "world"] {
        assert!(h.get(k).is_some(), "header lacks {k}");
    }

    // One `day` line for the one full day, and its counters equal the records before it.
    let days: Vec<&Value> = recs.iter().filter(|r| r["t"] == "day").collect();
    assert_eq!(days.len(), 1);
    let before: Vec<&Value> = recs.iter().take_while(|r| r["t"] != "day").collect();
    let count = |t: &str| before.iter().filter(|r| r["t"] == t).count() as u64;
    assert_eq!(days[0]["plans_made"], count("decide"));
    assert_eq!(days[0]["plans_failed"], count("plan_failed"));
    assert_eq!(days[0]["plans_abandoned"], count("abandoned"));
    assert!(count("decide") > 0);
    assert!(days[0]["need_crossings"].get("starving").is_some());

    // Failed and abandoned records carry a reason.
    for r in recs
        .iter()
        .filter(|r| r["t"] == "plan_failed" || r["t"] == "abandoned")
    {
        assert!(!r["reason"].as_str().unwrap().is_empty(), "{r}");
    }

    // Acceptance: take a decision with alternatives and answer from the log alone.
    let d = recs
        .iter()
        .find(|r| r["t"] == "decide" && r["options"].as_array().is_some_and(|o| o.len() >= 2))
        .expect("no decision with alternatives");
    let (agent, tick) = (d["agent"].as_u64().unwrap(), d["tick"].as_u64().unwrap());
    let options = d["options"].as_array().unwrap();
    let chose = d["chose"].as_str().unwrap();
    let top = options.iter().find(|o| o["available"] == true).unwrap();
    assert_eq!(
        top["goal"], chose,
        "the chosen goal is the best available option"
    );
    let others: Vec<&Value> = options.iter().filter(|o| o["goal"] != chose).collect();
    assert!(
        !others.is_empty(),
        "agent {agent} at {tick}: no alternatives recorded"
    );
    assert!(options.iter().all(|o| o["score"].is_number()));
    assert!(d["why"].as_str().is_some());

    // And the header is enough to reproduce: rebuild from it alone and find the same decision.
    let again = dir.path().join("again.jsonl");
    let seed = h["seed"].as_u64().unwrap();
    let small = h["settings"]["small"] == "true";
    let n: usize = h["settings"]["agents"].as_str().unwrap().parse().unwrap();
    hf_sim::debuglog::configure(again.clone(), settings(), String::new());
    let mut w = build(seed, small, n);
    for _ in 0..tick {
        w.step();
    }
    hf_sim::debuglog::disable();
    let second = lines(&again);
    let same = second
        .iter()
        .find(|r| r["t"] == "decide" && r["tick"] == tick && r["agent"] == agent)
        .expect("the rerun did not decide the same");
    assert_eq!(same, d, "the rerun decided differently");

    // Bounded: a tiny cap rotates to `.1` and the new file has its own header.
    let capped = dir.path().join("capped.jsonl");
    hf_sim::debuglog::configure_capped(capped.clone(), vec![], String::new(), 3_000);
    run(300);
    hf_sim::debuglog::disable();
    assert!(dir.path().join("capped.jsonl.1").exists(), "no rotation");
    assert_eq!(lines(&capped)[0]["t"], "header");
    assert!(std::fs::metadata(&capped).unwrap().len() < 30_000);
}
