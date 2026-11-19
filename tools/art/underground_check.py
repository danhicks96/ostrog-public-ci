#!/usr/bin/env python3
"""Focused guard for the underground production art slice."""
import os
import sys

import numpy as np
from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
TERRAIN = os.path.join(ROOT, 'godot', 'assets', 'terrain')
OVERLAYS = os.path.join(ROOT, 'godot', 'assets', 'overlays')


def image(folder, name):
    return np.asarray(Image.open(os.path.join(folder, name)).convert('RGBA'))


def main():
    failures = []
    required = {
        'cave_floor.png': (256, 32),
        'dark_stone.png': (256, 32),
        'black_stone.png': (256, 32),
        'ore_iron.png': (256, 32),
        'ore_copper.png': (256, 32),
        'ore_coal.png': (256, 32),
        'ore_gem.png': (256, 32),
        'ore_tin.png': (256, 32),
        'ore_silver.png': (256, 32),
        'ore_gold.png': (256, 32),
    }
    for name, size in required.items():
        path = os.path.join(TERRAIN, name)
        if not os.path.exists(path):
            failures.append('%s is missing' % name)
            continue
        if Image.open(path).size != size:
            failures.append('%s is %s, expected %s' % (name, Image.open(path).size, size))

    seep = image(OVERLAYS, 'cave_seepage.png')
    strata = image(OVERLAYS, 'strata_face.png')
    if seep.shape[:2] != (32, 256):
        failures.append('cave_seepage.png is not 8 transparent 32px variants')
    if strata.shape[:2] != (16, 256):
        failures.append('strata_face.png is not 8 transparent 32x16 variants')
    if int((seep[:, :, 3] == 0).sum()) < seep.shape[0] * seep.shape[1] * 0.60:
        failures.append('cave seepage is too opaque for a fact-backed overlay')
    if int((strata[:, :, 3] == 0).sum()) < strata.shape[0] * strata.shape[1] * 0.55:
        failures.append('strata face is too opaque for a texture overlay')

    cave = image(TERRAIN, 'cave_floor.png')[:, :, :3].astype(float)
    black = image(TERRAIN, 'black_stone.png')[:, :, :3].astype(float)
    # The cave floor must remain earth-ramp material rather than collapsing into deep rock.
    cave_mean = cave.mean(axis=(0, 1))
    black_mean = black.mean(axis=(0, 1))
    if float(np.linalg.norm(cave_mean - black_mean)) < 8.0:
        failures.append('cave_floor does not separate from black_stone')
    # Gem rock must contain high-value gem/cool facets beyond the black host.
    gem = image(TERRAIN, 'ore_gem.png')[:, :, :3].astype(float)
    if int(((gem[:, :, 2] > gem[:, :, 0] * 1.05) & (gem[:, :, 2] > 70)).sum()) < 8:
        failures.append('ore_gem has no readable cool/gem facets')
    # Strata and seepage must be visibly populated, while remaining overlays rather than opaque
    # replacement tiles.
    if int((seep[:, :, 3] > 0).sum()) < 20:
        failures.append('cave seepage has no visible channels')
    if int((strata[:, :, 3] > 0).sum()) < 20:
        failures.append('strata face has no visible bands')
    if failures:
        for failure in failures:
            print('underground_check: ' + failure, file=sys.stderr)
        return 1
    print('underground_check: terrain families, gem facets, seepage and strata overlays pass')
    return 0


if __name__ == '__main__':
    sys.exit(main())
