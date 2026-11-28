#!/usr/bin/env python3
"""The public snapshot may omit the plan, but never the harness checks."""

import contextlib
import importlib.util
import io
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("frame_time_guard", HERE / "frame_time_harness_test.py")
GUARD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GUARD)


class FrameTimeGuardTest(unittest.TestCase):
    def test_public_snapshot_checks_harness_without_private_plan(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with patch.object(GUARD, "ROOT", root), patch.object(GUARD, "PLAN", root / "missing.md"):
                with contextlib.redirect_stdout(io.StringIO()) as output:
                    GUARD.main()
                self.assertIn("private renderer plan absent", output.getvalue())
                with patch.object(GUARD, "HARNESS", root / "missing.gd"):
                    with self.assertRaises(FileNotFoundError):
                        GUARD.main()

    def test_present_plan_is_still_checked(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            plan = root / "plan.md"
            plan.write_text("invalid", encoding="utf-8")
            with patch.object(GUARD, "ROOT", root), patch.object(GUARD, "PLAN", plan):
                with self.assertRaises(AssertionError):
                    GUARD.main()
                plan.write_text("Status: open (6.1 complete\nDepends on: no dependency for 6.1-6.4\n", encoding="utf-8")
                with contextlib.redirect_stdout(io.StringIO()):
                    GUARD.main()

    def test_private_tree_still_requires_its_plan(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "tools/publish").mkdir(parents=True)
            with patch.object(GUARD, "ROOT", root), patch.object(GUARD, "PLAN", root / "missing.md"):
                with self.assertRaises(FileNotFoundError):
                    GUARD.main()


if __name__ == "__main__":
    unittest.main()
