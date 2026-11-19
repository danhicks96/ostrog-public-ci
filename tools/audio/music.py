#!/usr/bin/env python3
"""Ostrog's music: folk by day, ambient by night. Writes godot/assets/audio/music/<cue>_<stem>.ogg,
the manifest godot/assets/audio/music.json that the client's music director reads, and the
provenance rows tools/audio/provenance_music.toml.

    python3 tools/audio/music.py            # compose and render every cue
    python3 tools/audio/music.py spring_day # one cue (manifest and provenance still list all)

Every cue is 32 bars in a mode chosen by season (spring Mixolydian or Lydian, summer Ionian,
autumn Dorian, winter sparse Aeolian) and is split into four stems of exactly equal length that
loop seamlessly and play together: drone, pad, melody, and tension (frame drum plus a second
melody), which the director raises with the community's strain.

Composition is rule-based and seeded by the cue's name, so a rebuild writes the same notes:
- Harmony: four-chord phrases from per-mode tables. An antecedent ends away from the tonic (a
  half cadence), its consequent ends on the tonic, so bar 32 resolves into bar 1 when it loops.
- Form: day cues are A A' B A'' (eight 4-bar phrases, one chord a bar); night cues are A B A'
  with one chord every two bars.
- Melody: a one-chord motif (a rhythm and a contour of scale steps) is stated, sequenced a step
  up or down, contrasted (inverted or re-shaped towards the phrase peak), then a cadence figure
  approaches its target by step. Strong beats take chord tones; a leap is recovered by a step
  the other way; the line stays inside an octave and a half. Later sections repeat the motif
  with ornaments (neighbour and passing notes); B inverts it in a higher register. Winter drops
  and ties weak notes so the line is sparse.
- Second melody (tension stem): a counter-line in contrary motion that fills the beats where the
  melody holds (day), or a low ostinato on the chord (night).

Synthesis is tools/audio/synth.py: Karplus-Strong lute and harp, a breathy flute, modal bells
and frame drum, a bowed drone and a formant pad, with circular-convolution reverb so the room's
tail at bar 32 rings into bar 1. tools/audio/score_report.py prints the notes per bar.
"""
import json
import math
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import synth as S  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
OUT = os.path.join(ROOT, 'godot', 'assets', 'audio', 'music')
MANIFEST = os.path.join(ROOT, 'godot', 'assets', 'audio', 'music.json')
PROVENANCE = os.path.join(ROOT, 'tools', 'audio', 'provenance_music.toml')
SR = S.SR
BARS = 32
PHRASE_BARS = 4
STEMS = ('drone', 'pad', 'melody', 'tension')
QUALITY = 0  # Vorbis -q:a; about 64 kb/s stereo nominal. Raise it for fidelity at ~1.3x size per step

MODES = {
    'ionian': (0, 2, 4, 5, 7, 9, 11),
    'lydian': (0, 2, 4, 6, 7, 9, 11),
    'mixolydian': (0, 2, 4, 5, 7, 9, 10),
    'dorian': (0, 2, 3, 5, 7, 9, 10),
    'aeolian': (0, 2, 3, 5, 7, 8, 10),
}
NAMES = ('C', 'C#', 'D', 'Eb', 'E', 'F', 'F#', 'G', 'G#', 'A', 'Bb', 'B')
FLATS = ('C', 'Db', 'D', 'Eb', 'E', 'F', 'Gb', 'G', 'Ab', 'A', 'Bb', 'B')
PC = {n: i for i, n in enumerate(NAMES)}

# Four-chord phrases as scale-degree roots (0 = tonic). Diminished triads never appear.
# 'ante' ends away from the tonic, 'cons' ends on it, 'b' is the contrasting section's opening.
PROGS = {
    'ionian': {'ante': [[0, 3, 4, 4], [0, 5, 3, 4], [0, 3, 0, 4]],
               'cons': [[0, 3, 4, 0], [5, 3, 4, 0], [0, 5, 4, 0]],
               'b': [[3, 4, 5, 4], [5, 3, 1, 4], [3, 0, 3, 4]]},
    'mixolydian': {'ante': [[0, 6, 3, 4], [0, 6, 0, 4], [0, 3, 6, 6]],
                   'cons': [[0, 6, 3, 0], [0, 4, 6, 0], [3, 6, 0, 0]],
                   'b': [[6, 3, 6, 4], [3, 6, 3, 4], [6, 5, 3, 4]]},
    'lydian': {'ante': [[0, 1, 0, 1], [0, 1, 4, 4], [0, 2, 1, 4]],
               'cons': [[0, 1, 4, 0], [0, 2, 1, 0], [5, 1, 4, 0]],
               'b': [[1, 4, 2, 1], [5, 1, 2, 4], [2, 1, 5, 1]]},
    'dorian': {'ante': [[0, 3, 0, 4], [0, 3, 6, 4], [0, 6, 3, 3]],
               'cons': [[0, 3, 6, 0], [0, 6, 3, 0], [2, 3, 6, 0]],
               'b': [[3, 6, 2, 4], [6, 3, 6, 4], [2, 6, 3, 4]]},
    'aeolian': {'ante': [[0, 5, 6, 4], [0, 3, 4, 4], [0, 5, 3, 4]],
                'cons': [[0, 5, 6, 0], [0, 3, 6, 0], [5, 6, 0, 0]],
                'b': [[5, 2, 6, 4], [3, 5, 6, 4], [2, 6, 3, 4]]},
}

# Rhythms for one chord's span (in beats; negative = rest) and for the cadence figure.
RHYTHMS = {
    3: [[1, 1, 1], [2, 1], [1.5, 0.5, 1], [1, 0.5, 0.5, 1], [0.5, 0.5, 1, 1], [1, 2]],
    4: [[1, 1, 2], [1.5, 0.5, 1, 1], [1, 1, 1, 1], [2, 1, 1], [0.5, 0.5, 1, 2], [1, 0.5, 0.5, 2]],
    8: [[2, 2, 4], [3, 1, 4], [4, 4], [2, 6], [1, 1, 2, 4], [-2, 2, 4], [-4, 1, 3], [2, -2, 4]],
}
CADENCE = {3: [[1, 2], [0.5, 0.5, 2], [3]], 4: [[1, 3], [2, 2], [1, 1, 2]],
           8: [[2, 6], [-2, 2, 4], [4, 4], [-4, 4]]}
STRONG = {3: {0}, 4: {0, 2}, 8: {0, 4}}

# Cue table. time: day, night or deep (underground). lead: melody instrument; accomp: the day
# pad stem's plucked accompaniment; counter: the tension stem's second melody.
CUES = [
    dict(name='spring_day', season=0, time='day', tonic='G', mode='mixolydian', bpm=76, meter=3,
         lead='flute', accomp='lute', counter='harp', vowel='oh'),
    dict(name='summer_day', season=1, time='day', tonic='D', mode='ionian', bpm=80, meter=3,
         lead='lute', accomp='harp', counter='flute', vowel='ah'),
    dict(name='autumn_day', season=2, time='day', tonic='A', mode='dorian', bpm=72, meter=4,
         lead='flute', accomp='lute', counter='harp', vowel='oh'),
    dict(name='winter_day', season=3, time='day', tonic='E', mode='aeolian', bpm=66, meter=3,
         lead='harp', accomp='harp', counter='flute', vowel='oo', sparse=True),
    dict(name='spring_night', season=0, time='night', tonic='D', mode='lydian', bpm=64, meter=4,
         lead='bell', counter='harp', vowel='ah'),
    dict(name='summer_night', season=1, time='night', tonic='A', mode='ionian', bpm=62, meter=4,
         lead='bell', counter='harp', vowel='ah'),
    dict(name='autumn_night', season=2, time='night', tonic='E', mode='dorian', bpm=60, meter=4,
         lead='bell', counter='harp', vowel='oh'),
    dict(name='winter_night', season=3, time='night', tonic='B', mode='aeolian', bpm=60, meter=4,
         lead='bell', counter='harp', vowel='oo', sparse=True),
    dict(name='deep', season=-1, time='deep', tonic='D', mode='aeolian', bpm=60, meter=4,
         lead='bell', counter='bowed', vowel='oo', sparse=True, octave=0),
]

OCTAVE = {'flute': 12, 'bell': 12, 'lute': 0, 'harp': 0, 'bowed': -12}


# ---------------------------------------------------------------- music theory
class Scale:
    def __init__(self, tonic, mode):
        pc = PC[tonic]
        self.tonic = 60 + pc if pc <= 6 else 48 + pc  # melody tonic between G3 and F#4
        self.iv = MODES[mode]
        self.mode = mode
        pcs = {(pc + i) % 12 for i in self.iv}
        self.flats = not pcs & {1, 6}  # spell with flats only in keys with no C# or F#

    def midi(self, deg):
        o, k = divmod(int(deg), 7)
        return self.tonic + 12 * o + self.iv[k]

    def numeral(self, root):
        third = (self.iv[(root + 2) % 7] - self.iv[root]) % 12
        major = (0, 2, 4, 5, 7, 9, 11)
        acc = 'b' if self.iv[root] < major[root] else ('#' if self.iv[root] > major[root] else '')
        num = ('I', 'II', 'III', 'IV', 'V', 'VI', 'VII')[root]
        return acc + (num if third == 4 else num.lower())


def name(midi, flats=False):
    return '%s%d' % ((FLATS if flats else NAMES)[midi % 12], midi // 12 - 1)


def chord_tones(root):
    return {root % 7, (root + 2) % 7, (root + 4) % 7}


def nearest_tone(deg, root, lo=-99, hi=99, exclude=()):
    """The chord tone (as a degree) nearest `deg` inside [lo, hi], skipping pitch classes in
    `exclude`; ties go downward."""
    tones = chord_tones(root) - set(exclude)
    best = None
    for d in range(lo, hi + 1):
        if d % 7 in tones and (best is None or abs(d - deg) < abs(best - deg)):
            best = d
    return deg if best is None else best


def pick(r, xs):
    return xs[int(r.integers(len(xs)))]


def contour(r, k, shape):
    """k scale-step intervals following a shape: rise, fall, arch or wave."""
    out = []
    for i in range(k):
        step = pick(r, [1, 1, 1, 1, 2, 2, 0, 3])
        if shape == 'rise':
            sgn = 1
        elif shape == 'fall':
            sgn = -1
        elif shape == 'arch':
            sgn = 1 if i < (k + 1) // 2 else -1
        else:
            sgn = 1 if i % 2 == 0 else -1
        if r.random() < 0.2:
            sgn = -sgn
        out.append(sgn * step)
    return out


class Motif:
    def __init__(self, rhythm, intervals):
        self.rhythm = list(rhythm)
        self.intervals = list(intervals)

    def inverted(self):
        return Motif(self.rhythm, [-i for i in self.intervals])


def realise(motif, root, hint, unit, lo, hi, start=None):
    """Place a motif over one chord: first note is the chord tone nearest `hint`, then the
    contour; strong beats snap to chord tones; a leap of a fourth or more is recovered by a step
    back; the range is folded inside [lo, hi]. Returns [(beat, dur, degree)] (rests omitted)."""
    notes = []
    pos = 0.0
    deg = None
    prev_iv = 0
    k = 0
    for d in motif.rhythm:
        if d < 0:
            pos += -d
            continue
        if deg is None:
            deg = nearest_tone(hint, root, lo, hi) if start is None else start
        else:
            iv = motif.intervals[k % len(motif.intervals)] if motif.intervals else 0
            k += 1
            if abs(prev_iv) >= 3:
                iv = -int(math.copysign(1, prev_iv))
            deg += iv
            prev_iv = iv
        if deg > hi:
            deg = hi - (deg - hi)
        if deg < lo:
            deg = lo + (lo - deg)
        if pos in STRONG[unit] or d >= 1.5:  # strong beats and held notes are chord tones
            deg = nearest_tone(deg, root, lo, hi)
        notes.append([pos, d, deg])
        pos += d
    return notes


def cadence(r, root, last, unit, final, lo, hi):
    """Cadence figure over the phrase's last chord: approach the target by step. Final (tonic)
    cadences land on the tonic degree; half cadences on a chord tone that is not the tonic."""
    rhythm = pick(r, CADENCE[unit])
    if final:
        target = min((d for d in range(lo, hi + 1) if d % 7 == 0), key=lambda d: (abs(d - last), d))
    else:
        target = nearest_tone(last, root, lo, hi, exclude=(0,))
    sounding = [d for d in rhythm if d > 0]
    above = last >= target
    degs = [target + (i if above else -i) for i in range(len(sounding) - 1, -1, -1)]
    out = []
    pos = 0.0
    j = 0
    for d in rhythm:
        if d < 0:
            pos += -d
            continue
        g = degs[j]
        if j < len(degs) - 1 and d >= 1.5:  # a held approach note is a chord tone, not a step
            g = nearest_tone(g, root, lo, hi, exclude=((target % 7),))
        out.append([pos, d, g])
        j += 1
        pos += d
    return out


def ornament(r, notes, amount):
    """Neighbour and passing-note ornaments on notes of a beat or more (variation for repeats)."""
    out = []
    for i, (p, d, g) in enumerate(notes):
        nxt = notes[i + 1][2] if i + 1 < len(notes) else None
        if d >= 1 and r.random() < amount:
            if nxt is not None and abs(nxt - g) == 2:
                out += [[p, d / 2, g], [p + d / 2, d / 2, (g + nxt) // 2]]  # passing note
            else:
                out += [[p, d / 2, g], [p + d / 2, d / 2, g + pick(r, [1, -1])]]  # neighbour
        else:
            out.append([p, d, g])
    return out


def phrase(r, motif, contrast, roots, kind, hint, unit, lo, hi, orn=0.0, seq=None):
    """Four chord-spans: motif, motif sequenced, contrast, cadence. Returns notes with beats
    relative to the phrase start and the last degree."""
    seq = seq if seq is not None else pick(r, [1, -1, 0, 2])
    units = []
    a = realise(motif, roots[0], hint, unit, lo, hi)
    units.append(a)
    last = a[-1][2] if a else hint
    # Sequence: restate the motif from the next chord tone in the direction of `seq`.
    first = a[0][2] if a else hint
    tones = [t for t in range(lo, hi + 1) if t % 7 in chord_tones(roots[1])]
    if seq > 0:
        go = [t for t in tones if t >= first + seq]
        st = go[0] if go else tones[-1]
    elif seq < 0:
        go = [t for t in tones if t <= first + seq]
        st = go[-1] if go else tones[0]
    else:
        st = nearest_tone(first, roots[1], lo, hi)
    b = realise(motif, roots[1], first, unit, lo, hi, start=st)
    units.append(b)
    last = b[-1][2] if b else last
    c = realise(contrast, roots[2], last + (1 if kind == 'ante' else 0), unit, lo, hi)
    units.append(c)
    last = c[-1][2] if c else last
    d = cadence(r, roots[3], last, unit, kind != 'ante', lo, hi)
    out = []
    for i, u in enumerate(units):
        if orn:
            u = ornament(r, u, orn)
        out += [[p + i * unit, dd, g] for (p, dd, g) in u]
    out += [[p + 3 * unit, dd, g, 'cad'] for (p, dd, g) in d]
    return out, d[-1][2]


def thin(r, notes, unit):
    """Winter: drop weak, non-cadence notes; a dropped note is tied into the one before or
    left as a rest."""
    out = []
    for n in notes:
        weak = (n[0] % unit) not in STRONG[unit]
        if weak and len(n) < 4 and out and r.random() < 0.45:
            if r.random() < 0.6 and abs(out[-1][0] + out[-1][1] - n[0]) < 1e-6:
                out[-1][1] += n[1]
            continue
        out.append(n)
    return out


def compose(cue):
    """The whole score of a cue: chord roots per span plus note events per stem. Events are
    (beat, beats, pitches, velocity, instrument). Pure and cheap (no audio)."""
    r = S.rng(cue['name'] + '/score')
    sc = Scale(cue['tonic'], cue['mode'])
    day = cue['time'] == 'day'
    meter = cue['meter']
    hr = 1 if day else 2              # bars per chord
    unit = meter * hr                 # beats per chord
    total = BARS * meter
    P = PROGS[cue['mode']]
    ante, cons, bopen = pick(r, P['ante']), pick(r, P['cons']), pick(r, P['b'])
    cons2 = pick(r, [c for c in P['cons'] if c != cons] or P['cons'])
    if day:
        form = [('A', ante, 'ante'), ('A', cons, 'cons'), ("A'", ante, 'ante'), ("A'", cons, 'cons'),
                ('B', bopen, 'ante'), ('B', cons2, 'cons'), ("A''", ante, 'ante'), ("A''", cons, 'cons')]
    else:
        form = [('A', ante, 'ante'), ('A', cons, 'cons'), ('B', bopen, 'ante'), ("A'", cons2, 'cons')]
    roots = [x for (_, prog, _) in form for x in prog]
    sections = [s for (s, _, _) in form for _ in range(4)]

    # Melody: a motif, its contrast, and the B section's inversion.
    lo, hi = -3, 7
    shape = pick(r, ['arch', 'rise', 'wave', 'fall'])
    motif = Motif(pick(r, [x for x in RHYTHMS[unit] if sum(1 for d in x if d > 0) >= 3] or RHYTHMS[unit]),
                  contour(r, 4, shape))
    contrast = Motif(pick(r, RHYTHMS[unit]), contour(r, 4, 'rise'))
    contrast_c = Motif(contrast.rhythm, contour(r, 4, 'fall'))
    motif_b = Motif(pick(r, RHYTHMS[unit]), motif.inverted().intervals)
    contrast_b = Motif(pick(r, RHYTHMS[unit]), contour(r, 4, 'arch'))
    mel = []
    hint = pick(r, [0, 2, 4])
    plen = 4 * unit
    cache = {}
    for gi, (sec, prog, kind) in enumerate(form):
        start = gi * plen
        if sec == 'B':
            notes, _ = phrase(r, motif_b, contrast_b, prog, kind, hint + 3, unit, lo + 2, hi + 2)
        else:
            key = (kind,)
            if sec == 'A' or key not in cache:
                notes, _ = phrase(r, motif, contrast if kind == 'ante' else contrast_c, prog, kind,
                                  hint, unit, lo, hi, seq=1 if kind == 'ante' else -1)
                cache[key] = notes
            else:
                base = cache[key]
                notes = ornament(r, [n[:3] for n in base if len(n) < 4], 0.35 if sec == "A'" else 0.2)
                notes += [n for n in base if len(n) == 4]
                if sec == "A''" and kind == 'cons':
                    # The last statement re-shapes its third span towards the final cadence.
                    alt = Motif(contrast_c.rhythm, contour(r, 4, 'fall'))
                    early = [n for n in notes if n[0] < 2 * unit]
                    last = early[-1][2] if early else hint
                    c = realise(alt, prog[2], last, unit, lo, hi)
                    notes = [n for n in notes if not (2 * unit <= n[0] < 3 * unit)]
                    notes += [[p + 2 * unit, d, g] for (p, d, g) in c]
                notes.sort(key=lambda n: n[0])
        if cue.get('sparse'):
            notes = thin(r, notes, unit)
        mel += [[start + n[0]] + list(n[1:]) for n in notes]

    lead = cue['lead']
    octave = cue.get('octave', OCTAVE[lead])
    if lead == 'bell' and 'octave' not in cue:
        octave = 12 if sc.tonic < 60 else 0  # night bells sit with the tonic in G4..F#5
    parts = {s: [] for s in STEMS}
    for n in mel:
        vel = (0.95 if (n[0] % meter) == 0 else 0.8) * (1.0 if len(n) < 4 else 0.9)
        parts['melody'].append((n[0], n[1], (sc.midi(n[2]) + octave,), vel, lead))

    # Drone: tonic and fifth in D2..B2, re-bowed every phrase.
    dl = PHRASE_BARS * meter * (1 if day else 2)
    low = sc.tonic - 12
    while low > 47:
        low -= 12
    for b in range(0, total, dl):
        for (st, v) in ((0, 1.0), (7, 0.55)):
            parts['drone'].append((b, dl, (low + st,), v * (0.7 if day else 1.0), 'bowed'))

    # Pad: voice-led triads (formant pad); day adds the plucked accompaniment.
    prev = None
    for i, root in enumerate(roots):
        b = i * unit
        voicing = voice_lead(sc, root, prev)
        prev = voicing
        parts['pad'].append((b, unit, tuple(voicing), 0.35 if day else 1.0, 'pad'))
        if day:
            acc = cue['accomp']
            bass = sc.midi(root) - 12
            if bass > sc.tonic - 5:
                bass -= 12
            upper = tuple(sorted(voicing)[1:])
            if meter == 3:
                figure = [(0, (bass,), 0.9), (1, upper, 0.55), (2, upper, 0.5)]
            else:
                figure = [(0, (bass,), 0.9), (1, upper, 0.55), (2, (bass + 7,), 0.7), (3, upper, 0.5)]
            for (p, pitches, v) in figure:
                parts['pad'].append((b + p, 1, pitches, v, acc))

    # Tension: frame drum and the second melody.
    parts['tension'] += drums(r, cue, total, unit, meter, day)
    parts['tension'] += counter_line(r, cue, sc, mel, roots, unit, meter, day)
    return dict(cue=cue, scale=sc, roots=roots, sections=sections, unit=unit, total=total,
                meter=meter, parts=parts, melody=mel)


def voice_lead(sc, root, prev):
    """A close triad near A3..E4, choosing the inversion nearest the previous voicing."""
    best, bscore = None, None
    for inv in range(3):
        degs = [root, root + 2, root + 4]
        degs = degs[inv:] + [d + 7 for d in degs[:inv]]
        for o in (-14, -7, 0):
            m = sorted(sc.midi(d + o) for d in degs)
            centre = sum(m) / 3.0
            if not 48 <= centre <= 62:
                continue
            score = sum(abs(a - b) for a, b in zip(m, prev)) if prev else abs(centre - 55)
            if bscore is None or score < bscore:
                best, bscore = m, score
    return best


def drums(r, cue, total, unit, meter, day):
    out = []
    if day:
        if meter == 3:
            pats = [[(0, 'dum', 1.0), (1, 'tek', 0.5), (2, 'tek', 0.6)],
                    [(0, 'dum', 1.0), (1, 'tek', 0.5), (1.5, 'tek', 0.3), (2, 'tek', 0.6)]]
            fill = [(0, 'dum', 1.0), (1, 'tek', 0.5), (1.5, 'tek', 0.4), (2, 'dum', 0.7), (2.5, 'tek', 0.45)]
        else:
            pats = [[(0, 'dum', 1.0), (1, 'tek', 0.5), (1.5, 'dum', 0.55), (2, 'tek', 0.6), (3, 'tek', 0.5)],
                    [(0, 'dum', 1.0), (1, 'tek', 0.5), (2, 'dum', 0.7), (3, 'tek', 0.55), (3.5, 'tek', 0.35)]]
            fill = [(0, 'dum', 1.0), (1, 'tek', 0.5), (2, 'tek', 0.5), (2.5, 'tek', 0.4), (3, 'dum', 0.8), (3.5, 'tek', 0.5)]
        main = pick(r, pats)
        alt = pick(r, pats)
        for bar in range(BARS):
            pat = fill if bar % PHRASE_BARS == PHRASE_BARS - 1 else (alt if bar % 2 else main)
            out += [(bar * meter + p, 0.5, (), v, k) for (p, k, v) in pat]
    else:
        # A slow heartbeat, a rim tap on every other bar, a fuller figure at phrase ends.
        for bar in range(BARS):
            b = bar * meter
            out += [(b, 0.5, (), 0.8, 'dum'), (b + 0.4, 0.5, (), 0.45, 'dum')]
            if bar % 2 == 1:
                out.append((b + 2, 0.5, (), 0.3, 'tek'))
            if bar % 8 == 7:
                out += [(b + 3, 0.5, (), 0.35, 'tek'), (b + 3.5, 0.5, (), 0.45, 'dum')]
    return out


def counter_line(r, cue, sc, mel, roots, unit, meter, day):
    """Second melody. Day: contrary motion against the melody's span-to-span direction, on the
    beats where the melody has no onset (harp), or one held note per chord (flute). Night: a low
    root-fifth-octave ostinato (harp) or slow bowed swells (deep)."""
    inst = cue['counter']
    out = []
    onsets = {round(n[0] * 2) / 2 for n in mel}
    means = []
    for i in range(len(roots)):
        ds = [n[2] for n in mel if i * unit <= n[0] < (i + 1) * unit]
        means.append(sum(ds) / len(ds) if ds else (means[-1] if means else 2))
    if day and inst == 'harp':
        lo, hi = -7, 2
        cur = nearest_tone(-3, roots[0], lo, hi)
        for i, root in enumerate(roots):
            d = 0 if i == 0 else (1 if means[i] > means[i - 1] else (-1 if means[i] < means[i - 1] else 0))
            cur = nearest_tone(cur - 2 * d, root, lo, hi)
            for k in range(unit):
                b = i * unit + k
                if b in onsets and r.random() < 0.7:
                    continue
                tones = [t for t in range(lo, hi + 1) if t % 7 in chord_tones(root)]
                j = tones.index(cur) if cur in tones else 0
                j = max(0, min(len(tones) - 1, j + (1 if k % 2 == 0 else -1) * (-d or 1)))
                cur = tones[j]
                out.append((b, 1, (sc.midi(cur) + OCTAVE[inst],), 0.55, inst))
    elif day:
        lo, hi = 0, 7
        cur = nearest_tone(4, roots[0], lo, hi)
        for i, root in enumerate(roots):
            d = 0 if i == 0 else (1 if means[i] > means[i - 1] else (-1 if means[i] < means[i - 1] else 0))
            cur = nearest_tone(cur - 2 * d, root, lo, hi, exclude=(root % 7,))
            out.append((i * unit + 1, unit - 1, (sc.midi(cur) + OCTAVE[inst],), 0.5, inst))
    elif inst == 'harp':
        for i, root in enumerate(roots):
            fig = [root - 7, root - 3, root, root - 3]
            for k in range(unit):
                g = fig[k % 4]
                out.append((i * unit + k, 1, (sc.midi(g) + OCTAVE[inst],), 0.4 if k % 4 else 0.55, inst))
    else:
        for i, root in enumerate(roots):
            g = nearest_tone(2, root, -1, 6, exclude=(root % 7,))
            out.append((i * unit + 2, unit - 3, (sc.midi(g) + OCTAVE[inst],), 0.6, inst))
    return out


# ---------------------------------------------------------------- rendering
PAN = {'flute': 0.12, 'lute': -0.2, 'harp': -0.3, 'bell': 0.0, 'bowed': 0.0, 'pad': 0.0,
       'dum': 0.2, 'tek': 0.28}
ROOM = {'day': (2.2, 0.22, 4500.0), 'night': (4.0, 0.42, 3200.0), 'deep': (5.5, 0.5, 2200.0)}
LEVEL = {'drone': 0.035, 'pad': 0.05, 'melody': 0.07, 'tension': 0.055}
CUE_LEVEL = {'day': 1.0, 'night': 0.8, 'deep': 0.75}


def voice(inst, pitches, dur, vel, r, cue):
    if inst in ('dum', 'tek'):
        return S.frame_drum(inst, r, 82.0 if cue['time'] == 'day' else 70.0) * vel
    if inst == 'pad':
        return S.pad_chord([S.midi_hz(m) for m in pitches], dur, r, cue['vowel']) * vel * 0.12
    x = None
    for m in pitches:
        f = S.midi_hz(m)
        if inst == 'lute':
            y = S.lute(f, dur, r, 0.35 + 0.3 * vel)
        elif inst == 'harp':
            y = S.harp(f, dur, r)
        elif inst == 'flute':
            y = S.flute(f, dur * 0.97, r)
        elif inst == 'bell':
            y = S.bell(f, r, max(4.0, dur + 3.0))
        elif inst == 'bowed':
            y = S.bowed(f, dur, r, attack=min(1.8, dur * 0.4))
        else:
            raise ValueError(inst)
        x = y if x is None else _add(x, y)
    return x * vel


def _add(a, b):
    if len(a) < len(b):
        a, b = b, a
    a = a.copy()
    a[:len(b)] += b
    return a


def add_circular(buf, start, sig):
    """Add a stereo signal into a loop buffer, wrapping past the end back to the start."""
    n = len(buf)
    i = 0
    s = start % n
    while i < len(sig):
        k = min(len(sig) - i, n - s)
        buf[s:s + k] += sig[i:i + k]
        i += k
        s = 0


def active_rms(x):
    """Loudness of the parts that sound: RMS over 100 ms windows within 20 dB of the loudest."""
    m = x if x.ndim == 1 else x.mean(axis=1)
    w = int(SR * 0.1)
    k = len(m) // w
    win = np.sqrt(np.mean(m[:k * w].reshape(k, w) ** 2, axis=1))
    on = win[win > win.max() * 0.1]
    return float(np.sqrt(np.mean(on ** 2))) if len(on) else 1e-9


def render(score):
    cue = score['cue']
    spb = 60.0 / cue['bpm']
    n = int(round(score['total'] * spb * SR))
    stems = {}
    for stem in STEMS:
        r = S.rng('%s/%s' % (cue['name'], stem))
        buf = np.zeros((n, 2))
        for (beat, beats, pitches, vel, inst) in score['parts'][stem]:
            x = voice(inst, pitches, beats * spb, vel, r, cue)
            p = PAN[inst] + (r.uniform(-0.45, 0.45) if inst == 'bell' else 0.0)
            # a few milliseconds of human timing on plucks and drums
            jit = int(SR * r.uniform(0, 0.012)) if inst in ('lute', 'harp', 'dum', 'tek') else 0
            add_circular(buf, int(round(beat * spb * SR)) + jit, S.pan(x, p))
        secs, mix, damp = ROOM[cue['time']]
        buf = S.reverb_loop(buf, secs, mix, S.rng('%s/%s/room' % (cue['name'], stem)), damp)
        buf *= LEVEL[stem] * CUE_LEVEL[cue['time']] / active_rms(buf)
        stems[stem] = buf
    mixed = sum(stems.values())
    peak = float(np.max(np.abs(mixed)))
    if peak > 0.89:
        for s in stems:
            stems[s] *= 0.89 / peak
    return stems, n


def main(argv):
    os.makedirs(OUT, exist_ok=True)
    want = set(argv) or {c['name'] for c in CUES}
    man = {'stems': list(STEMS), 'phrase_bars': PHRASE_BARS, 'cues': {}}
    rows = []
    for cue in CUES:
        score = compose(cue)
        spb = 60.0 / cue['bpm']
        n = int(round(score['total'] * spb * SR))
        files = {s: 'music/%s_%s.ogg' % (cue['name'], s) for s in STEMS}
        if cue['name'] in want:
            stems, n = render(score)
            for s in STEMS:
                x = stems[s].mean(axis=1) if s == 'drone' else stems[s]
                S.write_ogg(os.path.join(OUT, '%s_%s.ogg' % (cue['name'], s)), x, QUALITY)
            mixed = sum(stems.values())
            print('music: %-13s %s %s %3d bpm %d/4 %6.2fs  mix peak %.2f rms %.3f' % (
                cue['name'], cue['tonic'], cue['mode'], cue['bpm'], cue['meter'], n / SR,
                float(np.max(np.abs(mixed))), math.sqrt(float(np.mean(mixed ** 2)))))
        man['cues'][cue['name']] = {
            'season': cue['season'], 'time': cue['time'], 'key': '%s %s' % (cue['tonic'], cue['mode']),
            'bpm': cue['bpm'], 'meter': cue['meter'], 'bars': BARS, 'seconds': round(n / SR, 4),
            'files': files}
        for s in STEMS:
            rows.append('[[asset]]\npath = "godot/assets/audio/%s"\nkind = "audio"\nsource = "generated"\n'
                        'generator = "tools/audio/music.py"\nlicence = "project-original"\n' % files[s])
    with open(MANIFEST, 'w') as f:
        json.dump(man, f, indent=1, sort_keys=True)
    with open(PROVENANCE, 'w') as f:
        f.write('# Written by tools/audio/music.py; every cue is composed and synthesised there.\n\n'
                + '\n'.join(rows))


if __name__ == '__main__':
    main(sys.argv[1:])
