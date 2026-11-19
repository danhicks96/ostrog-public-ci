//! The census behind #568 (regret and rationalising): over the opening soaks, how many quarrels
//! there were, how many were judged and which way, the helping lean's range, and how far the
//! opinions of judged pairs moved; enough to see that judgements do not swing anyone without
//! bound. Sampled daily; every living person.
//!
//! Run: cargo run --release -p hf-sim --example conscience_census [days] [seeds, e.g. 1,2,3,4]
use hf_sim::events::EventKind as E;
use hf_sim::time::TICKS_PER_DAY;
use hf_sim::World;

#[derive(Default, Debug)]
struct Tally {
    quarrels: u64,
    regret: u64,
    rationalise: u64,
    /// Lowest and highest helping lean seen at a daily sample.
    help_lo: f32,
    help_hi: f32,
    /// Opinions of the judged at the daily samples: lowest and highest.
    op_lo: f32,
    op_hi: f32,
    /// Most judgements one person held at once, and most leanings.
    most_judged: usize,
    most_leanings: usize,
    /// Tend choices made by one leaning toward the sick person (amends or more willing).
    amends: u64,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let days: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(120);
    let seeds: Vec<u64> = args
        .get(2)
        .map(|s| s.split(',').filter_map(|x| x.parse().ok()).collect())
        .unwrap_or_else(|| vec![1, 2, 3, 4]);
    for seed in seeds {
        let mut w = World::opening_world(seed);
        let mut t = Tally {
            help_lo: 1.0,
            help_hi: 1.0,
            ..Default::default()
        };
        for k in 0..days * TICKS_PER_DAY {
            w.step();
            for e in &w.last_events {
                match &e.kind {
                    E::Quarrelled { .. } => t.quarrels += 1,
                    E::Chose { what, .. } if what.starts_with("regretted quarrelling") => {
                        t.regret += 1
                    }
                    E::Chose { what, .. } if what.starts_with("told themselves") => {
                        t.rationalise += 1
                    }
                    _ => {}
                }
            }
            if k % TICKS_PER_DAY != 0 {
                continue;
            }
            let now = w.meta.tick;
            for a in w.agents.iter().filter(|a| a.alive) {
                t.most_judged = t.most_judged.max(a.conscience.judgements.len());
                t.most_leanings = t.most_leanings.max(a.conscience.leanings.len());
                for l in &a.conscience.leanings {
                    let h = a.conscience.help_toward(l.toward, now);
                    t.help_lo = t.help_lo.min(h);
                    t.help_hi = t.help_hi.max(h);
                    let op = a.opinion_of(l.toward);
                    t.op_lo = t.op_lo.min(op);
                    t.op_hi = t.op_hi.max(op);
                }
                if let hf_sim::mind::Activity::Working { plan } = &a.activity {
                    if plan.label.starts_with("tend")
                        && a.conscience.leanings.iter().any(|l| l.help > 1.0)
                    {
                        t.amends += 1;
                    }
                }
            }
        }
        println!(
            "CONSCIENCE seed {seed} days {days}: quarrels {} judged {} (regret {}, rationalise {}); help {:.2}..{:.2}; judged opinions {:+.2}..{:+.2}; most judgements held {} leanings {}; tend days while leaning {}",
            t.quarrels,
            t.regret + t.rationalise,
            t.regret,
            t.rationalise,
            t.help_lo,
            t.help_hi,
            t.op_lo,
            t.op_hi,
            t.most_judged,
            t.most_leanings,
            t.amends
        );
    }
}
