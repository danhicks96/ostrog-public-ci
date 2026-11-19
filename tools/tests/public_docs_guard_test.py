#!/usr/bin/env python3
"""Real tiny exports reject absent policy and newly added nested documentation."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
ROOT = Path(__file__).resolve().parents[2]
class DocsTests(unittest.TestCase):
    def export(self, policy, extra=False, unlisted_asset=False):
        with tempfile.TemporaryDirectory(prefix='exact-docs-') as raw:
            repo = Path(raw) / 'source'
            publish = repo / 'tools/publish'
            publish.mkdir(parents=True)
            for name in ('make-snapshot.sh', 'docs_policy.py', 'public_text.py', 'public-ci.yml'):
                shutil.copy2(ROOT / 'tools/publish' / name, publish / name)
            for name in ('scrub.sed', 'forbidden.txt'):
                (publish / name).write_text('')
            (publish / 'exclude.txt').write_text('tools/publish\n')
            if policy is not None:
                (publish / 'public-docs.txt').write_text(policy)
            # FIXLOG 2026-11-19: exercise the real required provenance gate.
            # This docs-only fixture has no fallback art. The replacement-policy
            # shim models that empty scope; the real fallback suite tests its policy.
            art = repo / 'tools/art'
            art.mkdir(parents=True)
            shutil.copy2(ROOT / 'tools/art/provenance_check.py', art / 'provenance_check.py')
            (art / 'replacement_policy.py').write_text('def check_provenance():\n    pass\n')
            if unlisted_asset:
                assets = repo / 'godot/assets'
                assets.mkdir(parents=True)
                (assets / 'synthetic.png').write_bytes(b'synthetic filename-only fixture')
            docs = repo / 'docs/nested'
            docs.mkdir(parents=True)
            (docs / 'known.md').write_text('retained fixture\n')
            if extra:
                (docs / 'new.md').write_text('unreviewed fixture\n')
            env = {k: v for k, v in os.environ.items() if not k.startswith(('GIT_', 'HYGIENE_'))}
            env.update(GIT_AUTHOR_NAME='Fixture', GIT_COMMITTER_NAME='Fixture',
                       GIT_AUTHOR_EMAIL='fixture@example.com', GIT_COMMITTER_EMAIL='fixture@example.com')
            for args in (['git', 'init', '-q'], ['git', 'add', '-A'],
                         ['git', '-c', 'core.hooksPath=/dev/null', 'commit', '-qm', 'fixture']):
                subprocess.run(args, cwd=repo, env=env, capture_output=True, check=True)
            result = subprocess.run(['bash', 'tools/publish/make-snapshot.sh', str(Path(raw) / 'out'), 'HEAD'],
                                    cwd=repo, env=env, capture_output=True)
            return result.returncode
    def test_missing_empty_or_invalid_policy_fails(self):
        for policy in (None, '', '# no entries\n', '../escape\n', 'nested\n', 'nested/*\n'):
            self.assertNotEqual(self.export(policy), 0)
    def test_new_nested_file_requires_explicit_review(self):
        self.assertEqual(self.export('nested/known.md\n'), 0)
        self.assertNotEqual(self.export('nested/known.md\n', extra=True), 0)
        self.assertEqual(self.export('nested/known.md\nnested/new.md\n', extra=True), 0)
    def test_required_provenance_gate_is_not_bypassed(self):
        self.assertNotEqual(self.export('nested/known.md\n', unlisted_asset=True), 0)
if __name__ == '__main__':
    unittest.main(verbosity=2)
