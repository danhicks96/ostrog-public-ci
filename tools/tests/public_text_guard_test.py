#!/usr/bin/env python3
"""Public-view cleanup preserves available links and never edits private source."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('public_text', ROOT / 'tools/publish/public_text.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class PublicTextTests(unittest.TestCase):
    def test_missing_local_links_and_images_only(self):
        with tempfile.TemporaryDirectory(prefix='public-view-') as raw:
            root = Path(raw).resolve()
            file = root / 'README.md'
            (root / 'present.md').write_text('fixture')
            source = '[keep](present.md) [external](https://example.com) [anchor](#x) [old](missing.md) ![image](image.png)'
            result = module.rewrite(source, file, root)
            self.assertIn('[keep](present.md)', result)
            self.assertIn('[external](https://example.com)', result)
            self.assertIn('[anchor](#x)', result)
            self.assertNotIn('(missing.md)', result)
            self.assertNotIn('![image]', result)
            self.assertIn('not included', result)

    def test_path_outside_export_is_not_linked(self):
        with tempfile.TemporaryDirectory(prefix='public-view-') as raw:
            root = Path(raw).resolve()
            result = module.rewrite('[outside](../../outside)', root / 'README.md', root)
            self.assertNotIn('](../../outside)', result)


    def test_reviewed_context_is_scoped_and_preserves_examples(self):
        root = Path('/synthetic-export')
        source = '[GitHub issues](https://github.com/example/project/issues)'
        self.assertNotIn('github.com', module.rewrite(source, root / 'README.md', root))
        self.assertEqual(source, module.rewrite(source, root / 'NOTICE', root))
        source = '- The code is on GitHub (private): https://example.invalid/project'
        self.assertNotIn('example.invalid', module.rewrite(source, root / 'docs/HANDBOOK.md', root))
        source = ('fresh four-seed 600-day soak in\n  `Z:\\synthetic-validation`, evidence\n'
                  '[2026-11-13 alive audit](https://github.com/example/project/audit)\n'
                  'Reject `C:/absolute.exe`; retain https://example.com/license')
        result = module.rewrite(source, root / 'FIXLOG.md', root)
        self.assertNotIn('synthetic-validation', result)
        self.assertNotIn('github.com', result)
        self.assertIn('C:/absolute.exe', result)
        self.assertIn('https://example.com/license', result)


if __name__ == '__main__':
    unittest.main(verbosity=2)
