#!/usr/bin/env python3
"""Images for README.md: the thumbnail and small contact sheets, written to docs/readme/.

    python3 tools/docs/readme_images.py

thumbnail.png      four in-game frames from the review folders (not staged art), 2x2
terrain.png        the first variant of every terrain family, labelled
structures.png     every structure sheet's first frame, labelled
plants.png         every plant sheet, labelled (trees by stage and season)
items.png          every item icon at 2x, labelled
people.png         eight composed people (body + clothing + hair, palette-swapped as the client
                   does) in all four facings, plus one figure's full frame strip
animals.png        every animal sheet, first idle frame per facing

Needs Pillow (run without -I). Every output is quantised to a 256-colour PNG and must stay
under 400 KB (the thumbnail) or 300 KB (the sheets); the script fails if one does not.
"""
import json
import os
import sys

from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
A = os.path.join(ROOT, 'godot', 'assets')
OUT = os.path.join(ROOT, 'docs', 'readme')
FONT = ImageFont.load_default()
BG = (38, 36, 44)
FG = (226, 222, 210)
T = 32

# Seeded, unscripted frames from the C1 and C5 reviews (seed and ticks in their READMEs).
THUMB = [
    'docs/reviews/render-2026-10-16/noon.png',
    'docs/reviews/render-2026-10-16/autumn_noon.png',
    'docs/reviews/render-2026-10-16/winter_forest_2x.png',
    'docs/reviews/techniques-2026-10-16/fire_night_unfought.png',
]


def save(img, name, limit_kb):
    p = os.path.join(OUT, name)
    img.convert('RGB').quantize(colors=256, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE).save(p, optimize=True)
    kb = os.path.getsize(p) / 1024
    print(f'{name}: {img.size[0]}x{img.size[1]}, {kb:.0f} KB')
    if kb > limit_kb:
        sys.exit(f'{name} is {kb:.0f} KB, over {limit_kb} KB')


def thumbnail():
    w, h = 640, 310  # 1280x620 of each frame: below the HUD, above most of the feed box
    out = Image.new('RGB', (w * 2, h * 2), BG)
    for i, f in enumerate(THUMB):
        im = Image.open(os.path.join(ROOT, f)).convert('RGB')
        # Skip the two HUD lines at the top; keep the middle of the frame at 1:1 pixels so the
        # pixel art reads, then halve it.
        cw, ch = w * 2, h * 2
        x0 = (im.width - cw) // 2
        y0 = 60
        im = im.crop((x0, y0, x0 + cw, y0 + ch)).resize((w, h), Image.Resampling.LANCZOS)
        out.paste(im, ((i % 2) * w, (i // 2) * h))
    save(out, 'thumbnail.png', 400)


def label_grid(cells, cols, cell_w, cell_h, scale=2, label_h=12):
    """cells: [(label, RGBA image)]. Each image is scaled by `scale` and centred in its cell."""
    rows = (len(cells) + cols - 1) // cols
    W, H = cols * cell_w, rows * (cell_h + label_h)
    out = Image.new('RGBA', (W, H), BG + (255,))
    d = ImageDraw.Draw(out)
    for i, (lab, im) in enumerate(cells):
        cx, cy = (i % cols) * cell_w, (i // cols) * (cell_h + label_h)
        im = im.resize((im.width * scale, im.height * scale), Image.Resampling.NEAREST)
        if im.width > cell_w or im.height > cell_h:
            f = min(cell_w / im.width, cell_h / im.height)
            im = im.resize((max(1, int(im.width * f)), max(1, int(im.height * f))), Image.Resampling.NEAREST)
        out.alpha_composite(im, (cx + (cell_w - im.width) // 2, cy + cell_h - im.height))
        d.text((cx + 2, cy + cell_h + 1), lab[:cell_w // 6], fill=FG, font=FONT)
    return out


def sheet(folder, name):
    return Image.open(os.path.join(A, folder, name)).convert('RGBA')


def terrain():
    cells = []
    for n in sorted(os.listdir(os.path.join(A, 'terrain'))):
        if n.endswith('.png'):
            cells.append((n[:-4], sheet('terrain', n).crop((0, 0, T, T))))
    save(label_grid(cells, 8, 96, 64), 'terrain.png', 300)


def structures():
    cells = []
    for n in sorted(os.listdir(os.path.join(A, 'structures'))):
        if not n.endswith('.png'):
            continue
        im = sheet('structures', n)
        if n.startswith('wall_'):
            im = im.crop((15 * T, 0, 16 * T, T))  # piece 15: joined on all four sides
        elif im.width > T:
            frames = im.width // T
            im = im.crop(((frames - 1) * T, 0, frames * T, im.height))  # last frame (lit / burning)
        cells.append((n[:-4], im))
    save(label_grid(cells, 8, 96, 64), 'structures.png', 300)


def plants():
    cells = []
    for n in sorted(os.listdir(os.path.join(A, 'plants'))):
        if n.endswith('.png'):
            im = sheet('plants', n)
            if im.width > 64:
                im = im.crop((0, 0, T, im.height))
            cells.append((n[:-4], im))
    save(label_grid(cells, 10, 96, 128), 'plants.png', 300)


def items():
    cells = []
    for n in sorted(os.listdir(os.path.join(A, 'items'))):
        if n.endswith('.png'):
            cells.append((n[:-4], sheet('items', n)))
    save(label_grid(cells, 9, 96, 40), 'items.png', 300)


def swap(im, src, dst):
    px = im.load()
    m = {tuple(int(s[i:i + 2], 16) for i in (1, 3, 5)): tuple(int(t[i:i + 2], 16) for i in (1, 3, 5))
         for s, t in zip(src, dst)}
    for y in range(im.height):
        for x in range(im.width):
            r, g, b, a = px[x, y]
            if a and (r, g, b) in m:
                px[x, y] = m[(r, g, b)] + (a,)
    return im


def people():
    sw = json.load(open(os.path.join(A, 'people', 'swaps.json')))
    cast = [  # body, hair, outfit, skin, hair colour, cloth colour
        ('slim_adult', 3, 'dress', 4, 4, 8), ('average_adult', 1, 'tunic', 2, 1, 2),
        ('broad_adult', 6, 'apron', 1, 0, 1), ('average_child', 2, 'tunic', 3, 3, 6),
        ('slim_adult', 4, 'cloak', 5, 5, 9), ('broad_adult', 9, 'coat', 0, 6, 11),
        ('average_adult', 7, 'workshirt', 3, 2, 10), ('slim_child', 10, 'dress', 2, 7, 4),
    ]
    layers = []
    for body, hair, outfit, sk, hc, cc in cast:
        b = swap(sheet('people', f'body_{body}.png'), sw['skin']['source'], sw['skin']['targets'][sk])
        c = swap(sheet('people', f'clothing_{outfit}.png'), sw['cloth']['source'], sw['cloth']['targets'][cc])
        h = swap(sheet('people', f'hair_{hair:02d}.png'), sw['hair']['source'], sw['hair']['targets'][hc])
        im = b.copy()
        im.alpha_composite(c)
        im.alpha_composite(h)
        layers.append((f'{body} {outfit}', im))
    cells = []
    for lab, im in layers:  # idle, every facing
        for r, f in enumerate('SWEN'):
            cells.append((f'{lab.split()[1]}-{f}' if r == 0 else f, im.crop((0, r * T, T, (r + 1) * T))))
    names = ['idle1', 'idle2', 'idle3', 'walk1', 'walk2', 'walk3', 'walk4', 'work1', 'work2',
             'carry1', 'carry2', 'sleep', 'sit', 'turn']
    im = layers[1][1]
    cols = im.width // T
    for c in range(cols):
        cells.append((names[c] if c < len(names) else str(c), im.crop((c * T, 0, (c + 1) * T, T))))
    save(label_grid(cells, 16, 64, 64), 'people.png', 300)


def animals():
    cells = []
    for n in sorted(os.listdir(os.path.join(A, 'animals'))):
        if not n.endswith('.png'):
            continue
        im = sheet('animals', n)
        ch = im.height // 4
        for r, f in enumerate('SWEN'):
            cells.append((f'{n[:-4]}-{f}' if r == 0 else f, im.crop((0, r * ch, T, (r + 1) * ch))))
    save(label_grid(cells, 8, 96, 96), 'animals.png', 300)


def main():
    os.makedirs(OUT, exist_ok=True)
    thumbnail()
    terrain()
    structures()
    plants()
    items()
    people()
    animals()


if __name__ == '__main__':
    main()
