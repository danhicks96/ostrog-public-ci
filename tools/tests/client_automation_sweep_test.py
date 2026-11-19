#!/usr/bin/env python3
"""The automation sweep's first increment (#294 / #378, WP-08 8.9): run the client headless
through the --automation-json surface (#721) and validate the contract end-to-end, instead of
only grepping for it (tools/tests/client_automation_guard_test.py). Two runs of one seed must
agree byte for byte; a second seed must change the world identity."""

import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GODOT = ROOT / "tools" / "godot"
PROJECT = ROOT / "godot"


def godot_command() -> list[str]:
    """`tools/godot` is a bash script. Windows cannot start it directly (WinError 193 under
    Git Bash's CI, FIXLOG 2026-11-13), so run it through Git Bash there, as the package tests do."""
    if os.name != "nt":
        return [str(GODOT)]
    bash = os.environ.get("OSTROG_GIT_BASH")
    if not bash:
        candidate = Path(r"C:\Program Files\Git\bin\bash.exe")
        bash = str(candidate) if candidate.is_file() else shutil.which("bash")
    return [bash or "bash", str(GODOT)]


def run(
    seed: int,
    out: Path,
    inspect_id: int | None = None,
    inspect_block: tuple[int, int, int] | None = None,
) -> subprocess.CompletedProcess:
    args = [
        *godot_command(),
        "--path",
        ".",
        "--headless",
        "--",  # user args begin here (OS.get_cmdline_user_args), as in tools/shots/shot.sh
        "--stage=peoples",
        f"--seed={seed}",
        f"--automation-json={out}",
    ]
    if inspect_id is not None:
        args.append(f"--inspect={inspect_id}")
    if inspect_block is not None:
        args.append(
            f"--inspect-block={inspect_block[0]},{inspect_block[1]},{inspect_block[2]}"
        )
    return subprocess.run(
        args,
        cwd=PROJECT,
        capture_output=True,
        text=True,
        timeout=180,
    )


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="hf-auto-sweep-") as td:
        tmp = Path(td)
        a, b, c = tmp / "a.json", tmp / "b.json", tmp / "c.json"
        runs = [(7, a), (7, b), (8, c)]
        for seed, out in runs:
            p = run(seed, out)
            if p.returncode != 0 or not out.exists():
                print(f"sweep: seed {seed}: run failed rc={p.returncode}", file=sys.stderr)
                print(p.stdout[-500:], file=sys.stderr)
                print(p.stderr[-500:], file=sys.stderr)
                sys.exit(1)

        raw_a = a.read_text(encoding="utf-8")
        # The _json_safe normalization must keep non-finite floats out of the JSON entirely.
        for bad in ("NaN", "Infinity", "-Infinity"):
            if bad in raw_a:
                print(f"sweep: raw {bad} leaked into the automation JSON", file=sys.stderr)
                sys.exit(1)

        for seed, out in runs:
            d = json.loads(out.read_text(encoding="utf-8"))
            if d.get("schema") != 1:
                print(f"sweep: schema != 1 ({d.get('schema')})", file=sys.stderr)
                sys.exit(1)
            if d.get("world_seed") != seed:
                print(
                    f"sweep: world_seed {d.get('world_seed')} != --seed {seed}",
                    file=sys.stderr,
                )
                sys.exit(1)
            if not isinstance(d.get("tick"), int) or d["tick"] < 0:
                print(f"sweep: tick missing or negative ({d.get('tick')})", file=sys.stderr)
                sys.exit(1)
            if not d.get("world_name") or not d.get("display"):
                print("sweep: world_name/display empty", file=sys.stderr)
                sys.exit(1)
            # No --inspect-id/--inspect-block: both read models must be empty objects.
            if d.get("inspect") != {} or d.get("inspect_block") != {}:
                print("sweep: inspect models present without inspect args", file=sys.stderr)
                sys.exit(1)
            for key in ("shot", "footage"):
                if not isinstance(d.get(key), str):
                    print(f"sweep: {key} is not a string", file=sys.stderr)
                    sys.exit(1)

        if a.read_bytes() != b.read_bytes():
            print("sweep: two runs of seed 7 produced different JSON", file=sys.stderr)
            sys.exit(1)
        if a.read_bytes() == c.read_bytes():
            print("sweep: seeds 7 and 8 produced identical JSON", file=sys.stderr)
            sys.exit(1)

        # Second increment (WP-08 8.9): the inspect read model. Two runs with --inspect-id=0
        # must agree byte for byte, and the model must carry the bounded read-model fields.
        d, e = tmp / "d.json", tmp / "e.json"
        for out in (d, e):
            p = run(7, out, inspect_id=0)
            if p.returncode != 0 or not out.exists():
                print(f"sweep: inspect run failed rc={p.returncode}", file=sys.stderr)
                sys.exit(1)
        for out in (d, e):
            m = json.loads(out.read_text(encoding="utf-8"))["inspect"]
            if not isinstance(m, dict) or not m:
                print("sweep: inspect model missing or empty", file=sys.stderr)
                sys.exit(1)
            if not m.get("name") or not isinstance(m.get("name"), str):
                print(f"sweep: inspect name missing ({m.get('name')!r})", file=sys.stderr)
                sys.exit(1)
            age = m.get("age")
            if not isinstance(age, int) or age < 16:
                print(f"sweep: inspect age implausible ({age!r})", file=sys.stderr)
                sys.exit(1)
            if not isinstance(m.get("past"), list):
                print("sweep: inspect past is not a list", file=sys.stderr)
                sys.exit(1)
        if d.read_bytes() != e.read_bytes():
            print("sweep: two inspect runs of seed 7 differed", file=sys.stderr)
            sys.exit(1)

        # Third increment (WP-08 8.9): the block read model. The valley's centre comes from
        # the client's own "world ... centre (x, y)" line, and the block at it must read back
        # with the same coordinates and its facts present.
        import re

        m = re.search(r"centre \((\d+), (\d+), \d+\)", p.stdout)
        if not m:
            print("sweep: no centre in the client output to inspect", file=sys.stderr)
            sys.exit(1)
        cx, cy = int(m.group(1)), int(m.group(2))
        f = tmp / "f.json"
        p = run(7, f, inspect_block=(cx, cy, 9999))
        if p.returncode != 0 or not f.exists():
            print(f"sweep: block run failed rc={p.returncode}", file=sys.stderr)
            sys.exit(1)
        b = json.loads(f.read_text(encoding="utf-8"))["inspect_block"]
        if not isinstance(b, dict) or not b:
            print("sweep: block model missing or empty", file=sys.stderr)
            sys.exit(1)
        if b.get("x") != cx or b.get("y") != cy:
            print(
                f"sweep: block coordinates {b.get('x')},{b.get('y')} != {cx},{cy}",
                file=sys.stderr,
            )
            sys.exit(1)
        for key in ("title", "biome", "ground"):
            if not b.get(key) or not isinstance(b.get(key), str):
                print(f"sweep: block {key} missing ({b.get(key)!r})", file=sys.stderr)
                sys.exit(1)

    print("client automation sweep OK")


if __name__ == "__main__":
    main()
