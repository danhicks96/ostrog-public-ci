#!/usr/bin/env python3
"""Light texture: godot/assets/light/soft.png, 128x128 white with a smooth radial alpha
falloff (full at the centre, zero at the edge; smoothstep so the rim does not band).
Exempt from palette_check (it is a mask, not drawn art)."""
import os

import numpy as np
from PIL import Image

from pal import ROOT

OUT = os.path.join(ROOT, 'godot', 'assets', 'light')
SIZE = 128


def main():
    os.makedirs(OUT, exist_ok=True)
    y, x = np.mgrid[0:SIZE, 0:SIZE]
    cx = cy = (SIZE - 1) / 2.0
    d = np.sqrt((x - cx) ** 2 + (y - cy) ** 2) / (SIZE / 2.0)
    t = np.clip(1.0 - d, 0.0, 1.0)
    a = t * t * (3 - 2 * t)                      # smoothstep
    img = np.zeros((SIZE, SIZE, 4), dtype=np.uint8)
    img[:, :, :3] = 255
    img[:, :, 3] = np.round(a * 255).astype(np.uint8)
    Image.fromarray(img, 'RGBA').save(os.path.join(OUT, 'soft.png'), optimize=True)
    print('light -> %s' % OUT)


if __name__ == '__main__':
    main()
