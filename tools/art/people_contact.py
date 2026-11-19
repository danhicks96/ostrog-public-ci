#!/usr/bin/env python3
"""Contact sheets and animation strips for inhabitants and animals, for human review.

shots/out/contact_people.png (+ @2x, @4x): composed inhabitants (body + clothing + hair +
bandage + equipment, each with its own skin / hair / cloth swap, every layer from the sheet for
that build and age) in every frame of every facing, labelled.
shots/out/strip_people_<cycle>.png (@4x): one cycle per row (walk, carry, swing, stoop, tap) in
each facing, frame by frame, on a grid with guide lines at the feet row and the idle head top,
so a frame that jumps or a leg that does not alternate shows.
shots/out/contact_animals.png (+ @2x, @4x): every animal sheet, and strip_animals.png (@4x):
each species' walk in E and S, frame by frame, on the same kind of guide grid.

Swaps are applied here exactly as the runtime must: source[i] -> target[i] per layer, using
godot/assets/people/swaps.json.

Run: python3 tools/art/people_contact.py
"""
import json
import os
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pal import ROOT, hex_to_rgb  # noqa: E402

PEOPLE = os.path.join(ROOT, 'godot', 'assets', 'people')
ANIMALS = os.path.join(ROOT, 'godot', 'assets', 'animals')
OUT = os.path.join(ROOT, 'shots', 'out')
BG = (46, 42, 52)
BG2 = (70, 82, 64)     # grass-ish ground behind composed figures
GUIDE = (96, 110, 88)
FONT = ImageFont.load_default()
DIRS = ['S', 'W', 'E', 'N']


def layout():
    with open(os.path.join(PEOPLE, 'swaps.json')) as f:
        swaps = json.load(f)
    return swaps, swaps['layout']['columns']


# The people cell size, as the generator wrote it (48 since style bible v2). Animal sheets keep
# their own 32 px cells.
with open(os.path.join(PEOPLE, 'swaps.json')) as _f:
    CELL = json.load(_f)['layout'].get('cell', 32)


# (label, build, age, hair no., outfit, equipment, skin idx, hair colour idx, cloth idx, bandage)
CAST = [
    ('miner avg/tunic',     'average', 'adult', 1, 'tunic',     'pickaxe', 2, 1, 2, False),
    ('gatherer slim/dress', 'slim',    'adult', 3, 'dress',     'basket',  4, 4, 8, False),
    ('smith broad/apron',   'broad',   'adult', 6, 'apron',     'axe',     1, 0, 1, True),
    ('child avg/vest',      'average', 'child', 2, 'vest',      'bucket',  3, 3, 6, False),
    ('wanderer slim/cloak', 'slim',    'adult', 4, 'cloak',     'hoe',     5, 5, 9, False),
    ('elder broad/coat',    'broad',   'adult', 9, 'coat',      'sack',    0, 6, 11, False),
    ('scribe avg/robe',     'average', 'adult', 7, 'robe',      'bundle',  3, 2, 10, False),
    ('child broad/shirt',   'broad',   'child', 10, 'workshirt', 'axe',    2, 7, 4, True),
]

CYCLES = [('walk', ['walk%d' % i for i in range(1, 7)]),
          ('carry', ['carry%d' % i for i in range(1, 7)]),
          ('swing', ['swing1', 'swing2', 'swing3']),
          ('stoop', ['stoop1', 'stoop2', 'stoop3']),
          ('tap', ['tap1', 'tap2', 'tap3']),
          ('talk', ['talk1', 'talk2', 'talk3']),
          ('eat', ['eat1', 'eat2', 'eat3']),
          ('sleep', ['sleep', 'sleep', 'sleep2', 'sleep', 'sleep3']),
          ('tend', ['tend1', 'tend2', 'tend3']),
          ('drink', ['drink1', 'drink2', 'drink3'])]


def load(folder, name):
    return np.array(Image.open(os.path.join(folder, name + '.png')).convert('RGBA'))


def swap(arr, source, target):
    out = arr.copy()
    for s, t in zip(source, target):
        s, t = hex_to_rgb(s), hex_to_rgb(t)
        m = (arr[:, :, 0] == s[0]) & (arr[:, :, 1] == s[1]) & (arr[:, :, 2] == s[2]) & (arr[:, :, 3] > 0)
        out[m, :3] = t
    return out


def over(base, layer):
    a = layer[:, :, 3:4] > 0
    return np.where(a, layer, base)


def compose(swaps, member, include_equipment=True):
    label, build, age, hair, outfit, equip, si, hi, ci, band = member
    ba = '%s_%s' % (build, age)
    sfx = '' if ba == 'average_adult' else '_' + ba   # same naming rule as people.sheet_list
    b = swap(load(PEOPLE, 'body_' + ba), swaps['skin']['source'], swaps['skin']['targets'][si])
    c = swap(load(PEOPLE, 'clothing_%s%s' % (outfit, sfx)), swaps['cloth']['source'], swaps['cloth']['targets'][ci])
    h = swap(load(PEOPLE, 'hair_%02d%s' % (hair, sfx)), swaps['hair']['source'], swaps['hair']['targets'][hi])
    layers = [b, c, h]
    if band:
        layers.append(load(PEOPLE, 'overlay_bandage_' + ba))
    if include_equipment:
        layers.append(load(PEOPLE, 'equipment_%s%s' % (equip, sfx)))
    img = np.zeros_like(b)
    for layer in layers:
        img = over(img, layer)
    return img


def cell(sheet, frames, d, frame, ch=CELL, cw=CELL):
    r, col = DIRS.index(d), frames.index(frame)
    return Image.fromarray(sheet[r * ch:(r + 1) * ch, col * cw:(col + 1) * cw])


def label(d, x, y, text):
    d.text((x, y), text, fill=(235, 230, 220), font=FONT)


def save(img, name, scales=(1, 2, 4)):
    os.makedirs(OUT, exist_ok=True)
    for k in scales:
        im = img if k == 1 else img.resize((img.width * k, img.height * k), Image.NEAREST)
        im.convert('RGB').save(os.path.join(OUT, name + ('' if k == 1 else '@%dx' % k) + '.png'))
    print('wrote', os.path.join(OUT, name + '.png'), 'at', ', '.join('%dx' % k for k in scales))


def people_sheet(swaps, frames):
    """Every composed cast member, every frame, every facing."""
    pad, left, head = 4, 120, 14
    gap = 2
    W = left + len(frames) * (CELL + gap) + pad
    block = 4 * (CELL + gap) + 6
    H = head + len(CAST) * block + pad
    img = Image.new('RGBA', (W, H), BG)
    d = ImageDraw.Draw(img)
    for j, f in enumerate(frames):
        label(d, left + j * (CELL + gap), 2, f[:6])
    y = head
    for m in CAST:
        comp = compose(swaps, m)
        tone = '%s/%s/%s' % (swaps['skin']['names'][m[6]], swaps['hair']['names'][m[7]], swaps['cloth']['names'][m[8]])
        label(d, pad, y + 4, m[0])
        label(d, pad, y + 16, tone)
        for r, dd in enumerate(DIRS):
            yy = y + r * (CELL + gap)
            label(d, left - 10, yy + 10, dd)
            d.rectangle([left - 1, yy - 1, left + len(frames) * (CELL + gap) - gap, yy + CELL], fill=BG2)
            for j, f in enumerate(frames):
                img.alpha_composite(cell(comp, frames, dd, f), (left + j * (CELL + gap), yy))
        y += block
    save(img, 'contact_people')


def guide_cell(d, x, y, cw, ch, rows):
    """Ground-coloured cell with horizontal guide lines (feet row, head top) and a centre line."""
    d.rectangle([x, y, x + cw - 1, y + ch - 1], fill=BG2)
    for gy in rows:
        d.line([x, y + gy, x + cw - 1, y + gy], fill=GUIDE)
    d.line([x + cw // 2, y, x + cw // 2, y + ch - 1], fill=GUIDE)


def people_strips(swaps, frames):
    """One strip per cycle: rows = cast member x facing, columns = the cycle twice over."""
    for name, cyc in CYCLES:
        seq = cyc * 2 if len(cyc) == 6 else cyc * 3
        pad, left, gap = 4, 120, 1
        W = left + len(seq) * (CELL + gap) + pad
        rows = [(m, dd) for m in CAST[:4] for dd in ('S', 'E', 'N')]
        H = 14 + len(rows) * (CELL + gap) + pad
        img = Image.new('RGBA', (W, H), BG)
        d = ImageDraw.Draw(img)
        for j, f in enumerate(seq):
            label(d, left + j * (CELL + gap), 2, f[-1])
        y = 14
        for m, dd in rows:
            comp = compose(swaps, m, include_equipment=name not in ('eat', 'sleep', 'tend', 'drink'))
            label(d, pad, y + 10, '%s %s' % (m[0][:14], dd))
            head_top = 6 if m[2] == 'adult' else 12
            for j, f in enumerate(seq):
                x = left + j * (CELL + gap)
                guide_cell(d, x, y, CELL, CELL, (29, head_top))
                img.alpha_composite(cell(comp, frames, dd, f), (x, y))
            y += CELL + gap
        save(img, 'strip_people_' + name, scales=(4,))


ANIMAL_FRAMES = ['idle', 'idle2', 'walk1', 'walk2', 'walk3', 'walk4', 'walk5', 'walk6', 'sleep']
ANIMAL_NAMES = ['deer', 'boar', 'wolf', 'hare', 'goat', 'chicken', 'fox', 'crow', 'bear', 'duck']


def animal_sheets():
    ims = [(n, Image.open(os.path.join(ANIMALS, n + '.png')).convert('RGBA')) for n in ANIMAL_NAMES]
    pad = 8
    W = pad * 2 + max(im.width for _, im in ims)
    H = pad + sum(im.height + 14 + pad for _, im in ims)
    img = Image.new('RGBA', (W, H), BG)
    d = ImageDraw.Draw(img)
    y = pad
    for name, im in ims:
        ch = im.height // 4
        cw = im.width // len(ANIMAL_FRAMES)
        label(d, pad, y, '%s  cell %dx%d  rows S W E N  cols %s' % (name, cw, ch, ' '.join(ANIMAL_FRAMES)))
        d.rectangle([pad - 1, y + 13, pad + im.width, y + 14 + im.height], fill=BG2)
        img.alpha_composite(im, (pad, y + 14))
        y += im.height + 14 + pad
    save(img, 'contact_animals')
    # walk strips: E and S, the cycle twice, on a guide grid at each species' cell size
    seq = ['walk%d' % i for i in range(1, 7)] * 2
    gap, left = 1, 70
    rows = [(n, im, dd) for n, im in ims for dd in ('E', 'S')]
    W = left + len(seq) * (48 + gap) + pad
    H = 14 + sum(im.height // 4 + gap for _, im, _ in rows) + pad
    img = Image.new('RGBA', (W, H), BG)
    d = ImageDraw.Draw(img)
    y = 14
    for n, im, dd in rows:
        ch = im.height // 4
        cw = im.width // len(ANIMAL_FRAMES)
        arr = np.array(im)
        label(d, 4, y + ch // 2 - 5, '%s %s' % (n, dd))
        for j, f in enumerate(seq):
            x = left + j * (48 + gap) + (48 - cw) // 2
            guide_cell(d, x, y, cw, ch, (ch - 3,))
            img.alpha_composite(cell(arr, ANIMAL_FRAMES, dd, f, ch=ch, cw=cw), (x, y))
        y += ch + gap
    save(img, 'strip_animals', scales=(4,))


def main():
    swaps, frames = layout()
    people_sheet(swaps, frames)
    people_strips(swaps, frames)
    animal_sheets()


if __name__ == '__main__':
    main()
