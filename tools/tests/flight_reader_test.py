"""Run the capture reader as another process and verify it never modifies evidence."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

READER = Path(__file__).absolute().parents[1] / "soak" / "read_flight.py"


class FlightReaderTest(unittest.TestCase):
    def test_retained_summary_and_reported_coverage(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            survey = root / "survey.jsonl"
            trace = root / "trace-0000000005.jsonl"
            survey.write_text("".join(json.dumps(row) + "\n" for row in [
                {"t": "header", "seed": 7},
                {"t": "day", "day": 1, "tick": 10},
                {"t": "evicted", "from_tick": 1, "through_tick": 6},
                {"t": "trace_incomplete", "from_tick": 8},
                {"t": "closed", "last_observed_tick": 10},
            ]))
            trace.write_text('{"t":"path_attempt","agent":5,"tick":8,"seq":3}\n{"partial"')
            saved = [p.read_bytes() for p in (survey, trace)]

            def run(*args):
                return json.loads(subprocess.check_output(
                    [sys.executable, "-I", str(READER), str(root), *args],
                    text=True,
                ))

            result = run("--agent", "5", "--start", "8")
            self.assertEqual(len(result["records"]), 1)
            self.assertTrue(result["recording_may_be_incomplete"])
            self.assertEqual(result["coverage_notices"][0]["through_tick"], 6)
            self.assertEqual(result["issues"][0]["reason"], "oversized or unfinished record")
            day = run("--summary")
            self.assertEqual(day["records"][0]["day"], 1)
            self.assertEqual(saved, [p.read_bytes() for p in (survey, trace)])

    def test_raw_query_detects_missing_first_last_all_and_filtered_results(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            survey = root / "survey.jsonl"
            def survey_with_bounds(first, next_seq):
                survey.write_text(json.dumps({"t": "header"}) + "\n" + json.dumps({
                    "t": "closed", "raw_retained_first_seq": first,
                    "raw_next_seq": next_seq,
                }) + "\n")

            def run(*args):
                return json.loads(subprocess.check_output(
                    [sys.executable, "-I", str(READER), str(root), *args], text=True,
                ))

            survey_with_bounds(0, 2)
            paths = [root / f"trace-{i:010d}.jsonl" for i in range(2)]
            for keep in ((0, 1), (0,), (1,), ()):
                with self.subTest(keep=keep):
                    for i, p in enumerate(paths):
                        if i in keep:
                            p.write_text(json.dumps({"t": "path_attempt", "seq": i,
                                                     "agent": 1, "tick": i}) + "\n")
                        else:
                            p.unlink(missing_ok=True)
                    res = run("--agent", "9")
                    self.assertEqual(res["records"], [])
                    self.assertEqual(res["recording_may_be_incomplete"], len(keep) != 2)
            survey_with_bounds(0, 0)
            self.assertFalse(run()["recording_may_be_incomplete"])


if __name__ == "__main__":
    unittest.main()
