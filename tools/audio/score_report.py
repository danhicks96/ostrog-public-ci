#!/usr/bin/env python3
"""Print each music cue's score bar by bar, so the composition rules can be read without
listening: section, chord (Roman numeral and root), the melody's notes with durations in beats,
and optionally the second melody and the drone.

    python3 tools/audio/score_report.py                 # every cue, melody only
    python3 tools/audio/score_report.py spring_day -a   # one cue, every pitched part

Notes print as NAME:BEATS (a trailing * marks a cadence note); '-' is a bar with no onset.
The score is recomposed from tools/audio/music.py (no audio is rendered), so it is exactly what
the OGG stems were rendered from.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import music as M  # noqa: E402


def fmt(beats):
    s = ('%.2f' % beats).rstrip('0').rstrip('.')
    return s[1:] if s.startswith('0.') else s


def bar_notes(events, bar, meter, flats=False):
    lo, hi = bar * meter, (bar + 1) * meter
    out = []
    for (b, d, pitches, _v, _i) in sorted(events, key=lambda e: (e[0], e[2])):
        if lo <= b < hi and pitches:
            out.append('+'.join(M.name(p, flats) for p in pitches) + ':' + fmt(d))
    return ' '.join(out) or '-'


def report(cue, every=False):
    sc = M.compose(cue)
    s = sc['scale']
    meter, unit = sc['meter'], sc['unit']
    hr = unit // meter
    cad = {round(n[0], 3) for n in sc['melody'] if len(n) == 4}
    print('== %s: %s %s, %d bpm, %d/4, %d bars, lead %s, second melody %s%s' % (
        cue['name'], cue['tonic'], cue['mode'], cue['bpm'], meter, M.BARS, cue['lead'],
        cue['counter'], ', sparse' if cue.get('sparse') else ''))
    mel = sc['parts']['melody']
    ctr = [e for e in sc['parts']['tension'] if e[2]]
    for bar in range(M.BARS):
        span = bar // hr
        root = sc['roots'][span]
        chord = '%-5s %-3s' % (s.numeral(root), (M.FLATS if s.flats else M.NAMES)[s.midi(root) % 12]) if bar % hr == 0 else ' ' * 9
        sec = sc['sections'][span] if bar % (M.PHRASE_BARS) == 0 else ''
        notes = []
        for (b, d, pitches, _v, _i) in sorted(mel, key=lambda e: e[0]):
            if bar * meter <= b < (bar + 1) * meter:
                notes.append(M.name(pitches[0], s.flats) + ':' + fmt(d) + ('*' if round(b, 3) in cad else ''))
        print('%4s %02d | %s | %s' % (sec, bar + 1, chord, ' '.join(notes) or '-'))
        if every:
            print('        |           | 2nd: %s' % bar_notes(ctr, bar, meter, s.flats))
            if bar % M.PHRASE_BARS == 0:
                print('        |           | pad: %s   drone: %s' % (
                    bar_notes(sc['parts']['pad'], bar, meter, s.flats), bar_notes(sc['parts']['drone'], bar, meter, s.flats)))
    print()


def main(argv):
    every = '-a' in argv
    names = [a for a in argv if not a.startswith('-')]
    for cue in M.CUES:
        if not names or cue['name'] in names:
            report(cue, every)


if __name__ == '__main__':
    main(sys.argv[1:])
