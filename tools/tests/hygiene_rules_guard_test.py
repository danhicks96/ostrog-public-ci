#!/usr/bin/env python3
"""Guard tests for the hygiene rules (docs/HYGIENE.md): each rule fails a synthetic input that
breaks it and passes a clean one. Banned words are assembled from pieces so this file is clean."""
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SCAN = str(ROOT / "tools/hygiene/scan_identifiers.py")
NAME = "".join(map(chr, (68, 97, 110)))            # the owner's first name
BRAND = "Cla" + "ude"
OWNER_LOGIN = "dan" + "hicks96"
REPO = OWNER_LOGIN + "/os" + "trog"


def run(args, text=None, cwd=None):
    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    return subprocess.run([sys.executable, "-I", SCAN, *args], input=text, text=True,
                          capture_output=True, cwd=cwd, env=env)


class TextRules(unittest.TestCase):
    def bad(self, text):
        r = run(["--text", "-"], text)
        self.assertEqual(r.returncode, 1, text)
        self.assertNotIn(NAME, r.stdout + r.stderr)  # matched text is never echoed

    def test_clean(self):
        self.assertEqual(run(["--text", "-"], "Fix the wild plants cadence. See #620.\n").returncode, 0)

    def test_owner_name(self):
        self.bad(f"{NAME}'s rule 3")
        self.bad(f"ask {NAME.lower()}")

    def test_the_box(self):
        self.bad("run it on the box")

    def test_ai_brand(self):
        self.bad(f"made with {BRAND} Code")

    def test_private_repo(self):
        self.bad(f"see {REPO}/pull/1")
        self.assertEqual(run(["--text", "-"], "see example-owner/ostrog-public-ci/pull/1").returncode, 0)

    def ok(self, text):
        self.assertEqual(run(["--text", "-"], text).returncode, 0, text)

    def test_private_repo_false_positives(self):
        self.ok("palette/ostrog.json and app_userdata/Ostrog and godot/ostrog/x")
        self.ok("see https://github.com/example-owner/ostrog-public-ci/pull/1")
        self.bad("clone https://github.com/example-owner/os" + "trog and push")
        self.bad(f"gh api repos/{OWNER_LOGIN}/os" + "trog/commits")

    def test_the_box_false_positives(self):
        self.ok("the box-ci status on the head")
        self.bad("run it on the box")

    def test_thread_id_false_positives(self):
        self.ok("env_variables and session_snapshot and chan_count")
        self.bad("thread=cmsg_012TReBor8XMdHXqVNRKr1qxRqzUjGAjr7rSALDZrPHsAB")
        self.bad("session_013fHBkxMuHiWiNPmf5TgWaM")

    def test_footer_and_attribution(self):
        self.bad(f"Generated with [{BRAND} Code](https://example.invalid)")
        self.bad("<!-- x-projects-attribution: {} -->")
        self.bad("_Requested by **someone** · thread_")
        self.bad("[project thread](https://example.invalid?thread=cmsg_0123456789abcdef)")


class BranchRules(unittest.TestCase):
    def test_branches(self):
        self.assertEqual(run(["--branch", BRAND.lower() + "/fix"]).returncode, 1)
        self.assertEqual(run(["--branch", "co" + "pilot/fix"]).returncode, 1)
        self.assertEqual(run(["--branch", "scrub/personal-info"]).returncode, 0)


class CommitRules(unittest.TestCase):
    def repo(self, name, email, msg, content="ok\n"):
        d = tempfile.mkdtemp(prefix="hyg-")
        g = lambda *a: subprocess.run(["git", *a], cwd=d, check=True, capture_output=True,
                                      env={**os.environ, "GIT_CONFIG_GLOBAL": "/dev/null"})
        g("init", "-q"); g("config", "user.name", "Ostrog CI"); g("config", "user.email", "ci@users.noreply.github.com")
        (Path(d) / "a").write_text("base\n"); g("add", "a"); g("commit", "-qm", "base")
        (Path(d) / "a").write_text(content); g("add", "a")
        g("-c", f"user.name={name}", "-c", f"user.email={email}", "commit", "-qm", msg)
        return d

    def test_neutral_commit_passes(self):
        d = self.repo("Ostrog CI", "ci@users.noreply.github.com", "Fix a thing")
        self.assertEqual(run(["--commits", "HEAD~1..HEAD"], cwd=d).returncode, 0)

    def test_lane_identity_passes(self):
        d = self.repo("Ostrog CI", "ci@ostrog.invalid", "Fix a thing")
        self.assertEqual(run(["--commits", "HEAD~1..HEAD"], cwd=d).returncode, 0)

    def test_web_merge_commit_identity_is_not_checked(self):
        d = self.repo("Ostrog CI", "ci@users.noreply.github.com", "Fix")
        g = lambda *a: subprocess.run(["git", *a], cwd=d, check=True, capture_output=True,
                                      env={**os.environ, "GIT_CONFIG_GLOBAL": "/dev/null"})
        g("checkout", "-q", "-b", "side", "HEAD~1"); (Path(d) / "b").write_text("b\n"); g("add", "b")
        g("commit", "-qm", "side")
        g("-c", f"user.name={NAME} Smith", "-c", "user.email=p@mail.org", "merge", "--no-ff", "-q", "master", "-m", "Merge branch")
        self.assertEqual(run(["--commits", "HEAD~2..HEAD"], cwd=d).returncode, 0)

    def test_ai_trailer(self):
        d = self.repo("Ostrog CI", "ci@users.noreply.github.com", f"Fix\n\nCo-Authored-By: {BRAND} <x@example.com>")
        self.assertEqual(run(["--commits", "HEAD~1..HEAD"], cwd=d).returncode, 1)

    def test_personal_email_and_name(self):
        d = self.repo("Ostrog CI", "someone@mail.org", "Fix")
        self.assertEqual(run(["--commits", "HEAD~1..HEAD"], cwd=d).returncode, 1)
        d = self.repo(NAME + " Smith", "ci@users.noreply.github.com", "Fix")
        self.assertEqual(run(["--commits", "HEAD~1..HEAD"], cwd=d).returncode, 1)


class DiffRules(unittest.TestCase):
    def test_added_line_and_skips(self):
        d = tempfile.mkdtemp(prefix="hyg-")
        g = lambda *a: subprocess.run(["git", *a], cwd=d, check=True, capture_output=True)
        g("init", "-q"); g("config", "user.name", "Ostrog CI"); g("config", "user.email", "c@example.com")
        (Path(d) / "a.md").write_text("x\n"); (Path(d) / "NOTICE").write_text("x\n"); g("add", "."); g("commit", "-qm", "b")
        (Path(d) / "a.md").write_text(f"x\nby {NAME}\n"); g("add", ".")
        self.assertEqual(run(["--staged"], cwd=d).returncode, 1)
        g("reset", "-q"); (Path(d) / "a.md").write_text("x\n"); (Path(d) / "NOTICE").write_text(f"Copyright {NAME}\n"); g("add", ".")
        self.assertEqual(run(["--staged"], cwd=d).returncode, 0)  # the copyright notice is allowed


if __name__ == "__main__":
    unittest.main(verbosity=1)
