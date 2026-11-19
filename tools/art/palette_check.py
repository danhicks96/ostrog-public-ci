#!/usr/bin/env python3
"""Fail (exit 1) if any opaque pixel under godot/assets is not a palette colour.
Alpha-mask textures are exempt: light/soft.png, overlays/edge_blend.png, overlays/cliff_shadow.png.
Semi-transparent pixels (0 < alpha < 255) still have to use palette colours; only fully
transparent pixels are ignored."""
import os
import sys

import numpy as np
from PIL import Image

from pal import Palette, ROOT

ASSETS = os.path.join(ROOT, 'godot', 'assets')
EXEMPT = {'light/soft.png', 'overlays/edge_blend.png', 'overlays/cliff_shadow.png', 'palette/ostrog.png'}


def check(path, pal):
    im = np.asarray(Image.open(path).convert('RGBA'))
    vis = im[:, :, 3] > 0
    rgb = im[vis][:, :3]
    if len(rgb) == 0:
        return []
    packed = rgb[:, 0].astype(np.int64) * 65536 + rgb[:, 1].astype(np.int64) * 256 + rgb[:, 2]
    ok = np.array([c[0] * 65536 + c[1] * 256 + c[2] for c in pal.colours], dtype=np.int64)
    bad = np.unique(packed[~np.isin(packed, ok)])
    return ['#%06x' % int(b) for b in bad]


def main():
    pal = Palette.load()
    failures = 0
    files = 0
    for root, _, names in os.walk(ASSETS):
        for n in sorted(names):
            if not n.endswith('.png'):
                continue
            path = os.path.join(root, n)
            # EXEMPT is written with '/'; relpath uses '\\' on Windows (issue 81).
            rel = os.path.relpath(path, ASSETS).replace(os.sep, '/')
            if rel in EXEMPT:
                continue
            files += 1
            bad = check(path, pal)
            if bad:
                failures += 1
                print('OFF-PALETTE %s: %s%s' % (rel, ' '.join(bad[:8]), ' ...' if len(bad) > 8 else ''))
    print('palette_check: %d files, %d off-palette' % (files, failures))
    sys.exit(1 if failures else 0)


if __name__ == '__main__':
    main()
