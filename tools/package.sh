#!/usr/bin/env bash
# Build release packages: app archives plus portable launcher packages under releases/, with
# SHA256SUMS. The launcher package is separate from the app archive: the updater replaces only
# the app tree, while the launcher stays outside it.
# Needs: Godot export templates 4.6.3, the Linux/Windows bridge and launcher binaries, and the
# OSTROG_MANIFEST_URL and OSTROG_PUBLIC_KEY release inputs.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
VERSION="${1:-$(git rev-parse --short HEAD)}"
MANIFEST_URL="${OSTROG_MANIFEST_URL:-https://updates.example.com/stable/windows-x64.json}"
PUBLIC_KEY="${OSTROG_PUBLIC_KEY:-8b7bd93f1f380ead6e015c6f68d87a301fa4a21c55f29ecab1098d5957e4ae7f}"
mkdir -p godot/bin/linux godot/bin/windows releases
# A friend's PC may not have the Visual C++ runtime: the launcher and bridge must not need
# VCRUNTIME140.dll (static C runtime, .cargo/config.toml; FIXLOG 2026-11-13).
for bin in target/x86_64-pc-windows-msvc/release/hf_godot.dll \
  target/x86_64-pc-windows-msvc/release/hf-launcher.exe; do
  if grep -a -q -i 'vcruntime140' "$bin"; then
    echo "$bin needs VCRUNTIME140.dll; build it with the static C runtime (.cargo/config.toml)" >&2
    exit 1
  fi
done
cp target/release/libhf_godot.so godot/bin/linux/libhf_godot.so
strip --strip-debug godot/bin/linux/libhf_godot.so
cp target/x86_64-pc-windows-msvc/release/hf_godot.dll godot/bin/windows/hf_godot.dll
rm -rf releases/ostrog-linux-x86_64 releases/ostrog-windows-x64
mkdir -p releases/ostrog-linux-x86_64 releases/ostrog-windows-x64
(cd godot && timeout 900 "$ROOT/tools/godot" --headless --path . --export-release "Linux" ../releases/ostrog-linux-x86_64/ostrog.x86_64 >/dev/null 2>&1)
(cd godot && timeout 900 "$ROOT/tools/godot" --headless --path . --export-release "Windows Desktop" ../releases/ostrog-windows-x64/ostrog.exe >/dev/null 2>&1)
test -s releases/ostrog-linux-x86_64/ostrog.x86_64
test -s releases/ostrog-linux-x86_64/libhf_godot.so
test -s releases/ostrog-windows-x64/ostrog.exe
test -s releases/ostrog-windows-x64/hf_godot.dll
cp docs/REFERENCE.md releases/ostrog-linux-x86_64/README.md
cp docs/REFERENCE.md releases/ostrog-windows-x64/README.md
echo "$VERSION" > releases/ostrog-linux-x86_64/VERSION
echo "$VERSION" > releases/ostrog-windows-x64/VERSION
rm -rf releases/ostrog-linux-x86_64-portable releases/ostrog-windows-x64-portable
mkdir -p releases/ostrog-linux-x86_64-portable/app releases/ostrog-linux-x86_64-portable/data \
  releases/ostrog-windows-x64-portable/app releases/ostrog-windows-x64-portable/data
cp -a releases/ostrog-linux-x86_64/. releases/ostrog-linux-x86_64-portable/app/
cp -a releases/ostrog-windows-x64/. releases/ostrog-windows-x64-portable/app/
cp target/release/hf-launcher releases/ostrog-linux-x86_64-portable/ostrog-launcher
cp target/x86_64-pc-windows-msvc/release/hf-launcher.exe \
  releases/ostrog-windows-x64-portable/ostrog-launcher.exe
cat > releases/ostrog-linux-x86_64-portable/launcher.toml <<EOF
manifest_url = "$MANIFEST_URL"
public_key = "$PUBLIC_KEY"
current_version = "$VERSION"
channel = "stable"
timeout_seconds = 10
check_enabled = true
deferred_until = 0
application_dir = "app"
user_data_dir = "data"
executable = "ostrog.x86_64"
EOF
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
cp docs/REFERENCE.md releases/ostrog-linux-x86_64-portable/README.md
cp docs/REFERENCE.md releases/ostrog-windows-x64-portable/README.md
echo "$VERSION" > releases/ostrog-linux-x86_64-portable/VERSION
echo "$VERSION" > releases/ostrog-windows-x64-portable/VERSION
touch releases/ostrog-linux-x86_64-portable/data/.keep
touch releases/ostrog-windows-x64-portable/data/.keep
(cd releases && rm -f ostrog-linux-x86_64.tar.gz ostrog-windows-x64.zip \
  ostrog-linux-x86_64-portable.tar.gz ostrog-windows-x64-portable.zip SHA256SUMS \
  && tar czf ostrog-linux-x86_64.tar.gz ostrog-linux-x86_64 \
  && zip -qr ostrog-windows-x64.zip ostrog-windows-x64 \
  && tar czf ostrog-linux-x86_64-portable.tar.gz ostrog-linux-x86_64-portable \
  && zip -qr ostrog-windows-x64-portable.zip ostrog-windows-x64-portable \
  && sha256sum ostrog-linux-x86_64.tar.gz ostrog-windows-x64.zip \
    ostrog-linux-x86_64-portable.tar.gz ostrog-windows-x64-portable.zip > SHA256SUMS)
# The unpacked trees are build output; only the archives and sums are kept.
rm -rf releases/ostrog-linux-x86_64 releases/ostrog-windows-x64 \
  releases/ostrog-linux-x86_64-portable releases/ostrog-windows-x64-portable
ls -la releases
echo "PACKAGE OK $VERSION"
