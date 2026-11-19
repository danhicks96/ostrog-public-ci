#!/usr/bin/env bash
# Regenerate every art asset, check the palette, write provenance, render contact sheets.
# Usage: tools/art/build_all.sh   (from anywhere)
set -euo pipefail
cd "$(dirname "$0")"
python3 replacement_policy.py
for gen in palette terrain underground overlays structures plants items light effects; do
    python3 "$gen.py"
done
# people.py / animals.py are owned by another generator pass; run them separately.
python3 palette_check.py
python3 provenance.py
python3 contact_sheet.py
