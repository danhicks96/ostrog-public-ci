#!/usr/bin/env python3
"""Client-only underground presentation assets.

These overlays are deliberately driven by facts the client already receives:

* cave_seepage is drawn only when a real water tile neighbours an underground floor;
* strata_face is a texture detail for a visible natural rock face, not a new material.

They are transparent, palette-quantised sheets so the normal renderer can keep its existing
black-base and material-backed rules.
"""
import os

from draw import Canvas, rng, sheet
from pal import Palette, ROOT

OUT = os.path.join(ROOT, 'godot', 'assets', 'overlays')
T = 32
pal = Palette.load()
P = pal.ramp
N = pal.neutral


def _seepage(v):
    r = rng('underground-seepage', v)
    c = Canvas(T, T)
    # The channels vary in origin and route, but stay sparse enough that the overlay reads as
    # dampness rather than a second water tile. The water ramp is the only colour family used.
    count = 2 + v % 2
    for _ in range(count):
        x = r.randint(3, T - 4)
        y = r.randint(0, 4)
        length = r.randint(8, 18)
        for step in range(length):
            if step % 3 == 0:
                x += r.choice((-1, 0, 0, 1))
            y += 1
            c.px(x, y, P('water', 2))
            if step % 2 == 0:
                c.px(x + 1, y, P('water', 3))
            if step % 5 == 0:
                c.px(x, y + 1, P('water', 4))
        # A small pooled glint at the end gives the eye a terminus without implying depth or
        # quality that the simulation does not model.
        c.px(x, min(T - 1, y + 1), P('water', 5))
        c.px(x + 1, min(T - 1, y + 1), P('water', 4))
    return c


def _strata_face(v):
    r = rng('underground-strata-face', v)
    c = Canvas(T, 16)
    # Transparent by default: the existing cliff face and its material tint remain authoritative.
    # These broken horizontal bands provide scale and depth when the face is an underground rock
    # exposure. They are intentionally neutral stone/cool colours, not a new geology label.
    y = 1 + v % 2
    band = 0
    while y < 14:
        h = r.randint(2, 4)
        dark = P('stone', max(0, 1 + (band + v) % 3))
        light = P('stone', min(5, 3 + (band + v) % 3))
        for x in range(T):
            if x % 5 == 0:
                drift = r.choice((-1, 0, 0, 1))
            else:
                drift = 0
            yy = max(1, min(14, y + drift))
            c.px(x, yy, light)
            if x % 3 != 0:
                c.px(x, min(15, yy + h - 1), dark)
        if band % 2 == 0:
            x = r.randint(3, 23)
            c.line(x, y + 1, x + r.randint(3, 7), min(13, y + h + 2), P('cool', 2))
        y += h + 1
        band += 1
    return c


def main():
    os.makedirs(OUT, exist_ok=True)
    sheet([_seepage(v) for v in range(8)], T, T).save(os.path.join(OUT, 'cave_seepage.png'))
    sheet([_strata_face(v) for v in range(8)], T, 16).save(os.path.join(OUT, 'strata_face.png'))
    print('underground overlays: cave_seepage.png, strata_face.png -> %s' % OUT)


if __name__ == '__main__':
    main()
