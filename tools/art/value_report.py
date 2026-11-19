#!/usr/bin/env python3
"""Mean luminance per terrain family, and which families are too close to tell apart.

The art review (docs/reviews/art-review-2026-10-14/review.md) found too many families living
in one mid-value band: they read as the same tile at gameplay zoom even though their hues
differ. This prints the numbers so a change can be judged instead of argued about.

Run WITHOUT -I (isolated mode drops user site-packages, where Pillow lives here):
    python3 tools/art/value_report.py
"""
import os

from PIL import Image

from pal import ROOT

ASSETS = os.path.join(ROOT, 'godot', 'assets', 'terrain')
CLOSE = 12.0       # luminance points within which two families read as "the same value"
HUE_CLOSE = 30.0   # colour distance within which they also read as "the same stuff"


def lum_and_mean(img):
    px = img.convert('RGBA').getdata()
    tl = tr = tg = tb = n = 0
    for r, g, b, a in px:
        if a == 0:
            continue
        tl += 0.299 * r + 0.587 * g + 0.114 * b
        tr += r
        tg += g
        tb += b
        n += 1
    if not n:
        return 0.0, (0, 0, 0)
    return tl / n, (tr / n, tg / n, tb / n)


def dist(a, b):
    """Perceptual-ish RGB distance (Rec. 601 weights)."""
    dr, dg, db = a[0] - b[0], a[1] - b[1], a[2] - b[2]
    return (2 * dr * dr + 4 * dg * dg + 3 * db * db) ** 0.5


def main():
    import sys
    assets = sys.argv[1] if len(sys.argv) > 1 else ASSETS
    rows = []
    for name in sorted(os.listdir(assets)):
        if not name.endswith('.png'):
            continue
        im = Image.open(os.path.join(assets, name))
        v, mean = lum_and_mean(im)
        rows.append((name[:-4], v, mean))
    rows.sort(key=lambda r: r[1])
    print('%-14s %s' % ('family', 'luma'))
    for name, v, _ in rows:
        print('%-14s %6.1f' % (name, v))

    # A pair is only a real problem when it is close in BOTH value and colour: families that
    # share a value but differ in hue (grass vs clay) separate fine at a glance, and families
    # that share a hue but differ in value separate by their light and shadow.
    print('\nconfusable: within %.0f luma AND %.0f colour distance' % (CLOSE, HUE_CLOSE))
    clashes = []
    for i in range(len(rows)):
        for j in range(i + 1, len(rows)):
            dv = rows[j][1] - rows[i][1]
            dh = dist(rows[i][2], rows[j][2])
            if dv < CLOSE and dh < HUE_CLOSE:
                clashes.append((rows[i][0], rows[j][0], dv, dh))
    if not clashes:
        print('  none')
    for a, b, dv, dh in clashes:
        print('  %-12s ~ %-12s  (%.1f luma, %.1f colour apart)' % (a, b, dv, dh))
    print('\n%d families, %d confusable pairs' % (len(rows), len(clashes)))


if __name__ == '__main__':
    main()
