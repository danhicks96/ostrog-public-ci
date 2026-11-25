"""Ostrog's sound kit: small, deterministic synthesis building blocks in numpy alone (no
scipy on the build server). Everything is project-original and seeded by name, so a rebuild gives the
same files. Sample rate 44.1 kHz; mono arrays of float64 in -1..1 unless stated.

Filtering is done in the frequency domain (rfft, multiply, irfft), which is exact for whole
clips and fast. Plucked strings use Karplus-Strong processed one delay-line block at a time so
it vectorises. Struck objects are modal: a sum of damped sinusoids.
"""
import hashlib
import math
import os
import subprocess
import wave

import numpy as np

SR = 44100


def rng(name):
    seed = int(hashlib.sha256(name.encode()).hexdigest()[:12], 16)
    return np.random.default_rng(seed)


def t_axis(seconds):
    return np.arange(int(SR * seconds)) / SR


# ---------------------------------------------------------------- filters
def band(x, lo=None, hi=None, slope=2.0):
    """Smooth band-pass by FFT: a raised-power response, `slope` sets the steepness."""
    n = len(x)
    f = np.fft.rfftfreq(n, 1.0 / SR)
    X = np.fft.rfft(x)
    g = np.ones_like(f)
    if lo:
        g *= 1.0 / (1.0 + (lo / np.maximum(f, 1e-3)) ** (2 * slope))
    if hi:
        g *= 1.0 / (1.0 + (f / hi) ** (2 * slope))
    return np.fft.irfft(X * g, n)


def lowpass(x, hi, slope=2.0):
    return band(x, None, hi, slope)


def highpass(x, lo, slope=2.0):
    return band(x, lo, None, slope)


def resonant(x, centre, q=8.0):
    """A resonant peak (formant) by FFT."""
    n = len(x)
    f = np.fft.rfftfreq(n, 1.0 / SR)
    X = np.fft.rfft(x)
    bw = centre / q
    g = 1.0 / (1.0 + ((f - centre) / bw) ** 2)
    return np.fft.irfft(X * g, n)


def formants(x, peaks):
    """Sum of resonant peaks [(centre, q, gain)], like a vocal tract."""
    out = np.zeros_like(x)
    for (c, q, gain) in peaks:
        out += resonant(x, c, q) * gain
    return out


# ---------------------------------------------------------------- envelopes and sources
def env_ad(n, attack, decay, curve=4.0):
    """Attack (seconds, linear) then exponential decay to silence over `decay` seconds."""
    a = max(1, int(SR * attack))
    e = np.ones(n)
    e[:a] = np.linspace(0, 1, a)
    t = np.arange(n - a) / SR
    e[a:] = np.exp(-curve * t / max(decay, 1e-4))
    return e


def env_adsr(n, a, d, s, r):
    e = np.zeros(n)
    ia, idd, ir = int(SR * a), int(SR * d), int(SR * r)
    isus = max(0, n - ia - idd - ir)
    i = 0
    e[i:i + ia] = np.linspace(0, 1, ia, endpoint=False); i += ia
    e[i:i + idd] = np.linspace(1, s, idd, endpoint=False); i += idd
    e[i:i + isus] = s; i += isus
    e[i:i + ir] = np.linspace(s, 0, max(1, n - i))[:max(0, n - i)][:ir]
    return e


def noise(n, r):
    return r.standard_normal(n)


def brown(n, r):
    w = r.standard_normal(n)
    b = highpass(np.cumsum(w), 18.0, 1.0)  # integrate, then remove the drift
    return b / (np.max(np.abs(b)) + 1e-9)


def modal(seconds, modes, r=None, strike=0.002):
    """Struck object: modes [(freq, decay_seconds, amp)] excited by a short noise click."""
    n = int(SR * seconds)
    t = np.arange(n) / SR
    out = np.zeros(n)
    for (f, d, a) in modes:
        ph = r.random() * 6.283 if r is not None else 0.0
        out += a * np.sin(2 * math.pi * f * t + ph) * np.exp(-t / d)
    if r is not None and strike:
        k = int(SR * strike)
        click = r.standard_normal(k) * np.linspace(1, 0, k)
        out[:k] += click * 0.4
    return out


def pluck(freq, seconds, r, bright=0.5, decay=0.996):
    """Karplus-Strong string, vectorised by delay-line block. `bright` 0..1 shapes the pick
    noise; `decay` is the per-period loss."""
    n = int(SR * seconds)
    L = max(2, int(round(SR / freq)))
    buf = r.uniform(-1, 1, L)
    if bright < 1.0:
        buf = lowpass(np.concatenate([buf, buf]), 800 + 7000 * bright)[:L]
    out = np.zeros(n + L + 1)
    out[:L] = buf
    i = L
    while i < n + 1:
        j = min(i + L, n + 1)
        prev = out[i - L:j - L]
        prev1 = out[i - L - 1:j - L - 1] if i - L - 1 >= 0 else np.concatenate([[0.0], out[i - L:j - L - 1]])
        out[i:j] = decay * 0.5 * (prev + prev1)
        i = j
    return out[:n]


def sine(freq, seconds, phase=0.0):
    return np.sin(2 * math.pi * freq * t_axis(seconds) + phase)


def glide(f0, f1, seconds, curve=1.0):
    t = t_axis(seconds)
    k = (t / max(seconds, 1e-6)) ** curve
    f = f0 + (f1 - f0) * k
    return np.sin(2 * math.pi * np.cumsum(f) / SR)


def vibrato_tone(freq, seconds, rate=5.0, depth=0.006, harmonics=((1, 1.0),)):
    t = t_axis(seconds)
    f = freq * (1 + depth * np.sin(2 * math.pi * rate * t))
    ph = 2 * math.pi * np.cumsum(f) / SR
    return sum(a * np.sin(h * ph) for (h, a) in harmonics)


# ---------------------------------------------------------------- instruments (music.py)
def midi_hz(m):
    return 440.0 * 2 ** ((m - 69) / 12.0)


def saw_wave(freq, seconds, r, drift=0.0, rate=0.3):
    """Naive sawtooth with an optional slow random pitch drift (fraction of freq). Aliasing is
    small below ~500 Hz and the callers low-pass it anyway."""
    n = int(SR * seconds)
    f = np.full(n, float(freq))
    if drift:
        k = max(4, int(seconds * rate * 4) + 4)
        pts = r.uniform(-1, 1, k)
        f *= 1 + drift * np.interp(np.linspace(0, k - 1, n), np.arange(k), pts)
    ph = np.cumsum(f) / SR + r.random()
    return 2 * (ph % 1.0) - 1


def lute(freq, seconds, r, bright=0.55):
    """Plucked gut string through a small wooden body (three soft resonances), damped by the
    hand at the end of the note."""
    x = pluck(freq, seconds + 0.35, r, bright, 0.995 if freq < 300 else 0.993)
    body = formants(x, [(260, 3, 1.0), (520, 4, 0.6), (1400, 5, 0.3)])
    x = 0.65 * x + 0.5 * body
    n = len(x)
    e = np.ones(n)
    k = int(SR * 0.35)
    e[-k:] = np.linspace(1, 0, k) ** 2
    return x * e


def harp(freq, seconds, r):
    """Brighter, longer-ringing pluck with a faint octave shimmer; rings past the note."""
    ring = max(seconds, 1.6)
    x = pluck(freq, ring, r, 0.35, 0.998)
    x += 0.15 * pluck(freq * 2.0, ring, r, 0.25, 0.996)
    x *= env_ad(len(x), 0.002, ring * 1.4, 2.5)
    return x


def flute(freq, seconds, r, breath=0.09):
    """Breathy flute: a few sine harmonics with delayed vibrato, plus noise band-passed around
    the pitch and a short breath chiff at the onset."""
    dur = seconds + 0.15
    n = int(SR * dur)
    t = np.arange(n) / SR
    vib = np.clip((t - 0.25) / 0.4, 0, 1) * 0.0045
    f = freq * (1 + vib * np.sin(2 * math.pi * 5.1 * t + r.random() * 6.28))
    ph = 2 * math.pi * np.cumsum(f) / SR
    tone = np.sin(ph) + 0.22 * np.sin(2 * ph) + 0.07 * np.sin(3 * ph) + 0.025 * np.sin(4 * ph)
    air = band(noise(n, r), freq * 0.8, freq * 3.5, 1.5)
    air /= np.sqrt(np.mean(air ** 2)) + 1e-9
    chiff = env_ad(n, 0.004, 0.06)
    x = tone + air * (breath + 0.25 * chiff)
    return x * env_adsr(n, 0.07, 0.12, 0.85, 0.15)


BELL = [(0.5, 7.0, 0.35), (1.0, 5.0, 1.0), (1.19, 3.5, 0.32), (1.5, 3.0, 0.25),
        (2.0, 2.4, 0.5), (2.74, 1.4, 0.22), (3.01, 1.1, 0.15), (4.16, 0.7, 0.08)]


def bell(freq, r, seconds=6.0, soft=True):
    """Modal bell: the classic hum, prime, tierce, quint, nominal series with longer decay on
    the low partials. `soft` strikes with a padded mallet (no bright click)."""
    modes = [(freq * k, d, a) for (k, d, a) in BELL if freq * k < 12000]
    x = modal(seconds, modes, r, 0.0 if soft else 0.002)
    a = int(SR * 0.004)
    x[:a] *= np.linspace(0, 1, a)
    return x


MEMBRANE = [1.0, 1.59, 2.14, 2.30, 2.65, 2.92, 3.16]


def frame_drum(kind, r, f0=88.0):
    """Frame drum by modal synthesis. kind 'dum' (centre: low modes, long) or 'tek' (rim: high
    modes, short, with a slap of noise)."""
    n = int(SR * 0.9)
    if kind == 'dum':
        modes = [(f0 * k, 0.32 / (1 + i * 0.6), 1.0 / (1 + i)) for i, k in enumerate(MEMBRANE)]
        x = modal(0.9, modes, r, 0.0)
        x += lowpass(noise(n, r), 400) * env_ad(n, 0.001, 0.03) * 0.6
    else:
        modes = [(f0 * k * 2.3, 0.07, 0.6 / (1 + i * 0.5)) for i, k in enumerate(MEMBRANE)]
        x = modal(0.9, modes, r, 0.0) * 0.6
        x += band(noise(n, r), 1200, 5000) * env_ad(n, 0.0005, 0.025)
    a = int(SR * 0.0015)
    x[:a] *= np.linspace(0, 1, a)
    return x


def bowed(freq, seconds, r, attack=1.8, release=2.2):
    """Bowed drone: two slightly detuned drifting saws, low-passed, with rosin noise riding on
    a slow swell."""
    dur = seconds + release
    n = int(SR * dur)
    x = saw_wave(freq * 1.0012, dur, r, 0.0015) + saw_wave(freq * 0.9988, dur, r, 0.0015)
    x = lowpass(x, min(5 * freq, 3000), 1.5)
    rosin = band(noise(n, r), 1500, 5000) * 0.02
    t = np.arange(n) / SR
    swell = 0.85 + 0.15 * np.sin(2 * math.pi * t / (seconds * 0.7 + 3) + r.random() * 6.28)
    return (x + rosin) * swell * env_adsr(n, attack, 0.5, 0.9, release)


VOWELS = {'ah': [(700, 5, 1.0), (1220, 6, 0.5), (2600, 8, 0.25)],
          'oh': [(450, 5, 1.0), (800, 6, 0.6), (2830, 8, 0.15)],
          'oo': [(300, 4, 1.0), (870, 6, 0.35), (2240, 8, 0.1)]}


def pad_chord(freqs, seconds, r, vowel='ah', attack=1.6, release=2.0):
    """Formant pad: three detuned saws per chord tone, shaped by a vowel's formants, slow in and
    slow out."""
    dur = seconds + release
    n = int(SR * dur)
    x = np.zeros(n)
    for f in freqs:
        for d in (0.996, 1.0, 1.004):
            x += saw_wave(f * d, dur, r, 0.001)
    x = formants(lowpass(x, 4000), VOWELS[vowel]) + 0.15 * lowpass(x, 1200)
    return x * env_adsr(n, attack, 0.6, 0.85, release)


# ---------------------------------------------------------------- space
def impulse(seconds, r, damp=4000.0):
    """A generated room: exponentially decaying, darkening noise with a 12 ms pre-delay, unit energy."""
    n = int(SR * seconds)
    ir = r.standard_normal(n) * np.exp(-np.arange(n) / SR * (6.9 / seconds))
    ir = lowpass(ir, damp)
    ir[:int(SR * 0.012)] = 0.0
    return ir / (np.sqrt(np.sum(ir ** 2)) + 1e-9)


def reverb_loop(x, seconds, mix, r, damp=4000.0):
    """Reverb for a loop: circular convolution over the clip's own length, so the tail of the end
    rings into the start and the seam stays continuous. Stereo input gets a different room per
    side."""
    n = len(x)
    if x.ndim == 1:
        ir = impulse(seconds, r, damp)
        wet = np.fft.irfft(np.fft.rfft(x) * np.fft.rfft(ir, n), n)
        return (1 - mix) * x + mix * wet
    out = np.empty_like(x)
    for c in range(x.shape[1]):
        out[:, c] = reverb_loop(x[:, c], seconds, mix, r, damp)
    return out


def reverb(x, seconds=1.4, mix=0.25, r=None, damp=4000.0):
    """FFT convolution with a generated impulse: exponentially decaying, darkening noise."""
    r = r or rng('reverb')
    n = int(SR * seconds)
    ir = r.standard_normal(n) * np.exp(-np.arange(n) / SR * (6.9 / seconds))
    ir = lowpass(ir, damp)
    ir[:int(SR * 0.012)] = 0.0  # pre-delay
    ir /= np.sqrt(np.sum(ir ** 2)) + 1e-9
    m = len(x) + n
    wet = np.fft.irfft(np.fft.rfft(x, m) * np.fft.rfft(ir, m), m)[:len(x)]
    return (1 - mix) * x + mix * wet


def pan(x, p):
    """Mono to stereo, p in -1..1 (equal power)."""
    a = (p + 1) * math.pi / 4
    return np.stack([x * math.cos(a), x * math.sin(a)], axis=1)


# ---------------------------------------------------------------- output
def normalise(x, rms_target=0.12, peak=0.89):
    rms = math.sqrt(float(np.mean(x ** 2))) + 1e-9
    y = x * (rms_target / rms)
    p = float(np.max(np.abs(y)))
    if p > peak:
        y = y * (peak / p)
    return y


def fade(x, a=0.004, b=0.01):
    n = len(x)
    ia, ib = int(SR * a), int(SR * b)
    e = np.ones(n)
    if ia:
        e[:ia] = np.linspace(0, 1, ia)
    if ib:
        e[-ib:] = np.linspace(1, 0, ib)
    return x * (e if x.ndim == 1 else e[:, None])


def loopify(x, xfade=1.0):
    """Make a clip loop seamlessly: crossfade its tail into its head and drop the tail."""
    k = int(SR * xfade)
    head = x[:k]
    tail = x[-k:]
    ramp = np.linspace(0, 1, k)
    if x.ndim == 2:
        ramp = ramp[:, None]
    out = x[:-k].copy()
    out[:k] = head * ramp + tail * (1 - ramp)
    return out


def write_ogg(path, x, quality=5):
    """Write float audio (mono or stereo) as OGG Vorbis via ffmpeg; returns the path."""
    x = np.clip(x, -1.0, 1.0)
    ch = 1 if x.ndim == 1 else x.shape[1]
    pcm = (x * 32767).astype('<i2')
    tmp = path + '.wav'
    with wave.open(tmp, 'wb') as w:
        w.setnchannels(ch)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes(pcm.tobytes())
    # bitexact: a fixed Ogg serial number and no encoder tag, so a rebuild is byte-identical
    subprocess.run(['ffmpeg', '-y', '-loglevel', 'error', '-i', tmp, '-c:a', 'libvorbis',
                    '-q:a', str(quality), '-fflags', '+bitexact', '-flags:a', '+bitexact', path],
                   check=True)
    os.remove(tmp)
    return path
