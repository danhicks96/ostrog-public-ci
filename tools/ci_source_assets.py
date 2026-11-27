#!/usr/bin/env python3
"""Build ephemeral PNG test inputs from tracked recipes; never fetch or ship media.
FIXLOG 2026-11-19: source-only CI keeps asset coverage without committed media.
"""
from pathlib import Path
import argparse
import hashlib
import json
import os
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]

def load_rows(path):
    rows = json.loads(path.read_text())
    if not isinstance(rows, list) or not rows:
        raise ValueError('missing or empty asset manifest')
    seen = set()
    for row in rows:
        p = row['path']
        if (not isinstance(p, str) or p.startswith('/') or '\\' in p or ':' in p
                or any(x in ('', '.', '..') for x in p.split('/')) or p in seen
                or len(row['sha256']) != 64
                or any(c not in '0123456789abcdef' for c in row['sha256'])):
            raise ValueError('invalid asset manifest entry')
        seen.add(p)
    return rows

def regular(root, path):
    p = root / path
    if p.is_symlink() or any(x.is_symlink() for x in p.parents) or not p.is_file():
        raise ValueError('missing or linked required asset input')
    return p

def verify_rows(root, rows):
    for row in rows:
        if hashlib.sha256(regular(root, row['path']).read_bytes()).hexdigest() != row['sha256']:
            raise ValueError('asset input/output hash mismatch')

def verify_outputs(root, rows):
    expected = {x['path'] for x in rows}
    if not all(p.startswith('godot/assets/') and p.endswith('.png') for p in expected):
        raise ValueError('unexpected output scope')
    actual = set()
    for p in (root / 'godot/assets').rglob('*'):
        if p.is_symlink():
            raise ValueError('linked asset output')
        if p.suffix.lower() in ('.png', '.ogg', '.wav', '.jpg', '.webp'):
            actual.add(p.relative_to(root).as_posix())
    if actual != expected:
        raise ValueError('missing or unexpected generated media')
    verify_rows(root, rows)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', help='verify generated outputs without generating')
    args = parser.parse_args()
    outputs = load_rows(ROOT / 'tools/art/ci-pngs.json')
    if len(outputs) != 2828:
        raise ValueError('incomplete expected PNG inventory')
    if args.check:
        verify_outputs(ROOT, outputs)
        print('source assets: 2828 exact ephemeral PNGs; no audio')
        return
    inputs = load_rows(ROOT / 'tools/art/ci-inputs.json')
    verify_rows(ROOT, inputs)
    if any(p.suffix.lower() in ('.png', '.ogg', '.wav', '.jpg', '.webp')
           for p in (ROOT / 'godot/assets').rglob('*')):
        raise ValueError('bootstrap requires a pristine media-free checkout')
    # Directories absent from Git must exist before the pinned fallback writer runs.
    for row in outputs:
        (ROOT / row['path']).parent.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE='1', OPENBLAS_NUM_THREADS='1')
    steps = [('fallback/generate.py', []), ('palette.py', []), ('terrain.py', []),
             ('underground.py', []), ('overlays.py', []), ('structures.py', []),
             ('plants.py', []), ('items.py', ['--big']), ('light.py', []),
             ('effects.py', []), ('animals.py', []), ('people.py', [])]
    for name, flags in steps:
        subprocess.run([sys.executable, str(ROOT / 'tools/art' / name), *flags],
                       cwd=ROOT, env=env, check=True, timeout=900)
    verify_outputs(ROOT, outputs)
    print('source assets: 2828 exact ephemeral PNGs; no audio')

if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as exc:
        print('source assets: ' + str(exc), file=sys.stderr)
        sys.exit(1)
