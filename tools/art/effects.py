#!/usr/bin/env python3
"""Small, palette-checked production effects used by the Godot client.

These are deliberately restrained: the simulation supplies the event or pose, while this
generator supplies a readable visual accent.  Nothing here creates a gameplay fact.
"""
import os

from draw import Canvas, sheet
from pal import Palette, ROOT

OUT = os.path.join(ROOT, 'godot', 'assets', 'effects')
T = 32
pal = Palette.load()
P = pal.ramp
N = pal.neutral


def _burst(phase, family='warm'):
    """A four-frame, 32 px weapon/environment accent."""
    c = Canvas(T, T)
    cx, cy = 16, 16
    hot = P('warm', 7)
    mid = P('warm', 5)
    cool = P('water', 5) if family == 'water' else P('stone', 5) if family == 'stone' else P('earth', 5)
    dark = P('stone', 1) if family == 'stone' else P('earth', 1)
    if phase == 0:
        c.px(cx, cy, hot)
        c.px(cx - 1, cy, mid)
        c.px(cx + 1, cy, cool)
    elif phase == 1:
        for x, y in ((cx, cy - 5), (cx + 5, cy), (cx, cy + 5), (cx - 5, cy)):
            c.line(cx, cy, x, y, mid)
        c.disc(cx, cy, 2.5, 2.5, hot)
    elif phase == 2:
        for x, y in ((cx - 8, cy - 2), (cx + 8, cy + 1), (cx - 2, cy + 8), (cx + 2, cy - 8)):
            c.line(cx, cy, x, y, cool)
        c.disc(cx, cy, 3.5, 2.5, mid)
        c.px(cx, cy, hot)
    else:
        for x, y in ((cx - 9, cy - 5), (cx + 8, cy - 1), (cx - 3, cy + 8), (cx + 4, cy - 8)):
            c.px(x, y, dark)
            c.px(x + (1 if x < cx else -1), y, cool)
    return c


def weapon_impact(phase):
    """Readable weapon contact: warm core, cool edge, no screen-filling bloom."""
    c = _burst(phase, 'warm')
    if phase == 1:
        c.line(8, 23, 23, 8, P('gold', 4))
        c.line(10, 24, 24, 10, P('warm', 6))
    return c


def environmental_hit(phase):
    """Chips and dust for actual mining, chopping, building and dousing work."""
    c = _burst(phase, 'stone')
    if phase in (1, 2):
        c.px(7 + phase, 11, P('earth', 4))
        c.px(24 - phase, 21, P('earth', 2))
    return c


def resonance(phase):
    """A cool, ancient echo around the already-modelled thin place/portal."""
    c = Canvas(48, 48)
    cx, cy = 24, 24
    r = 5 + phase * 3
    for x in range(48):
        for y in range(48):
            d = abs(((x - cx) ** 2 + (y - cy) ** 2) ** 0.5 - r)
            if d < 1.5 and ((x + y + phase) % 3 != 0):
                c.px(x, y, P('cool', 4 if phase < 3 else 3), 210 if phase < 4 else 150)
    for x, y in ((cx, cy - r), (cx + r, cy), (cx, cy + r), (cx - r, cy)):
        c.px(int(x), int(y), N('white'), 220 if phase < 4 else 150)
    return c


def fire_flare():
    """A compact warm ignition accent for a real FireStarted event."""
    c = Canvas(32, 32)
    c.disc(16, 18, 4, 4, P('warm', 5))
    c.line(16, 13, 16, 5, P('gold', 6))
    c.line(12, 16, 7, 11, P('warm', 7))
    c.line(20, 16, 25, 11, P('warm', 6))
    c.px(16, 18, N('white'))
    return c


def ember():
    c = Canvas(6, 6)
    c.px(2, 4, P('warm', 3))
    c.px(3, 3, P('warm', 6))
    c.px(3, 2, P('gold', 7))
    c.px(4, 1, N('white'))
    return c


def smoke():
    c = Canvas(12, 12)
    for x, y, r in ((4, 8, 3), (7, 5, 3), (6, 2, 2)):
        c.disc(x, y, r, r, P('stone', 3), 210)
    return c


def main():
    os.makedirs(OUT, exist_ok=True)
    sheet([weapon_impact(k) for k in range(4)], T, T).save(os.path.join(OUT, 'weapon_impact.png'))
    sheet([environmental_hit(k) for k in range(4)], T, T).save(os.path.join(OUT, 'environmental_hit.png'))
    sheet([resonance(k) for k in range(6)], 48, 48).save(os.path.join(OUT, 'resonance.png'))
    fire_flare().save(os.path.join(OUT, 'fire_flare.png'))
    ember().save(os.path.join(OUT, 'ember.png'))
    smoke().save(os.path.join(OUT, 'smoke.png'))
    print('effects -> %s' % OUT)


if __name__ == '__main__':
    main()
