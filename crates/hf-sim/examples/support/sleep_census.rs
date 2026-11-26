//! Read-only counts of completed sleeps, kept outside simulation and save state.

use std::collections::BTreeMap;

use hf_sim::ids::{AgentId, StructureId};
use hf_sim::time::{TICKS_PER_DAY, TICKS_PER_HOUR};

#[derive(Default)]
struct Person {
    beds: BTreeMap<StructureId, u64>,
    windows: Vec<BedWindow>,
    rough: u64,
    last_end: Option<u64>,
}

/// One person's consecutive blocks of ten completed nights; the last can be partial.
#[derive(Default)]
pub struct BedWindow {
    pub nights: u64,
    pub beds: std::collections::BTreeSet<StructureId>,
}

#[derive(Default)]
pub struct SleepCensus {
    people: BTreeMap<AgentId, Person>,
    bed_nights: u64,
    unknown_bed_nights: u64,
    taken: u64,
    finished_streaks: [u64; 4],
    longest: Option<(AgentId, u64)>,
    unobserved_breaks: u64,
}

pub struct Summary<'a> {
    pub bed_nights: u64,
    pub most_used_nights: u64,
    pub unknown_bed_nights: u64,
    pub taken: u64,
    /// Episodes, including a person's unfinished streak at the end of the run.
    pub streaks: [u64; 4],
    pub longest: Option<(AgentId, u64)>,
    /// Rough runs split by a whole unobserved game-day between qualifying sleeps.
    pub unobserved_breaks: u64,
    pub windows: Vec<(AgentId, &'a BedWindow)>,
}

fn band(nights: u64) -> usize {
    match nights {
        0 => unreachable!("a rough episode has at least one night"),
        1 => 0,
        2..=3 => 1,
        4..=7 => 2,
        _ => 3,
    }
}

impl SleepCensus {
    /// Called for the same six-hour-or-longer Slept events as the existing census.
    /// A bed-night with an unavailable structure id stays in the denominator.
    pub fn night(
        &mut self,
        who: AgentId,
        in_bed: bool,
        bed: Option<StructureId>,
        ended: u64,
        hours: f32,
    ) {
        let p = self.people.entry(who).or_default();
        let duration = (f64::from(hours) * TICKS_PER_HOUR as f64).round() as u64;
        let started = ended.saturating_sub(duration);
        if p.rough > 0
            && p.last_end
                .is_some_and(|last| started.saturating_sub(last) >= TICKS_PER_DAY)
        {
            self.finished_streaks[band(p.rough)] += 1;
            p.rough = 0;
            self.unobserved_breaks += 1;
        }
        p.last_end = Some(ended);
        if p.windows.last().is_none_or(|window| window.nights == 10) {
            p.windows.push(BedWindow::default());
        }
        let window = p.windows.last_mut().unwrap();
        window.nights += 1;
        if in_bed {
            self.bed_nights += 1;
            if let Some(bed) = bed {
                *p.beds.entry(bed).or_default() += 1;
                window.beds.insert(bed);
            } else {
                self.unknown_bed_nights += 1;
            }
            if p.rough > 0 {
                self.finished_streaks[band(p.rough)] += 1;
                p.rough = 0;
            }
        } else {
            p.rough += 1;
            if self.longest.is_none_or(|(other, nights)| {
                p.rough > nights || (p.rough == nights && who < other)
            }) {
                self.longest = Some((who, p.rough));
            }
        }
    }

    pub fn failure(&mut self, reason: &str) {
        self.taken += u64::from(reason == "the bed was taken");
    }

    pub fn summary(&self) -> Summary<'_> {
        let mut streaks = self.finished_streaks;
        for p in self.people.values().filter(|p| p.rough > 0) {
            streaks[band(p.rough)] += 1;
        }
        Summary {
            bed_nights: self.bed_nights,
            most_used_nights: self
                .people
                .values()
                .map(|p| p.beds.values().copied().max().unwrap_or(0))
                .sum(),
            unknown_bed_nights: self.unknown_bed_nights,
            taken: self.taken,
            streaks,
            longest: self.longest,
            unobserved_breaks: self.unobserved_breaks,
            windows: self
                .people
                .iter()
                .flat_map(|(who, p)| p.windows.iter().map(move |window| (*who, window)))
                .collect(),
        }
    }
}
