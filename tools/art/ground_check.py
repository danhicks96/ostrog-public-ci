#!/usr/bin/env python3
"""Glance checks on ground and tree art that a QA round judged wrong (C65), each a number that
failed on the old art: tools/ci.sh runs it.

- #202 sand: at most 5 % of its pixels much darker than its base (the old sand had 14 %, two
  dark patches and six flecks a tile that read at 2x as a leopard print).
- #202 frozen soil (the "sand" of the issue's shot, a cold morning): at most 10 % of its
  pixels far from its median (the old had 25 %, brown blotches on white).
- #201 bone moss: well lighter than scree, greener than snow (the old moss was scree's grey).
- #201 bone tree: a white crown, at least 130 near-white pixels a variant (the old had 95).
"""
import os
import sys

import numpy as np
from PIL import Image

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'godot', 'assets')


def rgb(path):
    return np.asarray(Image.open(os.path.join(ROOT, path)).convert('RGBA'), float)


def luma(a):
    return a[..., 0] * 0.299 + a[..., 1] * 0.587 + a[..., 2] * 0.114


def main():
    bad = []
    sand = luma(rgb('terrain/sand.png'))
    dark = float((sand < np.median(sand) - 25).mean())
    if dark > 0.05:
        bad.append('sand: %.1f %% of pixels dark flecks, over 5 %%' % (dark * 100))
    frost = luma(rgb('terrain/frozen_soil.png'))
    far = float((np.abs(frost - np.median(frost)) > 60).mean())
    if far > 0.10:
        bad.append('frozen_soil: %.1f %% of pixels blotches far from its ground, over 10 %%' % (far * 100))
    moss = rgb('terrain/bone_moss.png')[..., :3].reshape(-1, 3).mean(0)
    scree = rgb('terrain/scree.png')[..., :3].reshape(-1, 3).mean(0)
    snow = rgb('terrain/snow.png')[..., :3].reshape(-1, 3).mean(0)
    if luma(moss) < luma(scree) + 50:
        bad.append('bone_moss: luma %.0f, not 50 over scree %.0f' % (luma(moss), luma(scree)))
    if moss[1] - moss[2] < 30 or np.linalg.norm(moss - snow) < 30:
        bad.append('bone_moss: mean %s reads as grey or snow (snow %s)' % (moss.round(), snow.round()))
    tree = rgb('plants/bone_tree.png')
    white = (luma(tree[..., :3]) > 215) & (tree[..., 3] > 0)
    for k in range(tree.shape[1] // 32):
        n = int(white[:, k * 32:(k + 1) * 32].sum())
        if n < 130:
            bad.append('bone_tree %d: %d white crown pixels, under 130' % (k, n))
    for b in bad:
        print('ground_check: ' + b, file=sys.stderr)
    if bad:
        sys.exit(1)
    print('ground_check: sand flecks %.1f %%, bone moss luma %.0f (scree %.0f), white crowns ok'
          % (dark * 100, luma(moss), luma(scree)))


if __name__ == '__main__':
    main()
