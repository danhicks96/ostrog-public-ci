#!/usr/bin/env python3
"""Smoke-test the Windows-only portable package path without Godot or Rust builds."""

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
import sys
import zipfile


SOURCE_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(SOURCE_ROOT / "tools" / "privacy"))

import files_register  # noqa: E402


class WindowsPackageTest(unittest.TestCase):
    def test_a_launcher_that_needs_the_vc_runtime_is_refused(self):
        # FIXLOG 2026-11-13: VCRUNTIME140.dll is not part of Windows, so a dynamically linked
        # launcher or bridge would not start on a friend's PC without the VC++ redistributable.
        with tempfile.TemporaryDirectory(prefix="hf-windows-package-") as raw:
            checkout, package = self._package(
                Path(raw), launcher=b"MZ launcher imports VCRUNTIME140.dll"
            )
            self.assertNotEqual(package.returncode, 0, package.stdout + package.stderr)
            self.assertIn("VCRUNTIME140.dll", package.stderr)
            self.assertFalse((checkout / "releases/ostrog-windows-x64-portable.zip").exists())

    def test_windows_package_does_not_require_linux_artifacts(self):
        with tempfile.TemporaryDirectory(prefix="hf-windows-package-") as raw:
            checkout, package = self._package(Path(raw))
            self.assertEqual(package.returncode, 0, package.stdout + package.stderr)
            self._check_portable(checkout)

    def _package(self, root, launcher=b"launcher"):
        checkout = root / "checkout"
        checkout.mkdir()
        (checkout / "tools").mkdir()
        (checkout / "godot").mkdir()
        script = SOURCE_ROOT / "tools" / "package_windows.sh"
        (checkout / "tools/package_windows.sh").write_bytes(
            script.read_bytes().replace(b"\r\n", b"\n")
        )
        (checkout / "docs/privacy").mkdir(parents=True)
        shutil.copy2(SOURCE_ROOT / "docs/REFERENCE.md", checkout / "docs/REFERENCE.md")
        shutil.copy2(
            SOURCE_ROOT / "docs/privacy/READ-ME-FIRST.txt",
            checkout / "docs/privacy/READ-ME-FIRST.txt",
        )

        godot = checkout / "tools/godot"
        godot.write_bytes(
            b"#!/usr/bin/env bash\n"
            b"out=\"${!#}\"\n"
            b"mkdir -p \"$(dirname \"$out\")\"\n"
            b"printf export > \"$out\"\n"
            b"printf bridge > \"$(dirname \"$out\")/hf_godot.dll\"\n"
        )
        godot.chmod(0o755)
        target = checkout / "target/x86_64-pc-windows-msvc/release"
        target.mkdir(parents=True)
        (target / "hf_godot.dll").write_bytes(b"bridge")
        (target / "hf-launcher.exe").write_bytes(launcher)

        fakebin = root / "bin"
        fakebin.mkdir()
        timeout = fakebin / "timeout"
        timeout.write_text(
            "#!/usr/bin/env bash\nshift\nexec \"$@\"\n", encoding="utf-8"
        )
        timeout.chmod(0o755)
        cygpath = fakebin / "cygpath"
        cygpath.write_text(
            "#!/usr/bin/env bash\n"
            "printf '%s\\n' \"${!#}\"\n",
            encoding="utf-8",
        )
        cygpath.chmod(0o755)
        powershell = fakebin / "powershell.exe"
        powershell.write_text(
            "#!/usr/bin/env python3\n"
            "import os, re, sys, zipfile\n"
            "command = sys.argv[sys.argv.index('-Command') + 1]\n"
            "source, destination = re.findall(r\"'([^']+)'\", command)\n"
            "with zipfile.ZipFile(destination, 'w', zipfile.ZIP_DEFLATED) as out:\n"
            "    root = os.path.basename(os.path.normpath(source))\n"
            "    for base, dirs, files in os.walk(source):\n"
            "        for name in dirs + files:\n"
            "            path = os.path.join(base, name)\n"
            "            relative = os.path.relpath(path, source)\n"
            "            out.write(path, os.path.join(root, relative))\n",
            encoding="utf-8",
        )
        powershell.chmod(0o755)
        env = os.environ.copy()
        env["PATH"] = str(fakebin) + os.pathsep + env.get("PATH", "")
        env.pop("OSTROG_MANIFEST_URL", None)
        env.pop("OSTROG_PUBLIC_KEY", None)
        bash = os.environ.get("OSTROG_GIT_BASH")
        if os.name == "nt" and not bash:
            candidate = Path(r"C:\Program Files\Git\bin\bash.exe")
            bash = str(candidate) if candidate.is_file() else shutil.which("bash")
        bash = bash or "bash"
        package = subprocess.run(
            [bash, "tools/package_windows.sh", "1.2.3"],
            cwd=checkout,
            env=env,
            capture_output=True,
            text=True,
        )
        return checkout, package

    def _check_portable(self, checkout):
        app_archive = checkout / "releases/ostrog-windows-x64.zip"
        portable_archive = checkout / "releases/ostrog-windows-x64-portable.zip"
        self.assertTrue(app_archive.is_file())
        self.assertTrue(portable_archive.is_file())
        with zipfile.ZipFile(portable_archive) as package:
            names = set(package.namelist())
            self.assertIn("ostrog-windows-x64-portable/ostrog-launcher.exe", names)
            self.assertIn("ostrog-windows-x64-portable/app/ostrog.exe", names)
            self.assertIn("ostrog-windows-x64-portable/app/hf_godot.dll", names)
            self.assertIn("ostrog-windows-x64-portable/data/.keep", names)
            self.assertIn("ostrog-windows-x64-portable/launcher.toml", names)
            # WP-09 9.12: the player's guide, byte for byte the generated one.
            self.assertEqual(
                package.read("ostrog-windows-x64-portable/READ ME FIRST.txt"),
                (SOURCE_ROOT / "docs/privacy/READ-ME-FIRST.txt").read_bytes(),
            )
            # WP-09 9.11: every file shipped to a friend has a row in the files register.
            root = "ostrog-windows-x64-portable/"
            shipped = ["install:" + n[len(root):] for n in names if n.startswith(root) and n != root]
            self.assertEqual(
                files_register.unlisted(shipped, files_register.load_register()), []
            )
            config = package.read(
                "ostrog-windows-x64-portable/launcher.toml"
            ).decode("utf-8")
        self.assertIn(
            'manifest_url = "https://updates.example.com/stable/windows-x64.json"',
            config,
        )
        self.assertIn(
            'public_key = "8b7bd93f1f380ead6e015c6f68d87a301fa4a21c55f29ecab1098d5957e4ae7f"',
            config,
        )
        self.assertIn('application_dir = "app"', config)
        self.assertIn('user_data_dir = "data"', config)
        self.assertNotIn("github_token", config.lower())


if __name__ == "__main__":
    unittest.main()
