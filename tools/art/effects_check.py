#!/usr/bin/env python3
"""Guard the production effects slice: expected sheets, dimensions and palette."""
import os
import sys

from PIL import Image

from pal import Palette, ROOT

ASSETS = os.path.join(ROOT, 'godot', 'assets', 'effects')
EXPECTED = {
    'weapon_impact.png': (128, 32),
    'environmental_hit.png': (128, 32),
    'resonance.png': (288, 48),
    'fire_flare.png': (32, 32),
    'ember.png': (6, 6),
    'smoke.png': (12, 12),
}


def main():
    pal = Palette.load()
    failures = 0
    for name, size in EXPECTED.items():
        path = os.path.join(ASSETS, name)
        if not os.path.isfile(path):
            print('MISSING %s' % name)
            failures += 1
            continue
        im = Image.open(path).convert('RGBA')
        if im.size != size:
            print('SIZE %s: expected %s, got %s' % (name, size, im.size))
            failures += 1
        bad = set()
        for r, g, b, a in im.getdata():
            if a and not pal.has((r, g, b)):
                bad.add('#%02x%02x%02x' % (r, g, b))
        if bad:
            print('OFF-PALETTE %s: %s' % (name, ' '.join(sorted(bad))))
            failures += 1
    print('effects_check: %d assets, %d failures' % (len(EXPECTED), failures))
    sys.exit(1 if failures else 0)


if __name__ == '__main__':
    main()
