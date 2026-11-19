#!/usr/bin/env python3
"""Smoke-test the clean portable package layout without a Godot or Rust build."""

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
import zipfile


SOURCE_ROOT = Path(__file__).resolve().parents[2]


class PackageLayoutTest(unittest.TestCase):
    def test_portable_package_keeps_launcher_outside_app_and_data_on_uninstall(self):
        with tempfile.TemporaryDirectory(prefix="hf-package-") as raw:
            root = Path(raw)
            checkout = root / "checkout"
            checkout.mkdir()
            (checkout / "tools").mkdir()
            (checkout / "godot").mkdir()
            package_script = SOURCE_ROOT / "tools" / "package.sh"
            (checkout / "tools/package.sh").write_bytes(
                package_script.read_bytes().replace(b"\r\n", b"\n")
            )
            (checkout / "docs").mkdir(exist_ok=True)
            shutil.copy2(SOURCE_ROOT / "docs/REFERENCE.md", checkout / "docs/REFERENCE.md")
            godot = checkout / "tools/godot"
            godot.write_bytes(
                b"#!/usr/bin/env bash\n"
                b"out=\"${!#}\"\n"
                b"mkdir -p \"$(dirname \"$out\")\"\n"
                b"printf export > \"$out\"\n"
                b"case \"$out\" in\n"
                b"  *.x86_64) printf bridge > \"$(dirname \"$out\")/libhf_godot.so\" ;;\n"
                b"  *.exe) printf bridge > \"$(dirname \"$out\")/hf_godot.dll\" ;;\n"
                b"esac\n"
            )
            godot.chmod(0o755)
            (checkout / "target/release").mkdir(parents=True)
            (checkout / "target/x86_64-pc-windows-msvc/release").mkdir(parents=True)
            for path in (
                checkout / "target/release/libhf_godot.so",
                checkout / "target/release/hf-launcher",
                checkout / "target/x86_64-pc-windows-msvc/release/hf_godot.dll",
                checkout / "target/x86_64-pc-windows-msvc/release/hf-launcher.exe",
            ):
                path.write_bytes(b"built")

            fakebin = root / "bin"
            fakebin.mkdir()
            strip = fakebin / "strip"
            strip.write_text("#!/usr/bin/env bash\nexit 0\n", encoding="utf-8")
            strip.chmod(0o755)
            zip_tool = fakebin / "zip"
            zip_tool.write_bytes(
                b"#!/usr/bin/env python3\n"
                b"import os, sys, zipfile\n"
                b"args = [arg for arg in sys.argv[1:] if not arg.startswith('-')]\n"
                b"with zipfile.ZipFile(args[0], 'w', zipfile.ZIP_DEFLATED) as out:\n"
                b"    root = args[1]\n"
                b"    for base, dirs, files in os.walk(root):\n"
                b"        for name in dirs + files:\n"
                b"            path = os.path.join(base, name)\n"
                b"            out.write(path, path)\n"
            )
            zip_tool.chmod(0o755)
            env = os.environ.copy()
            env["PATH"] = str(fakebin) + os.pathsep + env.get("PATH", "")
            env["OSTROG_MANIFEST_URL"] = (
                "https://releases.example/ostrog/stable/windows-x64.json"
            )
            env["OSTROG_PUBLIC_KEY"] = "07" * 32
            bash = os.environ.get("OSTROG_GIT_BASH")
            if os.name == "nt" and not bash:
                candidate = Path(r"C:\Program Files\Git\bin\bash.exe")
                bash = str(candidate) if candidate.is_file() else shutil.which("bash")
            bash = bash or "bash"
            package = subprocess.run(
                [bash, "tools/package.sh", "1.2.3"],
                cwd=checkout,
                env=env,
                capture_output=True,
                text=True,
            )
            self.assertEqual(package.returncode, 0, package.stdout + package.stderr)

            archive = checkout / "releases/ostrog-windows-x64-portable.zip"
            self.assertTrue(archive.is_file())
            with zipfile.ZipFile(archive) as package:
                names = set(package.namelist())
                self.assertIn("ostrog-windows-x64-portable/ostrog-launcher.exe", names)
                self.assertIn("ostrog-windows-x64-portable/app/ostrog.exe", names)
                self.assertIn("ostrog-windows-x64-portable/data/.keep", names)
                self.assertIn("ostrog-windows-x64-portable/launcher.toml", names)
                config = package.read(
                    "ostrog-windows-x64-portable/launcher.toml"
                ).decode("utf-8")
            self.assertIn('application_dir = "app"', config)
            self.assertIn('user_data_dir = "data"', config)
            self.assertIn('executable = "ostrog.exe"', config)
            self.assertNotIn("github_token", config.lower())

            install = root / "install"
            with zipfile.ZipFile(archive) as package:
                package.extractall(install)
            portable = install / "ostrog-windows-x64-portable"
            shutil.rmtree(portable / "app")
            for name in ("ostrog-launcher.exe", "launcher.toml", "README.md", "VERSION"):
                (portable / name).unlink()
            self.assertTrue((portable / "data").is_dir())


if __name__ == "__main__":
    unittest.main()
