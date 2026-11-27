#!/usr/bin/env python3
"""Plants: godot/assets/plants/
  {oak,birch,pine}_{0,1,2}[_autumn|_winter].png  (autumn canopies for oak and birch; bare
      snowy crowns for oak and birch and snow-laden tiers for pine in winter) stage 0 sapling 32x32, 1 young 32x48, 2 mature 64x64 with the
      trunk base centred in the bottom-middle 32x32 tile (x 16..47, y 32..63), canopy lit from
      the upper-left, darker underside, a shadow blob at the base.
  shrub.png (3 variants in a row), berry_bush_bare.png, berry_bush_fruit.png, reeds.png (3),
  grain_0..3.png, root_0..3.png, wildflowers.png (3), mushrooms.png, stump.png, log.png
  {herbs,fern,reed,root_crops,vine,flowering_plant}_{0,1,2}.png — 3 growth stages each
      (the client picks by growth: < 0.35, < 0.8, grown).
  Season sheets (the client uses them when present, else the plain sheet):
      shrub_{autumn,winter}.png (3 variants), berry_bush_{bare,fruit}_{autumn,winter}.png,
      fern_{0,1,2}_{autumn,winter}.png (bracken russet, then dead and snowed on),
      reed_{0,1,2}_winter.png and reeds_winter.png (dry straw with snow on the heads),
      {herbs,vine,root_crops,flowering_plant}_{0,1,2}_winter.png (redrawn dormant: dry stems,
      seed heads, leaf stubs, snow on the tops), {vine,root_crops,flowering_plant}_{0,1,2}_autumn.png.
  Non-tree sheets are 32 px tall.
"""
import math
import os

from draw import Canvas, rng, sheet
from pal import Palette, ROOT

OUT = os.path.join(ROOT, 'godot', 'assets', 'plants')
pal = Palette.load()
P = pal.ramp
N = pal.neutral
BLACK = N('black')
SHADOW = N('shadow')


def ground_shadow(c, cx, cy, rx, ry, alpha=110):
    s = Canvas(c.w, c.h)
    s.disc(cx, cy, rx, ry, BLACK, alpha)
    for y in range(c.h):
        for x in range(c.w):
            if s.a[y, x, 3] and c.a[y, x, 3] == 0:
                c.px(x, y, BLACK, alpha)


def canopy_blob(c, cx, cy, rx, ry, ramp_steps, r, leaf_tufts=6):
    """A rounded leaf mass: dark underside disc, mid body, lit upper-left disc, then a few
    leaf-tuft stamps for texture and 1-px bites at the rim so it is not a perfect ellipse."""
    dark, mid, lit, hi = ramp_steps
    c.disc(cx + 1, cy + 2, rx, ry, dark)
    c.disc(cx, cy, rx - 0.5, ry - 1, mid)
    c.disc(cx - rx * 0.3, cy - ry * 0.35, rx * 0.55, ry * 0.5, lit)
    c.disc(cx - rx * 0.4, cy - ry * 0.5, rx * 0.25, ry * 0.25, hi)
    for _ in range(leaf_tufts):
        x = cx + r.randint(-int(rx) + 1, int(rx) - 1)
        y = cy + r.randint(-int(ry) + 1, int(ry) - 1)
        above = y < cy
        c.stamp(['.h', 'hd'] if above else ['d.', '.d'], x, y, {'h': lit if above else mid, 'd': dark})
    # Break up the broad mid-tone with a few authored leaf catches.  They are deliberately
    # sparse and seeded by the caller, so mature crowns gain depth without becoming noisy at 1x.
    for _ in range(max(2, leaf_tufts // 2)):
        x = cx + r.randint(-max(1, int(rx) - 2), max(1, int(rx) - 2))
        y = cy + r.randint(-max(1, int(ry) - 2), max(1, int(ry) - 2))
        if 0 <= x < c.w and 0 <= y < c.h and c.a[y, x, 3]:
            c.px(x, y, hi if y <= cy else lit)
    # rim bites
    for _ in range(int(rx)):
        ang = r.random() * 6.283
        x = int(round(cx + rx * math.cos(ang)))
        y = int(round(cy + ry * math.sin(ang)))
        if r.random() < 0.5 and 0 <= x < c.w and 0 <= y < c.h:
            c.a[y, x, 3] = 0


def trunk(c, x, y0, y1, w, lit, mid, dark):
    c.rect(x, y0, w, y1 - y0 + 1, mid)
    c.vline(x, y0, y1, lit)
    c.vline(x + w - 1, y0, y1, dark)
    # root flare
    c.px(x - 1, y1, dark)
    c.px(x + w, y1, dark)


OAK = (P('foliage', 1), P('foliage', 3), P('foliage', 5), P('foliage', 6))
BIRCH = (P('foliage', 2), P('foliage', 4), P('foliage', 6), P('foliage', 7))
PINE = (P('foliage', 0), P('foliage', 1), P('foliage', 3), P('foliage', 4))
BARK = (P('earth', 4), P('earth', 3), P('earth', 1))
BIRCH_BARK = (N('white'), P('stone', 6), P('stone', 3))
PINE_BARK = (P('earth', 3), P('earth', 2), P('earth', 0))
OAK_AUTUMN = (P('warm', 1), P('warm', 2), P('warm', 4), P('warm', 5))
BIRCH_AUTUMN = (P('earth', 4), P('warm', 5), P('warm', 6), P('warm', 7))
SNOW = (P('stone', 6), P('stone', 7))


def branch(c, x, y, ang, length, depth, r, bark, out):
    """One limb as a pixel line, splitting into two thinner limbs; tips are collected in `out`
    so the crown can be finished with twigs. Angle 0 is straight up."""
    x1 = x + math.sin(ang) * length
    y1 = y - math.cos(ang) * length
    col = bark[1] if depth >= 2 else bark[2]
    c.line(int(round(x)), int(round(y)), int(round(x1)), int(round(y1)), col)
    if depth >= 3:                                   # thick limbs: a lit upper-left edge
        c.line(int(round(x)) - 1, int(round(y)), int(round(x1)) - 1, int(round(y1)), bark[0])
    if depth == 0:
        out.append((int(round(x1)), int(round(y1))))
        return
    spread = 0.35 + r.random() * 0.25
    for sgn in (-1, 1):
        branch(c, x1, y1, ang + sgn * spread + (r.random() - 0.5) * 0.2,
               length * (0.62 + r.random() * 0.12), depth - 1, r, bark, out)


def bare_crown(c, cx, top, base, spread, r, bark, depth=4):
    """A winter crown: limbs from the trunk top fanning up and out, twig flecks at the tips,
    then snow lying on every upper edge (a pale pixel above each limb pixel open to the sky)."""
    tips = []
    length = (base - top) * 0.42
    for k, ang in enumerate((-spread, -spread * 0.35, spread * 0.35, spread)):
        branch(c, cx + (k - 1.5) * 0.6, base, ang + (r.random() - 0.5) * 0.15,
               length * (0.85 + r.random() * 0.3), depth, r, bark, tips)
    for (x, y) in tips:
        for _ in range(2):
            c.px(x + r.randint(-1, 1), y + r.randint(-1, 0), bark[2])
    snow = []
    for y in range(1, c.h):
        for x in range(c.w):
            if c.a[y, x, 3] and not c.a[y - 1, x, 3] and r.random() < 0.6:
                snow.append((x, y - 1))
    for (x, y) in snow:
        c.px(x, y, SNOW[1] if r.random() < 0.7 else SNOW[0])


def snow_on_tiers(c, r):
    """Snow on a pine: pale pixels along every upper edge of the foliage, heavier on the lit
    (left) side, so the tiers read as laden rather than tinted."""
    marks = []
    leaf = {tuple(int(v) for v in col[:3]) for col in PINE}
    for y in range(1, c.h - 6):
        for x in range(1, c.w - 1):
            if not c.a[y, x, 3] or tuple(int(v) for v in c.a[y, x, :3]) not in leaf:
                continue
            open_above = not c.a[y - 1, x, 3] or not c.a[y - 1, x - 1, 3] or not c.a[y - 1, x + 1, 3]
            if open_above:
                p = 0.95 if x < c.w // 2 else 0.75
                if r.random() < p:
                    marks.append((x, y))
    for (x, y) in marks:
        c.px(x, y, SNOW[1] if r.random() < 0.75 else SNOW[0])
        if y + 1 < c.h and c.a[y + 1, x, 3] and r.random() < 0.55:
            c.px(x, y + 1, SNOW[0])


def oak(stage, season=''):
    r = rng('oak', stage)
    leaves = OAK_AUTUMN if season == 'autumn' else OAK
    if season == 'winter':
        if stage == 0:
            c = Canvas(32, 32)
            ground_shadow(c, 16, 29, 5, 2)
            trunk(c, 15, 22, 29, 2, *BARK)
            bare_crown(c, 16, 9, 22, 0.8, r, BARK, 2)
        elif stage == 1:
            c = Canvas(32, 48)
            ground_shadow(c, 16, 44, 8, 3)
            trunk(c, 14, 26, 45, 4, *BARK)
            bare_crown(c, 16, 6, 27, 0.9, r, BARK, 3)
        else:
            c = Canvas(64, 64)
            ground_shadow(c, 32, 59, 14, 4)
            trunk(c, 29, 32, 60, 6, *BARK)
            c.px(28, 40, BARK[2])
            c.px(28, 41, BARK[2])
            bare_crown(c, 32, 4, 34, 1.0, r, BARK, 4)
        return c
    OAK_ = leaves
    if stage == 0:
        c = Canvas(32, 32)
        ground_shadow(c, 16, 29, 5, 2)
        trunk(c, 15, 20, 29, 2, *BARK)
        canopy_blob(c, 16, 16, 6, 5, OAK_, r, 3)
        canopy_blob(c, 13, 19, 3, 2.5, OAK_, r, 1)
    elif stage == 1:
        c = Canvas(32, 48)
        ground_shadow(c, 16, 44, 8, 3)
        trunk(c, 14, 26, 45, 4, *BARK)
        canopy_blob(c, 16, 18, 11, 9, OAK_, r, 6)
        canopy_blob(c, 10, 24, 6, 5, OAK_, r, 2)
        canopy_blob(c, 22, 23, 6, 5, OAK_, r, 2)
        canopy_blob(c, 15, 12, 7, 5, OAK_, r, 3)
    else:
        c = Canvas(64, 64)
        ground_shadow(c, 32, 59, 14, 4)
        trunk(c, 29, 34, 60, 6, *BARK)
        c.px(28, 40, BARK[2])                             # bark scar
        c.px(28, 41, BARK[2])
        c.line(31, 36, 24, 30, BARK[1])                   # a limb
        c.line(33, 36, 42, 29, BARK[1])
        canopy_blob(c, 32, 24, 24, 17, OAK_, r, 10)
        canopy_blob(c, 16, 30, 11, 8, OAK_, r, 4)
        canopy_blob(c, 46, 29, 11, 8, OAK_, r, 4)
        canopy_blob(c, 30, 12, 13, 8, OAK_, r, 5)
        canopy_blob(c, 40, 18, 9, 6, OAK_, r, 3)
    return c


def birch(stage, season=''):
    r = rng('birch', stage)
    BIRCH_ = BIRCH_AUTUMN if season == 'autumn' else BIRCH
    if season == 'winter':
        if stage == 0:
            c = Canvas(32, 32)
            ground_shadow(c, 16, 29, 4, 2)
            trunk(c, 15, 18, 29, 2, *BIRCH_BARK)
            bare_crown(c, 16, 6, 18, 0.55, r, (P('stone', 5), P('stone', 4), P('stone', 2)), 2)
        elif stage == 1:
            c = Canvas(32, 48)
            ground_shadow(c, 16, 44, 6, 3)
            trunk(c, 15, 20, 45, 3, *BIRCH_BARK)
            for y in (26, 33, 40):
                c.px(16, y, BIRCH_BARK[2])
            bare_crown(c, 16, 3, 21, 0.6, r, (P('stone', 5), P('stone', 4), P('stone', 2)), 3)
        else:
            c = Canvas(64, 64)
            ground_shadow(c, 32, 59, 10, 4)
            trunk(c, 30, 24, 60, 4, *BIRCH_BARK)
            for y in (30, 37, 45, 52):
                c.px(31, y, BIRCH_BARK[2])
                c.px(32, y + 1, BIRCH_BARK[2])
            bare_crown(c, 32, 2, 26, 0.65, r, (P('stone', 5), P('stone', 4), P('stone', 2)), 4)
        return c
    if stage == 0:
        c = Canvas(32, 32)
        ground_shadow(c, 16, 29, 4, 2)
        trunk(c, 15, 16, 29, 2, *BIRCH_BARK)
        canopy_blob(c, 16, 12, 5, 6, BIRCH_, r, 3)
    elif stage == 1:
        c = Canvas(32, 48)
        ground_shadow(c, 16, 44, 6, 3)
        trunk(c, 15, 20, 45, 3, *BIRCH_BARK)
        for y in (26, 33, 40):                             # bark dashes
            c.px(16, y, BIRCH_BARK[2])
        canopy_blob(c, 16, 13, 9, 11, BIRCH_, r, 6)
        canopy_blob(c, 12, 20, 5, 5, BIRCH_, r, 2)
    else:
        c = Canvas(64, 64)
        ground_shadow(c, 32, 59, 10, 4)
        trunk(c, 30, 24, 60, 4, *BIRCH_BARK)
        for y in (30, 37, 45, 52):
            c.px(31, y, BIRCH_BARK[2])
            c.px(32, y + 1, BIRCH_BARK[2])
        c.line(31, 28, 22, 18, BIRCH_BARK[1])
        c.line(33, 28, 42, 20, BIRCH_BARK[1])
        canopy_blob(c, 32, 18, 17, 17, BIRCH_, r, 10)
        canopy_blob(c, 20, 26, 8, 7, BIRCH_, r, 3)
        canopy_blob(c, 44, 24, 8, 7, BIRCH_, r, 3)
        canopy_blob(c, 30, 8, 9, 6, BIRCH_, r, 3)
        if not season:
            # Keep two sunlit leaf catches on the outer crown after the blobs overlap.
            c.px(49, 20, BIRCH_[3])
            c.px(50, 24, BIRCH_[3])
    return c


def pine_tier(c, cx, y, half_w, h, r, steps):
    """A triangular tier: dark right side, lit left, ragged bottom edge."""
    dark, mid, lit, hi = steps
    for k in range(h):
        w = int(round(half_w * (k + 1) / h))
        c.rect(cx - w, y + k, w, 1, mid)
        c.rect(cx, y + k, w + 1, 1, dark)
        if k < h * 0.6:
            c.rect(cx - w, y + k, max(1, w // 2), 1, lit)
    for x in range(cx - half_w, cx + half_w + 1, 2):   # ragged bottom
        c.px(x + r.randint(0, 1), y + h, dark)
    c.px(cx - 1, y, hi)


def pine(stage, season=''):
    c = _pine(stage)
    if season == 'winter':
        snow_on_tiers(c, rng('pine-snow', stage))
    return c


def _pine(stage):
    r = rng('pine', stage)
    if stage == 0:
        c = Canvas(32, 32)
        ground_shadow(c, 16, 29, 4, 2)
        trunk(c, 15, 24, 29, 2, *PINE_BARK)
        pine_tier(c, 16, 10, 6, 14, r, PINE)
        pine_tier(c, 16, 6, 3, 6, r, PINE)
    elif stage == 1:
        c = Canvas(32, 48)
        ground_shadow(c, 16, 44, 6, 3)
        trunk(c, 15, 30, 45, 3, *PINE_BARK)
        pine_tier(c, 16, 22, 11, 11, r, PINE)
        pine_tier(c, 16, 13, 8, 11, r, PINE)
        pine_tier(c, 16, 4, 5, 11, r, PINE)
    else:
        c = Canvas(64, 64)
        ground_shadow(c, 32, 59, 10, 4)
        trunk(c, 30, 40, 60, 5, *PINE_BARK)
        pine_tier(c, 32, 38, 20, 14, r, PINE)
        pine_tier(c, 32, 26, 16, 14, r, PINE)
        pine_tier(c, 32, 14, 12, 14, r, PINE)
        pine_tier(c, 32, 4, 7, 12, r, PINE)
        c.px(32, 2, PINE[3])
        c.px(32, 3, PINE[2])
    return c


# ---------------------------------------------------------------- seasons for low plants
def _key(col):
    return tuple(int(v) for v in col[:3])


# Autumn: each green step goes to a turning-leaf colour of similar value (darkest to lightest).
AUTUMN_RUSSET = {_key(P('foliage', k)): c for k, c in enumerate(
    (P('earth', 0), P('warm', 1), P('warm', 1), P('warm', 2), P('warm', 3), P('warm', 4), P('warm', 5),
     P('warm', 6)))}
AUTUMN_GOLD = {_key(P('foliage', k)): c for k, c in enumerate(
    (P('earth', 1), P('earth', 2), P('earth', 3), P('earth', 4), P('earth', 5), P('warm', 5), P('earth', 6),
     P('earth', 7)))}
# Dead and dry: browns a step or two darker than the gold.
DEAD = {_key(P('foliage', k)): c for k, c in enumerate(
    (P('earth', 0), P('earth', 1), P('earth', 2), P('earth', 2), P('earth', 3), P('earth', 3), P('earth', 4),
     P('earth', 5)))}


def recolour(c, mapping):
    out = Canvas(c.w, c.h)
    out.a[:] = c.a
    for y in range(c.h):
        for x in range(c.w):
            if out.a[y, x, 3]:
                k = _key(out.a[y, x])
                if k in mapping:
                    out.a[y, x, :3] = mapping[k]
    return out


def snow_on_tops(c, r, chance=0.7, skip=()):
    """Snow along every upper edge open to the sky: a pale pixel on the topmost pixel of each
    run, so a plant reads as snowed on rather than tinted."""
    skip = {_key(k) for k in skip}
    marks = []
    for y in range(1, c.h - 3):
        for x in range(c.w):
            if c.a[y, x, 3] and not c.a[y - 1, x, 3] and _key(c.a[y, x]) not in skip and r.random() < chance:
                marks.append((x, y))
    for (x, y) in marks:
        c.px(x, y, SNOW[1] if r.random() < 0.75 else SNOW[0])


def bare_bush(r, bark, spread=1.0, twigs=7):
    """A winter bush: a fan of thin twigs from the base, a few forked, then snow on the tops."""
    c = Canvas(32, 32)
    ground_shadow(c, 16, 27, 9, 2)
    tips = []
    for k in range(twigs):
        ang = (k - (twigs - 1) / 2) / ((twigs - 1) / 2) * 0.95 * spread + (r.random() - 0.5) * 0.2
        branch(c, 16 + (k - twigs // 2) * 0.7, 26, ang, 6 + r.random() * 4, 2, r, bark, tips)
    for (x, y) in tips:
        c.px(x + r.randint(-1, 1), y - 1, bark[2])
    c.disc(16, 26, 4, 1.2, bark[1])
    return c


# ---------------------------------------------------------------- low plants
def shrub(v, season=''):
    if season == 'winter':
        c = bare_bush(rng('shrub-winter', v), BARK, 0.8 + v * 0.15, 6 + v)
        snow_on_tops(c, rng('shrub-snow', v))
        return c
    c = _shrub(v)
    if season == 'autumn':
        c = recolour(c, AUTUMN_GOLD if v == 1 else AUTUMN_RUSSET)
    return c


def _shrub(v):
    r = rng('shrub', v)
    c = Canvas(32, 32)
    ground_shadow(c, 16, 27, 9, 2)
    canopy_blob(c, 16, 19, 10 - v, 7, OAK, r, 5)
    canopy_blob(c, 11 + v * 2, 16, 5, 4, OAK, r, 2)
    canopy_blob(c, 21 - v, 15 + v, 5, 4, OAK, r, 2)
    c.px(14, 26, BARK[1])
    c.px(17, 26, BARK[1])
    # Three shrub silhouettes share a family but carry different field cues: a low berry-like
    # glint, a flowering sprig, and a cool alien-tinged leaf edge.  These remain readable as
    # vegetation at normal zoom and make mixed biomes less copy-pasted.
    if v == 0:
        for x, y in ((10, 20), (22, 18), (16, 14)):
            c.px(x, y, P('warm', 4))
    elif v == 1:
        for x, y in ((9, 17), (24, 16), (15, 12), (19, 21)):
            c.px(x, y, P('warm', 6))
            c.px(x + 1, y, P('warm', 4))
    else:
        for x, y in ((9, 18), (23, 17), (15, 13), (19, 20)):
            c.px(x, y, P('cool', 5))
    return c


def berry_bush(fruit, season=''):
    """Autumn turns the leaves russet with the berries still on; winter drops them to bare,
    snowed twigs that keep any fruit not yet picked."""
    if season == 'winter':
        c = bare_bush(rng('berry-winter', fruit), BARK, 0.9, 8)
        if fruit:
            for x, y in ((10, 17), (14, 13), (19, 12), (22, 18), (16, 20)):
                c.stamp(['bB'], x, y, {'b': P('warm', 5), 'B': P('warm', 3)})
        snow_on_tops(c, rng('berry-snow', fruit), skip=(P('warm', 5), P('warm', 3)))
        return c
    c = _berry_bush(fruit)
    if season == 'autumn':
        # gold rather than russet leaves, so the red berries still stand out
        c = recolour(c, AUTUMN_GOLD)
    return c


def _berry_bush(fruit):
    r = rng('berry', fruit)
    c = Canvas(32, 32)
    ground_shadow(c, 16, 27, 9, 2)
    steps = (P('foliage', 1), P('foliage', 2), P('foliage', 4), P('foliage', 5))
    canopy_blob(c, 16, 18, 10, 8, steps, r, 6)
    canopy_blob(c, 12, 14, 5, 4, steps, r, 2)
    canopy_blob(c, 21, 14, 5, 4, steps, r, 2)
    for x, y in ((9, 15), (13, 11), (18, 10), (22, 16), (15, 20), (20, 21), (11, 21)):
        if fruit:
            c.stamp(['bB', 'BB'], x, y, {'b': P('warm', 5), 'B': P('warm', 3)})
        else:
            c.px(x, y, P('earth', 2))                      # bare twig tips
            c.px(x + 1, y - 1, P('earth', 3))
    c.px(14, 26, BARK[2])
    c.px(18, 26, BARK[2])
    return c


def reeds(v):
    r = rng('reeds', v)
    c = Canvas(32, 32)
    c.disc(16, 29, 8, 1.5, P('earth', 1), 140)
    for i in range(6 + v):
        x = 6 + r.randint(0, 20)
        h = r.randint(12, 24)
        col = P('foliage', 2) if i % 3 == 0 else (P('foliage', 3) if i % 2 else P('foliage', 4))
        dx = r.choice((-2, -1, 1, 2))
        c.line(x, 29, x + dx, 29 - h, col)
        c.line(x + 1, 29, x + 1 + dx, 29 - h + 3, P('foliage', 2))
        if h > 16:
            c.line(x, 25, x - dx, 25 - h // 3, P('foliage', 5))
        c.px(x + dx, 29 - h, P('foliage', 5))
        if r.random() < 0.4:                                # a seed head
            sx = x + r.choice((-2, 1))
            c.rect(sx, 29 - h + 1, 2, 4, P('earth', 3))
            c.px(sx + 1, 29 - h, P('earth', 5))
    # A dark, broken waterline anchors the stand against shallow water or wet soil.
    for x in range(7, 27, 4):
        c.px(x + r.randint(-1, 1), 29, P('olive', 2))
    return c


def grain(stage):
    """Four rows of stalks running east-west. 0: seedlings, 1: green blades, 2: green ears,
    3: golden ripe ears."""
    r = rng('grain', stage)
    c = Canvas(32, 32)
    n = 6 + stage * 2
    for i in range(n):
        x = 2 + (i * 28) // n + r.randint(0, 1)
        h = 7 + stage * 6 + r.randint(0, 2)
        green = P('foliage', 4) if i % 2 else P('foliage', 3)
        col = green if stage < 3 else (P('earth', 6) if i % 2 else P('earth', 5))
        c.line(x, 30, x + r.choice((-1, 0, 1)), 30 - h, col)
        c.px(x, 30 - h, P('foliage', 5) if stage < 3 else P('earth', 7))
        if stage == 0:                                      # seedling: two small leaves
            c.px(x - 1, 30 - h + 2, P('foliage', 5))
            c.px(x + 1, 30 - h + 3, P('foliage', 5))
        if stage == 1:                                      # blade: a side leaf
            c.line(x, 30 - h + 4, x + 2, 30 - h + 1, P('foliage', 5))
        if stage >= 2:                                      # ear: 3 px wide, 5 tall, with awns
            ear = (P('foliage', 5), P('foliage', 4)) if stage == 2 else (P('earth', 7), P('earth', 6))
            ex = x - 1 + r.choice((0, 1))
            for k in range(5):
                c.px(ex + (k % 2), 30 - h - 4 + k, ear[0])
                c.px(ex + 1 + (k % 2), 30 - h - 4 + k, ear[1])
            c.px(ex + 1, 30 - h - 5, ear[0])
    c.rect(0, 30, 32, 2, P('earth', 2), 90)                 # the furrow line
    return c


def root(stage):
    """Root crop in four plants: leaf rosettes that grow from 3 px to 9 px, and from stage 2 the
    orange crown of the root shows above the soil."""
    r = rng('root', stage)
    c = Canvas(32, 32)
    for (x, y) in ((7, 9), (21, 7), (9, 23), (23, 21)):
        x += r.randint(-1, 1)
        if stage == 0:
            c.stamp(['.l.', 'lLl', '.l.'], x - 1, y - 1, {'l': P('foliage', 3), 'L': P('foliage', 5)})
            continue
        leaves = 4 + stage * 2
        for k in range(leaves):
            ang = k * 6.283 / leaves + r.random() * 0.4
            ln = 2 + stage
            lx, ly = x + int(round(math.cos(ang) * ln)), y + int(round(math.sin(ang) * ln * 0.7))
            c.line(x, y, lx, ly, P('foliage', 3))
            c.disc(lx, ly, 1.2, 1, P('foliage', 4))
            c.px(lx - 1, ly - 1, P('foliage', 5))
        c.disc(x, y, 1.5, 1.2, P('foliage', 2))             # dark centre
        if stage >= 2:
            c.stamp(['.rr.', 'rRRr'], x - 2, y + 2, {'r': P('warm', 5), 'R': P('warm', 4)})
            c.px(x - 1, y + 2, P('warm', 6))
        if stage == 3:
            c.stamp(['rRRr'], x - 2, y + 4, {'r': P('warm', 4), 'R': P('warm', 3)})
    return c


def wildflowers(v):
    r = rng('flowers', v)
    c = Canvas(32, 32)
    petal = [P('warm', 6), P('cool', 6), N('white')][v]
    centre = [P('warm', 7), P('warm', 7), P('earth', 6)][v]
    for (x, y) in ((6, 10), (16, 7), (25, 12), (10, 20), (21, 22), (15, 15)):
        x += r.randint(-1, 1)
        c.line(x, y + 2, x + r.choice((-1, 0, 1)), y + 8, P('foliage', 3))
        c.px(x + 2, y + 6, P('foliage', 4))
        c.stamp(['.p.', 'pcp', '.p.'], x - 1, y - 1, {'p': petal, 'c': centre})
    return c


def alien_bloom(v, season=''):
    """A low alien flower with a readable three-stage silhouette.

    The plant is intentionally built from a familiar stem-and-rosette grammar, then diverges
    with asymmetrical petals, a cool inner glow, and one trailing tendril.  Autumn closes the
    crown around warm seed pods; winter leaves a low, luminous husk instead of pretending the
    organism is an ordinary deciduous plant.
    """
    r = rng('alien-bloom', v, season)
    c = Canvas(32, 32)
    ground_shadow(c, 16, 28, 6 + v, 2)
    stem = P('cool', 2) if season != 'winter' else P('stone', 4)
    glow = P('gem', 6) if season != 'winter' else P('cool', 6)
    petal = (P('cool', 2), P('cool', 4), P('gem', 4), P('gem', 6))[min(3, v)]
    if season == 'autumn':
        petal = P('warm', 4 + min(2, v))
    if season == 'winter':
        c.line(16, 27, 16 + (v - 1), 19, stem)
        c.disc(16 + (v - 1), 18, 3 + v, 2 + v // 2, P('stone', 3))
        for x, y in ((14, 17), (18, 17), (16, 15)):
            c.px(x, y, glow)
        c.px(16, 14, P('gem', 5))
        c.line(16, 20, 11, 24, stem)
        c.line(16, 20, 21, 23, stem)
        return c
    base = 27 - v
    c.line(16, base, 16 + r.choice((-1, 0, 1)), 18 - v, stem)
    c.line(16, 25, 11 - v, 22 - v, P('foliage', 3))
    c.line(16, 25, 21 + v, 22 - v, P('foliage', 4))
    cx, cy = 16 + r.choice((-1, 0, 1)), 17 - v
    petals = 4 + v
    for i in range(petals):
        ang = i * 6.283 / petals + r.random() * 0.35
        length = 4 + v + (1 if i % 3 == 0 else 0)
        tx = int(round(cx + math.cos(ang) * length))
        ty = int(round(cy + math.sin(ang) * length * 0.65))
        c.line(cx, cy, tx, ty, petal)
        c.px(tx, ty, P('gem', 5) if i % 2 else P('cool', 6))
    c.disc(cx, cy, 2 + v * 0.4, 1.5, glow)
    c.px(cx, cy - 1, P('gem', 7))
    # The trailing tendril is the silhouette cue that separates it from a normal flower patch.
    c.line(cx - 1, cy + 2, cx - 4 - v, cy + 6, P('cool', 3))
    c.px(cx - 5 - v, cy + 6, glow)
    if season == 'autumn':
        c.px(cx + 4, cy + 3, P('warm', 7))
        c.px(cx + 5, cy + 4, P('warm', 5))
    return c


def mushrooms():
    c = Canvas(32, 32)
    for (x, y, w) in ((8, 20, 5), (16, 14, 7), (23, 22, 4), (13, 25, 3)):
        c.rect(x + w // 2 - 1, y, 2, 5, P('stone', 6))    # stalk
        c.px(x + w // 2, y + 4, P('stone', 4))
        c.disc(x + w // 2, y - 1, w / 2.0, 1.5, P('warm', 2))   # cap
        c.hline(x, x + w - 1, y - 2, P('warm', 3))
        c.px(x + 1, y - 2, P('warm', 5))
        if w > 4:
            c.px(x + w // 2, y - 3, P('warm', 5))
        c.px(x + w - 1, y, P('cool', 5))                     # faint cave glow fleck
    c.outline(BLACK)
    return c


def bone_tree(v):
    """The bone forest's tree (C59): a white trunk and crooked white limbs carrying pale,
    hanging fronds of bone-coloured leaf with grey shadow, so it reads as a living tree of
    a strange kind, not a dead one (grey-brown and bare) and not a birch (green leaves). Three
    variants differ in height, lean and how the crown hangs."""
    r = rng('bone-tree', v)
    c = Canvas(32, 48)
    base = 45
    ground_shadow(c, 16, base + 1, 9, 2)
    h = 15 + v * 3
    lean = (-1, 0, 1)[v % 3]
    for k in range(h):                                   # the trunk, lit west, shaded east
        y = base - k
        x = 14 + (lean * k) // 10
        wdt = 4 if k < h - 5 else 3
        c.rect(x, y, wdt, 1, P('stone', 6))
        c.px(x, y, P('stone', 7))
        c.px(x + wdt - 1, y, P('stone', 4))
    for _ in range(2):
        c.px(15, base - r.randint(3, h - 4), P('stone', 3))  # knots
    top_x, top_y = 15 + (lean * h) // 10, base - h
    limbs = [(-8, -4), (8, -4), (0, -9), (-5, -11), (5, -10)]
    for dx, dy in limbs:                                 # crooked limbs
        c.line(top_x + 1, top_y + 2, top_x + 1 + dx, top_y + dy, P('stone', 6))
        c.line(top_x + 2, top_y + 2, top_x + 2 + dx, top_y + dy, P('stone', 4))
    # The crown: a pale clump of fronds at each limb's end, a cool shadow underneath, white on
    # top, with the white limbs showing between. #201 (QA 2026-10-20): three small grey clumps
    # (a third of an oak's crown) read at 1x as a dead wood; the crown is now as wide as an
    # oak's and white, cream and lavender on its shadow side.
    for k, (dx, dy) in enumerate(limbs):
        cx, cy = top_x + 1 + dx, top_y + dy
        rx, ry = (5, 3) if k < 2 else (4, 3)
        c.disc(cx, cy + 1, rx, ry, P('stone', 6))
        c.hline(cx - rx + 2, cx + rx - 2, cy + ry, P('cool', 6))
        c.disc(cx, cy, rx - 1, ry - 1, P('stone', 7))
        c.disc(cx - 1, cy - 1, rx - 2, ry - 2, N('white'))
        for _ in range(2):                               # fronds hanging below the clump
            fx = cx + r.randint(-rx + 2, rx - 2)
            c.vline(fx, cy + ry, cy + ry + r.randint(1, 3), P('stone', 6))
        # Cool seams make the bone canopy read as living tissue rather than a pale dead tree.
        c.px(cx - max(1, rx // 2), cy, P('cool', 5))
        c.px(cx + max(1, rx // 2), cy + 1, P('cool', 4))
    return c


def fungus_tree(v):
    """A pale fungus as tall as a sapling, in the cavern forests: a soft stalk with a ring, a
    broad cap whose rim droops over dark gills, and glowing spots. Three variants differ in
    height, lean and cap; the middle one is amber rather than the cool cave colour."""
    r = rng('fungus-tree', v)
    c = Canvas(32, 48)
    h = 21 + v * 4                       # stalk height
    lean = (-1, 0, 1)[v % 3]
    base = 44
    ground_shadow(c, 16, base + 1, 8, 2)
    if v == 1:
        cap = (P('warm', 2), P('warm', 3), P('warm', 4), P('warm', 6))
    else:
        cap = (P('cool', 2), P('cool', 3), P('cool', 4), P('cool', 6))
    ring_at = h * 2 // 3
    for k in range(h):
        y = base - k
        x = 15 + (lean * k) // 12
        wdt = 5 if k < 3 else 4
        c.rect(x - 2, y, wdt, 1, P('stone', 6))
        c.px(x - 2, y, P('stone', 4))                 # shaded side
        c.px(x + wdt - 3, y, P('stone', 7))           # lit side
        if k == ring_at:                              # the ring, a skirt under the cap
            c.hline(x - 3, x + 2, y, P('stone', 5))
            c.hline(x - 2, x + 1, y + 1, P('stone', 3))
    top_x = 16 + (lean * h) // 12
    top_y = base - h
    rx = 10 + (v % 2) * 2
    c.disc(top_x, top_y + 3, rx, 2.5, cap[0])          # drooping rim
    for gx in range(top_x - rx + 2, top_x + rx - 1, 2):    # gills under the rim
        c.px(gx, top_y + 3, P('cool', 1))
    c.disc(top_x, top_y, rx, 5, cap[1])
    c.disc(top_x, top_y - 1, rx - 1, 4, cap[2])
    c.disc(top_x - 2, top_y - 3, rx - 5, 1.5, cap[3])  # lit crown
    # Glowing spots: a bright core with a softer ring.
    for _ in range(3 + v):
        sx = top_x + r.randint(-rx + 3, rx - 3)
        sy = top_y + r.randint(-2, 2)
        c.px(sx, sy, P('cool', 7))
        c.px(sx + 1, sy, P('cool', 7))
        c.px(sx, sy + 1, P('cool', 5))
        c.px(sx + 1, sy + 1, P('cool', 5))
    # Radial cap veins and hanging gill droplets add a second material layer at 1x.
    for off in range(-rx + 3, rx - 2, max(2, rx // 3)):
        c.line(top_x, top_y + 1, top_x + off, top_y + 3, cap[2])
    for off in range(-rx + 3, rx - 1, 3):
        c.px(top_x + off, top_y + 5, P('cool', 3))
    if v != 0:                                    # a small cap low on the stalk
        sx = 15 + (lean * (h // 3)) // 12 + 4
        sy = base - h // 3
        c.disc(sx + 1, sy, 3, 1.5, cap[1])
        c.px(sx + 1, sy - 1, cap[3])
        c.px(sx, sy + 1, P('cool', 1))
    c.outline(BLACK)
    return c


def stump():
    c = Canvas(32, 32)
    ground_shadow(c, 16, 27, 8, 2)
    c.disc(16, 22, 8, 4, P('earth', 2))                     # bark base
    c.disc(16, 19, 8, 4, P('earth', 3))
    c.disc(16, 17, 7, 3, P('earth', 6))                     # cut face
    c.disc(16, 17, 5, 2, P('earth', 5))                     # growth rings
    c.disc(16, 17, 3, 1, P('earth', 6))
    c.px(16, 17, P('earth', 4))
    c.hline(10, 22, 20, P('earth', 2))
    c.px(7, 24, P('earth', 1))                              # roots
    c.px(25, 24, P('earth', 1))
    c.outline(P('earth', 0))
    return c


def log():
    c = Canvas(32, 32)
    ground_shadow(c, 16, 24, 12, 2)
    c.rect(4, 14, 24, 8, P('earth', 3))
    c.hline(4, 27, 14, P('earth', 5))
    c.hline(4, 27, 21, P('earth', 1))
    for x in (9, 15, 21):                                   # bark grooves
        c.vline(x, 15, 20, P('earth', 2))
    c.disc(27, 18, 2.5, 4, P('earth', 6))                   # cut end (east)
    c.px(27, 18, P('earth', 4))
    c.disc(4, 18, 2, 4, P('earth', 2))
    c.px(14, 12, P('earth', 2))                             # stub branch
    c.px(14, 13, P('earth', 3))
    c.outline(P('earth', 0))
    return c


def herbs(v):
    """A small herb plant: leafy bush in stages (basil/thyme/sage)."""
    r = rng('herbs', v)
    c = Canvas(32, 32)
    ground_shadow(c, 16, 27, 7, 2)
    steps = (P('foliage', 1), P('foliage', 3), P('foliage', 5), P('foliage', 6))
    canopy_blob(c, 16, 18, 7 - v, 5 + v, steps, r, 4)
    canopy_blob(c, 11 + v * 2, 15, 4 + v, 3 + v, steps, r, 2)
    canopy_blob(c, 21 - v, 16, 4 + v, 3, steps, r, 2)
    for _ in range(2 + v):
        x, y = r.randint(10, 22), r.randint(10, 20)
        c.px(x, y, P('foliage', 6))
    return c


def thistle(v):
    """A thistle in three stages: a flat rosette of spiny leaves, then a stalk and one purple
    head, then full and spined all over (#286). It reads apart from the herbs by the dark
    jagged rosette and the violet flower."""
    r = rng('thistle', v)
    c = Canvas(32, 32)
    ground_shadow(c, 16, 27, 6 + v, 2)
    steps = (P('olive', 0), P('olive', 1), P('foliage', 1), P('olive', 2))
    # the rosette lies flat to the ground, spiny-edged (few big lobes, no soft tufts)
    canopy_blob(c, 16, 22, 8 - v, 2 + v // 2, steps, r, 3)
    canopy_blob(c, 11 + v, 20, 3 + v, 2, steps, r, 2)
    canopy_blob(c, 21 - v, 20, 3 + v, 2, steps, r, 2)
    if v >= 1:
        # the flower stalk rises out of the rosette, a violet head with pale spines
        for (sx, h) in ((15, 8 + v * 2), (19, 6 + v)):
            c.line(sx, 21, sx, 21 - h, P('olive', 1))
            hy = 21 - h
            c.disc(sx, hy, 1.6 + 0.4 * v, 2.0, P('cool', 2))
            c.px(sx, hy - 2, P('cool', 3))
            c.px(sx - 1, hy + 1, P('gem', 3))
            c.px(sx + 1, hy + 1, P('gem', 3))
    return c


def reed(v):
    """A reed stand in three stages, the same drawing as reeds.png: 0 a few young shoots,
    1 a half-grown stand, 2 a full stand with brown seed heads."""
    r = rng('reed', v)
    c = Canvas(32, 32)
    c.disc(16, 29, 5 + v * 2, 1.5, P('earth', 1), 140)
    for i in range(4 + v * 3):
        x = 7 + r.randint(0, 18)
        h = [r.randint(6, 10), r.randint(11, 17), r.randint(15, 24)][v]
        col = P('foliage', 3) if i % 2 else P('foliage', 4)
        dx = r.choice((-1, 1)) * (1 + v // 2)
        c.line(x, 29, x + dx, 29 - h, col)
        c.line(x + 1, 29, x + 1 + dx, 29 - h + 3, P('foliage', 2))
        c.px(x + dx, 29 - h, P('foliage', 5))
        if v == 2 and r.random() < 0.45:                    # a seed head
            c.rect(x + dx + r.choice((-1, 0)), 29 - h + 1, 2, 4, P('earth', 3))
    return c


def vine(v):
    """Climbing vine with leaves. 3 stages: sparse vines → full coverage."""
    r = rng('vine', v)
    c = Canvas(32, 32)
    n = 3 + v * 2
    for i in range(n):
        x = r.randint(4, 28)
        y = 30 - r.randint(8 + v * 2, 18 + v * 3)
        dx = r.choice((-2, -1, 1, 2))
        c.line(x, 30, x + dx, y, P('foliage', 3))
        if v >= 1:
            c.px(x + r.choice((-1, 1)), y + 2, P('foliage', 4))
    for _ in range(3 + v * 2):
        x = r.randint(4, 28)
        y = 14 + v * 2 + r.randint(0, 10)
        c.px(x, y, P('foliage', 5))
        c.px(x + 1, y, P('foliage', 4))
        c.px(x - 1, y, P('foliage', 4))
    c.outline(P('foliage', 3))
    return c


def root_crops(v):
    """Wild turnips in three stages: leaf rosettes that fill out, and from stage 1 the pale
    purple-topped crown of each root showing at the soil."""
    r = rng('root_crops', v)
    c = Canvas(32, 32)
    for (x, y) in ((8, 10), (22, 9), (11, 23), (23, 22)):
        x += r.randint(-1, 1)
        c.disc(x, y + 3, 3, 1, P('earth', 2), 160)          # the soil it pushes up
        leaves = 4 + v * 2
        for k in range(leaves):
            ang = k * 6.283 / leaves + r.random() * 0.4
            ln = 2 + v * 1.5
            lx, ly = x + int(round(math.cos(ang) * ln)), y - 1 + int(round(math.sin(ang) * ln * 0.6))
            c.line(x, y, lx, ly, P('foliage', 3))
            c.disc(lx, ly, 1.0 + v * 0.3, 1, P('foliage', 4))
            c.px(lx - 1, ly - 1, P('foliage', 5))
        if v >= 1:
            c.stamp(['.ww.', 'cwwc'], x - 2, y + 1, {'w': N('white'), 'c': P('cool', 5)})
        if v == 2:
            c.stamp(['cc'], x - 1, y + 3, {'c': P('cool', 4)})
    return c


def flowering_plant(v):
    """A clump of leafy stems: 0 leaves only, 1 green buds, 2 open purple flowers."""
    r = rng('flowering', v)
    c = Canvas(32, 32)
    ground_shadow(c, 16, 28, 6, 1.5)
    for k in range(3 + v):
        sx = 16 + (k - (2 + v) / 2.0) * 3 + r.randint(-1, 1)
        h = 7 + v * 4 + r.randint(0, 3)
        tx = sx + r.choice((-1, 0, 1))
        c.line(int(sx), 28, int(tx), 28 - h, P('foliage', 3))
        for j, ly in enumerate((25, 21, 17)):                 # leaves, alternate sides
            if 28 - h < ly - 1:
                side = 1 if (j + k) % 2 else -1
                x0 = int(sx) + side
                c.px(x0, ly, P('foliage', 4))
                c.px(x0 + side, ly - 1, P('foliage', 5))
                c.px(x0 + side * 2, ly - 1, P('foliage', 4))
        top = (int(tx), 28 - h)
        if v == 1:
            c.stamp(['b', 'g'], top[0], top[1] - 1, {'b': P('foliage', 6), 'g': P('foliage', 4)})
        elif v == 2:
            c.stamp(['.p.', 'pcp', '.p.'], top[0] - 1, top[1] - 2,
                    {'p': P('cool', 6), 'c': P('warm', 7)})
    return c


# ---------------------------------------------------------------- dormant low plants
# Winter sheets for the small food and flower plants: each is redrawn as the dead or dormant
# plant (dry stems, seed heads, leaf stubs), not recoloured, then snowed on like the shrubs.
STRAW = (P('earth', 5), P('earth', 4), P('earth', 3))       # lit, mid, shade
DRY_LEAF = (P('earth', 3), P('earth', 2))


def _stem(c, x, y, ang, length, lit, shade):
    """A dry stem from (x, y) at angle `ang` (0 straight up); returns the tip. Stems leaning
    left catch the upper-left light, so they take the lit colour."""
    x1 = x + math.sin(ang) * length
    y1 = y - math.cos(ang) * length
    c.line(int(round(x)), int(round(y)), int(round(x1)), int(round(y1)), lit if ang < 0.1 else shade)
    return int(round(x1)), int(round(y1))


def herbs_dormant(v):
    """Herbs in winter: the leaves fallen to a dark mat at the crown, a fan of woody stems,
    and from stage 1 the dry seed spikes left at their tips (thyme and sage gone over)."""
    r = rng('herbs-winter', v)
    c = Canvas(32, 32)
    ground_shadow(c, 16, 27, 6 + v, 2)
    c.disc(16, 25, 3 + v, 1.4, DRY_LEAF[1])
    c.px(14, 24, DRY_LEAF[0])
    c.px(16, 24, DRY_LEAF[0])
    n = 5 + v * 2
    for k in range(n):
        ang = (k - (n - 1) / 2) / ((n - 1) / 2) * 0.85 + (r.random() - 0.5) * 0.2
        ln = 5 + v * 2 + r.randint(0, 3)
        tx, ty = _stem(c, 16 + (k - n // 2) * 0.6, 25, ang, ln, STRAW[1], STRAW[2])
        if r.random() < 0.4:                                # a side twig halfway up
            mx = int(round(16 + math.sin(ang) * ln * 0.5))
            my = int(round(25 - math.cos(ang) * ln * 0.5))
            side = -1 if ang < 0 else 1
            c.px(mx + side, my - 1, STRAW[2])
        if v >= 1 and r.random() < 0.55 + v * 0.15:          # a seed spike: alternate whorls
            for j in range(1 + v):
                c.px(tx, ty - j, STRAW[0] if j % 2 == 0 else DRY_LEAF[1])
                c.px(tx - 1, ty - j, STRAW[0] if j % 2 else DRY_LEAF[0])
    snow_on_tops(c, rng('herbs-snow', v), 0.45, skip=(DRY_LEAF[1],))
    return c


def vine_dormant(v):
    """Vine in winter: the bare woody strands where the leaves were, crooked and thin, curled
    tendrils at the tips, a few dead leaves still hanging, and on a grown vine clusters of
    dark berries."""
    r = rng('vine-winter', v)
    c = Canvas(32, 32)
    n = 3 + v * 2
    for i in range(n):
        x = r.randint(4, 28)
        top = 30 - r.randint(8 + v * 2, 18 + v * 3)
        # three crooked segments, so it reads as a woody strand rather than a post
        pts = [(x, 30)]
        for k in (1, 2, 3):
            pts.append((pts[-1][0] + r.choice((-1, 0, 1)) + (1 if i % 2 else -1) * (k == 2),
                        30 - (30 - top) * k // 3))
        for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
            c.line(x0, y0, x1, y1, STRAW[1] if i % 2 else STRAW[2])
        bx, by = pts[1]
        c.px(bx + 1, by, P('earth', 1))                      # shade on the lower right of the stem
        c.px(bx + 1, by + 1, P('earth', 1))
        tx, ty = pts[-1]
        curl = r.choice((-1, 1))
        c.stamp(['ab', '.a'] if curl > 0 else ['ba', 'a.'], tx - (0 if curl > 0 else 1), ty - 2,
                {'a': STRAW[2], 'b': STRAW[0]})             # a curled tendril at the tip
        if r.random() < 0.6:                                 # a dead leaf hanging off it
            lx, ly = pts[2]
            lx += r.choice((-2, 1))
            c.stamp(['dl', 'd.'], lx, ly + 1, {'d': DRY_LEAF[1], 'l': DRY_LEAF[0]})
    if v == 2:
        for _ in range(3):
            x, y = r.randint(7, 25), r.randint(15, 22)
            c.stamp(['.b.', 'bBb'], x - 1, y, {'b': P('cool', 1), 'B': P('cool', 3)})
    snow_on_tops(c, rng('vine-snow', v), 0.5, skip=(P('cool', 1), P('cool', 3)))
    return c


ROOT_CROWN = (N('white'), P('cool', 5), P('cool', 4))


def root_crops_dormant(v):
    """Wild turnips in winter: the leaves died back to flattened brown stubs around each root,
    and from stage 1 the root's pale crown showing at the soil (still there to lift), with
    snow on the stubs and on the mound."""
    r = rng('root_crops-winter', v)
    c = Canvas(32, 32)
    for (x, y) in ((8, 10), (22, 9), (11, 23), (23, 22)):
        x += r.randint(-1, 1)
        c.disc(x, y + 3, 3, 1, P('earth', 2), 160)
        stubs = 3 + v
        for k in range(stubs):
            ang = k * 6.283 / stubs + r.random() * 0.5
            ln = 1.5 + v
            lx, ly = x + int(round(math.cos(ang) * ln)), y + 1 + int(round(math.sin(ang) * ln * 0.4))
            c.line(x, y + 1, lx, ly, DRY_LEAF[1])
            c.px(lx, ly - 1, DRY_LEAF[0])
        if v >= 1:
            c.stamp(['.ww.', 'cwwc'], x - 2, y + 1, {'w': ROOT_CROWN[0], 'c': ROOT_CROWN[1]})
        if v == 2:
            c.stamp(['cc'], x - 1, y + 3, {'c': ROOT_CROWN[2]})
        c.px(x, y - 1, DRY_LEAF[0])                          # the dead leaf stalk at the crown
        c.px(x, y, DRY_LEAF[1])
    snow_on_tops(c, rng('root_crops-snow', v), 0.75, skip=ROOT_CROWN)
    return c


def flowering_plant_dormant(v):
    """Flowering plants in winter: the same clump of stems dried to straw, a few curled dead
    leaves low down, and the seed heads left standing (0 broken stems, 1 small pods,
    2 round dark heads), capped with snow."""
    r = rng('flowering', v)                                  # the summer layout, so it is the same clump
    c = Canvas(32, 32)
    ground_shadow(c, 16, 28, 5, 1.5)
    for k in range(3 + v):
        sx = 16 + (k - (2 + v) / 2.0) * 3 + r.randint(-1, 1)
        h = 7 + v * 4 + r.randint(0, 3)
        tx = sx + r.choice((-1, 0, 1))
        if v == 0:
            h = h * 2 // 3                                   # snapped off short
        c.line(int(sx), 28, int(tx), 28 - h, STRAW[1] if k % 2 else STRAW[2])
        c.px(int(sx) - 1, 28 - h // 2, STRAW[0])             # lit fleck on the upper-left side
        side = 1 if k % 2 else -1
        c.px(int(sx) + side, 26, DRY_LEAF[0])                # a curled leaf at the base
        c.px(int(sx) + side * 2, 27, DRY_LEAF[1])
        top = (int(tx), 28 - h)
        if v == 1:
            c.stamp(['p', 'P'], top[0], top[1] - 1, {'p': STRAW[1], 'P': DRY_LEAF[1]})
        elif v == 2:
            c.stamp(['.hh.', 'hDDd', '.dd.'], top[0] - 1, top[1] - 2,
                    {'h': P('earth', 2), 'D': P('earth', 0), 'd': P('earth', 1)})
    snow_on_tops(c, rng('flowering-snow', v), 0.7)
    return c


def flowering_plant_autumn(v):
    """Autumn: the leaves turn gold, and a grown plant's flowers have gone over to brown seed
    heads with a last petal or two."""
    c = recolour(flowering_plant(v), AUTUMN_GOLD)
    if v == 2:
        petal = _key(P('cool', 6))
        centre = _key(P('warm', 7))
        for y in range(c.h):
            for x in range(c.w):
                if c.a[y, x, 3]:
                    k = _key(c.a[y, x])
                    if k == centre:
                        c.a[y, x, :3] = P('earth', 1)
                    elif k == petal and (x + y) % 3:
                        c.a[y, x, :3] = P('earth', 3)
    return c


def fern(v, season=''):
    """Fern fronds arching out from a crown, each a curved stem with leaflets on both sides.
    Stages 0-2 lengthen the fronds and add more. Autumn: russet bracken; winter: dead and
    snowed on."""
    r = rng('fern', v)
    c = Canvas(32, 32)
    ground_shadow(c, 16, 27, 5 + v * 2, 1.5)
    n = 4 + v * 2
    length = 5 + v * 4
    for i in range(n):
        side = -1 if i % 2 else 1
        spread = (i // 2 + 0.5) / ((n + 1) // 2)
        ang = side * (0.25 + spread * 1.1)                  # 0 is straight up
        x, y = 16.0, 26.0
        for k in range(length):
            bend = ang + side * k * 0.07
            x += math.sin(bend)
            y -= math.cos(bend) * 0.9
            px, py = int(round(x)), int(round(y))
            c.px(px, py, P('foliage', 3))
            if k % 2 == 0 and k < length - 1:               # leaflets, lit on top
                c.px(px - 1, py - 1, P('foliage', 5))
                c.px(px + 1, py, P('foliage', 2))
        c.px(int(round(x)), int(round(y)) - 1, P('foliage', 4))
    c.disc(16, 26, 2, 1, P('foliage', 2))
    if season == 'autumn':
        c = recolour(c, AUTUMN_RUSSET)
    elif season == 'winter':
        c = recolour(c, DEAD)
        snow_on_tops(c, rng('fern-snow', v), 0.5)
    return c



# ---------------------------------------------------------------- the valley's flora (#286)
# Five more trees, each with three stages and season sheets like the oak; wild food, look-alikes,
# fibre, dye and healing plants in three stages; and five field crops in four, like the grain.
# Drawn with the same ramps, sizes and light (upper left) as the plants above.
G = pal.ramp
OLIVE = (G('olive', 2), G('olive', 3), G('olive', 5), G('olive', 6))
ALDER = (P('foliage', 0), P('foliage', 2), P('foliage', 3), P('foliage', 4))
ASH = (P('foliage', 2), P('foliage', 4), P('foliage', 5), P('foliage', 6))
APPLE = (P('foliage', 1), P('foliage', 3), P('foliage', 4), P('foliage', 6))
YEW = (P('foliage', 0), P('foliage', 1), P('foliage', 2), P('foliage', 3))
GREY_BARK = (P('stone', 5), P('stone', 4), P('stone', 2))
DARK_BARK = (P('stone', 3), P('stone', 2), P('stone', 1))
RED_BARK = (G('copper', 4), G('copper', 3), G('copper', 1))
ASH_AUTUMN = (G('gold', 3), G('gold', 4), G('gold', 5), G('gold', 6))
WILLOW_AUTUMN = (G('olive', 3), G('gold', 4), G('gold', 5), G('gold', 6))
DULL_AUTUMN = (P('earth', 2), P('earth', 3), P('earth', 4), P('earth', 5))   # alder: browns


def _bare(c, stage, cx, base, bark, r, spread):
    """The winter tree of a stage: trunk already drawn; limbs to the canvas top."""
    top = 4 if stage else 8
    bare_crown(c, cx, top, base, spread, r, bark, 2 + stage)


def alder(stage, season=''):
    """An alder: a narrow, rounded cone of dark leaf on a straight grey trunk, by water. Autumn
    browns rather than colours; winter shows the bare limbs and the little dark cones."""
    r = rng('alder', stage)
    leaves = DULL_AUTUMN if season == 'autumn' else ALDER
    size = [(32, 32), (32, 48), (64, 64)][stage]
    c = Canvas(*size)
    cx, base = size[0] // 2, size[1] - 3
    ground_shadow(c, cx, base, [4, 6, 10][stage], 2)
    tw = [2, 3, 4][stage]
    trunk(c, cx - tw // 2, [18, 18, 20][stage], base, tw, *DARK_BARK)
    if season == 'winter':
        _bare(c, stage, cx, [18, 20, 22][stage], DARK_BARK, r, 0.5)
        for _ in range(3 + stage * 3):                       # last year's cones
            x, y = r.randint(cx - 6 - stage * 4, cx + 6 + stage * 4), r.randint(6, [16, 18, 22][stage])
            if c.a[y, x, 3]:
                c.px(x, y, P('earth', 0))
        return c
    # A tall oval crown with a lobe low on each side, so it reads narrow and upright.
    w = [5, 8, 14][stage]
    h = [7, 11, 17][stage]
    cy = [11, 15, 21][stage]
    canopy_blob(c, cx, cy, w, h, leaves, r, 3 + stage * 2)
    canopy_blob(c, cx - w + 1, cy + h // 2, w // 2 + 1, h // 3 + 1, leaves, r, 1 + stage)
    canopy_blob(c, cx + w - 2, cy + h // 3, w // 2 + 1, h // 3 + 1, leaves, r, 1 + stage)
    return c


def ash(stage, season=''):
    """An ash: tall pale grey trunk, an open crown of separate leaf clusters with sky between,
    yellow in autumn; bare in winter."""
    r = rng('ash', stage)
    leaves = ASH_AUTUMN if season == 'autumn' else ASH
    size = [(32, 32), (32, 48), (64, 64)][stage]
    c = Canvas(*size)
    cx, base = size[0] // 2, size[1] - 3
    ground_shadow(c, cx, base, [4, 7, 12][stage], 2)
    tw = [2, 3, 5][stage]
    trunk(c, cx - tw // 2, [18, 22, 28][stage], base, tw, *GREY_BARK)
    if season == 'winter':
        _bare(c, stage, cx, [18, 22, 28][stage], GREY_BARK, r, 0.85)
        return c
    if stage == 2:
        c.line(cx - 1, 30, cx - 12, 18, GREY_BARK[1])
        c.line(cx + 1, 30, cx + 13, 20, GREY_BARK[1])
        c.line(cx, 28, cx + 2, 10, GREY_BARK[1])
    spots = [[(16, 13, 5, 4), (12, 18, 3, 3), (20, 17, 3, 3)],
             [(16, 12, 6, 5), (9, 20, 4, 4), (23, 19, 4, 4), (16, 22, 4, 3)],
             [(32, 9, 9, 6), (18, 18, 8, 6), (46, 19, 8, 6), (26, 27, 7, 5), (40, 28, 7, 5), (32, 18, 6, 4)]][stage]
    for (x, y, rx, ry) in spots:
        canopy_blob(c, x, y, rx, ry, leaves, r, 2 + stage)
    return c


def willow(stage, season=''):
    """A willow: a leaning trunk and a crown of silver-green hanging strands to near the ground;
    autumn yellows them; winter leaves the bare yellow-grey whips hanging."""
    r = rng('willow', stage)
    leaves = WILLOW_AUTUMN if season == 'autumn' else OLIVE
    size = [(32, 32), (32, 48), (64, 64)][stage]
    c = Canvas(*size)
    cx, base = size[0] // 2, size[1] - 3
    ground_shadow(c, cx, base, [5, 8, 14][stage], 2)
    tw = [2, 3, 5][stage]
    top_y = [16, 20, 26][stage]
    for y in range(top_y, base + 1):                         # a trunk leaning a little west
        x = cx - tw // 2 - (base - y) // 8
        c.rect(x, y, tw, 1, BARK[1])
        c.px(x, y, BARK[0])
        c.px(x + tw - 1, y, BARK[2])
    hx = cx - (base - top_y) // 8
    w = [7, 11, 22][stage]
    crown_y = [9, 11, 14][stage]
    if season != 'winter':
        canopy_blob(c, hx, crown_y, w, [5, 7, 10][stage], leaves, r, 2 + stage)
    strand = (G('olive', 4), G('olive', 5)) if season != 'winter' else (G('gold', 3), G('olive', 3))
    if season == 'autumn':
        strand = (G('gold', 4), G('gold', 5))
    for k in range(6 + stage * 6):                           # hanging strands
        x = hx - w + 1 + (k * (2 * w - 2)) // (5 + stage * 6)
        y0 = crown_y + r.randint(-2, 3)
        y1 = min(base - 2, y0 + [10, 18, 30][stage] + r.randint(-3, 2))
        c.vline(x, y0, y1, strand[k % 2])
        if season != 'winter' and r.random() < 0.5:
            c.px(x + 1, (y0 + y1) // 2, leaves[1])
    if season == 'winter':
        snow_on_tops(c, rng('willow-snow', stage), 0.4)
    return c


def crab_apple(stage, season=''):
    """A crab apple: a short crooked trunk and a round, wide crown; red-yellow apples on a grown
    tree in summer and autumn; bare and snowed in winter."""
    r = rng('crab_apple', stage)
    leaves = (P('warm', 1), P('warm', 2), P('earth', 5), P('warm', 5)) if season == 'autumn' else APPLE
    size = [(32, 32), (32, 48), (64, 64)][stage]
    c = Canvas(*size)
    cx, base = size[0] // 2, size[1] - 3
    ground_shadow(c, cx, base, [5, 8, 14][stage], 2)
    tw = [2, 3, 5][stage]
    tt = [20, 28, 38][stage]
    trunk(c, cx - tw // 2, tt, base, tw, *BARK)
    c.px(cx - tw // 2 - 1, tt + 3, BARK[2])                 # a crook
    if season == 'winter':
        _bare(c, stage, cx, tt, BARK, r, 1.0)
        return c
    if stage == 2:
        c.line(cx, tt + 1, cx - 10, tt - 8, BARK[1])
        c.line(cx, tt + 1, cx + 11, tt - 7, BARK[1])
    crown = [(16, 14, 7, 6), (16, 17, 12, 10), (32, 24, 24, 15)][stage]
    canopy_blob(c, crown[0], crown[1], crown[2], crown[3], leaves, r, 4 + stage * 3)
    if stage == 2:
        canopy_blob(c, 18, 30, 9, 6, leaves, r, 3)
        canopy_blob(c, 46, 29, 9, 6, leaves, r, 3)
        for _ in range(14):                                  # the apples
            x = r.randint(12, 52)
            y = r.randint(12, 34)
            if c.a[y, x, 3]:
                c.stamp(['ab', 'bb'], x, y, {'a': P('warm', 6), 'b': P('warm', 3)})
    return c


def yew(stage, season=''):
    """A yew: a short red-barked trunk under a broad, very dark, dense crown that keeps its
    leaves all year; in winter snow lies on it."""
    r = rng('yew', stage)
    size = [(32, 32), (32, 48), (64, 64)][stage]
    c = Canvas(*size)
    cx, base = size[0] // 2, size[1] - 3
    ground_shadow(c, cx, base, [5, 9, 16][stage], 2)
    tw = [2, 4, 7][stage]
    tt = [20, 30, 40][stage]
    trunk(c, cx - tw // 2, tt, base, tw, *RED_BARK)
    if stage == 2:                                           # a fluted, split trunk
        c.vline(cx, tt + 2, base - 1, RED_BARK[2])
    crown = [(16, 13, 8, 8), (16, 18, 13, 12), (32, 22, 26, 18)][stage]
    canopy_blob(c, crown[0], crown[1] + 2, crown[2], crown[3] - 2, YEW, r, 6 + stage * 4)
    canopy_blob(c, crown[0], crown[1] - crown[3] // 2, crown[2] * 2 // 3, crown[3] // 2 + 1, YEW, r, 3)
    if stage >= 1:
        for _ in range(4 + stage * 4):                       # red arils
            x, y = r.randint(4, size[0] - 5), r.randint(4, tt)
            if c.a[y, x, 3]:
                c.px(x, y, P('warm', 3))
    if season == 'winter':
        snow_on_tops(c, rng('yew-snow', stage), 0.8)
    return c


# ---- low plants: three stages each (0 young, 1 grown, 2 bearing or in flower)
def _leafy(c, r, cx, cy, rx, ry, steps, tufts=3):
    canopy_blob(c, cx, cy, rx, ry, steps, r, tufts)


LEAF = (P('foliage', 1), P('foliage', 3), P('foliage', 4), P('foliage', 5))
BRIGHT = (P('foliage', 2), P('foliage', 4), P('foliage', 5), P('foliage', 6))


def hazel(v):
    """Hazel: a clump of thin straight stems with round leaves; from stage 2 clusters of pale
    brown nuts in their green husks."""
    r = rng('hazel', v)
    c = Canvas(32, 32)
    ground_shadow(c, 16, 28, 7 + v, 2)
    for k in range(3 + v * 2):                               # the stems
        x = 11 + k * (10 // (2 + v * 2)) + r.randint(0, 1)
        c.line(x, 28, x + r.choice((-2, -1, 1, 2)), 28 - (10 + v * 4), BARK[1])
    for (x, y) in [(16, 14), (11, 17), (21, 17), (15, 9), (20, 11), (10, 12)][:2 + v * 2]:
        _leafy(c, r, x, y + (2 - v), 4 + v // 2, 3 + v // 2, LEAF, 2)
    if v == 2:
        for (x, y) in ((12, 15), (19, 12), (22, 18), (15, 19)):
            c.stamp(['nN', 'Nn'], x, y, {'n': P('earth', 6), 'N': P('earth', 5)})
            c.px(x - 1, y, P('foliage', 5))
    return c


def _berry_low(name, v, berry, leaf=LEAF, size=6):
    """A low wiry bush (bilberry, nightshade): a mound of small leaves; berries at stage 2."""
    r = rng(name, v)
    c = Canvas(32, 32)
    ground_shadow(c, 16, 28, 6 + v, 1.5)
    for k in range(4 + v * 2):
        x = 10 + r.randint(0, 12)
        c.line(16, 28, x, 28 - (4 + v * 3) - r.randint(0, 2), BARK[2])
    _leafy(c, r, 16, 25 - v * 2, size + v * 2, 3 + v, leaf, 4 + v)
    if v == 2:
        for _ in range(7):
            x, y = r.randint(9, 23), r.randint(17, 26)
            c.stamp(['b', 'B'] if r.random() < 0.5 else ['bB'], x, y, {'b': berry[0], 'B': berry[1]})
    return c


def bilberry(v):
    """Bilberry: a low wiry mound of small bright leaves with dark blue berries when grown."""
    return _berry_low('bilberry', v, (P('water', 2), P('cool', 1)), BRIGHT)


def nightshade(v):
    """Nightshade: a straggling plant of dull leaves; when grown, purple star flowers with
    yellow eyes and shining black berries, one to a stalk (so it reads apart from a bilberry)."""
    c = _berry_low('nightshade', v, (P('stone', 0), N('black')), LEAF, 5)
    if v >= 1:
        r = rng('nightshade-flower', v)
        for _ in range(2 + v):
            x, y = r.randint(9, 23), r.randint(16, 23)
            c.stamp(['.p.', 'pyp', '.p.'], x - 1, y - 1, {'p': P('cool', 4), 'y': P('warm', 7)})
    if v == 2:
        r = rng('nightshade-shine', v)
        for _ in range(4):
            x, y = r.randint(10, 22), r.randint(18, 25)
            c.stamp(['k', 'K'], x, y, {'k': P('stone', 4), 'K': N('black')})   # a lit black berry
    return c


def wild_strawberry(v):
    """Wild strawberry: low three-leaved plants in the grass, white flowers at 1, small red
    berries at 2."""
    r = rng('strawberry', v)
    c = Canvas(32, 32)
    for (x, y) in ((8, 12), (20, 9), (13, 21), (24, 22), (6, 25)):
        x += r.randint(-1, 1)
        for dx, dy in ((-2, 0), (2, 0), (0, -2)):              # three leaflets
            c.disc(x + dx, y + dy, 1.5, 1.2, P('foliage', 4))
            c.px(x + dx - 1, y + dy - 1, P('foliage', 5))
        c.px(x, y, P('foliage', 2))
        if v >= 1:
            c.stamp(['.w.', 'wyw', '.w.'], x + 2, y + 1, {'w': N('white'), 'y': P('warm', 7)})
        if v == 2:
            c.stamp(['rR', 'RR'], x - 3, y + 2, {'r': P('warm', 5), 'R': P('warm', 3)})
    return c


def bramble(v):
    """Bramble: arching thorny canes with leaves; at 2 blackberries, some still red."""
    r = rng('bramble', v)
    c = Canvas(32, 32)
    ground_shadow(c, 16, 28, 9, 2)
    for k in range(3 + v * 2):                               # arching canes
        x0 = 8 + r.randint(0, 16)
        h = 8 + v * 4 + r.randint(0, 3)
        dirn = r.choice((-1, 1))
        pts = [(x0, 28), (x0 + dirn * 2, 28 - h), (x0 + dirn * 7, 28 - h + 3), (x0 + dirn * 9, 26)]
        for (a, b), (cc, d) in zip(pts, pts[1:]):
            c.line(a, b, cc, d, G('copper', 2))
        for (a, b) in pts[1:3]:
            c.disc(a, b, 2, 1.5, P('foliage', 3))
            c.px(a - 1, b - 1, P('foliage', 5))
    if v == 2:
        for _ in range(8):
            x, y = r.randint(7, 25), r.randint(12, 24)
            ripe = r.random() < 0.7
            c.stamp(['bB', 'BB'], x, y, {'b': P('cool', 2) if ripe else P('warm', 4),
                                          'B': P('cool', 0) if ripe else P('warm', 2)})
    return c


def _rosette(c, r, x, y, n, ln, col, lit, broad=False):
    for k in range(n):
        ang = k * 6.283 / n + r.random() * 0.3
        lx, ly = x + int(round(math.cos(ang) * ln)), y + int(round(math.sin(ang) * ln * 0.6))
        c.line(x, y, lx, ly, col)
        if broad:
            c.disc((x + lx) / 2.0, (y + ly) / 2.0, 1.3, 1, col)
        c.px(lx, ly - 1, lit)


def wild_garlic(v):
    """Wild garlic: broad bright leaves from the ground; at 2 balls of white star flowers."""
    r = rng('garlic', v)
    c = Canvas(32, 32)
    for (x, y) in ((9, 11), (21, 9), (12, 22), (23, 21)):
        _rosette(c, r, x, y, 4 + v, 3 + v, P('foliage', 4), P('foliage', 6), True)
        if v >= 1:
            c.line(x, y, x + 1, y - 4 - v, P('foliage', 5))
        if v == 2:
            c.stamp(['w.w', '.W.', 'w.w'], x, y - 8, {'w': N('white'), 'W': P('stone', 6)})
    return c


def lily_of_the_valley(v):
    """Lily of the valley: two broad leaves to a plant, like wild garlic's, and an arching stalk
    of small white bells (not a round head of stars)."""
    r = rng('lily', v)
    c = Canvas(32, 32)
    for (x, y) in ((9, 12), (21, 10), (12, 23), (23, 22)):
        for side in (-1, 1):
            c.line(x, y + 2, x + side * (2 + v), y - 2 - v, P('foliage', 3))
            c.line(x + side, y + 2, x + side * (3 + v), y - 1 - v, P('foliage', 4))
        if v >= 1:
            c.line(x, y + 1, x + 2, y - 4 - v, P('foliage', 5))
            c.px(x + 3, y - 4 - v, P('foliage', 5))
        if v == 2:
            for k in range(3):
                c.px(x + 3 + k % 2, y - 3 - v + k * 2, N('white'))
    return c


def _mushroom_patch(name, v, spots, draw):
    r = rng(name, v)
    c = Canvas(32, 32)
    n = 1 + v
    for (x, y) in spots[:n + 1]:
        draw(c, x + r.randint(-1, 1), y, v)
    c.outline(BLACK)
    return c


def chanterelle(v):
    """Chanterelles: egg-yellow funnels with a wavy rim, standing in the leaf litter."""
    def one(c, x, y, v):
        w = 3 + v
        c.rect(x - 1, y - 3, 2, 4, G('gold', 5))             # tapering stem
        c.px(x, y, G('gold', 4))
        c.disc(x, y - 4, w / 2.0, 1.3, G('gold', 4))         # the funnel
        c.hline(x - w // 2, x + w // 2, y - 5, G('gold', 6))
        c.px(x - w // 2, y - 5, G('gold', 3))
    return _mushroom_patch('chanterelle', v, [(10, 22), (18, 18), (23, 25), (13, 27)], one)


def death_cap(v):
    """Death caps: pale greenish-white caps on white stalks, a skirt below the cap and a cup at
    the foot."""
    def one(c, x, y, v):
        w = 4 + v
        c.rect(x - 1, y - 5, 2, 6, P('stone', 7))
        c.hline(x - 2, x + 1, y - 3, P('stone', 6))          # the skirt
        c.disc(x, y, 2, 1, P('stone', 6))                    # the cup
        c.disc(x, y - 6, w / 2.0, 1.6, G('olive', 6))
        c.hline(x - w // 2 + 1, x + w // 2 - 1, y - 7, G('olive', 7))
    return _mushroom_patch('death_cap', v, [(11, 24), (19, 20), (24, 27), (14, 28)], one)


def _umbel(name, v, stem, head, leaf, blotch=None, feathery=False):
    """Umbellifers (hemlock, yarrow): stems with flat flower heads; leaves low down."""
    r = rng(name, v)
    c = Canvas(32, 32)
    ground_shadow(c, 16, 28, 6, 1.5)
    for k in range(2 + v):
        x = 10 + k * (12 // (1 + v)) + r.randint(0, 2)
        h = 8 + v * 6 + r.randint(0, 3)
        c.line(x, 28, x, 28 - h, stem)
        if blotch and v >= 1:
            for y in range(28 - h + 2, 27, 3):
                c.px(x, y, blotch)
        for ly in range(26, 28 - h + 3, -4):                 # leaves
            side = 1 if (ly // 4 + k) % 2 else -1
            if feathery:
                for j in range(3):
                    c.px(x + side * (1 + j), ly - j % 2, leaf)
            else:
                c.line(x, ly, x + side * 3, ly - 2, leaf)
                c.px(x + side * 2, ly - 2, P('foliage', 5))
        if v >= 1:                                           # the flat head
            w = 2 + v
            c.hline(x - w, x + w, 28 - h - 1, head[0])
            c.hline(x - w + 1, x + w - 1, 28 - h - 2, head[1])
            for j in range(-w, w + 1, 2):
                c.px(x + j, 28 - h, stem)
    return c


def hemlock(v):
    """Hemlock: tall smooth stems blotched with purple, ferny leaves, white umbels (like a wild
    carrot gone tall)."""
    return _umbel('hemlock', v, P('foliage', 3), (N('white'), P('stone', 6)), P('foliage', 4),
                  blotch=P('cool', 2))


def yarrow(v):
    """Yarrow: feathered leaves and flat cream-white flower heads, shorter than hemlock."""
    c = _umbel('yarrow', v, P('foliage', 2), (P('stone', 6), N('white')), P('foliage', 5),
               feathery=True)
    return c


def cattail(v):
    """Bulrushes: tall strap leaves at the water's edge; brown cigar heads on the grown stand."""
    r = rng('cattail', v)
    c = Canvas(32, 32)
    c.disc(16, 29, 6 + v * 2, 1.5, P('earth', 1), 140)
    for i in range(4 + v * 3):
        x = 8 + r.randint(0, 16)
        h = [8, 14, 22][v] + r.randint(0, 3)
        dx = r.choice((-1, 0, 1))
        c.line(x, 29, x + dx, 29 - h, G('olive', 4) if i % 2 else G('olive', 3))
        c.px(x + dx, 29 - h, G('olive', 6))
        if v == 2 and i % 2 == 0:
            c.rect(x + dx, 29 - h + 2, 2, 5, P('earth', 1))
            c.px(x + dx, 29 - h + 2, P('earth', 3))
    return c


def nettle(v):
    """Nettles: upright square stems with paired, toothed dark leaves and hanging green catkins."""
    r = rng('nettle', v)
    c = Canvas(32, 32)
    ground_shadow(c, 16, 28, 7, 1.5)
    for k in range(3 + v):
        x = 9 + k * (14 // (2 + v)) + r.randint(0, 1)
        h = 8 + v * 5 + r.randint(0, 2)
        c.line(x, 28, x, 28 - h, P('foliage', 2))
        for ly in range(26, 28 - h, -3):
            for side in (-1, 1):
                c.stamp(['ab', 'ba'] if side > 0 else ['ba', 'ab'], x + (1 if side > 0 else -2), ly - 1,
                        {'a': P('foliage', 3), 'b': P('foliage', 4)})
            if v == 2 and ly < 22:
                c.px(x + 2, ly + 1, G('olive', 5))
        c.px(x, 28 - h - 1, P('foliage', 5))
    return c


def woad(v):
    """Woad: a blue-green rosette, then tall stems with clouds of small yellow flowers."""
    r = rng('woad', v)
    c = Canvas(32, 32)
    ground_shadow(c, 16, 28, 6, 1.5)
    _rosette(c, r, 16, 26, 6, 4 + v, G('olive', 3), G('olive', 5), True)
    if v >= 1:
        for k in range(2 + v):
            x = 12 + k * 3 + r.randint(0, 1)
            h = 8 + v * 5
            c.line(16, 25, x, 26 - h, G('olive', 4))
            if v == 2:
                c.disc(x, 26 - h - 1, 2.5, 1.5, G('gold', 5))
                c.px(x - 1, 26 - h - 2, G('gold', 6))
                c.px(x + 1, 26 - h, G('gold', 4))
    return c


def madder(v):
    """Madder: a sprawl of slender stems with whorls of narrow leaves and tiny yellow flowers;
    the red root shows at the soil when grown."""
    r = rng('madder', v)
    c = Canvas(32, 32)
    for k in range(4 + v * 2):
        x0 = 16 + r.randint(-3, 3)
        ang = (k / (3.0 + v * 2) - 0.5) * 2.4
        ln = 6 + v * 3
        x1 = int(round(x0 + math.sin(ang) * ln))
        y1 = int(round(27 - math.cos(ang) * ln * 0.6))
        c.line(x0, 27, x1, y1, P('foliage', 3))
        for t in (0.5, 1.0):                                 # whorls
            wx = int(round(x0 + (x1 - x0) * t))
            wy = int(round(27 + (y1 - 27) * t))
            for dx, dy in ((-1, 0), (1, 0), (0, -1)):
                c.px(wx + dx, wy + dy, P('foliage', 4))
        if v >= 1:
            c.px(x1, y1 - 1, G('gold', 6))
    if v == 2:
        c.stamp(['rRr'], 15, 28, {'r': P('warm', 2), 'R': P('warm', 3)})
    return c


def weld(v):
    """Weld: a rosette of wavy leaves, then a tall slim spike of pale yellow-green flowers."""
    r = rng('weld', v)
    c = Canvas(32, 32)
    ground_shadow(c, 16, 28, 5, 1.5)
    _rosette(c, r, 16, 26, 7, 3 + v, P('foliage', 4), P('foliage', 6), True)
    if v >= 1:
        for k in range(1 + v):
            x = 14 + k * 4
            h = 10 + v * 6
            c.line(x, 25, x, 25 - h, P('foliage', 3))
            for y in range(25 - h, 25 - h + (4 + v * 3)):
                c.px(x, y, G('olive', 6) if y % 2 else G('olive', 7))
                if v == 2 and y % 2:
                    c.px(x - 1, y, G('olive', 5))
    return c


def comfrey(v):
    """Comfrey: big rough leaves, and from 1 drooping clusters of purple bells."""
    r = rng('comfrey', v)
    c = Canvas(32, 32)
    ground_shadow(c, 16, 28, 8, 2)
    for (x, y, rx, ry) in [(16, 22, 6, 4), (11, 20, 4, 3), (21, 20, 4, 3)][:1 + v] + [(16, 18, 4, 3)][:v // 2]:
        _leafy(c, r, x, y + 2, rx + v, ry, LEAF, 2)
    if v >= 1:
        for k in range(1 + v):
            x = 12 + k * 4
            c.line(x, 22, x + 1, 13 - v * 2, P('foliage', 3))
            c.stamp(['pP', '.p'], x, 13 - v * 2, {'p': P('cool', 3), 'P': P('cool', 4)})
    return c


# ---- crops: four stages like the grain (0 seedlings, 1 green, 2 green heads or flowers,
# 3 ripe). Rows run east-west over a furrow line.
def _crop_rows(name, stage, draw, n0=6):
    r = rng(name, stage)
    c = Canvas(32, 32)
    n = n0 + stage * 2
    for i in range(n):
        x = 2 + (i * 28) // n + r.randint(0, 1)
        draw(c, r, i, x, stage)
    c.rect(0, 30, 32, 2, P('earth', 2), 90)
    return c


def barley(stage):
    """Barley: shorter than wheat, the heads nodding over with long whiskers; ripe a pale gold."""
    def one(c, r, i, x, s):
        h = 6 + s * 5 + r.randint(0, 2)
        green = P('foliage', 4) if i % 2 else P('foliage', 3)
        col = green if s < 3 else (G('gold', 5) if i % 2 else G('gold', 4))
        c.line(x, 30, x, 30 - h, col)
        if s == 0:
            c.px(x + 1, 30 - h + 2, P('foliage', 5))
        if s >= 2:
            ear = (P('foliage', 5), P('foliage', 4)) if s == 2 else (G('gold', 6), G('gold', 5))
            for k in range(4):                               # nodding over to the east
                c.px(x + 1 + k // 2, 30 - h + k, ear[k % 2])
            for k in range(3):                               # whiskers
                c.px(x - 1 - k, 30 - h - 1 - k, ear[0])
    return _crop_rows('barley', stage, one)


def oats(stage):
    """Oats: open, loose heads of hanging spikelets; ripe a pale straw."""
    def one(c, r, i, x, s):
        h = 7 + s * 6 + r.randint(0, 2)
        col = (P('foliage', 4) if i % 2 else P('foliage', 3)) if s < 3 else P('earth', 6)
        c.line(x, 30, x + r.choice((-1, 0, 1)), 30 - h, col)
        if s == 1:
            c.line(x, 30 - h + 5, x - 2, 30 - h + 3, P('foliage', 5))
        if s >= 2:
            sp = P('foliage', 5) if s == 2 else P('earth', 7)
            for dx, dy in ((-2, 1), (2, 1), (-1, 3), (2, 4), (-2, 5)):
                c.px(x + dx, 30 - h + dy, sp)
    return _crop_rows('oats', stage, one)


def rye(stage):
    """Rye: the tallest of the grains, blue-green blades and slim ears; ripe a grey-gold."""
    def one(c, r, i, x, s):
        h = 8 + s * 6 + r.randint(0, 2)
        col = (G('olive', 4) if i % 2 else G('olive', 3)) if s < 3 else (P('stone', 5) if i % 2 else P('earth', 5))
        c.line(x, 30, x, 30 - h, col)
        if s == 1:
            c.px(x + 1, 30 - h + 3, G('olive', 5))
        if s >= 2:
            ear = G('olive', 6) if s == 2 else G('gold', 3)
            c.vline(x, 30 - h - 5, 30 - h, ear)
            c.px(x - 1, 30 - h - 3, ear)
    return _crop_rows('rye', stage, one, 7)


def peas(stage):
    """Peas: low leafy vines along the rows with curling tendrils; white flowers at 2, green
    pods at 3."""
    def one(c, r, i, x, s):
        if i % 2:
            return
        h = 4 + s * 3
        c.line(x, 30, x + 1, 30 - h, P('foliage', 3))
        for k in range(1 + s):
            y = 30 - 2 - k * 3
            c.disc(x + (2 if k % 2 else -1), y, 1.3, 1, P('foliage', 4))
            c.px(x + (3 if k % 2 else -2), y - 1, P('foliage', 5))
        c.px(x + 2, 30 - h - 1, P('foliage', 5))            # a tendril
        if s == 2:
            c.px(x - 1, 30 - h, N('white'))
        if s == 3:
            c.vline(x + 2, 30 - h + 2, 30 - h + 4, P('foliage', 6))
    return _crop_rows('peas', stage, one)


def flax(stage):
    """Flax: thin upright stems; small sky-blue flowers at 2; ripe stems golden with round seed
    bolls at 3."""
    def one(c, r, i, x, s):
        h = 6 + s * 5 + r.randint(0, 2)
        col = P('foliage', 4) if s < 3 else P('earth', 4)
        c.line(x, 30, x + r.choice((-1, 0, 1)), 30 - h, col)
        if s == 2:
            c.stamp(['.b.', 'bwb'], x - 1, 30 - h - 1, {'b': P('water', 5), 'w': P('water', 7)})
        if s == 3 and i % 2 == 0:                            # round seed bolls on the ripe stems
            c.stamp(['oO', 'Oo'], x - 1, 30 - h - 2, {'o': P('earth', 6), 'O': P('earth', 5)})
    return _crop_rows('flax', stage, one, 8)


NEW_TREES = (('alder', alder), ('ash', ash), ('yew', yew), ('willow', willow), ('crab_apple', crab_apple))
NEW_STAGED = (('hazel', hazel), ('bilberry', bilberry), ('wild_strawberry', wild_strawberry),
              ('bramble', bramble), ('wild_garlic', wild_garlic), ('chanterelle', chanterelle),
              ('cattail', cattail), ('death_cap', death_cap), ('hemlock', hemlock),
              ('nightshade', nightshade), ('lily_of_the_valley', lily_of_the_valley),
              ('nettle', nettle), ('woad', woad), ('madder', madder), ('weld', weld),
              ('yarrow', yarrow), ('comfrey', comfrey), ('thistle', thistle))
# Which of them turn in autumn (the rest keep their summer sheet until the winter one).
TURNS = {'hazel', 'bramble', 'nettle', 'woad', 'weld', 'comfrey', 'cattail', 'hemlock'}
NEW_CROPS = (('barley', barley), ('oats', oats), ('rye', rye), ('peas', peas), ('flax', flax))


def flora_286():
    for name, fn in NEW_TREES:
        for s in range(3):
            fn(s).save(os.path.join(OUT, '%s_%d.png' % (name, s)))
            fn(s, 'winter').save(os.path.join(OUT, '%s_%d_winter.png' % (name, s)))
            if name != 'yew':
                fn(s, 'autumn').save(os.path.join(OUT, '%s_%d_autumn.png' % (name, s)))
    for name, fn in NEW_STAGED:
        for s in range(3):
            c = fn(s)
            c.save(os.path.join(OUT, '%s_%d.png' % (name, s)))
            if name in TURNS:
                recolour(c, AUTUMN_GOLD).save(os.path.join(OUT, '%s_%d_autumn.png' % (name, s)))
            w = recolour(c, DEAD)
            snow_on_tops(w, rng(name + '-snow', s), 0.6)
            w.save(os.path.join(OUT, '%s_%d_winter.png' % (name, s)))
    for name, fn in NEW_CROPS:
        for s in range(4):
            fn(s).save(os.path.join(OUT, '%s_%d.png' % (name, s)))


def main():
    os.makedirs(OUT, exist_ok=True)
    for name, fn in (('oak', oak), ('birch', birch), ('pine', pine)):
        for s in range(3):
            fn(s).save(os.path.join(OUT, '%s_%d.png' % (name, s)))
            fn(s, 'winter').save(os.path.join(OUT, '%s_%d_winter.png' % (name, s)))
            if name != 'pine':
                fn(s, 'autumn').save(os.path.join(OUT, '%s_%d_autumn.png' % (name, s)))
    sheet([shrub(v) for v in range(3)], 32, 32).save(os.path.join(OUT, 'shrub.png'))
    berry_bush(False).save(os.path.join(OUT, 'berry_bush_bare.png'))
    berry_bush(True).save(os.path.join(OUT, 'berry_bush_fruit.png'))
    for season in ('autumn', 'winter'):
        sheet([shrub(v, season) for v in range(3)], 32, 32).save(os.path.join(OUT, 'shrub_%s.png' % season))
        for fruit, name in ((False, 'bare'), (True, 'fruit')):
            berry_bush(fruit, season).save(os.path.join(OUT, 'berry_bush_%s_%s.png' % (name, season)))
        for s in range(3):
            fern(s, season).save(os.path.join(OUT, 'fern_%d_%s.png' % (s, season)))
    # Reeds stand through the winter as dry straw with snow on the heads.
    for s in range(3):
        c = recolour(reed(s), AUTUMN_GOLD)
        snow_on_tops(c, rng('reed-snow', s), 0.5)
        c.save(os.path.join(OUT, 'reed_%d_winter.png' % s))
    c = recolour(sheet([reeds(v) for v in range(3)], 32, 32), AUTUMN_GOLD)
    snow_on_tops(c, rng('reeds-snow'), 0.5)
    c.save(os.path.join(OUT, 'reeds_winter.png'))
    sheet([reeds(v) for v in range(3)], 32, 32).save(os.path.join(OUT, 'reeds.png'))
    for s in range(4):
        grain(s).save(os.path.join(OUT, 'grain_%d.png' % s))
        root(s).save(os.path.join(OUT, 'root_%d.png' % s))
    sheet([wildflowers(v) for v in range(3)], 32, 32).save(os.path.join(OUT, 'wildflowers.png'))
    # Reserved alien flora inventory: no simulation PlantKind or spawn mapping yet, so these
    # variants are not reachable in play and must not be presented as a runtime family.
    sheet([alien_bloom(v) for v in range(3)], 32, 32).save(os.path.join(OUT, 'alien_bloom.png'))
    sheet([alien_bloom(v, 'autumn') for v in range(3)], 32, 32).save(os.path.join(OUT, 'alien_bloom_autumn.png'))
    sheet([alien_bloom(v, 'winter') for v in range(3)], 32, 32).save(os.path.join(OUT, 'alien_bloom_winter.png'))
    mushrooms().save(os.path.join(OUT, 'mushrooms.png'))
    sheet([fungus_tree(v) for v in range(3)], 32, 48).save(os.path.join(OUT, 'fungus_tree.png'))
    sheet([bone_tree(v) for v in range(3)], 32, 48).save(os.path.join(OUT, 'bone_tree.png'))
    stump().save(os.path.join(OUT, 'stump.png'))
    log().save(os.path.join(OUT, 'log.png'))
    for name, fn in (('herbs', herbs), ('reed', reed), ('vine', vine),
                      ('root_crops', root_crops), ('flowering_plant', flowering_plant),
                      ('fern', fern)):
        for s in range(3):
            fn(s).save(os.path.join(OUT, '%s_%d.png' % (name, s)))
    # Winter: the dormant plant, redrawn. Autumn where the plant turns: vine leaves go russet,
    # turnip tops gold, flowering plants gold and gone to seed. Herbs keep their summer sheet
    # in autumn (thyme and sage hold their leaves until the frost).
    for name, fn in (('herbs', herbs_dormant), ('vine', vine_dormant),
                     ('root_crops', root_crops_dormant), ('flowering_plant', flowering_plant_dormant)):
        for s in range(3):
            fn(s).save(os.path.join(OUT, '%s_%d_winter.png' % (name, s)))
    for s in range(3):
        recolour(vine(s), AUTUMN_RUSSET).save(os.path.join(OUT, 'vine_%d_autumn.png' % s))
        recolour(root_crops(s), AUTUMN_GOLD).save(os.path.join(OUT, 'root_crops_%d_autumn.png' % s))
        flowering_plant_autumn(s).save(os.path.join(OUT, 'flowering_plant_%d_autumn.png' % s))
    flora_286()
    print('plants -> %s' % OUT)


if __name__ == '__main__':
    main()
