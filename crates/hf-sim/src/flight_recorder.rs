//! Bounded, local diagnostics storage. It never reads or changes simulation state.
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

const SEGMENT_BYTES: u64 = 8 * 1024 * 1024;
const SUMMARY_BYTES: u64 = 8 * 1024 * 1024;
const SEGMENTS: usize = 4;

struct Segment {
    path: PathBuf,
    first_tick: u64,
    first_seq: u64,
    last_tick: u64,
    bytes: u64,
}

pub(crate) struct Recorder {
    dir: PathBuf,
    survey: Option<File>,
    survey_bytes: u64,
    segments: VecDeque<Segment>,
    trace: Option<BufWriter<File>>,
    next_segment: u64,
    seq: u64,
    segment_bytes: u64,
    segment_limit: usize,
    trace_failed: bool,
    lost_from_tick: Option<u64>,
    last_tick: u64,
}

impl Recorder {
    pub(crate) fn create(dir: &Path) -> io::Result<Self> {
        Self::create_capped(dir, SEGMENT_BYTES, SEGMENTS)
    }

    fn create_capped(dir: &Path, segment_bytes: u64, segment_limit: usize) -> io::Result<Self> {
        assert!(segment_bytes > 0 && segment_limit > 0);
        // Require a fresh run directory. Never alter another run or a save directory.
        fs::create_dir(dir)?;
        let survey = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(dir.join("survey.jsonl"))?;
        Ok(Self {
            dir: dir.to_path_buf(),
            survey: Some(survey),
            survey_bytes: 0,
            segments: VecDeque::new(),
            trace: None,
            next_segment: 0,
            seq: 0,
            segment_bytes,
            segment_limit,
            trace_failed: false,
            lost_from_tick: None,
            last_tick: 0,
        })
    }

    fn survey_line(&mut self, v: &Value) {
        let Some(out) = &mut self.survey else { return };
        let line = format!("{v}\n");
        let result = if self.survey_bytes + line.len() as u64 > SUMMARY_BYTES {
            Err(io::Error::other("summary budget exhausted"))
        } else {
            out.write_all(line.as_bytes())
        };
        match result {
            Ok(()) => self.survey_bytes += line.len() as u64,
            Err(err) => {
                eprintln!("flight recorder: survey incomplete: {err}");
                self.survey = None;
            }
        }
    }

    fn new_segment(&mut self, tick: u64) -> io::Result<()> {
        self.flush_trace()?;
        if self.segments.len() == self.segment_limit {
            let old = self.segments.front().expect("segment exists");
            fs::remove_file(&old.path)?;
            let old = self.segments.pop_front().expect("segment exists");
            self.survey_line(&json!({
                "t": "evicted", "from_tick": old.first_tick, "through_tick": old.last_tick,
            }));
        }
        let path = self
            .dir
            .join(format!("trace-{:010}.jsonl", self.next_segment));
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        self.next_segment += 1;
        self.trace = Some(BufWriter::new(file));
        self.segments.push_back(Segment {
            path,
            first_tick: tick,
            first_seq: self.seq,
            last_tick: tick,
            bytes: 0,
        });
        Ok(())
    }

    fn flush_trace(&mut self) -> io::Result<()> {
        if let Some(trace) = &mut self.trace {
            trace.flush()?;
        }
        Ok(())
    }

    fn trace_error(&mut self, tick: u64, err: io::Error) {
        if !self.trace_failed {
            self.trace_failed = true;
            // Flush errors can taint previously buffered rows, so mark the whole active
            // segment conservatively. Do not claim all preceding buffered lines survived.
            let first = self
                .segments
                .back()
                .map_or(tick, |segment| segment.first_tick);
            self.lost_from_tick = Some(first);
            self.survey_line(&json!({"t": "trace_incomplete", "from_tick": first}));
            eprintln!("flight recorder: trace incomplete from tick {first}: {err}");
        }
    }

    pub(crate) fn flush(&mut self) {
        if self.trace_failed {
            return;
        }
        if let Err(err) = self.flush_trace() {
            self.trace_error(self.last_tick, err);
        }
    }

    pub(crate) fn record(&mut self, tick: u64, mut value: Value) {
        self.last_tick = tick;
        match value["t"].as_str() {
            Some("header") | Some("day") => {
                self.flush();
                self.survey_line(&value);
                return;
            }
            _ if self.trace_failed => return,
            _ => {}
        }
        value["seq"] = self.seq.into();
        let line = format!("{value}\n");
        let len = line.len() as u64;
        if len > self.segment_bytes {
            self.trace_error(
                tick,
                io::Error::other("trace record exceeds segment budget"),
            );
            return;
        }
        if self
            .segments
            .back()
            .is_none_or(|s| s.bytes + len > self.segment_bytes)
        {
            if let Err(err) = self.new_segment(tick) {
                self.trace_error(tick, err);
                return;
            }
        }
        let trace = self.trace.as_mut().expect("active segment exists");
        if let Err(err) = trace.write_all(line.as_bytes()) {
            self.trace_error(tick, err);
            return;
        }
        let segment = self.segments.back_mut().expect("active segment exists");
        segment.bytes += len;
        segment.last_tick = tick;
        self.seq += 1;
    }

    pub(crate) fn finish(&mut self) {
        self.flush();
        self.survey_line(&json!({
            "t": "closed", "last_observed_tick": self.last_tick, "raw_incomplete_from": self.lost_from_tick,
            "raw_retained_first_seq": self.segments.front().map_or(self.seq, |segment| segment.first_seq),
            "raw_next_seq": self.seq,
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flush_failure_is_visible_without_losing_daily_summaries() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("capture");
        let mut rec = Recorder::create_capped(&dir, 1024, 2).unwrap();
        rec.record(0, json!({"t": "header", "seed": 7}));
        rec.record(1, json!({"t": "path_attempt", "tick": 1}));
        rec.flush();
        let raw = rec.segments.front().unwrap().path.clone();
        rec.trace = Some(BufWriter::new(File::open(raw).unwrap()));
        rec.record(2, json!({"t": "path_attempt", "tick": 2}));
        rec.flush();
        assert!(rec.trace_failed);
        rec.record(3, json!({"t": "day", "day": 1}));
        rec.finish();
        let summary = fs::read_to_string(dir.join("survey.jsonl")).unwrap();
        assert!(summary.contains(r#""from_tick":1,"t":"trace_incomplete""#));
        assert!(summary.contains(r#""day":1"#));
        assert!(summary.contains(r#""raw_incomplete_from":1"#));
    }

    #[test]
    fn rotation_retains_survey_and_marks_trace_loss() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("capture");
        let mut rec = Recorder::create_capped(&dir, 60, 2).unwrap();
        rec.record(0, json!({"t": "header", "seed": 7}));
        rec.record(1, json!({"t": "day", "day": 1}));
        for tick in 2..12 {
            rec.record(tick, json!({"t": "path_attempt", "tick": tick}));
        }
        assert!(rec.next_segment > 4);
        assert_eq!(rec.segments.len(), 2);
        rec.record(12, json!({"t": "huge", "too_large": "x".repeat(100)}));
        rec.record(13, json!({"t": "day", "day": 2}));
        rec.finish();
        let summary = fs::read_to_string(dir.join("survey.jsonl")).unwrap();
        for marker in [
            "header",
            "evicted",
            "trace_incomplete",
            "closed",
            "\"day\":1",
            "\"day\":2",
        ] {
            assert!(summary.contains(marker), "{marker}: {summary}");
        }
        assert!(Recorder::create(&dir).is_err());
        assert_eq!(fs::read_dir(dir).unwrap().count(), 3);
    }
}
