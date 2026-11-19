"""Palette helper for Ostrog generators.

    from pal import Palette
    pal = Palette.load()
    pal.ramp('foliage', 3)      -> (r, g, b)
    pal.neutral('black')        -> (r, g, b)
    pal.nearest((r, g, b))      -> (r, g, b) from the palette
"""
import json
import os

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
PALETTE_JSON = os.path.join(ROOT, 'godot', 'assets', 'palette', 'ostrog.json')
RAMPS = ['earth', 'stone', 'foliage', 'water', 'warm', 'cool', 'skin', 'gold', 'copper', 'olive', 'gem']
NEUTRALS = ['black', 'shadow', 'light', 'white']


def hex_to_rgb(h):
    h = h.lstrip('#')
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


def rgb_to_hex(c):
    return '#%02x%02x%02x' % tuple(int(v) for v in c[:3])


class Palette:
    def __init__(self, entries):
        self.entries = entries
        self.by_name = {e['name']: hex_to_rgb(e['hex']) for e in entries}
        self.ramps = {}
        for e in entries:
            if e['ramp'] in RAMPS:
                self.ramps.setdefault(e['ramp'], {})[e['step']] = hex_to_rgb(e['hex'])
        self.colours = [hex_to_rgb(e['hex']) for e in entries]
        self.colour_set = set(self.colours)
        self._near = {}

    @classmethod
    def load(cls, path=PALETTE_JSON):
        with open(path) as f:
            return cls(json.load(f))

    def ramp(self, name, step):
        step = max(0, min(7, int(step)))
        return self.ramps[name][step]

    def neutral(self, name):
        return self.by_name[name]

    def has(self, rgb):
        return tuple(rgb[:3]) in self.colour_set

    def nearest(self, rgb):
        key = tuple(int(v) for v in rgb[:3])
        hit = self._near.get(key)
        if hit is None:
            r, g, b = key
            best = min(self.colours, key=lambda c: (c[0] - r) ** 2 * 2 + (c[1] - g) ** 2 * 4 + (c[2] - b) ** 2 * 3)
            self._near[key] = best
            hit = best
        return hit

    # Shortcuts used by every generator: a ramp step as an (r,g,b) tuple.
    def __call__(self, ramp, step):
        if ramp in NEUTRALS or ramp in ('hair',):
            return self.by_name[ramp] if ramp in self.by_name else self.by_name[ramp + '_%d' % step]
        return self.ramp(ramp, step)
