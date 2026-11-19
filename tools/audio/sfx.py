#!/usr/bin/env python3
"""Ostrog's sound effects and ambience: godot/assets/audio/<name>.ogg plus sounds.json, a
manifest the client reads (variants per sound, which ones loop, their bus).

    python3 tools/audio/sfx.py

All sounds are synthesised here from tools/audio/synth.py; nothing is sampled or downloaded.
Each sound's seed is its own name, so a rebuild writes the same audio.
"""
import json
import math
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import synth as S  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
OUT = os.path.join(ROOT, 'godot', 'assets', 'audio')
SR = S.SR
MANIFEST = {}


def emit(name, x, bus, loop=False, rms=0.12, stereo_ok=True):
    x = S.normalise(x, rms)
    if not loop:
        x = S.fade(x, 0.002, 0.03)
    S.write_ogg(os.path.join(OUT, name + '.ogg'), x)
    base = name.rsplit('_', 1)[0] if name[-1].isdigit() and '_' in name else name
    m = MANIFEST.setdefault(base, {'bus': bus, 'loop': loop, 'files': []})
    m['files'].append(name + '.ogg')


def jit(r, v, amt):
    return v * (1 + (r.random() * 2 - 1) * amt)


# ---------------------------------------------------------------- work
def chop(k):
    r = S.rng('chop%d' % k)
    n = int(SR * 0.5)
    body = S.modal(0.5, [(jit(r, 170, .08), .09, 1.0), (jit(r, 410, .08), .06, .6),
                         (jit(r, 770, .1), .035, .35), (jit(r, 1300, .1), .02, .2)], r)
    crack = S.band(S.noise(n, r), 1500, 6000) * S.env_ad(n, 0.0005, 0.02)
    split = S.band(S.noise(n, r), 600, 3000) * S.env_ad(n, 0.01, 0.09) * 0.25
    split = np.roll(split, int(SR * 0.012))
    return body + crack * 0.9 + split


def mine(k):
    r = S.rng('mine%d' % k)
    n = int(SR * 0.7)
    ring = S.modal(0.7, [(jit(r, 1750, .06), .07, .8), (jit(r, 2650, .06), .05, .5),
                         (jit(r, 4100, .08), .03, .35), (jit(r, 6300, .1), .02, .2)], r)
    click = S.highpass(S.noise(n, r), 2000) * S.env_ad(n, 0.0002, 0.012)
    thud = S.modal(0.7, [(jit(r, 120, .1), .05, .5)], r, 0)
    rubble = np.zeros(n)
    for _ in range(r.integers(6, 14)):
        at = int(SR * r.uniform(0.04, 0.45))
        ln = int(SR * 0.008)
        if at + ln < n:
            rubble[at:at + ln] += r.standard_normal(ln) * r.uniform(0.1, 0.4) * np.linspace(1, 0, ln)
    rubble = S.band(rubble, 900, 5000)
    return ring * 0.7 + click + thud + rubble * 0.6


def hammer(k):
    r = S.rng('hammer%d' % k)
    n = int(SR * 0.45)
    knock = S.modal(0.45, [(jit(r, 320, .07), .05, 1.0), (jit(r, 860, .07), .035, .5),
                           (jit(r, 1620, .1), .02, .25)], r)
    nail = S.modal(0.45, [(jit(r, 2400, .05), .12, .22), (jit(r, 3900, .05), .07, .1)], r, 0)
    return knock + nail


def saw(k):
    r = S.rng('saw%d' % k)
    dur = jit(r, 0.5, .15)
    n = int(SR * dur)
    t = np.arange(n) / SR
    teeth = 0.5 + 0.5 * np.sign(np.sin(2 * math.pi * jit(r, 75, .15) * t))
    rasp = S.band(S.noise(n, r), 1400, 6500) * (0.4 + 0.6 * teeth)
    stroke = np.sin(np.pi * t / dur) ** 1.5
    return rasp * stroke


def hoe(k):
    r = S.rng('hoe%d' % k)
    n = int(SR * 0.4)
    hit = S.band(S.noise(n, r), 180, 1600) * S.env_ad(n, 0.002, 0.06)
    scrape = S.band(S.noise(n, r), 400, 2500) * S.env_ad(n, 0.04, 0.18) * 0.35
    scrape = np.roll(scrape, int(SR * 0.05))
    return hit + scrape


def cook(k):
    """Stirring a pot over the fire: a spoon knocking the iron rim, a scrape round the bottom,
    and the stew bubbling under it."""
    r = S.rng('cook%d' % k)
    dur = 0.9
    n = int(SR * dur)
    clink = S.modal(dur, [(jit(r, 1150, .06), .05, 1.0), (jit(r, 2780, .06), .03, .45),
                          (jit(r, 4300, .08), .015, .2)], r, 0.0006)
    scrape = S.band(S.noise(n, r), 700, 3200) * S.env_ad(n, 0.08, 0.25) * 0.18
    scrape = np.roll(scrape, int(SR * 0.08))
    bubbles = np.zeros(n)
    for _ in range(int(r.integers(5, 9))):
        at = int(SR * r.uniform(0.05, dur - 0.12))
        ln = int(SR * r.uniform(0.025, 0.05))
        f0 = r.uniform(240, 520)
        tt = np.arange(ln) / SR
        blip = np.sin(2 * math.pi * (f0 * tt + 900 * tt * tt)) * np.sin(np.pi * tt / tt[-1]) ** 2
        bubbles[at:at + ln] += blip * r.uniform(0.2, 0.45)
    return clink * 0.55 + scrape + S.lowpass(bubbles, 2000)


def door(k):
    r = S.rng('door%d' % k)
    dur = 0.6
    n = int(SR * dur)
    creak = S.glide(jit(r, 170, .1), jit(r, 260, .1), dur, 0.7)
    rough = 0.5 + 0.5 * np.sign(np.sin(2 * math.pi * jit(r, 38, .2) * np.arange(n) / SR))
    creak = S.band(creak * rough, 150, 3000) * S.env_adsr(n, 0.05, 0.2, 0.5, 0.25) * 0.5
    knock = S.modal(dur, [(jit(r, 140, .1), .06, 1.0), (jit(r, 390, .1), .04, .5)], r)
    knock = np.roll(knock, int(SR * 0.45))
    knock[:int(SR * 0.45)] = 0
    return creak + knock * 0.6


# ---------------------------------------------------------------- fire
def fire_loop():
    r = S.rng('fire_loop')
    dur = 9.0
    n = int(SR * (dur + 1))
    t = np.arange(n) / SR
    bed = S.lowpass(S.brown(n, r), 650) * (0.7 + 0.3 * np.sin(2 * math.pi * t / 3.1) * np.sin(2 * math.pi * t / 1.7))
    crackle = np.zeros(n)
    k = int(r.poisson(9 * dur))
    for _ in range(k):
        at = int(r.uniform(0, n - 600))
        ln = int(SR * r.uniform(0.002, 0.009))
        amp = r.uniform(0.15, 1.0) ** 2
        crackle[at:at + ln] += r.standard_normal(ln) * amp * np.linspace(1, 0, ln)
    crackle = S.band(crackle, 1800, 9000)
    pops = np.zeros(n)
    for _ in range(int(r.poisson(1.5 * dur))):
        at = int(r.uniform(0, n - 4000))
        pop = S.modal(0.08, [(r.uniform(300, 700), .015, 1.0)], r, 0.001)
        pops[at:at + len(pop)] += pop * r.uniform(0.3, 0.8)
    return S.loopify(bed * 0.8 + crackle * 0.55 + pops * 0.3, 1.0)


def flare(k):
    r = S.rng('flare%d' % k)
    n = int(SR * 1.2)
    t = np.arange(n) / SR
    whoosh = S.band(S.noise(n, r), 150, 2500) * np.sin(np.pi * np.clip(t / 1.2, 0, 1)) ** 2
    return whoosh


def douse(k):
    r = S.rng('douse%d' % k)
    n = int(SR * 1.6)
    splash = S.band(S.noise(n, r), 300, 4000) * S.env_ad(n, 0.005, 0.12)
    hiss = S.highpass(S.noise(n, r), 3200) * S.env_ad(n, 0.05, 1.1, 3.0) * 0.6
    return splash + hiss


# ---------------------------------------------------------------- weather and ambience (stereo)
def stereo_noise_bed(name, dur, lo, hi, slow=7.0, depth=0.35):
    r = S.rng(name)
    n = int(SR * (dur + 1.5))
    t = np.arange(n) / SR
    ch = []
    for side in range(2):
        x = S.band(S.noise(n, r), lo, hi)
        x *= 1 - depth + depth * (0.5 + 0.5 * np.sin(2 * math.pi * t / (slow + side * 1.3) + side))
        ch.append(x)
    return np.stack(ch, axis=1)


def drops(n, r, rate, lo=2500, hi=9000):
    x = np.zeros(n)
    for _ in range(int(r.poisson(rate * n / SR))):
        at = int(r.uniform(0, n - 200))
        ln = int(SR * r.uniform(0.001, 0.004))
        x[at:at + ln] += r.standard_normal(ln) * r.uniform(0.2, 1.0) * np.linspace(1, 0, ln)
    return S.band(x, lo, hi)


def rain_loop(level):
    name = 'rain_%s' % level
    r = S.rng(name)
    dur = 12.0
    hi = {'light': 7000, 'mid': 9000, 'heavy': 11000}[level]
    bed = stereo_noise_bed(name, dur, 700, hi, 5.0, 0.15)
    n = bed.shape[0]
    rate = {'light': 25, 'mid': 70, 'heavy': 160}[level]
    d = np.stack([drops(n, r, rate), drops(n, r, rate)], axis=1)
    mixb = {'light': 0.25, 'mid': 0.5, 'heavy': 0.9}[level]
    return S.loopify(bed * mixb + d * 0.6, 1.5)


def thunder(k):
    r = S.rng('thunder%d' % k)
    dur = r.uniform(4.5, 7.0)
    n = int(SR * dur)
    t = np.arange(n) / SR
    rumble = S.lowpass(S.brown(n, r), 160) * 3.0
    envl = np.zeros(n)
    for _ in range(r.integers(2, 5)):
        c = r.uniform(0.2, dur * 0.6)
        envl += np.exp(-((t - c) / r.uniform(0.4, 1.2)) ** 2)
    envl = envl / (envl.max() + 1e-9) * np.exp(-t / (dur * 0.6))
    crack = S.lowpass(S.noise(n, r), 2200) * S.env_ad(n, 0.01, 0.35) * (0.5 if k % 2 else 0.0)
    x = rumble * envl + crack
    x = S.reverb(x, 2.5, 0.35, r, 900)
    return S.pan(x, r.uniform(-0.6, 0.6))


def gust(k):
    r = S.rng('gust%d' % k)
    dur = r.uniform(2.0, 3.5)
    n = int(SR * dur)
    t = np.arange(n) / SR
    x = S.band(S.noise(n, r), 180, 1300) * np.sin(np.pi * t / dur) ** 2
    return S.pan(x, r.uniform(-0.8, 0.8))


def wind_loop(name='wind', lo=90, hi=700):
    bed = stereo_noise_bed(name, 20.0, lo, hi, 9.0, 0.55)
    return S.loopify(bed, 1.5)


def bird_chirp(r):
    kind = r.integers(0, 3)
    if kind == 0:   # quick up-slur
        d = r.uniform(0.06, 0.12)
        x = S.glide(r.uniform(2200, 3200), r.uniform(3400, 5200), d, 0.6)
    elif kind == 1:  # trill
        d = r.uniform(0.25, 0.5)
        tt = S.t_axis(d)
        f = r.uniform(3000, 4500) + 500 * np.sin(2 * math.pi * r.uniform(18, 30) * tt)
        x = np.sin(2 * math.pi * np.cumsum(f) / SR)
    else:            # two-note whistle
        d1 = r.uniform(0.12, 0.2)
        a = S.sine(r.uniform(2400, 3000), d1)
        b = S.sine(r.uniform(1800, 2300), d1)
        x = np.concatenate([a, np.zeros(int(SR * 0.04)), b])
    return x * S.env_adsr(len(x), 0.005, 0.02, 0.8, 0.03)


def day_loop():
    r = S.rng('day_birds')
    dur = 24.0
    base = wind_loop('day_wind', 120, 800)[:int(SR * (dur + 1.5))] * 0.35
    n = base.shape[0]
    out = base.copy()
    for _ in range(int(r.poisson(1.6 * dur))):
        c = bird_chirp(r) * r.uniform(0.15, 0.6)
        at = int(r.uniform(0, n - len(c) - 1))
        p = r.uniform(-0.9, 0.9)
        out[at:at + len(c)] += S.pan(c, p)
    return S.loopify(out, 1.5)


def night_loop():
    r = S.rng('night_insects')
    dur = 20.0
    base = wind_loop('night_wind', 80, 500)[:int(SR * (dur + 1.5))] * 0.25
    n = base.shape[0]
    out = base.copy()
    for c in range(5):
        f = r.uniform(3800, 5200)
        p = r.uniform(-0.9, 0.9)
        amp = r.uniform(0.05, 0.16)
        period = r.uniform(0.55, 1.1)
        tt = 0.0
        while tt < dur:
            for k in range(3):
                pulse = S.sine(f, 0.018) * S.env_ad(int(SR * 0.018), 0.002, 0.012)
                at = int(SR * (tt + k * 0.034))
                if at + len(pulse) < n:
                    out[at:at + len(pulse)] += S.pan(pulse * amp, p)
            tt += period * r.uniform(0.85, 1.15)
    return S.loopify(out, 1.5)


def water_loop():
    r = S.rng('water_lap')
    dur = 10.0
    n = int(SR * (dur + 1))
    t = np.arange(n) / SR
    lap = np.zeros(n)
    tt = 0.0
    while tt < dur + 1:
        ln = r.uniform(0.6, 1.6)
        a = int(SR * tt)
        b = min(n, a + int(SR * ln))
        seg = np.sin(np.pi * np.linspace(0, 1, b - a)) ** 2 * r.uniform(0.4, 1.0)
        lap[a:b] += seg
        tt += ln * r.uniform(0.6, 1.0)
    x = S.band(S.noise(n, r), 150, 1400) * lap
    for _ in range(int(r.poisson(0.8 * dur))):
        at = int(r.uniform(0, n - 8000))
        plop = S.glide(r.uniform(350, 600), r.uniform(700, 1100), 0.06, 0.5) * S.env_ad(int(SR * 0.06), 0.002, 0.04)
        x[at:at + len(plop)] += plop * r.uniform(0.2, 0.5)
    return S.loopify(x, 1.0)


def cave_loop():
    r = S.rng('cave_drips')
    dur = 16.0
    n = int(SR * (dur + 1.5))
    bed = S.lowpass(S.brown(n, r), 120) * 0.25
    x = bed.copy()
    for _ in range(int(r.poisson(0.9 * dur))):
        d = 0.09
        f0 = r.uniform(700, 1100)
        drip = S.glide(f0, f0 * r.uniform(1.6, 2.2), d, 0.4) * S.env_ad(int(SR * d), 0.001, 0.05)
        at = int(r.uniform(0, n - len(drip) - 1))
        x[at:at + len(drip)] += drip * r.uniform(0.3, 0.8)
    x = S.reverb(x, 2.8, 0.5, r, 2500)
    return S.loopify(np.stack([x, np.roll(x, int(SR * 0.013))], axis=1), 1.5)


# ---------------------------------------------------------------- animals
def saw_tone(freq, dur, r, jitter=0.0):
    t = S.t_axis(dur)
    f = freq * (1 + jitter * S.lowpass(r.standard_normal(len(t)), 30) * 3)
    ph = np.cumsum(f) / SR
    return 2 * (ph % 1.0) - 1


def wolf(k):
    r = S.rng('wolf%d' % k)
    dur = r.uniform(2.4, 3.4)
    t = S.t_axis(dur)
    f0, f1, f2 = r.uniform(330, 400), r.uniform(480, 560), r.uniform(380, 440)
    f = np.where(t < dur * 0.35, f0 + (f1 - f0) * (t / (dur * 0.35)) ** 0.6, f1 + (f2 - f1) * ((t - dur * 0.35) / (dur * 0.65)))
    f *= 1 + 0.008 * np.sin(2 * math.pi * 5.5 * t)
    ph = 2 * math.pi * np.cumsum(f) / SR
    x = np.sin(ph) + 0.35 * np.sin(2 * ph) + 0.12 * np.sin(3 * ph)
    x *= S.env_adsr(len(x), 0.25, 0.3, 0.8, 0.8)
    x = S.reverb(S.lowpass(x, 2500), 2.2, 0.4, r, 2000)
    return x


def deer(k):
    r = S.rng('deer%d' % k)
    dur = r.uniform(0.28, 0.45)
    x = saw_tone(r.uniform(420, 560), dur, r, 0.01)
    x = S.formants(x, [(800, 5, 1.0), (1900, 6, 0.5), (3100, 8, 0.25)]) + S.band(S.noise(len(x), r), 1500, 5000) * 0.15
    return x * S.env_adsr(len(x), 0.01, 0.08, 0.6, 0.12)


def boar(k):
    r = S.rng('boar%d' % k)
    dur = r.uniform(0.25, 0.4)
    x = saw_tone(r.uniform(80, 110), dur, r, 0.03) * (0.6 + 0.4 * S.band(r.standard_normal(int(SR * dur)), 20, 80) * 4)
    x = S.formants(x, [(380, 4, 1.0), (900, 5, 0.5)])
    return x * S.env_adsr(len(x), 0.02, 0.05, 0.7, 0.1)


def fox(k):
    r = S.rng('fox%d' % k)
    d = r.uniform(0.18, 0.28)
    t = S.t_axis(d)
    f = r.uniform(800, 950) + r.uniform(300, 500) * np.sin(np.pi * t / d)
    ph = 2 * math.pi * np.cumsum(f) / SR
    x = np.sin(ph) + 0.4 * np.sin(2 * ph) + 0.2 * np.sin(3 * ph)
    x += S.band(S.noise(len(x), r), 1500, 4000) * 0.2
    x *= S.env_adsr(len(x), 0.01, 0.03, 0.7, 0.06)
    if k % 2:
        x = np.concatenate([x, np.zeros(int(SR * 0.12)), x * 0.8])
    return x


def crow(k):
    r = S.rng('crow%d' % k)
    caws = []
    for i in range(1 + k % 3):
        d = r.uniform(0.22, 0.32)
        x = saw_tone(r.uniform(480, 620), d, r, 0.02)
        x = x * (0.6 + 0.4 * np.sign(np.sin(2 * math.pi * 55 * S.t_axis(d))))
        x = S.formants(x, [(1200, 4, 1.0), (2500, 5, 0.6), (3600, 6, 0.3)])
        caws.append(x * S.env_adsr(len(x), 0.01, 0.06, 0.7, 0.08))
        caws.append(np.zeros(int(SR * r.uniform(0.12, 0.22))))
    return np.concatenate(caws)


def duck(k):
    r = S.rng('duck%d' % k)
    qs = []
    for i in range(2 + k % 2):
        d = r.uniform(0.14, 0.2)
        x = saw_tone(r.uniform(260, 320), d, r, 0.02)
        x = S.formants(x, [(1050, 7, 1.0), (2400, 9, 0.7)])
        qs.append(x * S.env_adsr(len(x), 0.005, 0.04, 0.7, 0.05))
        qs.append(np.zeros(int(SR * r.uniform(0.06, 0.12))))
    return np.concatenate(qs)


def bear(k):
    r = S.rng('bear%d' % k)
    dur = r.uniform(1.0, 1.5)
    x = saw_tone(r.uniform(65, 85), dur, r, 0.05)
    rough = 0.5 + 0.5 * S.lowpass(r.standard_normal(len(x)), 40) * 6
    x = S.formants(x * rough, [(320, 3, 1.0), (650, 4, 0.6), (1100, 5, 0.2)])
    return x * S.env_adsr(len(x), 0.15, 0.2, 0.8, 0.4)


def hare(k):
    r = S.rng('hare%d' % k)
    n = int(SR * 0.25)
    x = S.band(S.noise(n, r), 300, 2000) * S.env_ad(n, 0.002, 0.03)
    x = x + np.roll(x, int(SR * 0.09)) * 0.7
    return x


# ---------------------------------------------------------------- interface
def ui_click():
    r = S.rng('ui_click')
    return S.modal(0.12, [(1800, .02, 1.0), (3200, .012, .4)], r, 0.0008)


def ui_chime():
    r = S.rng('ui_chime')
    a = S.modal(1.4, [(1318.5, .5, 1.0), (2637, .25, .3), (3955, .12, .15)], r, 0)
    b = S.modal(1.2, [(1975.5, .45, .8), (3951, .2, .25)], r, 0)
    b = np.concatenate([np.zeros(int(SR * 0.12)), b])
    b = np.pad(b, (0, max(0, len(a) - len(b))))[:len(a)]
    return S.reverb(a + b, 1.2, 0.25)


def ui_page():
    r = S.rng('ui_page')
    n = int(SR * 0.25)
    t = np.arange(n) / SR
    return S.band(S.noise(n, r), 1500, 8000) * np.sin(np.pi * t / 0.25) ** 2


def main():
    os.makedirs(OUT, exist_ok=True)
    for k in range(4):
        emit('chop_%d' % k, chop(k), 'SFX')
        emit('mine_%d' % k, mine(k), 'SFX')
        emit('hammer_%d' % k, hammer(k), 'SFX')
        emit('hoe_%d' % k, hoe(k), 'SFX', rms=0.09)
    for k in range(3):
        emit('cook_%d' % k, cook(k), 'SFX', rms=0.07)
    for k in range(3):
        emit('saw_%d' % k, saw(k), 'SFX', rms=0.08)
    for k in range(2):
        emit('door_%d' % k, door(k), 'SFX', rms=0.08)
        emit('flare_%d' % k, flare(k), 'SFX', rms=0.1)
        emit('douse_%d' % k, douse(k), 'SFX', rms=0.1)
    emit('fire_loop', fire_loop(), 'SFX', loop=True, rms=0.1)
    for lv in ('light', 'mid', 'heavy'):
        emit('rain_%s' % lv, rain_loop(lv), 'Ambience', loop=True, rms={'light': 0.05, 'mid': 0.08, 'heavy': 0.12}[lv])
    for k in range(4):
        emit('thunder_%d' % k, thunder(k), 'Ambience', rms=0.14)
    for k in range(3):
        emit('gust_%d' % k, gust(k), 'Ambience', rms=0.07)
    emit('wind_loop', wind_loop(), 'Ambience', loop=True, rms=0.06)
    emit('day_loop', day_loop(), 'Ambience', loop=True, rms=0.07)
    emit('night_loop', night_loop(), 'Ambience', loop=True, rms=0.05)
    emit('water_loop', water_loop(), 'Ambience', loop=True, rms=0.08)
    emit('cave_loop', cave_loop(), 'Ambience', loop=True, rms=0.06)
    for k in range(2):
        emit('wolf_%d' % k, wolf(k), 'SFX', rms=0.1)
        emit('deer_%d' % k, deer(k), 'SFX', rms=0.08)
        emit('boar_%d' % k, boar(k), 'SFX', rms=0.09)
        emit('fox_%d' % k, fox(k), 'SFX', rms=0.08)
        emit('duck_%d' % k, duck(k), 'SFX', rms=0.08)
        emit('bear_%d' % k, bear(k), 'SFX', rms=0.11)
        emit('hare_%d' % k, hare(k), 'SFX', rms=0.05)
    for k in range(3):
        emit('crow_%d' % k, crow(k), 'SFX', rms=0.08)
    emit('ui_click', ui_click(), 'UI', rms=0.08)
    emit('ui_chime', ui_chime(), 'UI', rms=0.08)
    emit('ui_page', ui_page(), 'UI', rms=0.05)
    with open(os.path.join(OUT, 'sounds.json'), 'w') as f:
        json.dump(MANIFEST, f, indent=1, sort_keys=True)
    rows = []
    for base in sorted(MANIFEST):
        for fn in MANIFEST[base]['files']:
            rows.append('[[asset]]\npath = "godot/assets/audio/%s"\nkind = "audio"\nsource = "generated"\n'
                        'generator = "tools/audio/sfx.py"\nlicence = "project-original"\n' % fn)
    with open(os.path.join(ROOT, 'tools', 'audio', 'provenance_audio.toml'), 'w') as f:
        f.write('# Written by tools/audio/sfx.py; every sound is synthesised there.\n\n' + '\n'.join(rows))
    print('audio: %d sounds, %d files -> %s' % (len(MANIFEST), sum(len(m['files']) for m in MANIFEST.values()), OUT))


if __name__ == '__main__':
    main()
