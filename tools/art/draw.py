"""Tiny deterministic pixel-drawing kit shared by the Ostrog art generators.

Canvas pixels are RGBA uint8. Colours are (r,g,b) tuples from pal.py; alpha is 255 unless
given. Templates are lists of equal-length strings; '.' is transparent and every other
character is looked up in a mapping dict to a colour.
"""
import os
import random

import numpy as np
from PIL import Image


class Canvas:
    def __init__(self, w, h, fill=None):
        self.w, self.h = w, h
        self.a = np.zeros((h, w, 4), dtype=np.uint8)
        if fill is not None:
            self.a[:, :, :3] = fill
            self.a[:, :, 3] = 255

    # --- primitives -------------------------------------------------------
    def px(self, x, y, c, alpha=255):
        if 0 <= x < self.w and 0 <= y < self.h:
            self.a[y, x, :3] = c
            self.a[y, x, 3] = alpha

    def get(self, x, y):
        if 0 <= x < self.w and 0 <= y < self.h:
            return tuple(int(v) for v in self.a[y, x])
        return None

    def rect(self, x, y, w, h, c, alpha=255):
        x0, y0 = max(0, x), max(0, y)
        x1, y1 = min(self.w, x + w), min(self.h, y + h)
        if x1 > x0 and y1 > y0:
            self.a[y0:y1, x0:x1, :3] = c
            self.a[y0:y1, x0:x1, 3] = alpha

    def hline(self, x0, x1, y, c):
        for x in range(min(x0, x1), max(x0, x1) + 1):
            self.px(x, y, c)

    def vline(self, x, y0, y1, c):
        for y in range(min(y0, y1), max(y0, y1) + 1):
            self.px(x, y, c)

    def line(self, x0, y0, x1, y1, c):
        dx, dy = abs(x1 - x0), -abs(y1 - y0)
        sx, sy = (1 if x0 < x1 else -1), (1 if y0 < y1 else -1)
        err = dx + dy
        while True:
            self.px(x0, y0, c)
            if x0 == x1 and y0 == y1:
                break
            e2 = 2 * err
            if e2 >= dy:
                err += dy
                x0 += sx
            if e2 <= dx:
                err += dx
                y0 += sy

    def disc(self, cx, cy, rx, ry, c, alpha=255):
        """Filled ellipse centred on pixel (cx, cy) with radii rx, ry (floats ok)."""
        for y in range(int(cy - ry) - 1, int(cy + ry) + 2):
            for x in range(int(cx - rx) - 1, int(cx + rx) + 2):
                u = (x - cx) / (rx + 0.5)
                v = (y - cy) / (ry + 0.5)
                if u * u + v * v <= 1.0:
                    self.px(x, y, c, alpha)

    def outline(self, c):
        """1-px outline in colour c around every opaque region (drawn into transparent pixels)."""
        a = self.a[:, :, 3] > 0
        pad = np.pad(a, 1)
        nb = pad[:-2, 1:-1] | pad[2:, 1:-1] | pad[1:-1, :-2] | pad[1:-1, 2:]
        edge = nb & ~a
        self.a[edge, :3] = c
        self.a[edge, 3] = 255

    def stamp(self, tpl, x, y, m, wrap=None):
        """Draw template tpl at (x,y). m maps characters to colours. wrap=(w,h) wraps
        coordinates inside that box (seamless tiles)."""
        for j, row in enumerate(tpl):
            for i, ch in enumerate(row):
                if ch == '.' or ch == ' ':
                    continue
                c = m[ch]
                if c is None:
                    continue
                px, py = x + i, y + j
                if wrap:
                    px %= wrap[0]
                    py %= wrap[1]
                self.px(px, py, c)

    def paste(self, other, x, y):
        """Alpha-over paste of another Canvas (binary alpha)."""
        for j in range(other.h):
            for i in range(other.w):
                if other.a[j, i, 3] > 0:
                    self.px(x + i, y + j, tuple(other.a[j, i, :3]), int(other.a[j, i, 3]))

    def sub(self, x, y, w, h):
        c = Canvas(w, h)
        c.a[:, :] = self.a[y:y + h, x:x + w]
        return c

    def flip_h(self):
        c = Canvas(self.w, self.h)
        c.a[:, :] = self.a[:, ::-1]
        return c

    def image(self):
        return Image.fromarray(self.a, 'RGBA')

    def save(self, path):
        os.makedirs(os.path.dirname(path), exist_ok=True)
        self.image().save(path, optimize=True)


class Tile(Canvas):
    """A canvas whose px() wraps, so motifs that cross an edge continue on the other side."""

    def px(self, x, y, c, alpha=255):
        Canvas.px(self, x % self.w, y % self.h, c, alpha)


def rng(*tags):
    """Deterministic random stream keyed by purpose tags."""
    return random.Random('ostrog|' + '|'.join(str(t) for t in tags))


def scatter(r, n, w, h, margin=0, min_dist=0, tries=40):
    """n deterministic positions with simple blue-noise rejection."""
    pts = []
    for _ in range(n):
        for _t in range(tries):
            x = r.randint(margin, w - 1 - margin)
            y = r.randint(margin, h - 1 - margin)
            if all((x - a) ** 2 + (y - b) ** 2 >= min_dist ** 2 for a, b in pts):
                pts.append((x, y))
                break
    return pts


def sheet(canvases, cell_w, cell_h, cols=None):
    """Lay canvases out left-to-right (wrapping to rows if cols given)."""
    n = len(canvases)
    cols = cols or n
    rows = (n + cols - 1) // cols
    out = Canvas(cell_w * cols, cell_h * rows)
    for i, c in enumerate(canvases):
        out.a[(i // cols) * cell_h:(i // cols) * cell_h + c.h, (i % cols) * cell_w:(i % cols) * cell_w + c.w] = c.a
    return out


def ensure_dir(path):
    os.makedirs(path, exist_ok=True)
    return path
