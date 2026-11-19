#!/usr/bin/env python3
"""Focused guard for the 414 terrain-edge flora and wildlife art slice.

This catches the easy-to-miss failures that a palette check cannot: a seasonal sheet missing
from a family, an animal sheet with the wrong animation grid, or an authored alien accent being
rendered as an empty/flat transparent image.
"""
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from animals import FRAMES  # noqa: E402
from pal import Palette, ROOT  # noqa: E402

PLANTS = os.path.join(ROOT, 'godot', 'assets', 'plants')
ANIMALS = os.path.join(ROOT, 'godot', 'assets', 'animals')
PALETTE = Palette.load()

PLANT_SHEETS = {
    'birch_2.png': (64, 64, 1),
    'shrub.png': (32, 32, 3),
    'shrub_autumn.png': (32, 32, 3),
    'shrub_winter.png': (32, 32, 3),
    'reeds.png': (32, 32, 3),
    'reeds_winter.png': (32, 32, 3),
    'fungus_tree.png': (32, 48, 3),
    'bone_tree.png': (32, 48, 3),
    'alien_bloom.png': (32, 32, 3),
    'alien_bloom_autumn.png': (32, 32, 3),
    'alien_bloom_winter.png': (32, 32, 3),
}

ANIMAL_CELLS = {
    'deer.png': (32, 48), 'boar.png': (32, 32), 'wolf.png': (32, 32),
    'hare.png': (32, 32), 'goat.png': (32, 32), 'chicken.png': (32, 32),
    'fox.png': (32, 32), 'crow.png': (32, 32), 'bear.png': (48, 48),
    'duck.png': (32, 32), 'bat.png': (32, 32), 'crawler.png': (32, 32),
    'burrower.png': (32, 32), 'hart.png': (32, 48),
    'slime.png': (32, 32), 'rats.png': (32, 32), 'centipede.png': (32, 32),
}

# Colour and broad placement checks, rather than exact pixels: these details may be retuned
# without changing the guard, but removing the authored feature fails it. Sleep has its own
# deliberately static pose, so coat checks cover the idle and walking frames only.
COAT_MARKINGS = {
    'deer.png': ('earth', 7, 10, (0, 1, 2, 3)),
    'fox.png': ('earth', 7, 4, (0, 1, 2, 3)),
    'wolf.png': ('stone', 6, 6, (0, 3)),
    'boar.png': ('stone', 2, 15, (1, 2)),
}


def rgba(name, folder):
    path = os.path.join(folder, name)
    if not os.path.exists(path):
        raise AssertionError('missing %s' % os.path.relpath(path, ROOT))
    return np.asarray(Image.open(path).convert('RGBA'))


def opaque_count(arr):
    return int((arr[:, :, 3] > 0).sum())


def colour_count(arr, ramp, step):
    colour = PALETTE.ramp(ramp, step)
    return int((np.all(arr[:, :, :3] == colour, axis=2) & (arr[:, :, 3] > 0)).sum())


def colours(arr):
    return {tuple(px[:3]) for row in arr for px in row if px[3]}


def main():
    plant_cells = {}
    for name, (cw, ch, variants) in PLANT_SHEETS.items():
        arr = rgba(name, PLANTS)
        assert (arr.shape[1], arr.shape[0]) == (cw * variants, ch), \
            '%s: expected %dx%d' % (name, cw * variants, ch)
        plant_cells[name] = [arr[:, v * cw:(v + 1) * cw] for v in range(variants)]
        for v, cell in enumerate(plant_cells[name]):
            assert opaque_count(cell) >= 20, '%s: variant %d is nearly empty' % (name, v)

    birch = plant_cells['birch_2.png'][0]
    assert colour_count(birch[:42, 48:], 'foliage', 7) >= 1, \
        'mature birch lost its sparse outer-canopy leaf catches'

    for v, (ramp, step) in enumerate((('warm', 4), ('warm', 6), ('cool', 5))):
        assert colour_count(plant_cells['shrub.png'][v], ramp, step) >= 3, \
            'shrub variant %d lost its berries, flowers, or cool leaf accents' % v
    for v, cell in enumerate(plant_cells['reeds.png']):
        assert colour_count(cell, 'earth', 5) >= 1, 'reeds variant %d lost its seed-head light' % v
        assert colour_count(cell, 'olive', 2) >= 3, 'reeds variant %d lost its waterline' % v
    assert colour_count(plant_cells['fungus_tree.png'][1][20:30], 'cool', 3) >= 5, \
        'amber fungus lost its hanging gill droplets'
    for v, cell in enumerate(plant_cells['bone_tree.png']):
        assert colour_count(cell, 'cool', 4) >= 3 and colour_count(cell, 'cool', 5) >= 3, \
            'bone tree variant %d lost its living-tissue seams' % v

    for name, ramp, step in (('alien_bloom.png', 'gem', 6),
                             ('alien_bloom_autumn.png', 'gem', 6),
                             ('alien_bloom_winter.png', 'cool', 6)):
        for v, cell in enumerate(plant_cells[name]):
            assert colour_count(cell, ramp, step) >= 1, \
                '%s: variant %d lost its seasonal glow' % (name, v)

    animal_cells = {}
    for name, (cw, ch) in ANIMAL_CELLS.items():
        arr = rgba(name, ANIMALS)
        assert (arr.shape[1], arr.shape[0]) == (cw * len(FRAMES), ch * 4), \
            '%s: expected %dx%d animation grid' % (name, cw * len(FRAMES), ch * 4)
        animal_cells[name] = [[arr[row * ch:(row + 1) * ch, col * cw:(col + 1) * cw]
                               for col in range(len(FRAMES))] for row in range(4)]
        for row in range(4):
            for col, frame in enumerate(FRAMES):
                assert opaque_count(animal_cells[name][row][col]) >= 8, \
                    '%s: %s cell in row %d is empty' % (name, frame, row)

    for name, (ramp, step, minimum, rows) in COAT_MARKINGS.items():
        for row in rows:
            for col, frame in enumerate(FRAMES[:-1]):
                assert colour_count(animal_cells[name][row][col], ramp, step) >= minimum, \
                    '%s: %s cell in row %d lost its coat marking' % (name, frame, row)

    crawler = colours(rgba('crawler.png', ANIMALS))
    assert PALETTE.ramp('cool', 7) in crawler, 'crawler lost its glow-pit highlight'
    plant_variants = sum(len(cells) for cells in plant_cells.values())
    animal_frames = sum(len(FRAMES) * 4 for _ in animal_cells)
    print('plants_animals_check: %d plant variants and %d animal frames passed' %
          (plant_variants, animal_frames))


if __name__ == '__main__':
    main()
