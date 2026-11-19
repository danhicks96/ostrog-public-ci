#!/usr/bin/env python3
"""Guard the server-side release step and the static host config (WP-02 slice 2.7)."""

import hashlib
import os
from pathlib import Path
import subprocess
import tempfile

SOURCE_ROOT = Path(__file__).resolve().parents[2]
SERVER = SOURCE_ROOT / "tools" / "server"
SCRIPT = SERVER / "pull_sign_publish.sh"


PREFIX = "k3Jx9Qm2vLp7Rt5Wz8Nc4B"


def run(tag, version, compare_status, *, env_extra=None, prefix=PREFIX, portable=True):
    with tempfile.TemporaryDirectory(prefix="hf-server-") as raw:
        root = Path(raw)
        bin_dir = root / "bin"
        bin_dir.mkdir()
        log = root / "calls.txt"
        key = root / "fixture.key"
        key.write_text("0" * 64)
        pub = root / "fixture.pub"
        pub.write_text("1" * 64)
        (bin_dir / "gh").write_text(
            "#!/usr/bin/env bash\n"
            f'echo "gh $*" >> "{log.as_posix()}"\n'
            'if [ "$1" = api ]; then echo ' + compare_status + "; exit 0; fi\n"
            'if [ "$1" = release ]; then\n'
            '  while [ "$#" -gt 0 ]; do [ "$1" = --dir ] && d="$2"; shift; done\n'
            '  echo fixture-archive > "$d/ostrog-windows-x64.zip"\n'
            + ('  echo fixture-portable > "$d/ostrog-windows-x64-portable.zip"\n' if portable else "")
            + "fi\n"
        )
        release_bin = root / "hf-release"
        release_bin.write_text(
            "#!/usr/bin/env bash\n"
            f'echo "hf-release $*" >> "{log.as_posix()}"\n'
            'while [ "$#" -gt 0 ]; do [ "$1" = --output ] && echo \'{"fixture":true}\' > "$2"; shift; done\n'
        )
        for p in (bin_dir / "gh", release_bin):
            p.chmod(0o755)
        server_root = root / "srv"
        server_root.mkdir()
        if prefix is not None:
            (server_root / "prefix.txt").write_text(prefix + "\n")
        env = dict(
            os.environ,
            PATH=f"{bin_dir.as_posix()}:{os.environ['PATH']}",
            HF_SERVER_ROOT=server_root.as_posix(),
            HF_RELEASE_BIN=release_bin.as_posix(),
            HF_SIGNING_KEY=key.as_posix(),
            HF_PUBLIC_KEY=pub.as_posix(),
            HF_PUBLIC_BASE="https://example.invalid",
        )
        env.update(env_extra or {})
        result = subprocess.run(
            ["bash", SCRIPT.as_posix(), tag, version], env=env, capture_output=True, text=True
        )
        calls = log.read_text() if log.exists() else ""
        public = server_root / "public"
        # Every file the job made under the served tree, relative to it.
        # The server layout is URL-shaped on every host; Path.__str__ uses backslashes on
        # Windows, which made this guard fail before it could inspect the published tree.
        files = sorted(p.relative_to(public).as_posix() for p in public.rglob("*") if p.is_file()) if public.exists() else []
        texts = {f: (public / f).read_text() for f in files if f.endswith((".html", ".sha256"))}
        return result, calls, files, texts


def main() -> None:
    ok, calls, files, texts = run("c169", "0.4.1", "behind")
    assert ok.returncode == 0, ok.stderr
    assert "PUBLISH OK c169 0.4.1" in ok.stdout
    # Everything public sits under the private prefix, and nothing sits outside it (slice 2.11a).
    assert files == sorted(
        [
            f"{PREFIX}/ostrog-windows-x64-portable-c169.zip",
            f"{PREFIX}/ostrog-windows-x64-portable-c169.zip.sha256",
            f"{PREFIX}/index.html",
            f"{PREFIX}/stable/ostrog-windows-x64-c169.zip",
            f"{PREFIX}/stable/windows-x64.json",
        ]
    ), files
    assert all(f.startswith(PREFIX + "/") for f in files), "a file is reachable outside the prefix"
    assert f"--archive-url https://example.invalid/{PREFIX}/stable/ostrog-windows-x64-c169.zip" in calls
    sign = calls.index("--signing-key")
    verify = calls.index("--verify-manifest")
    assert sign < verify, "must verify after signing"
    # The secret path is never printed by the job.
    assert PREFIX not in ok.stdout and PREFIX not in ok.stderr, "the prefix was printed"
    # The first-install page carries the real checksum and links the real file.
    portable = "ostrog-windows-x64-portable-c169.zip"
    digest = hashlib.sha256(b"fixture-portable\n").hexdigest()
    assert texts[f"{PREFIX}/{portable}.sha256"] == f"{digest}  {portable}\n"
    page = texts[f"{PREFIX}/index.html"]
    assert "@ZIP@" not in page and "@SHA256@" not in page and "@VERSION@" not in page and "@TAG@" not in page
    assert f'href="{portable}"' in page and digest in page and "0.4.1" in page
    bad, calls, files, _ = run("c169", "0.4.1", "diverged")
    assert bad.returncode != 0 and "not on master" in bad.stderr
    assert files == [] and "hf-release" not in calls, "unmerged tag must never be signed"

    # A missing, short or unsafe prefix refuses before anything is downloaded or signed.
    for label, prefix in (("missing", None), ("short", "abc123"), ("slash", "k3Jx9Qm2vLp7Rt5Wz8Nc4B/../x"), ("space", "k3Jx9Qm2vLp7 Rt5Wz8Nc4B")):
        r, calls, files, _ = run("c169", "0.4.1", "behind", prefix=prefix)
        assert r.returncode != 0, label
        assert files == [] and "gh release" not in calls and "hf-release" not in calls, label

    # A release without the first-install package publishes nothing at all.
    r, calls, files, _ = run("c169", "0.4.1", "behind", portable=False)
    assert r.returncode != 0 and "first-install package missing" in r.stderr
    assert files == [] and "hf-release" not in calls, "nothing may be signed or published without it"

    for tag, version in (("v1", "0.4.1"), ("c169", "0.4"), ("c169;rm", "0.4.1")):
        r, _, files, _ = run(tag, version, "behind")
        assert r.returncode == 2 and files == [], (tag, version)

    # The script and configs never read, print or copy the private key.
    text = SCRIPT.read_text()
    for forbidden in ("cat \"$SIGNING_KEY\"", "cp \"$SIGNING_KEY\"", "echo \"$SIGNING_KEY\"", "xxd", "base64"):
        assert forbidden not in text, forbidden
    caddy = (SERVER / "Caddyfile.ostrog").read_text()
    assert "reverse_proxy" not in caddy and "file_server" in caddy
    assert "@not_read not method GET HEAD" in caddy
    nft = (SERVER / "ostrog-limits.nft").read_text()
    assert "flush ruleset" not in nft and "table inet ostrog" in nft
    # A reload replaces the table instead of stacking duplicate rules.
    lines = [ln.strip() for ln in nft.splitlines() if ln.strip() and not ln.startswith("#")]
    assert lines[:3] == ["add table inet ostrog", "delete table inet ostrog", "table inet ostrog {"], lines[:3]
    limits = (SERVER / "ostrog-limits.service").read_text()
    assert "Before=ostrog-caddy.service" in limits and "Type=oneshot" in limits
    assert "ExecStart=/usr/sbin/nft -f /srv/ostrog/ostrog-limits.nft" in limits
    assert "ExecStop=/usr/sbin/nft delete table inet ostrog" in limits
    assert "flush ruleset" not in limits and "WantedBy=multi-user.target" in limits
    unit = (SERVER / "ostrog-caddy.service").read_text()
    assert "CPUQuota=" in unit and "ReadOnlyPaths=/srv/ostrog" in unit
    print("server host guard OK")


if __name__ == "__main__":
    main()
