#!/usr/bin/env bash
# Publish already-built Windows release assets to the private GitHub Release staging area or
# verify a signed manifest + archive pair before mirroring. GH_TOKEN is supplied by the CI/release
# environment only; it is never written into a package or passed as a command argument.
#
# Modes:
#   5 arguments (build off-VPS -> stage on private GitHub Release; signing happens on the VPS):
#     publish_windows.sh TAG ARCHIVE PORTABLE CHECKSUMS NOTES
#   6 arguments (signed manifest + archive verification + GitHub Release upload):
#     publish_windows.sh TAG MANIFEST ARCHIVE PORTABLE CHECKSUMS NOTES
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
if [ "$#" -eq 5 ]; then
  TAG="$1"
  MANIFEST=""
  ARCHIVE="$2"
  PORTABLE="$3"
  CHECKSUMS="$4"
  NOTES="$5"
elif [ "$#" -eq 6 ]; then
  TAG="$1"
  MANIFEST="$2"
  ARCHIVE="$3"
  PORTABLE="$4"
  CHECKSUMS="$5"
  NOTES="$6"
else
  echo "usage: $0 TAG [MANIFEST] ARCHIVE PORTABLE CHECKSUMS NOTES" >&2
  exit 2
fi
REPO="${OSTROG_GITHUB_REPO:-danhicks96/ostrog}"
: "${GH_TOKEN:?set GH_TOKEN only in the release environment}"

for path in "$ARCHIVE" "$PORTABLE" "$CHECKSUMS" "$NOTES"; do
  test -s "$path" || { echo "release asset is missing or empty: $path" >&2; exit 1; }
done
if [ -n "$MANIFEST" ]; then
  : "${OSTROG_RELEASE_PUBLIC_KEY_FILE:?set OSTROG_RELEASE_PUBLIC_KEY_FILE to the external Ed25519 public key file}"
  test -s "$MANIFEST" || { echo "release asset is missing or empty: $MANIFEST" >&2; exit 1; }
fi

python3 - "$MANIFEST" "$ARCHIVE" "$PORTABLE" <<'PY'
import hashlib
import json
import sys
import tomllib
import zipfile
from pathlib import Path

manifest_arg, archive_path, portable_path = sys.argv[1:]
archive_path = Path(archive_path)
portable_path = Path(portable_path)

def check_non_github_https(url: str, label: str) -> None:
    if not url.startswith("https://"):
        raise SystemExit(f"{label} must be anonymous HTTPS")
    lowered = url.lower()
    if "github.com" in lowered or "githubusercontent.com" in lowered:
        raise SystemExit(
            f"{label} must point at the friend-installer mirror host, not private GitHub ({url})"
        )

if manifest_arg:
    manifest_path = Path(manifest_arg)
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("schema") != 1:
        raise SystemExit("release manifest schema must be 1")
    if manifest.get("platform") != "windows" or manifest.get("architecture") != "x86_64":
        raise SystemExit("release manifest must target Windows x64")
    check_non_github_https(manifest.get("archive_url", ""), "release archive URL")
    actual = hashlib.sha256(archive_path.read_bytes()).hexdigest()
    if manifest.get("archive_sha256") != actual:
        raise SystemExit("manifest hash does not match the app archive")

root = "ostrog-windows-x64-portable/"
required = {
    root + "ostrog-launcher.exe",
    root + "launcher.toml",
    root + "README.md",
    root + "VERSION",
    root + "app/ostrog.exe",
    root + "app/hf_godot.dll",
    root + "data/.keep",
}
with zipfile.ZipFile(portable_path) as package:
    names = {info.filename for info in package.infolist() if not info.is_dir()}
    missing = sorted(required - names)
    if missing:
        raise SystemExit("portable package is missing: " + ", ".join(missing))
    if root + "app/ostrog-launcher.exe" in names:
        raise SystemExit("portable launcher must remain outside app/")
    data_files = sorted(
        name for name in names if name.startswith(root + "data/") and name != root + "data/.keep"
    )
    if data_files:
        raise SystemExit("portable package must not ship user data: " + ", ".join(data_files))
    launcher_cfg = tomllib.loads(package.read(root + "launcher.toml").decode("utf-8"))
    check_non_github_https(str(launcher_cfg.get("manifest_url", "")), "portable launcher.toml manifest_url")
    pub_key = str(launcher_cfg.get("public_key", ""))
    if len(pub_key) != 64 or any(ch not in "0123456789abcdef" for ch in pub_key):
        raise SystemExit("portable launcher.toml public_key must be 64 lowercase hex characters")
PY

if [ -n "$MANIFEST" ]; then
  cargo run --quiet --release -p hf-release --manifest-path "$ROOT/Cargo.toml" -- \
    --verify-manifest "$MANIFEST" \
    --archive "$ARCHIVE" \
    --public-key "$OSTROG_RELEASE_PUBLIC_KEY_FILE" \
    --channel "${OSTROG_RELEASE_CHANNEL:-stable}"
  case "$(basename "$MANIFEST")" in
    ostrog-windows-x64.json|windows-x64.json) ;;
    *) echo "unexpected manifest filename: $(basename "$MANIFEST")" >&2; exit 1 ;;
  esac
fi

test "$(basename "$ARCHIVE")" = "ostrog-windows-x64.zip"
test "$(basename "$PORTABLE")" = "ostrog-windows-x64-portable.zip"
test "$(basename "$CHECKSUMS")" = "SHA256SUMS"

if [ -n "$MANIFEST" ]; then
  gh release create "$TAG" "$ARCHIVE" "$PORTABLE" "$MANIFEST" "$CHECKSUMS" \
    --repo "$REPO" \
    --verify-tag \
    --title "Ostrog $TAG" \
    --notes-file "$NOTES"
else
  gh release create "$TAG" "$ARCHIVE" "$PORTABLE" "$CHECKSUMS" \
    --repo "$REPO" \
    --verify-tag \
    --title "Ostrog $TAG" \
    --notes-file "$NOTES"
fi
echo "PUBLISH OK $TAG"
