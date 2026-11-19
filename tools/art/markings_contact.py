#!/usr/bin/env python3
"""Review sheet for the hairless kinds' head markings (#485): per kind, its first and last skin
tone, the idle frame facing S and W, without a marking and with each of the three. Writes
docs/reviews/485/markings@4x.png. Composes exactly as the client does: body, clothing (tunic),
then the marking in the hair slot, each with its swap.

Run: python3 tools/art/markings_contact.py
"""
import json
import os
import re
import sys

import numpy as np
from PIL import Image, ImageDraw

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pal import ROOT  # noqa: E402
from people_contact import swap, over  # noqa: E402

PEOPLE = os.path.join(ROOT, 'godot', 'assets', 'people')
KINDS = [('murrow', 'murrow', ''), ('skarri', 'skarri', 'winged_'), ('screeling', 'screeling', 'digitigrade_tail_'),
         ('meer', 'amphibious', 'amphibious_'), ('below', 'below', 'serpentine_')]


def skins(species):
    text = open(os.path.join(ROOT, 'data', 'peoples', 'species', species + '.toml'), encoding='utf-8').read()
    m = re.search(r'^\[art\][^\[]*?skin\s*=\s*\[([^\]]*)\]', text, re.M | re.S)
    return re.findall(r'"([^"]+)"', m.group(1))


def main():
    sw = json.load(open(os.path.join(PEOPLE, 'swaps.json')))
    names = sw['skin']['names']
    cells = sw['plan_cells']
    col_frame = sw['layout']['columns'].index('idle')
    rows = []
    for species, body_set, plan_prefix in KINDS:
        plan = plan_prefix[:-1] or 'biped'
        n = cells[plan]
        body_name = 'body_%s_average_adult' % body_set
        cloth_name = 'clothing_tunic_%saverage_adult' % (plan_prefix or ('murrow_' if body_set == 'murrow' else ''))
        tones = skins(species)
        for tone in (tones[0], tones[-1]):
            si = names.index(tone)
            body = swap(np.asarray(Image.open(os.path.join(PEOPLE, body_name + '.png')).convert('RGBA')),
                        sw['skin']['source'], sw['skin']['targets'][si])
            cloth = np.asarray(Image.open(os.path.join(PEOPLE, cloth_name + '.png')).convert('RGBA'))
            cloth = swap(cloth, sw['cloth']['source'], sw['cloth']['targets'][0])
            row = []
            for mark in (0, 1, 2, 3):
                img = over(body, cloth)
                if mark:
                    m = np.asarray(Image.open(os.path.join(PEOPLE, 'marks_%s_%d_average_adult.png' % (body_set, mark))).convert('RGBA'))
                    img = over(img, swap(m, sw['skin']['source'], sw['skin']['targets'][si]))
                for r in (0, 1):   # S, W
                    cell = img[r * n:(r + 1) * n, col_frame * n:(col_frame + 1) * n]
                    row.append(Image.fromarray(cell.astype(np.uint8)).resize((48, 48), Image.NEAREST) if n != 48 else Image.fromarray(cell.astype(np.uint8)))
            rows.append(('%s %s' % (species, tone), row))
    W, H = 110 + 8 * 50, 16 + len(rows) * 50
    sheet = Image.new('RGBA', (W, H), (70, 92, 64, 255))
    d = ImageDraw.Draw(sheet)
    for j, t in enumerate(['none S', 'none W', 'cap S', 'cap W', 'mask S', 'mask W', 'spots S', 'spots W']):
        d.text((110 + j * 50, 2), t, fill=(240, 236, 224))
    for i, (label, row) in enumerate(rows):
        d.text((4, 16 + i * 50 + 18), label, fill=(240, 236, 224))
        for j, c in enumerate(row):
            sheet.alpha_composite(c, (110 + j * 50, 16 + i * 50))
    out = os.path.join(ROOT, 'docs', 'reviews', '485', 'markings@4x.png')
    sheet.resize((W * 4, H * 4), Image.NEAREST).save(out)
    print('wrote', out)


if __name__ == '__main__':
    main()
