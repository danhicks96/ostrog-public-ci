#!/usr/bin/env python3
"""Contact sheets for human review: shots/out/contact_<group>.png at 1x and 2x, labelled.

terrain: each family's sheet plus a 4x3 mixed-variant tiling so seams can be judged.
overlays / structures / plants / items / people / animals: every sheet in the folder, labelled with file name and size.
"""
import os
import sys

from PIL import Image, ImageDraw, ImageFont

from draw import rng
from pal import ROOT

ASSETS = os.path.join(ROOT, 'godot', 'assets')
OUT = os.path.join(ROOT, 'shots', 'out')
BG = (46, 42, 52)
BG2 = (64, 58, 72)
FONT = ImageFont.load_default()
LABEL_H = 12


def label(d, x, y, text):
    d.text((x, y), text, fill=(235, 230, 220), font=FONT)


def tiled_preview(img, cols=4, rows=3, seed='preview'):
    """Mixed-variant tiling of a 32x32-per-variant terrain sheet (first row only)."""
    n = img.width // 32
    r = rng('contact', seed)
    out = Image.new('RGBA', (cols * 32, rows * 32))
    for j in range(rows):
        for i in range(cols):
            v = r.randrange(n)
            out.paste(img.crop((v * 32, 0, v * 32 + 32, 32)), (i * 32, j * 32))
    return out


def gather(folder):
    path = os.path.join(ASSETS, folder)
    if not os.path.isdir(path):
        return []
    files = sorted(f for f in os.listdir(path) if f.endswith('.png'))
    return [(f[:-4], Image.open(os.path.join(path, f)).convert('RGBA')) for f in files]


def compose(items, max_w=1024, pad=8):
    """items: list of (name, image). Flow layout with labels."""
    rows, cur, cur_w, cur_h = [], [], 0, 0
    def cell_w(name, im):
        return max(im.width, int(FONT.getlength('%s %dx%d' % (name, im.width, im.height))) + 2)
    for name, im in items:
        w = cell_w(name, im) + pad
        if cur and cur_w + w > max_w:
            rows.append((cur, cur_h))
            cur, cur_w, cur_h = [], 0, 0
        cur.append((name, im))
        cur_w += w
        cur_h = max(cur_h, im.height)
    if cur:
        rows.append((cur, cur_h))
    H = sum(h + LABEL_H + pad * 2 for _, h in rows) + pad
    sheet = Image.new('RGBA', (max_w, H), BG)
    d = ImageDraw.Draw(sheet)
    y = pad
    for items_row, h in rows:
        x = pad
        for name, im in items_row:
            label(d, x, y, '%s %dx%d' % (name, im.width, im.height))
            d.rectangle([x - 1, y + LABEL_H - 1, x + im.width, y + LABEL_H + im.height], fill=BG2)
            sheet.alpha_composite(im, (x, y + LABEL_H))
            x += cell_w(name, im) + pad
        y += h + LABEL_H + pad * 2
    return sheet


def save(sheet, name):
    os.makedirs(OUT, exist_ok=True)
    sheet.convert('RGB').save(os.path.join(OUT, name + '.png'))
    big = sheet.resize((sheet.width * 2, sheet.height * 2), Image.NEAREST)
    big.convert('RGB').save(os.path.join(OUT, name + '@2x.png'))


def terrain_sheet():
    items = []
    for name, im in gather('terrain'):
        items.append((name, im))
        items.append((name + ' tiled', tiled_preview(im, seed=name)))
    save(compose(items, max_w=900), 'contact_terrain')


def folder_sheet(folders, out, max_w=1024):
    items = []
    for f in folders:
        items += gather(f)
    save(compose(items, max_w=max_w), out)


def main():
    terrain_sheet()
    folder_sheet(['overlays'], 'contact_overlays')
    folder_sheet(['effects'], 'contact_effects')
    folder_sheet(['structures'], 'contact_structures')
    folder_sheet(['plants'], 'contact_plants')
    folder_sheet(['items', 'light'], 'contact_items')
    folder_sheet(['people'], 'contact_people')
    folder_sheet(['animals'], 'contact_animals')
    print('contact sheets -> %s' % OUT)


if __name__ == '__main__':
    main()
