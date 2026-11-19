#!/usr/bin/env python3
"""A line-up of every kind of people (the MP review's readability check): each species' body,
adult and child of the average build, idle and walking facing south and west, side by side at
the size the game draws them, then the same at 2x, three kinds to a row. A kind whose silhouette reads like another's
shows at once.

Bodies are shown in their sheet colours, before the runtime's skin swap, so shape is what is
compared. Plans with bigger cells (taur 64, huge biped and serpentine 96) sit on the same feet
line as the 48-pixel kinds.

Run: python3 tools/art/lineup.py OUT.png
"""
import json
import os
import sys

from PIL import Image, ImageDraw

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pal import ROOT  # noqa: E402

PEOPLE = os.path.join(ROOT, 'godot', 'assets', 'people')
# (label, body sheet, plan) in the library's order: the six of the land, the three of the deep,
# the last three anchors.
KINDS = [
    ('human', 'body', 'biped'),
    ('Hollin', 'body_hollin', 'biped'),
    ('Grisk', 'body_grisk', 'small_biped'),
    ('Screeling', 'body_screeling', 'digitigrade_tail'),
    ('Skarri', 'body_skarri', 'winged'),
    ('Tharn', 'body_tharn', 'four_armed'),
    ('Murrow', 'body_murrow', 'biped'),
    ('Holm', 'body_holm', 'huge_biped'),
    ('Below', 'body_below', 'serpentine'),
    ('Meer', 'body_amphibious', 'amphibious'),
    ('Gnarrow', 'body_taur', 'taur'),
    ('Seamfolk', 'body_seamfolk', 'biped'),
]
FRAMES = [('idle', 'S'), ('walk2', 'S'), ('idle', 'W'), ('walk2', 'W')]


def main(out):
    swaps = json.load(open(os.path.join(PEOPLE, 'swaps.json')))
    cols = swaps['layout']['columns']
    rows = swaps['layout']['rows']
    cells = swaps['plan_cells']
    tallest = max(cells.values())
    pad = 4
    per_row = 3
    band = tallest + 14
    widths = [(cells[p] * len(FRAMES) + pad) * 2 for _, _, p in KINDS]
    width = max(sum(widths[i:i + per_row]) for i in range(0, len(KINDS), per_row))
    img = Image.new('RGBA', (width, band * ((len(KINDS) + per_row - 1) // per_row)),
                    (40, 44, 52, 255))
    draw = ImageDraw.Draw(img)
    for n, (label, body, plan) in enumerate(KINDS):
        if n % per_row == 0:
            x = 0
        y = (n // per_row) * band
        c = cells[plan]
        for age in ('adult', 'child'):
            path = os.path.join(PEOPLE, f'{body}_average_{age}.png')
            if not os.path.exists(path):
                continue
            sheet = Image.open(path).convert('RGBA')
            for k, (col, row) in enumerate(FRAMES):
                fx, fy = cols.index(col) * c, rows.index(row) * c
                frame = sheet.crop((fx, fy, fx + c, fy + c))
                img.alpha_composite(frame, (x + k * c, y + tallest - c))
            draw.text((x, y + tallest + 2), f'{label} {age[0]}', fill=(230, 230, 230, 255))
            x += c * len(FRAMES) + pad
    img.save(out)
    big = img.resize((img.width * 2, img.height * 2), Image.NEAREST)
    big.save(out.replace('.png', '@2x.png'))
    print(out, img.size)


if __name__ == '__main__':
    main(sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, 'shots', 'out', 'lineup.png'))
