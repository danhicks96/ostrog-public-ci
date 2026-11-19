#!/usr/bin/env python3
"""Guard the public snapshot exporter against unsafe exclusions and metadata leaks."""

from __future__ import annotations

import shutil
import subprocess
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "tools" / "publish" / "make-snapshot.sh"


def run(*args: str, cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, cwd=cwd, text=True, capture_output=True, check=False)


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="ostrog-snapshot-guard-") as raw:
        temp = Path(raw)
        repo = temp / "repo"
        outside = temp / "outside"
        output = temp / "output"
        publish = repo / "tools" / "publish"
        publish.mkdir(parents=True)
        outside.write_text("keep me\n", encoding="utf-8")
        (repo / "private.txt").write_text("private\n", encoding="utf-8")
        shutil.copy2(SCRIPT, publish / SCRIPT.name)
        for name in ('docs_policy.py', 'public_text.py', 'public-ci.yml'):
            shutil.copy2(ROOT / 'tools/publish' / name, publish / name)
        (publish / 'public-docs.txt').write_text('fixture.md\n')
        (publish / "scrub.sed").write_text("", encoding="utf-8")
        (publish / "forbidden.txt").write_text("", encoding="utf-8")
        (repo / "README.md").write_text("snapshot test\n", encoding="utf-8")
        # This exclusion/metadata fixture models the art gate, while
        # fallback_integration_test.py exercises its real implementation.
        art = repo / 'tools/art'
        art.mkdir()
        (art / 'provenance_check.py').write_text('print("fixture art gate invoked")\n')
        assert run("git", "init", "-q", "-b", "master", cwd=repo).returncode == 0
        run("git", "config", "user.name", "Ostrog Guard", cwd=repo)
        run("git", "config", "user.email", "ostrog-guard@users.noreply.github.com", cwd=repo)
        assert run("git", "add", ".", cwd=repo).returncode == 0
        assert run("git", "commit", "-qm", "fixture", cwd=repo).returncode == 0

        for exclusion in ("../outside", "private.txt ../outside", "folder/..", ".", "/tmp",
                          "E:/outside", "folder\\..\\outside"):
            (publish / "exclude.txt").write_bytes((exclusion + "\r\n").encode())
            result = run("bash", (publish / SCRIPT.name).as_posix(), output.as_posix(), "HEAD", cwd=repo)
            assert result.returncode != 0, result.stdout + result.stderr
            assert outside.read_text(encoding="utf-8") == "keep me\n"
            assert "unsafe snapshot exclusion" in result.stderr, result.stdout + result.stderr
            shutil.rmtree(output)
        (publish / "exclude.txt").write_bytes(b"private.txt\r\n")
        result = run("bash", (publish / SCRIPT.name).as_posix(), output.as_posix(), "HEAD", cwd=repo)
        assert result.returncode == 0, result.stdout + result.stderr
        assert "fixture art gate invoked" in result.stdout
        assert not (output / "private.txt").exists()
        metadata = run(
            "git",
            "-C",
            str(output),
            "log",
            "-1",
            "--format=%an <%ae>%n%cn <%ce>%n%B",
            cwd=repo,
        ).stdout
        assert metadata.strip().splitlines() == [
            "Ostrog Maintainer <ostrog-maintainer@users.noreply.github.com>",
            "Ostrog Maintainer <ostrog-maintainer@users.noreply.github.com>",
            "Ostrog: first public release",
        ], metadata

        (art / 'provenance_check.py').write_text('raise SystemExit(1)\n')
        assert run('git', 'add', '.', cwd=repo).returncode == 0
        assert run('git', 'commit', '-qm', 'failing toy art gate', cwd=repo).returncode == 0
        failed = temp / 'failed-art-export'
        result = run('bash', str(publish / SCRIPT.name), str(failed), 'HEAD', cwd=repo)
        assert result.returncode != 0
        assert not (failed / '.git').exists(), 'art failure must stop before snapshot commit'


if __name__ == "__main__":
    main()
