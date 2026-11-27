#!/usr/bin/env python3
"""Ostrog inhabitant sprite generator.

Writes layered sheets to godot/assets/people/. Every sheet is len(FRAMES) columns x 4 rows of
48x48 cells (larger for the plans listed below). Rows: S, W, E, N. Columns (FRAMES, also written
to swaps.json "layout"):

  0-3    idle, idle2 (breath), idle3 (weight shift), glance
  4-9    walk1-6: contact, down, passing, contact, down, passing (1 px bob)
  10-12  swing1-3: wind-up, swing, strike            (chop, mine)
  13-15  stoop1-3: bend, reach to the ground, draw back (hoe, field work, forage, beat out fire)
  16-18  tap1-3:   raise, strike, recoil              (build, craft, cook)
  19     hold: standing with a load in both arms
  20-25  carry1-6: the walk with a load in both arms
  26-27  sleep, sit
  28-30  strike1-3: draw back, thrust, recover          (a blow in a fight, a spear's thrust)
  31-33  hit, fall, lying: struck, falling, down       (P2 of milestone MP)
  34-36  talk1-3: turn, gesture, settle                 (#485, Activity::Talking only)
  37-39  eat1-3:  lift, hand to mouth, lower            (#485, Activity::Eating only)
  40-41  sleep2, sleep3: breathing in, stirring        (#485, asleep: with 26, a breathing cycle)
  42-44  tend1-3: kneel, dress, check                   (#485, Tend work: a healer at a patient)
  45-47  drink1-3: scoop, lift, drink                   (#485, Activity::Drinking only)

Body plans (P2, data/peoples/plans): the same skeleton is drawn per plan. 'biped' is the human
figure; 'digitigrade_tail' bends the knee back, lifts the heel and adds a tail that sways with
the walk. Sheets for a plan other than the biped carry `_<plan>` before the build and age.
'amphibious' adds webbed hands and feet and gills; 'taur' (64 px cell) and 'serpentine' (96 px)
set the biped's upper body on a lower body of their own (composite_skeleton, draw_lower);
'huge_biped' (96 px) is drawn on the 48 px rig and scaled by two before shading. A plan's sheets
use its cell (data/peoples/plans/<plan>.toml), recorded in swaps.json "plan_cells".

Layers (composed at runtime, in this order): body, clothing, hair, bandage overlay, equipment.
Every layer is drawn for every build and age, so each one sits on the same skeleton as the
body it is worn by:
  body_{build}_{age}.png                     drawn in SKIN swap colours
  hair_{01..10}[_{build}_{age}].png          drawn in HAIR swap colours
  clothing_{outfit}[_{build}_{age}].png      drawn in CLOTH primary + trim swap colours
  equipment_{item}[_{build}_{age}].png       fixed palette colours, positioned per frame
  (the suffix is omitted for the average adult, whose sheets keep their original names)
  overlay_bandage_{build}_{age}.png          fixed palette colours
  swaps.json                                 exact swap source colours and target sets

Construction is parametric: a skeleton (head and torso boxes, legs as rows of pixel spans from
hip to foot, arms as chains of square stamps from shoulder to hand, hand points) is computed per
build, age, direction and frame with explicit coordinates; each layer paints "groups" (skin,
hair, cloth, trim, or a fixed ramp) onto a 32x32 index grid from that one skeleton, so layers
cannot drift apart. A side-view lean (a stoop, a swing) is a shear applied to the finished grid
of every layer alike. The renderer then applies 1-px shading lit from the upper-left using the
layer's silhouette and a 1-px dark outline around it. W is the mirror of E at the shape level,
so lighting stays upper-left.

Swapping: a runtime palette swap maps the four source colours of a group to the four
colours of a target set (shadow, mid, light, highlight). Sources are chosen so no layer
reuses another layer's colours and none collides with the outline colour.

Run: python3 tools/art/people.py [PREFIX]   (PREFIX: redraw only the sheets named so)
"""
import json
import math
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pal import Palette, rgb_to_hex, ROOT  # noqa: E402  (switch palette module here)

OUT = os.path.join(ROOT, 'godot', 'assets', 'people')
CELL = 48
# Drawn for 48 px cells since style bible v2 (M2); the pose tables below were tuned at 32 px
# and every offset goes through k(), which scales by K but keeps a 1 (the adjacent pixel).
K = CELL / 32
DIRS = ['S', 'W', 'E', 'N']
FRAMES = (['idle', 'idle2', 'idle3', 'glance']
          + ['walk%d' % i for i in range(1, 7)]
          + ['swing1', 'swing2', 'swing3', 'stoop1', 'stoop2', 'stoop3', 'tap1', 'tap2', 'tap3']
          + ['hold'] + ['carry%d' % i for i in range(1, 7)]
          + ['sleep', 'sit']
          + ['strike1', 'strike2', 'strike3', 'hit', 'fall', 'lying']
          + ['talk1', 'talk2', 'talk3', 'eat1', 'eat2', 'eat3']
          + ['sleep2', 'sleep3', 'tend1', 'tend2', 'tend3', 'drink1', 'drink2', 'drink3'])
# The three frames of sleep (#485): at rest, breathing in (the chest a pixel higher) and stirring
# (the head rolled a pixel). Every check for the sleeping figure goes through asleep(); each
# frame is drawn from a standing frame and then laid down (SLEEP_DRAW).
SLEEPS = ('sleep', 'sleep2', 'sleep3')
SLEEP_DRAW = {'sleep': 'idle', 'sleep2': 'breath', 'sleep3': 'glance'}


def asleep(frame):
    return frame in SLEEPS


def rest_frame(frame):
    """The frame a lower body and the posture checks treat this one as: every sleep frame is
    'sleep'."""
    return 'sleep' if asleep(frame) else frame
# Frame families that hold a tool along a line from the hand ('strike' is the fight set, P2).
WORK = ('swing', 'stoop', 'tap', 'strike')
# Body plans drawn (data/peoples/plans); the first is the human one, whose sheets keep their names.
# New plans are appended; 'amphibious' is the biped with webbed hands and feet and gills, 'taur'
# a human torso on a four-legged body (64 px cell), 'huge_biped' the biped rig at double size
# (96 px cell, two by two tiles) and 'serpentine' a human torso on a coiled tail (96 px cell).
PLANS = ['biped', 'small_biped', 'digitigrade_tail', 'winged', 'four_armed',
         'amphibious', 'taur', 'huge_biped', 'serpentine']
# Each species' body sheet (data/peoples/species/*.toml [art] body) and the plan it is drawn on;
# its head and markings are drawn into the body layer. Humans are the plain biped body.
SPECIES = {'human': 'biped', 'hollin': 'biped', 'grisk': 'small_biped',
           'screeling': 'digitigrade_tail', 'skarri': 'winged', 'tharn': 'four_armed',
           'murrow': 'biped', 'holm': 'huge_biped', 'below': 'serpentine', 'seamfolk': 'biped'}
# Biped kinds drawn on a rig of their own (C53, docs/reviews/readability-2026-10-19), so that
# their outline reads apart from the human's at 1x. Their hair, clothing, equipment and bandage
# are drawn on that rig as well (<layer>_<species>_<build>_<age>.png, no unsuffixed average
# adult), so every layer stays in register with the body under it.
#   lift  px taller than the human (head top higher, feet where they are; negative = lower)
#   neck  of which px go to the neck (the rest lengthen the legs)
#   dw    width deltas: head, torso front, torso side, leg, arm
#   lean  side-view stoop (32 px units, the forward shear of a lean, held in every standing frame)
#   dh    head height delta (a longer face)
#   arm   arm length delta, px (a digger's long arms, the hands low at the knees)
#   splay px each foot stands wider than the hip from the front and back (a wide, bowed stance)
# A child gets about two thirds of the lift and the same width deltas, kept above a minimum.
RIG = {
    'hollin':   dict(lift=3, neck=0, dw=(0, -2, -1, -1, 0)),
    'seamfolk': dict(lift=8, neck=3, dw=(-1, -4, -2, -1, -1), dh=1),
    'murrow':   dict(lift=-4, neck=-1, dw=(0, 4, 2, 1, 1), lean=3, arm=5, splay=2),
}
# Plans whose upper body is the biped's (head, torso, arms) set on a lower body of their own,
# drawn natively in their larger cell; and plans drawn on the 48 px rig then scaled up.
COMPOSITE = ('taur', 'serpentine')
SCALED = {'huge_biped': 2}


def _plan_cells():
    """Cell (px) and footprint (tiles) per plan, from data/peoples/plans/<plan>.toml."""
    import tomllib
    out = {}
    for p in PLANS:
        with open(os.path.join(ROOT, 'data', 'peoples', 'plans', p + '.toml'), 'rb') as f:
            t = tomllib.load(f)
        out[p] = (int(t.get('cell', 48)), int(t.get('tiles', 1)))
    return out


PLAN_CELLS = _plan_cells()
BUILDS = ['slim', 'average', 'broad']
AGES = ['adult', 'child']
OUTFITS = ['tunic', 'dress', 'apron', 'cloak', 'coat', 'workshirt', 'robe', 'vest']
EQUIPMENT = ['pickaxe', 'axe', 'basket', 'hoe', 'bundle', 'bucket', 'sack', 'spear']
HAIR_STYLES = ['crop', 'bowl', 'long', 'ponytail', 'bun', 'balding', 'braids', 'crest', 'curly', 'fringe']

pal = Palette.load()
OUTLINE = pal.neutral('black')

# --- swap groups: 4 steps each, shadow, mid, light, highlight ----------------------------
# Source colours (what is painted into the sheets). Each is a distinct palette colour.
SWAP_SOURCES = {
    'skin': [pal.ramp('skin', 1), pal.ramp('skin', 2), pal.ramp('skin', 3), pal.ramp('skin', 4)],
    'hair': [pal.ramp('stone', 1), pal.ramp('stone', 2), pal.ramp('stone', 3), pal.ramp('stone', 4)],
    'cloth': [pal.ramp('warm', 2), pal.ramp('warm', 3), pal.ramp('warm', 4), pal.ramp('warm', 5)],
    'trim': [pal.ramp('cool', 2), pal.ramp('cool', 3), pal.ramp('cool', 4), pal.ramp('cool', 5)],
}


def R(ramp, *steps):
    return [pal.ramp(ramp, s) for s in steps]


SKIN_TONES = {
    'deep':   [pal.ramp('earth', 1)] + R('skin', 0, 1, 2),
    'dark':   R('skin', 0, 1, 2, 3),
    'tan':    R('skin', 1, 2, 3, 4),
    'olive':  [pal.ramp('earth', 2), pal.ramp('earth', 4), pal.ramp('skin', 3), pal.ramp('skin', 4)],
    'fair':   R('skin', 2, 3, 4, 5),
    'pale':   [pal.ramp('skin', 3), pal.ramp('skin', 4), pal.ramp('skin', 5), pal.neutral('light')],
    # The species' own (P3; crates/hf-sim/src/species.rs SKIN_NAMES keeps this order).
    'ash':           R('stone', 3, 4, 5, 6),
    'bark':          R('earth', 1, 2, 3, 4),
    'lichen':        R('olive', 2, 3, 4, 5),
    'moss_skin':     R('foliage', 1, 2, 3, 4),
    'slate_skin':    R('stone', 1, 2, 3, 4),
    'scale_rust':    R('copper', 1, 2, 3, 4),
    'scale_green':   R('foliage', 2, 3, 4, 5),
    'scale_dun':     R('olive', 3, 4, 5, 6),
    'feather_brown': R('earth', 2, 3, 4, 5),
    'feather_grey':  R('stone', 2, 3, 4, 5),
    'feather_white': [pal.ramp('stone', 4), pal.ramp('stone', 5), pal.ramp('stone', 6), pal.neutral('white')],
    'copper_skin':   R('copper', 2, 3, 4, 5),
    'umber':         R('earth', 0, 1, 2, 3),
    'dust':          R('earth', 3, 4, 5, 6),
    # The deep peoples (docs/design-peoples.md section 9): Murrow chalk and lilac, Holm stone
    # and basalt, the Below ember and abyss. Appended only; species.rs SKIN_NAMES follows.
    'chalk':         R('stone', 4, 5, 6, 7),
    'pale_lilac':    [pal.ramp('stone', 4), pal.ramp('cool', 5), pal.ramp('cool', 6), pal.ramp('cool', 7)],
    'stone_grey':    R('stone', 1, 3, 4, 5),
    'basalt':        R('stone', 0, 1, 2, 3),
    'ember':         [pal.ramp('copper', 1), pal.ramp('warm', 2), pal.ramp('warm', 3), pal.ramp('warm', 5)],
    'abyss':         [pal.ramp('cool', 1), pal.ramp('cool', 2), pal.ramp('water', 2), pal.ramp('water', 4)],
}
HAIR_COLOURS = {
    'black':    [pal.neutral('black'), pal.ramp('skin', 6), pal.neutral('shadow'), pal.ramp('stone', 1)],
    'brown':    R('earth', 1, 2, 3, 4),
    'chestnut': [pal.ramp('earth', 1), pal.ramp('earth', 2), pal.ramp('warm', 3), pal.ramp('earth', 4)],
    'red':      R('warm', 1, 2, 3, 4),
    'blonde':   [pal.ramp('earth', 3), pal.ramp('skin', 7), pal.ramp('earth', 5), pal.ramp('earth', 6)],
    'grey':     R('stone', 2, 3, 4, 5),
    'white':    [pal.ramp('stone', 4), pal.ramp('stone', 5), pal.ramp('stone', 6), pal.neutral('white')],
    'auburn':   [pal.ramp('warm', 1), pal.ramp('earth', 2), pal.ramp('warm', 3), pal.ramp('warm', 4)],
}
CLOTH_COLOURS = {
    'undyed':   R('earth', 3, 4, 5, 6),
    'brown':    R('earth', 1, 2, 3, 4),
    'rust':     R('warm', 1, 2, 3, 4),
    'madder':   R('warm', 2, 3, 4, 5),
    'ochre':    R('warm', 3, 4, 5, 6),
    'moss':     R('foliage', 1, 2, 3, 4),
    'leaf':     R('foliage', 2, 3, 4, 5),
    'teal':     R('water', 1, 2, 3, 4),
    'sky':      R('water', 3, 4, 5, 6),
    'indigo':   R('cool', 0, 1, 2, 3),
    'violet':   R('cool', 2, 3, 4, 5),
    'slate':    R('stone', 1, 2, 3, 4),
}
# fixed ramps for equipment and boots (not swapped)
FIXED = {
    'wood':   R('earth', 1, 2, 3, 4),
    'iron':   R('stone', 2, 3, 4, 5),
    'wicker': R('earth', 3, 4, 5, 6),
    'burlap': R('earth', 2, 3, 4, 5),
    'twine':  R('warm', 2, 3, 4, 5),
    'boot':   R('earth', 0, 1, 2, 3),
}
GROUPS = {}
for _k, _v in list(SWAP_SOURCES.items()) + list(FIXED.items()):
    GROUPS[_k] = _v
GID = {k: i for i, k in enumerate(GROUPS)}

SHADOW, MID, LIGHT, HIGH = 0, 1, 2, 3


def rnd(v):
    """Round half up (Python's round() is banker's rounding, which makes lines lumpy)."""
    return int(math.floor(v + 0.5))


def k(v):
    """A 32 px offset at this cell size: scaled by K, except that 0 and 1 stay as they are (a
    1 is "the next pixel": a nose bump, a hem, a rounded corner)."""
    if -1 <= v <= 1:
        return v
    return rnd(v * K) if v > 0 else -rnd(-v * K)


# --- canvas ---------------------------------------------------------------------------------
class Canvas:
    """32x32 grid of group ids (-1 = transparent) plus per-pixel forced shade / fixed colour.
    `noline` marks transparent pixels that must not receive the outline: holes cut in a garment
    where a limb of the body layer below passes in front of it."""

    def __init__(self, n=None):
        n = CELL if n is None else n
        self.n = n
        self.g = np.full((n, n), -1, dtype=np.int16)
        self.shade = np.full((n, n), -1, dtype=np.int8)      # forced step within group
        self.fixed = np.full((n, n, 3), -1, dtype=np.int16)  # forced rgb
        self.noline = np.zeros((n, n), dtype=bool)

    def px(self, x, y, grp, shade=-1):
        if 0 <= x < self.n and 0 <= y < self.n:
            self.g[y, x] = GID[grp]
            self.shade[y, x] = shade

    def rect(self, x0, y0, x1, y1, grp, shade=-1):
        """Inclusive rectangle."""
        for y in range(min(y0, y1), max(y0, y1) + 1):
            for x in range(min(x0, x1), max(x0, x1) + 1):
                self.px(x, y, grp, shade)

    def line(self, x0, y0, x1, y1, grp, shade=-1, w=1):
        n = max(abs(x1 - x0), abs(y1 - y0), 1)
        for i in range(n + 1):
            x = rnd(x0 + (x1 - x0) * i / n)
            y = rnd(y0 + (y1 - y0) * i / n)
            for k in range(w):
                self.px(x + k, y, grp, shade)

    def blob(self, x0, y0, x1, y1, grp, shade=-1, round2=False):
        """Inclusive rectangle with the four corner pixels cut (rounded); round2 also cuts the
        pixel beside each corner along the top and bottom (a rounder head at 48 px)."""
        self.rect(x0, y0, x1, y1, grp, shade)
        if x1 - x0 >= 2 and y1 - y0 >= 2:
            cut = [(x0, y0), (x1, y0), (x0, y1), (x1, y1)]
            if round2 and x1 - x0 >= 6 and y1 - y0 >= 6:
                cut += [(x0 + 1, y0), (x1 - 1, y0), (x0, y0 + 1), (x1, y0 + 1),
                        (x0 + 1, y1), (x1 - 1, y1)]
            for (x, y) in cut:
                if 0 <= x < self.n and 0 <= y < self.n:
                    self.g[y, x] = -1

    def clear(self, x, y):
        if 0 <= x < self.n and 0 <= y < self.n:
            self.g[y, x] = -1
            self.fixed[y, x] = -1

    def fixed_px(self, x, y, rgb):
        if 0 <= x < self.n and 0 <= y < self.n:
            self.fixed[y, x] = rgb
            if self.g[y, x] < 0:
                self.g[y, x] = GID['skin']  # any group; colour is forced

    def _arrays(self):
        return ('g', -1), ('shade', -1), ('fixed', -1), ('noline', False)

    def flipped(self):
        c = Canvas(self.n)
        for name, _ in self._arrays():
            setattr(c, name, getattr(self, name)[:, ::-1].copy())
        return c

    def lying(self):
        """Rotate 90 degrees so the top of the standing figure points left."""
        c = Canvas(self.n)
        for name, _ in self._arrays():
            setattr(c, name, np.rot90(getattr(self, name), 1).copy())
        return c

    def shear(self, shifts):
        """Shift row y by shifts[y] pixels (+x). Used for a side-view lean: every layer of a
        frame gets the same shear, so they stay in register."""
        for name, empty in self._arrays():
            a = getattr(self, name)
            b = a.copy()
            for y, dx in shifts.items():
                if not dx or not 0 <= y < self.n:
                    continue
                row = a[y].copy()
                b[y] = empty
                if dx > 0:
                    b[y, dx:] = row[:self.n - dx]
                else:
                    b[y, :self.n + dx] = row[-dx:]
            setattr(self, name, b)

    def scaled(self, f):
        """Every pixel as an f x f block on an f-times larger canvas (the huge plan: the figure
        is drawn on the 48 px rig, then shaded and outlined at the full cell, so its outline and
        edge light stay one pixel like everyone else's)."""
        c = Canvas(self.n * f)
        for name, _ in self._arrays():
            a = getattr(self, name)
            setattr(c, name, np.repeat(np.repeat(a, f, axis=0), f, axis=1).copy())
        return c

    def rotated(self, turns):
        """Rotate by 90 degrees `turns` times counter-clockwise (1 = the top points left)."""
        c = Canvas(self.n)
        for name, _ in self._arrays():
            setattr(c, name, np.rot90(getattr(self, name), turns).copy())
        return c

    def moved(self, dx, dy):
        """Translate by (dx, dy); what leaves the canvas is lost."""
        c = Canvas(self.n)
        n = self.n
        for name, empty in self._arrays():
            a = getattr(self, name)
            b = getattr(c, name)
            ys, yd = (slice(0, n - dy), slice(dy, n)) if dy >= 0 else (slice(-dy, n), slice(0, n + dy))
            xs, xd = (slice(0, n - dx), slice(dx, n)) if dx >= 0 else (slice(-dx, n), slice(0, n + dx))
            b[yd, xd] = a[ys, xs]
        return c

    def over(self, below):
        """This canvas drawn over `below` (same size): one canvas, shaded as one silhouette."""
        c = Canvas(self.n)
        top = self.g >= 0
        for name, _ in self._arrays():
            a, b = getattr(self, name), getattr(below, name)
            m = top if a.ndim == 2 else top[:, :, None]
            setattr(c, name, np.where(m, a, b).copy())
        c.noline = (self.noline | below.noline) & (c.g < 0)
        return c

    def render(self, groups=GROUPS, outline=OUTLINE):
        """Shade (lit from upper-left) and outline. Returns an RGBA uint8 array."""
        out = np.zeros((self.n, self.n, 4), dtype=np.uint8)
        mask = self.g >= 0
        pad = np.zeros((self.n + 2, self.n + 2), dtype=bool)
        pad[1:-1, 1:-1] = mask
        top_edge = ~pad[:-2, 1:-1]
        bot_edge = ~pad[2:, 1:-1]
        left_edge = ~pad[1:-1, :-2]
        right_edge = ~pad[1:-1, 2:]
        auto = np.full((self.n, self.n), MID, dtype=np.int8)
        # At 48 px the shadow side is two pixels deep (one pixel of light, two of shade): forms
        # read round instead of outlined. A second-row pixel needs the group to continue past it
        # (a 2 px limb would otherwise go all shadow).
        if self.n >= 48:
            pad2 = np.zeros((self.n + 4, self.n + 4), dtype=bool)
            pad2[2:-2, 2:-2] = mask
            bot2 = ~pad2[4:, 2:-2] & pad2[1:-3, 2:-2]
            right2 = ~pad2[2:-2, 4:] & pad2[2:-2, 1:-3]
            auto[(bot2 | right2) & mask] = SHADOW
        auto[bot_edge | right_edge] = SHADOW
        auto[(top_edge | left_edge) & ~(bot_edge | right_edge)] = LIGHT
        auto[top_edge & left_edge] = HIGH
        steps = np.where(self.shade >= 0, self.shade, auto).astype(np.int64)
        table = np.array([groups[n] for n in groups], dtype=np.uint8)   # [group, step, rgb]
        rgb = table[np.clip(self.g, 0, None).astype(np.int64), steps]
        out[mask, :3] = rgb[mask]
        out[mask, 3] = 255
        forced = mask & (self.fixed[:, :, 0] >= 0)
        out[forced, :3] = self.fixed[forced]
        # 1-px outline around the silhouette (8-neighbour)
        ol = np.zeros_like(mask)
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                ol |= pad[1 + dy:self.n + 1 + dy, 1 + dx:self.n + 1 + dx]
        ol &= ~mask & ~self.noline
        out[ol, :3] = outline
        out[ol, 3] = 255
        return out


# --- skeleton -------------------------------------------------------------------------------
# widths: head, torso (front), torso (side), leg, arm
DIM = {
    'adult': {'slim': (11, 10, 7, 4, 3), 'average': (12, 13, 8, 4, 3), 'broad': (13, 17, 10, 5, 4)},
    'child': {'slim': (10, 8, 6, 3, 2), 'average': (11, 10, 7, 3, 3), 'broad': (11, 12, 8, 4, 3)},
}
FEET = CELL - 3  # last body row


class use_cell:
    """Draw in an n px cell for the duration of a with-block: the canvas size and the foot row
    follow it; the scale of the rig (K, every k() offset) does not, so a person in a larger cell
    is drawn at the same pixel size, with more room around them."""

    def __init__(self, n):
        self.n = n

    def __enter__(self):
        global CELL, FEET
        self.saved = (CELL, FEET)
        CELL, FEET = self.n, self.n - 3

    def __exit__(self, *exc):
        global CELL, FEET
        CELL, FEET = self.saved

# Side-view walk, facing +x. Per frame: near leg (foot dx, foot dy, knee dx), far leg (same),
# body bob (+1 = 1 px lower), near-arm swing dx (the far arm swings the other way).
# Contact: feet spread, body low. Down: weight on the front foot, rear heel up. Passing: the
# rear leg swings through with its knee forward, body high. The planted foot slides back under
# the hip from contact to the next contact, so the legs read as walking, not sliding.
WALK_SIDE = [
    ((3, 0, 0), (-3, 0, 0), 1, -2),
    ((2, 0, 0), (-3, -1, 0), 1, -1),
    ((0, 0, 0), (1, -2, 2), 0, 0),
    ((-3, 0, 0), (3, 0, 0), 1, 2),
    ((-3, -1, 0), (2, 0, 0), 1, 1),
    ((1, -2, 2), (0, 0, 0), 0, 0),
]
# Front-view walk (S; N flips the one-pixel reach because the leading foot is then the far one).
# Per frame: viewer-left foot dy, viewer-right foot dy (+1 = stepping toward the viewer, -2 =
# lifted), bob, arm dy for viewer-left and viewer-right (-2 = swung forward, foreshortened).
WALK_FRONT = [
    (1, -1, 1, 0, -2),
    (0, -1, 1, 0, -1),
    (0, -2, 0, 0, 0),
    (-1, 1, 1, -2, 0),
    (-1, 0, 1, -1, 0),
    (-2, 0, 0, 0, 0),
]
# Work cycles: body bob, stoop drop (head and torso lowered, knees bent), side-view lean (px of
# forward shear at the head), side-view stance (near foot dx, far foot dx).
WORK_BODY = {
    'swing1': dict(bob=0, drop=0, lean=-1, stance=(2, -2)),
    'swing2': dict(bob=0, drop=0, lean=1, stance=(2, -2)),
    'swing3': dict(bob=1, drop=0, lean=2, stance=(2, -2)),
    'stoop1': dict(bob=0, drop=2, lean=3, stance=(1, -2)),
    'stoop2': dict(bob=0, drop=3, lean=4, stance=(1, -2)),
    'stoop3': dict(bob=0, drop=2, lean=2, stance=(1, -2)),
    'tap1':   dict(bob=0, drop=0, lean=1, stance=(1, -1)),
    'tap2':   dict(bob=1, drop=0, lean=1, stance=(1, -1)),
    'tap3':   dict(bob=0, drop=0, lean=1, stance=(1, -1)),
    # The fight set: weight back to draw the arm, forward into the thrust, back to guard.
    'strike1': dict(bob=0, drop=0, lean=-1, stance=(2, -2)),
    'strike2': dict(bob=0, drop=1, lean=2, stance=(3, -2)),
    'strike3': dict(bob=0, drop=0, lean=1, stance=(2, -2)),
}


def _child_dx(v):
    """A child's stride is shorter: 3 -> 2, 2 -> 1, 1 -> 1."""
    if v == 0:
        return 0
    m = {1: 1, 2: 1, 3: 2}.get(abs(v), abs(v) - 1)
    return m if v > 0 else -m


def frame_params(frame, side, d='S', age='adult'):
    """Per-frame pose parameters. side=True for E/W views."""
    p = dict(bob=0, sway=0, head_dx=0, drop=0, lean=0, pose='stand', tool='side',
             legA=(0, 0, 0), legB=(0, 0, 0), armA=0, armB=0)
    child = age == 'child'
    if frame.startswith('talk'):
        p.update(tool=frame, head_dx={'talk1': -1, 'talk2': 0, 'talk3': 1}[frame])
    elif frame.startswith('eat'):
        p.update(tool=frame)
    elif frame == 'glance':
        p.update(head_dx=1)  # a glance aside: the head turns a pixel
    elif frame == 'idle2':
        p.update(bob=1)      # breathing: the whole figure settles 1 px
    elif frame == 'idle3':
        p.update(sway=1)     # weight shift with no vertical drop
    elif frame == 'hold':
        p.update(tool='carry')
    elif frame.startswith('walk') or frame.startswith('carry'):
        kk = int(frame[-1]) - 1
        if side:
            a, b, bob, sw = WALK_SIDE[kk]
            if child:
                a = (_child_dx(a[0]), a[1], _child_dx(a[2]))
                b = (_child_dx(b[0]), b[1], _child_dx(b[2]))
            p.update(legA=a, legB=b, bob=bob, armA=sw, armB=-sw)
        else:
            ady, bdy, bob, aa, ba = WALK_FRONT[kk]
            if d == 'N':
                ady = -ady if abs(ady) == 1 else ady
                bdy = -bdy if abs(bdy) == 1 else bdy
            p.update(legA=(0, ady, 0), legB=(0, bdy, 0), bob=bob, armA=aa, armB=ba)
        if frame.startswith('carry'):
            p.update(tool='carry')
    elif frame[:-1] in WORK:
        w = WORK_BODY[frame]
        drop = w['drop'] if not child else (w['drop'] + 1) // 2
        lean = w['lean'] if not child else rnd(w['lean'] * 0.67)
        p.update(tool=frame, bob=w['bob'], drop=drop, lean=lean if side else 0)
        if side:
            na, fa = w['stance']
            if child:
                na, fa = _child_dx(na), _child_dx(fa)
            p.update(legA=(na, 0, 0), legB=(fa, 0, 0))
    elif frame == 'sit':
        p.update(pose='sit')
    elif asleep(frame):
        p.update(pose='sleep')
    elif frame == 'breath':
        # breathing in while asleep: drawn standing, then laid down (SLEEP_DRAW)
        p.update(breath=1)
    elif frame.startswith('tend') or frame.startswith('drink'):
        # #485: kneeling low to tend a patient, or bent to water: a held stoop with the hands
        # forward (tend) or cupped from the ground to the mouth (drink).
        idx = int(frame[-1]) - 1
        if frame.startswith('tend'):
            drop, lean = [(3, 2), (3, 3), (2, 2)][idx]
        else:
            drop, lean = [(3, 4), (2, 2), (0, 0)][idx]
        p.update(tool=frame, drop=drop if not child else (drop + 1) // 2,
                 lean=(lean if not child else rnd(lean * 0.67)) if side else 0)
        if side:
            p.update(legA=(1, 0, 0), legB=(-2, 0, 0))
    elif frame == 'hit':
        # struck: the head snaps back, the shoulders follow, the weight on the back foot
        p.update(head_dx=-1, lean=-2 if side else 0, sway=0 if side else -1,
                 legA=(1, 0, 0) if side else (0, 0, 0), legB=(-2, 0, 0) if side else (0, 0, 0),
                 armA=1, armB=-1)
    elif frame == 'fall':
        # going down: knees buckling, the body tipping back
        p.update(head_dx=-1, bob=2, drop=3, lean=-4 if side else 0, sway=0 if side else -1,
                 legA=(2, 0, 1) if side else (0, 0, 0), legB=(-1, 0, 1) if side else (0, 0, 0))
    # The tables are in 32 px units: scale every offset to this cell size.
    for key in ('bob', 'sway', 'head_dx', 'drop', 'lean', 'armA', 'armB'):
        p[key] = k(p[key])
    for key in ('legA', 'legB'):
        p[key] = tuple(k(v) for v in p[key])
    return p


class Skel:
    pass


def rect_rows(x0, y0, x1, y1):
    return [(y, x0, x1) for y in range(y0, y1 + 1)]


def leg_rows(hx, hy, spec, w, toe=False):
    """A leg as one pixel span per row, from the hip (hx, hy) to the foot. spec = (foot dx,
    foot dy, knee dx): the foot sits at (hx + dx, FEET + dy); a knee dx bends the leg at mid
    height. toe=True (side view) lengthens the foot row by a pixel toward the facing."""
    dx, dy, kdx = spec
    fx, fy = hx + dx, max(hy, FEET + dy)
    pts = [(hx, hy)]
    if kdx:
        pts.append((hx + kdx, (hy + fy) // 2))
    pts.append((fx, fy))
    rows = []
    for i in range(len(pts) - 1):
        (x0, y0), (x1, y1) = pts[i], pts[i + 1]
        for y in range(y0 if i == 0 else y0 + 1, y1 + 1):
            t = (y - y0) / (y1 - y0) if y1 > y0 else 1.0
            x = rnd(x0 + (x1 - x0) * t)
            rows.append((y, x, x + w - 1))
    if toe and rows:
        y, a, b = rows[-1]
        rows[-1] = (y, a, b + 1)
    return rows


def arm_path(points, w):
    """An arm as a chain of w x w square stamps (x0, y0, x1, y1) stepping one pixel at a time
    from the shoulder (first point = top-left of the first stamp) to the hand. Sleeves cover a
    prefix of the chain and cuffs its end, so garments follow any arm pose."""
    out = []
    pts = list(points)
    if len(pts) == 1:
        pts = pts * 2
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        n = max(abs(x1 - x0), abs(y1 - y0), 1)
        for i in range(n + 1):
            x = rnd(x0 + (x1 - x0) * i / n)
            y = rnd(y0 + (y1 - y0) * i / n)
            sq = (x, y, x + w - 1, y + w - 1)
            if not out or out[-1] != sq:
                out.append(sq)
    return out


def corner(stamps, which):
    x0, y0, x1, y1 = stamps[-1]
    return {'bl': (x0, y1), 'br': (x1, y1), 'tl': (x0, y0), 'tr': (x1, y0)}[which]


def skeleton(build, age, d, frame, plan='biped', species='human'):
    """Explicit coordinates for one pose. d is 'S', 'E' or 'N' (W is mirrored later). A biped
    species with a RIG entry is drawn on its own proportions."""
    head_w, torso_w, torso_side, leg_w, arm_w = DIM[age][build]
    rig = RIG.get(species) if plan == 'biped' else None
    if rig:
        dh, dt, ds, dl, da = rig['dw']
        head_w, torso_w, torso_side = max(8, head_w + dh), max(6, torso_w + dt), max(5, torso_side + ds)
        leg_w, arm_w = max(2, leg_w + dl), max(2, arm_w + da)
    if plan == 'amphibious':
        # a wide, flat head (wide from the front; in profile only a pixel deeper)
        head_w += 1 if d == 'E' else (6 if age == 'adult' else 4)
    if plan == 'small_biped':
        head_w += 3 if age == 'adult' else 2   # a big head on a small body
    huge = plan == 'huge_biped'
    if huge:
        # A cave giant: drawn on this rig and scaled by two, so the build is in the widths. A
        # trunk four pixels broader (three deeper in profile), thicker limbs and the same head,
        # which at double size reads small on the shoulders.
        torso_w, torso_side, leg_w, arm_w = torso_w + 4, torso_side + 3, leg_w + 1, arm_w + 1
    if plan == 'winged':
        leg_w = max(2, leg_w - 1)   # thin bird's legs (taloned feet: draw_plan_features)
    if plan == 'four_armed':
        # a deep chest to carry four arms
        torso_w, torso_side = torso_w + 2, torso_side + 1
    side = d == 'E'
    p = frame_params(frame, side, d, age)
    if plan == 'digitigrade_tail' and side and p['pose'] == 'stand':
        # The knee bends back and the heel stays up: every side-view leg is drawn so.
        p['legA'] = (p['legA'][0], p['legA'][1] - 1, p['legA'][2] - k(2))
        p['legB'] = (p['legB'][0], p['legB'][1] - 1, p['legB'][2] - k(2))
    adult = age == 'adult'
    if rig and rig.get('lean') and side and p['pose'] == 'stand':
        p['lean'] += k(rig['lean'] if adult else rig['lean'] - 1)
    if rig and rig.get('splay') and not side and p['pose'] == 'stand':
        sp_ = rig['splay'] if adult else rig['splay'] - 1
        p['legA'] = (p['legA'][0] - sp_, p['legA'][1], p['legA'][2] - sp_ // 2)
        p['legB'] = (p['legB'][0] + sp_, p['legB'][1], p['legB'][2] + sp_ // 2)
    s = Skel()
    s.d, s.side, s.p, s.age, s.build, s.frame, s.plan = d, side, p, age, build, frame, plan
    s.species = species
    s.tail = []
    # 48 px proportions: an adult about 3.2 heads (35 px) tall with 12 px of leg; the 32 px
    # figure scaled up had 8 px of leg (stubby), a 39 px one stood taller than a door (lanky).
    s.head_h = 11 if adult else 10
    neck = 1
    torso_h = 12 if adult else 8
    top = (10 if adult else 18) + p['bob'] + p['drop']
    if huge:
        # No neck to speak of and a longer trunk: the shoulders hunch up to the jaw.
        neck = 0
        torso_h += 1
    if plan == 'small_biped':
        # A small people (about two thirds of a human): a big head on a short body.
        s.head_h += 2 if adult else 1
        torso_h = 8 if adult else 6
        top += 7 if adult else 4
    if plan == 'amphibious':
        s.head_h -= 2   # flat
    if rig:
        lift = rig['lift'] if adult else rnd(rig['lift'] * 0.67)
        dn = rig['neck'] if adult else rnd(rig['neck'] * 0.67)
        top -= lift
        neck = max(0, neck + dn)
        s.head_h += rig.get('dh', 0)
    cx = CELL // 2 + p['sway']
    s.cx = cx
    if side:
        head_w = head_w - 1
        torso_w = torso_side
    s.head_w = head_w
    s.torso_w = torso_w
    s.leg_w = leg_w
    s.arm_w = arm_w
    w = arm_w
    # head
    hx0 = cx - head_w // 2 + p.get('head_dx', 0)
    s.head = (hx0, top, hx0 + head_w - 1, top + s.head_h - 1)
    # torso
    ty0 = top + s.head_h + neck
    tx0 = cx - torso_w // 2
    if p['pose'] == 'sit':
        drop = (FEET - (ty0 + torso_h)) - (k(4) if d != 'E' else k(2))   # torso drops to seat height
        s.head = (hx0, top + drop, hx0 + head_w - 1, top + drop + s.head_h - 1)
        ty0 += drop
    s.torso = (tx0, ty0, tx0 + torso_w - 1, ty0 + torso_h - 1)
    if p.get('breath'):
        # breathing in: the chest a pixel fuller on the side that is uppermost once laid down
        torso_w += 1
        s.torso = (tx0, ty0, tx0 + torso_w - 1, ty0 + torso_h - 1)
    s.neck = (cx - 1, ty0 - neck, cx, ty0 - 1) if neck else None
    ty1 = s.torso[3]
    # legs: (kind, rows, far). Side view: [far, near] (far drawn behind the torso); front view:
    # [viewer-left A, viewer-right B].
    s.legs = []
    s.feet_y = FEET
    if p['pose'] == 'sit':
        if side:
            # thigh forward (to +x = facing direction) then shin down
            s.legs = [('thigh', rect_rows(tx0 + 1, ty1 + 1, tx0 + torso_w + 3, ty1 + k(2)), False),
                      ('shin', rect_rows(tx0 + torso_w + 2, ty1 + 1, tx0 + torso_w + 3, FEET), False)]
        elif d == 'S':
            la = cx - 1 - leg_w
            lb = cx + 1
            # lap: thighs come toward the viewer as a wide block under the torso, shins below it
            s.legs = [('knee', rect_rows(la - 1, ty1 + 1, la + leg_w, ty1 + k(2)), False),
                      ('knee', rect_rows(lb - 1, ty1 + 1, lb + leg_w, ty1 + k(2)), False),
                      ('leg', rect_rows(la, ty1 + k(3), la + leg_w - 1, FEET), False),
                      ('leg', rect_rows(lb, ty1 + k(3), lb + leg_w - 1, FEET), False)]
        else:
            s.legs = []  # hidden behind the body when seated facing north
    else:
        hip_y = ty1 + 1
        if side:
            lx = cx - leg_w // 2
            s.legs = [('leg', leg_rows(lx - 1, hip_y, p['legB'], leg_w, toe=True), True),
                      ('leg', leg_rows(lx, hip_y, p['legA'], leg_w, toe=True), False)]
        else:
            gap = 2 if torso_w <= 10 else (3 if torso_w <= 13 else 5)
            la = cx - gap // 2 - leg_w - (gap % 2)
            lb = cx + (gap + 1) // 2
            s.legs = [('leg', leg_rows(la, hip_y, p['legA'], leg_w), False),
                      ('leg', leg_rows(lb, hip_y, p['legB'], leg_w), False)]
    # arms: chains of stamps; hands (x, y) = the hand pixel the equipment hangs from
    sh = ty0 + 2
    s.arms = {'A': [], 'B': []}
    s.hands = {}
    s.tool_line = None
    L = torso_h - 2 if adult else torso_h
    if huge:
        L = torso_h   # long arms, the hands low at the thighs
    if rig:
        L += rig.get('arm', 0) if adult else rnd(rig.get('arm', 0) * 0.67)
    tool = p['tool']
    if p['pose'] == 'sit':
        L = max(k(3), L - k(2))
    work = tool[:-1] if tool[:-1] in WORK else None
    kk = int(tool[-1]) - 1 if work else 0
    if side:
        # A = near arm (over the torso), B = far arm (behind it)
        ax = cx - arm_w // 2
        if tool == 'carry':
            # both arms bent forward holding something in front of the chest
            fx = s.torso[2] + 1
            s.arms['A'] = arm_path([(ax, sh), (ax, sh + 1), (fx + 1 - (w - 1), sh + k(2))], w)
            s.arms['B'] = arm_path([(ax - 1, sh), (ax - 1, sh + k(2)), (fx - (w - 1), sh + k(3))], w)
            s.hands['A'] = corner(s.arms['A'], 'br')
            s.hands['B'] = corner(s.arms['B'], 'br')
        elif tool.startswith('tend') or tool.startswith('drink'):
            # #485: both hands forward and low at a patient on the ground (tend), or cupped
            # from the water to the mouth (drink). No tool or item is drawn in them.
            idx = int(tool[-1]) - 1
            if tool.startswith('tend'):
                ha = [(s.torso[2] + k(3), FEET - k(5)), (s.torso[2] + k(2), FEET - k(7)),
                      (s.torso[2] + k(4), FEET - k(4))][idx]
                hb = [(s.torso[2] + k(2), FEET - k(4)), (s.torso[2] + k(1), FEET - k(6)),
                      (s.torso[0], FEET - k(6))][idx]
            else:
                ha = [(s.torso[2] + k(3), FEET - k(2)), (s.torso[2] + k(2), sh + k(4)),
                      (s.head[2] + 1, s.head[3] - 1)][idx]
                hb = (ha[0] - 1, ha[1] + 1)
            s.arms['A'] = arm_path([(ax, sh), (s.torso[2] - 1, sh + k(3)), (ha[0] - (w - 1), ha[1] - (w - 1))], w)
            s.arms['B'] = arm_path([(ax - 1, sh + 1), (hb[0] - (w - 1), hb[1] - (w - 1))], w)
            s.hands['A'] = corner(s.arms['A'], 'br')
            s.hands['B'] = corner(s.arms['B'], 'br')
        elif tool.startswith('talk') or tool.startswith('eat'):
            # #485: only the real talking/eating activities use these gestures. The near hand
            # rises to the mouth for the middle eating frame; talking stays below the face.
            idx = int(tool[-1]) - 1
            if tool.startswith('talk'):
                ex = s.torso[2] + [k(2), k(5), k(3)][idx]
                ey = sh + [k(3), 0, k(2)][idx]
            else:
                ex = [s.torso[2] + k(2), s.head[2] + 1, s.torso[2] + k(2)][idx]
                ey = [sh + k(4), s.head[3] - 1, sh + k(3)][idx]
            s.arms['A'] = arm_path([(ax, sh), (s.torso[2], sh + k(3)), (ex, ey)], w)
            s.arms['B'] = arm_path([(ax - 1, sh + 1), (ax - 1, sh + L - w)], w)
            s.hands['A'] = corner(s.arms['A'], 'br')
            s.hands['B'] = corner(s.arms['B'], 'bl')
        elif work == 'swing':
            if kk == 0:     # wind-up: both hands high behind the head
                s.arms['A'] = arm_path([(ax, sh), (ax - 1, sh - k(2)), (ax - k(2), sh - k(5))], w)
                s.arms['B'] = arm_path([(ax - 1, sh + 1), (ax - k(2), sh - 1), (ax - k(2), sh - k(4))], w)
                s.hands['A'] = corner(s.arms['A'], 'tl')
                s.tool_line = (s.hands['A'], (s.hands['A'][0] - 3, s.hands['A'][1] - 5))
            elif kk == 1:   # the swing: arms level and forward
                s.arms['A'] = arm_path([(ax, sh), (ax + k(2), sh), (ax + k(5), sh - 1)], w)
                s.arms['B'] = arm_path([(ax - 1, sh + 1), (ax + k(3), sh)], w)
                s.hands['A'] = corner(s.arms['A'], 'tr')
                s.tool_line = (s.hands['A'], (s.hands['A'][0] + 5, s.hands['A'][1] - 2))
            else:          # strike: arms down and forward, the head of the tool at the ground
                s.arms['A'] = arm_path([(ax, sh), (ax + k(2), sh + k(2)), (ax + k(4), sh + k(4))], w)
                s.arms['B'] = arm_path([(ax - 1, sh + 1), (ax + k(3), sh + k(4))], w)
                s.hands['A'] = corner(s.arms['A'], 'br')
                hx, hy = s.hands['A']
                s.tool_line = ((hx, hy), (hx + k(4), min(FEET - 1, hy + k(6))))
        elif work == 'stoop':
            hdx, hdy = [(k(3), k(-5)), (k(4), k(-2)), (k(2), k(-4))][kk]
            hy = FEET + hdy - (w - 1)
            s.arms['A'] = arm_path([(ax, sh), (ax + hdx - (w - 1), hy)], w)
            s.arms['B'] = arm_path([(ax - 1, sh + 1), (ax + hdx - w, hy)], w)
            s.hands['A'] = corner(s.arms['A'], 'br')
            hx, hy = s.hands['A']
            reach = [k(3), k(4), k(2)][kk]
            s.tool_line = ((hx - k(2), hy - k(3)), (hx + reach, FEET if kk == 1 else FEET - 1))
        elif work == 'strike':
            # A thrust: both hands on the haft, drawn back to the hip, driven forward at chest
            # height, then half back to guard.
            fwd = [k(1), k(6), k(3)][kk]
            yy = [sh + k(4), sh + k(1), sh + k(2)][kk]
            s.arms['A'] = arm_path([(ax, sh), (ax + fwd // 2, yy), (ax + fwd, yy)], w)
            s.arms['B'] = arm_path([(ax - 1, sh + 1), (ax + fwd // 2 - 1, yy + 1)], w)
            s.hands['A'] = corner(s.arms['A'], 'br')
            hx, hy = s.hands['A']
            s.tool_line = ((hx - k(4), hy + 1), (hx + k(5), hy - 1))
        elif work == 'tap':
            pts = [[(ax, sh), (ax + k(2), sh + k(2)), (ax + k(4), sh)],
                   [(ax, sh), (ax + k(2), sh + k(2)), (ax + k(4), sh + k(3))],
                   [(ax, sh), (ax + k(2), sh + k(2)), (ax + k(3), sh + 1)]][kk]
            s.arms['A'] = arm_path(pts, w)
            s.arms['B'] = arm_path([(ax - 1, sh + 1), (ax + 1, sh + k(4)), (ax + k(3), sh + k(4))], w)
            s.hands['A'] = corner(s.arms['A'], 'br' if kk == 1 else 'tr')
            hx, hy = s.hands['A']
            s.tool_line = ((hx, hy), [(hx - 1, hy - k(4)), (hx + k(3), hy + k(2)), (hx + k(2), hy - k(3))][kk])
        else:
            # hanging, swinging opposite the legs in the walk
            def hang(x, y, sw):
                lift = 1 if abs(sw) >= 2 else 0
                return arm_path([(x, y), (x, y + 1), (x + sw, y + L - w - lift - (y - sh))], w)
            s.arms['A'] = hang(ax, sh, p['armA'])
            s.arms['B'] = hang(ax - 1, sh + 1, p['armB'])
            s.hands['A'] = corner(s.arms['A'], 'bl')
        if 'B' not in s.hands:
            s.hands['B'] = corner(s.arms['B'], 'bl')
    else:
        axA = s.torso[0] - arm_w       # viewer-left arm
        axB = s.torso[2] + 1           # viewer-right arm (the tool hand)
        north = d == 'N'
        restA = arm_path([(axA, sh), (axA, sh + L - w + p['armA'])], w)
        restB = arm_path([(axB, sh), (axB, sh + L - w + p['armB'])], w)
        if tool == 'carry':
            # elbows out, forearms meeting at chest height
            ey = sh + k(3)
            s.arms['A'] = arm_path([(axA, sh), (axA, ey), (cx - w, ey)], w)
            s.arms['B'] = arm_path([(axB, sh), (axB, ey), (cx, ey)], w)
            s.hands['A'] = corner(s.arms['A'], 'br')
            s.hands['B'] = corner(s.arms['B'], 'bl')
        elif tool.startswith('tend') or tool.startswith('drink'):
            # #485: hands together low in front (tend), or cupped from the ground to the mouth
            # (drink); from behind the same arms show at the sides.
            idx = int(tool[-1]) - 1
            low = s.torso[3] + k(3)
            if tool.startswith('tend'):
                ya, yb = [(low, low), (low - k(2), low - k(1)), (low + k(1), low - k(3))][idx]
                xa, xb = [(cx - k(2), cx + k(1)), (cx - k(1), cx), (cx - k(3), axB + k(2))][idx]
            else:
                ya = yb = [FEET - k(3), sh + k(3), s.head[3] - 1][idx]
                xa, xb = cx - k(1), cx
            s.arms['A'] = arm_path([(axA, sh), (axA, sh + k(3)), (xa - (w - 1), ya - (w - 1))], w)
            s.arms['B'] = arm_path([(axB, sh), (axB, sh + k(3)), (xb, yb - (w - 1))], w)
            s.hands['A'] = corner(s.arms['A'], 'bl')
            s.hands['B'] = corner(s.arms['B'], 'br')
        elif tool.startswith('talk') or tool.startswith('eat'):
            # #485: a small conversational hand gesture, or food lifted to the mouth. No item,
            # emotion or social outcome is invented by the pose.
            idx = int(tool[-1]) - 1
            s.arms['A'] = restA
            if tool.startswith('talk'):
                ex = axB + [0, k(3), k(1)][idx]
                ey = sh + [k(2), -k(1), k(1)][idx]
            else:
                ex = [cx + 1, cx, cx + 1][idx]
                ey = [sh + k(2), s.head[3] - 1, sh + k(1)][idx]
            s.arms['B'] = arm_path([(axB, sh), (axB + 1, sh + k(3)), (ex, ey)], w)
            s.hands['B'] = corner(s.arms['B'], 'br')
        elif work == 'swing':
            s.arms['A'] = restA
            if kk == 0:     # tool hand raised high beside the head
                s.arms['B'] = arm_path([(axB, sh), (axB, sh - k(6))], w)
                s.hands['B'] = corner(s.arms['B'], 'tl')
                hx, hy = s.hands['B']
                s.tool_line = ((hx, hy), (hx + 1, max(1, hy - k(5))))
            elif kk == 1:   # out to the side at shoulder height
                s.arms['B'] = arm_path([(axB, sh), (axB + k(2), sh - 1)], w)
                s.hands['B'] = corner(s.arms['B'], 'tr')
                hx, hy = s.hands['B']
                s.tool_line = ((hx, hy), (hx + k(4), hy + 1))
            else:          # swung down and out in front, the tool head at the ground
                s.arms['B'] = arm_path([(axB, sh), (axB + 1, sh + k(2)), (axB + 1, sh + k(5))], w)
                s.hands['B'] = corner(s.arms['B'], 'br')
                hx, hy = s.hands['B']
                s.tool_line = ((hx, hy), (hx + k(2), min(FEET, hy + k(6))))
        elif work == 'stoop':
            hy = FEET + [k(-5), k(-2), k(-4)][kk] - (w - 1)
            inA, inB = (0, 0) if north else (1, -1)
            s.arms['A'] = arm_path([(axA, sh), (axA + inA, hy)], w)
            s.arms['B'] = arm_path([(axB, sh), (axB + inB, hy)], w)
            s.hands['B'] = corner(s.arms['B'], 'br')
            hx, hy2 = s.hands['B']
            s.tool_line = ((hx - 1, hy2 - k(3)), (hx + k(2), FEET if kk == 1 else FEET - 1))
        elif work == 'strike':
            # Facing the viewer the thrust comes toward them: the tool hand lifts to the chest
            # and the point drops toward the viewer; from behind, the arm goes forward and away.
            s.arms['A'] = restA
            lift = [k(2), k(4), k(3)][kk]
            s.arms['B'] = arm_path([(axB, sh), (axB - 1, sh + lift)], w)
            s.hands['B'] = corner(s.arms['B'], 'bl')
            hx, hy = s.hands['B']
            reach = [k(2), k(5), k(3)][kk]
            s.tool_line = ((hx, hy - k(3)), (hx - 1, hy + (reach if not north else -reach)))
        elif work == 'tap':
            if north:
                pts = [[(axB, sh), (axB + 1, sh - 1)], [(axB, sh), (axB, sh + k(5))], [(axB, sh), (axB, sh + k(3))]][kk]
                s.arms['A'] = restA
            else:
                pts = [[(axB, sh), (axB + 1, sh + k(2)), (axB + 1, sh - 1)],
                       [(axB, sh), (axB, sh + k(3)), (axB - 1, sh + k(5))],
                       [(axB, sh), (axB + 1, sh + k(3))]][kk]
                s.arms['A'] = arm_path([(axA, sh), (axA, sh + k(3)), (axA + k(2), sh + k(5))], w)
            s.arms['B'] = arm_path(pts, w)
            s.hands['B'] = corner(s.arms['B'], ['tr', 'bl', 'br'][kk] if not north else ['tr', 'br', 'br'][kk])
            hx, hy = s.hands['B']
            s.tool_line = ((hx, hy), [(hx + 1, hy - k(4)), (hx + k(2), hy + k(2)), (hx + k(2), hy - k(2))][kk])
        else:
            s.arms['A'] = restA
            s.arms['B'] = restB
        s.hands.setdefault('A', corner(s.arms['A'], 'bl'))
        s.hands.setdefault('B', corner(s.arms['B'], 'br'))
    # Wings (winged plan): folded along the back, the tips past the hips; seen from the front
    # their tops (the wrists of the folded wing) stand well above the shoulders, beside the
    # head, so the outline is a peaked cowl at 1x; from behind they cover the back's outer edges.
    s.wings = []
    if plan == 'winged':
        flap = 0
        if frame.startswith('walk') or frame.startswith('carry'):
            flap = [0, 1, 0, 0, 1, 0][int(frame[-1]) - 1]
        tx0, ty0, tx1, ty1 = s.torso
        if side:
            s.wings = [arm_path([(tx0 - k(1), ty0 - k(4) - flap), (tx0 - k(5), ty0 + k(3)),
                                 (tx0 - k(2), ty1 + k(4))], max(3, arm_w + 1))]
        else:
            ww = max(3, arm_w + 1)
            s.wings = [arm_path([(tx0, ty0 - k(5) - flap), (tx0 - k(5), ty0 + k(1)),
                                 (tx0 - k(3), ty1 + k(3))], ww),
                       arm_path([(tx1 - ww + 1, ty0 - k(5) - flap), (tx1 + k(5) - ww + 1, ty0 + k(1)),
                                 (tx1 + k(3) - ww + 1, ty1 + k(3))], ww)]
    # A second pair of arms (four-armed plan), from the waist, held clear of the trunk so the
    # outline shows four limbs at 1x: from the front and back, elbows out and the hands spread
    # wide of the hips; in profile, the near forearm forward and the far one back, level with
    # the waist. They swing a little with the walk.
    s.lower_arms = []
    if plan == 'four_armed' and p['pose'] != 'sleep':
        tx0, _, tx1, _ = s.torso
        wy = sh + k(4)
        out = k(6) if adult else k(4)
        lw = arm_w + 1 if adult else arm_w
        if side:
            sw = p['armA'] // 2
            s.lower_arms = [arm_path([(cx - 1, wy), (tx0 - out + sw, wy + k(2))], lw),
                            arm_path([(cx, wy), (tx1 + out - lw + 1 + sw, wy + 1)], lw)]
        else:
            ya, yb = p['armA'] // 2, p['armB'] // 2
            s.lower_arms = [arm_path([(tx0 - lw, wy), (tx0 - lw - k(2), wy + k(2)),
                                      (tx0 - lw - out, wy + k(5) + ya)], lw),
                            arm_path([(tx1 + 1, wy), (tx1 + 1 + k(2), wy + k(2)),
                                      (tx1 + 1 + out, wy + k(5) + yb)], lw)]
    # A tail (digitigrade plan): a chain of stamps from the base of the spine, swaying with the
    # walk. Side view: out behind and down; facing north it hangs between the legs; facing the
    # viewer only its tip shows beside the legs.
    if plan == 'digitigrade_tail' and p['pose'] != 'sit':
        tw = max(2, leg_w - 1)
        sway = 0
        if frame.startswith('walk') or frame.startswith('carry'):
            sway = k([0, 1, 1, 0, -1, -1][int(frame[-1]) - 1])
        bx, by = s.torso[0], s.torso[3] - k(2)
        if side:
            s.tail = arm_path([(bx - 1, by), (bx - k(4), by + k(3) + sway),
                               (bx - k(8), by + k(4) - sway)], tw)
        elif d == 'N':
            # from under the hem, so a garment over the hips does not hide it
            hem = s.torso[3] + k(5)
            s.tail = arm_path([(cx - tw // 2, hem), (cx - tw // 2 + sway, hem + k(3)),
                               (cx + k(4), FEET)], tw)
        else:
            s.tail = arm_path([(s.torso[2] + k(1), FEET - k(5)), (s.torso[2] + k(4), FEET - k(3) + sway)],
                              tw)
    # arms that pass in front of the torso and legs (garments are cut away under them)
    s.front_arms = ['A'] if side else ([] if d == 'N' else ['A', 'B'])
    s.chest = (cx, sh + k(3))
    return s


def torso_inset(s, y):
    """How far in from the torso box row y is drawn (each side): rounded shoulders, and from
    halfway down a waist a pixel narrower than the chest (front and back views of slim and
    average adults; broad people and children keep a straight trunk, side views their depth)."""
    tx0, ty0, tx1, ty1 = s.torso
    if CELL < 48 or s.side:
        return 0
    if y == ty0:
        return 1
    if s.age == 'adult' and s.build != 'broad' and y >= ty0 + (ty1 - ty0) * 3 // 5:
        return 1
    return 0


def lean_shifts(s):
    """Row -> x shift for the side-view lean: nothing at the hip, rising through the torso to
    `lean` px at the shoulders; the head (and anything above it) moves rigidly with them."""
    lean = s.p['lean']
    if not lean:
        return {}
    hip = s.torso[3]
    shoulder = s.torso[1]
    out = {}
    for y in range(0, hip + 1):
        t = min(1.0, (hip - y) / max(1, hip - shoulder))
        out[y] = rnd(lean * t)
    return out


def iter_poses():
    for d in ('S', 'E', 'N'):
        for f in FRAMES:
            yield d, f


# --- composite plans: the biped's upper body on a lower body of the plan's own ----------------
# The taur (64 px cell) and the serpentine (96 px cell) keep the human head, torso and arms of
# the biped rig, so every garment, hair style, tool and overlay sits on them unchanged; the rig
# is placed in the larger cell so that its hip lands on the lower body's withers or tail. The
# lower body (horse barrel and four legs, or a coiled tail) is drawn natively in the cell and is
# never sheared by a lean: only the upper body bends.
_HIP = {}


def _biped_hip(build, age, d):
    """Where the biped's hip (bottom of the torso) is at rest in its 48 px cell."""
    key = (build, age, d)
    if key not in _HIP:
        with use_cell(48):
            s = skeleton(build, age, d, 'idle', 'biped')
        _HIP[key] = (s.cx, s.torso[3])
    return _HIP[key]


def translate(s, dx, dy):
    """Move every coordinate of an upper-body skeleton by (dx, dy)."""
    def r4(b):
        return (b[0] + dx, b[1] + dy, b[2] + dx, b[3] + dy)
    s.head = r4(s.head)
    s.torso = r4(s.torso)
    s.neck = r4(s.neck) if s.neck else None
    s.arms = {n: [r4(b) for b in v] for n, v in s.arms.items()}
    s.hands = {n: (x + dx, y + dy) for n, (x, y) in s.hands.items()}
    if s.tool_line:
        s.tool_line = tuple((x + dx, y + dy) for (x, y) in s.tool_line)
    s.chest = (s.chest[0] + dx, s.chest[1] + dy)
    s.cx += dx


def walk_index(frame):
    if frame.startswith('walk') or frame.startswith('carry'):
        return int(frame[-1]) - 1
    return None


def composite_skeleton(build, age, d, frame, plan):
    """The upper body of the biped rig for this frame, placed on the plan's lower body. The
    lower body is a list of draw operations in s.lower (body layer) and s.cloth_lower (a
    blanket for long garments, taur only)."""
    upper = SLEEP_DRAW[frame] if asleep(frame) else ('idle' if frame in ('sit', 'lying') else frame)
    with use_cell(48):
        s = skeleton(build, age, d, upper, 'biped')
    s.plan, s.frame = plan, frame
    s.lower, s.cloth_lower, s.tail_px = [], [], set()
    s.upper_turn, s.upper_at = 0, None
    s.legs = []
    if plan == 'taur':
        hip = taur_lower(s, build, age, d, frame)
    else:
        hip = serpent_lower(s, build, age, d, frame)
    bx, by = _biped_hip(build, age, d)
    dx, dy = rnd(hip[0]) - bx, rnd(hip[1]) - by
    if s.upper_at:
        s.upper_at = (rnd(s.upper_at[0]), rnd(s.upper_at[1]))
    translate(s, dx, dy)
    # A tool that reached the rig's ground (a hoe's blade, a pick's head) reaches this cell's.
    if s.tool_line:
        (x0, y0), (x1, y1) = s.tool_line
        rig_ground = 48 - 3 + dy
        if y1 >= rig_ground - 1 and y1 > y0:
            ty = FEET - (rig_ground - y1)
            t = min(3.0, (ty - y0) / (y1 - y0))
            s.tool_line = ((x0, y0), (rnd(x0 + (x1 - x0) * t), rnd(y0 + (y1 - y0) * t)))
    # Garments stop a little below the waist: nothing hangs over the horse or the tail.
    s.hem_cap = s.torso[3] + (2 if plan == 'taur' else k(3))
    return s


def _ell_px(cx, cy, rx, ry):
    """Pixels of a filled ellipse, without the one-pixel nub a raster ellipse leaves at its
    top, bottom and ends."""
    rows = {}
    for y in range(int(math.floor(cy - ry)) - 1, int(math.ceil(cy + ry)) + 2):
        for x in range(int(math.floor(cx - rx)) - 1, int(math.ceil(cx + rx)) + 2):
            if ((x - cx) / (rx + 0.5)) ** 2 + ((y - cy) / (ry + 0.5)) ** 2 <= 1.0:
                rows.setdefault(y, []).append(x)
    out = []
    ys = sorted(rows)
    for y in ys:
        if (y == ys[0] or y == ys[-1]) and len(rows[y]) < 3:
            continue
        xs = rows[y]
        for x in xs:
            out.append((x, y))
    return out


def taur_lower(s, build, age, d, frame):
    """The taur's lower body; downed, the rider half has fallen back across the horse's back."""
    hip = _taur_lower(s, build, age, d, frame)
    if frame == 'lying':
        s.upper_turn = 1
        s.upper_at = (hip[0] + 2, hip[1] + 1)
    return hip


def _taur_lower(s, build, age, d, frame):
    """A horse's body: a barrel with haunch and chest, four legs and a tail. Returns the point
    the human hip is set on (the withers, at the front of the barrel)."""
    adult = age == 'adult'
    _, tw, tside, lw0, _ = DIM[age][build]
    lw = max(3, lw0 - 1)
    leg_len = 16 if adult else 11
    ry = 7 if adult else 5
    rx = 17 if adult else 12
    p = s.p
    side = d == 'E'
    kk = walk_index(frame)
    pose = 'fold' if rest_frame(frame) in ('sit', 'sleep', 'lying') else ('fall' if frame == 'fall' else 'stand')
    lift = {'fold': leg_len - 4, 'fall': k(3)}.get(pose, 0)
    cy = FEET - leg_len - ry + 3 + p['bob'] + lift
    sway = k([0, 1, 1, 0, -1, -1][kk]) if kk is not None else 0
    # front legs follow the biped's leg table; the hind pair is half a cycle behind
    front = (p['legA'], p['legB'])
    if kk is not None:
        rp = frame_params('walk%d' % ((kk + 3) % 6 + 1), side, d, age)
        rear = (rp['legA'], rp['legB'])
    else:
        rear = ((0, 0, 0), (0, 0, 0))
    if pose == 'fall':
        front = ((k(2), 0, k(2)), (k(1), 0, k(2)))
        rear = ((-k(1), 0, -k(2)), (-k(2), 0, -k(2)))
    L = s.lower
    cl = s.cloth_lower
    top = cy + ry - 3   # where the legs leave the barrel
    if side:
        bxc = CELL // 2 - 4 if adult else CELL // 2 - 2
        tail = arm_path([(bxc - rx + 1, cy - ry + 2), (bxc - rx - 3, cy + sway),
                         (bxc - rx - 3, cy + ry + 5 - sway)], 3)
        L.append(('stamps', tail, MID))
        fx, rxx = bxc + rx - 6 - lw // 2, bxc - rx + 4
        if pose == 'fold':
            # lying down: forelegs folded forward under the chest, a hind hoof under the belly
            L.append(('rect', fx - 4, FEET - 2, fx + 5, FEET - 1, SHADOW))
            L.append(('rect', fx + 4, FEET - 2, fx + 5, FEET - 1, SHADOW))
        else:
            L.append(('leg', leg_rows(rxx - 2, top, (rear[1][0], rear[1][1], rear[1][2] - k(2)), lw), SHADOW))
            L.append(('leg', leg_rows(fx - 2, top, front[1], lw), SHADOW))
        L.append(('ell', bxc, cy, rx, ry, -1))
        L.append(('ell', bxc - rx + 5, cy - 1, 6 if adult else 5, ry + 1, -1))     # haunch
        L.append(('ell', bxc + rx - 4, cy, 5 if adult else 4, ry + 1, -1))      # chest
        L.append(('ell', bxc + rx - 1, cy + 1, 3, ry - 2, -1))                 # breast
        if pose == 'fold':
            L.append(('rect', fx - 6, FEET - 1, fx + 4, FEET, -1))
            L.append(('hoof', FEET, fx + 3, fx + 4))
            L.append(('rect', rxx + 2, FEET - 1, rxx + 9, FEET, -1))
            L.append(('hoof', FEET, rxx + 8, rxx + 9))
        else:
            L.append(('leg', leg_rows(rxx, top, (rear[0][0], rear[0][1], rear[0][2] - k(2)), lw, toe=True), -1))
            L.append(('leg', leg_rows(fx, top, front[0], lw, toe=True), -1))
        cl.append(('blanket', bxc - 1, cy, rx - 2, ry, cy + 1))
        return (bxc + rx - 5, cy - ry + 3)
    cx = CELL // 2
    cw = tw // 2 + (3 if adult else 2)   # the horse's chest is broader than the rider's waist
    if d == 'S':
        if pose != 'fold':
            L.append(('leg', leg_rows(cx - cw - 2, cy - 2, (0, rear[0][1] - k(2), 0), lw), SHADOW))
            L.append(('leg', leg_rows(cx + cw - lw + 3, cy - 2, (0, rear[1][1] - k(2), 0), lw), SHADOW))
        L.append(('ell', cx, cy - 3, cw + 3, ry, -1))          # the barrel, going away
        L.append(('crease_ell', cx, cy + 1, cw, ry + 1))
        L.append(('ell', cx, cy + 1, cw, ry + 1, -1))          # the chest
        xa, xb = cx - cw + 2, cx + cw - lw - 1
        if pose == 'fold':
            for x in (xa - 1, xb):
                L.append(('rect', x, FEET - 2, x + lw, FEET, -1))
                L.append(('hoof', FEET, x, x + lw))
        else:
            L.append(('leg', leg_rows(xa, top + 1, front[0], lw), -1))
            L.append(('leg', leg_rows(xb, top + 1, front[1], lw), -1))
        cl.append(('blanket', cx, cy - 3, cw + 3, ry, cy - 4))
        return (cx, cy - ry + 1)
    # N: the hindquarters toward the viewer, the tail over them, the human back beyond
    if pose != 'fold':
        L.append(('leg', leg_rows(cx - cw + 2, cy - 2, (0, front[0][1] - k(2), 0), lw), SHADOW))
        L.append(('leg', leg_rows(cx + cw - lw - 1, cy - 2, (0, front[1][1] - k(2), 0), lw), SHADOW))
    L.append(('ell', cx, cy - 3, cw, ry, -1))
    L.append(('ell', cx, cy + 1, cw + 3, ry + 1, -1))
    xa, xb = cx - cw + 1, cx + cw - lw
    if pose == 'fold':
        for x in (xa - 2, xb + 1):
            L.append(('rect', x, FEET - 1, x + lw, FEET, -1))
            L.append(('hoof', FEET, x, x + lw))
    else:
        L.append(('leg', leg_rows(xa, top + 2, rear[0], lw), -1))
        L.append(('leg', leg_rows(xb, top + 2, rear[1], lw), -1))
    tail = arm_path([(cx - 2, cy - ry + 2), (cx - 2 + sway, cy + 2), (cx - 2 + sway, cy + ry + 6)], 4)
    L.append(('stamps', tail, SHADOW))
    s.tail_px = stamp_pixels(tail)
    cl.append(('blanket', cx, cy + 1, cw + 3, ry + 1, cy))
    return (cx, cy - ry + 1)


def catmull(pts, per=10):
    """A smooth curve through control points (Catmull-Rom), as a dense list of points."""
    if len(pts) < 2:
        return list(pts)
    P = [pts[0]] + list(pts) + [pts[-1]]
    out = []
    for i in range(1, len(P) - 2):
        p0, p1, p2, p3 = P[i - 1], P[i], P[i + 1], P[i + 2]
        seg = max(2, int(math.hypot(p2[0] - p1[0], p2[1] - p1[1]) * 2))
        for j in range(seg):
            t = j / seg
            t2, t3 = t * t, t * t * t
            out.append(tuple(0.5 * (2 * p1[q] + (-p0[q] + p2[q]) * t + (2 * p0[q] - 5 * p1[q] + 4 * p2[q] - p3[q]) * t2
                                    + (-p0[q] + 3 * p1[q] - 3 * p2[q] + p3[q]) * t3) for q in (0, 1)))
    out.append(pts[-1])
    return out


def serpent_lower(s, build, age, d, frame):
    """The tail of a serpentine body: in profile it rises from the ground to the hip; at rest
    it lies in a loose ring, in the slither cycle it trails in a travelling wave, sitting and
    asleep it is a coiled pile, downed it lies stretched out. Returns the hip point."""
    adult = age == 'adult'
    _, tw, tside, _, _ = DIM[age][build]
    side = d == 'E'
    w0 = (tside + 3 if side else tw) - (0 if adult else 1)
    H = 36 if adult else 24
    taper = [(0, 2), (0.45, w0 * 0.75), (1, w0)]
    sc = 1.0 if adult else 0.72
    p = s.p
    kk = walk_index(frame)
    L = s.lower
    G = FEET - 3     # the middle of a coil lying on the ground
    if rest_frame(frame) in ('sit', 'sleep'):
        cx = CELL // 2
        rise = 12 * sc
        top = G - rise - 2
        pts = []
        turns = 2.2
        for i in range(161):
            t = i / 160
            a = math.pi * 0.6 + 2 * math.pi * turns * t
            rxe = (22 - 13 * t) * sc
            rye = (7 - 3.5 * t) * sc
            pts.append((cx + rxe * math.cos(a), G - rise * t + rye * math.sin(a)))
        pts.append((cx, top - 2))
        L.append(('tube', pts, [(0, 2), (0.3, w0 + 1), (1, w0)], None))
        if frame == 'sit':
            hip = (cx + (2 if side else 0), top - 6)
            L.append(('tube', catmull([(cx, top - 1), hip]), [(0, w0), (1, w0)], 'S' if not side else 'E'))
            L.append(('crease',))
            return hip
        L.append(('crease',))
        # asleep: the upper body lies along the top of the coil, the head to the left
        s.upper_turn = 1
        s.upper_at = (cx + 4, top + 1)
        return (cx, H)   # placement is replaced by the turn
    if frame == 'lying':
        # downed: the tail stretched out along the ground behind the fallen body
        x0, y0 = 38, G - 6
        ctrl = [(x0, y0), (52, y0 + 4), (66, y0 - 2), (80, y0 + 4), (CELL - 6, y0 - 1)]
        L.append(('tube', list(reversed(catmull(ctrl))), [(0, 2), (0.6, w0), (1, w0)], None))
        L.append(('crease',))
        s.upper_turn = 1
        s.upper_at = (x0 + 2, y0)
        return (CELL // 2, H)
    drop = {'stoop1': 8, 'stoop2': 12, 'stoop3': 8, 'fall': k(4)}.get(frame, 0)
    hip_y = FEET - H + drop
    if side:
        cx = CELL // 2 + 12
        base = (cx - 4, G + 1)
        if kk is not None:
            ph = kk * 2 * math.pi / 6
            trail = [(x, G + 1 + 3 * math.sin(2 * math.pi * x / 26 + ph) * min(1.0, (base[0] - x) / 12))
                     for x in range(4, base[0], 2)]
            col = [base, (cx + 2 + rnd(math.sin(ph)), (hip_y + G) / 2 + 4), (cx, hip_y)]
            L.append(('tube', catmull(trail + [base]), taper, None))
        else:
            ring = []
            for i in range(73):
                t = i / 72
                a = -math.pi * 0.15 - 2 * math.pi * 0.95 * t
                ring.append((cx - 18 * sc + 18 * sc * math.cos(a), G - 1 + 5.5 * sc * math.sin(a)))
            ring = list(reversed(ring))
            L.append(('tube', ring + [base], taper, None))
            col = [base, (cx + 2, (hip_y + G) / 2 + 4), (cx, hip_y)]
        L.append(('tube', catmull(col), [(0, w0), (1, w0)], 'E'))
        L.append(('crease',))
        return (cx, hip_y)
    cx = CELL // 2
    if d == 'S':
        base = (cx, G + 1)
        if kk is not None:
            # the body trails away behind the column in a wave from side to side
            ph = kk * 2 * math.pi / 6
            trail = []
            for i in range(25):
                t = i / 24
                y = G - 26 * sc + 26 * sc * t
                amp = 17 * sc * (1 - t) ** 0.6
                trail.append((cx + amp * math.sin(2 * math.pi * t * 1.2 + ph), y))
            L.append(('tube', catmull(trail + [base]), taper, None))
        else:
            ring = []
            for i in range(73):
                t = i / 72
                a = math.pi * 0.5 + 2 * math.pi * 0.9 * t
                ring.append((cx + 19 * sc * math.cos(a) * (1 if t > 0 else 1), G - 2 + 6 * sc * math.sin(a)))
            ring = list(reversed(ring))
            L.append(('tube', ring + [base], taper, None))
        col = [base, (cx, (hip_y + G) / 2), (cx, hip_y)]
        L.append(('tube', catmull(col), [(0, w0), (1, w0)], 'S'))
        L.append(('crease',))
        return (cx, hip_y)
    # N: the column first, then the body coming toward the viewer over its foot
    hip_y -= 4
    base = (cx, G - 10)
    col = [(cx, hip_y), (cx, (hip_y + base[1]) / 2), base]
    L.append(('tube', catmull(col), [(0, w0), (1, w0)], None))
    if kk is not None:
        ph = kk * 2 * math.pi / 6
        trail = []
        for i in range(25):
            t = i / 24
            trail.append((cx + 14 * sc * t ** 0.6 * math.sin(2 * math.pi * t * 1.2 + ph), base[1] + (FEET - 2 - base[1]) * t))
        L.append(('tube', catmull([base] + trail), [(0, w0), (0.55, w0 * 0.75), (1, 2)], None))
    else:
        ring = []
        for i in range(73):
            t = i / 72
            a = -math.pi * 0.5 - 2 * math.pi * 0.85 * t
            ring.append((cx + 18 * sc * math.cos(a), G - 4 + 6 * sc * math.sin(a) + 6 * sc))
        L.append(('tube', [base] + ring, [(0, w0), (0.55, w0 * 0.75), (1, 2)], None))
    L.append(('crease',))
    return (cx, hip_y)


def _width_at(prof, t):
    for (t0, w0), (t1, w1) in zip(prof, prof[1:]):
        if t <= t1:
            return w0 + (w1 - w0) * ((t - t0) / (t1 - t0) if t1 > t0 else 1)
    return prof[-1][1]


def draw_lower(c, s, kind, arg):
    """Paint a composite plan's lower body (body layer) or its blanket (clothing layer)."""
    if kind == 'clothing':
        if arg not in ('cloak', 'robe', 'dress'):
            return
        for op in s.cloth_lower:
            _, cx, cy, rx, ry, ymax = op
            for (x, y) in _ell_px(cx, cy, rx, ry):
                if (x, y) in s.tail_px:
                    continue
                if y <= ymax:
                    c.px(x, y, 'cloth')
                elif y == ymax + 1:
                    c.px(x, y, 'trim')
        return
    if kind != 'body':
        return
    idx = np.full((c.n, c.n), -1, dtype=np.int32)
    count = [0]
    belly = []
    for op in s.lower:
        t = op[0]
        if t == 'ell':
            _, cx, cy, rx, ry, sh = op
            for (x, y) in _ell_px(cx, cy, rx, ry):
                c.px(x, y, 'skin', sh)
        elif t == 'crease_ell':
            # a shadow line where the next shape (in front) meets the one behind it
            _, cx, cy, rx, ry = op
            for (x, y) in _ell_px(cx, cy, rx + 1, ry + 1):
                if 0 <= x < c.n and 0 <= y < c.n and c.g[y, x] >= 0:
                    c.shade[y, x] = SHADOW
        elif t == 'rect':
            _, x0, y0, x1, y1, sh = op
            c.rect(x0, y0, x1, y1, 'skin', sh)
        elif t == 'hoof':
            _, y, x0, x1 = op
            c.rect(x0, y, x1, y, 'skin', SHADOW)
        elif t == 'leg':
            _, rows, sh = op
            n = len(rows)
            for j, (y, x0, x1) in enumerate(rows):
                if j < n // 3:
                    x1 += 1      # the forearm and gaskin are heavier than the cannon
                c.rect(x0, y, x1, y, 'skin', sh)
            if rows:
                y, x0, x1 = rows[-1]
                c.rect(x0, y, x1, y, 'skin', SHADOW)    # the hoof
        elif t == 'stamps':
            _, stamps, sh = op
            for j, (x0, y0, x1, y1) in enumerate(stamps):
                c.rect(x0, y0, x1, y1, 'skin', sh if j % 3 else SHADOW)
        elif t == 'tube':
            _, pts, prof, bel = op
            N = max(1, len(pts) - 1)
            for i, (x, y) in enumerate(pts):
                r = _width_at(prof, i / N) / 2
                for yy in range(int(math.floor(y - r)), int(math.ceil(y + r)) + 1):
                    for xx in range(int(math.floor(x - r)), int(math.ceil(x + r)) + 1):
                        if (xx - x) ** 2 + (yy - y) ** 2 <= r * r + 0.3 and 0 <= xx < c.n and 0 <= yy < c.n:
                            c.px(xx, yy, 'skin')
                            idx[yy, xx] = count[0]
                count[0] += 1
                if bel:
                    belly.append((x, y, r, bel))
        elif t == 'crease':
            _crease(c, idx)
    # belly plates on the rising column: across it facing the viewer, down its front in profile
    for (x, y, r, bel) in belly:
        yy = rnd(y)
        if bel == 'S':
            half = max(1, int(r * 0.45))
            sh = MID if yy % 3 == 0 else LIGHT
            for xx in range(rnd(x) - half, rnd(x) + half):
                if 0 <= xx < c.n and 0 <= yy < c.n and c.g[yy, xx] >= 0 and c.shade[yy, xx] < 0:
                    c.shade[yy, xx] = sh
        else:
            xx = rnd(x + r) - 2
            for q in (0, 1):
                if 0 <= xx - q < c.n and 0 <= yy < c.n and c.g[yy, xx - q] >= 0 and c.shade[yy, xx - q] < 0:
                    c.shade[yy, xx - q] = LIGHT if yy % 3 else MID


def _crease(c, idx):
    """Where a later stretch of the body lies over an earlier one (a coil on a coil, the column
    over the trail), the earlier one gets a shadow line along the edge so the two read apart.
    Then a scale texture: staggered shadow dots on the interior."""
    n = c.n
    painted = idx >= 0
    gap = 14
    for y in range(n):
        for x in range(n):
            if not painted[y, x]:
                continue
            me = idx[y, x]
            for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                yy, xx = y + dy, x + dx
                if 0 <= yy < n and 0 <= xx < n and painted[yy, xx] and idx[yy, xx] - me > gap:
                    c.shade[y, x] = SHADOW
                    break
    for y in range(1, n - 1):
        for x in range(1, n - 1):
            if not painted[y, x] or c.shade[y, x] >= 0:
                continue
            if not (painted[y - 1, x] and painted[y + 1, x] and painted[y, x - 1] and painted[y, x + 1]):
                continue
            if y % 2 == 0 and (x + (y // 2) * 2) % 4 == 0:
                c.shade[y, x] = SHADOW


# --- body ----------------------------------------------------------------------------------
def face_eye_row(s):
    """The row of the top of the eyes: about the middle of the head (higher on the Meer's
    flat head)."""
    return s.head[1] + (4 if s.plan == 'amphibious' else 5)


def face_eye_cols(s):
    """Left pixel of each eye in the front view, a third of the way in from each side."""
    hx0, _, hx1, _ = s.head
    inset = 3 if s.age == 'adult' else 2
    w = 2 if s.age == 'adult' else 1
    return [hx0 + inset, hx1 - inset - w + 1]


def draw_features(c, s):
    """A species' head (P3): Hollin ears long and pointed, Grisk ears wide and a broad nose,
    Screeling a snout and a scaled crest, Skarri a beak and a crest, Tharn a heavy brow."""
    sp = getattr(s, 'species', 'human')
    if sp == 'human' or s.p['pose'] == 'sleep':
        return
    hx0, hy0, hx1, hy1 = s.head
    ey = face_eye_row(s)
    d = s.d
    beak = FIXED['twine']
    adult = s.age == 'adult'
    if sp == 'hollin':
        # Long pointed ears that sweep up and out from the side of the head, the tips standing
        # two pixels above the crown and clear of any hair; in profile the near ear sweeps back.
        n = 9 if adult else 7
        if d in ('S', 'N'):
            roots = ((hx0 - 1, -1), (hx1 + 1, 1))
        else:
            roots = ((hx0 + 2, -1),)
        for base, step in roots:
            for j in range(n):
                y = ey + 1 - j
                x = base + step * (j * 4 // n if d != 'E' else j * 9 // n)
                c.px(x, y, 'skin', SHADOW if j == 0 else (LIGHT if step < 0 else MID))
                if j < n - 3:
                    c.px(x - step, y, 'skin', SHADOW if d == 'E' else MID)
    elif sp == 'grisk':
        # Long ears out to the sides, wide at the root and lifting to a point, longer than the
        # head is wide is deep: they make the Grisk's outline at any size.
        n = 8 if adult else 6
        if d in ('S', 'N'):
            roots = ((hx0 - 1, -1), (hx1 + 1, 1))
        else:
            roots = ((hx0 + 2, -1),)
        for base, step in roots:
            for i in range(n):
                x = base + step * i
                yc = ey - i // 2
                h = 2 if i < n - 2 else 1
                for yy in range(yc - h + 1, yc + 1 + (1 if i < 2 else 0)):
                    c.px(x, yy, 'skin', LIGHT if yy == yc - h + 1 else (SHADOW if i == 0 else MID))
        if d == 'S':
            c.rect(s.cx - 1, ey + 2, s.cx, ey + 3, 'skin', SHADOW)
        elif d == 'E':
            c.rect(hx1 + 1, ey + 1, hx1 + 2, ey + 2, 'skin', SHADOW)
    elif sp == 'screeling':
        if d == 'E':
            c.rect(hx1 + 1, ey + 1, hx1 + k(3), ey + 4, 'skin', MID)
            c.px(hx1 + k(3), ey + 2, 'skin', SHADOW)
        for x in range(hx0 + 2, hx1 - 1, 2):
            c.px(x, hy0, 'skin', HIGH)
    elif sp == 'skarri':
        if d == 'S':
            c.fixed_px(s.cx - 1, ey + 2, beak[2]); c.fixed_px(s.cx, ey + 2, beak[1])
            c.fixed_px(s.cx - 1, ey + 3, beak[1]); c.fixed_px(s.cx, ey + 3, beak[0])
        elif d == 'E':
            for j in range(k(3)):
                c.fixed_px(hx1 + 1 + j, ey + 2 + j // 2, beak[2 - min(2, j)])
        for j in range(k(3)):
            c.px(s.cx - 1 + (j if d == 'E' else 0), hy0 - 1 - j, 'skin', HIGH if j == k(3) - 1 else LIGHT)
    elif sp == 'tharn':
        if d != 'N':
            c.rect(hx0 + 1, ey - 2, hx1 - 1, ey - 2, 'skin', SHADOW)
            c.rect(hx0, hy1 - 1, hx1, hy1, 'skin', MID)
    elif sp == 'murrow':
        murrow_head(c, s, ey)
    elif sp == 'holm':
        holm_head(c, s, ey)
    elif sp == 'below':
        below_head(c, s, ey)


def _skin_over(c, x, y, shade=-1):
    """Paint a face pixel back to plain skin (an eye or a nose taken away)."""
    if 0 <= x < c.n and 0 <= y < c.n:
        c.fixed[y, x] = -1
        c.px(x, y, 'skin', shade)


def _no_eyes(c, s, ey):
    hx0, hy0, hx1, hy1 = s.head
    if s.d == 'S':
        for ex in face_eye_cols(s):
            for yy in range(ey - 2, ey + 3):
                for xx in (ex, ex + 1):
                    _skin_over(c, xx, yy)
    elif s.d == 'E':
        ex = hx1 - 2
        for yy in range(ey - 2, ey + 3):
            for xx in (ex, ex + 1):
                _skin_over(c, xx, yy)


def murrow_head(c, s, ey):
    """Murrow: blind, with no eyes at all (smooth skin under a lit brow) and wide fan ears,
    ribbed, pale inside, that stand out from the head to either side."""
    hx0, hy0, hx1, hy1 = s.head
    d = s.d
    _no_eyes(c, s, ey)
    adult = s.age == 'adult'
    if d == 'S':
        for ex in face_eye_cols(s):
            c.px(ex, ey, 'skin', LIGHT); c.px(ex + 1, ey, 'skin', LIGHT)
            c.px(ex, ey + 1, 'skin', MID); c.px(ex + 1, ey + 1, 'skin', SHADOW)
    # wider at the top than at the root, like a fan held up
    prof = [5, 7, 8, 7, 6, 4, 2] if adult else [4, 5, 6, 5, 3, 2]
    y0 = ey - (5 if adult else 4)

    def fan(base, step):
        for j, e in enumerate(prof):
            y = y0 + j
            for i in range(e):
                x = base + step * i
                if i == 0:
                    sh = SHADOW                     # the crease where it meets the head
                elif i == e - 1:
                    sh = MID if j < len(prof) // 2 else SHADOW
                elif j % 2 == 1 and i > 0:
                    sh = SHADOW                     # the ribs of the fan
                else:
                    sh = LIGHT if i < e - 2 else MID
                c.px(x, y, 'skin', sh)
    if d in ('S', 'N'):
        fan(hx0 - 1, -1)
        fan(hx1 + 1, 1)
    else:
        # in profile the near ear spreads back from the side of the head, over its back edge
        base = hx0 + 2
        for j, e in enumerate(prof):
            y = y0 + j
            for i in range(e):
                x = base - i
                sh = SHADOW if (i == 0 or (j % 2 == 1 and i < e - 1)) else (LIGHT if i < e - 1 else MID)
                c.px(x, y, 'skin', sh)


def holm_head(c, s, ey):
    """Holm: a jutting brow over small deep-set eyes, a broad flat nose, a heavy jaw and stone
    knobs on the crown (drawn on the 48 px rig, so every pixel is two at the giant's size)."""
    hx0, hy0, hx1, hy1 = s.head
    d = s.d
    _no_eyes(c, s, ey)
    if d == 'S':
        c.rect(hx0 + 1, ey - 2, hx1 - 1, ey - 2, 'skin', LIGHT)      # the brow ridge
        c.rect(hx0 + 2, ey - 1, hx1 - 2, ey, 'skin', SHADOW)         # the hollow under it
        for ex in face_eye_cols(s):
            c.fixed_px(ex + 1, ey, OUTLINE)                          # small eyes in the hollow
        nx = s.cx
        c.rect(nx - 2, ey + 2, nx + 1, ey + 3, 'skin', MID)
        c.px(nx - 2, ey + 2, 'skin', HIGH); c.px(nx + 1, ey + 3, 'skin', SHADOW)
        c.rect(hx0 - 1, hy1 - 3, hx0 - 1, hy1 - 1, 'skin', MID)      # the jaw, wider than the skull
        c.rect(hx1 + 1, hy1 - 3, hx1 + 1, hy1 - 1, 'skin', SHADOW)
    elif d == 'E':
        c.rect(hx1 - 3, ey - 2, hx1 + 1, ey - 2, 'skin', LIGHT)
        c.rect(hx1 - 3, ey - 1, hx1, ey - 1, 'skin', SHADOW)
        c.fixed_px(hx1 - 1, ey, OUTLINE)
        c.rect(hx1 + 1, ey + 1, hx1 + 2, ey + 3, 'skin', MID)
        c.px(hx1 + 2, ey + 3, 'skin', SHADOW)
        c.rect(hx1 - 1, hy1, hx1 + 1, hy1, 'skin', SHADOW)           # the jutting chin
    for x in (hx0 + 3, hx1 - 3):
        c.px(x, hy0 - 1, 'skin', LIGHT)                              # stone knobs


BELOW_EYE = (pal.ramp('gold', 7), pal.ramp('warm', 6))


def below_head(c, s, ey):
    """The Below: a serpent's hood spread behind the head, slit eyes that burn, no nose but
    two slits, and scales along the crown."""
    hx0, hy0, hx1, hy1 = s.head
    d = s.d
    prof = [1, 2, 3, 4, 4, 4, 4, 3, 3, 2, 1]
    y0 = ey - 2

    def hood_px(x, y, sh):
        if 0 <= x < c.n and 0 <= y < c.n and c.g[y, x] < 0:
            c.px(x, y, 'skin', sh)
    if d in ('S', 'N'):
        for j, e in enumerate(prof):
            y = y0 + j
            for i in range(e):
                sh = SHADOW if i == e - 1 or (i + j) % 3 == 0 else MID
                hood_px(hx0 - 1 - i, y, sh)
                hood_px(hx1 + 1 + i, y, sh)
    else:
        for j, e in enumerate(prof):
            for i in range(e + 1):
                hood_px(hx0 - i, y0 + j, SHADOW if i == e or (i + j) % 3 == 0 else MID)
    if d == 'S':
        _no_eyes(c, s, ey)
        for ex in face_eye_cols(s):
            c.px(ex, ey - 1, 'skin', SHADOW); c.px(ex + 1, ey - 1, 'skin', SHADOW)
            c.fixed_px(ex, ey, BELOW_EYE[0]); c.fixed_px(ex + 1, ey, OUTLINE)
            c.fixed_px(ex, ey + 1, BELOW_EYE[1]); c.fixed_px(ex + 1, ey + 1, OUTLINE)
        nx = s.cx
        for yy in (ey + 2, ey + 3):
            _skin_over(c, nx - 1, yy); _skin_over(c, nx, yy)
        c.px(nx - 2, ey + 3, 'skin', SHADOW); c.px(nx + 1, ey + 3, 'skin', SHADOW)
    elif d == 'E':
        _no_eyes(c, s, ey)
        ex = hx1 - 2
        c.px(ex, ey - 1, 'skin', SHADOW)
        c.fixed_px(ex, ey, BELOW_EYE[0]); c.fixed_px(ex + 1, ey, OUTLINE)
        c.fixed_px(ex, ey + 1, BELOW_EYE[1])
        for yy in (ey + 1, ey + 2):
            if 0 <= hx1 + 1 < c.n:
                c.clear(hx1 + 1, yy)
        c.px(hx1, ey + 3, 'skin', SHADOW)
    else:
        # from behind: the hood's two pale eye-spots
        c.px(hx0 - 2, ey + 3, 'skin', HIGH); c.px(hx1 + 2, ey + 3, 'skin', HIGH)
    for x in range(hx0 + 2, hx1 - 1, 2):
        c.px(x, hy0, 'skin', HIGH)


def draw_plan_features(c, s):
    """Parts of a plan that every body sheet of it carries: the amphibious plan's webbed
    hands and feet, gills and fin; the winged plan's taloned feet."""
    if s.plan == 'winged':
        # a bird's foot: toes spread wide on the ground, a back toe in profile
        for (kind, rows, far) in s.legs:
            if kind in ('leg', 'shin') and rows:
                y, x0, x1 = rows[-1]
                xs = range(x0 - 1, x1 + 4) if s.side else range(x0 - 2, x1 + 3)
                for i, x in enumerate(xs):
                    c.px(x, y, 'skin', SHADOW if i % 2 == 0 else MID)
                if s.side:
                    c.px(x0 - 2, y, 'skin', SHADOW)
                else:
                    c.px(x0 - 2, y - 1, 'skin', MID); c.px(x1 + 2, y - 1, 'skin', MID)
        return
    if s.plan != 'amphibious':
        return
    hx0, hy0, hx1, hy1 = s.head
    ey = face_eye_row(s)
    d = s.d
    if s.p['pose'] != 'sleep':
        # gills: three short slits low on each side of the head
        if d == 'E':
            for x in (hx0 + 4, hx0 + 6):
                c.px(x, ey + 3, 'skin', SHADOW); c.px(x, ey + 4, 'skin', SHADOW)
                c.px(x + 1, ey + 3, 'skin', LIGHT)
        else:
            for x0, step in ((hx0, 1), (hx1, -1)):
                for yy in (ey + 2, ey + 4):
                    c.px(x0, yy, 'skin', SHADOW); c.px(x0 + step, yy, 'skin', SHADOW)
                    c.px(x0, yy + 1, 'skin', LIGHT)
            # gill fronds: three short frills out from each side of the jaw, rising at the tips
            n = 4 if s.age == 'adult' else 3
            for x0, step in ((hx0 - 1, -1), (hx1 + 1, 1)):
                for f in range(3):
                    y = ey + 1 + f * 2
                    for i in range(n - (f == 2)):
                        c.px(x0 + step * i, y - (i == n - 1), 'skin', LIGHT if i == n - 1 else MID)
        if d == 'S':
            nx = s.cx                                                   # a wide mouth
            my = ey + (5 if s.age == 'adult' else 4)
            c.fixed_px(nx - 2, my - 1, OUTLINE); c.fixed_px(nx + 1, my - 1, OUTLINE)
        # webbed hands: broad paddles, the fingers spread two pixels past the wrist on each
        # side, dark toes of the fingers and the pale web between them
        for name in s.arms:
            if not s.arms[name]:
                continue
            x0, y0, x1, y1 = s.arms[name][-1]
            for yy in (y1, y1 + 1, y1 + 2):
                xs = range(x0 - 2, x1 + 3) if yy > y1 else range(x0 - 1, x1 + 2)
                for i, x in enumerate(xs):
                    if yy == y1 + 2:
                        c.px(x, yy, 'skin', SHADOW if i % 2 == 0 else HIGH)
                    elif not (x0 <= x <= x1 and yy == y1):
                        c.px(x, yy, 'skin', LIGHT if i % 2 else MID)
        # a fin crest: a ridge on the crown from the front, a sawtooth fin along the head and
        # down the nape in profile, standing clear of the skull
        if d in ('S', 'N'):
            for j, x in enumerate(range(s.cx - 2, s.cx + 2)):
                hgt = 4 if j in (1, 2) else 2
                for yy in range(hy0 - hgt, hy0):
                    c.px(x, yy, 'skin', HIGH if yy == hy0 - hgt else (LIGHT if x < s.cx else MID))
        else:
            for x in range(hx0 - 1, hx1 - 1):
                hgt = 3 - ((x - hx0) % 3)
                for yy in range(hy0 - hgt, hy0):
                    c.px(x, yy, 'skin', HIGH if yy == hy0 - hgt else MID)
            tx0, ty0, _, _ = s.torso
            for yy in list(range(hy0 + 2, hy1)) + list(range(ty0, ty0 + k(5))):
                back = hx0 if yy < hy1 else tx0
                ext = 3 if (yy - hy0) % 3 else 2
                for x in range(back - ext, back):
                    c.px(x, yy, 'skin', SHADOW if x == back - ext else MID)
    # webbed feet: wide paddles, the toes dark and the web pale between them
    for (kind, rows, far) in s.legs:
        if kind in ('leg', 'shin') and rows:
            y, x0, x1 = rows[-1]
            if s.side:
                xs = range(x0 - 2, x1 + 6)
            else:
                xs = range(x0 - 3, x1 + 4)
            for i, x in enumerate(xs):
                c.px(x, y, 'skin', SHADOW if i % 2 == 0 else HIGH)
            for x in xs[1:-1]:
                c.px(x, y - 1, 'skin', MID)
            c.px(xs[0], y - 2, 'skin', MID); c.px(xs[-1], y - 2, 'skin', MID)


def draw_body(c, s):
    d = s.d
    hx0, hy0, hx1, hy1 = s.head
    # draw order matters for side view: far arm, far leg, torso, head, near leg, near arm

    def arm(name):
        for (x0, y0, x1, y1) in s.arms[name]:
            c.rect(x0, y0, x1, y1, 'skin')

    def leg(i):
        if i < len(s.legs):
            kind, rows, far = s.legs[i]
            n = len(rows)
            for j, (y, x0, x1) in enumerate(rows):
                # 48 px: the thigh is a pixel fuller than the calf (outer side in front views)
                if CELL >= 48 and kind == 'leg' and j < n // 3 and not s.side:
                    if x1 < s.cx:
                        x0 -= 0
                    else:
                        x1 += 0
                c.rect(x0, y, x1, y, 'skin', SHADOW if far else -1)
    def tail():
        for (x0, y0, x1, y1) in s.tail:
            c.rect(x0, y0, x1, y1, 'skin', SHADOW)

    def wings():
        for chain in s.wings:
            for j, (x0, y0, x1, y1) in enumerate(chain):
                c.rect(x0, y0, x1, y1, 'skin', SHADOW if j % 3 == 2 else MID)

    def lower_arms():
        for chain in s.lower_arms:
            for (x0, y0, x1, y1) in chain:
                c.rect(x0, y0, x1, y1, 'skin', SHADOW)
    if s.d != 'N':
        tail()   # behind the body, except from behind
        wings()
    lower_arms()
    if s.side:
        arm('B'); leg(0)
    else:
        for i in range(len(s.legs)):
            leg(i)
    tx0, ty0, tx1, ty1 = s.torso
    for y in range(ty0, ty1 + 1):
        i = torso_inset(s, y)
        c.rect(tx0 + i, y, tx1 - i, y, 'skin')
    if s.neck:
        c.rect(*s.neck, 'skin')
    if s.side:
        leg(1)
    # head: rounded box (rounder at 48 px; the face draws the nose)
    c.blob(hx0, hy0, hx1, hy1, 'skin', round2=CELL >= 48)
    if s.side:
        arm('A')
    else:
        arm('A'); arm('B')
    # face (48 px): brows, eyes two pixels tall with a glint on the lit side, a lit lower lid
    # (so eyes read on the darkest skin, measured by contrast_report.py), a nose lit on the
    # left and shaded on the right, a mouth over a lit chin, and ears.
    ey = face_eye_row(s)
    glint = pal.neutral('light')
    adult = s.age == 'adult'
    if d == 'S':
        for ex in face_eye_cols(s):
            if adult:
                c.px(ex, ey - 2, 'skin', SHADOW); c.px(ex + 1, ey - 2, 'skin', SHADOW)   # brow
                c.fixed_px(ex, ey, glint); c.fixed_px(ex + 1, ey, OUTLINE)
                c.fixed_px(ex, ey + 1, OUTLINE); c.fixed_px(ex + 1, ey + 1, OUTLINE)
                c.px(ex, ey + 2, 'skin', HIGH); c.px(ex + 1, ey + 2, 'skin', HIGH)       # lower lid
            else:
                c.fixed_px(ex, ey, OUTLINE); c.fixed_px(ex, ey + 1, OUTLINE)
                c.px(ex, ey + 2, 'skin', HIGH)
        nx = s.cx
        c.px(nx - 1, ey + 2, 'skin', HIGH); c.px(nx, ey + 2, 'skin', SHADOW)           # nose
        if adult:
            c.px(nx, ey + 3, 'skin', SHADOW)
            my = min(ey + 5, hy1)
            c.px(hx0 + 1, ey + 3, 'skin', SHADOW); c.px(hx1 - 1, ey + 3, 'skin', SHADOW)  # cheeks
        else:
            my = ey + 4
        c.fixed_px(nx - 1, my, OUTLINE); c.fixed_px(nx, my, OUTLINE)                   # mouth
        if s.frame == 'talk2':
            c.fixed_px(nx - 1, my + 1, OUTLINE); c.fixed_px(nx, my + 1, OUTLINE)       # speech, not emotion
        else:
            c.px(nx - 1, my + 1, 'skin', HIGH); c.px(nx, my + 1, 'skin', HIGH)           # lit chin
        for xx in (hx0 - 1, hx1 + 1):                                                    # ears
            c.px(xx, ey, 'skin'); c.px(xx, ey + 1, 'skin', SHADOW)
    elif d == 'E':
        ex = hx1 - 2
        if adult:
            c.px(ex, ey - 2, 'skin', SHADOW); c.px(ex + 1, ey - 2, 'skin', SHADOW)
        c.fixed_px(ex, ey, OUTLINE); c.fixed_px(ex, ey + 1, OUTLINE)
        c.px(ex, ey + 2, 'skin', HIGH)
        c.px(hx1 + 1, ey + 1, 'skin', MID); c.px(hx1 + 1, ey + 2, 'skin', SHADOW)         # nose
        my = ey + (5 if adult else 4)
        c.fixed_px(hx1 - 1, my, OUTLINE)                                                # mouth
        if s.frame == 'talk2':
            c.fixed_px(hx1 - 1, my + 1, OUTLINE)
        c.px(hx0 + 3, ey, 'skin', SHADOW); c.px(hx0 + 3, ey + 1, 'skin', SHADOW)         # ear
    elif d == 'N':
        for xx in (hx0 - 1, hx1 + 1):
            c.px(xx, ey, 'skin', SHADOW); c.px(xx, ey + 1, 'skin', SHADOW)
    if s.d == 'N':
        tail()
        wings()
    draw_features(c, s)
    # feet: darker bottom row so the stance reads
    for (kind, rows, far) in s.legs:
        if kind in ('leg', 'shin') and rows:
            y, x0, x1 = rows[-1]
            for x in range(x0, x1 + 1):
                c.px(x, y, 'skin', SHADOW)
    draw_plan_features(c, s)


# --- hair ----------------------------------------------------------------------------------
def draw_hair(c, s, style):
    d = s.d
    hx0, hy0, hx1, hy1 = s.head
    side = s.side
    w = hx1 - hx0 + 1
    top = hy0
    back_x = hx0 if side else None   # back of the head is viewer-left in E view (facing +x)
    if style == 'crop':
        c.rect(hx0, top, hx1, top + k(2), 'hair')
        c.clear(hx0, top); c.clear(hx1, top)
        if d == 'N':
            c.rect(hx0, top, hx1, top + k(4), 'hair'); c.clear(hx0, top); c.clear(hx1, top)
        if side:
            c.rect(hx0, top, hx0 + 1, top + k(4), 'hair')
    elif style == 'bowl':
        c.rect(hx0, top, hx1, top + k(3), 'hair')
        c.clear(hx0, top); c.clear(hx1, top)
        c.rect(hx0, top + k(3), hx0, top + k(4), 'hair'); c.rect(hx1, top + k(3), hx1, top + k(4), 'hair')
        if d == 'N':
            c.rect(hx0, top, hx1, top + k(5), 'hair'); c.clear(hx0, top); c.clear(hx1, top)
        if side:
            c.rect(hx0, top, hx0 + 1, top + k(5), 'hair')
    elif style == 'long':
        c.rect(hx0, top, hx1, top + k(2), 'hair')
        c.clear(hx0, top); c.clear(hx1, top)
        if d == 'S':
            c.rect(hx0 - 1, top + k(2), hx0, hy1 + k(3), 'hair'); c.rect(hx1, top + k(2), hx1 + 1, hy1 + k(3), 'hair')
        elif d == 'N':
            c.rect(hx0 - 1, top, hx1 + 1, hy1 + k(4), 'hair')
            c.clear(hx0 - 1, top); c.clear(hx1 + 1, top); c.clear(hx0 - 1, hy1 + k(4)); c.clear(hx1 + 1, hy1 + k(4))
        else:
            c.rect(hx0 - 1, top, hx0 + k(2), hy1 + k(4), 'hair'); c.clear(hx0 - 1, top); c.clear(hx0 - 1, hy1 + k(4))
    elif style == 'ponytail':
        c.rect(hx0, top, hx1, top + k(2), 'hair')
        c.clear(hx0, top); c.clear(hx1, top)
        if d == 'N':
            c.rect(hx0, top, hx1, top + k(4), 'hair'); c.clear(hx0, top); c.clear(hx1, top)
            c.rect(s.cx - 1, top + k(4), s.cx, hy1 + k(5), 'hair')
        elif side:
            c.rect(hx0, top, hx0 + 1, top + k(4), 'hair')
            c.rect(hx0 - k(2), top + k(2), hx0 - 1, hy1 + k(4), 'hair'); c.clear(hx0 - k(2), hy1 + k(4))
    elif style == 'bun':
        c.rect(hx0, top, hx1, top + k(2), 'hair')
        c.clear(hx0, top); c.clear(hx1, top)
        if d == 'N':
            c.rect(hx0, top, hx1, top + k(4), 'hair'); c.clear(hx0, top); c.clear(hx1, top)
            c.blob(s.cx - k(2), top + k(2), s.cx + 1, top + k(5), 'hair')
        elif side:
            c.rect(hx0, top, hx0 + 1, top + k(4), 'hair')
            c.blob(hx0 - k(3), top + 1, hx0 - 1, top + k(3), 'hair')
        else:
            c.rect(s.cx - 1, top - 1, s.cx, top - 1, 'hair')
    elif style == 'balding':
        if d == 'S':
            c.rect(hx0, top + k(2), hx0, top + k(4), 'hair'); c.rect(hx1, top + k(2), hx1, top + k(4), 'hair')
        elif d == 'N':
            c.rect(hx0, top + k(2), hx1, top + k(5), 'hair')
        else:
            c.rect(hx0, top + k(2), hx0 + 1, top + k(5), 'hair'); c.rect(hx1 - 1, top + k(2), hx1 - 1, top + k(3), 'hair')
    elif style == 'braids':
        c.rect(hx0, top, hx1, top + k(2), 'hair')
        c.clear(hx0, top); c.clear(hx1, top)
        if d == 'S':
            c.rect(hx0 - 1, top + k(2), hx0 - 1, hy1 + k(5), 'hair'); c.rect(hx1 + 1, top + k(2), hx1 + 1, hy1 + k(5), 'hair')
            c.px(hx0 - k(2), hy1 + k(4), 'hair'); c.px(hx1 + k(2), hy1 + k(4), 'hair')
        elif d == 'N':
            c.rect(hx0, top, hx1, top + k(4), 'hair'); c.clear(hx0, top); c.clear(hx1, top)
            c.rect(hx0 - 1, top + k(2), hx0 - 1, hy1 + k(5), 'hair'); c.rect(hx1 + 1, top + k(2), hx1 + 1, hy1 + k(5), 'hair')
            c.px(hx0 - k(2), hy1 + k(4), 'hair'); c.px(hx1 + k(2), hy1 + k(4), 'hair')
        else:
            c.rect(hx0, top, hx0 + 1, top + k(4), 'hair')
            c.rect(hx0 - 1, top + k(2), hx0 - 1, hy1 + k(5), 'hair'); c.px(hx0 - k(2), hy1 + k(4), 'hair')
    elif style == 'crest':
        if side:
            c.rect(hx0 + 1, top - k(2), hx1 - 1, top, 'hair')
            c.rect(hx0, top, hx0 + 1, top + k(3), 'hair')
        else:
            c.rect(s.cx - 1, top - k(2), s.cx, top + 1, 'hair')
            if d == 'N':
                c.rect(s.cx - 1, top - k(2), s.cx, top + k(5), 'hair')
    elif style == 'curly':
        c.blob(hx0 - 1, top - 1, hx1 + 1, top + k(3), 'hair')
        if d == 'N':
            c.blob(hx0 - 1, top - 1, hx1 + 1, hy1, 'hair')
        elif side:
            c.blob(hx0 - k(2), top - 1, hx0 + k(2), hy1 - 1, 'hair')
        else:
            c.px(hx0 - 1, top + k(4), 'hair'); c.px(hx1 + 1, top + k(4), 'hair')
    elif style == 'fringe':
        c.rect(hx0, top, hx1, top + k(3), 'hair')
        c.clear(hx0, top); c.clear(hx1, top)
        if d == 'S':
            c.px(s.cx - k(2), top + k(3), 'hair', LIGHT); c.clear(s.cx, top + k(3))
            c.rect(hx0 - 1, top + k(2), hx0, hy1 + k(2), 'hair'); c.rect(hx1, top + k(2), hx1 + 1, hy1 + k(2), 'hair')
        elif d == 'N':
            c.rect(hx0 - 1, top, hx1 + 1, hy1 + k(2), 'hair'); c.clear(hx0 - 1, top); c.clear(hx1 + 1, top)
        else:
            c.rect(hx0 - 1, top, hx0 + k(2), hy1 + k(2), 'hair'); c.clear(hx0 - 1, top)
            c.rect(hx1 - 1, top, hx1, top + k(2), 'hair')
    # specular highlight on the crown
    if style != 'balding':
        c.px(hx0 + 1 if not side else hx0 + k(2), top if style not in ('crest', 'curly') else top - 1, 'hair', HIGH)
    if CELL >= 48:
        hair_texture(c, s, style)


def hair_texture(c, s, style):
    """48 px: strands and a sheen. Every few columns a strand line in shadow runs down the
    hair (leaning with the rows so it reads as hair, not stripes), and a short streak of light
    sits on the lit (upper-left) side of the crown."""
    g = GID['hair']
    ys, xs = np.nonzero(c.g == g)
    if len(ys) == 0:
        return
    inside = set(zip(ys.tolist(), xs.tolist()))
    curly = style == 'curly'
    for y, x in sorted(inside):
        if c.shade[y, x] >= 0 or c.fixed[y, x, 0] >= 0:
            continue
        # interior only: the renderer's edge light and shade stay as they are
        if not all((y + dy, x + dx) in inside for dy, dx in ((0, 1), (0, -1), (1, 0), (-1, 0))):
            continue
        if curly:
            if (x + 2 * y) % 4 == 0:
                c.shade[y, x] = SHADOW
            elif (x + 2 * y) % 4 == 2 and y < s.head[1] + 4:
                c.shade[y, x] = LIGHT
        elif (x - y // 4) % 3 == 0 and y > s.head[1] + 1:
            c.shade[y, x] = SHADOW
    top = s.head[1]
    sx = s.head[0] + (3 if not s.side else 4)
    for i, (dx, dy) in enumerate(((0, 1), (1, 1), (2, 2))):
        if (top + dy, sx + dx) in inside and c.shade[top + dy, sx + dx] < 0:
            c.shade[top + dy, sx + dx] = HIGH if i < 2 else LIGHT


# --- clothing -------------------------------------------------------------------------------
# Sleeve length per outfit in rows from the shoulder (None = full length, 0 = bare arms).
SLEEVE = {'tunic': 3, 'dress': 2, 'apron': 3, 'cloak': None, 'coat': None, 'workshirt': 4,
          'robe': None, 'vest': 0}


def sleeve_stamps(stamps, length, w):
    """The prefix of an arm chain a sleeve of `length` rows covers."""
    if length is None:
        return list(stamps)
    if length <= 0:
        return []
    return list(stamps[:max(1, length - w + 1)])


def stamp_pixels(stamps):
    out = set()
    for (x0, y0, x1, y1) in stamps:
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                out.add((x, y))
    return out


def draw_clothing(c, s, outfit):
    d = s.d
    tx0, ty0, tx1, ty1 = s.torso
    side = s.side
    sit = s.p['pose'] == 'sit'
    legs = s.legs
    w = s.arm_w

    def arms(group, length=None):
        """Cover arm chains. length: rows from the shoulder to cover (None = all)."""
        for name in ('B', 'A') if side else ('A', 'B'):
            for (x0, y0, x1, y1) in sleeve_stamps(s.arms[name], length, w):
                c.rect(x0, y0, x1, y1, group)

    def trousers(group):
        for (kind, rows, far) in legs:
            body = rows if kind in ('knee', 'thigh') else rows[:-1]
            if kind not in ('knee', 'thigh') and len(rows) == 1:
                body = rows
            for (y, x0, x1) in body:
                c.rect(x0, y, x1, y, group, SHADOW if far else -1)

    def boots():
        if s.plan == 'amphibious':
            return   # barefoot: the webbed feet show
        for (kind, rows, far) in legs:
            if kind in ('leg', 'shin') and rows:
                y, x0, x1 = rows[-1]
                c.rect(x0, y, x1, y, 'boot', SHADOW if far else -1)

    def torso(group, extra=0, hem=None):
        hem = ty1 if hem is None else hem
        for y in range(ty0, hem + 1):
            i = torso_inset(s, min(y, ty1)) if not extra else 0
            c.rect(tx0 - extra + i, y, tx1 + extra - i, y, group)

    def skirt(group, hem, flare):
        """Trapezoid from the torso bottom to `hem`, widening by `flare` total per side."""
        rows = max(1, hem - ty1)
        for i in range(rows + 1):
            y = ty1 + i
            f = rnd(flare * i / rows)
            c.rect(tx0 - f, y, tx1 + f, y, group)

    hem_long = FEET if not sit else ty1 + k(2)
    hem_knee = min(ty1 + k(4), FEET - 1) if not sit else ty1 + 1
    hem_thigh = min(ty1 + k(3), FEET - k(2)) if not sit else ty1 + 1
    if sit and side:
        hem_long = ty1 + k(2)
    # A taur or serpentine body: long garments stop over the join (hem_cap), and a dress's hem
    # band is drawn at its own hem, not on the ground.
    cap = getattr(s, 'hem_cap', None)
    ground = FEET
    if cap is not None:
        hem_long, hem_knee, hem_thigh = min(hem_long, cap), min(hem_knee, cap), min(hem_thigh, cap)
        ground = hem_long
    # far arm first in side view so the torso overlaps it
    if outfit == 'tunic':
        trousers('trim'); boots()
        torso('cloth', hem=hem_thigh)
        arms('cloth', length=3)
        if d != 'N':
            c.rect(tx0, ty1 - 1, tx1, ty1 - 1, 'trim', SHADOW)   # belt
        if d == 'S':
            c.clear(s.cx - 1, ty0)  # v-neck: body shows through
    elif outfit == 'dress':
        torso('cloth')
        skirt('cloth', ground if not sit else ty1 + k(3), 3 if not side else 2)
        arms('cloth', length=2)
        if not sit:
            fl = k(3) if cap is None else (3 if not side else 2)
            c.rect(tx0 - fl, ground, tx1 + fl, ground, 'trim')       # hem band
            c.rect(tx0 - fl, ground - 1, tx1 + fl, ground - 1, 'trim', LIGHT)
        if d != 'N':
            c.rect(tx0, ty1 - k(2), tx1, ty1 - k(2), 'trim', SHADOW)  # waist band
            c.rect(s.cx - 1, ty0, s.cx, ty0, 'trim')         # neckline
    elif outfit == 'apron':
        trousers('cloth'); boots()
        torso('cloth', hem=hem_knee)
        arms('cloth', length=3)
        if d != 'N':
            # trim apron panel from chest to the knee, narrower than the torso
            c.rect(tx0 + 1, ty0 + k(2), tx1 - 1, hem_knee + 1, 'trim')
            c.px(tx0 + 1, ty0 + 1, 'trim', SHADOW); c.px(tx1 - 1, ty0 + 1, 'trim', SHADOW)  # straps
        else:
            c.rect(tx0, ty0 + 1, tx1, ty0 + 1, 'trim', SHADOW)   # tie across the back
    elif outfit == 'cloak':
        # wide trapezoid from shoulders to the ankles, arms inside, hood bunched at the neck
        flare = 2 if not side else 1
        torso('cloth', extra=1)
        skirt('cloth', hem_long, flare)
        c.rect(tx0 - 1, ty0 - 1, tx1 + 1, ty0 + 1, 'trim')   # hood (down) over the shoulders
        c.rect(tx0 - k(2), ty0, tx1 + k(2), ty0, 'trim', SHADOW)
        if d != 'N':
            c.rect(s.cx - 1, ty0 + k(2), s.cx, ty1, 'trim', SHADOW)  # front opening
    elif outfit == 'coat':
        trousers('trim'); boots()
        torso('cloth', extra=0, hem=hem_knee)
        skirt('cloth', hem_knee, 1)
        arms('cloth')
        if d != 'N':
            c.rect(s.cx - 1, ty0, s.cx - 1, hem_knee, 'trim')   # lapel/button line
            c.rect(tx0, ty0, tx1, ty0, 'trim', LIGHT)         # collar
        else:
            c.rect(tx0, ty0, tx1, ty0, 'trim', LIGHT)
        # cuffs
        for name in s.arms:
            x0, y0, x1, y1 = s.arms[name][-1]
            c.rect(x0, y1, x1, y1, 'trim')
    elif outfit == 'workshirt':
        trousers('trim'); boots()
        torso('cloth', hem=ty1)
        arms('cloth', length=4)
        if d != 'N':
            c.rect(tx0, ty0, tx1, ty0, 'trim')            # collar
            c.px(s.cx, ty0 + k(4), 'trim', SHADOW)           # chest patch
            c.rect(tx0, ty1, tx1, ty1, 'boot')            # belt
        for name in s.arms:
            sl = sleeve_stamps(s.arms[name], 4, w)
            if len(sl) < len(s.arms[name]):
                x0, y0, x1, y1 = sl[-1]
                c.rect(x0, y1, x1, y1, 'cloth', LIGHT)    # rolled sleeve
    elif outfit == 'robe':
        torso('cloth')
        skirt('cloth', hem_long, 1 if not side else 0)
        # wide sleeves: arm stamps one pixel wider on the outside
        for name in s.arms:
            for (x0, y0, x1, y1) in s.arms[name]:
                if side:
                    c.rect(x0 - 1, y0, x1 + 1, y1, 'cloth')
                else:
                    c.rect(x0 - (1 if name == 'A' else 0), y0, x1 + (1 if name == 'B' else 0), y1, 'cloth')
        for name in s.arms:
            x0, y0, x1, y1 = s.arms[name][-1]
            c.rect(x0 - (0 if side else (1 if name == 'A' else 0)), y1 - 1, x1 + (0 if side else (1 if name == 'B' else 0)), y1, 'trim')
        if d != 'N':
            c.rect(tx0, ty1 - k(2), tx1, ty1 - 1, 'trim')      # sash
            c.rect(s.cx - 1, ty0, s.cx, ty0 + k(2), 'trim', SHADOW)  # neckline
        else:
            c.rect(tx0, ty1 - k(2), tx1, ty1 - k(2), 'trim')
    elif outfit == 'vest':
        trousers('trim'); boots()
        torso('cloth', hem=ty1)
        if d != 'N':
            c.rect(tx0, ty0, tx0, ty1, 'trim'); c.rect(tx1, ty0, tx1, ty1, 'trim')  # side panels
            for yy in (ty0, ty0 + 1):
                c.clear(s.cx - 1, yy); c.clear(s.cx, yy)   # open neck: body shows through
            c.px(s.cx, ty0 + k(3), 'trim', SHADOW); c.px(s.cx, ty0 + k(5), 'trim', SHADOW)  # buttons
        else:
            c.rect(s.cx - 1, ty0, s.cx, ty1, 'trim', SHADOW)
        # arms bare: nothing drawn on them
    if CELL >= 48:
        cloth_folds(c, s, outfit)
    # in side view the near arm must sit over the torso cover: redraw its sleeve
    if side and outfit not in ('vest', 'cloak'):
        if outfit == 'robe':
            for (x0, y0, x1, y1) in s.arms['A']:
                c.rect(x0 - 1, y0, x1 + 1, y1, 'cloth')
            x0, y0, x1, y1 = s.arms['A'][-1]
            c.rect(x0 - 1, y1 - 1, x1 + 1, y1, 'trim')
        else:
            for (x0, y0, x1, y1) in sleeve_stamps(s.arms['A'], SLEEVE[outfit], w):
                c.rect(x0, y0, x1, y1, 'cloth')
            if outfit == 'coat':
                x0, y0, x1, y1 = s.arms['A'][-1]
                c.rect(x0, y1, x1, y1, 'trim')
    # A bare forearm or hand in front of the garment shows through it: cut the garment away
    # there (no outline in the hole), so the body layer's arm is seen, not lost under the cloth.
    if outfit != 'cloak':
        for name in s.front_arms:
            covered = stamp_pixels(sleeve_stamps(s.arms[name], SLEEVE[outfit], w))
            for (x, y) in stamp_pixels(s.arms[name]) - covered:
                if 0 <= x < CELL and 0 <= y < CELL and c.g[y, x] >= 0:
                    c.clear(x, y)
                    c.noline[y, x] = True

    # On the human robe the original cuff filled the entire terminal arm stamp,
    # including the grip. Leave the last part of the hand visible past the cuff.
    if outfit == 'robe' and s.plan == 'biped' and s.species == 'human':
        for name in (('A',) if s.side else s.arms):
            chain = s.arms[name]
            x0, y0, x1, y1 = chain[-1]
            prev = next((stamp for stamp in reversed(chain) if stamp != chain[-1]), chain[-1])
            dx, dy = x0 - prev[0], y0 - prev[1]
            for y in range(y0, y1 + 1):
                for x in range(x0, x1 + 1):
                    if abs(dx) > abs(dy):
                        distal = x >= (x0 + x1) // 2 if dx > 0 else x <= (x0 + x1 + 1) // 2
                    else:
                        distal = y >= (y0 + y1) // 2 if dy >= 0 else y <= (y0 + y1 + 1) // 2
                    if distal and 0 <= x < c.n and 0 <= y < c.n:
                        c.clear(x, y)
                        c.noline[y, x] = True


def cloth_folds(c, s, outfit):
    """48 px: the cloth hangs. Below the waist, skirts, robes, cloaks and coat tails get
    vertical folds in shadow every third column, with a lit ridge beside each; on the torso a
    pull line from the shoulder on the shaded side. Only interior pixels with no forced shade."""
    g = GID['cloth']
    tx0, ty0, tx1, ty1 = s.torso
    long_hem = outfit in ('dress', 'robe', 'cloak', 'coat', 'apron')
    for y in range(ty0 + 2, FEET):
        for x in range(1, CELL - 1):
            if c.g[y, x] != g or c.shade[y, x] >= 0:
                continue
            if c.g[y, x - 1] != g or c.g[y, x + 1] != g or c.g[y + 1, x] != g:
                continue
            col = x - s.cx
            if y > ty1 and long_hem:
                if col % 3 == 0:
                    c.shade[y, x] = SHADOW
                elif col % 3 == 1 and y > ty1 + 1:
                    c.shade[y, x] = LIGHT
            elif y <= ty1 and not s.side and x == tx1 - 2 and y < ty0 + 6:
                c.shade[y, x] = SHADOW


# --- equipment -------------------------------------------------------------------------------
DIRS8 = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)]


def _dir8(dx, dy):
    k = rnd(math.atan2(dy, dx) / (math.pi / 4)) % 8
    return DIRS8[k]


def draw_tool(c, item, butt, end):
    """A long-handled tool along butt -> end with its head at `end`, for any of 8 directions.
    The blade side is the one facing forward and down (the stroke direction)."""
    c.line(butt[0], butt[1], end[0], end[1], 'wood')
    ux, uy = _dir8(end[0] - butt[0], end[1] - butt[1])
    p1, p2 = (-uy, ux), (uy, -ux)
    pf = p1 if (2 * p1[0] + p1[1]) > (2 * p2[0] + p2[1]) else p2
    pb = (-pf[0], -pf[1])
    ex, ey = end

    def at(px_, k, back=0):
        return ex + px_[0] * k - ux * back, ey + px_[1] * k - uy * back
    big = CELL >= 48   # heads a pixel longer and deeper at 48 px, to sit with the bodies
    reach = (1, 2, 3) if big else (1, 2)
    if item == 'pickaxe':
        c.px(ex, ey, 'iron')
        for side_ in (pf, pb):
            for kk in reach:
                c.px(*at(side_, kk), 'iron')
            c.px(*at(side_, reach[-1], back=1), 'iron', SHADOW)
        if big:
            c.px(*at(pf, 1, back=-1), 'iron', LIGHT)
    elif item == 'axe':
        for kk in reach:
            c.px(*at(pf, kk), 'iron'); c.px(*at(pf, kk, back=1), 'iron')
            if big:
                c.px(*at(pf, kk, back=-1), 'iron')
        c.px(*at(pf, reach[-1], back=-1 if not big else -2), 'iron', LIGHT)
        if big:
            c.px(*at(pf, reach[-1], back=2), 'iron', LIGHT)
    elif item == 'hoe':
        for kk in reach:
            c.px(*at(pf, kk), 'iron')
        c.px(*at(pf, reach[-1], back=-1), 'iron')
        if big:
            c.px(*at(pf, reach[-1], back=-2), 'iron', SHADOW)
    elif item == 'spear':
        # C33: a leaf-shaped head on along the shaft's own line, past its end.
        for kk in range(1, 4 if big else 3):
            c.px(ex + ux * kk, ey + uy * kk, 'iron', LIGHT if kk == 1 else -1)
        c.px(*at(pf, 1, back=-1), 'iron', SHADOW)
        c.px(*at(pb, 1, back=-1), 'iron', SHADOW)


def draw_equipment(c, s, item):
    """Draw the held item relative to the tool hand (B in front view, A in side view)."""
    tool = s.p['tool']
    d = s.d
    hand = s.hands['A'] if s.side else s.hands['B']
    hx, hy = hand
    carried = item in ('basket', 'bundle', 'bucket', 'sack')

    def handle(x0, y0, x1, y1):
        c.line(x0, y0, x1, y1, 'wood')

    def pick_head(x, y, vertical):
        if CELL >= 48:
            if vertical:
                c.rect(x - 3, y, x + 3, y, 'iron'); c.px(x - 3, y + 1, 'iron'); c.px(x + 3, y + 1, 'iron')
                c.px(x - 1, y, 'iron', LIGHT)
            else:
                c.rect(x, y - 3, x, y + 3, 'iron'); c.px(x + 1, y - 3, 'iron'); c.px(x + 1, y + 3, 'iron')
            return
        if vertical:
            c.rect(x - k(2), y, x + k(2), y, 'iron'); c.px(x - k(2), y + 1, 'iron'); c.px(x + k(2), y + 1, 'iron')
        else:
            c.rect(x, y - k(2), x, y + k(2), 'iron'); c.px(x + 1, y - k(2), 'iron'); c.px(x + 1, y + k(2), 'iron')

    def axe_head(x, y, vertical):
        if CELL >= 48:
            if vertical:
                c.rect(x + 1, y, x + 3, y + 3, 'iron'); c.rect(x + 4, y + 1, x + 4, y + 2, 'iron', LIGHT)
            else:
                c.rect(x - 1, y - 3, x + 2, y - 1, 'iron'); c.rect(x, y - 4, x + 1, y - 4, 'iron', LIGHT)
            return
        if vertical:
            c.rect(x + 1, y, x + k(2), y + k(2), 'iron'); c.px(x + k(3), y + 1, 'iron')
        else:
            c.rect(x - 1, y - k(2), x + 1, y - 1, 'iron'); c.px(x, y - k(3), 'iron')

    def spear_head(x, y, vertical):
        # A leaf-shaped iron head (C33): a lit point, a broad middle three pixels across with
        # its shadowed and lit sides, and the socket where it meets the shaft.
        n = k(4)
        for i in range(n):
            wide = 0 < i < n - 1
            if vertical:
                yy = y - i
                c.px(x, yy, 'iron', HIGH if i == n - 1 else LIGHT)
                if wide:
                    c.px(x - 1, yy, 'iron', SHADOW); c.px(x + 1, yy, 'iron', MID)
            else:
                xx = x + i
                c.px(xx, y, 'iron', HIGH if i == n - 1 else LIGHT)
                if wide:
                    c.px(xx, y + 1, 'iron', SHADOW); c.px(xx, y - 1, 'iron', MID)

    def hoe_head(x, y, vertical):
        if vertical:
            c.rect(x - 1, y, x + k(2), y, 'iron'); c.px(x + k(2), y + 1, 'iron')
        else:
            c.rect(x, y - 1, x, y + k(2), 'iron'); c.px(x + 1, y + k(2), 'iron')

    def basket(x, y):
        c.rect(x - k(3), y - 1, x + k(2), y + k(2), 'wicker')
        c.rect(x - k(3), y - 1, x + k(2), y - 1, 'wicker', LIGHT)
        c.rect(x - 1, y - k(3), x, y - k(3), 'wicker', SHADOW); c.px(x - k(2), y - k(2), 'wicker', SHADOW); c.px(x + 1, y - k(2), 'wicker', SHADOW)
        for xx in range(x - k(3), x + k(3)):
            if (xx + y) % 2 == 0:
                c.px(xx, y + 1, 'wicker', SHADOW)

    def bucket(x, y):
        c.rect(x - k(2), y - 1, x + 1, y + k(2), 'wood')
        c.rect(x - k(2), y - 1, x + 1, y - 1, 'iron')
        c.rect(x - k(2), y + 1, x + 1, y + 1, 'iron', SHADOW)
        c.rect(x - 1, y - k(3), x, y - k(3), 'iron', SHADOW); c.px(x - k(2), y - k(2), 'iron', SHADOW); c.px(x + 1, y - k(2), 'iron', SHADOW)
        c.clear(x - k(2), y + k(2)); c.clear(x + 1, y + k(2))

    def sack(x, y):
        c.blob(x - k(3), y - k(2), x + k(2), y + k(2), 'burlap')
        c.rect(x - 1, y - k(3), x, y - k(3), 'burlap', SHADOW)
        c.rect(x - k(2), y - k(2), x + 1, y - k(2), 'twine', SHADOW)

    def bundle(x, y):
        c.rect(x - k(4), y - 1, x + k(3), y + 1, 'wood')
        c.px(x - k(4), y - 1, 'wood', LIGHT); c.px(x + k(3), y + 1, 'wood', SHADOW)
        c.rect(x - 1, y - 1, x, y + 1, 'twine')
        c.px(x - k(4), y, 'wood', SHADOW); c.px(x + k(3), y - 1, 'wood', SHADOW)

    holders = {'basket': basket, 'bucket': bucket, 'sack': sack, 'bundle': bundle}
    if tool[:-1] in WORK:
        if carried:
            # set down on the ground while working: behind in side view, beside in front view
            if s.side:
                holders[item](s.cx - k(7), FEET - k(2))
            else:
                holders[item](s.cx + k(8), FEET - k(2))
            return
        butt, end = s.tool_line
        draw_tool(c, item, butt, end)
        return
    if tool == 'carry':
        cx, cy = s.chest
        cy = s.hands['A'][1] - 1
        if item in holders:
            holders[item](cx, cy)
        else:
            # tools carried horizontally across the chest
            handle(cx - k(4), cy, cx + k(4), cy)
            if item == 'pickaxe':
                pick_head(cx + k(4), cy, False)
            elif item == 'axe':
                axe_head(cx + k(3), cy, False)
            elif item == 'hoe':
                hoe_head(cx + k(4), cy, False)
            elif item == 'spear':
                handle(cx - k(6), cy, cx + k(5), cy)
                spear_head(cx + k(6), cy, False)
        return
    if carried:
        # held down at the side (slightly below the hand)
        x, y = hx + 1, hy + k(2)
        if s.side:
            x = hx + k(2)
        holders[item](x, y)
        return
    # tools at rest: handle vertical beside the hand, head up (a spear stands taller)
    x = hx + k(2)
    top = hy - (k(10) if item == 'spear' else k(7))
    handle(x, hy + (k(4) if item == 'spear' else k(2)), x, top)
    if item == 'pickaxe':
        pick_head(x, top, True)
    elif item == 'axe':
        axe_head(x, top, True)
    elif item == 'hoe':
        hoe_head(x, top, True)
    elif item == 'spear':
        spear_head(x, top - 1, True)


# --- overlays (injury) ----------------------------------------------------------------------
def draw_overlay(c, s, kind):
    """A bandage: a strip of undyed cloth round the head and the near upper arm. Drawn in a
    fixed ramp so it reads on any skin or hair."""
    if kind != 'bandage':
        return
    hx0, hy0, hx1, hy1 = s.head
    y = hy0 + k(2)
    c.rect(hx0, y, hx1, y, 'burlap')
    c.rect(hx0, y + 1, hx1, y + 1, 'burlap', SHADOW)
    if s.d != 'N':
        c.px(hx1 + (1 if s.side else 0), y + 1, 'burlap', LIGHT)   # the knot
    segs = s.arms['A']
    if segs:
        x0, y0, x1, y1 = segs[0]
        yy = min(y0 + 1, y1)
        c.rect(x0, yy, x1, yy, 'burlap')


# --- head markings (#485): the hairless kinds' individual variation -------------------------
# A kind without hair (data/peoples/species/*.toml art.hair = false) draws its markings in the
# hair slot instead: one of three tonal patterns on the head, chosen by the person's hair-style
# gene (Look.hair, an inherited genome locus) and painted in the skin swap colours, so it is the
# person's own skin a step darker or lighter, never a new colour. Head only: the clothing is
# drawn over the body and under this slot, so a marking anywhere else would show through it.
#   (sheet set, plan, species) for every hairless kind
MARKED = [('murrow', 'biped', 'murrow'), ('skarri', 'winged', 'skarri'),
          ('screeling', 'digitigrade_tail', 'screeling'), ('amphibious', 'amphibious', 'human'),
          ('below', 'serpentine', 'below')]
MARKINGS = ['cap', 'mask', 'spots']


def draw_marks(c, s, pattern):
    """One head marking on the skin the body layer draws for this pose."""
    bc = Canvas(c.n)
    draw_body(bc, s)
    hx0, hy0, hx1, hy1 = s.head
    ey = face_eye_row(s)
    skin = GID['skin']
    for y in range(max(0, hy0 - 1), min(c.n, hy1 + 1)):
        for x in range(max(0, hx0 - 1), min(c.n, hx1 + 2)):
            if bc.g[y, x] != skin or bc.fixed[y, x, 0] >= 0:
                continue
            cap = hy0 + max(2, (hy1 - hy0) // 3)
            if pattern == 'cap':
                # a dark crown with a light edge under it, so it reads on pale and dark tones
                on, shade = y <= cap + 1, (SHADOW if y <= cap else HIGH)
            elif pattern == 'mask':
                # a dark band across the eyes with a light brow over it (none seen from behind)
                on, shade = s.d != 'N' and ey - 2 <= y <= ey + 1, (HIGH if y == ey - 2 else SHADOW)
            else:
                # light dabs two pixels wide on a hashed grid
                on = hy0 < y < hy1 and (((x // 2) * 5 + y * 3) % 7 == 0) and y % 2 == 0
                shade = HIGH
            if on:
                c.px(x, y, 'skin', shade)


# --- sheet assembly -----------------------------------------------------------------------
def draw_layer(c, s, kind, arg, frame):
    """Paint one layer of one pose onto c."""
    if kind == 'body':
        draw_body(c, s)
        if asleep(frame):
            # closed eyes: replace the eye pixels with a skin-shadow dash
            hx0, hy0, hx1, hy1 = s.head
            ey = face_eye_row(s)
            for yy in (ey, ey + 1):
                for x in range(hx0, hx1 + 1):
                    if 0 <= yy < c.n and 0 <= x < c.n and c.fixed[yy, x, 0] >= 0:
                        c.fixed[yy, x] = -1
                        c.px(x, yy, 'skin', SHADOW if yy == ey + 1 else MID)
    elif kind == 'hair':
        draw_hair(c, s, arg)
    elif kind == 'clothing':
        draw_clothing(c, s, arg)
    elif kind == 'overlay':
        draw_overlay(c, s, arg)
    elif kind == 'equipment':
        draw_equipment(c, s, arg)
    elif kind == 'marks':
        draw_marks(c, s, arg)


def frame_canvas(kind, build, age, d, frame, arg=None, plan='biped', species='human'):
    """One layer of one pose on the 48 px rig, before shading."""
    src_d = 'E' if d == 'W' else d
    draw_frame = SLEEP_DRAW[frame] if asleep(frame) else ('idle' if frame == 'lying' else frame)
    s = skeleton(build, age, src_d, draw_frame, plan, species)
    c = Canvas()
    if kind == 'bruise':
        # The side-view rig faces east before the west row is mirrored. Only
        # the near forearm is reliably visible in profile.
        if not s.side or ((d == 'E') == (arg == 'right')):
            arm = 'A' if s.side or ((d == 'S') == (arg == 'right')) else 'B'
            x0, y0, x1, y1 = s.arms[arm][-1]
            cx, cy = (x0 + x1) // 2, (y0 + y1) // 2
            for y in range(y0, y1 + 1):
                for x in range(x0, x1 + 1):
                    dx, dy = x - cx, y - cy
                    if dx * dx + dy * dy <= 5:
                        shade = 2 if abs(dx) + abs(dy) <= 1 else 3
                        c.fixed_px(x, y, pal.ramp('cool', shade))
    else:
        draw_layer(c, s, kind, arg, frame)
    shifts = lean_shifts(s)
    if shifts:
        c.shear(shifts)
    if asleep(frame) or frame == 'lying':
        c = c.lying()
    if d == 'W':
        c = c.flipped()
    return c


def composite_canvas(kind, build, age, d, frame, arg, plan, species):
    """One layer of a taur or serpentine pose in the plan's own cell: the upper body drawn as
    the biped's and bent by the lean, the lower body under it, never bent."""
    src_d = 'E' if d == 'W' else d
    with use_cell(PLAN_CELLS[plan][0]):
        s = composite_skeleton(build, age, src_d, frame, plan)
        s.species = species
        up = Canvas()
        draw_layer(up, s, kind, arg, frame)
        shifts = lean_shifts(s)
        if shifts:
            up.shear(shifts)
        if asleep(frame) and plan == 'taur':
            # dozing where it lies down: the head sunk forward onto the chest
            up = up.moved(1 if s.side else 0, k(2))
        if s.upper_turn:
            # lying down: the upper body turned so its top points left, the hip set on the tail
            n = up.n
            hx, hy = s.cx, s.torso[3]
            rx_, ry_ = (hy, n - 1 - hx) if s.upper_turn == 1 else (n - 1 - hy, hx)
            up = up.rotated(s.upper_turn).moved(s.upper_at[0] - rx_, s.upper_at[1] - ry_)
        low = Canvas()
        draw_lower(low, s, kind, arg)
        c = up.over(low)
    if d == 'W':
        c = c.flipped()
    return c


def make_frame(kind, build, age, d, frame, arg=None, plan='biped', species='human'):
    """Return a rendered RGBA cell for one layer."""
    if plan in COMPOSITE:
        c = composite_canvas(kind, build, age, d, frame, arg, plan, species)
        if kind == 'marks':
            c.noline[:] = True   # a marking lies on the skin: no outline of its own
        return c.render()
    c = frame_canvas(kind, build, age, d, frame, arg, plan, species)
    if kind in ('marks', 'bruise'):
        c.noline[:] = True
    if plan in SCALED:
        c = c.scaled(SCALED[plan])
        if kind == 'body':
            stone_detail(c)
    return c.render()


def stone_detail(c):
    """The detail pass of a scaled body, at the full cell's own pixel: single-pixel flecks of
    shade and light on the skin's interior (weathered hide; on the Holm's tones, stone), so the
    doubled figure is not all two-pixel blocks."""
    skin = GID['skin']
    g = c.g
    n = c.n
    for y in range(1, n - 1):
        for x in range(1, n - 1):
            if g[y, x] != skin or c.shade[y, x] >= 0 or c.fixed[y, x, 0] >= 0:
                continue
            if not (g[y - 1, x] == skin and g[y + 1, x] == skin and g[y, x - 1] == skin and g[y, x + 1] == skin):
                continue
            h = (x * 7 + y * 13 + (x * y) % 5) % 19
            if h == 0:
                c.shade[y, x] = SHADOW
            elif h == 9 and (x + y) % 2 == 0:
                c.shade[y, x] = LIGHT


def make_sheet(kind, build='average', age='adult', arg=None, plan='biped', species='human'):
    n = PLAN_CELLS[plan][0]   # sheets of a plan use that plan's cell
    img = np.zeros((n * 4, n * len(FRAMES), 4), dtype=np.uint8)
    for r, d in enumerate(DIRS):
        for col, f in enumerate(FRAMES):
            img[r * n:(r + 1) * n, col * n:(col + 1) * n] = make_frame(kind, build, age, d, f, arg, plan, species)
    return Image.fromarray(img, 'RGBA')


def sheet_list():
    """(filename, kind, build, age, arg, plan) for every sheet. Every layer is drawn per plan,
    build and age, so a child's hair sits on a child's head and a broad tunic on a broad torso."""
    out = []
    for plan in PLANS:
        for b in BUILDS:
            for a in AGES:
                ba = '%s_%s' % (b, a) if plan == 'biped' else '%s_%s_%s' % (plan, b, a)
                # The average adult biped's hair, clothing and equipment keep their original
                # unsuffixed names (crates/hf-sim/tests/look.rs checks those files exist); every
                # other plan, build and age carries the suffix. The renderer applies the same rule.
                sfx = '' if ba == 'average_adult' else '_' + ba
                out.append(('body_%s.png' % ba, 'body', b, a, None, plan))
                for i, st in enumerate(HAIR_STYLES, 1):
                    out.append(('hair_%02d%s.png' % (i, sfx), 'hair', b, a, st, plan))
                for o in OUTFITS:
                    out.append(('clothing_%s%s.png' % (o, sfx), 'clothing', b, a, o, plan))
                for e in EQUIPMENT:
                    out.append(('equipment_%s%s.png' % (e, sfx), 'equipment', b, a, e, plan))
                out.append(('overlay_bandage_%s.png' % ba, 'overlay', b, a, 'bandage', plan))
                # Each species drawn on this plan has its own body sheet (its head, markings).
                for sp, sp_plan in SPECIES.items():
                    if sp_plan == plan and sp != 'human':
                        out.append(('body_%s_%s_%s.png' % (sp, b, a), 'body', b, a, None, plan, sp))
                # A biped kind on a rig of its own (RIG) wears every layer drawn on that rig.
                if plan != 'biped':
                    continue
                for side in ('left', 'right'):
                    out.append(('overlay_bruise_%s_%s.png' % (side, ba), 'bruise', b, a, side, plan))
                for sp in RIG:
                    sx = '_%s_%s_%s' % (sp, b, a)
                    for i, st in enumerate(HAIR_STYLES, 1):
                        out.append(('hair_%02d%s.png' % (i, sx), 'hair', b, a, st, plan, sp))
                    for o in OUTFITS:
                        out.append(('clothing_%s%s.png' % (o, sx), 'clothing', b, a, o, plan, sp))
                    for e in EQUIPMENT:
                        out.append(('equipment_%s%s.png' % (e, sx), 'equipment', b, a, e, plan, sp))
                    out.append(('overlay_bandage%s.png' % sx, 'overlay', b, a, 'bandage', plan, sp))
    for body_set, plan, sp in MARKED:
        for b in BUILDS:
            for a in AGES:
                for i, pat in enumerate(MARKINGS, 1):
                    out.append(('marks_%s_%d_%s_%s.png' % (body_set, i, b, a), 'marks', b, a, pat, plan, sp))
    return out


def describe(fn):
    """Provenance description for one sheet file."""
    n = '%d frames x 4 dirs' % len(FRAMES)
    if fn.startswith('expressive_adult_'):
        return 'adult human, fitted ' + fn.removeprefix('expressive_adult_').removesuffix('.png').replace('_', ' ') + ', ' + n
    if fn.startswith('expressive_long_'):
        return 'broad adult human, long parted hair, ' + n
    if fn.startswith('expressive_bowl_'):
        return 'average adult human, detailed bowl cut, ' + n
    if fn.startswith('body_'):
        return 'body layer, skin swap source colours, ' + n
    if fn.startswith('hair_'):
        return 'hair layer (%s), hair swap source colours, %s' % (HAIR_STYLES[int(fn[5:7]) - 1], n)
    if fn.startswith('clothing_'):
        return 'clothing layer, cloth + trim swap source colours, ' + n
    if fn.startswith('equipment_'):
        return 'equipment layer, fixed palette ramps, ' + n
    if fn.startswith('marks_'):
        return 'head marking (%s) of a hairless kind, skin swap source colours, %s' % (MARKINGS[int(fn.split('_')[2]) - 1], n)
    if fn.startswith('overlay_bruise_'):
        return 'untreated forearm bruise, anatomical side and pose, ' + n
    if fn.startswith('overlay_'):
        return 'bandage overlay, fixed palette ramps, ' + n
    return 'swap scheme: skin(6) hair(8) cloth(12) targets, all palette hex'


CLOTH_TRIM = {   # trim set paired with each cloth primary (contrasting, same palette)
    'undyed': 'moss', 'brown': 'ochre', 'rust': 'undyed', 'madder': 'ochre', 'ochre': 'brown',
    'moss': 'undyed', 'leaf': 'brown', 'teal': 'sky', 'sky': 'indigo', 'indigo': 'undyed',
    'violet': 'ochre', 'slate': 'rust',
}


def carry_lift(plan):
    """Px from the bottom of the cell up to the chest of the average adult holding a load,
    facing the viewer, less the same for the biped."""
    def lift(p):
        n = PLAN_CELLS[p][0]
        if p in COMPOSITE:
            with use_cell(n):
                s = composite_skeleton('average', 'adult', 'S', 'hold', p)
            return n - 1 - s.chest[1]
        s = skeleton('average', 'adult', 'S', 'hold', p)
        f = SCALED.get(p, 1)
        return n - 1 - (s.chest[1] * f + f - 1)
    return lift(plan) - lift('biped')


def swaps_doc():
    """{"skin": {"source": [4], "targets": [[4] x 6]}, "hair": {... x 8},
    "cloth": {"source": [4 primary + 4 trim], "targets": [[8] x 12]}}; every colour is a palette hex."""
    def hexes(cols):
        return [rgb_to_hex(c) for c in cols]
    return {
        'skin': {'source': hexes(SWAP_SOURCES['skin']),
                 'targets': [hexes(v) for v in SKIN_TONES.values()],
                 'names': list(SKIN_TONES)},
        'hair': {'source': hexes(SWAP_SOURCES['hair']),
                 'targets': [hexes(v) for v in HAIR_COLOURS.values()],
                 'names': list(HAIR_COLOURS)},
        'cloth': {'source': hexes(SWAP_SOURCES['cloth']) + hexes(SWAP_SOURCES['trim']),
                  'targets': [hexes(CLOTH_COLOURS[k]) + hexes(CLOTH_COLOURS[CLOTH_TRIM[k]]) for k in CLOTH_COLOURS],
                  'names': list(CLOTH_COLOURS)},
        'steps': ['shadow', 'mid', 'light', 'highlight'],
        'outline': rgb_to_hex(OUTLINE),
        'hair_styles': {'hair_%02d' % i: st for i, st in enumerate(HAIR_STYLES, 1)},
        'layout': {'cell': CELL, 'rows': DIRS, 'columns': FRAMES},
        # Per plan: its sheets' cell, its footprint in tiles (a 2-tile plan's cell is centred on
        # a 2 x 2 block around the person's tile) and how many px higher its chest (where a
        # carried load is held) sits above the cell's bottom than the 48 px biped's.
        'plan_cells': {p: PLAN_CELLS[p][0] for p in PLANS},
        'plan_tiles': {p: PLAN_CELLS[p][1] for p in PLANS},
        'plan_carry_lift': {p: carry_lift(p) for p in PLANS},
        'note': 'Layers are painted in source colours; swap source[i] -> target[i]. Hair, clothing, '
                'equipment and overlay sheets are drawn per build and age on the same skeleton as the body '
                'sheet: <layer>_<build>_<age>.png, except that the average adult hair, clothing and '
                'equipment sheets have no suffix.',
    }


def main():
    os.makedirs(OUT, exist_ok=True)
    keep = set()
    # `people.py PREFIX` redraws only the sheets whose names start with PREFIX (a full run takes
    # about 25 minutes); every other sheet is left as it is and still counts as written.
    only = sys.argv[1] if len(sys.argv) > 1 else ''
    for fn, kind, b, a, arg, plan, *sp in sheet_list():
        if fn.startswith(only):
            make_sheet(kind, b, a, arg, plan, sp[0] if sp else 'human').save(os.path.join(OUT, fn))
        keep.add(fn)
    import expressive_biped
    for layer in ('body', 'hair', 'clothing'):
        fn = 'expressive_bowl_%s.png' % layer
        if fn.startswith(only):
            expressive_biped.make_sheet(sys.modules[__name__], layer).save(os.path.join(OUT, fn))
        keep.add(fn)
    for layer in ('body', 'hair'):
        fn = 'expressive_long_%s.png' % layer
        if fn.startswith(only):
            expressive_biped.make_sheet(sys.modules[__name__], layer, 'broad', 'long').save(os.path.join(OUT, fn))
        keep.add(fn)
    for build in ('slim', 'average', 'broad'):
        layers = [('hair', style) for style in HAIR_STYLES
                  if (build, style) not in [('average', 'bowl'), ('broad', 'long')]]
        if build == 'slim':
            layers.append(('body', ''))
        for layer, style in layers:
            fn = 'expressive_adult_%s_%s.png' % (build, style or layer)
            if fn.startswith(only):
                expressive_biped.make_sheet(sys.modules[__name__], layer, build, style).save(os.path.join(OUT, fn))
            keep.add(fn)
    print('wrote %d sheets (%d frames x 4 dirs each)' % (len(keep), len(FRAMES)))
    with open(os.path.join(OUT, 'swaps.json'), 'w') as f:
        json.dump(swaps_doc(), f, indent=1)
    print('wrote swaps.json')
    stale = sorted(f for f in os.listdir(OUT) if f.endswith('.png') and f not in keep)
    if stale:
        print('not written by this generator (remove them): %s' % ' '.join(stale))
    import provenance
    provenance.refresh_folder('people', describe)


if __name__ == '__main__':
    main()
