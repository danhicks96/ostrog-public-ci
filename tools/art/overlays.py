#!/usr/bin/env python3
"""Overlays: godot/assets/overlays/
  cliff_face.png        3 variants in a row, 32x16 each: earthen/rocky elevation face, dark lower edge
  cliff_shadow.png      8 pieces in a row (N,E,S,W edges then NW,NE,SE,SW corners), 32x32, black alpha
  wall_face_wood.png / wall_face_stone_block.png / wall_face_thatch.png   32x8 south faces
  ruin.png              4 variants in a row, 32x32: moss, chips and cracks over an old wall's top (#202)
  rubble.png            4 variants in a row, 32x32: tumbled blocks where an old wall fell (#202)
  edge_blend.png        8 alpha masks in a row (N,E,S,W edges then NW,NE,SE,SW corners), 32x32, white
cliff_shadow and edge_blend are alpha gradients and are exempt from palette_check.
"""
import os

from draw import Canvas, rng, sheet
from pal import Palette, ROOT

OUT = os.path.join(ROOT, 'godot', 'assets', 'overlays')
T = 32
pal = Palette.load()
P = pal.ramp
N = pal.neutral


def cliff_face(v):
    """32x16: a lip of turf-soil at the top, irregular rock strata, a few protruding boulders,
    and a near-black bottom edge so the drop reads as a drop. Variants differ in strata
    thickness and boulder placement."""
    r = rng('cliff', v)
    c = Canvas(32, 16)
    c.rect(0, 0, 32, 16, P('stone', 3))
    # soil lip 3 px: dark crumb line, lit soil, dangling roots
    c.hline(0, 31, 0, P('earth', 2))
    c.hline(0, 31, 1, P('earth', 4))
    c.hline(0, 31, 2, P('earth', 3))
    for x in range(0, 32, 3):
        if r.random() < 0.5:
            c.px(x + r.randint(0, 1), 3, P('earth', 2))
    # strata: wavy seams of darker rock; each band a slightly different step
    y = 3
    band = 0
    while y < 14:
        h = r.randint(3, 5)
        step = (3, 4, 3, 2)[band % 4]
        c.rect(0, y, 32, h, P('stone', step))
        c.hline(0, 31, y, P('stone', step + 1))            # lit top of the band
        # wavy seam below the band (1 px, wandering by +-1)
        sy = y + h - 1
        dy = 0
        for x in range(32):
            if x % 5 == 0:
                dy = r.choice((-1, 0, 0, 1))
            c.px(x, min(13, max(3, sy + dy)), P('stone', max(0, step - 2)))
        # vertical fractures, sparse and irregular
        for x in r.sample(range(2, 30), 2):
            c.vline(x, y + 1, sy - 1, P('stone', max(0, step - 2)))
        y += h
        band += 1
    # protruding boulders with a lit top-left and a dark underside
    for _ in range(2 + v % 2):
        x, y = r.randint(1, 26), r.randint(4, 9)
        w, h = r.randint(4, 6), r.randint(3, 4)
        c.disc(x + w / 2.0, y + h / 2.0, w / 2.0, h / 2.0, P('stone', 4))
        c.hline(x + 1, x + w - 2, y, P('stone', 6))
        c.px(x, y + 1, P('stone', 5))
        c.hline(x + 1, x + w - 1, y + h, P('stone', 1))
        c.px(x + w, y + h - 1, P('stone', 1))
    # dark lower edge (the ground below is in this face's shadow)
    c.hline(0, 31, 14, P('stone', 0))
    c.hline(0, 31, 15, N('black'))
    return c


def gradient_piece(kind, depth, maxa, colour):
    """32x32 piece whose alpha falls off from the named edge(s). kind in N,E,S,W,NW,NE,SE,SW.
    Edges fade inward over `depth` px; corners are the max of the two edge falloffs."""
    c = Canvas(T, T)
    for y in range(T):
        for x in range(T):
            a = 0.0
            if 'N' in kind:
                a = max(a, 1.0 - y / depth)
            if 'S' in kind:
                a = max(a, 1.0 - (T - 1 - y) / depth)
            if 'W' in kind:
                a = max(a, 1.0 - x / depth)
            if 'E' in kind:
                a = max(a, 1.0 - (T - 1 - x) / depth)
            if a > 0:
                c.px(x, y, colour, int(round(maxa * a)))
    return c


KINDS = ['N', 'E', 'S', 'W', 'NW', 'NE', 'SE', 'SW']


def cliff_shadow():
    return sheet([gradient_piece(k, 10, 150, N('black')) for k in KINDS], T, T)


def edge_blend():
    return sheet([gradient_piece(k, 12, 255, N('white')) for k in KINDS], T, T)


def wall_face(material):
    """32x8 south face of a wall: lit at the top row, a dark ground-contact row at the bottom."""
    c = Canvas(32, 8)
    if material == 'wood':
        c.rect(0, 0, 32, 8, P('earth', 3))
        for x in range(0, 32, 6):                          # upright planks
            c.vline(x, 0, 7, P('earth', 1))
            c.vline(x + 1, 0, 7, P('earth', 4))
        c.hline(0, 31, 0, P('earth', 5))
        c.hline(0, 31, 7, P('earth', 0))
    elif material == 'stone_block':
        c.rect(0, 0, 32, 8, P('stone', 3))
        c.hline(0, 31, 0, P('stone', 5))
        c.hline(0, 31, 3, P('stone', 1))                   # course seam
        c.hline(0, 31, 4, P('stone', 4))
        for x in range(2, 32, 10):                         # joints, offset per course
            c.vline(x, 0, 2, P('stone', 1))
            c.vline(x + 5, 4, 6, P('stone', 1))
        c.hline(0, 31, 7, P('stone', 0))
    else:  # thatch (wattle face below a thatched top)
        c.rect(0, 0, 32, 8, P('earth', 4))
        c.hline(0, 31, 0, P('earth', 6))
        for y in (2, 5):                                   # horizontal withies
            c.hline(0, 31, y, P('earth', 2))
        for x in range(1, 32, 4):
            c.vline(x, 1, 6, P('earth', 3))
        c.hline(0, 31, 7, P('earth', 0))
    return c


def look_face(name):
    """The south face of a wall of a particular wood or stone (C56), from structures.WALL_LOOKS."""
    from structures import WALL_LOOKS
    style, ramp, off = WALL_LOOKS[name]
    q = lambda k: P(ramp, k + off)
    c = Canvas(32, 8)
    if style == 'wood':
        c.rect(0, 0, 32, 8, q(3))
        for x in range(0, 32, 6):
            c.vline(x, 0, 7, q(1))
            c.vline(x + 1, 0, 7, q(4))
        c.hline(0, 31, 0, q(5))
        c.hline(0, 31, 7, q(0))
    else:
        c.rect(0, 0, 32, 8, q(3))
        c.hline(0, 31, 0, q(5))
        c.hline(0, 31, 3, q(1))
        c.hline(0, 31, 4, q(4))
        for x in range(2, 32, 10):
            c.vline(x, 0, 2, q(1))
            c.vline(x + 5, 4, 6, q(1))
        c.hline(0, 31, 7, q(0))
    return c


def ruin(v):
    """32x32, drawn over the top of a wall from the valley's past (deep::ruins): moss creeping
    over its north half, bites knocked out of its edges (a dark chip with a lit lower lip), a
    crack or two, and a weed in a joint. #202 (QA 2026-10-20): ruins used a new wall's blocks."""
    r = rng('ruin', v)
    c = Canvas(T, T)
    for _ in range(4 + v % 2):                           # moss cushions, thickest to the north
        x, y = r.randint(2, 29), r.randint(1, 14)
        c.disc(x, y, r.randint(2, 4), r.randint(1, 2), P('olive', 3))
        c.disc(x - 1, y - 1, 1.5, 1, P('olive', 4))
    for _ in range(3):                                   # chips out of the edges
        side = r.randint(0, 3)
        k = r.randint(4, 26)
        x, y = [(k, 0), (31, k), (k, 31), (0, k)][side]
        dx, dy = [(0, 1), (-1, 0), (0, -1), (1, 0)][side]
        for d in range(r.randint(2, 4)):
            c.px(x + dx * d, y + dy * d, P('stone', 1))
            c.px(x + dx * d + dy, y + dy * d + dx, P('stone', 2))
        c.px(x + dx * 4 + dy, y + dy * 4 + dx, P('stone', 5))
    for _ in range(1 + v % 2):                           # cracks
        x, y = r.randint(6, 26), r.randint(10, 24)
        for _ in range(r.randint(4, 7)):
            c.px(x, y, P('stone', 1))
            x += r.choice((-1, 0, 1))
            y += 1
    if v % 2 == 0:                                       # a weed in a joint
        x, y = r.randint(8, 24), r.randint(16, 26)
        c.stamp(['h.h', '.d.'], x, y, {'h': P('foliage', 5), 'd': P('foliage', 3)})
    return c


def rubble(v):
    """32x32: blocks from an old wall where it fell, tumbled and half sunk, lit from the upper
    left, with moss on them and a weed between (#202)."""
    r = rng('rubble', v)
    c = Canvas(T, T)
    for k in range(3 + v % 3):
        x, y = r.randint(3, 22), r.randint(6, 24)
        w, h = r.randint(5, 8), r.randint(3, 5)
        c.rect(x + 1, y + h, w, 1, P('stone', 1), 160)   # contact shadow
        c.rect(x, y, w, h, P('stone', 4))
        c.hline(x, x + w - 1, y, P('stone', 6))
        c.vline(x, y, y + h - 1, P('stone', 5))
        c.hline(x, x + w - 1, y + h - 1, P('stone', 2))
        if k % 2 == 0:
            c.disc(x + w // 2, y, 1.5, 1, P('olive', 4))
    for _ in range(4):                                   # chips and grit
        c.px(r.randint(2, 29), r.randint(4, 29), P('stone', 3))
    c.stamp(['h.h', '.d.'], r.randint(4, 24), r.randint(20, 27), {'h': P('foliage', 5), 'd': P('foliage', 3)})
    return c


def main():
    os.makedirs(OUT, exist_ok=True)
    sheet([cliff_face(v) for v in range(3)], 32, 16).save(os.path.join(OUT, 'cliff_face.png'))
    cliff_shadow().save(os.path.join(OUT, 'cliff_shadow.png'))
    edge_blend().save(os.path.join(OUT, 'edge_blend.png'))
    sheet([ruin(v) for v in range(4)], T, T).save(os.path.join(OUT, 'ruin.png'))
    sheet([rubble(v) for v in range(4)], T, T).save(os.path.join(OUT, 'rubble.png'))
    for m in ('wood', 'stone_block', 'thatch'):
        wall_face(m).save(os.path.join(OUT, 'wall_face_%s.png' % m))
    from structures import WALL_LOOKS
    for name in WALL_LOOKS:
        look_face(name).save(os.path.join(OUT, 'wall_face_%s.png' % name))
    print('overlays -> %s' % OUT)


if __name__ == '__main__':
    main()
