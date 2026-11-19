#!/usr/bin/env python3
"""Synthetic generic scanner fixtures; never accepts a private list."""
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class BoundaryTests(unittest.TestCase):
    def scan(self, text, private=False):
        with tempfile.TemporaryDirectory(prefix='generic-hygiene-') as cwd:
            env = {k: v for k, v in os.environ.items()
                   if not k.startswith('GIT_') and not k.startswith('HYGIENE_')}
            # Synthetic configuration must be ignored, including nonexistent files.
            env.update(HYGIENE_BLOCKLIST='synthetic_boundary_marker',
                       HYGIENE_BLOCKLIST_FILE=str(Path(cwd) / 'absent'))
            args = ['--require-blocklist'] if private else []
            return subprocess.run([sys.executable, '-I', str(ROOT / 'tools/hygiene/scan_identifiers.py'),
                                   *args, '--text', '-'], input=text, text=True, env=env,
                                  cwd=cwd, capture_output=True)

    def test_public_scanner_ignores_confidential_input_channels(self):
        self.assertEqual(self.scan('synthetic_boundary_marker').returncode, 0)

    def test_no_inline_suppression(self):
        value = '.'.join(('10', '20', '30', '40'))
        result = self.scan(value + ' hygiene:ok')
        self.assertEqual(result.returncode, 1)
        self.assertNotIn(value, result.stdout + result.stderr)

    def test_private_mode_fails_closed(self):
        self.assertEqual(self.scan('', private=True).returncode, 1)

    def test_documentation_placeholder_passes(self):
        self.assertEqual(self.scan('user@example.com 192.0.2.1').returncode, 0)

    def test_committed_marker_detection(self):
        spec = importlib.util.spec_from_file_location('structure', ROOT / 'tools/check_export_structure.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        self.assertFalse(module.assess(['README.md', 'tools/ci.sh']))
        for path in ('AGENTS.md', 'tools/publish/run.sh', 'go-public/notes.md', 'tools/boxci/run.sh', '.github/workflows/automerge.yml'):
            self.assertTrue(module.assess([path]))

    def test_workflow_has_no_secret_or_private_audit_wiring(self):
        template = ROOT / 'tools/publish/public-ci.yml'
        text = (template if template.is_file() else ROOT / '.github/workflows/ci.yml').read_text()
        self.assertNotIn('secrets.', text)
        self.assertNotIn('HYGIENE_BLOCKLIST', text)
        self.assertNotIn('snapshot-gate:', text)
        self.assertIn('export-structure:', text)
        self.assertIn('pull_request:', text)
        self.assertIn('push:', text)


if __name__ == '__main__':
    unittest.main(verbosity=2)
