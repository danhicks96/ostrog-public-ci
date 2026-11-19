#!/usr/bin/env python3
"""WP-09 slice 9.11: every file the Windows audits found (the 2026-11-13 first install and the
2026-11-14 update runs of WP-02 2.9) is in the files register, and a file the register does not list
is reported. windows_package_test.py checks the packaged tree."""

import sys
import unittest
from pathlib import Path

SOURCE_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(SOURCE_ROOT / "tools" / "privacy"))

import files_register  # noqa: E402

AUDIT = SOURCE_ROOT / "docs" / "reviews" / "2026-11-13-9.11-files-audit" / "audit-paths.txt"
UPDATE_AUDIT = SOURCE_ROOT / "docs" / "reviews" / "2026-11-14-9.11-update-audit" / "audit-paths.txt"
PUBLIC_SOURCE = not (SOURCE_ROOT / 'tools/publish').exists() and not (SOURCE_ROOT / 'AGENTS.md').exists()


class FilesRegisterTest(unittest.TestCase):
    # FIXLOG 2026-11-19: empirical audit records remain private; synthetic coverage ships.
    @unittest.skipIf(PUBLIC_SOURCE, 'empirical Windows audit retained in private source')
    def test_every_file_of_the_windows_audit_is_listed(self):
        names = files_register.load_register()
        lines = AUDIT.read_text(encoding="utf-8").splitlines()
        self.assertEqual(files_register.unlisted(lines, names), [])
        self.assertGreater(len([l for l in lines if l and not l.startswith("#")]), 20)

    @unittest.skipIf(PUBLIC_SOURCE, 'empirical Windows update audit retained in private source')
    def test_every_file_of_the_windows_update_audit_is_listed(self):
        names = files_register.load_register()
        lines = UPDATE_AUDIT.read_text(encoding="utf-8").splitlines()
        self.assertEqual(files_register.unlisted(lines, names), [])
        paths = [l for l in lines if l and not l.startswith("#")]
        self.assertIn("install:.ostrog-update/backup/ostrog.exe", paths)
        self.assertTrue(any(l.startswith("temp:ostrog-archive-") for l in paths))

    def test_synthetic_install_and_update_paths_are_covered(self):
        names = files_register.load_register()
        lines = ['install:app/ostrog.exe', 'install:.ostrog-update/backup/ostrog.exe',
                 'userdata:saves/auto.1.hfsave/world.bin',
                 'temp:ostrog-archive-1-2-3.zip.part']
        self.assertEqual(files_register.unlisted(lines, names), [])

    def test_an_unlisted_file_is_reported(self):
        names = files_register.load_register()
        lines = [
            "install:app/ostrog.exe",
            "install:telemetry.dll",
            "userdata:saves/auto.1.hfsave/world.bin",
            "userdata:crash_reports/today.dmp",
            "temp:ostrog/chronicle-12-0.sqlite",
            "temp:somebody-else.tmp",
            "elsewhere:anything",
        ]
        self.assertEqual(
            files_register.unlisted(lines, names),
            [
                "install:telemetry.dll",
                "userdata:crash_reports/today.dmp",
                "temp:somebody-else.tmp",
                "elsewhere:anything",
            ],
        )

    def test_folders_and_patterns_cover_what_is_inside(self):
        names = ["saves/", "ostrog-archive-*.zip.part", "settings.cfg"]
        self.assertTrue(files_register.covered("saves", names))
        self.assertTrue(files_register.covered("saves/auto.0.hfsave/world.bin", names))
        self.assertTrue(files_register.covered("ostrog-archive-1-2-3.zip.part", names))
        self.assertTrue(files_register.covered("settings.cfg", names))
        self.assertFalse(files_register.covered("savestate", names))
        self.assertFalse(files_register.covered("settings.cfg.bak", names))


if __name__ == "__main__":
    unittest.main()
