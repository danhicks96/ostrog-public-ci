#!/usr/bin/env bash
# Full local verification: tests, lints, asset provenance, canonical shots (when the client exists).
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
if [ -f Cargo.toml ]; then
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets -- -D warnings
  # The tests: with cargo-nextest (pinned 0.9.148, tools/boxci/README.md "Parallel tests") every
  # test runs as its own process, all binaries at once, so the long sim tests run side by side
  # instead of one binary after another. Without it (a fresh clone, the Windows machine) the
  # same tests run under `cargo test`. Either way the first failing test fails CI. nextest does
  # not run doctests, so those run separately.
  if cargo nextest --version > /dev/null 2>&1; then
    cargo nextest run --workspace --no-tests=warn
    cargo test --workspace --doc
  else
    echo "cargo-nextest not installed: running the tests with cargo test, one binary at a time"
    cargo test --workspace
  fi
fi
python3 -I tools/art/provenance_check.py   # mandatory: a missing checker is a failure
# The reference sheets and prompts are not in the public snapshot; skip the check there.
[ ! -d tools/art/sources/reference ] || python3 -I tools/art/prompt_library_check.py   # every derived sheet keeps its successful prompt
python3 tools/art/effects_check.py   # Pillow: user site, so no -I (#414)
python3 -I tools/art/provenance_report.py --check   # ASSETS.md counts match the tables (C54)
# Every pair of kinds of people must read apart by outline at 1x (C53).
python3 tools/art/silhouette_check.py --quiet   # numpy and Pillow: user site, so no -I
# The full drawn roster: every kind x build x age must read apart across kinds (#485). Strict on
# purpose: master's art predates it, so this is red until the production art PR closes the gaps
# listed in docs/design-character-visuals.md.
python3 tools/art/silhouette_check.py --full --quiet
# Ground and tree art a QA round judged wrong (C65, #201 #202).
python3 tools/art/ground_check.py
# Underground materials and fact-backed seepage/strata overlays (#414, #519).
python3 tools/art/underground_check.py
# Every flora variant and wildlife animation cell, plus this slice's authored material details.
python3 tools/art/plants_animals_check.py
# Audio levels and loop seams: fails on clipping or an audible click where a loop wraps.
python3 tools/audio/audio_report.py "${TMPDIR:-/tmp}/hf_audio_report.png" > /dev/null
# No tracked symlinks: a link committed from one machine dangles on every other (2026-11-12, #724
# committed `.tools` -> $HOME/ostrog/.tools, shadowing the project-local Godot).
links="$(git ls-files -s | awk '$1=="120000"{print $4}')"
if [ -n "$links" ]; then echo "tracked symlinks are not allowed: $links" >&2; exit 1; fi
# The README's generated tables (strings, assets, bridge layout, pipeline) must match the code.
python3 -I tools/docs/readme_tables.py --check
# Every client flag and CLI command and option is in the README (#188).
python3 -I tools/docs/readme_flags.py
python3 -I tools/tests/client_automation_guard_test.py
# The Windows watch-copy launcher uses real temporary Git repositories and stub build/client tools.
[ ! -e tools/watch-copy.sh ] || python3 -I tools/tests/watch_copy_test.py   # not in the public snapshot
# The release package keeps the launcher outside the replaceable app tree and preserves data on
# clean uninstall; this uses stub exports, not a network or a game build.
python3 -I tools/tests/package_layout_test.py
# The Windows-only release path must not require Linux bridge artifacts (#626).
python3 -I tools/tests/windows_package_test.py
# Every file the Windows install audit found, and every packaged file, is in the files register (WP-09 9.11).
python3 -I tools/tests/files_register_test.py
# READ ME FIRST.txt and the in-game files page are the register in plain words (WP-09 9.12).
if [ -e tools/server/index.html ]; then   # tools/server is not in the public snapshot
  python3 -I tools/privacy/files_guide.py --check
  python3 -I tools/tests/files_guide_test.py
fi
# The release publisher binds the manifest hash to the exact public app archive and never puts
# GH_TOKEN in the gh command arguments or any shipped file.
python3 -I tools/tests/release_publish_test.py
[ ! -d tools/server ] || python3 -I tools/tests/server_host_test.py
python3 -I tools/tests/frame_time_harness_test.py
python3 -I tools/tests/world_renderer_guard_test.py
# FIXLOG 2026-11-19: public structural CI and private offline audit are separate.
if [ -d tools/publish ]; then
  python3 -I tools/tests/publish_snapshot_guard_test.py
  python3 -I tools/tests/public_docs_guard_test.py
  python3 -I tools/tests/public_text_guard_test.py
  python3 -I tools/tests/history_quarantine_guard_test.py
  python3 -I tools/tests/snapshot_gate_guard_test.py
  python3 -I tools/tests/snapshot_ci_guard_test.py
  python3 -I tools/tests/public_hygiene_guard_test.py
  echo "private release privacy audit is separate; no clearance established by CI"
else
  python3 -I tools/check_export_structure.py
fi
python3 -I tools/tests/public_boundary_guard_test.py
python3 -I tools/tests/repo_settings_guard_test.py
# Saved enums keep their variants' positions: each enum at the last tag is an unchanged prefix
# of the same enum here (a merge once put a branch's new variants before master's, 2026-10-26).
if base=$(git tag --merged HEAD~1 --sort=-creatordate 2>/dev/null | head -1) && [ -n "$base" ]; then
  python3 -I tools/enum_order.py . "$base"
else
  echo "enum_order: no tag to compare against (shallow clone?), skipped"
fi
# Every GDScript must parse and compile (a client that failed to load once passed CI, C10).
if [ -x tools/godot ]; then
  # The box-ci container runs as a numeric uid with HOME=/ (no passwd entry), so Godot
  # cannot create its user data dir and segfaults on user://logs before the checks run
  # (PR #537, 2026-10-31): point HOME at a writable scratch dir when it is not one.
  if [ ! -w "${HOME:-/}" ]; then
    export HOME="${TMPDIR:-/tmp}/hf-godot-home"
    mkdir -p "$HOME"
  fi
  # A fresh clone or worktree has neither the extension library (gitignored) nor Godot's import
  # cache, and the check then fails with 'Could not find type "SimHost"' (#148): provide both.
  # `cargo test` does not build the extension (a cdylib), so build it here when it is missing.
  # The library by platform: on Windows (Git Bash) cargo writes hf_godot.dll and
  # ostrog.gdextension loads it from bin/windows (#198, QA 2026-10-20, C63: the Linux name
  # was copied everywhere, so a fresh Windows clone stopped here).
  read -r libdir lib < <(tools/godot_lib.sh)
  # Guard for #198: a Git Bash clone gets the Windows library, not the Linux one.
  [ "$(tools/godot_lib.sh MINGW64_NT-10.0-22631)" = "windows hf_godot.dll" ] || { echo "godot_lib.sh: wrong Windows library" >&2; exit 1; }
  fresh=0
  # Built and copied whenever it changed, not only when missing (C64: the client's tests ran
  # against the library of an earlier build, so a renamed bridge key passed CI).
  cargo build -q -p hf-godot
  if [ ! -f "godot/bin/$libdir/$lib" ] || ! cmp -s "target/debug/$lib" "godot/bin/$libdir/$lib"; then
    mkdir -p "godot/bin/$libdir" && cp "target/debug/$lib" "godot/bin/$libdir/$lib"
    fresh=1
  fi
  if [ "$fresh" = 1 ] || [ ! -f godot/.godot/extension_list.cfg ]; then
    (cd godot && timeout 900 ../tools/godot --path . --headless --import > /dev/null 2>&1 || true)
  fi
  out="$(cd godot && timeout 120 ../tools/godot --path . --headless -s "$ROOT/tools/gd_check.gd" 2>&1 || true)"
  if ! grep -q "GD_CHECK .* 0 failed" <<<"$out" || grep -q "SCRIPT ERROR" <<<"$out"; then
    grep -E "GD_CHECK|SCRIPT ERROR" <<<"$out" >&2
    echo "GDScript check failed" >&2
    exit 1
  fi
  # The client's guard tests (C63): rules.gd, each against a fault a QA round found.
  out="$(cd godot && timeout 120 ../tools/godot --path . --headless -s "$ROOT/tools/gd_test.gd" 2>&1 || true)"
  if ! grep -q "GD_TEST .* 0 failed" <<<"$out" || grep -q "SCRIPT ERROR" <<<"$out"; then
    grep -E "GD_TEST|failed:|SCRIPT ERROR" <<<"$out" >&2
    echo "GDScript tests failed" >&2
    exit 1
  fi
  # #294 (WP-08 8.9): the automation sweep — run the client through the --automation-json
  # surface and validate the contract end-to-end (schema, seed flow, determinism, NaN
  # normalization). Needs the godot binary and the import above.
  python3 -I tools/tests/client_automation_sweep_test.py || exit 1
fi
if [ -x tools/shots/canonical.sh ]; then tools/shots/canonical.sh; fi
echo "CI OK"
