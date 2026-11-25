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


if __name__ == "__main__":
    unittest.main()
