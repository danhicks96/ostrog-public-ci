#!/usr/bin/env python3
"""Build compact before/after evidence for the underground art review."""
import os

from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
OUT = os.path.join(ROOT, 'docs', 'reviews', 'c414-underground-assets-2026-10-30')
BEFORE = os.path.join(OUT, 'before')
ASSETS = os.path.join(ROOT, 'godot', 'assets')
FONT = ImageFont.load_default()
NAMES = ['cave_floor', 'black_stone', 'dark_stone', 'ore_iron', 'ore_copper', 'ore_coal',
         'ore_gem', 'ore_tin', 'ore_silver', 'ore_gold']


def panel(folder, title, out_name):
    cells = []
    for name in NAMES:
        path = os.path.join(folder, name + '.png')
        im = Image.open(path).convert('RGBA')
        # Show all variants in a readable 2x strip without introducing filtering.
        im = im.resize((im.width * 2, im.height * 2), Image.Resampling.NEAREST)
        cells.append((name, im))
    w = 1024
    cell_w, cell_h = 512, 88
    canvas = Image.new('RGB', (w, 10 + len(cells) * cell_h), (30, 28, 34))
    d = ImageDraw.Draw(canvas)
    d.text((10, 5), title, fill=(240, 236, 224), font=FONT)
    y = 25
    for name, im in cells:
        canvas.paste(im, (10, y), im)
        d.text((530, y + 28), name, fill=(240, 236, 224), font=FONT)
        y += cell_h
    canvas.save(os.path.join(OUT, out_name))


def main():
    os.makedirs(OUT, exist_ok=True)
    panel(BEFORE, 'Before — underground terrain and deposits', 'before.png')
    panel(os.path.join(ASSETS, 'terrain'), 'After — underground terrain and deposits', 'after.png')
    print('underground review: %s' % OUT)


if __name__ == '__main__':
    main()
