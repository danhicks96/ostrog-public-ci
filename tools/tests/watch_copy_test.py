#!/usr/bin/env python3
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


SOURCE_ROOT = Path(__file__).resolve().parents[2]


class WatchCopyTest(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.TemporaryDirectory(prefix="hf-watch-copy-")
        self.temp = Path(self.temp_dir.name)
        self.root = self.temp / "checkout"
        self.prebuilt = self.temp / "prebuilt"
        self.fakebin = self.temp / "bin"
        self.godot_log = self.temp / "godot.log"
        self.cargo_log = self.temp / "cargo.log"
        self.fakebin.mkdir()
        # FIXLOG 2026-10-31 (#430): do not let a host fallback Godot bypass this stub.
        (self.root / ".tools").mkdir(parents=True)
        self._write_executable(
            self.root / ".tools" / "godot4",
            '#!/usr/bin/env bash\nprintf \'CALL\\n\' >> "$HF_TEST_GODOT_LOG"\n'
            'printf \'%s\\n\' "$@" >> "$HF_TEST_GODOT_LOG"\n'
            'mkdir -p "$2/.godot"\n'
            'touch "$2/.godot/extension_list.cfg"\n',
        )
        self._write_executable(
            self.fakebin / "cargo",
            "#!/usr/bin/env bash\n"
            'printf \'%s\\n\' "$*" > "$HF_TEST_CARGO_LOG"\n'
            'mkdir -p "$CARGO_TARGET_DIR/release"\n'
            'printf \'local build\' > "$CARGO_TARGET_DIR/release/hf_godot.dll"\n',
        )

        (self.root / "tools").mkdir(parents=True)
        (self.root / "godot").mkdir()
        for name in ("watch-copy.sh", "godot"):
            shutil.copy2(SOURCE_ROOT / "tools" / name, self.root / "tools" / name)
        (self.root / ".gitignore").write_text("/godot/bin/\n", encoding="utf-8")
        (self.root / "tracked.txt").write_text("initial\n", encoding="utf-8")
        self._git("init", "-b", "master")
        self._git("config", "user.name", "Watch Copy Test")
        self._git("config", "user.email", "watch-copy@example.invalid")
        self._git("add", ".")
        self._git("commit", "-m", "initial")
        self.remote = self.temp / "remote.git"
        subprocess.run(
            ["git", "init", "--bare", str(self.remote)], check=True, capture_output=True
        )
        self._git("remote", "add", "origin", str(self.remote))
        self._git("push", "-u", "origin", "master")

    def tearDown(self):
        self.temp_dir.cleanup()

    def _write_executable(self, path, text):
        path.write_text(text, encoding="utf-8", newline="\n")
        path.chmod(0o755)

    def _git(self, *args, cwd=None):
        return subprocess.run(
            ["git", *args],
            cwd=cwd or self.root,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()

    def _advance_master(self):
        peer = self.temp / "peer"
        if not peer.exists():
            subprocess.run(
                ["git", "clone", str(self.remote), str(peer)],
                check=True,
                capture_output=True,
            )
            self._git("config", "user.name", "Watch Copy Test", cwd=peer)
            self._git("config", "user.email", "watch-copy@example.invalid", cwd=peer)
        with (peer / "tracked.txt").open("a", encoding="utf-8") as stream:
            stream.write("update\n")
        self._git("add", "tracked.txt", cwd=peer)
        self._git("commit", "-m", "update", cwd=peer)
        self._git("push", "origin", "master", cwd=peer)
        return self._git("rev-parse", "HEAD", cwd=peer)

    def _publish(self, commit, contents, checksum=None):
        folder = self.prebuilt / commit
        folder.mkdir(parents=True)
        (folder / "hf_godot.dll").write_bytes(contents)
        checksum = checksum or hashlib.sha256(contents).hexdigest()
        (folder / "hf_godot.dll.sha256").write_text(
            f"{checksum}  hf_godot.dll\n", encoding="ascii"
        )

    def _run_watch(self):
        env = os.environ.copy()
        env.update(
            OSTROG_PREBUILT_ROOT=str(self.prebuilt),
            HF_TEST_GODOT_LOG=str(self.godot_log),
            HF_TEST_CARGO_LOG=str(self.cargo_log),
            PATH=str(self.fakebin) + os.pathsep + env["PATH"],
        )
        return subprocess.run(
            ["bash", str(self.root / "tools" / "watch-copy.sh")],
            cwd=self.temp,
            env=env,
            capture_output=True,
            text=True,
        )

    def test_clean_master_pulls_matching_prebuilt_and_launches(self):
        old = self._git("rev-parse", "HEAD")
        new = self._advance_master()
        self.assertNotEqual(old, new)
        self._publish(new, b"matching prebuilt")

        result = self._run_watch()

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self._git("rev-parse", "HEAD"), new)
        installed = self.root / "godot" / "bin" / "windows" / "hf_godot.dll"
        self.assertEqual(installed.read_bytes(), b"matching prebuilt")
        expected_project = subprocess.run(
            ["bash", "-c", 'cd "$1" && pwd', "_", str(self.root / "godot")],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        self.assertEqual(
            self.godot_log.read_text(encoding="utf-8").splitlines(),
            [
                "CALL",
                "--path",
                expected_project,
                "--headless",
                "--editor",
                "--quit",
                "CALL",
                "--path",
                expected_project,
            ],
        )
        self.assertFalse(self.cargo_log.exists())

    def test_missing_prebuilt_falls_back_to_local_release_build(self):
        new = self._advance_master()

        result = self._run_watch()

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self._git("rev-parse", "HEAD"), new)
        self.assertEqual(
            self.cargo_log.read_text(encoding="utf-8").strip(),
            "build --release -p hf-godot",
        )
        installed = self.root / "godot" / "bin" / "windows" / "hf_godot.dll"
        self.assertEqual(installed.read_bytes(), b"local build")
        self.assertTrue(self.godot_log.exists())

    def test_dirty_checkout_is_not_pulled_or_launched(self):
        old = self._git("rev-parse", "HEAD")
        self._advance_master()
        (self.root / "tracked.txt").write_text("local work\n", encoding="utf-8")

        result = self._run_watch()

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("clean master", result.stderr)
        self.assertEqual(self._git("rev-parse", "HEAD"), old)
        self.assertFalse(self.godot_log.exists())
        self.assertFalse(self.cargo_log.exists())

    def test_non_master_checkout_is_not_pulled_or_launched(self):
        old = self._git("rev-parse", "HEAD")
        self._advance_master()
        self._git("checkout", "-b", "feature")

        result = self._run_watch()

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("clean master", result.stderr)
        self.assertEqual(self._git("rev-parse", "HEAD"), old)
        self.assertFalse(self.godot_log.exists())
        self.assertFalse(self.cargo_log.exists())

    def test_bad_prebuilt_checksum_preserves_existing_dll(self):
        new = self._advance_master()
        self._publish(new, b"damaged download", checksum="0" * 64)
        installed = self.root / "godot" / "bin" / "windows" / "hf_godot.dll"
        installed.parent.mkdir(parents=True)
        installed.write_bytes(b"previous working dll")

        result = self._run_watch()

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("checksum", result.stderr)
        self.assertEqual(installed.read_bytes(), b"previous working dll")
        self.assertFalse(self.godot_log.exists())
        self.assertFalse(self.cargo_log.exists())


if __name__ == "__main__":
    unittest.main()
