#!/usr/bin/env bash
# Windowed #484 integration: exact-PNG, immutability, panel identity, 121-sample retention,
# and a three-day capture sequence. Requires the extension and imported assets (run tools/ci.sh
# or tools/worktree_godot.sh first). Never use --headless: there are no viewport pixels there.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="${1:?usage: tools/shots/timelapse.sh OUTPUT_DIR}"
case "$OUT" in /*) ;; *) OUT="$PWD/$OUT" ;; esac
mkdir -p "$OUT"
export DISPLAY="${HF_DISPLAY:-:96}"
export HF_TIMELAPSE_OUTPUT="$OUT"
cd "$ROOT/godot"
set +e
timeout "${HF_TIMELAPSE_TIMEOUT:-600}" "$ROOT/tools/godot" --path . --rendering-driver opengl3 \
  -s "$ROOT/tools/timelapse_visual.gd" -- --fresh --seed=484 --no-director \
  >"$OUT/run.log" 2>&1
rc=$?
set -e
if [ "$rc" -ne 0 ]; then
  echo "timelapse visual FAILED (exit $rc), see $OUT/run.log" >&2
  exit "$rc"
fi
python3 -I - "$OUT/report.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding='utf-8') as f:
    report = json.load(f)
if report.get('failed') != 0:
    raise SystemExit(f"timelapse visual FAILED: {report['failed']} checks; see {sys.argv[1]}")
print(f"timelapse visual OK: {report['checks']} checks; {sys.argv[1]}")
PY
