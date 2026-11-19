#!/usr/bin/env python3
"""Terrain tiles: godot/assets/terrain/<family>.png, usually 8 variants in a row (256x32).
Grass and autumn grass use 24 variants to reduce visible repetition over large areas.
water: 4 frames x 2 variants (256x64, frames left-to-right, variants top-to-bottom).
deep_water: 4 frames x 2 variants likewise.

Every tile is drawn on a wrapping Tile so motifs that cross an edge continue on the
opposite edge: a tile is seamless with itself and the low-contrast base mottle makes
mixed arrangements seamless too. Motifs are explicit pixel templates, scattered with a
deterministic blue-noise placement; no per-pixel noise.
"""
import os

import numpy as np
from PIL import Image

from draw import Tile, Canvas, rng, scatter, sheet
from pal import Palette, ROOT

OUT = os.path.join(ROOT, 'godot', 'assets', 'terrain')
T = 32
pal = Palette.load()
P = pal.ramp
N = pal.neutral

FAMILIES = ['grass', 'soil', 'clay', 'sand', 'gravel', 'pale_stone', 'warm_stone', 'grey_stone',
            'dark_stone', 'black_stone', 'ore_iron', 'ore_copper', 'ore_coal', 'water', 'deep_water',
            'wood_floor', 'stone_floor', 'thatch', 'peat', 'snow', 'mud', 'cave_floor',
            'wildflower', 'mossy', 'ash', 'scree', 'packed_earth', 'frozen_soil', 'wet_soil',
            'grass_autumn', 'ore_gem', 'marble', 'ore_tin', 'ore_silver', 'ore_gold',
            'milkstone', 'bone_moss']

# ---------------------------------------------------------------- motif templates
TUFT = ['.h.h', 'h.h.', '.d..']          # grass tuft: 2 highlight tips over a dark root
TUFT2 = ['h..', '.hd', '.d.']
TUFT_SMALL = ['h.', '.d']
CLOD = ['.hh.', 'hmmd', '.dd.']          # soil clod lit from upper-left
CLOD_SMALL = ['hm', '.d']
PEBBLE = ['hm', 'md']                     # 2x2 pebble
PEBBLE_BIG = ['.hh.', 'hmmd', '.dd.']
FLOWER = ['.p.', 'pcp', '.s.']            # petal ring, centre, stem below
FLOWER_SMALL = ['p', 's']
MUSH_FLECK = ['h']
RIPPLE = ['hhh.', '...h']
MOSS = ['mm', 'm.']

LEAF = ['.d.', '.l.']
FLOWER2 = ['.p.', 'p']
ASHPATT = ['.d', '.s.']
STONE2 = ['h', 'd']
POOL = ['www']


def mottle(t, r, colours, count, w=(1, 3), h=(1, 2)):
    """Low-contrast blobs of neighbouring ramp steps over the base fill: rounded patches
    (wrapping), so the ground reads as soft variation rather than specks."""
    for c in colours:
        for _ in range(count):
            x, y = r.randint(0, T - 1), r.randint(0, T - 1)
            bw, bh = r.randint(*w), r.randint(*h)
            t.disc(x, y, bw / 2.0, bh / 2.0, c)


def patches(t, r, colours, count=3):
    """Large soft patches: the main tonal variation of a tile. Low contrast and few, so the
    deliberate motifs (tufts, clods, cracks) stay the thing the eye reads. Edges are
    ragged by knocking 1-px bites out of the ellipse so no patch reads as a stamped blob.
    The first colour is treated as the dark side (bigger, count patches); later colours
    are lighter and get count-1 smaller patches."""
    for ci, c in enumerate(colours):
        n = count if ci == 0 else max(1, count - 1)
        for _ in range(n):
            x, y = r.randint(0, T - 1), r.randint(0, T - 1)
            rx = r.randint(4, 7) if ci == 0 else r.randint(3, 5)
            ry = r.randint(2, 4) if ci == 0 else r.randint(2, 3)
            t.disc(x, y, rx, ry, c)
            # extend one lobe so the shape is not an ellipse
            t.disc(x + r.randint(-rx, rx), y + r.randint(-ry, ry), r.randint(2, 3), r.randint(1, 2), c)
            base = tuple(int(v) for v in t.a[y % T, x % T, :3])
    # ragged edges: a pixel on a patch boundary is bitten back 1 in 3 times
    a = t.a
    rgb = a[:, :, :3].copy()
    for j in range(T):
        for i in range(T):
            here = tuple(rgb[j, i])
            left = tuple(rgb[j, (i - 1) % T])
            up = tuple(rgb[(j - 1) % T, i])
            if here != left and r.random() < 0.35:
                a[j, i, :3] = left
            elif here != up and r.random() < 0.35:
                a[j, i, :3] = up


def crack(t, r, x, y, length, dark, light, bias=(1, 0)):
    """1-px dark crack with a 1-px light edge below-right (light from upper-left)."""
    pts = []
    for _ in range(length):
        pts.append((x, y))
        roll = r.random()
        if roll < 0.45:
            x += bias[0]
            y += bias[1]
        elif roll < 0.75:
            x += 1
            y += r.choice((-1, 1))
        else:
            x += r.choice((0, 1))
            y += 1
    for (px, py) in pts:
        t.px(px, py, dark)
    for (px, py) in pts:
        if (px + 1, py + 1) not in pts and (px + 1, py) not in pts:
            t.px(px + 1, py + 1, light)


def vein(t, r, x, y, length, body, glint, dark, rows=2):
    """Ore seam, `rows` px thick, with a dark edge and glint pixels.

    `rows` lets coal read as a thick band and metals as thin veins: seam thickness is part of
    how a mineral is recognised at a glance.
    """
    pts = []
    dy = 0
    for i in range(length):
        pts.append((x, y))
        x += 1
        if i % 3 == 2:
            dy = r.choice((-1, 0, 1))
        y += dy
    for (px, py) in pts:
        t.px(px, py + rows, dark)
    for i, (px, py) in enumerate(pts):
        for k in range(rows):
            t.px(px, py + k, body)
        if i % 4 == 1:
            t.px(px, py - 1, body)
            t.px(px + 1, py - 1, dark)
    for (px, py) in pts[1::3]:
        t.px(px, py + r.choice((0, 1)), glint)


def stamp_scatter(t, r, tpl, m, n, min_dist=5, margin=0):
    for (x, y) in scatter(r, n, T, T, margin=margin, min_dist=min_dist):
        t.stamp(tpl, x, y, m, wrap=(T, T))


# ---------------------------------------------------------------- new families (surface detail)
def wildflower(t, r, v):
    patches(t, r, [P('foliage', 3), P('foliage', 5)], 2)
    stamp_scatter(t, r, TUFT, {'h': P('foliage', 6), 'd': P('foliage', 2)}, 3, min_dist=9)
    stamp_scatter(t, r, TUFT2, {'h': P('foliage', 5), 'd': P('foliage', 2)}, 4, min_dist=6)
    stamp_scatter(t, r, TUFT_SMALL, {'h': P('foliage', 5), 'd': P('foliage', 2)}, 5, min_dist=4)
    stamp_scatter(t, r, FLOWER, {'p': P('warm', 6), 'c': P('warm', 7), 's': P('foliage', 2)}, 3, min_dist=12)
    stamp_scatter(t, r, FLOWER2, {'p': N('white'), 's': P('foliage', 3)}, 4, min_dist=8)


def mossy(t, r, v):
    # Moss cushions over damp earth: the green has to read at gameplay zoom, or a mossy wood
    # floor looks like bare soil (2026-10-17, once the generator laid it by biome).
    patches(t, r, [P('earth', 2), P('foliage', 4)], 3)
    stamp_scatter(t, r, MOSS, {'m': P('foliage', 3)}, 6, min_dist=7)
    stamp_scatter(t, r, LEAF, {'d': P('foliage', 4), 'l': P('foliage', 5)}, 2, min_dist=11)
    stamp_scatter(t, r, ['h'], {'h': P('foliage', 3)}, 2, min_dist=10)
    stamp_scatter(t, r, ['.d'], {'d': P('earth', 1)}, 2, min_dist=9)


def ash(t, r, v):
    # Burned ground: charcoal-brown, not stone. Dark earth mottled with soot, black char flecks,
    # pale ash drifts, and the odd blackened stalk.
    mottle(t, r, [P('earth', 1), P('earth', 0), P('stone', 1)], 4, w=(2, 4), h=(1, 3))
    stamp_scatter(t, r, ['ss', '.s'], {'s': N('black')}, 3, min_dist=8)
    stamp_scatter(t, r, ['hh.', '.hh'], {'h': P('stone', 4)}, 3, min_dist=10)
    stamp_scatter(t, r, ['h'], {'h': P('stone', 5)}, 3, min_dist=9)
    stamp_scatter(t, r, ['s', 's'], {'s': P('earth', 0)}, 2, min_dist=10)


def scree(t, r, v):
    patches(t, r, [P('stone', 4), P('stone', 5)], 2)
    stamp_scatter(t, r, PEBBLE, {'h': P('stone', 6), 'm': P('stone', 5), 'd': P('stone', 2)}, 3, min_dist=7)
    stamp_scatter(t, r, STONE2, {'h': P('stone', 6), 'd': P('stone', 2)}, 3, min_dist=7)
    stamp_scatter(t, r, ['d'], {'d': P('stone', 3)}, 2, min_dist=8)
    stamp_scatter(t, r, ['h'], {'h': P('stone', 5)}, 2, min_dist=9)


def packed_earth(t, r, v):
    patches(t, r, [P('earth', 4), P('earth', 5)], 3)
    stamp_scatter(t, r, PEBBLE, {'h': P('stone', 6), 'm': P('earth', 6), 'd': P('earth', 3)}, 2, min_dist=10)
    for (x, y) in scatter(r, 2, T, T, min_dist=12):
        crack(t, r, x, y, r.randint(4, 7), P('earth', 3), P('earth', 5))
    stamp_scatter(t, r, ['.d'], {'d': P('earth', 2)}, 3, min_dist=7)


def frozen_soil(t, r, v):
    """Soil on a frosty morning: the soil's own brown, rimed with fine white frost and a few
    cold grey flecks. #202 (QA 2026-10-20, filed as sand): a white base with three brown
    earth patches a tile read at 2x as a leopard print, over every field below freezing."""
    patches(t, r, [P('earth', 3)], 1 + v % 2)
    mottle(t, r, [P('earth', 5)], 3, w=(2, 3), h=(1, 2))
    stamp_scatter(t, r, ['w'], {'w': P('stone', 6)}, 5, min_dist=6)
    stamp_scatter(t, r, ['w'], {'w': N('white')}, 6, min_dist=5)
    stamp_scatter(t, r, ['d'], {'d': P('cool', 7)}, 2, min_dist=9)
    stamp_scatter(t, r, CLOD_SMALL, {'h': N('light'), 'm': P('earth', 5), 'd': P('earth', 2)}, 1, min_dist=12)


def wet_soil(t, r, v):
    # Very wet, mucky ground: dark earth with small puddle glistens.
    patches(t, r, [P('earth', 0), P('earth', 1)], 3)
    for (x, y) in scatter(r, 2, T, T, min_dist=10):
        t.stamp(POOL, x, y, {'w': P('water', 3)}, wrap=(T, T))
    stamp_scatter(t, r, CLOD, {'h': P('earth', 1), 'm': P('earth', 0), 'd': P('earth', 0)}, 4, min_dist=7)
    stamp_scatter(t, r, ['h'], {'h': P('water', 3)}, 3, min_dist=8)


# ---------------------------------------------------------------- families
def grass(t, r, v):
    # Grass covers huge areas, so a full set of high-frequency tufts turns into visual static
    # at normal viewing zoom. Keep the broad tonal patches; use fewer, more separated blades so
    # figures, plants and terrain transitions retain the first read.
    patches(t, r, [P('foliage', 3), P('foliage', 5)], 2)
    stamp_scatter(t, r, TUFT, {'h': P('foliage', 6), 'd': P('foliage', 2)}, 2, min_dist=12)
    stamp_scatter(t, r, TUFT2, {'h': P('foliage', 5), 'd': P('foliage', 2)}, 2, min_dist=11)
    stamp_scatter(t, r, TUFT_SMALL, {'h': P('foliage', 5), 'd': P('foliage', 2)}, 1, min_dist=8)
    stamp_scatter(t, r, ['d'], {'d': P('foliage', 2)}, 1, min_dist=9)
    if v % 8 == 3:  # one in eight has flowers
        stamp_scatter(t, r, FLOWER, {'p': P('warm', 6), 'c': P('warm', 7), 's': P('foliage', 2)}, 2, min_dist=10)
        stamp_scatter(t, r, FLOWER_SMALL, {'p': N('white'), 's': P('foliage', 2)}, 2, min_dist=8)
    if v % 8 == 6:
        stamp_scatter(t, r, FLOWER_SMALL, {'p': P('cool', 6), 's': P('foliage', 2)}, 2, min_dist=10)


def grass_autumn(t, r, v):
    # The autumn look of grass (drawn instead of grass in autumn; not a separate family):
    # the same tufts, gone to straw at the tips over a duller green, with a few fallen leaves.
    patches(t, r, [P('foliage', 2), P('earth', 5)], 2)
    stamp_scatter(t, r, TUFT, {'h': P('earth', 6), 'd': P('foliage', 2)}, 2, min_dist=12)
    stamp_scatter(t, r, TUFT2, {'h': P('earth', 5), 'd': P('foliage', 2)}, 2, min_dist=11)
    stamp_scatter(t, r, TUFT_SMALL, {'h': P('foliage', 4), 'd': P('foliage', 2)}, 1, min_dist=8)
    stamp_scatter(t, r, ['d'], {'d': P('foliage', 2)}, 1, min_dist=9)
    stamp_scatter(t, r, ['lw'], {'l': P('warm', 4), 'w': P('warm', 3)}, 1 + v % 2, min_dist=10)


def soil(t, r, v):
    # Lifted off the dark-stone value band (they were 2.5 luma apart and read as the same
    # tile) and warmed, with pale root fibres as a signature: soil is worked ground and must
    # never read as bare rock.
    patches(t, r, [P('earth', 3), P('earth', 5)], 3)
    stamp_scatter(t, r, CLOD, {'h': P('earth', 5), 'm': P('earth', 4), 'd': P('earth', 2)}, 4, min_dist=8)
    stamp_scatter(t, r, CLOD_SMALL, {'h': P('earth', 5), 'm': P('earth', 4), 'd': P('earth', 3)}, 3, min_dist=7)
    stamp_scatter(t, r, PEBBLE, {'h': P('stone', 6), 'm': P('stone', 5), 'd': P('earth', 2)}, 1, min_dist=11)
    stamp_scatter(t, r, ['hh', '.h'], {'h': P('warm', 6)}, 2, min_dist=11)


def clay(t, r, v):
    # Lifted to the light, orange end: it was 0.6 luma from wood_floor, so worked floor and
    # bare clay read as the same brown. Its crack network is the signature at this value.
    patches(t, r, [P('earth', 5), P('earth', 7)], 2)
    stamp_scatter(t, r, ['..hh', '.hh.'], {'h': P('warm', 5)}, 2, min_dist=12)
    for (x, y) in scatter(r, 3, T, T, min_dist=9):
        crack(t, r, x, y, r.randint(5, 8), P('earth', 4), P('earth', 7))
    stamp_scatter(t, r, PEBBLE, {'h': P('earth', 7), 'm': P('earth', 6), 'd': P('earth', 4)}, 2, min_dist=8)


def sand(t, r, v):
    """Pale sand with soft wind ripples. #202 (QA 2026-10-20): two dark earth patches and six
    flecks a tile, repeated at tile pitch, read at 2x as a leopard print; now the tonal
    variation is a rare pale drift and wind ripples, and dark is kept to a rare pebble."""
    if v % 2 == 0:
        patches(t, r, [P('earth', 7)], 1)
    stamp_scatter(t, r, RIPPLE, {'h': P('earth', 7)}, 3, min_dist=9)
    stamp_scatter(t, r, ['h'], {'h': P('earth', 7)}, 3, min_dist=6)
    stamp_scatter(t, r, PEBBLE, {'h': P('stone', 6), 'm': P('stone', 5), 'd': P('earth', 5)}, 1 if v % 4 == 0 else 0)


def gravel(t, r, v):
    patches(t, r, [P('stone', 3), P('stone', 5)], 2)
    stamp_scatter(t, r, PEBBLE_BIG, {'h': P('stone', 6), 'm': P('stone', 5), 'd': P('stone', 2)}, 2, min_dist=10)
    stamp_scatter(t, r, PEBBLE, {'h': P('stone', 6), 'm': P('stone', 5), 'd': P('stone', 2)}, 4, min_dist=6)
    stamp_scatter(t, r, PEBBLE, {'h': P('earth', 6), 'm': P('earth', 5), 'd': P('stone', 2)}, 1, min_dist=9)


def stone(ramp, base_step, t, r, v, flecks=4, cracks=2, tint=None):
    patches(t, r, [P(ramp, base_step - 1)], 2)
    stamp_scatter(t, r, ['.hh.', 'hhhh', '.hh.'], {'h': P(ramp, min(7, base_step + 1))}, 2, min_dist=12)
    if tint:
        stamp_scatter(t, r, ['.hh', 'hh.'], {'h': tint}, 2, min_dist=12)
    for (x, y) in scatter(r, cracks, T, T, min_dist=10):
        crack(t, r, x, y, r.randint(7, 12), P(ramp, max(0, base_step - 3)), P(ramp, min(7, base_step + 2)))
    stamp_scatter(t, r, ['h'], {'h': P(ramp, min(7, base_step + 2))}, max(1, flecks // 2), min_dist=7)
    stamp_scatter(t, r, ['hh', '.d'], {'h': P(ramp, min(7, base_step + 1)), 'd': P(ramp, max(0, base_step - 2))}, 1, min_dist=10)


def pale_stone(t, r, v):
    stone('stone', 6, t, r, v, tint=P('earth', 6))


def warm_stone(t, r, v):
    # Taken down a step and pushed warm so that it no longer shares a value with stone_floor.
    stone('earth', 3, t, r, v, tint=P('warm', 4))


def grey_stone(t, r, v):
    # Slightly cool so it does not read as pale thatch once thatch went golden.
    stone('stone', 4, t, r, v, tint=P('cool', 4))


def dark_stone(t, r, v):
    # Unexcavated rock needs a readable layer direction at a glance. Keep it on the stone ramp
    # (so it remains materially distinct from cave_floor's earth ramp), then add broad strata,
    # broken seams and a few cool inclusions. The bands are texture, not a new geological fact.
    patches(t, r, [P('stone', 1), P('stone', 2)], 3)
    for y in [5 + (v * 3) % 5, 16 + (v * 5) % 4, 26 - (v * 2) % 4]:
        dy = r.choice((-1, 0, 0, 1))
        for x in range(T):
            if x % 5 == 0:
                dy = r.choice((-1, 0, 0, 1))
            t.px(x, y + dy, P('stone', 0))
            if x % 3 != 1:
                t.px(x, y + dy - 1, P('stone', 3))
    stamp_scatter(t, r, ['.h.', 'hHd', '.d.'],
                  {'h': P('cool', 3), 'H': P('cool', 4), 'd': P('stone', 0)},
                  2, min_dist=10)
    for (x, y) in scatter(r, 2, T, T, min_dist=11):
        crack(t, r, x, y, r.randint(5, 9), P('stone', 0), P('stone', 2))


def black_stone(t, r, v):
    # The black base is the material's unexcavated/deep-rock read. It stays the darkest family,
    # but its layered edges and sparse alien-looking inclusions stop underground from becoming a
    # featureless void once the ambient floor is applied.
    patches(t, r, [P('stone', 0), P('stone', 1)], 2)
    for y in [4 + (v * 2) % 4, 14 + (v * 3) % 5, 25 - (v % 3)]:
        dy = 0
        for x in range(T):
            if x % 4 == 0:
                dy = r.choice((-1, 0, 0, 1))
            t.px(x, y + dy, P('stone', 2))
            if x % 5 in (0, 1):
                t.px(x, y + dy - 1, P('cool', 2))
    stamp_scatter(t, r, ['.hh', 'hH.', '.d.'],
                  {'h': P('stone', 2), 'H': P('cool', 4), 'd': N('black')},
                  3, min_dist=9)
    for (x, y) in scatter(r, 2, T, T, min_dist=10):
        crack(t, r, x, y, r.randint(5, 9), N('black'), P('stone', 2))
    # A restrained cool glint gives ancient/alien stone a signature without turning every dark
    # tile into a crystal deposit; actual gem-bearing rock remains ore_gem below.
    stamp_scatter(t, r, ['h', '.h'], {'h': P('cool', 3)}, 2, min_dist=8)


def ore(body, glint, dark, rows=2, glints=2):
    def f(t, r, v):
        patches(t, r, [P('stone', 3), P('stone', 5)], 2)
        for (cx, cy) in scatter(r, 2, T, T, min_dist=12):
            crack(t, r, cx, cy, r.randint(4, 7), P('stone', 1), P('stone', 5))
        # Seams are placed from the variant index rather than drawn randomly the same way for
        # every tile: the review read the old set as stamped. Longer main seam, always a second
        # seam, and more glints so the ore colour reads at map zoom.
        x = (v * 11) % T
        y = (v * 7 + 3) % T
        vein(t, r, x, y, r.randint(10, 15), body, glint, dark, rows=rows)
        x2, y2 = (x + 13 + v * 2) % T, (y + 9) % T
        vein(t, r, x2, y2, r.randint(5, 9), body, glint, dark, rows=rows)
        # Scattered glints are tuned per mineral: copper is already loud so it gets few and
        # larger masses, iron is subtle so it gets more specks. Piling glints on every ore made
        # them read as noise rather than mineral.
        stamp_scatter(t, r, ['h'], {'h': glint}, glints, min_dist=5)
        if glints >= 3:
            stamp_scatter(t, r, ['hh'], {'h': glint}, 1, min_dist=9)
        # Small host-rock breaks around a vein make the deposit read as embedded in stone,
        # while the body/glint colours remain the only mineral identity shown.
        stamp_scatter(t, r, ['.d.', 'd.d', '.d.'], {'d': dark}, 1, min_dist=12)
    return f


def marble(t, r, v):
    """Limestone baked to marble (C32): the palest stone, cool, with long thin grey veins
    wandering across it so it reads as marble and not as snow or pale limestone."""
    stone('stone', 7, t, r, v, cracks=0, flecks=1, tint=P('cool', 6))
    x, y = (v * 9) % T, (v * 5 + 2) % T
    vein(t, r, x, y, r.randint(14, 20), P('stone', 4), P('stone', 5), P('stone', 3), rows=1)
    vein(t, r, (x + 15) % T, (y + 11) % T, r.randint(8, 12), P('stone', 4), P('stone', 5),
         P('stone', 3), rows=1)
    vein(t, r, (x + 7) % T, (y + 21) % T, r.randint(6, 10), P('stone', 5), P('stone', 6),
         P('stone', 4), rows=1)


def milkstone(t, r, v):
    """The bone forest's rock (C59): warm white, softer and cloudier than marble, with milky
    blooms and a few fine bone-coloured veins, so it reads as its own stone and not as snow."""
    stone('stone', 7, t, r, v, cracks=1, flecks=0, tint=P('gold', 7))
    mottle(t, r, [P('stone', 6), P('gold', 7)], 4, w=(3, 5), h=(2, 3))
    x, y = (v * 7) % T, (v * 11 + 3) % T
    vein(t, r, x, y, r.randint(8, 12), P('gold', 6), P('stone', 6), P('stone', 5), rows=1)


def bone_moss(t, r, v):
    """The bone forest's ground (C59): a pale moss, greenish white with cool shadows, in soft
    cushions; white twigs of fallen bonewood lie in it and small blue-white caps grow from it,
    so it reads as living ground of a strange kind, not as paving. #201 (QA 2026-10-20): on a
    grey base with grey mottle it read as scree; it is now a pale green-cream, lighter and
    warmer than any scree or gravel and greener than snow."""
    patches(t, r, [P('olive', 6), P('stone', 7)], 4)
    mottle(t, r, [P('olive', 6), N('light')], 4, w=(2, 4), h=(1, 3))
    stamp_scatter(t, r, MOSS, {'m': P('olive', 6)}, 7, min_dist=5)
    stamp_scatter(t, r, ['hhh', '..h'], {'h': N('white')}, 2, min_dist=10)
    stamp_scatter(t, r, ['.c.', 'cCc'], {'c': P('cool', 6), 'C': P('cool', 7)}, 2, min_dist=9)
    stamp_scatter(t, r, ['d'], {'d': P('olive', 5)}, 5, min_dist=6)


def gem_rock(t, r, v):
    """The deep basalt with rough gems in it: clusters of faceted cool crystals with a lit face
    and a white glint, and on alternate tiles an amber one, big enough to read at map zoom as
    gems rather than another ore."""
    black_stone(t, r, v)
    crystal = ['..h..', '.hHd.', 'dHWhd', '.ddd.']
    # Gem-bearing rock gets actual gem-ramp facets, alternating with cool and warm inclusions so
    # a cavern can feel strange without asserting a specific gem species the simulation does not
    # know at this layer.
    stamp_scatter(t, r, crystal, {'h': P('gem', 5), 'H': P('gem', 7), 'W': N('white'),
                                  'd': P('gem', 2)}, 2, min_dist=12)
    stamp_scatter(t, r, ['.h.', 'hHd', '.d.'], {'h': P('cool', 5), 'H': P('cool', 7),
                                                 'd': P('cool', 2)}, 2, min_dist=8)
    if v % 2 == 0:
        stamp_scatter(t, r, ['.h.', 'hHd', '.d.'], {'h': P('warm', 5), 'H': P('warm', 7),
                                                     'd': P('warm', 2)}, 1, min_dist=9)
    stamp_scatter(t, r, ['.g.', 'gGg', '.d.'], {'g': P('gem', 4), 'G': P('gem', 6),
                                                 'd': P('stone', 0)}, 2, min_dist=10)


def water_base(t, r, v, frame, base, lo, hi, crest, tip, n_crest, deep=False):
    """Draw one animated water tile with broad, deterministic current bands."""
    r2 = rng('water-static', base, v)         # mottle is per variant, not per frame
    mottle(t, r2, [lo, hi], 9 if deep else 8, w=(3, 6), h=(1, 2))
    # Long, broken bands survive a large lake view better than isolated flecks. Their phase
    # shifts by frame while their anchors remain variant-stable, so animation reads as flow.
    flow = rng('water-flow', base, v)
    for band in range(2 if deep else 3):
        y = flow.randint(1, T - 2)
        x = flow.randint(0, T - 1)
        length = flow.randint(8, 15) if deep else flow.randint(6, 12)
        drift = (frame * (1 if deep else 2) + band) % 5
        col = lo if band % 2 == 0 else hi
        for k in range(length):
            if (k + band * 3) % 7 != 5:
                t.px(x + k + drift, y + ((k + band) // 5 % 2), col)
    # Sparse moving highlights provide a readable surface cue at 1x without white noise.
    glint = rng('water-glints', base, v, frame)
    for _ in range(3 if deep else 5):
        x, y = glint.randint(0, T - 1), glint.randint(0, T - 1)
        t.px(x, y, crest if glint.random() < 0.7 else tip)
    r3 = rng('water-crests', base, v)
    pts = scatter(r3, n_crest, T, T, min_dist=7)
    for i, (x, y) in enumerate(pts):
        phase = (frame + i) % 4
        xx = x + phase
        length = 3 + (i % 2)
        if phase == 3:
            length = 2
        for k in range(length):
            t.px(xx + k, y, crest)
        if phase in (1, 2):
            t.px(xx + length, y - 1, tip)
        if phase == 2:
            t.px(xx - 1, y + 1, lo)


def water_sheet(name, base, lo, hi, crest, tip, n_crest, deep=False):
    frames = []
    for v in range(2):
        for frame in range(4):
            t = Tile(T, T, fill=base)
            water_base(t, rng(name, v, frame), v, frame, base, lo, hi, crest, tip, n_crest, deep=deep)
            frames.append(t)
    s = sheet(frames, T, T, cols=4)
    s.save(os.path.join(OUT, name + '.png'))


def wood_floor(t, r, v):
    # Deepened: it sat 0.6 luma from clay, so a laid floor and bare clay read alike. Plank
    # rhythm plus the lit top edge is its signature, not its value.
    plank_h = 8
    offset = (v * 11) % 32
    for row in range(4):
        y0 = row * plank_h
        t.rect(0, y0, T, plank_h, P('earth', 2))
        t.hline(0, T - 1, y0, P('earth', 4))               # lit top edge
        t.hline(0, T - 1, y0 + plank_h - 1, P('earth', 1))  # seam
        # grain: 2 broken lines per plank
        for g in range(2):
            gy = y0 + 2 + g * 3 + r.randint(0, 1)
            gx = r.randint(0, T - 1)
            for k in range(r.randint(5, 11)):
                t.px(gx + k, gy, P('earth', 3))
        # plank end seam (vertical) once per row, position wraps
        ex = (offset + row * 13) % T
        t.vline(ex, y0, y0 + plank_h - 2, P('earth', 1))
        t.vline(ex + 1, y0 + 1, y0 + plank_h - 2, P('earth', 4))
        if r.random() < 0.5:
            kx, ky = r.randint(0, T - 1), y0 + r.randint(2, 5)
            t.stamp(['.d.', 'dmd', '.d.'], kx, ky, {'d': P('earth', 2), 'm': P('earth', 3)}, wrap=(T, T))


def stone_floor(t, r, v):
    # Was 0.8 luma from warm_stone: same value, so paving and a warm rock face read alike.
    # Cut stone goes cooler and lighter; warm_stone takes the warm side instead.
    mortar = P('stone', 4)
    t.rect(0, 0, T, T, mortar)
    for row in range(2):
        y0 = row * 16
        x = 0
        widths = []
        remaining = T
        while remaining > 0:
            w = min(remaining, r.choice((8, 10, 12, 14, 16)))
            if remaining - w < 6 and remaining - w > 0:
                w = remaining
            widths.append(w)
            remaining -= w
        for w in widths:
            shade = r.choice((5, 6, 6))
            t.rect(x + 1, y0 + 1, w - 1, 14, P('stone', shade))
            t.hline(x + 1, x + w - 1, y0 + 1, P('stone', shade + 1))
            t.vline(x + 1, y0 + 1, y0 + 14, P('stone', shade + 1))
            t.hline(x + 1, x + w - 1, y0 + 14, P('stone', shade - 1))
            t.vline(x + w - 1, y0 + 1, y0 + 14, P('stone', shade - 1))
            for _ in range(2):
                t.px(x + r.randint(2, w - 2), y0 + r.randint(3, 12), P('stone', shade - 1))
            if r.random() < 0.3:
                crack(t, r, x + 3, y0 + r.randint(3, 10), 4, P('stone', shade - 2), P('stone', shade + 1))
            x += w


def thatch(t, r, v):
    # Thatch was within 0.6 luma of grey_stone and 0.2 of ore_copper — indistinguishable at
    # map zoom. Straw is golden, so it now takes the warm ramp while keeping earth for the
    # shadowed courses: separated by hue where value alone could not do it.
    t.rect(0, 0, T, T, P('warm', 5))
    for row in range(4):
        y0 = row * 8
        phase = (row % 2) * 2 + (v % 3)
        # The old version drew every course as two continuous straight lines with straws at
        # exactly 4 px, which read machine-made the moment it tiled. Here the course shadow is
        # laid down in jittered runs and straw spacing varies 3-5 px, so the weave breaks up.
        x = 0
        while x < T:
            seg = r.randint(4, 9)
            yy = y0 + 7 - (0 if r.random() < 0.6 else 1)
            for k in range(seg):
                t.px((x + k) % T, yy, P('earth', 3))
            for k in range(r.randint(3, seg)):
                t.px((x + k) % T, yy - 1, P('earth', 4))
            x += seg + r.randint(2, 6)
        x = phase
        while x < T:
            ln = 5 if r.random() < 0.85 else 7  # the odd long straw crossing into the next course
            t.line(x, y0, x + 2, y0 + ln, P('warm', 4))
            t.line(x + 1, y0, x + 3, y0 + ln, P('warm', 6))
            x += r.randint(3, 5)
    stamp_scatter(t, r, ['h'], {'h': P('warm', 6)}, 4, min_dist=6)
    stamp_scatter(t, r, ['d'], {'d': P('earth', 2)}, 3, min_dist=6)


def peat(t, r, v):
    # Bog ground: dark maroon-brown peat under olive sphagnum, with pale stringy fibres as its
    # signature (mud is mottled and glistens, peat is stringy and mossy). The bright orange
    # patches it had read as embers once the generator laid whole fens of it (2026-10-17).
    patches(t, r, [P('foliage', 2), P('foliage', 3)], 3)
    stamp_scatter(t, r, ['hhh', '..h'], {'h': P('earth', 4)}, 3, min_dist=8)
    stamp_scatter(t, r, ['.hhhh.'], {'h': P('earth', 3)}, 1, min_dist=11)
    stamp_scatter(t, r, MOSS, {'m': P('foliage', 3)}, 3, min_dist=8)
    stamp_scatter(t, r, ['h'], {'h': P('foliage', 4)}, 2, min_dist=9)
    stamp_scatter(t, r, ['h'], {'h': P('warm', 2)}, 1, min_dist=11)
    stamp_scatter(t, r, ['d'], {'d': P('earth', 0)}, 2, min_dist=8)


def snow(t, r, v):
    patches(t, r, [P('stone', 7), N('white')], 3)
    stamp_scatter(t, r, ['.d', 'd.'], {'d': P('cool', 6)}, 3, min_dist=9)
    stamp_scatter(t, r, ['d'], {'d': P('stone', 6)}, 2, min_dist=9)
    stamp_scatter(t, r, ['w'], {'w': N('white')}, 4, min_dist=7)


def mud(t, r, v):
    # Pushed down and made properly wet: at earth 1/3 it sat where the deepened wood_floor
    # now sits, so saturated ground read as laid timber. Darker base plus a stronger wet
    # sheen (water ramp glisten over damp dark) is the signature.
    patches(t, r, [P('earth', 0), P('earth', 2)], 3)
    for (x, y) in scatter(r, 2, T, T, min_dist=12):
        t.stamp(['.wwwww.', 'wwwhwww', '.wwwww.'], x, y, {'w': P('earth', 0), 'h': P('water', 3)}, wrap=(T, T))
        t.stamp(['.ddddd.'], x, y - 1, {'d': P('earth', 0)}, wrap=(T, T))
        t.stamp(['.lllll.'], x, y + 3, {'l': P('earth', 2)}, wrap=(T, T))
    stamp_scatter(t, r, CLOD_SMALL, {'h': P('earth', 2), 'm': P('earth', 1), 'd': P('earth', 0)}, 3, min_dist=8)
    stamp_scatter(t, r, ['h'], {'h': P('water', 4)}, 2, min_dist=9)


def cave_floor(t, r, v):
    # Was 1.9 luma and 5.8 colour from black_stone — the same tile twice. A cave floor is
    # damp earth under rock, not volcanic rock: dark ON THE EARTH ramp, with a sparse damp
    # glisten, so the pair separates by hue family at equal darkness.
    patches(t, r, [P('earth', 0), P('earth', 2), P('earth', 1)], 3)
    # Compacted cave soil: broad damp patches, a few angular chips, root-like cracks and a
    # restrained alien mineral fleck. Water itself is not painted here; seepage is overlaid only
    # by the client when a real water tile is adjacent.
    stamp_scatter(t, r, PEBBLE_BIG, {'h': P('stone', 4), 'm': P('stone', 2), 'd': N('black')}, 2, min_dist=11)
    stamp_scatter(t, r, PEBBLE, {'h': P('stone', 3), 'm': P('stone', 2), 'd': N('black')}, 3, min_dist=7)
    stamp_scatter(t, r, ['.h.', 'hHd', '.d.'],
                  {'h': P('cool', 3), 'H': P('gem', 4), 'd': P('earth', 0)},
                  2, min_dist=10)
    stamp_scatter(t, r, ['h'], {'h': P('warm', 2)}, 1, min_dist=11)
    stamp_scatter(t, r, ['h'], {'h': P('water', 3)}, 2, min_dist=11)
    for (x, y) in scatter(r, 2, T, T, min_dist=11):
        crack(t, r, x, y, r.randint(5, 8), N('black'), P('earth', 2))


RECIPES = {
    'grass': (P('foliage', 4), grass),
    'soil': (P('earth', 3), soil),
    'clay': (P('earth', 4), clay),
    'sand': (P('earth', 6), sand),
    'gravel': (P('stone', 4), gravel),
    'pale_stone': (P('stone', 6), pale_stone),
    'warm_stone': (P('earth', 5), warm_stone),
    'grey_stone': (P('stone', 4), grey_stone),
    'dark_stone': (P('stone', 2), dark_stone),
    'black_stone': (P('stone', 1), black_stone),
    # Coal seams are thick bands; metals are thin veins. That difference is how the three
    # minerals are told apart at a glance once they share one stone host.
    # Iron is the subtle metal: more glints. Copper is already the loudest tile, so it gets
    # few specks and relies on its streaks.
    'ore_iron': (P('stone', 4), ore(P('earth', 4), P('earth', 6), P('earth', 2), glints=3)),
    'ore_copper': (P('stone', 4), ore(P('warm', 4), P('warm', 6), P('warm', 2), glints=1)),
    'ore_coal': (P('stone', 4), ore(P('stone', 0), P('cool', 5), N('black'), rows=3, glints=2)),
    'ore_gem': (P('stone', 1), gem_rock),
    # C32: tin is a dark, dull metal (black cassiterite with cool glints); silver pale grey
    # threads with white glints; gold short bright veins with many glints, the rarest tile.
    'ore_tin': (P('stone', 4), ore(P('stone', 1), P('cool', 5), P('stone', 0), glints=2)),
    'ore_silver': (P('stone', 4), ore(P('stone', 6), N('white'), P('stone', 3), glints=3)),
    'ore_gold': (P('stone', 4), ore(P('gold', 4), P('gold', 6), P('gold', 2), glints=3)),
    'marble': (P('stone', 7), marble),
    'milkstone': (P('stone', 7), milkstone),
    'bone_moss': (P('olive', 7), bone_moss),
    'wood_floor': (P('earth', 4), wood_floor),
    'stone_floor': (P('stone', 4), stone_floor),
    'thatch': (P('earth', 5), thatch),
    'peat': (P('earth', 2), peat),
    'snow': (N('light'), snow),
    'mud': (P('earth', 2), mud),
    'cave_floor': (P('stone', 1), cave_floor),
    'wildflower': (P('foliage', 4), wildflower),
    'mossy': (P('earth', 3), mossy),
    'ash': (P('earth', 1), ash),
    'scree': (P('stone', 4), scree),
    'packed_earth': (P('earth', 4), packed_earth),
    'frozen_soil': (P('earth', 4), frozen_soil),
    'wet_soil': (P('earth', 1), wet_soil),
    'grass_autumn': (P('foliage', 3), grass_autumn),
}


# Open ground covers large parts of the map and is sampled independently per tile by the
# renderer. Give grass more source variation than compact materials so neighbouring tiles are
# less likely to repeat the same tuft/patch arrangement. This changes only visual assets; the
# tile ids, simulation and renderer protocol remain unchanged.
VARIANT_COUNTS = {'grass': 24, 'grass_autumn': 24}


def make_family(name):
    base, fn = RECIPES[name]
    tiles = []
    for v in range(VARIANT_COUNTS.get(name, 8)):
        t = Tile(T, T, fill=base)
        fn(t, rng('terrain', name, v), v)
        tiles.append(t)
    sheet(tiles, T, T).save(os.path.join(OUT, name + '.png'))


# ---------------------------------------------------------------- edge blending between families
# Where two ground families meet, the higher one in BLEND_ORDER spills over its neighbour with an
# irregular, dithered boundary, so a border reads as ground meeting ground rather than two
# stamped squares. world_view.gd draws the pieces from the sheets written here:
# godot/assets/terrain/edges/<family>.png, 20 columns x 4 rows of 32x32 (row = mask and texture
# variant, chosen per receiving tile so all pieces on a tile share one row; columns in groups of
# four, each group in the order given):
#   0-3    edges N E S W: the neighbour on that side is the family
#   4-7    corners NE SE SW NW: only the diagonal neighbour is
#   8-11   round fillets NE SE SW NW: both sides of that corner are, and so are both cells
#          beyond them (the inside corner of a straight-edged mass): a small rounded corner
#   12-15  mid fillets: both sides are, and one cell beyond them
#   16-19  chamfers: both sides are and neither cell beyond them is, which is a diagonal
#          staircase: cut straight across so the run reads as a slope, not as steps
# Every edge has depth EDGE_D0 at both ends and every corner and chamfer meets the tile border
# at that same depth, so pieces on adjacent tiles join without a step. Coverage is binary
# (opaque palette pixel or fully transparent), so the sheets stay on the palette.
BLEND_ORDER = ['black_stone', 'dark_stone', 'grey_stone', 'warm_stone', 'pale_stone', 'gravel', 'scree',
               'mud', 'wet_soil', 'sand', 'clay', 'peat', 'packed_earth', 'soil', 'frozen_soil', 'mossy',
               'bone_moss', 'grass', 'wildflower', 'ash', 'snow']
EDGE_OUT = os.path.join(OUT, 'edges')
EDGE_D0 = 6.0          # depth of every edge where it meets a tile corner, in px
EDGE_ROWS = 4
BAYER4 = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]]
YS, XS = np.mgrid[0:T, 0:T] + 0.5


def _profile(r):
    """Edge depth per pixel column: EDGE_D0 at both ends, wandering about it through five
    smoothstepped spans so the wander does not repeat with the tile."""
    # the knots next to the ends stay near EDGE_D0, or every seam pinches into a notch
    near = [r.uniform(-1.5, 1.5) for _ in range(2)]
    knots = [EDGE_D0, EDGE_D0 + near[0], EDGE_D0 + r.uniform(-3.5, 5.0), EDGE_D0 + r.uniform(-3.5, 5.0),
             EDGE_D0 + near[1], EDGE_D0]
    out = np.zeros(T)
    seg = T / (len(knots) - 1)
    for x in range(T):
        u = (x + 0.5) / seg
        i = min(int(u), len(knots) - 2)
        t = u - i
        t = t * t * (3 - 2 * t)
        out[x] = knots[i] * (1 - t) + knots[i + 1] * t
    # one or two rounded bumps or bites, 4-7 px wide, away from the ends
    for _ in range(r.randint(1, 2)):
        c, half = r.uniform(7, T - 7), r.uniform(2.0, 3.5)
        d = np.clip(1.0 - np.abs(np.arange(T) + 0.5 - c) / half, 0.0, 1.0)
        out += r.choice((-1.5, 1.5, 2.5)) * d * d * (3 - 2 * d)
    return out


def _wobble(r, amp):
    """Angular wobble for corner shapes, zero where they meet the tile border."""
    theta = np.arctan2(YS, XS)
    a1, a2 = r.uniform(-amp, amp), r.uniform(-amp, amp)
    return a1 * np.sin(2 * theta) + a2 * np.sin(4 * theta)


def _field(kind, r):
    """Signed coverage depth in the canonical orientation (edge on the north side, corners at
    the north-west): positive inside the spilling family."""
    if kind == 'edge':
        return _profile(r)[None, :] - YS
    if kind == 'corner':
        return EDGE_D0 + 0.8 + _wobble(r, 1.2) - np.hypot(XS, YS)
    # Fillets: the N and W edges are drawn as well, so these only add coverage inside the
    # corner. Each meets the far tile borders no deeper than EDGE_D0.
    if kind == 'round':
        return 13.0 + _wobble(r, 1.5) - np.hypot(XS, YS)
    diag = (XS + YS) / np.sqrt(2.0)
    if kind == 'mid':
        return (27.0 + _wobble(r, 2.0)) / np.sqrt(2.0) - diag
    # chamfer: the line x + y = 32 + D0, so it leaves the tile at depth D0 on the E and S sides,
    # exactly where the next corner piece of the staircase picks it up
    # a slow wander along the cut, zero where it leaves the tile, so long slopes are not ruled
    u = np.clip((XS - YS + T - EDGE_D0) / (2 * (T - EDGE_D0)), 0.0, 1.0)
    wander = r.uniform(-3.0, 3.0) * np.sin(np.pi * u) + r.uniform(-1.5, 1.5) * np.sin(2 * np.pi * u)
    return (T + EDGE_D0 - 1.0) / np.sqrt(2.0) - diag + wander


def edge_masks():
    """masks[column][row]: boolean 32x32 arrays in sheet order."""
    bayer = (np.array(BAYER4, dtype=float) + 0.5) / 16.0
    thresh = np.tile(bayer, (T // 4, T // 4)) * 2.0 - 1.0  # ordered dither across +-1 px
    masks = []
    # (kind, quarter turns clockwise from the canonical orientation)
    groups = [('edge', (0, 1, 2, 3))] + [(k, (1, 2, 3, 0)) for k in ('corner', 'round', 'mid', 'chamfer')]
    for kind, turns in groups:
        for q in turns:
            col = []
            for m in range(EDGE_ROWS):
                f = np.rot90(_field(kind, rng('edge-mask', kind, q, m)), -q)
                col.append(f > thresh)
            masks.append(col)
    return masks


def make_edges(name, masks):
    fam = np.asarray(Image.open(os.path.join(OUT, name + '.png')).convert('RGBA'))
    out = Canvas(T * len(masks), T * EDGE_ROWS)
    for v in range(EDGE_ROWS):
        # rows 0-3 cut from variants 0, 2, 4, 6 so each row has its own texture
        tile = fam[0:T, (2 * v) * T:(2 * v + 1) * T]
        for p, col in enumerate(masks):
            m = col[v]
            out.a[v * T:(v + 1) * T, p * T:(p + 1) * T][m] = tile[m]
    out.save(os.path.join(EDGE_OUT, name + '.png'))


def make_shore(masks):
    """edges/shore.png, same layout: a 1-px crest-coloured line just outside each piece's
    coverage and a sparse second line beyond it. Drawn on water under the bank's own piece, it
    reads as the wet line where water laps the bank."""
    out = Canvas(T * len(masks), T * EDGE_ROWS)
    crest, glint = P('water', 5), P('water', 4)
    ys, xs = np.mgrid[0:T, 0:T]

    def grow(m):
        pad = np.pad(m, 1)
        return (pad[:-2, 1:-1] | pad[2:, 1:-1] | pad[1:-1, :-2] | pad[1:-1, 2:]) & ~m

    for v in range(EDGE_ROWS):
        for p, col in enumerate(masks):
            m = col[v]
            ring1 = grow(m)
            ring2 = grow(m | ring1) & (((xs * 3 + ys * 5 + v) % 7) < 2)
            cell = out.a[v * T:(v + 1) * T, p * T:(p + 1) * T]
            cell[ring1, :3] = crest
            cell[ring1, 3] = 255
            cell[ring2, :3] = glint
            cell[ring2, 3] = 255
    out.save(os.path.join(EDGE_OUT, 'shore.png'))


# The 3x3 neighbourhood row by row, 4 being the tile itself; the same tables as world_view.gd.
EDGE_IDX = [1, 5, 7, 3]
CORNER_IDX = [2, 8, 6, 0]
CORNER_SIDES = [[1, 5], [7, 5], [7, 3], [1, 3]]
CORNER_BEYOND = [[0, 8], [2, 6], [8, 0], [2, 6]]
NB_BITS = [0, 1, 2, 3, 5, 6, 7, 8]   # bit i of a neighbour set is cell NB_BITS[i]


def pieces_for(cells):
    """Sheet columns drawn for a family holding these cells of the neighbourhood."""
    cols = [i for i in range(4) if EDGE_IDX[i] in cells]
    for i in range(4):
        a, b = CORNER_SIDES[i][0] in cells, CORNER_SIDES[i][1] in cells
        if a and b:
            o = sum(1 for j in CORNER_BEYOND[i] if j in cells)
            cols.append([16, 12, 8][o] + i)
        elif not a and not b and CORNER_IDX[i] in cells:
            cols.append(4 + i)
    return cols


def make_snow_cover(masks):
    """edges/snow_cover.png: for every set of neighbours (8 bits, 256 cells per row of the
    sheet's 32x8 block) and each mask row, the union of the pieces drawn for it, in snow.
    Snow lies over snowable ground at 82 % alpha; laid piece by piece it would build up where
    pieces overlap, so the client lays this union once instead."""
    snow = np.asarray(Image.open(os.path.join(OUT, 'snow.png')).convert('RGBA'))
    out = Canvas(T * 32, T * 8 * EDGE_ROWS)
    for v in range(EDGE_ROWS):
        tile = snow[0:T, (2 * v) * T:(2 * v + 1) * T]
        for bits in range(256):
            cells = {NB_BITS[i] for i in range(8) if bits >> i & 1}
            m = np.zeros((T, T), dtype=bool)
            for c in pieces_for(cells):
                m |= masks[c][v]
            x0, y0 = (bits % 32) * T, (v * 8 + bits // 32) * T
            out.a[y0:y0 + T, x0:x0 + T][m] = tile[m]
    out.save(os.path.join(EDGE_OUT, 'snow_cover.png'))


def main():
    os.makedirs(OUT, exist_ok=True)
    for name in RECIPES:
        make_family(name)
    water_sheet('water', P('water', 3), P('water', 2), P('water', 4), P('water', 5), P('water', 6), 6)
    water_sheet('deep_water', P('water', 1), P('water', 0), P('water', 2), P('water', 3), P('water', 4), 4, deep=True)
    os.makedirs(EDGE_OUT, exist_ok=True)
    masks = edge_masks()
    for name in BLEND_ORDER:
        make_edges(name, masks)
    make_edges('grass_autumn', masks)     # grass's edge pieces in autumn
    make_shore(masks)
    make_snow_cover(masks)
    missing = [f for f in FAMILIES if not os.path.exists(os.path.join(OUT, f + '.png'))]
    assert not missing, missing
    print('terrain: %d families -> %s' % (len(FAMILIES), OUT))


if __name__ == '__main__':
    main()
