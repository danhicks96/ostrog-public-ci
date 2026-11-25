"""Regression for the asset-independent generated data-map check."""
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

GENERATOR = Path(__file__).resolve().parents[1] / "docs" / "readme_tables.py"


class DataMapsTest(unittest.TestCase):
    def test_version_change_fails_until_regenerated(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            script = root / "tools/docs/readme_tables.py"
            script.parent.mkdir(parents=True)
            shutil.copyfile(GENERATOR, script)
            save = root / "crates/hf-sim/src/save"
            save.mkdir(parents=True)
            source = save / "mod.rs"
            source.write_text("pub const FORMAT_VERSION: u32 = 2;\n")
            page = save / "README.md"
            page.write_text("<!-- BEGIN GENERATED: save-contract -->\n<!-- END GENERATED: save-contract -->\n")

            def run(flag):
                return subprocess.run(
                    [sys.executable, "-I", str(script), flag],
                    check=False, capture_output=True, text=True,
                )

            self.assertEqual(run("--maps").returncode, 0)
            self.assertEqual(run("--check-maps").returncode, 0)
            source.write_text("// unrelated comment\npub const FORMAT_VERSION: u32 = 2;\n")
            self.assertEqual(run("--check-maps").returncode, 0)
            source.write_text("pub const FORMAT_VERSION: u32 = 3;\n")
            stale = run("--check-maps")
            self.assertEqual(stale.returncode, 1)
            self.assertIn("save data map is out of date", stale.stderr)
            self.assertIn("**2**", page.read_text())
            self.assertEqual(run("--maps").returncode, 0)
            self.assertEqual(run("--check-maps").returncode, 0)
            self.assertIn("**3**", page.read_text())


if __name__ == "__main__":
    unittest.main()
