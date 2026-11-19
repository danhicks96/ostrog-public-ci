#!/usr/bin/env bash
# Sign one already-built Windows x64 app archive. The key is read from an external file and is
# never copied into the package, launcher configuration or repository.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
if [ "$#" -ne 5 ]; then
  echo "usage: $0 VERSION ARCHIVE_URL ARCHIVE SIGNING_KEY OUTPUT" >&2
  exit 2
fi
VERSION="$1"
ARCHIVE_URL="$2"
ARCHIVE="$3"
SIGNING_KEY="$4"
OUTPUT="$5"
mkdir -p "$(dirname "$OUTPUT")"
cargo run --quiet --release -p hf-release -- \
  --version "$VERSION" \
  --channel stable \
  --archive-url "$ARCHIVE_URL" \
  --archive "$ARCHIVE" \
  --signing-key "$SIGNING_KEY" \
  --output "$OUTPUT"
