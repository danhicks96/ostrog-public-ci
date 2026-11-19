"""Small synthetic tests for source-only bootstrap guards; never regenerate artwork."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('source_assets', ROOT / 'tools/ci_source_assets.py')
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)

class Guards(unittest.TestCase):
    def test_missing_changed_and_linked_input_fail(self):
        with tempfile.TemporaryDirectory() as d:
            r = Path(d);p = r/'input';row = {'path':'input','sha256':hashlib.sha256(b'toy').hexdigest()}
            with self.assertRaises(ValueError):m.verify_rows(r,[row])
            p.write_bytes(b'toy');m.verify_rows(r,[row]);p.write_bytes(b'changed')
            with self.assertRaises(ValueError):m.verify_rows(r,[row])
            p.unlink();p.symlink_to(r/'absent')
            with self.assertRaises(ValueError):m.verify_rows(r,[row])
    def test_output_completeness_and_hash_fail_closed(self):
        with tempfile.TemporaryDirectory() as d:
            r=Path(d);p=r/'godot/assets/toy.png';p.parent.mkdir(parents=True)
            rows=[{'path':'godot/assets/toy.png','sha256':hashlib.sha256(b'toy').hexdigest()}]
            with self.assertRaises(ValueError):m.verify_outputs(r,rows)
            p.write_bytes(b'toy');m.verify_outputs(r,rows)
            extra=p.with_name('extra.ogg');extra.write_bytes(b'unexpected')
            with self.assertRaises(ValueError):m.verify_outputs(r,rows)
            extra.unlink();p.write_bytes(b'changed')
            with self.assertRaises(ValueError):m.verify_outputs(r,rows)
    def test_invalid_manifests_fail(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'manifest.json'
            for rows in ([],[{'path':'../escape','sha256':'0'*64}],
                         [{'path':'same','sha256':'0'*64}]*2):
                p.write_text(json.dumps(rows))
                with self.assertRaises(ValueError):m.load_rows(p)

class CommittedContent(unittest.TestCase):
    def test_media_unknown_binary_and_links_rejected(self):
        spec=importlib.util.spec_from_file_location('source_only',ROOT/'tools/check_source_only.py')
        checker=importlib.util.module_from_spec(spec);spec.loader.exec_module(checker)
        self.assertTrue(checker.allowed('README.md','100644',b'text'))
        for path,mode,data in [('toy.png','100644',b'text'),('unknown.bin','100644',b'\x00\xff'),('link','120000',b'target')]:
            self.assertFalse(checker.allowed(path,mode,data))
        for path in checker.FIXTURES:
            self.assertFalse(checker.allowed(path,'100644',b'changed fixture'))

if __name__=='__main__':unittest.main(verbosity=2)
