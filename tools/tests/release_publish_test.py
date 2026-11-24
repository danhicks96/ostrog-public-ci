#!/usr/bin/env python3
"""Guard the credential-free package/publication boundary for off-VPS staging and VPS mirror URLs."""

import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import zipfile


SOURCE_ROOT = Path(__file__).resolve().parents[2]
MIRROR_MANIFEST_URL = "https://updates.example.com/stable/windows-x64.json"
MIRROR_ARCHIVE_URL = "https://updates.example.com/stable/ostrog-windows-x64-c161.zip"
PUBLIC_KEY_HEX = "ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c"


def write_portable(
    path: Path,
    *,
    include_data: bool,
    manifest_url: str = MIRROR_MANIFEST_URL,
    public_key: str = PUBLIC_KEY_HEX,
) -> None:
    root = "ostrog-windows-x64-portable/"
    launcher_toml = (
        f'manifest_url = "{manifest_url}"\n'
        f'public_key = "{public_key}"\n'
        'current_version = "1.2.3"\n'
        'channel = "stable"\n'
        "timeout_seconds = 10\n"
        "check_enabled = true\n"
        "deferred_until = 0\n"
        'application_dir = "app"\n'
        'user_data_dir = "data"\n'
        'executable = "ostrog.exe"\n'
    ).encode("utf-8")
    files = {
        "ostrog-launcher.exe": b"fixture",
        "launcher.toml": launcher_toml,
        "README.md": b"fixture",
        "VERSION": b"1.2.3\n",
        "app/ostrog.exe": b"fixture",
        "app/hf_godot.dll": b"fixture",
    }
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as package:
        for name, content in files.items():
            package.writestr(root + name, content)
        if include_data:
            package.writestr(root + "data/.keep", b"")


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="hf-publish-") as raw:
        root = Path(raw)
        bin_dir = root / "bin"
        bin_dir.mkdir()
        record = root / "gh-args.txt"
        stub = bin_dir / "gh"
        stub.write_text(
            "#!/usr/bin/env python3\n"
            "import os, pathlib, sys\n"
            "pathlib.Path(os.environ['GH_RECORD']).write_text('\\n'.join(sys.argv[1:]))\n",
            encoding="utf-8",
        )
        stub.chmod(0o755)

        archive = root / "ostrog-windows-x64.zip"
        archive.write_bytes(b"app archive")
        manifest = root / "windows-x64.json"
        manifest.write_text(
            json.dumps(
                {
                    "schema": 1,
                    "version": "1.2.3",
                    "channel": "stable",
                    "platform": "windows",
                    "architecture": "x86_64",
                    "archive_url": MIRROR_ARCHIVE_URL,
                    "archive_sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
                    "signature": "00" * 64,
                }
            ),
            encoding="utf-8",
        )
        public_key = root / "public.key"
        public_key.write_text(PUBLIC_KEY_HEX + "\n", encoding="utf-8")
        signing_key = root / "signing.key"
        signing_key.write_bytes(bytes([7]) * 32)
        portable = root / "ostrog-windows-x64-portable.zip"
        write_portable(portable, include_data=True)
        checksums = root / "SHA256SUMS"
        checksums.write_text("checksums\n", encoding="utf-8")
        notes = root / "notes.md"
        notes.write_text("release notes\n", encoding="utf-8")

        env = os.environ.copy()
        env.update(
            {
                "PATH": str(bin_dir) + os.pathsep + env.get("PATH", ""),
                "GH_TOKEN": "test-token-never-an-argument",
                "GH_RECORD": str(record),
                "OSTROG_GITHUB_REPO": "example-owner/example-releases",
                "OSTROG_RELEASE_PUBLIC_KEY_FILE": str(public_key),
            }
        )
        bash = os.environ.get("OSTROG_GIT_BASH")
        if os.name == "nt" and not bash:
            candidate = Path(r"C:\Program Files\Git\bin\bash.exe")
            bash = str(candidate) if candidate.is_file() else None
        bash = bash or "bash"

        # 1. Off-VPS staging mode (5 args: build off-VPS, stage archive + portable + checksums on
        # private GitHub Release so the VPS job can pull, sign and mirror).
        staged = subprocess.run(
            [
                bash,
                "tools/release/publish_windows.sh",
                "c161",
                str(archive),
                str(portable),
                str(checksums),
                str(notes),
            ],
            cwd=SOURCE_ROOT,
            env=env,
            capture_output=True,
            text=True,
        )
        if staged.returncode != 0:
            raise AssertionError(staged.stdout + staged.stderr)
        staged_args = record.read_text(encoding="utf-8")
        assert "release\ncreate\nc161" in staged_args
        assert "--verify-tag" in staged_args
        assert "--repo\nexample-owner/example-releases" in staged_args
        assert "ostrog-windows-x64.zip" in staged_args
        assert "ostrog-windows-x64-portable.zip" in staged_args
        assert "windows-x64.json" not in staged_args
        assert "test-token-never-an-argument" not in staged_args
        record.unlink()

        # 2. Signed manifest + archive verification mode (6 args) with VPS mirror asset path.
        signed = subprocess.run(
            [
                bash,
                "tools/release/sign_manifest.sh",
                "1.2.3",
                MIRROR_ARCHIVE_URL,
                str(archive),
                str(signing_key),
                str(manifest),
            ],
            cwd=SOURCE_ROOT,
            capture_output=True,
            text=True,
        )
        if signed.returncode != 0:
            raise AssertionError(signed.stdout + signed.stderr)
        result = subprocess.run(
            [
                bash,
                "tools/release/publish_windows.sh",
                "c161",
                str(manifest),
                str(archive),
                str(portable),
                str(checksums),
                str(notes),
            ],
            cwd=SOURCE_ROOT,
            env=env,
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            raise AssertionError(result.stdout + result.stderr)
        args = record.read_text(encoding="utf-8")
        assert "release\ncreate\nc161" in args
        assert "--verify-tag" in args
        assert "--repo\nexample-owner/example-releases" in args
        assert "ostrog-windows-x64.zip" in args
        assert "ostrog-windows-x64-portable.zip" in args
        assert "windows-x64.json" in args
        assert "test-token-never-an-argument" not in args
        record.unlink()

        # 3. Reject a manifest whose archive_url points at private GitHub instead of the mirror.
        github_manifest = json.loads(manifest.read_text(encoding="utf-8"))
        github_manifest["archive_url"] = (
            "https://github.com/example-user/example-repository/releases/download/c161/ostrog-windows-x64.zip"
        )
        manifest.write_text(json.dumps(github_manifest), encoding="utf-8")
        rejected_github_url = subprocess.run(
            [
                bash,
                "tools/release/publish_windows.sh",
                "c161",
                str(manifest),
                str(archive),
                str(portable),
                str(checksums),
                str(notes),
            ],
            cwd=SOURCE_ROOT,
            env=env,
            capture_output=True,
            text=True,
        )
        assert rejected_github_url.returncode != 0
        assert not record.exists(), rejected_github_url.stdout + rejected_github_url.stderr

        # Restore valid signed manifest.
        subprocess.run(
            [
                bash,
                "tools/release/sign_manifest.sh",
                "1.2.3",
                MIRROR_ARCHIVE_URL,
                str(archive),
                str(signing_key),
                str(manifest),
            ],
            cwd=SOURCE_ROOT,
            check=True,
            capture_output=True,
            text=True,
        )

        # 4. Reject a portable package whose launcher.toml points at GitHub.
        write_portable(
            portable,
            include_data=True,
            manifest_url="https://github.com/example-user/example-repository/releases/latest/download/ostrog-windows-x64.json",
        )
        rejected_portable_url = subprocess.run(
            [
                bash,
                "tools/release/publish_windows.sh",
                "c161",
                str(archive),
                str(portable),
                str(checksums),
                str(notes),
            ],
            cwd=SOURCE_ROOT,
            env=env,
            capture_output=True,
            text=True,
        )
        assert rejected_portable_url.returncode != 0
        assert not record.exists(), rejected_portable_url.stdout + rejected_portable_url.stderr

        # 5. Reject malformed portable layout (missing data/.keep).
        write_portable(portable, include_data=False)
        rejected_layout = subprocess.run(
            [
                bash,
                "tools/release/publish_windows.sh",
                "c161",
                str(manifest),
                str(archive),
                str(portable),
                str(checksums),
                str(notes),
            ],
            cwd=SOURCE_ROOT,
            env=env,
            capture_output=True,
            text=True,
        )
        assert rejected_layout.returncode != 0
        assert not record.exists(), rejected_layout.stdout + rejected_layout.stderr

        # 6. Reject tampered signature.
        write_portable(portable, include_data=True)
        tampered = json.loads(manifest.read_text(encoding="utf-8"))
        tampered["signature"] = "00" * 64
        manifest.write_text(json.dumps(tampered), encoding="utf-8")
        rejected = subprocess.run(
            [
                bash,
                "tools/release/publish_windows.sh",
                "c161",
                str(manifest),
                str(archive),
                str(portable),
                str(checksums),
                str(notes),
            ],
            cwd=SOURCE_ROOT,
            env=env,
            capture_output=True,
            text=True,
        )
        assert rejected.returncode != 0
        assert not record.exists(), rejected.stdout + rejected.stderr
        print("RELEASE PUBLISH TEST OK")


if __name__ == "__main__":
    main()
