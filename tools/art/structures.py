#!/usr/bin/env python3
"""Structures: godot/assets/structures/
  wall_{wood,stone_block,thatch}.png  16 wall-top pieces in a row (512x32), index = 4-bit
      adjacency N=1 E=2 S=4 W=8. A piece is the full 32x32 tile; every side with no
      neighbour gets a 2-px bevel (lit on N/W, dark on S/E) so runs connect seamlessly and
      ends/corners close.
  door_closed, door_open, window, stairs_down, stairs_up, stairs_both, ladder: 32x32
  furniture at 32x32: bed table chair stool chest shelf workbench loom anvil barrel crate well
      fence_post stockpile; hearth 3 frames (96x32), cooking_pot 2 frames (64x32),
      lamp 2 frames (64x32, second lit).
  strangers' camp (threat.rs bands): lean_to 48x40, campfire 3 frames (96x32), bedroll 32x32.
  roofs (building.rs): roof_thatch, roof_shingle 16 pieces like the walls (512x32); roof_snow,
      roof_frame 32x32; wall_build_{wood,stone_block} 3 stages (96x32).
Everything is lit from the upper-left; dark outline for furniture so it separates from floors.
"""
import math
import os

from draw import Canvas, rng, sheet
from pal import Palette, ROOT

OUT = os.path.join(ROOT, 'godot', 'assets', 'structures')
T = 32
pal = Palette.load()
P = pal.ramp
N = pal.neutral
BLACK = N('black')
SHADOW = N('shadow')

WOOD = {'d0': P('earth', 1), 'd': P('earth', 2), 'm': P('earth', 3), 'l': P('earth', 4), 'h': P('earth', 5), 'hh': P('earth', 6)}
IRON = {'d': P('stone', 1), 'm': P('stone', 2), 'l': P('stone', 3), 'h': P('stone', 5)}


# ---------------------------------------------------------------- walls
def wall_fill(c, material):
    """Material texture over the whole 32x32 tile; identical on every piece so runs continue.
    These are wall TOPS seen from above, so they must read differently from the floors of
    the same material (planks, slabs): logs lengthwise with bark, rubble stone in mortar,
    packed straw bundles."""
    if material == 'wood':
        # two logs lying along the wall, bark on top, dark seam between them
        c.rect(0, 0, T, T, P('earth', 1))
        r = rng('wall-wood')
        for log in range(2):
            y0 = log * 16
            c.rect(0, y0 + 1, T, 13, P('earth', 3))
            c.hline(0, T - 1, y0 + 1, P('earth', 5))          # lit crown
            c.hline(0, T - 1, y0 + 2, P('earth', 4))
            c.hline(0, T - 1, y0 + 12, P('earth', 2))          # shaded underside
            c.hline(0, T - 1, y0 + 13, P('earth', 1))
            for k in range(5):                                   # bark grain
                x = r.randint(0, T - 7)
                y = y0 + r.randint(4, 10)
                c.hline(x, x + r.randint(3, 6), y, P('earth', 2))
                c.hline(x + 1, x + 3, y + 1, P('earth', 4))
            kx = r.randint(4, T - 6)
            c.stamp(['.dd.', 'dmmd', 'dmld', '.dd.'], kx, y0 + 5, {'d': P('earth', 2), 'm': P('earth', 3), 'l': P('earth', 4)})
    elif material == 'stone_block':
        # fieldstone laid in mortar: rounded stones of several sizes, lit upper-left
        c.rect(0, 0, T, T, P('stone', 2))
        stones = [(0, 0, 13, 9), (14, 0, 31, 7), (0, 10, 8, 20), (9, 8, 22, 19), (23, 8, 31, 20),
                  (0, 21, 15, 31), (16, 20, 31, 31)]
        r = rng('wall-stone')
        for (x0, y0, x1, y1) in stones:
            x0 += 1; y0 += 1
            for y in range(y0, y1):
                for x in range(x0, x1):
                    corner = (x == x0 or x == x1 - 1) and (y == y0 or y == y1 - 1)
                    if corner:
                        continue
                    if y == y0 or x == x0:
                        col = P('stone', 5)
                    elif y == y1 - 1 or x == x1 - 1:
                        col = P('stone', 3)
                    else:
                        col = P('stone', 4)
                    c.px(x, y, col)
            c.px(r.randint(x0 + 1, max(x0 + 1, x1 - 2)), r.randint(y0 + 1, max(y0 + 1, y1 - 2)), P('stone', 3))
    else:  # thatch: irregular straw bundles laid in staggered courses, with broken seams
        c.rect(0, 0, T, T, P('earth', 5))
        r = rng('wall-thatch')
        for row in range(4):
            y0 = row * 8
            offset = (row % 2) * 4
            for x in range(-8, T + 4, 8):
                sx = x + offset + r.randint(-1, 1)
                length = r.randint(5, 8)
                slant = r.choice((-1, 0, 0, 1))
                light = r.choice((5, 6, 6, 7))
                shade = r.choice((3, 4, 4))
                for k in range(length):
                    yy = y0 + 1 + (k + max(0, slant)) // 2
                    c.px(sx + k, yy, P('earth', light if k < length // 2 else shade))
                    if k % 3 != 2:
                        c.px(sx + k, yy + 1, P('earth', shade))
                c.px(sx + length - 1, y0 + 4 + max(0, slant), P('earth', 2))
            # A broken lower edge keeps the courses readable without making the whole wall
            # look like a stack of identical horizontal stripes.
            for x in range(0, T, 4):
                if (x + row * 3) % 7 < 5:
                    c.px(x, y0 + 6, P('earth', 4))
                if (x + row) % 5 < 3:
                    c.px(x + 1, y0 + 7, P('earth', 3))
        for _ in range(4):
            c.px(r.randint(0, 31), r.randint(0, 31), P('earth', 7))


def wall_bevel(c, material, mask):
    ramp = 'earth' if material != 'stone_block' else 'stone'
    base = 4 if material != 'thatch' else 5
    lit, lit2 = P(ramp, min(7, base + 2)), P(ramp, min(7, base + 1))
    dark, dark2 = P(ramp, max(0, base - 3)), P(ramp, max(0, base - 2))
    n, e, s, w = mask & 1, mask & 2, mask & 4, mask & 8
    if not n:
        c.hline(0, T - 1, 0, lit)
        c.hline(0, T - 1, 1, lit2)
    if not w:
        c.vline(0, 0, T - 1, lit)
        c.vline(1, 0, T - 1, lit2)
    if not s:
        c.hline(0, T - 1, T - 1, dark)
        c.hline(0, T - 1, T - 2, dark2)
    if not e:
        c.vline(T - 1, 0, T - 1, dark)
        c.vline(T - 2, 0, T - 1, dark2)
    return c


def wall_sheet(material):
    pieces = []
    for mask in range(16):
        c = Canvas(T, T)
        wall_fill(c, material)
        wall_bevel(c, material, mask)
        pieces.append(c)
    return sheet(pieces, T, T)


# What each wood and stone a wall can be built of looks like (C56): the style of the generic
# wall it follows, the palette ramp, and a step offset (lighter or darker). Names are the
# materials' file names (hf-sim items.rs `file_name`), so the client finds wall_<name>.png.
WALL_LOOKS = {
    'oak': ('wood', 'earth', 0),
    'birch': ('wood', 'earth', 2),
    'pine': ('wood', 'copper', 0),
    'fungus_wood': ('wood', 'cool', 0),
    'granite': ('stone_block', 'stone', 0),
    'basalt': ('stone_block', 'stone', -2),
    'slate': ('stone_block', 'water', -1),
    'marble': ('stone_block', 'stone', 2),
    'limestone': ('stone_block', 'gold', 2),
    'sandstone': ('stone_block', 'warm', 1),
    'bonewood': ('wood', 'stone', 3),
    'milkstone': ('stone_block', 'gold', 3),
    # #267: the deep's regional rocks.
    'gabbro': ('stone_block', 'olive', -3),
    'gneiss': ('stone_block', 'stone', -1),
    # #286: the woods of the valley's other trees.
    'alder': ('wood', 'gold', 0),
    'ash': ('wood', 'stone', 2),
    'yew': ('wood', 'copper', -1),
    'willow': ('wood', 'olive', 1),
    'applewood': ('wood', 'warm', 0),
}


def look_fill(c, style, ramp, off):
    """The generic wall's fill with its colours taken from another ramp, `off` steps lighter."""
    q = lambda k: P(ramp, k + off)
    if style == 'wood':
        c.rect(0, 0, T, T, q(1))
        r = rng('wall-wood')
        for log in range(2):
            y0 = log * 16
            c.rect(0, y0 + 1, T, 13, q(3))
            c.hline(0, T - 1, y0 + 1, q(5))
            c.hline(0, T - 1, y0 + 2, q(4))
            c.hline(0, T - 1, y0 + 12, q(2))
            c.hline(0, T - 1, y0 + 13, q(1))
            for k in range(5):
                x = r.randint(0, T - 7)
                y = y0 + r.randint(4, 10)
                c.hline(x, x + r.randint(3, 6), y, q(2))
                c.hline(x + 1, x + 3, y + 1, q(4))
            kx = r.randint(4, T - 6)
            c.stamp(['.dd.', 'dmmd', 'dmld', '.dd.'], kx, y0 + 5, {'d': q(2), 'm': q(3), 'l': q(4)})
    else:
        c.rect(0, 0, T, T, q(2))
        stones = [(0, 0, 13, 9), (14, 0, 31, 7), (0, 10, 8, 20), (9, 8, 22, 19), (23, 8, 31, 20),
                  (0, 21, 15, 31), (16, 20, 31, 31)]
        r = rng('wall-stone')
        for (x0, y0, x1, y1) in stones:
            x0 += 1; y0 += 1
            for y in range(y0, y1):
                for x in range(x0, x1):
                    corner = (x == x0 or x == x1 - 1) and (y == y0 or y == y1 - 1)
                    if corner:
                        continue
                    if y == y0 or x == x0:
                        col = q(5)
                    elif y == y1 - 1 or x == x1 - 1:
                        col = q(3)
                    else:
                        col = q(4)
                    c.px(x, y, col)
            c.px(r.randint(x0 + 1, max(x0 + 1, x1 - 2)), r.randint(y0 + 1, max(y0 + 1, y1 - 2)), q(3))


def look_bevel(c, ramp, off, mask):
    base = 4 + off
    lit, lit2 = P(ramp, base + 2), P(ramp, base + 1)
    dark, dark2 = P(ramp, base - 3), P(ramp, base - 2)
    n, e, s, w = mask & 1, mask & 2, mask & 4, mask & 8
    if not n:
        c.hline(0, T - 1, 0, lit)
        c.hline(0, T - 1, 1, lit2)
    if not w:
        c.vline(0, 0, T - 1, lit)
        c.vline(1, 0, T - 1, lit2)
    if not s:
        c.hline(0, T - 1, T - 1, dark)
        c.hline(0, T - 1, T - 2, dark2)
    if not e:
        c.vline(T - 1, 0, T - 1, dark)
        c.vline(T - 2, 0, T - 1, dark2)
    return c


def look_sheet(name):
    style, ramp, off = WALL_LOOKS[name]
    pieces = []
    for mask in range(16):
        c = Canvas(T, T)
        look_fill(c, style, ramp, off)
        look_bevel(c, ramp, off, mask)
        pieces.append(c)
    return sheet(pieces, T, T)


def tent():
    """A hide tent for one sleeper (C57), 32x32 seen from above and the south like the houses:
    a ridge of two poles, two hide slopes lit west and shaded east, a dark doorway to the south."""
    c = Canvas(T, T)
    r = rng('tent')
    for y in range(4, 26):
        t = (y - 4) / 21.0
        half = int(round(3 + 10 * t))
        for x in range(16 - half, 16 + half + 1):
            west = x < 16
            col = P('earth', 5) if west else P('earth', 3)
            if (x + y * 2) % 9 == 0:
                col = P('earth', 4) if west else P('earth', 2)   # hide seams
            c.px(x, y, col)
        c.px(16 - half, y, P('earth', 2))                       # outline
        c.px(16 + half, y, P('earth', 1))
    c.vline(16, 3, 25, P('earth', 6))                            # ridge catches the light
    c.rect(13, 19, 7, 7, BLACK)                                  # doorway
    c.rect(14, 20, 5, 2, SHADOW)
    for x in (15, 17):
        c.vline(x, 1, 4, P('earth', 2))                          # pole tips
    c.hline(4, 28, 26, SHADOW)                                   # ground shadow
    for _ in range(5):
        c.px(r.randint(6, 26), r.randint(10, 22), P('earth', 2))
    return c


def nest():
    """A raised woven nest for one sleeper (C57), 32x32: a ring of woven withies on four legs,
    a hollow of moss and down inside, lit from the upper left."""
    c = Canvas(T, T)
    r = rng('nest')
    cx, cy = 16, 14
    for y in range(T):
        for x in range(T):
            d = math.hypot(x - cx, (y - cy) * 1.3)
            if d <= 11:
                if d >= 7.5:
                    k = 4 if (x + y) % 3 else 2
                    if x < cx and y < cy:
                        k += 1
                    c.px(x, y, P('earth', k))
                elif d >= 6.5:
                    c.px(x, y, P('earth', 1))
                else:
                    c.px(x, y, P('olive', 3 + (1 if (x * y) % 5 == 0 else 0)))
    for x in (8, 24):
        for y0 in (21, 23):
            c.vline(x, y0, 28, P('earth', 2))                    # legs
    c.hline(6, 26, 29, SHADOW)
    for _ in range(6):
        c.px(r.randint(11, 21), r.randint(10, 18), N('light'))   # down
    return c


# ---------------------------------------------------------------- openings and stairs
def frame_posts(c, material='wood'):
    """Left and right jamb posts (6 px) of a wall opening, bevelled like a wall end."""
    for x0 in (0, 26):
        c.rect(x0, 0, 6, T, P('earth', 4))
        c.vline(x0, 0, T - 1, P('earth', 6))
        c.vline(x0 + 5, 0, T - 1, P('earth', 1))
        c.hline(x0, x0 + 5, 0, P('earth', 6))
        c.hline(x0, x0 + 5, T - 1, P('earth', 1))
        c.vline(x0 + 2, 2, T - 3, P('earth', 3))


def door_closed():
    c = Canvas(T, T)
    frame_posts(c)
    c.rect(6, 4, 20, 24, P('earth', 3))                 # door leaf, lying across the opening
    for y in range(6, 28, 5):                           # plank seams
        c.hline(6, 25, y, P('earth', 2))
        c.hline(6, 25, y + 1, P('earth', 4))
    c.hline(6, 25, 4, P('earth', 5))
    c.hline(6, 25, 27, P('earth', 1))
    for y in (8, 22):                                   # iron bands
        c.hline(7, 24, y, IRON['m'])
        c.hline(7, 24, y + 1, IRON['d'])
        c.px(8, y, IRON['h'])
        c.px(23, y, IRON['h'])
    c.rect(21, 14, 3, 3, IRON['l'])                     # handle ring
    c.px(22, 15, IRON['d'])
    c.rect(6, 0, 20, 4, SHADOW)                         # the wall gap above/below the leaf
    c.rect(6, 28, 20, 4, SHADOW)
    return c


def door_open():
    c = Canvas(T, T)
    frame_posts(c)
    c.rect(6, 13, 20, 6, SHADOW)                        # threshold shadow strip across the gap
    c.hline(6, 25, 13, BLACK)
    c.hline(6, 25, 18, P('stone', 2))
    # leaf swung open, hinged on the west post, standing north of the opening: a plank
    # 5 px wide running from the hinge up to the tile top, with its iron bands end-on
    c.rect(7, 0, 5, 14, P('earth', 3))
    c.vline(7, 0, 13, P('earth', 5))
    c.vline(11, 0, 13, P('earth', 1))
    c.hline(7, 11, 13, P('earth', 1))
    c.hline(7, 11, 0, P('earth', 5))
    for y in (3, 9):
        c.hline(8, 10, y, IRON['l'])
    c.px(9, 6, IRON['h'])                               # handle
    c.rect(12, 1, 2, 12, SHADOW, 150)                   # the leaf's cast shadow
    return c


def window():
    c = Canvas(T, T)
    wall_fill(c, 'wood')
    wall_bevel(c, 'wood', 10)                           # a wall running E-W
    c.rect(8, 10, 16, 12, P('earth', 1))                # sill box
    c.rect(10, 12, 12, 8, P('cool', 6))                 # glass
    c.rect(10, 12, 6, 4, P('cool', 7))                  # pane glint
    c.vline(15, 12, 19, P('earth', 2))                  # mullion
    c.hline(10, 21, 15, P('earth', 2))
    c.hline(8, 23, 10, P('earth', 5))
    return c


def stairs(kind):
    """A stone stairwell seen from above. down: the treads shrink and darken toward the south
    (into the dark), with a black drop at the bottom and a lit top lip. up: treads widen and
    brighten toward the north, with a lit landing and a dark riser face at the south edge.
    both: the west half goes down and the east half goes up, split by a wooden rail."""
    c = Canvas(T, T)
    c.rect(0, 0, T, T, P('stone', 2))                   # stone surround
    c.hline(0, T - 1, 0, P('stone', 4))
    c.vline(0, 0, T - 1, P('stone', 4))
    c.hline(0, T - 1, T - 1, P('stone', 0))
    c.vline(T - 1, 0, T - 1, P('stone', 0))

    def flight(x0, x1, down):
        w = x1 - x0 + 1
        c.rect(x0, 2, w, 28, BLACK)
        for i in range(6):
            y0 = 3 + i * 4
            if down:
                inset = i                                # narrows going south
                shade = 5 - i
            else:
                inset = 5 - i                            # widens going south toward the viewer
                shade = 1 + i
            shade = max(0, min(6, shade))
            c.rect(x0 + inset, y0, w - 2 * inset, 3, P('stone', shade))
            c.hline(x0 + inset, x1 - inset, y0, P('stone', min(7, shade + 2)))
            c.hline(x0 + inset, x1 - inset, y0 + 2, P('stone', max(0, shade - 2)))
        if down:
            c.hline(x0, x1, 2, P('stone', 6))            # lit top lip of the well
            c.rect(x0 + 6, 27, w - 12, 3, BLACK)
        else:
            c.rect(x0, 27, w, 3, P('stone', 1))          # bottom riser in shadow
            c.hline(x0, x1, 2, P('stone', 7))            # lit landing edge

    if kind == 'down':
        flight(3, 28, True)
    elif kind == 'up':
        flight(3, 28, False)
    else:
        flight(2, 13, True)
        flight(18, 29, False)
        c.rect(14, 1, 4, 30, P('earth', 3))             # central rail
        c.vline(14, 1, 30, P('earth', 5))
        c.vline(17, 1, 30, P('earth', 1))
    return c


def ladder():
    c = Canvas(T, T)
    c.rect(6, 2, 20, 28, BLACK)                         # shaft
    c.rect(4, 0, 2, T, P('stone', 2))
    c.rect(26, 0, 2, T, P('stone', 2))
    c.hline(4, 27, 0, P('stone', 3))
    c.hline(4, 27, T - 1, P('stone', 0))
    for x in (10, 20):                                  # rails
        c.vline(x, 1, 30, P('earth', 4))
        c.vline(x + 1, 1, 30, P('earth', 2))
    for y in range(4, 30, 5):                           # rungs
        c.hline(11, 19, y, P('earth', 5))
        c.hline(11, 19, y + 1, P('earth', 2))
    return c


# ---------------------------------------------------------------- furniture
def outlined(c):
    c.outline(BLACK)
    return c


def ground_shadow(c, cx=16, cy=28, rx=11, ry=2):
    """A soft, palette-safe contact shadow that anchors small props on any floor."""
    c.disc(cx, cy, rx, ry, SHADOW, 120)


def wood_marks(c, marks, dark=None, light=None):
    """Add deterministic, sparse grain marks without turning a prop into noise."""
    dark = dark or WOOD['d']
    light = light or WOOD['h']
    for x, y, length in marks:
        c.hline(x, min(31, x + length), y, dark)
        if y > 0 and length > 2:
            c.px(x + 1, y - 1, light)


def bed():
    c = Canvas(T, T)
    c.rect(5, 3, 22, 27, P('earth', 2))                 # frame
    c.rect(6, 4, 20, 25, P('earth', 3))
    c.rect(6, 4, 20, 1, P('earth', 5))                  # headboard lit edge
    c.rect(7, 9, 18, 19, P('cool', 3))                  # blanket
    c.rect(7, 9, 18, 2, P('cool', 5))                   # folded top of the blanket
    c.hline(7, 24, 11, P('cool', 2))
    for y in range(14, 27, 4):                          # blanket weave stripes
        c.hline(8, 23, y, P('cool', 4))
    c.rect(7, 27, 18, 1, P('cool', 1))
    c.rect(9, 5, 14, 4, N('light'))                     # pillow
    c.hline(9, 22, 8, P('stone', 5))
    c.px(9, 5, P('stone', 6))
    c.px(22, 5, P('stone', 6))
    c.vline(26, 5, 29, P('earth', 1))                   # frame dark side
    c.hline(6, 26, 29, P('earth', 1))
    return outlined(c)


def table():
    c = Canvas(T, T)
    c.rect(3, 6, 26, 18, P('earth', 4))                 # top
    c.rect(3, 6, 26, 1, P('earth', 6))
    c.vline(3, 6, 23, P('earth', 5))
    for y in (11, 16, 21):                              # plank seams
        c.hline(4, 28, y, P('earth', 3))
    c.hline(3, 28, 23, P('earth', 2))
    c.vline(28, 6, 23, P('earth', 2))
    for x in (4, 26):                                   # legs peeking below the top
        c.rect(x, 24, 2, 5, P('earth', 2))
        c.px(x, 24, P('earth', 3))
    c.stamp(['.d.', 'dmd', '.d.'], 18, 13, {'d': P('earth', 3), 'm': P('earth', 2)})
    return outlined(c)


def chair():
    c = Canvas(T, T)
    c.rect(9, 5, 14, 4, P('earth', 3))                  # back rest: a bar on two posts (north)
    c.hline(9, 22, 5, P('earth', 5))
    c.hline(9, 22, 8, P('earth', 1))
    for x in (9, 22):
        c.rect(x, 5, 2, 7, P('earth', 2))
        c.px(x, 5, P('earth', 4))
    c.rect(10, 12, 12, 10, P('earth', 4))               # seat, smaller than the table top
    c.hline(10, 21, 12, P('earth', 6))
    c.vline(10, 12, 21, P('earth', 5))
    c.hline(10, 21, 21, P('earth', 2))
    c.vline(21, 12, 21, P('earth', 2))
    c.hline(12, 19, 17, P('earth', 3))                  # seat plank seam
    for x in (10, 20):                                  # front legs showing below the seat
        c.rect(x, 22, 2, 5, P('earth', 2))
        c.px(x, 22, P('earth', 3))
    return outlined(c)


def stool():
    c = Canvas(T, T)
    c.disc(16, 15, 8, 6, P('earth', 3))                 # rim (dark side)
    c.disc(16, 14, 7, 5, P('earth', 4))
    c.disc(14, 12, 4, 3, P('earth', 5))                 # lit top
    c.px(13, 11, P('earth', 6))
    c.hline(10, 22, 20, P('earth', 2))
    for x in (9, 15, 21):                               # three legs
        c.rect(x, 21, 2, 6, P('earth', 2))
        c.px(x, 21, P('earth', 3))
    return outlined(c)


def chest():
    c = Canvas(T, T)
    ground_shadow(c, 16, 27, 10, 2)
    c.rect(5, 10, 22, 15, P('earth', 2))                # deep body
    c.rect(6, 9, 20, 7, P('earth', 4))                  # overhanging lid
    c.hline(6, 25, 9, P('earth', 6))
    c.vline(6, 9, 24, P('earth', 5))
    c.hline(6, 25, 15, P('earth', 1))                   # lid edge
    c.hline(5, 26, 24, P('earth', 1))
    c.vline(26, 10, 24, P('earth', 2))
    wood_marks(c, [(8, 12, 4), (18, 11, 5), (9, 19, 6), (18, 21, 4), (12, 23, 3)])
    for x in (8, 22):                                   # forged straps and corner caps
        c.vline(x, 10, 23, IRON['m'])
        c.vline(x + 1, 10, 23, IRON['d'])
        c.rect(x - 1, 10, 3, 2, IRON['l'])
        c.rect(x - 1, 22, 3, 2, IRON['d'])
    c.rect(14, 14, 4, 4, IRON['l'])                     # lock plate
    c.rect(15, 15, 2, 3, IRON['d'])
    c.px(16, 15, IRON['h'])
    return outlined(c)


def shelf():
    c = Canvas(T, T)
    c.rect(4, 3, 24, 26, P('earth', 3))
    c.rect(4, 3, 24, 1, P('earth', 5))
    c.vline(4, 3, 28, P('earth', 4))
    for y in (11, 19):                                  # shelf boards
        c.hline(5, 26, y, P('earth', 5))
        c.hline(5, 26, y + 1, P('earth', 1))
    c.hline(4, 27, 28, P('earth', 1))
    c.vline(27, 3, 28, P('earth', 2))
    # stored things: jars, a folded cloth, a sack
    c.rect(7, 6, 3, 5, P('cool', 4))
    c.px(7, 6, P('cool', 6))
    c.rect(12, 7, 4, 4, P('warm', 3))
    c.px(12, 7, P('warm', 5))
    c.rect(19, 5, 5, 6, P('stone', 5))
    c.px(19, 5, P('stone', 7))
    c.rect(7, 14, 7, 5, P('warm', 5))
    c.hline(7, 13, 16, P('warm', 4))
    c.rect(17, 13, 6, 6, P('earth', 5))
    c.px(19, 13, P('earth', 3))
    c.rect(8, 22, 5, 6, P('foliage', 4))
    c.rect(16, 23, 8, 5, P('earth', 4))
    c.hline(16, 23, 23, P('earth', 6))
    return outlined(c)


def hearth(frame):
    c = Canvas(T, T)
    # stone ring
    ring = [(9, 6), (15, 4), (21, 6), (25, 12), (25, 19), (21, 25), (15, 27), (9, 25), (5, 19), (5, 12)]
    c.disc(15.5, 16, 10, 10, P('stone', 1))             # ash pit
    c.disc(15.5, 16.5, 7, 7, SHADOW)
    for (x, y) in ring:
        c.rect(x - 2, y - 2, 5, 4, P('stone', 4))
        c.hline(x - 2, x + 2, y - 2, P('stone', 6))
        c.hline(x - 2, x + 2, y + 1, P('stone', 2))
        c.px(x + 2, y, P('stone', 2))
    # embers
    c.stamp(['.e.', 'eEe', '.e.'], 14, 15, {'e': P('warm', 2), 'E': P('warm', 3)})
    # fire: three frames, different tongue heights
    tongues = {
        0: [(12, 20, 8), (16, 22, 11), (19, 19, 7)],
        1: [(13, 21, 10), (16, 22, 8), (18, 20, 9)],
        2: [(12, 19, 7), (15, 22, 12), (19, 21, 9)],
    }[frame]
    for (x, base_y, h) in tongues:
        for k in range(h):
            y = base_y - k
            w = max(1, 3 - (k * 3) // h)
            col = P('warm', 3) if k < h * 0.3 else (P('warm', 5) if k < h * 0.7 else P('warm', 6))
            c.rect(x - w // 2, y, w, 1, col)
        c.px(x, base_y - h, P('warm', 7))
    c.rect(14, 19, 4, 3, P('warm', 6))                  # hot core
    c.px(15 + frame % 2, 20, P('warm', 7))
    c.px(10 + frame * 3, 9 + (frame % 2) * 2, P('warm', 6))     # spark
    return c


def workbench():
    c = Canvas(T, T)
    ground_shadow(c, 16, 28, 13, 2)
    # thick top, apron, and two trestles make the work surface legible at 1x
    c.rect(2, 7, 28, 5, P('earth', 3))
    c.hline(2, 29, 7, P('earth', 6))
    c.hline(2, 29, 11, P('earth', 1))
    wood_marks(c, [(4, 9, 6), (14, 8, 5), (22, 10, 4)])
    c.rect(4, 12, 24, 10, P('earth', 4))
    c.hline(4, 27, 12, P('earth', 5))
    c.hline(4, 27, 21, P('earth', 2))
    for x in (5, 24):
        c.rect(x, 20, 4, 8, P('earth', 2))
        c.vline(x, 20, 27, P('earth', 5))
        c.vline(x + 3, 20, 27, P('earth', 1))
        c.hline(x - 1, x + 4, 27, P('earth', 1))
    # vice, mallet, square, and a plank in progress
    c.rect(4, 9, 7, 4, IRON['m'])
    c.hline(4, 10, 9, IRON['h'])
    c.rect(8, 12, 3, 2, IRON['d'])
    c.line(14, 9, 21, 12, WOOD['d'])                       # mallet handle
    c.rect(19, 8, 5, 4, WOOD['m'])
    c.hline(19, 23, 8, WOOD['h'])
    c.line(25, 9, 28, 12, IRON['l'])                       # carpenter's square
    c.rect(13, 15, 9, 4, P('earth', 6))
    c.hline(13, 21, 15, P('earth', 7))
    c.hline(13, 21, 18, P('earth', 3))
    c.stamp(['s.s.', '.s..', '...s'], 24, 16, {'s': P('earth', 6)})
    return outlined(c)


def loom():
    c = Canvas(T, T)
    c.rect(4, 3, 24, 4, P('earth', 3))                  # top beam
    c.hline(4, 27, 3, P('earth', 5))
    c.rect(4, 26, 24, 3, P('earth', 3))                 # bottom beam
    c.hline(4, 27, 28, P('earth', 1))
    for x in (4, 26):                                   # uprights
        c.rect(x, 3, 2, 26, P('earth', 2))
        c.px(x, 3, P('earth', 4))
    for x in range(8, 25, 2):                           # warp threads
        c.vline(x, 7, 25, P('stone', 6))
    c.rect(7, 16, 18, 9, P('warm', 4))                  # woven cloth so far
    for y in range(17, 25, 2):
        c.hline(7, 24, y, P('warm', 5))
    c.hline(7, 24, 16, P('warm', 6))
    c.rect(6, 12, 20, 2, P('earth', 4))                 # heddle bar
    c.hline(6, 25, 12, P('earth', 6))
    c.rect(11, 9, 3, 2, P('earth', 6))                  # shuttle
    return outlined(c)


def anvil():
    c = Canvas(T, T)
    c.rect(8, 22, 16, 5, IRON['d'])                     # base block
    c.rect(10, 23, 12, 3, IRON['m'])
    c.rect(11, 16, 10, 6, IRON['m'])                    # waist
    c.vline(11, 16, 21, IRON['l'])
    c.rect(6, 10, 22, 6, IRON['l'])                     # face
    c.hline(6, 27, 10, IRON['h'])
    c.hline(6, 27, 15, IRON['d'])
    c.rect(2, 11, 4, 3, IRON['l'])                      # horn to the west
    c.hline(2, 5, 11, IRON['h'])
    c.px(2, 12, IRON['h'])
    c.rect(25, 11, 3, 2, IRON['m'])                     # heel
    c.px(26, 12, IRON['d'])
    c.hline(8, 23, 27, P('stone', 0))
    return outlined(c)


def carpenter_bench():
    """A sawhorse bench with a plank across it and a saw resting on top."""
    c = Canvas(T, T)
    ground_shadow(c, 16, 28, 13, 2)
    c.rect(2, 13, 28, 6, WOOD['m'])                     # the thick plank
    c.hline(2, 29, 13, WOOD['h'])
    c.hline(2, 29, 18, WOOD['d0'])
    wood_marks(c, [(4, 15, 5), (14, 14, 6), (22, 16, 4)], WOOD['d'], WOOD['hh'])
    for x in (5, 23):                                   # broad trestle legs
        c.rect(x, 19, 3, 8, WOOD['d'])
        c.vline(x, 19, 26, WOOD['h'])
        c.vline(x + 2, 19, 26, WOOD['d0'])
        c.rect(x - 2, 25, 7, 3, WOOD['d0'])
        c.hline(x - 1, x + 3, 25, WOOD['m'])
    c.hline(7, 25, 22, WOOD['d'])                    # stretcher between trestles
    c.px(8, 22, WOOD['h']); c.px(24, 22, WOOD['h'])
    c.rect(10, 8, 12, 3, IRON['l'])                     # saw blade
    c.hline(10, 21, 10, IRON['h'])
    c.hline(10, 21, 11, IRON['m'])
    for x in range(10, 22, 2):
        c.px(x, 12, IRON['d'])
    c.rect(22, 7, 5, 5, WOOD['m'])                      # saw handle
    c.px(23, 8, WOOD['h']); c.px(25, 10, WOOD['d0'])
    c.rect(6, 10, 3, 3, IRON['m'])                      # clamp
    c.px(7, 10, IRON['h'])
    c.hline(2, 29, 27, P('stone', 0))
    return outlined(c)


def smelter():
    """A squat stone furnace with a mouth that glows; two frames (cold, lit)."""
    def one(lit):
        c = Canvas(T, T)
        c.rect(7, 6, 18, 21, P('stone', 3))             # body
        c.hline(7, 24, 6, P('stone', 5))
        c.vline(7, 6, 26, P('stone', 4))
        c.vline(24, 6, 26, P('stone', 2))
        for y in range(8, 26, 4):                       # courses
            c.hline(8, 23, y, P('stone', 2))
        c.rect(11, 2, 10, 4, P('stone', 2))             # chimney
        c.hline(11, 20, 2, P('stone', 4))
        c.rect(12, 18, 8, 7, P('stone', 0))             # mouth
        if lit:
            c.rect(13, 19, 6, 5, P('warm', 2))
            c.rect(14, 20, 4, 3, P('warm', 4))
            c.rect(15, 21, 2, 1, P('warm', 6))
            c.px(15, 0, P('stone', 1)); c.px(16, 1, P('stone', 1))   # smoke
        c.hline(6, 25, 27, P('stone', 0))
        return outlined(c)
    return sheet([one(False), one(True)], T, T)


def fire(frame):
    """Flames on a tile: four frames, tongues rising and falling out of phase from a glowing
    bed of embers, a hot pale core low down, sparks above. Lit from within."""
    c = Canvas(T, T)
    r = rng('fire-%d' % frame)
    base_y = 27
    for x in range(5, 27):                               # ember bed
        c.px(x, base_y, P('warm', 2 + (x * 7 + frame) % 3))
        c.px(x, base_y + 1, N('black'))
        c.px(x, base_y + 2, P('warm', 0))
    phase = [0.0, 1.6, 3.1, 4.7][frame % 4]
    for k, tx in enumerate((7, 11, 15, 19, 23)):
        th = int(round(9 + 5 * math.sin(phase + k * 1.9) + (3 if k == 2 else 0)))
        lean = int(round(math.sin(phase * 0.7 + k) * 1.2))
        for j in range(th):
            y = base_y - 1 - j
            w_ = max(0, (th - j) // 3 + (1 if j < th * 0.4 else 0))
            x0 = tx + (lean * j) // max(1, th)
            for dx in range(-w_, w_ + 1):
                hot = j < th * 0.35 and abs(dx) < max(1, w_)
                col = P('warm', 7) if hot and k in (1, 2, 3) else (
                    P('warm', 2) if abs(dx) == w_ else (P('warm', 4) if j < th * 0.65 else P('warm', 6)))
                c.px(x0 + dx, y, col)
        c.px(tx + lean, base_y - th - 1, P('warm', 6))
    for _ in range(4):                                   # sparks
        c.px(r.randint(6, 25), r.randint(1, 10), P('warm', 6 + r.randint(0, 1)))
    return c


def furnace():
    """A low forge hearth with a coal bed and bellows; two frames (cold, lit)."""
    def one(lit):
        c = Canvas(T, T)
        c.rect(4, 14, 24, 12, P('stone', 3))            # hearth body
        c.hline(4, 27, 14, P('stone', 5))
        c.vline(4, 14, 25, P('stone', 4))
        c.vline(27, 14, 25, P('stone', 2))
        for y in range(16, 25, 3):                      # stone courses
            c.hline(5, 26, y, P('stone', 2))
        c.rect(9, 18, 14, 6, P('stone', 0))             # coal bed
        if lit:
            c.rect(10, 19, 12, 4, P('warm', 2))
            c.rect(12, 20, 8, 2, P('warm', 4))
            c.rect(14, 20, 4, 1, P('warm', 6))
            for i in range(3):                          # sparks over the coals
                c.px(11 + i * 4, 17, P('warm', 5))
        c.rect(2, 10, 7, 4, WOOD['m'])                  # bellows to the west
        c.hline(2, 8, 10, WOOD['h'])
        c.hline(2, 8, 13, WOOD['d'])
        c.vline(2, 10, 13, WOOD['d'])
        c.rect(24, 9, 4, 5, IRON['m'])                  # flue stack
        c.hline(24, 27, 9, IRON['l'])
        c.hline(3, 28, 26, P('stone', 0))
        return outlined(c)
    return sheet([one(False), one(True)], T, T)


def kiln():
    """A beehive clay kiln for firing pots; two frames (cold, lit)."""
    def one(lit):
        c = Canvas(T, T)
        c.rect(6, 8, 20, 22, P('earth', 4))             # domed clay body
        c.rect(7, 6, 18, 4, P('earth', 3))              # crown of the dome
        c.hline(7, 24, 6, P('earth', 5))
        c.vline(6, 8, 29, P('earth', 5))
        c.vline(25, 8, 29, P('earth', 2))
        for y in range(11, 29, 4):                      # clay courses
            c.hline(7, 24, y, P('earth', 2))
        c.rect(11, 20, 10, 9, P('earth', 0))            # firebox mouth
        c.hline(11, 20, 20, P('earth', 1))
        if lit:
            c.rect(12, 21, 8, 7, P('warm', 2))
            c.rect(13, 23, 6, 4, P('warm', 4))
            c.rect(15, 25, 2, 1, P('warm', 6))
        c.rect(14, 2, 4, 4, P('earth', 2))              # chimney flue
        c.hline(14, 17, 2, P('earth', 4))
        if lit:
            c.px(15, 1, P('stone', 1))                  # smoke
            c.px(16, 0, P('stone', 1))
        c.hline(5, 26, 29, P('earth', 0))
        return outlined(c)
    return sheet([one(False), one(True)], T, T)


def cooking_pot(frame):
    c = Canvas(T, T)
    # tripod legs
    for (x0, y0, x1, y1) in ((11, 12, 7, 27), (21, 12, 25, 27), (16, 12, 16, 28)):
        c.line(x0, y0, x1, y1, IRON['d'])
    # fire under it
    c.rect(11, 24, 10, 3, P('warm', 3))
    c.rect(12, 23, 8, 1, P('warm', 5))
    c.px(14 + frame * 3, 22, P('warm', 6))
    c.px(13 + frame * 2, 21, P('warm', 7))
    # pot body
    c.disc(16, 15, 8, 6, IRON['m'])
    c.disc(15, 14, 6, 4, IRON['l'])
    c.hline(9, 23, 20, IRON['d'])
    c.rect(8, 9, 16, 3, IRON['l'])                      # rim
    c.hline(8, 23, 9, IRON['h'])
    c.hline(9, 22, 11, IRON['d'])
    c.rect(10, 10, 12, 1, P('warm', 4) if frame == 0 else P('warm', 5))  # stew inside
    c.line(8, 9, 11, 5, IRON['l'])                      # handle arc
    c.line(23, 9, 20, 5, IRON['l'])
    c.hline(11, 20, 5, IRON['l'])
    # steam
    sy = 2 + frame
    c.px(14, sy, P('stone', 6))
    c.px(15, sy - 1, P('stone', 7))
    c.px(18, sy + 1 - frame, P('stone', 6))
    return outlined(c)


def barrel():
    c = Canvas(T, T)
    ground_shadow(c, 16, 27, 9, 2)
    c.disc(16, 16, 10, 9, P('earth', 2))
    c.disc(16, 16, 8, 8, P('earth', 4))
    c.hline(9, 23, 9, P('earth', 6))
    c.vline(8, 11, 21, P('earth', 5))
    c.vline(23, 11, 21, P('earth', 2))
    for x, col in ((11, WOOD['d']), (14, WOOD['m']), (17, WOOD['d']), (20, WOOD['m'])):
        c.vline(x, 10, 22, col)                              # individual staves
    c.hline(8, 24, 12, IRON['m'])                            # hoops
    c.hline(8, 24, 20, IRON['m'])
    c.hline(9, 23, 13, IRON['l'])
    c.hline(9, 23, 21, IRON['d'])
    c.px(8, 12, IRON['h'])
    c.px(23, 12, IRON['d'])
    c.px(8, 20, IRON['h'])
    c.px(23, 20, IRON['d'])
    c.disc(16, 16, 2, 2, P('earth', 2))                       # bung
    c.px(15, 15, P('earth', 6))
    return outlined(c)


def crate():
    c = Canvas(T, T)
    ground_shadow(c, 16, 28, 12, 2)
    c.rect(4, 5, 24, 22, P('earth', 2))
    c.rect(6, 6, 20, 19, P('earth', 4))
    c.hline(5, 26, 5, P('earth', 6))
    c.vline(5, 5, 26, P('earth', 5))
    c.hline(5, 26, 26, P('earth', 1))
    c.vline(26, 5, 26, P('earth', 2))
    # staggered slats, battens, nails, and a dark gap make this read as a real crate
    for y in (10, 16, 22):
        c.hline(6, 25, y, P('earth', 2))
        c.hline(10, 21, y - 1, P('earth', 5))
    c.rect(7, 6, 3, 20, P('earth', 3))
    c.rect(22, 6, 3, 20, P('earth', 3))
    c.vline(7, 6, 25, P('earth', 5))
    c.vline(22, 6, 25, P('earth', 1))
    for x, y in ((8, 8), (23, 8), (8, 24), (23, 24)):
        c.px(x, y, IRON['h'])
        c.px(x + (1 if x < 16 else -1), y + 1, IRON['d'])
    c.rect(13, 13, 6, 2, P('earth', 1))
    c.px(14, 13, P('earth', 6))
    return outlined(c)


def well():
    c = Canvas(T, T)
    ground_shadow(c, 16, 28, 12, 2)
    # two-post draw well, with a deep shaft and individually readable masonry
    c.disc(16, 18, 11, 9, P('stone', 2))                # outer ring shadow
    c.disc(16, 17, 10, 8, P('stone', 4))
    c.disc(16, 18, 7, 5, BLACK)                         # shaft
    c.disc(16, 19, 5, 3, P('water', 1))                 # water deep down
    c.hline(13, 18, 18, P('water', 3))
    c.px(15, 17, P('water', 5))
    # ring stones with lit crowns and dark joints
    for ang in range(0, 360, 30):
        x = int(round(16 + 9.2 * math.cos(math.radians(ang))))
        y = int(round(17 + 7.2 * math.sin(math.radians(ang))))
        top = 6 if ang < 180 else 5
        c.rect(x - 2, y - 1, 4, 3, P('stone', 3))
        c.hline(x - 2, x + 1, y - 1, P('stone', top))
        c.px(x + 1, y + 1, P('stone', 1))
    # posts and crossbar with pulley, rope, and a suspended bucket
    c.rect(4, 2, 3, 16, P('earth', 2))
    c.vline(4, 2, 17, P('earth', 5))
    c.rect(25, 2, 3, 16, P('earth', 2))
    c.vline(25, 2, 17, P('earth', 5))
    c.rect(4, 2, 24, 3, P('earth', 4))
    c.hline(4, 27, 2, P('earth', 6))
    c.disc(16, 6, 3, 3, IRON['d'])
    c.px(15, 5, IRON['h'])
    c.vline(16, 7, 13, P('earth', 6))
    c.rect(13, 12, 7, 5, WOOD['d'])
    c.hline(13, 19, 12, WOOD['h'])
    c.hline(13, 19, 16, WOOD['d0'])
    return outlined(c)


def fence_post():
    c = Canvas(T, T)
    c.rect(13, 8, 6, 18, P('earth', 3))
    c.vline(13, 8, 25, P('earth', 5))
    c.vline(18, 8, 25, P('earth', 1))
    c.rect(13, 8, 6, 2, P('earth', 5))
    c.hline(13, 18, 25, P('earth', 1))
    c.disc(16, 27, 5, 2, SHADOW)                        # ground shadow
    c.rect(13, 8, 6, 18, P('earth', 3))
    c.vline(13, 8, 25, P('earth', 5))
    c.vline(18, 8, 25, P('earth', 1))
    c.hline(13, 18, 8, P('earth', 6))
    c.px(15, 14, P('earth', 2))
    c.px(16, 19, P('earth', 2))
    return outlined(c)


def support():
    """A timber prop: two posts under a cap beam with angled braces, wedged at the foot, the
    way a gallery roof is held. Rough-hewn, darker where the wood has taken the damp."""
    c = Canvas(T, T)
    c.disc(16, 28, 11, 2, SHADOW)                       # ground shadow
    for x in (7, 22):                                   # posts
        c.rect(x, 7, 4, 20, P('earth', 3))
        c.vline(x, 7, 26, P('earth', 5))
        c.vline(x + 3, 7, 26, P('earth', 1))
        c.px(x + 1, 13, P('earth', 2))
        c.px(x + 2, 20, P('earth', 2))
        c.rect(x - 1, 25, 6, 2, P('earth', 2))          # wedge at the foot
        c.hline(x - 1, x + 4, 25, P('earth', 4))
    c.rect(4, 4, 24, 4, P('earth', 4))                  # cap beam
    c.hline(4, 27, 4, P('earth', 6))
    c.hline(4, 27, 7, P('earth', 1))
    c.px(9, 5, P('earth', 2))
    c.px(20, 6, P('earth', 2))
    for k in range(5):                                  # braces from post to beam
        c.px(11 + k, 12 - k, P('earth', 3))
        c.px(11 + k, 13 - k, P('earth', 1))
        c.px(21 - k, 12 - k, P('earth', 3))
        c.px(21 - k, 13 - k, P('earth', 1))
    return outlined(c)


def lamp(lit):
    c = Canvas(T, T)
    ground_shadow(c, 16, 28, 7, 1)
    c.rect(14, 10, 4, 16, IRON['d'])                    # post
    c.vline(14, 10, 25, IRON['l'])
    c.rect(11, 25, 10, 3, IRON['m'])                    # foot
    c.hline(11, 20, 25, IRON['h'])
    c.px(12, 27, IRON['d']); c.px(19, 27, IRON['d'])
    c.rect(10, 4, 12, 7, IRON['d'])                     # lantern housing
    c.hline(10, 21, 4, IRON['h'])
    c.vline(10, 4, 10, IRON['l'])
    c.vline(21, 4, 10, IRON['m'])
    pane = P('warm', 6) if lit else P('cool', 1)
    c.rect(12, 6, 8, 4, pane)
    c.vline(12, 6, 9, IRON['l']); c.vline(19, 6, 9, IRON['d'])
    c.vline(15, 6, 9, P('warm', 7) if lit else P('cool', 2))
    c.px(17, 7, P('warm', 7) if lit else P('cool', 2))
    c.rect(14, 2, 4, 2, IRON['l'])                      # cap
    c.hline(13, 18, 1, IRON['d'])
    c.px(15, 1, IRON['h'])
    c.line(12, 3, 11, 2, IRON['m'])                     # carry handle
    c.line(19, 3, 20, 2, IRON['m'])
    c = outlined(c)
    if lit:                                             # glow halo in palette warm colours, low alpha
        for (rad, a) in ((12, 60), (8, 110), (5, 150)):
            g = Canvas(T, T)
            g.disc(16, 8, rad, rad, P('warm', 6), a)
            for y in range(T):
                for x in range(T):
                    if g.a[y, x, 3] and c.a[y, x, 3] == 0:
                        c.px(x, y, P('warm', 6), int(g.a[y, x, 3]))
    return c


def stockpile():
    c = Canvas(T, T)
    # A stockpile is a designation, not a solid floor: keep the field transparent while
    # adding a low, readable pallet and three tiny goods silhouettes inside the boundary.
    shadow = P('stone', 2)
    col = P('stone', 7)
    for i in range(0, T, 4):                            # dashed border
        for k in range(2):
            c.px(i + k, 0, col, 160)
            c.px(i + k, T - 1, col, 160)
            c.px(0, i + k, col, 160)
            c.px(T - 1, i + k, col, 160)
    c.px(0, 0, col, 200)
    c.rect(7, 22, 18, 3, shadow, 120)                  # pallet shadow
    c.hline(7, 24, 22, P('earth', 5))
    c.hline(7, 24, 24, P('earth', 2))
    c.vline(9, 22, 24, P('earth', 3)); c.vline(21, 22, 24, P('earth', 3))
    c.rect(9, 13, 5, 7, P('earth', 4))                 # bundled goods
    c.hline(9, 13, 13, P('earth', 6)); c.vline(11, 13, 19, P('earth', 2))
    c.rect(15, 15, 5, 5, P('foliage', 4))              # produce sack
    c.px(16, 15, P('foliage', 6)); c.px(18, 16, P('foliage', 2))
    c.disc(23, 18, 3, 3, P('stone', 4))                # stone on the pallet
    c.px(22, 17, P('stone', 6))
    return c


def lean_to():
    """A strangers' shelter, 48x40, seen from the south like the houses: a sloped roof of hide
    and cut branches on a ridge pole (lit from above), and under its eave the open, dark front
    between two forked poles. Rough and patched: not something the settlement built."""
    c = Canvas(48, 40)
    r = rng('lean-to')
    # roof, seen from above: a trapezoid from the ridge (y=5) down to the eave (y=24)
    for y in range(5, 25):
        t = (y - 5) / 19.0
        x0 = int(round(9 - 4 * t))
        x1 = int(round(38 + 4 * t))
        for x in range(x0, x1 + 1):
            col = P('earth', 4) if y < 10 else P('earth', 3)
            if (x * 3 + y) % 11 == 0:
                col = P('earth', 2)                 # hide seams
            if x <= x0 + 1:
                col = P('earth', 5)                 # lit west edge
            elif x >= x1 - 1:
                col = P('earth', 2)
            c.px(x, y, col)
    c.hline(10, 37, 5, P('earth', 6))               # ridge catches the light
    # branches laid on the roof, inside its outline, a few with leaves
    for k in range(6):
        x = 11 + k * 5 + r.randint(-1, 1)
        c.line(x, 23, x + 2, 7, P('earth', 1))
        c.line(x + 1, 23, x + 3, 7, P('earth', 5))
        if k % 2 == 1:
            for _ in range(4):
                c.px(x + r.randint(1, 3), r.randint(8, 20), P('foliage', 2 + r.randint(0, 2)))
    c.rect(28, 12, 5, 4, P('earth', 2))             # a patch of darker hide
    c.hline(28, 32, 12, P('earth', 4))
    # eave: a dark lip, then the open front, deep in shadow
    c.hline(5, 42, 25, P('earth', 1))
    c.rect(7, 26, 34, 8, BLACK)
    c.rect(9, 26, 30, 2, SHADOW)
    c.rect(14, 30, 12, 3, P('cool', 1))             # a blanket inside, just visible
    # forked poles at the front corners
    for x in (7, 40):
        c.vline(x, 24, 35, P('earth', 2))
        c.vline(x + 1, 24, 35, P('earth', 4))
    c.hline(4, 44, 36, SHADOW)                       # ground shadow
    return outlined(c)


def campfire(frame):
    """A small fire on bare ground: sticks laid across, no stone ring; three frames."""
    c = Canvas(T, T)
    c.disc(15.5, 22, 9, 5, P('earth', 1))              # scorched earth
    c.disc(15.5, 22, 6, 3, SHADOW)
    for (x0, y0, x1, y1) in ((7, 25, 24, 19), (8, 19, 24, 25), (15, 26, 16, 17)):
        c.line(x0, y0, x1, y1, P('earth', 2))
        c.line(x0, y0 - 1, x1, y1 - 1, P('earth', 4))
    c.stamp(['.e.', 'eEe', '.e.'], 14, 21, {'e': P('warm', 2), 'E': P('warm', 4)})
    tongues = {
        0: [(13, 22, 6), (16, 23, 9), (18, 22, 5)],
        1: [(14, 22, 8), (16, 23, 6), (18, 22, 7)],
        2: [(13, 22, 5), (15, 23, 10), (18, 22, 6)],
    }[frame]
    for (x, base_y, h) in tongues:
        for k in range(h):
            col = P('warm', 3) if k < h * 0.3 else (P('warm', 5) if k < h * 0.7 else P('warm', 6))
            w = 2 if k < h * 0.5 else 1
            c.rect(x - w // 2, base_y - k, w, 1, col)
        c.px(x, base_y - h, P('warm', 7))
    c.px(11 + frame * 4, 8 + (frame % 2) * 3, P('warm', 6))     # spark
    return c


def bedroll():
    """A worn blanket laid flat on the ground, head end rolled up; seen from above it is a low
    wide mat, not a standing slab. Earth-dark, not dyed."""
    c = Canvas(T, T)
    c.rect(6, 19, 21, 8, P('stone', 2))
    c.rect(7, 20, 19, 6, P('stone', 3))
    c.hline(7, 25, 20, P('stone', 4))               # lit far edge
    for x in (12, 17, 22):
        c.vline(x, 21, 25, P('stone', 2))
    c.rect(4, 18, 4, 9, P('earth', 3))               # the roll at the head end
    c.vline(4, 18, 26, P('earth', 5))
    c.vline(7, 18, 26, P('earth', 1))
    c.hline(5, 27, 27, SHADOW)
    return outlined(c)


# ---------------------------------------------------------------- roofs and building stages
def roof_piece(kind, mask):
    """One roof tile seen from above, 32x32, for the 4-bit mask of roofed neighbours
    (N=1 E=2 S=4 W=8, as the walls). Thatch is bundled straw in rows; shingles are split wood
    in staggered courses. A side with no roof beyond it is an eave: a lit lip on the north and
    west, a dark overhang on the south and east."""
    c = Canvas(T, T)
    r = rng('roof-%s' % kind)                     # the same texture on every piece: seamless
    if kind == 'thatch':
        # Straw runs down the slope (north to south here): fibres in vertical strokes, in
        # courses whose lower edges are combed and shadowed.
        fib = [r.choice((5, 5, 6, 4, 6, 7)) for _ in range(T)]
        for y in range(T):
            k = y % 8
            shift = (y // 8) * 5
            for x in range(T):
                col = P('earth', fib[(x + shift) % T])
                if k == 0:
                    col = P('earth', min(7, fib[(x + shift) % T] + 1))
                if k == 7:
                    col = P('earth', 3) if (x + shift) % 3 else P('earth', 2)
                elif k == 6 and (x + shift) % 4 == 0:
                    col = P('earth', 3)              # fibre ends hanging over the course below
                c.px(x, y, col)
    else:
        # Split shingles, irregular widths, grain along their length, courses lapped so each
        # butt end is lit and throws a shadow on the course below.
        for course in range(T // 6 + 1):
            widths, total = [], 0
            while total < T:
                wd = r.randint(4, 7)
                widths.append(wd)
                total += wd
            widths[-1] -= total - T
            x0 = (course * 3) % T
            for wd in widths:
                shade = r.choice((2, 3, 3, 4))
                for dx in range(wd):
                    x = (x0 + dx) % T
                    for k in range(6):
                        y = course * 6 + k
                        if y >= T:
                            continue
                        col = P('earth', shade)
                        if dx == 0:
                            col = P('earth', 1)
                        elif k == 5:
                            col = P('earth', 1)
                        elif k == 0:
                            col = P('earth', min(6, shade + 2))
                        elif (dx + k * 2) % 5 == 0:
                            col = P('earth', max(1, shade - 1))   # grain
                        c.px(x, y, col)
                x0 += wd
    n, e, s, w = mask & 1, mask & 2, mask & 4, mask & 8
    lit = P('earth', 7) if kind == 'thatch' else P('earth', 5)
    if not n:
        c.hline(0, T - 1, 0, lit)
        c.hline(0, T - 1, 1, P('earth', 4))
    if not w:
        c.vline(0, 0, T - 1, lit)
    if not s:
        c.hline(0, T - 1, T - 1, BLACK)
        c.hline(0, T - 1, T - 2, P('earth', 1))
        c.hline(0, T - 1, T - 3, P('earth', 2))
    if not e:
        c.vline(T - 1, 0, T - 1, BLACK)
        c.vline(T - 2, 0, T - 1, P('earth', 1))
    return c


def roof_sheet(kind):
    return sheet([roof_piece(kind, m) for m in range(16)], T, T)


def roof_snow():
    """Snow lying on a roof tile: soft drifts with the courses showing through here and there
    (the client fades it in by how much lies there)."""
    c = Canvas(T, T)
    for y in range(T):
        for x in range(T):
            # Low-frequency drift pattern, seamless over the 32-px tile.
            v = (math.sin(x / 32 * math.tau * 2 + y / 32 * math.tau) +
                 math.sin(y / 32 * math.tau * 3 - x / 32 * math.tau) * 0.6)
            if v > -0.9:
                c.px(x, y, N('white') if v > 0.2 else N('light'))
            if y % 8 == 7 and v < -0.2:
                c.px(x, y, P('stone', 6))                # a course edge just showing
    return c


def roof_frame():
    """A roof going up: rafters laid from the walls, lashed, with daylight between."""
    c = Canvas(T, T)
    for x in (3, 13, 23):
        c.rect(x, 0, 3, T, P('earth', 2))
        c.vline(x, 0, T - 1, P('earth', 4))
        c.vline(x + 2, 0, T - 1, P('earth', 1))
    for y in (6, 22):
        c.rect(0, y, T, 2, P('earth', 3))
        c.hline(0, T - 1, y, P('earth', 5))
        for x in (3, 13, 23):
            c.px(x + 1, y, P('earth', 6))                # lashing
    return outlined(c)


def wall_build(material, stage):
    """A wall going up, as seen from above like the finished wall-top pieces: 0 the footing
    (a sill log on stakes, or a first course of stones), 1 half raised, 2 nearly closed."""
    c = Canvas(T, T)
    stone = material == 'stone_block'
    ramp = 'stone' if stone else 'earth'
    if stage == 0:
        if stone:
            for x in range(1, T - 1, 7):
                c.rect(x, 12, 6, 8, P(ramp, 4))
                c.hline(x, x + 5, 12, P(ramp, 6))
                c.hline(x, x + 5, 19, P(ramp, 2))
        else:
            c.rect(0, 13, T, 6, P(ramp, 3))
            c.hline(0, T - 1, 13, P(ramp, 5))
            c.hline(0, T - 1, 18, P(ramp, 1))
            for x in (2, 15, 28):
                c.rect(x, 10, 2, 12, P(ramp, 2))
                c.px(x, 10, P(ramp, 5))
        return outlined(c)
    rows = 14 if stage == 1 else 26
    top = (T - rows) // 2
    for y in range(top, top + rows):
        for x in range(T):
            if stone:
                course = (y - top) // 5
                col = P(ramp, 4) if ((x + course * 4) // 8) % 2 else P(ramp, 3)
                if (y - top) % 5 == 4 or (x + course * 4) % 8 == 0:
                    col = P(ramp, 1)
            else:
                col = P(ramp, 4) if ((y - top) // 4) % 2 else P(ramp, 3)
                if (y - top) % 4 == 3:
                    col = P(ramp, 1)
            c.px(x, y, col)
    c.hline(0, T - 1, top, P(ramp, 6))
    if stage == 2:
        # One gap not yet closed.
        c.rect(19, top, 6, rows, P(ramp, 1))
        c.rect(20, top + 1, 4, rows - 2, SHADOW)
    for x in (0, T - 3):
        c.rect(x, top - 2, 3, rows + 4, P('earth', 2))  # the posts it is built between
        c.vline(x, top - 2, top + rows + 1, P('earth', 4))
    return outlined(c)


def _rough_upright(c, x0, x1, y0, y1, seed):
    """A rough-hewn upright: lit on the left, shadowed on the right, worn and chipped."""
    for y in range(y0, y1):
        # Weathered edges: a row now and then bites in or bulges out by a pixel.
        a = x0 + (1 if (y * 5 + seed) % 7 == 0 else 0)
        b = x1 + (1 if (y * 3 + seed) % 9 == 0 else 0) - (1 if (y * 7 + seed) % 11 == 0 else 0)
        for x in range(a, b):
            col = P('stone', 3)
            if x == a:
                col = P('stone', 5)
            elif x == b - 1:
                col = P('stone', 1)
            elif (x * 7 + y * 3 + seed) % 11 == 0:
                col = P('stone', 2)
            c.px(x, y, col)
    c.px(x0 + 1, y0 + 4, P('foliage', 3))               # lichen
    c.px(x0 + 2, y1 - 6, P('foliage', 4))


def portal_stone(awake):
    """The stone the ten came through: two weathered uprights under a lintel, taller than a
    person. Awake, the gap between them shimmers cool and a faint halo hangs about it."""
    c = Canvas(T, 48)
    c.disc(16, 44, 13, 3, SHADOW)                       # ground shadow
    _rough_upright(c, 5, 11, 12, 45, 1)
    _rough_upright(c, 21, 27, 13, 45, 4)
    for y in range(7, 13):                              # the lintel, a little askew
        for x in range(3, 30):
            col = P('stone', 4) if y == 7 else P('stone', 3)
            if y == 12:
                col = P('stone', 1)
            if (x == 3 and y == 7) or (x == 29 and y == 12):
                continue
            c.px(x, y + (1 if x > 22 else 0), col)
    if awake:                                           # the gap: thin air, shimmering
        for y in range(14, 45):
            for x in range(11, 21):
                band = (x + y // 3) % 4
                col = P('cool', 6) if band == 0 else (P('cool', 4) if band == 1 else P('cool', 2))
                c.px(x, y, col, 170 if band < 2 else 110)
        c.px(15, 20, N('white'))
        c.px(17, 31, N('white'))
    c = outlined(c)
    if awake:
        for (rad, a) in ((15, 40), (10, 70)):
            g = Canvas(T, 48)
            g.disc(16, 28, rad, rad + 6, P('cool', 5), a)
            for y in range(48):
                for x in range(T):
                    if g.a[y, x, 3] and c.a[y, x, 3] == 0:
                        c.px(x, y, P('cool', 5), int(g.a[y, x, 3]))
    return c


def ward_stone():
    """A warding stone: a rough standing stone raised out of the valley's rock, a mark cut in
    its face."""
    c = Canvas(T, T)
    c.disc(16, 28, 7, 2, SHADOW)
    _rough_upright(c, 12, 20, 6, 28, 7)
    for y in range(4, 6):                               # a rounded top
        for x in range(13, 19):
            c.px(x, y, P('stone', 4) if y == 4 else P('stone', 3))
    for (x, y) in ((15, 11), (16, 12), (15, 13), (16, 14), (15, 15), (14, 13), (17, 13)):
        c.px(x, y, P('stone', 1))                       # the cut mark
    c.px(16, 12, P('stone', 6))
    return outlined(c)


def shrine():
    """A shrine (#297): a low stone plinth, and on it a little gabled house of posts and a
    pitched roof sheltering a carved upright, an offering bowl before it."""
    c = Canvas(T, T)
    c.disc(16, 27, 11, 3, SHADOW)
    # the plinth: two courses of stone, lit from the left
    for y in range(21, 27):
        for x in range(6, 26):
            col = P('stone', 3)
            if x == 6 or y == 21:
                col = P('stone', 5)
            elif x == 25 or y == 26:
                col = P('stone', 1)
            elif y == 24:
                col = P('stone', 2)                     # the course line
            c.px(x, y, col)
    for x in (11, 17, 22):
        c.px(x, 22, P('stone', 2))
        c.px(x, 23, P('stone', 2))
    # two posts
    c.rect(9, 11, 2, 10, P('earth', 3))
    c.vline(9, 11, 20, P('earth', 5))
    c.rect(21, 11, 2, 10, P('earth', 3))
    c.vline(22, 11, 20, P('earth', 1))
    # the carved upright between them, a mark on its face
    c.rect(14, 12, 4, 9, P('stone', 4))
    c.vline(14, 12, 20, P('stone', 6))
    c.vline(17, 12, 20, P('stone', 2))
    c.px(15, 14, P('stone', 1))
    c.px(16, 15, P('stone', 1))
    c.px(15, 16, P('stone', 1))
    # the roof: a gable of shingles over the posts
    for i in range(7):
        y = 4 + i
        x0, x1 = 16 - 2 - i * 1.6, 16 + 1 + i * 1.6
        for x in range(int(x0), int(x1) + 1):
            col = P('earth', 4) if x < 16 else P('earth', 2)
            if y == 10:
                col = P('earth', 1)
            c.px(x, y, col)
    c.hline(14, 17, 4, P('earth', 6))                   # light on the ridge
    # the offering bowl, something bright in it
    c.rect(14, 21, 4, 2, P('earth', 2))
    c.px(15, 20, P('warm', 5))
    c.px(16, 20, P('stone', 6))
    return outlined(c)


def grave():
    """A grave from the valley's past: a low mound grown over, a weathered standing stone at
    its head, leaning a little."""
    c = Canvas(T, T)
    c.disc(15.5, 22, 9, 5, P('foliage', 2))            # the mound, grassed over
    c.disc(15.5, 21, 8, 4, P('foliage', 3))
    c.hline(9, 22, 19, P('foliage', 4))                 # light on its crown
    c.hline(8, 23, 26, SHADOW)
    # the stone: a rough slab, lit on the left, worn at the top
    for y in range(6, 20):
        lean = (20 - y) // 7
        x0 = 13 + lean
        for x in range(x0, x0 + 6):
            col = P('stone', 3)
            if x == x0:
                col = P('stone', 5)
            elif x == x0 + 5:
                col = P('stone', 1)
            if y == 6 and x in (x0, x0 + 5):
                continue                                 # rounded, worn corners
            c.px(x, y, col)
    c.px(15, 10, P('stone', 2))                         # a chip, a lichen spot
    c.px(16, 13, P('foliage', 4))
    c.px(14, 15, P('foliage', 3))
    return outlined(c)


def main():
    os.makedirs(OUT, exist_ok=True)
    for m in ('wood', 'stone_block', 'thatch'):
        wall_sheet(m).save(os.path.join(OUT, 'wall_%s.png' % m))
    singles = {
        'door_closed': door_closed, 'door_open': door_open, 'window': window,
        'stairs_down': lambda: stairs('down'), 'stairs_up': lambda: stairs('up'),
        'stairs_both': lambda: stairs('both'), 'ladder': ladder,
        'bed': bed, 'table': table, 'chair': chair, 'stool': stool, 'chest': chest,
        'shelf': shelf, 'workbench': workbench, 'loom': loom, 'anvil': anvil,
        'barrel': barrel, 'crate': crate, 'well': well, 'fence_post': fence_post, 'support': support,
        'stockpile': stockpile, 'carpenter_bench': carpenter_bench,
        'furnace': furnace, 'kiln': kiln,
    }
    for name, fn in singles.items():
        fn().save(os.path.join(OUT, name + '.png'))
    sheet([hearth(f) for f in range(3)], T, T).save(os.path.join(OUT, 'hearth.png'))
    sheet([cooking_pot(f) for f in range(2)], T, T).save(os.path.join(OUT, 'cooking_pot.png'))
    sheet([lamp(False), lamp(True)], T, T).save(os.path.join(OUT, 'lamp.png'))
    sheet([portal_stone(False), portal_stone(True)], T, 48).save(os.path.join(OUT, 'portal_stone.png'))
    ward_stone().save(os.path.join(OUT, 'ward_stone.png'))
    smelter().save(os.path.join(OUT, 'smelter.png'))
    sheet([fire(f) for f in range(4)], T, T).save(os.path.join(OUT, 'fire.png'))
    lean_to().save(os.path.join(OUT, 'lean_to.png'))
    tent().save(os.path.join(OUT, 'tent.png'))
    nest().save(os.path.join(OUT, 'nest.png'))
    shrine().save(os.path.join(OUT, 'shrine.png'))
    sheet([campfire(f) for f in range(3)], T, T).save(os.path.join(OUT, 'campfire.png'))
    bedroll().save(os.path.join(OUT, 'bedroll.png'))
    grave().save(os.path.join(OUT, 'grave.png'))
    roof_sheet('thatch').save(os.path.join(OUT, 'roof_thatch.png'))
    roof_sheet('shingle').save(os.path.join(OUT, 'roof_shingle.png'))
    roof_snow().save(os.path.join(OUT, 'roof_snow.png'))
    roof_frame().save(os.path.join(OUT, 'roof_frame.png'))
    for m in ('wood', 'stone_block'):
        sheet([wall_build(m, k) for k in range(3)], T, T).save(os.path.join(OUT, 'wall_build_%s.png' % m))
    for name in WALL_LOOKS:
        look_sheet(name).save(os.path.join(OUT, 'wall_%s.png' % name))
    print('structures -> %s' % OUT)


if __name__ == '__main__':
    main()
