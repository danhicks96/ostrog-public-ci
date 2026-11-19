#!/usr/bin/env python3
"""Focused guard for the production shoreline's high-impact structure props.

This is intentionally a semantic smoke test rather than a pixel golden.  It catches an
accidental empty/fallback sprite, a wrong sheet size, or a lamp sheet whose two states have
collapsed to one frame while allowing the generator to evolve its pixel-level treatment.
"""
import os
import sys
import warnings

from PIL import Image

from pal import ROOT


warnings.filterwarnings('ignore', category=DeprecationWarning)


FOLDER = os.path.join(ROOT, 'godot', 'assets', 'structures')
SPECS = {
    'well.png': ((32, 32), 180, 12),
    'workbench.png': ((32, 32), 170, 12),
    'carpenter_bench.png': ((32, 32), 170, 12),
    'barrel.png': ((32, 32), 150, 10),
    'crate.png': ((32, 32), 170, 10),
    'chest.png': ((32, 32), 170, 10),
    'stockpile.png': ((32, 32), 55, 8),
    'lamp.png': ((64, 32), 220, 10),
}


def opaque_pixels(image):
    return sum(1 for px in image.getdata() if px[3] > 0)


def colour_count(image):
    return len({px[:3] for px in image.getdata() if px[3] > 0})


def main():
    failures = []
    loaded = {}
    for name, (size, min_opaque, min_colours) in SPECS.items():
        path = os.path.join(FOLDER, name)
        if not os.path.isfile(path):
            failures.append('%s: missing' % name)
            continue
        image = Image.open(path).convert('RGBA')
        loaded[name] = image
        if image.size != size:
            failures.append('%s: expected %sx%s, got %sx%s' % (name, size[0], size[1], image.width, image.height))
        if opaque_pixels(image) < min_opaque:
            failures.append('%s: only %d opaque pixels' % (name, opaque_pixels(image)))
        if colour_count(image) < min_colours:
            failures.append('%s: only %d opaque palette colours' % (name, colour_count(image)))

    lamp = loaded.get('lamp.png')
    if lamp is not None:
        cold = lamp.crop((0, 0, 32, 32))
        lit = lamp.crop((32, 0, 64, 32))
        if list(cold.getdata()) == list(lit.getdata()):
            failures.append('lamp.png: cold and lit frames are identical')

    if failures:
        for failure in failures:
            print('structures: ' + failure, file=sys.stderr)
        return 1
    print('structures: %d high-impact props passed size/detail/state guard' % len(SPECS))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
