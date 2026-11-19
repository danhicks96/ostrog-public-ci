#!/usr/bin/env bash
# Build the friend-facing Windows x64 app and portable launcher packages.
# Run from Git Bash or WSL on a Windows release checkout. This intentionally does not require
# the Linux bridge or a Linux export; those belong to the separate multi-platform package job.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
VERSION="${1:-$(git rev-parse --short HEAD)}"
MANIFEST_URL="${OSTROG_MANIFEST_URL:-https://updates.example.com/stable/windows-x64.json}"
PUBLIC_KEY="${OSTROG_PUBLIC_KEY:-8b7bd93f1f380ead6e015c6f68d87a301fa4a21c55f29ecab1098d5957e4ae7f}"

mkdir -p godot/bin/windows releases
# A friend's PC may not have the Visual C++ runtime: the launcher and bridge must not need
# VCRUNTIME140.dll (static C runtime, .cargo/config.toml; FIXLOG 2026-11-13).
for bin in target/x86_64-pc-windows-msvc/release/hf_godot.dll \
  target/x86_64-pc-windows-msvc/release/hf-launcher.exe; do
  if grep -a -q -i 'vcruntime140' "$bin"; then
    echo "$bin needs VCRUNTIME140.dll; build it with the static C runtime (.cargo/config.toml)" >&2
    exit 1
  fi
done
cp target/x86_64-pc-windows-msvc/release/hf_godot.dll godot/bin/windows/hf_godot.dll
rm -rf releases/ostrog-windows-x64 releases/ostrog-windows-x64-portable
mkdir -p releases/ostrog-windows-x64 releases/ostrog-windows-x64-portable/app \
  releases/ostrog-windows-x64-portable/data

(cd godot && timeout 900 "$ROOT/tools/godot" --headless --path . \
  --export-release "Windows Desktop" ../releases/ostrog-windows-x64/ostrog.exe)
test -s releases/ostrog-windows-x64/ostrog.exe
test -s releases/ostrog-windows-x64/hf_godot.dll
cp docs/REFERENCE.md releases/ostrog-windows-x64/README.md
echo "$VERSION" > releases/ostrog-windows-x64/VERSION

cp -a releases/ostrog-windows-x64/. releases/ostrog-windows-x64-portable/app/
cp target/x86_64-pc-windows-msvc/release/hf-launcher.exe \
  releases/ostrog-windows-x64-portable/ostrog-launcher.exe
cat > releases/ostrog-windows-x64-portable/launcher.toml <<EOF
manifest_url = "$MANIFEST_URL"
public_key = "$PUBLIC_KEY"
current_version = "$VERSION"
channel = "stable"
timeout_seconds = 10
check_enabled = true
deferred_until = 0
application_dir = "app"
user_data_dir = "data"
executable = "ostrog.exe"
EOF
cp docs/REFERENCE.md releases/ostrog-windows-x64-portable/README.md
# The player's guide to every file Ostrog makes (WP-09 9.12; tools/privacy/files_guide.py).
cp docs/privacy/READ-ME-FIRST.txt "releases/ostrog-windows-x64-portable/READ ME FIRST.txt"
echo "$VERSION" > releases/ostrog-windows-x64-portable/VERSION
touch releases/ostrog-windows-x64-portable/data/.keep

zip_tree() {
  local source="$1"
  local destination="$2"
  if command -v zip >/dev/null 2>&1; then
    (cd releases && zip -qr "$(basename "$destination")" "$(basename "$source")")
    return
  fi
  if ! command -v powershell.exe >/dev/null 2>&1; then
    echo "zip or powershell.exe is required to create Windows archives" >&2
    return 1
  fi
  if command -v cygpath >/dev/null 2>&1; then
    local source_win="$(cygpath -w "$source")"
    local destination_win="$(cygpath -w "$destination")"
  elif command -v wslpath >/dev/null 2>&1; then
    local source_win="$(wslpath -w "$source")"
    local destination_win="$(wslpath -w "$destination")"
  else
    echo "cygpath or wslpath is required for the powershell.exe archive fallback" >&2
    return 1
  fi
  powershell.exe -NoProfile -NonInteractive -Command \
    "Compress-Archive -Path '$source_win' -DestinationPath '$destination_win' -Force"
}

rm -f releases/ostrog-windows-x64.zip releases/ostrog-windows-x64-portable.zip releases/SHA256SUMS
zip_tree releases/ostrog-windows-x64 releases/ostrog-windows-x64.zip
zip_tree releases/ostrog-windows-x64-portable releases/ostrog-windows-x64-portable.zip
(cd releases && sha256sum ostrog-windows-x64.zip ostrog-windows-x64-portable.zip > SHA256SUMS)
rm -rf releases/ostrog-windows-x64 releases/ostrog-windows-x64-portable
ls -la releases
echo "WINDOWS PACKAGE OK $VERSION"
