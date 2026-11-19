#!/usr/bin/env bash
# Headless screenshot of the Godot client.
# usage: tools/shots/shot.sh OUT.png [--seed=N] [--steps=N] [--z=N] [--stage=demo] [extra godot user args]
# Display: HF_DISPLAY (default :96, the Xvfb software-GL server; :0 refuses frames from a
# non-session process on this box).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$1"; shift
case "$OUT" in /*) ;; *) OUT="$PWD/$OUT" ;; esac
mkdir -p "$(dirname "$OUT")"
export DISPLAY="${HF_DISPLAY:-:96}"
cd "$ROOT/godot"

# Every PNG and OGG under assets/ has its .import sidecar, and every file the sidecar names in
# .godot/imported exists and matches Godot's own record of it (the .md5 file beside it: the
# source's md5 is the source's now, the imported file's md5 is the file's). Prints what is
# missing, if anything.
import_complete() {
  python3 -I - <<'PY'
import glob, hashlib, os, re, sys
def md5(p):
    with open(p, 'rb') as f:
        return hashlib.md5(f.read()).hexdigest()
bad = []
for src in glob.glob('assets/**/*.png', recursive=True) + glob.glob('assets/**/*.ogg', recursive=True):
    side = src + '.import'
    if not os.path.exists(side):
        bad.append(src + ' (no .import)')
        continue
    m = re.search(r'^dest_files=\[(.*)\]', open(side, encoding='utf-8').read(), re.M)
    dests = re.findall(r'"res://([^"]+)"', m.group(1)) if m else []
    if not dests:
        bad.append(src + ' (no dest_files)')
    for d in dests:
        rec = os.path.splitext(d)[0] + '.md5'
        if not os.path.exists(d) or not os.path.exists(rec):
            bad.append(src + ' -> ' + d + ' (missing)')
            continue
        r = dict(re.findall(r'^(\w+)="([0-9a-f]*)"', open(rec, encoding='utf-8').read(), re.M))
        if r.get('source_md5') != md5(src) or r.get('dest_md5') != md5(d):
            bad.append(src + ' -> ' + d + ' (stale)')
if bad:
    print('%d assets not imported, e.g. %s' % (len(bad), '; '.join(bad[:3])))
    sys.exit(1)
PY
}

# Re-import when the cache is missing, incomplete, or any asset is newer than it; otherwise
# Godot draws stale imported textures (found 2026-10-16: regenerated sheets showed their old
# pixels).
#
# The import pass (`godot --headless --editor --quit`) sometimes dies (issue 42): Godot 4.6.3
# starts the threaded reimport, logs "/root: The caller thread can't call the function
# `propagate_notification()` on this node" from a worker thread, and crashes with SIGSEGV
# (once in 20 fresh imports here; apport logged four more SIGSEGV/SIGABRT on 2026-10-16). It is
# an engine-side race, and it leaves the cache part-written: .import sidecars that name
# textures which were never written, which the client used to draw as flat fallback colours.
# The exit code says nothing about whether the import finished, so the cache is checked file
# by file instead: a pass that exited badly after importing everything is fine, one that left
# the cache incomplete is run once more, and if it is still incomplete the shot is not taken.
# The stamp is only written for a complete cache. The pass's output, crash included, is kept
# in .godot/hf_import.log (and hf_import.retry.log for the second pass).
STAMP=.godot/.hf_import_stamp
if [ ! -f .godot/extension_list.cfg ] || [ ! -f "$STAMP" ] || [ -n "$(find assets -newer "$STAMP" \( -name '*.png' -o -name '*.ogg' \) -print -quit)" ] || ! import_complete >/dev/null; then
  mkdir -p .godot
  rc=0
  timeout 300 "$ROOT/tools/godot" --path . --headless --editor --quit >.godot/hf_import.log 2>&1 || rc=$?
  if ! why="$(import_complete)"; then
    echo "import pass exited $rc and left the cache incomplete ($why); running it once more" >&2
    rc=0
    timeout 300 "$ROOT/tools/godot" --path . --headless --editor --quit >.godot/hf_import.retry.log 2>&1 || rc=$?
    if ! why="$(import_complete)"; then
      echo "shot FAILED: the asset import is still incomplete after a retry (exit $rc): $why" >&2
      echo "see godot/.godot/hf_import.log and godot/.godot/hf_import.retry.log" >&2
      exit 1
    fi
  elif [ "$rc" -ne 0 ]; then
    echo "import pass exited $rc after importing everything (issue 42); continuing, log in godot/.godot/hf_import.log" >&2
  fi
  touch "$STAMP"
fi
LOG="${OUT%.png}.log"
timeout "${HF_SHOT_TIMEOUT:-120}" "$ROOT/tools/godot" --path . --rendering-driver opengl3 -- "--shot=$OUT" "$@" >"$LOG" 2>&1 || true
if grep -q "SHOT_SAVED" "$LOG" && [ -s "$OUT" ]; then
  echo "shot: $OUT"
else
  echo "shot FAILED, see $LOG" >&2
  grep -E "ERROR|error" "$LOG" | head -5 >&2
  exit 1
fi
