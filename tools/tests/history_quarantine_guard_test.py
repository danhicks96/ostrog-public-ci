#!/usr/bin/env python3
"""Synthetic export regression: private operations absent, public history retained."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
PRIVATE = ('docs/history/MIRROR.md', 'docs/history/HANDOFF-2026-10-30.md',
           'docs/history/handoffs/fixture.md')


class HistoryTests(unittest.TestCase):
    def test_public_docs_do_not_point_to_private_operations(self):
        for name in ('HANDBOOK.md', 'REFERENCE.md'):
            text = (ROOT / 'docs' / name).read_text()
            for target in ('MIRROR.md', 'HANDOFF-2026-10-30.md', 'history/handoffs', 'PLAN-NEXT-handoffs', 'STATUS-handoffs'):
                self.assertFalse(target in text, 'stale private reference remains')

    def test_real_exporter_with_synthetic_history(self):
        with tempfile.TemporaryDirectory(prefix='history-quarantine-') as raw:
            repo = Path(raw) / 'source'
            repo.mkdir()
            env = {k: v for k, v in os.environ.items()
                   if not k.startswith(('GIT_', 'HYGIENE_'))}
            env.update(GIT_AUTHOR_NAME='Fixture', GIT_COMMITTER_NAME='Fixture',
                       GIT_AUTHOR_EMAIL='fixture@example.com', GIT_COMMITTER_EMAIL='fixture@example.com',
                       PUBLIC_NAME='Fixture', PUBLIC_EMAIL='fixture@example.com')
            def run(*args):
                return subprocess.run(args, cwd=repo, env=env, capture_output=True)
            publish = repo / 'tools/publish'
            publish.mkdir(parents=True)
            for name in ('make-snapshot.sh', 'exclude.txt', 'scrub.sed', 'forbidden.txt', 'docs_policy.py', 'public_text.py', 'public-ci.yml'):
                shutil.copy2(ROOT / 'tools/publish' / name, publish / name)
            (publish / 'public-docs.txt').write_text('history/ROADMAP.md\nhistory/FIXLOG.md\nhistory/ITERATION_LOG.md\n')
            keep = ('docs/history/ROADMAP.md', 'docs/history/FIXLOG.md', 'docs/history/ITERATION_LOG.md')
            for name in PRIVATE + keep:
                p = repo / name
                p.parent.mkdir(parents=True, exist_ok=True)
                p.write_text('synthetic history fixture\n')
            for args in (('git', 'init', '-q'), ('git', 'add', '-A'),
                         ('git', '-c', 'core.hooksPath=/dev/null', 'commit', '-qm', 'fixture')):
                self.assertEqual(run(*args).returncode, 0)
            output = Path(raw) / 'export'
            result = run('bash', 'tools/publish/make-snapshot.sh', str(output), 'HEAD')
            self.assertEqual(result.returncode, 0, 'synthetic exporter failed; output withheld')
            for name in PRIVATE:
                self.assertFalse((output / name).exists(), 'private operation path survived')
                self.assertTrue((repo / name).is_file(), 'source history changed')
            for name in keep:
                self.assertTrue((output / name).is_file(), 'public history was over-excluded')


if __name__ == '__main__':
    unittest.main(verbosity=2)
