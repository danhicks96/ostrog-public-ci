#!/usr/bin/env python3
"""Ostrog animal sprite generator.

Writes godot/assets/animals/<species>.png for every name in ANIMALS. Each sheet is 9 columns
(idle, idle2 for breathing, walk1-6, sleep) x 4 rows (S, W, E, N). Cells are 32x32 except the
deer (32x48: antlers and neck need the height) and the bear (48x48: it is the biggest animal by
far). Feet sit at the cell bottom minus 2 px; a tall cell hangs north of the tile like a tree
canopy and a wide one is centred on it (the renderer reads the cell size from the sheet: width /
9 columns, height / 4 rows).

Walks: legs are drawn as angled lines from a fixed hip to a moving foot, one pixel span per row,
so a stride reads as a leg swinging rather than a box sliding. Four-legged animals walk on
diagonal pairs (near fore with far hind) half a cycle apart, each foot planted for three frames
and swung through, lifted, for three; the body rises a pixel at mid-stance. Birds step on two
alternating feet with the head bobbing forward as each lands. The hare hops: crouch, push off,
flight, land on the forefeet, gather the hind feet under, settle.

The deep's creatures (bat, crawler, burrower) have their own draw functions on the same Canvas.
The bat's walk frames are a wing-beat, hovering 9 px over the contact line (no shadow is drawn:
the renderer puts a blob under every animal). The crawler's legs ripple head to tail; the
burrower shuffles with its spade claws reaching forward in turn.

#423's beasts whose bodies are not skeletons are drawn the same way: the cave slime is one heaped
mass that creeps by stretching and gathering itself, the rats are a pack of six small bodies
scurrying out of step, and the cave centipede is a chain of segments rippling on its legs.

Same construction as people.py: a group grid is painted with explicit coordinates, then shaded
lit from the upper-left and given a 1-px dark outline. The E view is drawn; W is its mirror.
Colours are fixed palette ramps (animals are not palette-swapped).

Run: python3 tools/art/animals.py
"""
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pal import Palette, ROOT  # noqa: E402

OUT = os.path.join(ROOT, 'godot', 'assets', 'animals')
DIRS = ['S', 'W', 'E', 'N']
FRAMES = ['idle', 'idle2'] + ['walk%d' % i for i in range(1, 7)] + ['sleep']
BIRDS = ('chicken', 'crow', 'duck')
ANIMALS = ['deer', 'boar', 'wolf', 'hare', 'goat', 'chicken', 'fox', 'crow', 'bear', 'duck',
           'bat', 'crawler', 'burrower', 'slime', 'rats', 'centipede', 'cave_croc']
# The deep's creatures have bodies the quadruped skeleton cannot draw (wings, a many-legged
# grub, a digger built round its claws), so each has its own draw functions below.
CAVE = ('bat', 'crawler', 'burrower', 'slime', 'rats', 'centipede', 'cave_croc')
# #423's bodies that are not skeletons (the slime's one mass, the rats' many bodies, the
# centipede's chain of segments) are drawn the same way, by functions of their own.

pal = Palette.load()
OUTLINE = pal.neutral('black')
SHADOW, MID, LIGHT, HIGH = 0, 1, 2, 3


def R(ramp, *steps):
    return [pal.ramp(ramp, s) for s in steps]


# group name -> 4 shades (shadow, mid, light, highlight)
RAMPS = {
    'deer':     R('earth', 3, 4, 5, 6),
    'deer_pale': [pal.ramp('earth', 5), pal.ramp('earth', 6), pal.ramp('earth', 7), pal.neutral('light')],
    'antler':   R('earth', 2, 3, 4, 5),
    'boar':     R('earth', 0, 1, 2, 3),
    'bristle':  R('stone', 0, 1, 2, 3),
    'wolf':     R('stone', 2, 3, 4, 5),
    'wolf_pale': R('stone', 4, 5, 6, 7),
    'hare':     [pal.ramp('earth', 4), pal.ramp('earth', 5), pal.ramp('earth', 6), pal.ramp('earth', 7)],
    'hare_pale': [pal.ramp('stone', 5), pal.ramp('stone', 6), pal.ramp('stone', 7), pal.neutral('white')],
    'goat':     [pal.ramp('stone', 5), pal.ramp('stone', 6), pal.ramp('stone', 7), pal.neutral('white')],
    'horn':     R('earth', 2, 3, 4, 5),
    'hen':      [pal.ramp('earth', 5), pal.ramp('earth', 6), pal.ramp('earth', 7), pal.neutral('light')],
    'comb':     R('warm', 2, 3, 4, 5),
    'beak':     R('warm', 4, 5, 6, 7),
    'nose':     [pal.neutral('black'), pal.neutral('shadow'), pal.ramp('stone', 1), pal.ramp('stone', 2)],
    'tusk':     [pal.ramp('stone', 5), pal.ramp('stone', 6), pal.ramp('stone', 7), pal.neutral('white')],
    'fox':      R('earth', 3, 4, 5, 6),
    'fox_pale': [pal.ramp('earth', 5), pal.ramp('earth', 6), pal.ramp('earth', 7), pal.neutral('white')],
    'crow':     R('stone', 1, 2, 3, 4),
    'beak':     R('warm', 4, 5, 6, 7),
    'bear':     R('earth', 1, 2, 3, 4),
    'bear_pale': [pal.ramp('earth', 5), pal.ramp('earth', 6), pal.ramp('earth', 7), pal.neutral('white')],
    'duck':     R('earth', 3, 4, 5, 6),
    'duck_pale': [pal.ramp('earth', 5), pal.ramp('earth', 6), pal.ramp('earth', 7), pal.neutral('white')],
    'bat':      [pal.ramp('earth', 2), pal.ramp('earth', 3), pal.ramp('stone', 4), pal.ramp('stone', 5)],
    'bat_wing': [pal.ramp('earth', 0), pal.ramp('earth', 1), pal.ramp('earth', 2), pal.ramp('earth', 4)],
    'bat_eye':  R('warm', 4, 5, 6, 7),
    'crawler':  R('stone', 4, 5, 6, 7),
    'jaw':      R('earth', 2, 3, 4, 5),
    'glow':     R('cool', 3, 5, 6, 7),
    'mole':     [pal.ramp('earth', 0), pal.ramp('earth', 2), pal.ramp('earth', 3), pal.ramp('earth', 4)],
    'claw':     R('stone', 4, 5, 6, 7),
    'mole_nose': R('skin', 2, 3, 4, 5),
    'slime':    R('olive', 3, 4, 5, 6),
    'slime_core': R('olive', 0, 1, 2, 3),
    'rat':      R('stone', 1, 2, 3, 4),
    'rat_tail': R('skin', 2, 3, 4, 5),
    'centi':    R('copper', 2, 3, 4, 5),
    'centi_leg': R('earth', 1, 2, 3, 4),
    'croc':     R('olive', 2, 3, 4, 5),
}
GID = {k: i for i, k in enumerate(RAMPS)}
NAMES = list(RAMPS)


class Canvas:
    def __init__(self, w, h):
        self.w, self.h = w, h
        self.g = np.full((h, w), -1, dtype=np.int16)
        self.shade = np.full((h, w), -1, dtype=np.int8)
        self.fixed = np.full((h, w, 3), -1, dtype=np.int16)

    def px(self, x, y, grp, shade=-1):
        if 0 <= x < self.w and 0 <= y < self.h:
            self.g[y, x] = GID[grp]
            self.shade[y, x] = shade

    def rect(self, x0, y0, x1, y1, grp, shade=-1):
        for y in range(min(y0, y1), max(y0, y1) + 1):
            for x in range(min(x0, x1), max(x0, x1) + 1):
                self.px(x, y, grp, shade)

    def blob(self, x0, y0, x1, y1, grp, shade=-1):
        self.rect(x0, y0, x1, y1, grp, shade)
        if x1 - x0 >= 2 and y1 - y0 >= 2:
            for (x, y) in ((x0, y0), (x1, y0), (x0, y1), (x1, y1)):
                self.clear(x, y)

    def disc(self, cx, cy, rx, ry, grp, shade=-1):
        for y in range(int(cy - ry) - 1, int(cy + ry) + 2):
            for x in range(int(cx - rx) - 1, int(cx + rx) + 2):
                u = (x - cx) / (rx + 0.5)
                v = (y - cy) / (ry + 0.5)
                if u * u + v * v <= 1.0:
                    self.px(x, y, grp, shade)

    def line(self, x0, y0, x1, y1, grp, shade=-1):
        n = max(abs(x1 - x0), abs(y1 - y0), 1)
        for i in range(n + 1):
            self.px(round(x0 + (x1 - x0) * i / n), round(y0 + (y1 - y0) * i / n), grp, shade)

    def clear(self, x, y):
        if 0 <= x < self.w and 0 <= y < self.h:
            self.g[y, x] = -1
            self.fixed[y, x] = -1

    def fixed_px(self, x, y, rgb):
        if 0 <= x < self.w and 0 <= y < self.h:
            self.fixed[y, x] = rgb
            if self.g[y, x] < 0:
                self.g[y, x] = 0

    def flipped(self):
        c = Canvas(self.w, self.h)
        c.g = self.g[:, ::-1].copy()
        c.shade = self.shade[:, ::-1].copy()
        c.fixed = self.fixed[:, ::-1].copy()
        return c

    def render(self):
        H, W = self.h, self.w
        out = np.zeros((H, W, 4), dtype=np.uint8)
        mask = self.g >= 0
        pad = np.zeros((H + 2, W + 2), dtype=bool)
        pad[1:-1, 1:-1] = mask
        up, down, left, right = pad[:-2, 1:-1], pad[2:, 1:-1], pad[1:-1, :-2], pad[1:-1, 2:]
        for y in range(H):
            for x in range(W):
                if not mask[y, x]:
                    continue
                if self.fixed[y, x, 0] >= 0:
                    out[y, x, :3] = self.fixed[y, x]
                    out[y, x, 3] = 255
                    continue
                s = self.shade[y, x]
                if s < 0:
                    t, l, b, r = not up[y, x], not left[y, x], not down[y, x], not right[y, x]
                    if t and l:
                        s = HIGH
                    elif (t or l) and not (b or r):
                        s = LIGHT
                    elif b or r:
                        s = SHADOW
                    else:
                        s = MID
                out[y, x, :3] = RAMPS[NAMES[self.g[y, x]]][s]
                out[y, x, 3] = 255
        ol = np.zeros_like(mask)
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                ol |= pad[1 + dy:H + 1 + dy, 1 + dx:W + 1 + dx]
        ol &= ~mask
        out[ol, :3] = OUTLINE
        out[ol, 3] = 255
        return out


# --- species specs -------------------------------------------------------------------------
# body_w/body_h: side-view torso; leg_h: standing leg height; head: (w, h) side view;
# neck: (dx, dy) from the body front-top to the head centre; front_w: S/N body width
SPEC = {
    'deer':    dict(cell_h=48, body=(16, 8), leg_h=10, leg_w=2, head=(5, 6), neck=(3, -9), front_w=8,
                    coat='deer', pale='deer_pale', tail=1, ear=3),
    'boar':    dict(cell_h=32, body=(17, 10), leg_h=4, leg_w=2, head=(7, 7), neck=(4, 0), front_w=11,
                    coat='boar', pale='bristle', tail=2, ear=2),
    'wolf':    dict(cell_h=32, body=(16, 7), leg_h=7, leg_w=2, head=(7, 5), neck=(4, -2), front_w=8,
                    coat='wolf', pale='wolf_pale', tail=5, ear=2),
    'hare':    dict(cell_h=32, body=(10, 6), leg_h=3, leg_w=2, head=(4, 4), neck=(2, -2), front_w=7,
                    coat='hare', pale='hare_pale', tail=1, ear=5),
    'goat':    dict(cell_h=32, body=(13, 8), leg_h=7, leg_w=2, head=(5, 5), neck=(2, -5), front_w=7,
                    coat='goat', pale='goat', tail=2, ear=2),
    'chicken': dict(cell_h=32, body=(8, 6), leg_h=2, leg_w=1, head=(3, 3), neck=(2, -3), front_w=7,
                    coat='hen', pale='hen', tail=3, ear=0),
    'fox':     dict(cell_h=32, body=(12, 7), leg_h=6, leg_w=2, head=(5, 5), neck=(3, -4), front_w=7,
                    coat='fox', pale='fox_pale', tail=4, ear=2),
    'crow':    dict(cell_h=32, body=(7, 6), leg_h=3, leg_w=1, head=(3, 3), neck=(1, -2), front_w=6,
                    coat='crow', pale='crow', tail=3, ear=0),
    'bear':    dict(cell_h=48, cell_w=48, body=(24, 13), leg_h=9, leg_w=4, head=(9, 8), neck=(5, -1),
                    front_w=15, coat='bear', pale='bear_pale', tail=1, ear=2),
    'duck':    dict(cell_h=32, body=(8, 5), leg_h=2, leg_w=1, head=(3, 3), neck=(1, -2), front_w=6,
                    coat='duck', pale='duck_pale', tail=3, ear=0),
    # cave creatures: only the cell size is read; their own draw functions hold the rest
    'bat':      dict(cell_h=32),
    'crawler':  dict(cell_h=32),
    'burrower': dict(cell_h=32),
    'slime':    dict(cell_h=32),
    'rats':     dict(cell_h=32),
    'centipede': dict(cell_h=32),
    'cave_croc': dict(cell_h=32),
}


def _rnd(v):
    return int(np.floor(v + 0.5))


def foot_cycle(stride, lift):
    """One foot over the 6 walk frames: planted and sliding back for three (contact, mid-stance,
    push-off), then lifted and swung forward for three."""
    s, h = stride, (stride + 1) // 2
    l1 = 1 if lift else 0
    return [(s, 0), (0, 0), (-s, 0), (-h, l1), (0, lift), (h, l1)]


def gait(name, sp, frame):
    """(legs, bob, head_dx). legs: foot (dx, lift) for (front-near, back-near, front-far,
    back-far); bob: body y offset (-1 = up); head_dx: forward head bob (birds)."""
    if not frame.startswith('walk'):
        return [(0, 0)] * 4, 0, 0
    k = int(frame[-1]) - 1
    if name == 'hare':
        front = [(0, 0), (1, 1), (2, 1), (1, 0), (0, 0), (-1, 0)]
        back = [(1, 0), (-2, 0), (-2, 1), (-1, 1), (1, 1), (1, 0)]
        bob = [0, -1, -2, -1, 0, 0]
        return [front[k], back[k], front[k], back[k]], bob[k], 0
    stride = max(1, sp['leg_h'] // 3)
    lift = 2 if sp['leg_h'] >= 5 else 1
    cyc = foot_cycle(stride, lift)
    if name in BIRDS:
        near, far = cyc[k], cyc[(k + 3) % 6]
        return [near, near, far, far], 0, [1, 0, 0, 1, 0, 0][k]
    a, b = cyc[k], cyc[(k + 3) % 6]
    bob = [0, -1, 0, 0, -1, 0][k]
    # diagonal pairs: front-near with back-far, back-near with front-far
    return [a, b, b, a], bob, 0


def leg_line(c, hx, hy, fx, fy, w, grp, shade=-1):
    """A leg from the hip (hx, hy) to the foot (fx, fy), w px wide, one span per row."""
    fy = max(fy, hy)
    for y in range(hy, fy + 1):
        t = (y - hy) / (fy - hy) if fy > hy else 1.0
        x = _rnd(hx + (fx - hx) * t)
        c.rect(x, y, x + w - 1, y, grp, shade)
    return _rnd(fx), fy


def coat_details_side(c, name, bx0, by0, bx1, by1, pale):
    """Small authored markings that survive the 1x silhouette without flattening the forms."""
    if name == 'deer':
        for sx, sy in ((4, 2), (7, 1), (10, 3), (13, 2), (6, 5)):
            c.px(bx0 + sx, by0 + sy, pale, LIGHT)
    elif name == 'wolf':
        c.line(bx0 + 3, by0 + 1, bx1 - 3, by0 + 1, 'wolf', SHADOW)
        c.px(bx0 + 5, by0 + 2, pale, MID)
    elif name == 'fox':
        c.line(bx0 + 2, by1 - 2, bx1 - 2, by1 - 2, pale, LIGHT)
        c.px(bx0 + 3, by0 + 2, pale, MID)
    elif name == 'boar':
        for x in range(bx0 + 2, bx1 - 1, 3):
            c.px(x, by0 + 1, 'bristle', LIGHT)
    elif name == 'bear':
        c.rect(bx0 + 5, by1 - 2, bx1 - 5, by1 - 1, pale, SHADOW)
        c.px(bx0 + 7, by0 + 2, pale, MID)
    elif name in BIRDS:
        c.line(bx0 + 2, by0 + 2, bx1 - 2, by0 + 3, name if name != 'chicken' else 'hen', LIGHT)
        c.line(bx0 + 3, by0 + 4, bx1 - 3, by0 + 4, name if name != 'chicken' else 'hen', SHADOW)


def coat_details_front(c, name, cx, by0, by1, pale):
    """Front-facing chest, wing and ridge cues; kept separate from the side-view anatomy."""
    if name == 'deer':
        for sx, sy in ((-2, -2), (2, -1), (-1, 1), (2, 3)):
            c.px(cx + sx, by0 + (by1 - by0) // 2 + sy, pale, LIGHT)
    elif name == 'wolf':
        c.line(cx, by0 + 2, cx, by1 - 2, 'wolf_pale', LIGHT)
    elif name == 'fox':
        c.line(cx, by0 + 2, cx, by1 - 1, pale, LIGHT)
        c.px(cx - 1, by1 - 2, pale, MID)
    elif name == 'boar':
        c.line(cx - 1, by0 + 2, cx - 1, by1 - 2, 'bristle', SHADOW)
        c.px(cx + 1, by0 + 2, 'bristle', LIGHT)
    elif name == 'bear':
        c.rect(cx - 3, by1 - 2, cx + 3, by1 - 1, pale, SHADOW)
    elif name in BIRDS:
        c.line(cx - 1, by0 + 2, cx - 1, by1 - 1, name if name != 'chicken' else 'hen', LIGHT)
        c.line(cx + 1, by0 + 3, cx + 1, by1 - 1, name if name != 'chicken' else 'hen', SHADOW)


def draw_side(c, name, sp, frame):
    """E view: facing +x. Feet on the bottom row minus 2."""
    W, H = c.w, c.h
    bw, bh = sp['body']
    coat, pale = sp['coat'], sp['pale']
    feet = H - 3
    sleep = frame == 'sleep'
    legs, bob, head_dx = gait(name, sp, frame)
    cx = W // 2
    bx0 = cx - bw // 2
    if sleep:
        # lying on the side: full-length body, lowered; legs tucked as short stubs under it;
        # head resting on the ground in front, eye closed
        by1 = feet - 1
        by0 = by1 - max(4, bh - 2) + 1
        hw, hh = sp['head']
        for i in range(2):
            lx = bx0 + 3 + i * 4
            c.rect(lx, by1, lx + 2, feet, coat, SHADOW)
        c.blob(bx0, by0, bx0 + bw - 1, by1, coat)
        if pale != coat:
            c.rect(bx0 + 2, by1 - 1, bx0 + bw - 3, by1 - 1, pale, SHADOW)
        if name == 'boar':
            c.rect(bx0 + 1, by0, bx0 + bw - 4, by0, 'bristle')
        if name == 'deer':
            for (sx, sy) in ((3, 1), (7, 2), (11, 1)):
                c.px(bx0 + sx, by0 + sy, pale, LIGHT)
        if name == 'wolf':
            c.line(bx0, by0 + 1, bx0 - 3, by1, coat); c.px(bx0 - 3, by1, pale)
        hx0 = bx0 + bw - 3
        hy0 = feet - hh + 1
        c.blob(hx0, hy0, hx0 + hw - 1, hy0 + hh - 1, coat)
        c.rect(hx0 + hw - 2, hy0 + 1, hx0 + hw - 1, hy0 + 1, coat, SHADOW)   # closed eye
        if name == 'wolf':
            c.rect(hx0 + hw, hy0 + 2, hx0 + hw + 1, hy0 + hh - 1, pale); c.px(hx0 + hw + 1, hy0 + 2, 'nose')
        elif name == 'boar':
            c.px(hx0 + hw, hy0 + 3, 'nose'); c.px(hx0 + hw, hy0 + hh - 1, 'tusk')
        elif name != 'chicken':
            c.px(hx0 + hw, hy0 + hh - 2, 'nose')
        ear(c, sp, hx0, hy0, hw, coat, sleeping=True)
        if name == 'chicken':
            c.px(hx0 + hw, hy0 + 1, 'beak')
            c.px(hx0 + 1, hy0 - 1, 'comb')
        if name == 'deer':
            c.line(hx0 + 1, hy0 - 1, hx0 - 1, hy0 - 4, 'antler'); c.px(hx0, hy0 - 4, 'antler'); c.px(hx0 + 3, hy0 - 2, 'antler', SHADOW); c.px(hx0 + 3, hy0 - 3, 'antler', SHADOW)
        if name == 'goat':
            c.px(hx0, hy0 - 1, 'horn'); c.px(hx0 - 1, hy0 - 2, 'horn')
        if name == 'fox':
            # fox sleeping: curled with white belly visible, fluffy tail
            c.blob(bx0 - 1, by0 + 1, bx0 - 4, by0 + 5, coat)
            c.px(bx0 - 4, by0 + 5, pale)
            c.px(bx0 - 4, by0 + 5, 'nose')
        if name == 'crow':
            # crow sleeping: small curled body, beak tucked
            c.blob(bx0 + 1, by0 + 1, bx0 + 3, by0 + 3, coat)
            c.px(bx0 + 4, by0 + 2, 'beak')
        if name == 'bear':
            # bear sleeping: big curled body, small stub tail
            c.blob(bx0 + 1, by0 + 1, bx0 + 3, by0 + 5, coat)
            c.blob(bx0 - 2, by0 + 2, bx0, by0 + 5, coat)
        if name == 'duck':
            # duck sleeping: small curled body, beak tucked
            c.blob(bx0 + 1, by0 + 1, bx0 + 3, by0 + 4, coat)
            c.px(bx0 + 4, by0 + 3, 'beak')
        return
    by1 = feet - sp['leg_h'] + bob
    by0 = by1 - bh + 1
    bx1 = bx0 + bw - 1
    # far legs (drawn first, one shade darker via forced shadow)
    lw = sp['leg_w']
    leg_x = [bx1 - lw - 1, bx0 + 1]           # front, back
    if name in BIRDS:
        leg_x = [bx0 + bw // 2 - 1]           # one leg each side, under the middle
    for i, lx in enumerate(leg_x):
        dx, lift = legs[2 + i]
        leg_line(c, lx - 1, by1 - 1, lx - 1 + dx, feet - lift, lw, coat, SHADOW)
    # body
    c.blob(bx0, by0, bx1, by1, coat)
    if name == 'bear':
        c.blob(bx1 - 10, by0 - 2, bx1 - 3, by0 + 2, coat)        # shoulder hump
    if pale != coat:
        c.rect(bx0 + 2, by1 - 1, bx1 - 2, by1, pale)      # belly
    if name == 'boar':
        c.rect(bx0 + 1, by0, bx1 - 3, by0, 'bristle')     # bristle ridge
        c.rect(bx0 + 2, by0 + 1, bx1 - 4, by0 + 1, 'bristle', SHADOW)
    if name == 'deer':
        for (sx, sy) in ((3, 2), (6, 1), (9, 3), (12, 2), (7, 4), (11, 5)):
            c.px(bx0 + sx, by0 + sy, pale, LIGHT)          # spots
    if name == 'wolf':
        c.rect(bx0 + 2, by0, bx1 - 3, by0 + 1, coat, SHADOW)  # darker saddle
    if name == 'fox':
        c.rect(bx0 + 2, by1 - 1, bx1 - 2, by1, pale)       # white belly
    if name == 'crow':
        c.rect(bx0 + 2, by0 + 2, bx1 - 2, by0 + 3, coat, SHADOW)  # folded wing
        c.rect(bx0 + 3, by0 + 2, bx1 - 3, by0 + 2, coat, LIGHT)
    if name == 'chicken':
        c.rect(bx0 + 2, by0 + 2, bx1 - 2, by0 + 3, coat, SHADOW)  # folded wing
        c.rect(bx0 + 3, by0 + 2, bx1 - 3, by0 + 2, coat, LIGHT)
    coat_details_side(c, name, bx0, by0, bx1, by1, pale)
    # near legs
    for i, lx in enumerate(leg_x):
        dx, lift = legs[i]
        fx, fy = leg_line(c, lx, by1, lx + dx, feet - lift, lw, coat)
        c.px(fx + lw - 1, fy, coat, SHADOW)                  # hoof/paw
    # tail
    t = sp['tail']
    if name == 'wolf':
        c.line(bx0, by0 + 2, bx0 - 4, by0 + 5, coat)
        c.line(bx0 - 1, by0 + 2, bx0 - 4, by0 + 4, coat)
        c.px(bx0 - 4, by0 + 5, pale)
    elif name == 'fox':
        # bushy tail with white tip
        c.line(bx0 - 1, by0 + 2, bx0 - 5, by0 + 5, coat)
        c.line(bx0 - 2, by0 + 2, bx0 - 5, by0 + 4, coat, SHADOW)
        c.px(bx0 - 5, by0 + 5, pale)
    elif name == 'crow':
        # short stiff tail
        c.line(bx0, by0 + 2, bx0 - 2, by0 + 4, coat)
        c.line(bx0 + 1, by0 + 2, bx0 - 1, by0 + 3, coat)
        c.px(bx0 - 2, by0 + 4, pale)
    elif name == 'bear':
        # small rounded stub tail
        c.blob(bx0 - 1, by0 + 2, bx0 + 1, by0 + 5, coat)
    elif name == 'duck':
        # short stiff tail (like chicken but simpler)
        c.line(bx0, by0 + 1, bx0 - 2, by0 - 1, coat)
        c.px(bx0 - 2, by0 - 1, pale)
    elif name == 'chicken':
        c.line(bx0, by0 + 1, bx0 - 3, by0 - 2, coat)
        c.line(bx0, by0 + 2, bx0 - 2, by0 - 1, coat, SHADOW)
    elif name == 'boar':
        c.line(bx0 - 1, by0 + 1, bx0 - 2, by0 + 3, coat, SHADOW)
    elif t:
        c.rect(bx0 - 1, by0 + 1, bx0 - 1, by0 + t, pale if name != 'goat' else coat)
    # neck + head
    hw, hh = sp['head']
    ndx, ndy = sp['neck']
    hcx = bx1 + ndx + head_dx
    hcy = by0 + ndy
    hx0, hy0 = hcx - hw // 2, hcy - hh // 2
    hx1, hy1 = hx0 + hw - 1, hy0 + hh - 1
    if ndy < -2:
        # a visible neck between body top-front and the head
        c.line(bx1 - 2, by0 + 1, hx0 + 1, hy1, coat)
        c.line(bx1 - 1, by0 + 1, hx0 + 2, hy1, coat)
        if name == 'deer':
            c.line(bx1 - 3, by0 + 1, hx0, hy1, coat)
    c.blob(hx0, hy0, hx1, hy1, coat)
    # muzzle / nose
    if name == 'wolf':
        c.rect(hx1 + 1, hy0 + 2, hx1 + 2, hy1, pale)
        c.px(hx1 + 2, hy0 + 2, 'nose')
    elif name == 'deer':
        c.rect(hx1 + 1, hy1 - 1, hx1 + 1, hy1, coat)
        c.px(hx1 + 1, hy1, 'nose')
    elif name == 'boar':
        c.rect(hx1 + 1, hy0 + 3, hx1 + 1, hy1 - 1, coat)
        c.px(hx1 + 2, hy0 + 4, 'nose')
        c.px(hx1 + 1, hy1, 'tusk')                            # tusk
    elif name == 'goat':
        c.px(hx1 + 1, hy1 - 1, coat)
        c.px(hx1, hy1 + 1, 'horn', SHADOW)                    # beard
        c.px(hx1 + 1, hy1, 'nose')
    elif name == 'hare':
        c.px(hx1 + 1, hy0 + 2, 'nose')
    elif name == 'chicken':
        c.px(hx1 + 1, hy0 + 1, 'beak')
        c.px(hx0 + 1, hy0 - 1, 'comb'); c.px(hx0 + 2, hy0 - 1, 'comb', LIGHT)
        c.px(hx1 + 1, hy1, 'comb', SHADOW)                    # wattle
    # eye
    c.fixed_px(hx1 - 1 if hw > 3 else hx1, hy0 + 1, OUTLINE)
    ear(c, sp, hx0, hy0, hw, coat)
    if name == 'deer':
        antlers_side(c, hx0, hy0)
    if name == 'wolf':
        c.rect(hx1 + 1, hy0 + 2, hx1 + 2, hy1, pale); c.px(hx1 + 2, hy0 + 2, 'nose')
        c.px(hx1 + 2, hy1, 'nose')
    elif name == 'fox':
        # small muzzle with dark nose
        c.px(hx1 + 1, hy1, 'nose')
        # pointy ears
        c.px(hx0 + 1, hy0 - 1, coat)
        c.px(hx0 + hw - 2, hy0 - 1, coat, SHADOW)
        c.px(hx0 + 1, hy0 - 2, coat)
    elif name == 'crow':
        # short pointed beak
        c.px(hx1 + 1, hy0 + 1, 'beak')
    elif name == 'bear':
        # small rounded ears, a pale muzzle, dark nose
        c.px(hx0 + 1, hy0 - 1, coat); c.px(hx0 + 2, hy0 - 1, coat)
        c.px(hx0 + hw - 3, hy0 - 1, coat, SHADOW)
        c.rect(hx1 + 1, hy0 + 3, hx1 + 2, hy1 - 1, 'bear_pale')
        c.px(hx1 + 2, hy0 + 3, 'nose')
    elif name == 'duck':
        c.px(hx1 + 1, hy0 + 1, 'beak')
    if name == 'goat':
        c.line(hx0 + 1, hy0 - 1, hx0 - 1, hy0 - 3, 'horn')
        c.px(hx0 + 2, hy0 - 1, 'horn')


def ear(c, sp, hx0, hy0, hw, coat, sleeping=False):
    e = sp['ear']
    if not e:
        return
    if e >= 5:   # hare: two long ears up (back when sleeping)
        if sleeping:
            c.rect(hx0 - 4, hy0, hx0, hy0, coat); c.rect(hx0 - 4, hy0 + 1, hx0 - 1, hy0 + 1, sp['pale'], SHADOW)
        else:
            c.rect(hx0 + 1, hy0 - e, hx0 + 1, hy0 - 1, coat)
            c.rect(hx0 + 3, hy0 - e + 1, hx0 + 3, hy0 - 1, coat, SHADOW)
            c.px(hx0 + 1, hy0 - e + 1, sp['pale'])
    elif e == 3:  # deer: ear out to the back
        c.rect(hx0 - 1, hy0 - 1, hx0, hy0, coat)
        c.px(hx0 - 2, hy0 - 2, coat, LIGHT)
    else:         # small upright ears
        c.px(hx0 + 1, hy0 - 1, coat); c.px(hx0 + hw - 2, hy0 - 1, coat, SHADOW)
        if e == 2 and not sleeping:
            c.px(hx0 + 1, hy0 - 2, coat)


def antlers_side(c, hx0, hy0):
    x = hx0 + 2
    c.line(x, hy0 - 1, x - 1, hy0 - 5, 'antler')
    c.line(x - 1, hy0 - 5, x - 3, hy0 - 7, 'antler')
    c.line(x - 1, hy0 - 4, x + 1, hy0 - 7, 'antler')
    c.px(x - 2, hy0 - 3, 'antler'); c.px(x - 3, hy0 - 4, 'antler')
    x2 = hx0 + 4
    c.line(x2, hy0 - 1, x2, hy0 - 4, 'antler', SHADOW)
    c.line(x2, hy0 - 4, x2 + 2, hy0 - 6, 'antler', SHADOW)
    c.px(x2 - 1, hy0 - 3, 'antler', SHADOW)


def draw_front(c, name, sp, frame, back):
    """S view (walking toward the viewer) or N view (back=True), seen from above: the body is
    an oval running up the screen, the head at the near end (S) or far end (N), legs as stubs
    at the four corners. Feet of the near pair sit on the cell bottom minus 2."""
    W, H = c.w, c.h
    bw, bh = sp['body']
    fw = sp['front_w']
    coat, pale = sp['coat'], sp['pale']
    feet = H - 3
    cx = W // 2
    sleep = frame == 'sleep'
    legs, bob, head_dx = gait(name, sp, frame)
    hw, hh = sp['head']
    body_len = max(6, round(bw * 0.7))
    rx = fw / 2.0
    if sleep:
        # curled: a round body, the head tucked at the front (S) or hidden under the back (N)
        ry = max(2.5, body_len / 2.0 - 1)
        cy = feet - ry
        c.disc(cx, cy, rx + 0.5, ry, coat)
        if name == 'deer':
            for (sx, sy) in ((-2, -1), (1, -2), (2, 1), (-1, 2)):
                c.px(cx + sx, int(cy) + sy, pale, LIGHT)
        if name == 'boar':
            c.rect(cx - 1, int(cy - ry) + 1, cx + 1, int(cy + ry) - 1, 'bristle', SHADOW)
        if name == 'wolf':
            c.rect(cx - 2, int(cy + ry) - 1, cx + 2, int(cy + ry), coat, SHADOW)   # tail curled round
        if not back:
            hx0 = cx - hw // 2
            hy0 = feet - hh + 1
            c.blob(hx0, hy0, hx0 + hw - 1, hy0 + hh - 1, coat)
            c.px(hx0 + 1, hy0 + 1, coat, SHADOW); c.px(hx0 + hw - 2, hy0 + 1, coat, SHADOW)  # closed eyes
            ear_front(c, sp, hx0, hy0, hw, coat, flat=True)
            if name == 'chicken':
                c.px(cx, hy0 + 2, 'beak')
        else:
            ear_front(c, sp, cx - hw // 2, int(cy - ry) + 1, hw, coat, flat=True)
        return
    # layout along the screen: head at the near end (S: bottom; N: top), body oval behind it
    body_len = max(6, round(bw * 0.85))
    hx0 = cx - hw // 2
    hx1 = hx0 + hw - 1
    overlap = 2
    if back:
        hy0 = feet - (body_len + hh - overlap) + 1
        hy1 = hy0 + hh - 1
        by0 = hy1 - overlap + 1
        by1 = by0 + body_len - 1
    else:
        hy1 = feet
        hy0 = hy1 - hh + 1
        by1 = hy0 + overlap - 1
        by0 = by1 - body_len + 1
    # legs: stubs out to the sides at the shoulder and hip rows, sliding along the body in the walk
    lw = sp['leg_w']
    if sp['leg_h'] > 2:
        rows = (by1 - 2, by0 + 2) if not back else (by0 + 2, by1 - 2)   # (near pair, far pair)
        for pair, ry in enumerate(rows):
            for i, sgn in enumerate((-1, 1)):
                # legs[] is (front-near, back-near, front-far, back-far): viewer-left = near
                # side. Seen from above, a foot reaching forward moves along the body (toward
                # the viewer in S) and a lifted foot draws in under the body.
                dx, lift = legs[(0 if pair == 0 else 1) + (0 if sgn < 0 else 2)]
                if back:
                    dx = -dx
                y0 = ry + dx
                x0 = cx - fw // 2 - 1 if sgn < 0 else cx + fw // 2
                x0 += (1 if sgn < 0 else -1) * (1 if lift else 0)
                c.rect(x0, y0, x0 + min(lw, 2) - 1 if sgn > 0 else x0, y0 + 1, coat, SHADOW if (pair == 1 or sgn > 0) else MID)
                if sgn < 0 and lw > 1:
                    c.px(x0 - 1, y0 + 1, coat, SHADOW)
    else:
        for i, sgn in enumerate((-1, 1)):
            dx, lift = legs[0 if sgn < 0 else 2]
            x0 = cx + sgn * (fw // 2 + 1)
            c.px(x0, by1 - 1 + (dx if not back else -dx), 'beak' if name == 'chicken' else coat, SHADOW)
    # body oval seen from above
    cy = (by0 + by1) / 2.0
    c.disc(cx, cy, rx, (by1 - by0) / 2.0, coat)
    if name == 'deer':
        for (sx, sy) in ((-2, -3), (2, -2), (-1, 0), (2, 2), (-2, 3), (1, 4)):
            c.px(cx + sx, int(cy) + sy, pale, LIGHT)
    if name == 'boar':
        c.rect(cx, by0 + 1, cx, by1 - 2, 'bristle')
        c.rect(cx - 1, by0 + 2, cx - 1, by1 - 3, 'bristle', SHADOW)
    if name == 'wolf':
        c.rect(cx - 1, by0 + 1, cx + 1, by1 - 2, coat, SHADOW)   # dark back stripe
    if name == 'fox':
        c.rect(cx - 1, by0 + 1, cx + 1, by1 - 1, pale)        # white belly stripe
    if name == 'crow':
        c.rect(cx - 1, by0 + 1, cx + 1, by1 - 1, coat, SHADOW)     # folded wing
        c.rect(cx, by0 + 2, cx, by1 - 2, coat, LIGHT)
    if name == 'goat' or name == 'hare':
        c.rect(cx, by0 + 1, cx, by1 - 1, coat, LIGHT)
    if name == 'chicken':
        c.rect(cx - 1, by0 + 1, cx + 1, by1 - 1, coat, LIGHT)
    coat_details_front(c, name, cx, by0, by1, pale)
    # tail at the far (S) / near (N) end
    tail_y = by0 - 1 if not back else by1 + 1
    if name == 'wolf':
        if back:
            c.rect(cx, by1, cx, by1 + 3, coat); c.px(cx, by1 + 4, pale)
        else:
            c.rect(cx, by0 - 3, cx, by0, coat); c.px(cx, by0 - 3, pale)
    elif name == 'chicken':
        if back:
            c.rect(cx - 1, by1 + 1, cx + 1, by1 + 2, coat); c.px(cx, by1 + 3, coat, SHADOW)
        else:
            c.rect(cx - 1, by0 - 2, cx + 1, by0 - 1, coat, SHADOW)
    elif name == 'bear':
        if back:
            c.blob(cx - 1, by1 + 1, cx + 2, by1 + 3, coat)
        else:
            c.blob(cx - 1, by0 - 2, cx + 1, by0, coat)
    elif name == 'duck':
        if back:
            c.rect(cx - 1, by1 + 1, cx + 1, by1 + 2, coat); c.px(cx, by1 + 2, pale)
        else:
            c.rect(cx - 1, by0 - 2, cx + 1, by0 - 1, coat, SHADOW)
    elif name in ('deer', 'hare'):
        c.px(cx, tail_y, pale, LIGHT)
    elif name == 'goat':
        c.px(cx, tail_y, coat)
    elif name == 'boar':
        c.px(cx, tail_y, coat, SHADOW)
    # head
    if back:
        c.blob(hx0, hy0, hx1, hy1, coat)
        ear_front(c, sp, hx0, hy0, hw, coat)
        if name == 'deer':
            antlers_front(c, hx0, hy0, hw)
        if name == 'goat':
            horns_front(c, hx0, hx1, hy0)
        if name == 'chicken':
            c.px(cx, hy0 - 1, 'comb')
        return
    c.blob(hx0, hy0, hx1, hy1, coat)
    ey = hy0 + 1 if hh <= 4 else hy0 + 2
    if hw >= 5:
        c.fixed_px(hx0 + 1, ey, OUTLINE); c.fixed_px(hx1 - 1, ey, OUTLINE)
    else:
        c.fixed_px(hx0, ey, OUTLINE); c.fixed_px(hx1, ey, OUTLINE)
    if name == 'wolf':
        c.rect(cx - 1, hy1 - 1, cx + 1, hy1, pale); c.px(cx, hy1 - 1, 'nose')
    elif name == 'deer':
        c.rect(cx - 1, hy1 - 1, cx + 1, hy1, coat, MID); c.px(cx, hy1, 'nose')
    elif name == 'boar':
        c.rect(cx - 1, hy1 - 1, cx + 1, hy1, coat, SHADOW); c.px(cx, hy1, 'nose')
        c.px(cx - 2, hy1, 'tusk'); c.px(cx + 2, hy1, 'tusk')
    elif name == 'goat':
        c.px(cx, hy1, 'nose'); c.px(cx, hy1 + 1, 'horn', SHADOW)
    elif name == 'hare':
        c.px(cx, hy1 - 1, 'nose')
    elif name == 'chicken':
        c.px(cx, hy0 + 2, 'beak'); c.px(cx, hy0 + 3, 'comb', SHADOW)
    elif name == 'fox':
        c.px(cx, hy1, 'nose')
    elif name == 'crow':
        c.px(cx, hy1, 'beak')
    elif name == 'bear':
        c.px(cx, hy1, 'nose')
    elif name == 'duck':
        c.px(cx, hy1, 'beak')
    ear_front(c, sp, hx0, hy0, hw, coat)
    if name == 'deer':
        antlers_front(c, hx0, hy0, hw)
    if name == 'goat':
        horns_front(c, hx0, hx1, hy0)
    if name == 'chicken':
        c.px(cx, hy0 - 1, 'comb'); c.px(cx, hy0 - 2, 'comb', LIGHT)


def horns_front(c, hx0, hx1, hy0):
    c.px(hx0, hy0 - 1, 'horn'); c.px(hx1, hy0 - 1, 'horn'); c.px(hx0 - 1, hy0 - 2, 'horn'); c.px(hx1 + 1, hy0 - 2, 'horn')


def ear_front(c, sp, hx0, hy0, hw, coat, flat=False):
    e = sp['ear']
    hx1 = hx0 + hw - 1
    if not e:
        return
    if e >= 5 and flat:
        c.rect(hx0 - 1, hy0 - 1, hx0 + 1, hy0 - 1, coat); c.rect(hx1 - 1, hy0 - 1, hx1 + 1, hy0 - 1, coat, SHADOW)
    elif e >= 5:
        c.rect(hx0, hy0 - e, hx0, hy0 - 1, coat); c.rect(hx1, hy0 - e, hx1, hy0 - 1, coat, SHADOW)
        c.px(hx0, hy0 - e + 1, sp['pale']); c.px(hx1, hy0 - e + 1, sp['pale'])
    elif e == 3:
        c.rect(hx0 - 2, hy0, hx0 - 1, hy0 + 1, coat); c.rect(hx1 + 1, hy0, hx1 + 2, hy0 + 1, coat, SHADOW)
    else:
        c.px(hx0, hy0 - 1, coat); c.px(hx1, hy0 - 1, coat, SHADOW)
        if e == 2:
            c.px(hx0, hy0 - 2, coat); c.px(hx1, hy0 - 2, coat, SHADOW)


def antlers_front(c, hx0, hy0, hw):
    hx1 = hx0 + hw - 1
    for sgn, x in ((-1, hx0), (1, hx1)):
        c.line(x, hy0 - 1, x + sgn, hy0 - 4, 'antler')
        c.line(x + sgn, hy0 - 4, x + sgn * 3, hy0 - 6, 'antler')
        c.px(x + sgn * 2, hy0 - 3, 'antler')
        c.px(x + sgn, hy0 - 6, 'antler', LIGHT)


# --- cave creatures ------------------------------------------------------------------------
# Drawn on the same Canvas as the surface animals, so the shading and the 1-px outline match.
# Each creature carries one bright accent for the dark grade of the deep: the bat's ember eyes,
# the crawler's cool glow pits, the burrower's pale spade claws.

SHADE_CH = {'H': HIGH, 'L': LIGHT, 'M': MID, 'S': SHADOW, 'o': -1}


def stamp(c, x, y, rows, grp, mirror=False):
    """Paint a small template with its top-left at (x, y); letters pick the shade ('o' = auto),
    '.' is left alone. mirror flips it left-right about its own width."""
    for j, row in enumerate(rows):
        n = len(row)
        for i, ch in enumerate(row):
            if ch == '.':
                continue
            c.px(x + (n - 1 - i if mirror else i), y + j, grp, SHADE_CH[ch])


def poly(c, pts, grp, shade=-1):
    """Fill a polygon (pixel centres inside it), then trace its edges so a thin spar never
    breaks into dots."""
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    n = len(pts)
    for y in range(int(np.floor(min(ys))), int(np.ceil(max(ys))) + 1):
        for x in range(int(np.floor(min(xs))), int(np.ceil(max(xs))) + 1):
            inside = False
            for i in range(n):
                (x0, y0), (x1, y1) = pts[i], pts[(i + 1) % n]
                if (y0 > y) != (y1 > y) and x < x0 + (y - y0) * (x1 - x0) / (y1 - y0):
                    inside = not inside
            if inside:
                c.px(x, y, grp, shade)
    for i in range(n):
        (x0, y0), (x1, y1) = pts[i], pts[(i + 1) % n]
        c.line(_rnd(x0), _rnd(y0), _rnd(x1), _rnd(y1), grp, shade)


def _lerp(a, b, t):
    return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)


def walk_k(frame):
    return int(frame[-1]) - 1 if frame.startswith('walk') else -1


# --- bat --------------------------------------------------------------------------------------
# It flies: the walk frames are one wing-beat (up, high, low, down, low, high: a cosine), the body
# hovering 9 px over the contact line and lifted a pixel on the downstroke. idle hovers lower on
# half-folded wings; sleep is a wing-wrapped lump on the ground.

def bat_wing(c, shoulder, rear, tip, far=False):
    """One membrane wing: arm from the shoulder bowed out to the wrist, on to the tip; the
    trailing edge scalloped between finger ends back to the body at rear."""
    wrist = _lerp(shoulder, tip, 0.45)
    dx, dy = tip[0] - shoulder[0], tip[1] - shoulder[1]
    ln = max(1.0, (dx * dx + dy * dy) ** 0.5)
    # bow the arm away from the body (perpendicular, toward the side the rear is not on)
    px_, py_ = -dy / ln, dx / ln
    if (rear[0] - shoulder[0]) * px_ + (rear[1] - shoulder[1]) * py_ > 0:
        px_, py_ = -px_, -py_
    wrist = (wrist[0] + px_ * 1.2, wrist[1] + py_ * 1.2)
    fingers = [_lerp(tip, rear, t) for t in (0.4, 0.75)]
    pull = lambda p: _lerp(p, wrist, 0.3)   # noqa: E731
    pts = [shoulder, wrist, tip, pull(_lerp(tip, rear, 0.2)), fingers[0],
           pull(_lerp(tip, rear, 0.58)), fingers[1], pull(_lerp(tip, rear, 0.9)), rear]
    poly(c, pts, 'bat_wing', SHADOW if far else MID)
    if far:
        return
    for f in fingers:
        c.line(_rnd(wrist[0]), _rnd(wrist[1]), _rnd(f[0]), _rnd(f[1]), 'bat_wing', SHADOW)
    c.line(_rnd(shoulder[0]), _rnd(shoulder[1]), _rnd(wrist[0]), _rnd(wrist[1]), 'bat_wing', HIGH)
    c.line(_rnd(wrist[0]), _rnd(wrist[1]), _rnd(tip[0]), _rnd(tip[1]), 'bat_wing', LIGHT)


def bat_pose(frame):
    """(wing lift -1..1, span, body bottom height over the contact line)."""
    k = walk_k(frame)
    if k >= 0:
        a = float(np.cos(2 * np.pi * k / 6))
        return a, 1.0, 9 + (1 if k in (2, 3) else 0)
    return 0.35, 0.65, 5


def bat_side(c, frame):
    feet = c.h - 3
    if frame == 'sleep':
        bat_sleep(c, side=True)
        return
    a, span, hover = bat_pose(frame)
    cx = 15
    by1 = feet - hover
    cy = by1 - 3
    down = a < 0
    shoulder = (cx + 1, cy + 1) if down else (cx + 1, cy - 2)
    rear = (cx - 3, cy + 2) if down else (cx - 4, cy + 1)
    # seen side-on the wing sweeps up behind the head and down under the belly; between, it
    # fans out backward over the body
    th = np.radians(90 * a)
    L = 11 * span
    if down:
        tip = (shoulder[0] - 7 - 2 * np.cos(th), shoulder[1] - L * 0.8 * np.sin(th))
    else:
        tip = (shoulder[0] - 3 - 4 * np.cos(th) * span, shoulder[1] - L * np.sin(th) - 1)
    ftip = (tip[0] + 4, tip[1] - (2 if a > 0 else -2))
    bat_wing(c, (shoulder[0] + 2, shoulder[1]), (rear[0] + 3, rear[1]), ftip, far=True)
    if down:
        # on the downstroke the near wing hangs below the belly: drawn under the body so the
        # body's outline stays whole, its arm traced again on top
        bat_wing(c, shoulder, rear, tip)
    c.disc(cx, cy, 3.6, 2.6, 'bat')                       # body
    c.px(cx - 2, by1 + 1, 'bat', SHADOW); c.px(cx, by1 + 1, 'bat', SHADOW)   # hind claws
    hx, hy = cx + 4, cy - 2
    c.disc(hx, hy, 2, 1.8, 'bat')                         # head
    c.px(hx + 3, hy + 1, 'bat')                           # snout
    c.px(hx - 1, hy - 3, 'bat', LIGHT); c.px(hx - 1, hy - 4, 'bat', HIGH)   # ears, tall and pointed
    c.px(hx + 1, hy - 3, 'bat', MID)
    c.px(hx + 1, hy, 'bat_eye', HIGH)
    if not down:
        bat_wing(c, shoulder, rear, tip)


def bat_front(c, frame, back=False):
    feet = c.h - 3
    if frame == 'sleep':
        bat_sleep(c, side=False, back=back)
        return
    a, span, hover = bat_pose(frame)
    cx = 16
    by1 = feet - hover
    cy = by1 - 3
    L = 12 * span
    ph = np.radians(52 * a)
    for sgn in (-1, 1):
        shoulder = (cx + sgn * 2, cy - 2)
        rear = (cx + sgn * 1, cy + 2)
        tip = (cx + sgn * (2 + L * np.cos(ph)), shoulder[1] - L * np.sin(ph))
        bat_wing(c, shoulder, rear, tip)
    c.disc(cx, cy, 2.5, 3, 'bat')
    c.px(cx - 1, by1 + 1, 'bat', SHADOW); c.px(cx + 1, by1 + 1, 'bat', SHADOW)
    hy = cy - 3
    c.disc(cx, hy, 2.2, 1.8, 'bat')
    for sgn in (-1, 1):
        c.px(cx + sgn * 2, hy - 2, 'bat', LIGHT if sgn < 0 else SHADOW)
        c.px(cx + sgn * 2, hy - 3, 'bat', LIGHT if sgn < 0 else SHADOW)
    if not back:
        c.px(cx - 1, hy, 'bat_eye', HIGH); c.px(cx + 1, hy, 'bat_eye', HIGH)
        c.px(cx, hy + 1, 'bat', SHADOW)
    else:
        c.rect(cx, cy - 1, cx, cy + 2, 'bat', SHADOW)


def bat_sleep(c, side=True, back=False):
    """Folded on the ground: a small dark lump, the wings wrapped round it, ears up."""
    feet = c.h - 3
    cx = 16
    c.disc(cx, feet - 2.5, 4, 2.5, 'bat_wing')
    c.rect(cx - 3, feet - 1, cx + 3, feet - 1, 'bat_wing', SHADOW)
    c.line(cx - 3, feet - 3, cx + 2, feet - 4, 'bat_wing', LIGHT)    # folded arm along the top
    hx = cx + 3 if side else cx
    c.disc(hx, feet - 5, 1.6, 1.4, 'bat')
    c.px(hx - 1, feet - 7, 'bat', LIGHT); c.px(hx + 1, feet - 7, 'bat', SHADOW)     # ears
    if not side and not back:
        c.px(hx - 1, feet - 5, 'bat', SHADOW); c.px(hx + 1, feet - 5, 'bat', SHADOW)   # shut eyes


# --- crawler ----------------------------------------------------------------------------------
# The pale crawler: a blind, many-legged grub of the deep. Seven body segments, each lit on its
# back with a dark seam behind it; a dark pincer jaw in front; glow pits down the
# flank. Walk: the legs ripple in a wave running from head to tail, one frame apart per leg.

CRAWL_SEGS = [  # (x centre, half-length, half-height) tail to head, side view
    (6, 1.8, 1.8), (9, 2, 2.4), (12, 2, 3), (15, 2, 3.4), (18, 2, 3.4), (21, 2, 3.0), (24, 2.4, 2.6)]


def crawler_side(c, frame):
    feet = c.h - 3
    if frame == 'sleep':
        crawler_coil(c)
        return
    k = walk_k(frame)
    base = feet - 3                      # body underside; legs fill the 3 rows below
    cyc = foot_cycle(1, 1)

    def legs(far):
        for i, x in enumerate((8, 11, 14, 17, 20, 23) if not far else (9, 12, 15, 18, 21)):
            ph = (k + i + (3 if far else 0)) % 6 if k >= 0 else None
            dx, lift = cyc[ph] if ph is not None else (0, 0)
            hip = (x, base - 1)
            foot = (x + 1 + dx, feet - lift)
            knee = (x + 1 + (dx + 1) // 2, base + 1 - lift)
            sh = SHADOW if far else MID
            c.line(hip[0], hip[1], knee[0], knee[1], 'crawler', sh)
            c.line(knee[0], knee[1], foot[0], foot[1], 'crawler', sh if far else SHADOW)
    legs(far=True)
    bob = -1 if k in (1, 4) else 0
    tops = []
    for (x, rx, ry) in CRAWL_SEGS:
        cy = base - ry + bob
        c.disc(x, cy, rx, ry, 'crawler')
        tops.append(int(np.floor(cy - ry + 0.5)))
    # segment seams and per-segment back highlight
    for i in range(1, len(CRAWL_SEGS)):
        x = CRAWL_SEGS[i][0] - 2
        top = max(tops[i], tops[i - 1]) + 1
        for y in range(top, base + bob):
            if c.g[y, x] == GID['crawler']:
                c.px(x, y, 'crawler', SHADOW)
        c.px(x + 1, tops[i], 'crawler', HIGH)
        c.px(x + 2, tops[i], 'crawler', HIGH)
    for (x, rx, ry) in CRAWL_SEGS:
        c.px(x, base + bob, 'crawler', SHADOW)
    # glow pits down the flank, set in a darker band so they show
    for (x, rx, ry) in CRAWL_SEGS[1:-1]:
        y = _rnd(base - ry * 0.7 + bob)
        c.px(x - 1, y, 'crawler', SHADOW); c.px(x + 1, y, 'crawler', SHADOW)
        c.px(x, y, 'glow', HIGH)
        c.px(x, y + 1, 'glow', SHADOW)
    legs(far=False)
    # head: no eyes, only the jaw
    hx, hy = 26, base - 3 + bob
    gape = 1 if k in (0, 1, 2) else 0
    c.px(hx, hy, 'crawler', SHADOW)
    c.px(hx, hy + 1, 'crawler', SHADOW)
    # a pincer jaw: two hooks curving in to meet, parted a pixel while it hunts
    stamp(c, hx + 1, hy - 1, ['LLL.', '...M'], 'jaw')
    stamp(c, hx + 1, hy + 1 + gape, ['...S', 'MMS.'], 'jaw')


def crawler_front(c, frame, back=False):
    feet = c.h - 3
    if frame == 'sleep':
        crawler_coil(c, top=True)
        return
    k = walk_k(frame)
    cx = 16
    halfw = [2.5, 3.2, 3.8, 4.2, 4.2, 3.8, 3.4]       # tail to head
    n = len(halfw)
    pitch = 2.6
    near_y = feet - 4 if not back else feet - 2      # head (S) or tail (N) at the near end
    ys = []
    for i in range(n):
        j = (n - 1 - i) if not back else i           # distance from the near end
        ys.append(near_y - j * pitch)
    order = sorted(range(n), key=lambda i: ys[i])    # far to near
    cyc = foot_cycle(1, 1)
    # legs first, out of both sides, rippling along the body
    for i in range(1, n):
        ph = (k + i) % 6 if k >= 0 else None
        for sgn in (-1, 1):
            dy, lift = cyc[(ph + (3 if sgn > 0 else 0)) % 6] if ph is not None else (0, 0)
            y = _rnd(ys[i]) + dy
            x0 = cx + sgn * (_rnd(halfw[i]) + 1)
            x1 = x0 + sgn * (2 - lift)
            c.line(x0, _rnd(ys[i]), x1, y + 1, 'crawler', SHADOW if sgn > 0 else MID)
    for i in order:
        c.disc(cx, ys[i], halfw[i], 1.9, 'crawler')
    for i in order:
        yt = _rnd(ys[i] - 1.9)
        if i != order[0]:
            for x in range(cx - _rnd(halfw[i]) + 1, cx + _rnd(halfw[i])):
                if back:
                    c.px(x, _rnd(ys[i] + 1.9), 'crawler', SHADOW)
                else:
                    c.px(x, yt, 'crawler', SHADOW)
            c.px(cx - 1, yt + 1 if not back else yt, 'crawler', HIGH)
        c.px(cx - _rnd(halfw[i]) + 1, _rnd(ys[i]), 'glow', HIGH)
        c.px(cx + _rnd(halfw[i]) - 1, _rnd(ys[i]), 'glow', LIGHT)
    head = n - 1
    hy = _rnd(ys[head])
    if not back:
        gape = 1 if k in (0, 1, 2) else 0
        stamp(c, cx - 3 - gape, hy + 1, ['L.', 'LM', '.M', '.S'], 'jaw')
        stamp(c, cx + 2 + gape, hy + 1, ['.M', 'MS', 'S.', 'S.'], 'jaw')
        c.line(cx - 3, hy, cx - 6, hy - 2, 'jaw', LIGHT)
        c.line(cx + 3, hy, cx + 6, hy - 2, 'jaw', SHADOW)
    else:
        c.px(cx - 2, hy - 2, 'jaw', LIGHT); c.px(cx + 2, hy - 2, 'jaw', SHADOW)
        c.line(cx - 3, hy - 1, cx - 6, hy - 4, 'jaw', LIGHT)
        c.line(cx + 3, hy - 1, cx + 6, hy - 4, 'jaw', SHADOW)
        c.px(cx, _rnd(ys[0]) + 2, 'crawler', SHADOW)          # tail tip


def crawler_coil(c, top=False):
    """Asleep: curled into a flat coil, head tucked in against the tail."""
    feet = c.h - 3
    cx, cy = 16, feet - 6.5
    rx, ry = 7.0, 4.0
    n = 8
    for i in range(n):
        t = i / (n - 1)
        ang = np.radians(200 - 300 * t)              # tail at the back-left, round to the head
        r = 1.6 + 1.0 * t
        x = cx + rx * np.cos(ang) * (1 - 0.25 * t)
        y = cy - ry * np.sin(ang) * (1 - 0.25 * t)
        c.disc(x, y, r, r * 0.9, 'crawler')
        if 0 < i < n - 1 and i % 2 == 0:
            c.px(_rnd(x), _rnd(y), 'glow', HIGH)
    c.px(_rnd(cx + 2), _rnd(cy + 1), 'jaw', MID)
    c.px(_rnd(cx + 3), _rnd(cy + 1), 'jaw', SHADOW)


# --- burrower ---------------------------------------------------------------------------------
# A heavy mole-like digger the size of a big badger: humped dark body, a pointed snout with a
# pink nose, pin-head eyes and two huge pale spade claws. It shuffles low, the claws reaching
# forward one after the other; asleep it curls into a ball round its claws.

CLAW_SIDE = ['HL....',
             'LLLH..',
             'MMLLHH',
             'SMSMSM',
             '.S.S.S']
CLAW_FRONT = ['LHL.',
              'LLLH',
              'MMML',
              'SMSM',
              'S.S.']


def burrower_side(c, frame):
    feet = c.h - 3
    if frame == 'sleep':
        burrower_ball(c, side=True)
        return
    k = walk_k(frame)
    reach = [2, 1, 0, -1, 0, 1]
    near = reach[k] if k >= 0 else 0
    far = reach[(k + 3) % 6] if k >= 0 else 0
    lift_n = 1 if k in (4, 5) else 0
    lift_f = 1 if k in (1, 2) else 0
    bob = -1 if k in (1, 4) else 0
    # far fore-arm and claw, and the far hind foot, behind the body
    c.rect(19 + far, feet - 5, 20 + far, feet - 3 - lift_f, 'mole', SHADOW)
    stamp(c, 20 + far, feet - 4 - lift_f, ['MMMS.', 'SSSSS', 'S.S.S'], 'claw')
    c.rect(7 - far // 2, feet - 2, 8 - far // 2, feet, 'mole', SHADOW)
    # body: a humped oval, heavier at the shoulders
    c.disc(13, feet - 8 + bob, 9, 6, 'mole')
    c.disc(19, feet - 6 + bob, 4.5, 4.5, 'mole')
    c.rect(3, feet - 7 + bob, 4, feet - 6 + bob, 'mole')          # stub tail
    # head run out into a long pointed snout, pink nose at the tip
    sy = feet - 6 + bob
    c.rect(22, sy - 2, 24, sy + 2, 'mole')
    c.rect(25, sy - 1, 26, sy + 1, 'mole')
    c.rect(27, sy, 27, sy + 1, 'mole')
    c.rect(23, sy - 2, 26, sy - 2, 'mole', LIGHT)
    c.px(25, sy - 1, 'mole', LIGHT); c.px(26, sy - 1, 'mole', LIGHT)
    c.px(28, sy, 'mole_nose', HIGH); c.px(28, sy + 1, 'mole_nose', MID)
    c.px(29, sy, 'mole_nose', LIGHT)
    c.px(23, sy - 1, 'claw', MID)                                  # pin-head eye
    c.line(21, sy - 4, 21, sy + 1, 'mole', SHADOW)                 # crease behind the head
    # fur: a lighter streak along the back
    for x in (7, 10, 13, 16):
        c.px(x, feet - 13 + bob, 'mole', HIGH)
        c.px(x + 1, feet - 12 + bob, 'mole', LIGHT)
    # near hind foot
    c.rect(10 + near // 2, feet - 2, 11 + near // 2, feet, 'mole', MID)
    c.px(12 + near // 2, feet, 'claw', SHADOW)
    # near fore-arm and the big spade claw, the accent
    c.rect(18 + near, feet - 5 + bob, 20 + near, feet - 4 - lift_n, 'mole', MID)
    stamp(c, 20 + near, feet - 4 - lift_n, CLAW_SIDE, 'claw')


def burrower_front(c, frame, back=False):
    feet = c.h - 3
    if frame == 'sleep':
        burrower_ball(c, side=False, back=back)
        return
    k = walk_k(frame)
    reach = [2, 1, 0, -1, 0, 1]
    left = reach[k] if k >= 0 else 0
    right = reach[(k + 3) % 6] if k >= 0 else 0
    bob = -1 if k in (1, 4) else 0
    cx = 16
    if back:
        # from behind: the round back, a stub tail, hind feet; the fore-claws stick out sideways
        # at the shoulders, as a mole's do
        for sgn, r in ((-1, left), (1, right)):
            stamp(c, cx + sgn * 9 - 1, feet - 8 - max(0, r) // 2, ['LH.', 'MMM', 'S.S'], 'claw', mirror=sgn > 0)
        c.disc(cx, feet - 7 + bob, 8.5, 6.5, 'mole')
        for sgn, r in ((-1, right), (1, left)):
            c.rect(cx + sgn * 5 - 1, feet - 1 - max(0, r) // 2, cx + sgn * 5, feet, 'mole', SHADOW if sgn > 0 else MID)
        c.blob(cx - 1, feet - 3 + bob, cx + 1, feet - 1 + bob, 'mole', MID)
        c.px(cx, feet - 1 + bob, 'mole', SHADOW)
        for x in (cx - 4, cx - 1, cx + 2):
            c.px(x, feet - 12 + bob, 'mole', HIGH)
        return
    # two spade claws out to the sides, reaching forward in turn (a reaching claw sits lower)
    for sgn, r in ((-1, left), (1, right)):
        ax = cx + sgn * 7
        c.rect(min(ax, ax - sgn * 2), feet - 6 + bob, max(ax, ax - sgn * 2), feet - 5 + max(0, r),
               'mole', MID if sgn < 0 else SHADOW)
        stamp(c, ax - 3 if sgn < 0 else ax, feet - 4 + max(0, r) - (1 if r < 0 else 0),
              CLAW_FRONT, 'claw', mirror=sgn > 0)
    c.disc(cx, feet - 8 + bob, 8.5, 6, 'mole')
    for x in (cx - 5, cx - 2, cx + 1):
        c.px(x, feet - 13 + bob, 'mole', HIGH)
    # head, snout toward the viewer, pink nose at the bottom; a dark crease 
    c.disc(cx, feet - 4 + bob, 3.4, 2.6, 'mole')
    c.rect(cx - 2, feet - 6 + bob, cx + 1, feet - 6 + bob, 'mole', LIGHT)
    c.rect(cx - 1, feet - 2 + bob, cx + 1, feet - 1 + bob, 'mole', MID)
    c.px(cx - 1, feet - 2 + bob, 'mole', LIGHT)
    c.px(cx - 1, feet + bob, 'mole_nose', HIGH); c.px(cx, feet + bob, 'mole_nose', LIGHT)
    c.px(cx + 1, feet + bob, 'mole_nose', MID)
    c.px(cx - 2, feet - 4 + bob, 'claw', MID); c.px(cx + 2, feet - 4 + bob, 'claw', SHADOW)


def burrower_ball(c, side=True, back=False):
    """Asleep: curled into a ball, the claws folded over the snout."""
    feet = c.h - 3
    cx = 16
    c.disc(cx, feet - 5.5, 7.5, 5.5, 'mole')
    for x in (cx - 4, cx - 1, cx + 2):
        c.px(x, feet - 10, 'mole', HIGH)
    if side:
        c.line(cx + 1, feet - 7, cx + 3, feet - 3, 'mole', SHADOW)    # the curl of the back
        stamp(c, cx + 3, feet - 3, ['LLH.', 'MMMM', 'S.S.'], 'claw')
        c.px(cx + 7, feet - 4, 'mole_nose', LIGHT)
    elif not back:
        stamp(c, cx - 5, feet - 3, ['LHL', 'MMM', 'S.S'], 'claw')
        stamp(c, cx + 2, feet - 3, ['LHL', 'MMM', 'S.S'], 'claw', mirror=True)
        c.px(cx - 1, feet - 2, 'mole_nose', LIGHT); c.px(cx, feet - 2, 'mole_nose', SHADOW)
    else:
        c.blob(cx - 1, feet - 3, cx + 1, feet - 1, 'mole', MID)        # tail stub
        c.px(cx - 7, feet - 2, 'claw', SHADOW); c.px(cx + 7, feet - 2, 'claw', SHADOW)


# --- slime (#423) -----------------------------------------------------------------------------
# A cave slime: one heaped mass of pale olive, glossy on top, with darker lumps held inside. It
# creeps: the heap draws forward and gathers itself, long and low then short and tall. Asleep it
# settles into a flat puddle.

SLIME_STRETCH = [0, 1, 2, 1, 0, -1]


def slime_heap(c, cx, feet, rx, ry, front=False):
    c.disc(cx, feet - ry + 1, rx, ry, 'slime')
    c.rect(_rnd(cx - rx + 1), feet, _rnd(cx + rx - 1), feet, 'slime', SHADOW)
    top = _rnd(feet - 2 * ry + 2)
    c.px(_rnd(cx - rx * 0.4), top + 1, 'slime', HIGH)
    c.px(_rnd(cx - rx * 0.4) + 1, top + 1, 'slime', HIGH)
    c.px(_rnd(cx - rx * 0.5), top + 2, 'slime', HIGH)
    # what it holds inside: two dark lumps, low in the mass
    c.disc(cx + (0 if front else 2), feet - ry * 0.7, 1.4, 1.1, 'slime_core')
    c.px(_rnd(cx - 3), _rnd(feet - ry * 0.5), 'slime_core', MID)


def slime_side(c, frame):
    feet = c.h - 3
    if frame == 'sleep':
        slime_heap(c, 16, feet, 10.5, 2.6)
        return
    k = walk_k(frame)
    st = SLIME_STRETCH[k] if k >= 0 else 0
    slime_heap(c, 16 + st * 0.5, feet, 8.0 + st, 6.0 - st * 0.6)


def slime_front(c, frame, back=False):
    feet = c.h - 3
    if frame == 'sleep':
        slime_heap(c, 16, feet, 10.5, 2.6, front=True)
        return
    k = walk_k(frame)
    st = SLIME_STRETCH[k] if k >= 0 else 0
    slime_heap(c, 16, feet, 8.0 + st * 0.4, 6.0 - st * 0.6, front=True)


# --- rats (#423) ------------------------------------------------------------------------------
# A swarm of rats: six grey bodies in a loose pack, pink tails out behind, each scurrying a pixel
# forward and back out of step with the rest. Asleep they lie heaped together, tails out.

RAT_PACK = [(9, 21), (17, 23), (24, 20), (12, 27), (21, 28), (5, 26)]


def rat_side(c, x, y):
    c.line(x - 3, y + 1, x - 6, y + 2, 'rat_tail', MID)
    c.disc(x, y, 2.6, 1.6, 'rat')
    c.px(x + 3, y, 'rat', LIGHT)
    c.px(x + 3, y + 1, 'rat', MID)
    c.px(x + 4, y + 1, 'rat_tail', HIGH)          # nose
    c.px(x + 1, y - 2, 'rat', HIGH)               # ear
    c.px(x + 2, y, 'rat_tail', SHADOW)            # eye


def rat_front(c, x, y, back=False):
    if back:
        c.line(x, y + 2, x + 1, y + 5, 'rat_tail', MID)
    c.disc(x, y, 1.6, 2.4, 'rat')
    c.px(x - 1, y - 3, 'rat', HIGH); c.px(x + 1, y - 3, 'rat', LIGHT)   # ears
    if not back:
        c.px(x, y + 2, 'rat', LIGHT)
        c.px(x, y + 3, 'rat_tail', HIGH)          # nose


def rats_side(c, frame):
    k = walk_k(frame)
    if frame == 'sleep':
        for j, (x, y) in enumerate(((10, 25), (15, 26), (20, 25), (13, 23), (18, 23))):
            rat_side(c, x, y + 1)
        return
    for j, (x, y) in enumerate(RAT_PACK):
        dx = ((k + j) % 3 - 1) if k >= 0 else 0
        rat_side(c, x + dx, y)


def rats_front(c, frame, back=False):
    k = walk_k(frame)
    if frame == 'sleep':
        for (x, y) in ((11, 25), (16, 26), (21, 25), (14, 23), (19, 23)):
            rat_front(c, x, y, back)
        return
    for j, (x, y) in enumerate(RAT_PACK):
        dy = ((k + j) % 3 - 1) if k >= 0 else 0
        rat_front(c, 4 + (x * 24) // 28, y - 1 + dy, back)


# --- centipede (#423) -------------------------------------------------------------------------
# A cave centipede as long as an arm: a chain of copper segments, each on its own pair of legs,
# the chain rippling as the legs go in waves from head to tail; long feelers and hooked jaws at
# the head. Asleep it lies coiled.

def centipede_side(c, frame):
    feet = c.h - 3
    if frame == 'sleep':
        centipede_coil(c)
        return
    k = walk_k(frame)
    xs = [4, 7, 10, 13, 16, 19, 22]
    for i, x in enumerate(xs):
        ph = (k + i) % 6 if k >= 0 else 0
        wave = [0, -1, -1, 0, 0, 0][ph]
        y = feet - 3 + wave
        lift = 1 if ph in (1, 2) else 0
        c.line(x, y + 1, x - 1 + (1 if ph < 3 else -1), feet - lift, 'centi_leg', SHADOW)
        c.line(x + 1, y + 1, x + 2 - (1 if ph < 3 else -1), feet - (1 - lift), 'centi_leg', MID)
        c.disc(x + 1, y - 1, 1.8, 1.6, 'centi')
        c.px(x + 1, y - 3, 'centi', HIGH)
    hy = feet - 5
    c.disc(25.5, hy, 2.2, 1.9, 'centi')
    c.px(26, hy - 2, 'centi', HIGH)
    c.line(27, hy - 1, 30, hy - 5, 'centi_leg', LIGHT)     # feelers
    c.line(26, hy - 2, 28, hy - 6, 'centi_leg', MID)
    c.px(28, hy + 1, 'centi_leg', SHADOW); c.px(28, hy + 2, 'centi_leg', MID)   # jaws
    c.line(4, feet - 3, 1, feet - 1, 'centi_leg', MID)     # tail feelers


def centipede_front(c, frame, back=False):
    feet = c.h - 3
    if frame == 'sleep':
        centipede_coil(c)
        return
    k = walk_k(frame)
    cx = 16
    # the chain runs away from the viewer up the cell; the head is nearest (S) or furthest (N)
    ys = [feet - 2 - 3 * i for i in range(6)]
    if back:
        ys = ys[::-1]
    for i, y in enumerate(ys):
        ph = (k + i) % 6 if k >= 0 else 0
        sway = [0, 1, 1, 0, -1, -1][ph] if k >= 0 else 0
        c.line(cx - 2 + sway, y, cx - 5 + sway, y + 1, 'centi_leg', MID)
        c.line(cx + 2 + sway, y, cx + 5 + sway, y + 1, 'centi_leg', SHADOW)
        c.disc(cx + sway, y, 2.2, 1.5, 'centi')
        c.px(cx - 1 + sway, y - 1, 'centi', HIGH)
    hy = ys[0] if not back else ys[-1]
    if not back:
        c.disc(cx, hy + 1, 2.4, 1.8, 'centi')
        c.line(cx - 2, hy + 2, cx - 5, hy + 4, 'centi_leg', LIGHT)
        c.line(cx + 2, hy + 2, cx + 5, hy + 4, 'centi_leg', MID)
        c.px(cx - 1, hy + 3, 'centi_leg', SHADOW); c.px(cx + 1, hy + 3, 'centi_leg', SHADOW)
    else:
        c.line(cx - 1, hy - 2, cx - 4, hy - 6, 'centi_leg', LIGHT)
        c.line(cx + 1, hy - 2, cx + 4, hy - 6, 'centi_leg', MID)


def centipede_coil(c):
    feet = c.h - 3
    cx, cy = 16, feet - 5.5
    n = 9
    for i in range(n):
        t = i / (n - 1)
        ang = np.radians(210 - 330 * t)
        r = 6.5 * (1 - 0.45 * t)
        x = cx + r * np.cos(ang)
        y = cy - r * 0.6 * np.sin(ang)
        c.disc(x, y, 1.7, 1.4, 'centi')
        c.px(_rnd(x), _rnd(y - 1), 'centi', HIGH)
    c.px(cx + 1, _rnd(cy), 'centi_leg', SHADOW)


# --- cave crocodile (#336) ----------------------------------------------------------------------
# The cave crocodile: a low, long ambusher of the pool caverns. Side on it is a heavy tail, a
# flat body on short legs and a snout that rides low along the ground; the eyes shine out of
# the dark, as a hunter of black water's would. Walk is a slow belly crawl: the tail sways a
# frame behind the shuffling legs, the body barely lifts.

def croc_side(c, frame):
    feet = c.h - 3
    if frame == 'sleep':
        # Lying flat in the shallows: a low dark line of a body, the snout on the ground.
        c.disc(16, feet - 2, 11, 2, 'croc')
        c.disc(25, feet - 2, 4, 1.6, 'croc')
        c.rect(6, feet - 1, 28, feet - 1, 'croc', SHADOW)
        c.px(26, feet - 3, 'croc', SHADOW)   # the shut eye ridge
        return
    sway = {'walk1': -1, 'walk2': -1, 'walk3': 0, 'walk4': 1, 'walk5': 1, 'walk6': 0}.get(frame, 0)
    step = {'walk1': 0, 'walk2': 1, 'walk3': 1, 'walk4': 0, 'walk5': -1, 'walk6': -1}.get(frame, 0)
    bob = -1 if frame in ('walk2', 'walk5') else 0
    # tail: a tapering sweep behind, swinging low
    c.line(4, feet - 3 + sway, 8, feet - 3 + sway, 'croc', SHADOW)
    c.line(8, feet - 3 + sway, 12, feet - 5 + bob, 'croc')
    # body: flat and heavy, a pale line of back above it
    c.disc(17, feet - 5 + bob, 9, 3.2, 'croc')
    c.rect(9, feet - 3 + bob, 24, feet - 3 + bob, 'croc', SHADOW)      # the water-line belly
    c.line(10, feet - 8 + bob, 22, feet - 8 + bob, 'croc', LIGHT)      # the back ridge
    # short legs, shuffling one frame at a time
    for lx in (13, 20):
        c.px(lx, feet - 2, 'croc', SHADOW)
        c.px(lx + step, feet - 1, 'croc', SHADOW)
    # head and snout, low along the ground
    c.disc(25, feet - 5 + bob, 3.2, 2, 'croc')
    c.rect(26, feet - 5 + bob, 30, feet - 4 + bob, 'croc')
    c.rect(26, feet - 3 + bob, 30, feet - 3 + bob, 'croc', SHADOW)     # the closed jaw
    c.px(30, feet - 4 + bob, 'croc', SHADOW)                            # the nose tip
    # the eyes shine out of the dark
    c.px(25, feet - 7 + bob, 'bat_eye', HIGH)
    c.px(24, feet - 7 + bob, 'croc', SHADOW)


def croc_front(c, frame, back=False):
    feet = c.h - 3
    if frame == 'sleep':
        c.disc(16, feet - 2, 9, 2, 'croc')
        c.rect(8, feet - 1, 24, feet - 1, 'croc', SHADOW)
        return
    bob = -1 if frame in ('walk2', 'walk5') else 0
    step = 1 if frame in ('walk2', 'walk3') else (-1 if frame in ('walk5', 'walk6') else 0)
    # head on: a narrow column of snout and eyes; the body is nearly all behind it
    c.disc(16, feet - 3, 5, 2.2, 'croc')
    c.disc(16, feet - 6 + bob, 3.4, 2.6, 'croc')
    c.rect(14, feet - 1, 18, feet - 1, 'croc', SHADOW)
    for sgn in (-1, 1):
        c.px(16 + sgn * 3 + step, feet - 1, 'croc', SHADOW)            # the forefeet
    c.rect(15, feet - 3 + bob, 17, feet - 3 + bob, 'croc', SHADOW)     # the shut jaw
    if back:
        c.rect(16, feet - 8 + bob, 16, feet - 5 + bob, 'croc', LIGHT)  # the back ridge from above
    else:
        c.px(15, feet - 8 + bob, 'bat_eye', HIGH)                      # the eyes, up and lit
        c.px(17, feet - 8 + bob, 'bat_eye', HIGH)


CAVE_DRAW = {
    'bat': (bat_side, bat_front),
    'crawler': (crawler_side, crawler_front),
    'burrower': (burrower_side, burrower_front),
    'slime': (slime_side, slime_front),
    'rats': (rats_side, rats_front),
    'centipede': (centipede_side, centipede_front),
    'cave_croc': (croc_side, croc_front),
}


def make_frame(name, d, frame):
    sp = SPEC[name]
    c = Canvas(sp.get('cell_w', 32), sp['cell_h'])
    if name in CAVE:
        side, front = CAVE_DRAW[name]
        if d in ('E', 'W'):
            side(c, frame)
            if d == 'W':
                c = c.flipped()
        else:
            front(c, frame, back=(d == 'N'))
        return c.render()
    if d in ('E', 'W'):
        draw_side(c, name, sp, frame)
        if d == 'W':
            c = c.flipped()
    else:
        draw_front(c, name, sp, frame, back=(d == 'N'))
    out = c.render()
    if d in ('S', 'N'):
        # seen from above the rise of the body shows only as the whole figure lifting
        _, bob, _ = gait(name, sp, frame)
        if bob < 0:
            out[:bob] = out[-bob:].copy()
            out[bob:] = 0
    return out


def make_sheet(name):
    ch = SPEC[name]['cell_h']
    cw = SPEC[name].get('cell_w', 32)
    img = np.zeros((ch * 4, cw * len(FRAMES), 4), dtype=np.uint8)
    for r, d in enumerate(DIRS):
        for col, f in enumerate(FRAMES):
            tile = make_frame(name, d, f)
            if f == 'idle2':
                # Breathing: the animal settles 1 px. Applied at sheet assembly rather than inside
                # draw_side/draw_front, so no per-species coordinate maths has to change and every
                # species gets the same behaviour. Feet stop 2-3 px above the cell bottom, so the
                # row pushed off the bottom edge carries nothing.
                tile[1:] = tile[:-1]
                tile[0] = 0
            img[r * ch:(r + 1) * ch, col * cw:(col + 1) * cw] = tile
    return Image.fromarray(img, 'RGBA')


def hart_sheet():
    """The White Hart (creatures.rs): the deer's frames, its coat recoloured to pale greys and
    white, its antlers to bone, so it moves exactly as a deer and reads apart at a glance."""
    img = make_sheet('deer')
    img = img.convert('RGBA')
    swap = {}
    deer = RAMPS['deer']
    pale = RAMPS['deer_pale']
    white = [pal.ramp('stone', 5), pal.ramp('stone', 6), pal.ramp('stone', 7), pal.neutral('white')]
    for k in range(4):
        swap[tuple(deer[k])] = white[k]
        swap[tuple(pale[k])] = white[min(3, k + 1)]
    px = img.load()
    for y in range(img.height):
        for x in range(img.width):
            r, g, b, a = px[x, y]
            if a and (r, g, b) in swap:
                nr, ng, nb = swap[(r, g, b)]
                px[x, y] = (nr, ng, nb, a)
    return img


def blind_bear_sheet():
    """The blind bear of the deep (#336): the bear's frames, its coat recoloured to the pale
    stone of things that live without light, as the White Hart is the deer, pale. It moves
    exactly as a bear and reads apart at a glance down a dark cavern."""
    img = make_sheet('bear').convert('RGBA')
    swap = {}
    cave = [pal.ramp('stone', 4), pal.ramp('stone', 5), pal.ramp('stone', 6), pal.ramp('stone', 7)]
    paler = [pal.ramp('stone', 6), pal.ramp('stone', 7), pal.neutral('white'), pal.neutral('white')]
    for k in range(4):
        swap[tuple(RAMPS['bear'][k])] = cave[k]
        swap[tuple(RAMPS['bear_pale'][k])] = paler[k]
    px = img.load()
    for y in range(img.height):
        for x in range(img.width):
            r, g, b, a = px[x, y]
            if a and (r, g, b) in swap:
                nr, ng, nb = swap[(r, g, b)]
                px[x, y] = (nr, ng, nb, a)
    return img


def main():
    os.makedirs(OUT, exist_ok=True)
    for n in ANIMALS:
        make_sheet(n).save(os.path.join(OUT, n + '.png'))
        print('wrote', n + '.png', '(cell %dx%d)' % (SPEC[n].get('cell_w', 32), SPEC[n]['cell_h']))
    hart_sheet().save(os.path.join(OUT, 'hart.png'))
    print('wrote hart.png (the deer, pale)')
    blind_bear_sheet().save(os.path.join(OUT, 'blind_cave_bear.png'))
    print('wrote blind_cave_bear.png (the bear, pale)')
    import provenance
    provenance.refresh_folder('animals', describe)


def describe(fn):
    base = {'hart.png': 'deer', 'blind_cave_bear.png': 'bear'}.get(fn, fn[:-4])
    sp = SPEC[base]
    return 'animal sheet, %d frames x 4 dirs, cell %dx%d' % (len(FRAMES), sp.get('cell_w', 32), sp['cell_h'])


if __name__ == '__main__':
    main()
