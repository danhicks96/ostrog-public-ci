#!/usr/bin/env python3
"""Audio report: levels, loop seams and a spectrogram sheet for every OGG in godot/assets/audio,
plus the music stems in godot/assets/audio/music (listed in music.json; every stem loops).

    python3 tools/audio/audio_report.py [OUT.png]

Prints per file: duration, channels, peak dBFS, RMS dBFS, and for loops the seam jump (the
sample step from the last sample back to the first, relative to the clip's RMS; a click is
audible well above ~3). Exits 1 if any peak clips (>= -0.1 dBFS) or any loop seam exceeds 4.
The sheet shows a log-frequency spectrogram (60 Hz to 16 kHz) and the waveform per file; the
music stems go on a second sheet, OUT_music.png. For music each cue's stems are also summed as
the client plays them at full tension, and that mix must not clip either.
"""
import json
import math
import os
import subprocess
import sys

import numpy as np
from PIL import Image, ImageDraw

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
AUD = os.path.join(ROOT, 'godot', 'assets', 'audio')
SR = 44100


def decode(path):
    raw = subprocess.run(['ffmpeg', '-loglevel', 'error', '-i', path, '-f', 's16le', '-ac', '2',
                          '-ar', str(SR), '-'], check=True, capture_output=True).stdout
    x = np.frombuffer(raw, dtype='<i2').astype(np.float64) / 32768.0
    return x.reshape(-1, 2)


def db(v):
    return 20 * math.log10(max(v, 1e-9))


def spectro(mono, w=360, h=90):
    n = 2048
    hop = max(256, len(mono) // w)
    frames = []
    win = np.hanning(n)
    for i in range(0, max(1, len(mono) - n), hop):
        seg = mono[i:i + n]
        if len(seg) < n:
            break
        frames.append(np.abs(np.fft.rfft(seg * win)))
    if not frames:
        return np.zeros((h, w))
    S = np.array(frames).T
    f = np.fft.rfftfreq(n, 1 / SR)
    rows = np.geomspace(60, 16000, h)
    idx = np.searchsorted(f, rows)
    S = S[np.clip(idx, 0, len(f) - 1)][::-1]
    S = 20 * np.log10(S + 1e-6)
    S = np.clip((S - (S.max() - 70)) / 70, 0, 1)
    img = Image.fromarray((S * 255).astype(np.uint8)).resize((w, h))
    return np.array(img) / 255.0


def check(files, loops, label=None):
    """Print levels and seams for files (paths relative to AUD); returns (failures, tiles, decoded)."""
    bad = 0
    tiles = []
    decoded = {}
    for f in files:
        x = decode(os.path.join(AUD, f))
        decoded[f] = x
        mono = x.mean(axis=1)
        peak = float(np.max(np.abs(x)))
        rms = math.sqrt(float(np.mean(x ** 2)))
        line = '%-18s %6.2fs peak %6.1f dB rms %6.1f dB' % (f, len(x) / SR, db(peak), db(rms))
        if peak >= 0.989:
            line += '  CLIPS'
            bad += 1
        if f in loops:
            jump = float(np.max(np.abs(x[0] - x[-1]))) / (rms + 1e-9)
            line += '  seam %.2f' % jump
            if jump > 4.0:
                line += '  CLICK'
                bad += 1
        print(line)
        sp = spectro(mono)
        tile = Image.new('RGB', (360, 130), (12, 12, 16))
        rgb = np.stack([sp * 255, sp * 200, sp * 90 + 30], axis=-1).astype(np.uint8)
        tile.paste(Image.fromarray(rgb), (0, 14))
        d = ImageDraw.Draw(tile)
        d.text((3, 1), f, fill=(230, 230, 230))
        # waveform strip
        wv = mono[:: max(1, len(mono) // 360)][:360]
        for i, v in enumerate(wv):
            y = 117 - int(v / (peak + 1e-9) * 12)
            d.line([(i, 117), (i, y)], fill=(120, 180, 220))
        tiles.append(tile)
    return bad, tiles, decoded


def sheet(tiles, out_png):
    cols = 4
    rows = (len(tiles) + cols - 1) // cols
    sheet = Image.new('RGB', (cols * 364, rows * 134), (0, 0, 0))
    for i, t in enumerate(tiles):
        sheet.paste(t, ((i % cols) * 364, (i // cols) * 134))
    os.makedirs(os.path.dirname(out_png), exist_ok=True)
    sheet.save(out_png)
    print('sheet -> %s' % out_png)


def main():
    out_png = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, 'docs', 'reviews', 'audio_report.png')
    man = json.load(open(os.path.join(AUD, 'sounds.json')))
    loops = {f for m in man.values() if m['loop'] for f in m['files']}
    files = sorted(f for f in os.listdir(AUD) if f.endswith('.ogg'))
    bad, tiles, _ = check(files, loops)
    sheet(tiles, out_png)
    mpath = os.path.join(AUD, 'music.json')
    if os.path.exists(mpath):
        music = json.load(open(mpath))
        mfiles = [c['files'][s] for c in music['cues'].values() for s in music['stems']]
        missing = [f for f in mfiles if not os.path.exists(os.path.join(AUD, f))]
        for f in missing:
            print('%-18s MISSING' % f)
        bad += len(missing)
        mbad, mtiles, dec = check([f for f in mfiles if f not in missing], set(mfiles))
        bad += mbad
        for name, c in sorted(music['cues'].items()):
            xs = [dec[c['files'][s]] for s in music['stems'] if c['files'][s] in dec]
            if not xs:
                continue
            lens = {len(x) for x in xs}
            n = min(lens)
            mix = sum(x[:n] for x in xs)
            peak = float(np.max(np.abs(mix)))
            rms = math.sqrt(float(np.mean(mix ** 2)))
            line = 'mix %-14s %6.2fs peak %6.1f dB rms %6.1f dB' % (name, n / SR, db(peak), db(rms))
            if len(lens) > 1:
                line += '  STEM LENGTHS DIFFER %s' % sorted(lens)
                bad += 1
            if abs(n / SR - c['seconds']) > 0.01:
                line += '  LENGTH %.3f != manifest %.3f' % (n / SR, c['seconds'])
                bad += 1
            if peak >= 0.989:
                line += '  CLIPS'
                bad += 1
            print(line)
        root, ext = os.path.splitext(out_png)
        sheet(mtiles, root + '_music' + ext)
    sys.exit(1 if bad else 0)


if __name__ == '__main__':
    main()
