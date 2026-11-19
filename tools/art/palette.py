#!/usr/bin/env python3
"""Generate the Ostrog master palette: 7 hue ramps x 8 steps + 4 neutrals = 60 colours, and
since style bible v2 (M2) four more ramps after them: gold, copper, olive and gems = 92.

Each ramp is built from three hand-picked anchors (shadow, mid, highlight) and interpolated
in OKLab, so lightness is even. The anchors are deliberately hue-shifted: shadows lean
cool (blue/purple), highlights lean warm (yellow/orange), which is what gives painted
pixel art its glow instead of a flat lightness slider.

Writes godot/assets/palette/ostrog.json and ostrog.png (swatch).
"""
import json
import os

import numpy as np
from PIL import Image, ImageDraw

from pal import PALETTE_JSON, hex_to_rgb, rgb_to_hex

# ramp -> (shadow, mid, highlight)  step 0 = darkest, step 7 = lightest
ANCHORS = {
    'earth':   ('#2a1b2b', '#7b4a2c', '#f0cf92'),
    'stone':   ('#1f2232', '#6a7284', '#e9e3d6'),
    'foliage': ('#15282f', '#3c7a30', '#d6ea7c'),
    'water':   ('#141f44', '#2b6f93', '#c0eee6'),
    'warm':    ('#3f1230', '#d04a28', '#ffe79a'),
    'cool':    ('#170f30', '#6c4fae', '#cfd7ff'),
}
# Palette v2 (style bible v2, M2): appended after the first 60 so every existing sheet stays
# valid. Metals and an olive dye were missing (gold snapped to pale orange on the first derived
# item sheet); gems are four hues, dark and light each, then two spare accents.
ANCHORS_V2 = {
    'gold':    ('#2e1c12', '#c8922a', '#fff4b8'),
    'copper':  ('#26121a', '#b25c34', '#ffd6ac'),
    'olive':   ('#1a2016', '#6e7836', '#e4e8aa'),
}
GEMS = ['#5a0f24', '#e0365a', '#122a6e', '#4f8ef0', '#0d4a32', '#3fcf86', '#3b1a5c', '#b878f0']
# skin: 6 skin tones (0 deepest .. 5 palest) + 2 hair colours (6 dark, 7 light)
SKIN = ['#4a2a1e', '#73452c', '#a2694a', '#c68d66', '#e2b48c', '#f3d6b4', '#1f1518', '#c99a52']
NEUTRALS = {'black': '#0d0b11', 'shadow': '#2b2634', 'light': '#e6ded0', 'white': '#fbf7ef'}


def srgb_to_lin(c):
    c = np.asarray(c, dtype=float) / 255.0
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def lin_to_srgb(c):
    c = np.clip(c, 0, 1)
    s = np.where(c <= 0.0031308, c * 12.92, 1.055 * c ** (1 / 2.4) - 0.055)
    return tuple(int(round(v * 255)) for v in s)


def to_oklab(rgb):
    r, g, b = srgb_to_lin(rgb)
    l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b
    m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b
    s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b
    l, m, s = np.cbrt(l), np.cbrt(m), np.cbrt(s)
    return np.array([0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
                     1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
                     0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s])


def from_oklab(lab):
    L, a, b = lab
    l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3
    m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3
    s = (L - 0.0894841775 * a - 1.2914855480 * b) ** 3
    r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s
    g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s
    bb = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s
    return lin_to_srgb((r, g, bb))


def build_ramp(shadow, mid, high):
    a, b, c = (to_oklab(hex_to_rgb(h)) for h in (shadow, mid, high))
    out = []
    for i in range(8):
        t = i / 7.0
        if t <= 0.5:
            u = t / 0.5
            lab = a * (1 - u) + b * u
        else:
            u = (t - 0.5) / 0.5
            lab = b * (1 - u) + c * u
        out.append(from_oklab(lab))
    return out


def build():
    entries = []
    for ramp, anchors in ANCHORS.items():
        for step, rgb in enumerate(build_ramp(*anchors)):
            entries.append({'name': '%s_%d' % (ramp, step), 'hex': rgb_to_hex(rgb), 'ramp': ramp, 'step': step})
    for step, h in enumerate(SKIN):
        entries.append({'name': 'skin_%d' % step, 'hex': h, 'ramp': 'skin', 'step': step})
    for name, h in NEUTRALS.items():
        entries.append({'name': name, 'hex': h, 'ramp': 'neutral', 'step': list(NEUTRALS).index(name)})
    for ramp, anchors in ANCHORS_V2.items():
        for step, rgb in enumerate(build_ramp(*anchors)):
            entries.append({'name': '%s_%d' % (ramp, step), 'hex': rgb_to_hex(rgb), 'ramp': ramp, 'step': step})
    for step, h in enumerate(GEMS):
        entries.append({'name': 'gem_%d' % step, 'hex': h, 'ramp': 'gem', 'step': step})
    # de-duplicate (a collision would silently merge two steps)
    seen = {}
    for e in entries:
        assert e['hex'] not in seen, 'duplicate colour %s for %s and %s' % (e['hex'], e['name'], seen[e['hex']])
        seen[e['hex']] = e['name']
    return entries


def swatch(entries, path, cell=24):
    rows = list(ANCHORS) + ['skin', 'neutral'] + list(ANCHORS_V2) + ['gem']
    img = Image.new('RGB', (cell * 8 + 90, cell * len(rows)), (20, 20, 20))
    d = ImageDraw.Draw(img)
    for r, ramp in enumerate(rows):
        d.text((4, r * cell + 7), ramp, fill=(220, 220, 220))
        for e in entries:
            if e['ramp'] == ramp:
                x = 90 + e['step'] * cell
                d.rectangle([x, r * cell, x + cell - 1, r * cell + cell - 1], fill=hex_to_rgb(e['hex']))
    img.save(path)


def main():
    entries = build()
    os.makedirs(os.path.dirname(PALETTE_JSON), exist_ok=True)
    with open(PALETTE_JSON, 'w') as f:
        json.dump(entries, f, indent=1)
    swatch(entries, PALETTE_JSON.replace('.json', '.png'))
    print('palette: %d colours -> %s' % (len(entries), PALETTE_JSON))


if __name__ == '__main__':
    main()
