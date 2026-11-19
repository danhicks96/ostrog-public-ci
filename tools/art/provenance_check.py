#!/usr/bin/env python3
"""Check that every art file under godot/assets has a provenance row.

The art bible says this gate exists and tools/ci.sh runs it; in fact neither the script nor any
enforcement existed, so a file could be added to godot/assets with no recorded generator, source
or licence — which would ship unlicensed art (ASSETS.md policy: nothing without a licence record
ships). This closes that hole.

Stdlib only: ci.sh runs it with `python3 -I`, which hides user site-packages, so Pillow/numpy are
not available here. Rows are read from both provenance tables.
"""
import os
import sys
import tomllib

# Computed here rather than imported from pal.py: ci.sh runs this with `python3 -I`, and isolated
# mode does not put the script's own directory on sys.path, so sibling imports fail.
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))

ASSETS = os.path.join(ROOT, 'godot', 'assets')
TABLES = [
    os.path.join(ROOT, 'tools', 'art', 'provenance.toml'),
    os.path.join(ROOT, 'tools', 'art', 'provenance_people.toml'),
    os.path.join(ROOT, 'tools', 'audio', 'provenance_audio.toml'),
    os.path.join(ROOT, 'tools', 'audio', 'provenance_music.toml'),
]
# The palette sheet is the palette printed as an image, not an asset used in play; it already
# fails palette_check and cannot sensibly carry the same generator row.
EXEMPT = {'godot/assets/palette/ostrog.png'}


def asset_files():
    out = []
    for root, _, names in os.walk(ASSETS):
        for n in names:
            if n.endswith('.png') or n.endswith('.wav') or n.endswith('.ogg'):
                rel = os.path.relpath(os.path.join(root, n), ROOT).replace('\\', '/')
                out.append(rel)
    return sorted(out)


def listed_paths():
    paths = set()
    for t in TABLES:
        if not os.path.exists(t):
            continue
        with open(t, 'rb') as f:
            data = tomllib.load(f)
        for row in data.get('asset', []):
            p = row.get('path', '')
            if p:
                paths.add(p.replace('\\', '/'))
    return paths


def main():
    import importlib.util
    spec = importlib.util.spec_from_file_location("replacement_policy", os.path.join(ROOT, "tools/art/replacement_policy.py"))
    policy = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(policy)
    policy.check_provenance()
    files = [f for f in asset_files() if f not in EXEMPT]
    have = listed_paths()
    missing = [f for f in files if f not in have]
    if missing:
        print('provenance: %d of %d assets have no row' % (len(missing), len(files)))
        for m in missing[:25]:
            print('  %s' % m)
        if len(missing) > 25:
            print('  ... and %d more' % (len(missing) - 25))
        print('Run: python3 tools/art/provenance.py   # then this check again')
        return 1
    print('provenance: %d assets, all listed' % len(files))
    return 0


if __name__ == '__main__':
    sys.exit(main())
