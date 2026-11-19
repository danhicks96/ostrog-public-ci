#!/usr/bin/env python3
"""Silhouette check: can the kinds of people be told apart by outline alone at 1x?

For every species in the library (data/peoples/species/*.toml) this takes the average adult's
body sheet (art.body names the set: godot/assets/people/body_<body>_average_adult.png, or
body_average_adult.png for "human"; the cell is the plan's, swaps.json plan_cells), cuts the
idle frame facing S and facing W, and reduces each to its opaque mask (outline included, as it
is drawn). For every pair of kinds and each view the two masks are aligned on the feet row (the
lowest opaque row) and the horizontal centre of the mask's bounding box, and the difference is

    d = |A xor B| / |A or B|

the fraction of the pixels either figure covers that only one of them covers. The pair's score
is the smaller of its S and W differences: a kind must read apart from every other facing the
viewer and in profile, not on average.

No pair is exempt, including kinds that share a body plan (human, Hollin, Murrow and Seamfolk
are all bipeds). Their heads and parts are all that can tell them apart, so those parts have to
be large enough at 1x to move d past the threshold like any other difference. The human child
is in the table as a thirteenth figure: a small kind is only readable if it does not pass for a
human child.

Threshold, from the measurements before C53 (docs/reviews/readability-2026-10-19/README.md has
the tables before and after). The pairs the MP review found too alike at 1x (the six bipeds
among themselves, and the Grisk, which "read as small humans", against the human child) scored
0.000 to 0.282. Of the pairs it found readable, the Screeling, Holm, Below and Gnarrow pairs
scored 0.303 and up; the Skarri against the bipeds scored 0.247 to 0.273, the one place where
d is stricter than the review's eye. THRESHOLD = 0.30 is above every rejected pair (by 0.018)
and at the bottom of the accepted ones; the Skarri were brought over it (wing wrists higher,
bird's legs and taloned feet) rather than the threshold lowered into the rejected range. The
check fails (exit 1) when any pair scores under it.

The mask is the body layer only. Clothing, hair and equipment are drawn per body plan, not per
species, so a difference that a garment covers does not survive in play; the README lists what
the check cannot see.

--full: the gate for the whole drawn roster (#485). The client renders every kind in every
build and age (Look.body % BUILDS -> slim/average/broad in godot/scripts/world_view.gd; the
child flag from the life stage in crates/hf-sim/src/view/look.rs; the sheet resolution in
godot/scripts/render/entity_layer.gd), so the roster is 12 kinds x 3 builds x 2 ages = 72
figures and 2556 pairs. Pairs of figures of the SAME kind (build/age variants of one kind)
are measured and reported but not gated: within a kind, variation is meant to be bounded and
coherent (individuals of one kind may resemble each other), while two people of DIFFERENT
kinds must read apart whatever their build and age, so every cross-kind pair is gated at the
same 0.30. The human child is not a separate figure here: human children of all three builds
are in the roster, which covers the "small kind must not pass for a human child" case against
every small kind. Cross-kind pairs scoring under the threshold are listed as gaps for the
production art PR (D63); the current master's gaps are written down in
docs/design-character-visuals.md.

Run: python3 tools/art/silhouette_check.py [--quiet] [--full]
"""
import json
import os
import sys
import tomllib

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pal import ROOT  # noqa: E402

PEOPLE = os.path.join(ROOT, 'godot', 'assets', 'people')
SPECIES = os.path.join(ROOT, 'data', 'peoples', 'species')
THRESHOLD = 0.30
BASELINE = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'silhouette_baseline.json')
VIEWS = ('S', 'W')
# The client's order (v.BUILD in godot/scripts/world_view.gd); Look.body % BUILDS picks one.
BUILDS = ('slim', 'average', 'broad')
AGES = ('adult', 'child')


def library():
    """(id, name, body sheet, plan) for every species file, in id order."""
    out = []
    for fn in sorted(os.listdir(SPECIES)):
        if not fn.endswith('.toml'):
            continue
        with open(os.path.join(SPECIES, fn), 'rb') as f:
            t = tomllib.load(f)
        out.append((t['id'], t.get('name', t['id']), t['art']['body'], t['plan']))
    return out


def sheet_path(body, build='average', age='adult'):
    name = ('body_%s_%s.png' % (build, age) if body == 'human'
            else 'body_%s_%s_%s.png' % (body, build, age))
    return os.path.join(PEOPLE, name)


def masks(body, plan, swaps, build='average', age='adult'):
    """{view: bool mask} of the idle frame, cut from the plan's cell."""
    cols, rows = swaps['layout']['columns'], swaps['layout']['rows']
    cell = swaps['plan_cells'][plan]
    sheet = np.asarray(Image.open(sheet_path(body, build, age)).convert('RGBA'))
    assert sheet.shape[1] == cell * len(cols) and sheet.shape[0] == cell * len(rows), \
        '%s %s %s: sheet is not %d x %d cells of %d px' % (body, build, age, len(cols), len(rows), cell)
    x = cols.index('idle') * cell
    out = {}
    for v in VIEWS:
        y = rows.index(v) * cell
        out[v] = sheet[y:y + cell, x:x + cell, 3] > 0
    return out


def placed(m, size):
    """The mask on a size x size canvas, feet row at the bottom - 2 and bbox centre at the middle."""
    ys, xs = np.nonzero(m)
    feet = ys.max()
    cx = (xs.min() + xs.max()) // 2
    out = np.zeros((size, size), dtype=bool)
    dy = size - 2 - feet
    dx = size // 2 - cx
    out[ys + dy, xs + dx] = True
    return out


def diff(a, b):
    union = (a | b).sum()
    return float((a ^ b).sum()) / union if union else 0.0


def main():
    quiet = '--quiet' in sys.argv
    if '--full' in sys.argv:
        sys.exit(full_check(quiet))
    swaps = json.load(open(os.path.join(PEOPLE, 'swaps.json')))
    size = max(swaps['plan_cells'].values()) * 2
    kinds = library()
    m = {}
    for sid, name, body, plan in kinds:
        mk = masks(body, plan, swaps)
        m[sid] = {v: placed(mk[v], size) for v in VIEWS}
    # The human child stands in the table as a kind of its own: a small adult kind is only
    # readable if it does not pass for a human child (the MP review's "Grisk read as small
    # humans").
    mk = masks('human', 'biped', swaps, age='child')
    m['human_child'] = {v: placed(mk[v], size) for v in VIEWS}
    kinds.append(('human_child', 'human child', 'human', 'biped'))
    rows = []
    for i in range(len(kinds)):
        for j in range(i + 1, len(kinds)):
            a, b = kinds[i][0], kinds[j][0]
            ds = {v: diff(m[a][v], m[b][v]) for v in VIEWS}
            rows.append((min(ds.values()), a, b, ds))
    rows.sort()
    fails = [r for r in rows if r[0] < THRESHOLD]
    if not quiet:
        print('%-10s %-10s %6s %6s %6s' % ('kind', 'kind', 'S', 'W', 'min'))
        for score, a, b, ds in rows:
            flag = '  FAIL' if score < THRESHOLD else ''
            print('%-10s %-10s %6.3f %6.3f %6.3f%s' % (a, b, ds['S'], ds['W'], score, flag))
    print('silhouette_check: %d kinds and the human child, %d pairs, threshold %.2f, %d under it; '
          'weakest %s/%s %.3f' % (len(kinds) - 1, len(rows), THRESHOLD, len(fails),
                                  rows[0][1], rows[0][2], rows[0][0]))
    sys.exit(1 if fails else 0)


def full_check(quiet):
    """The #485 gate: every drawn kind x build x age figure; every cross-kind pair at THRESHOLD.

    Pairs of figures of the same kind are measured and reported but not gated: variation
    within a kind is meant to be bounded and coherent, so a slim Grisk may pass for a broad
    Grisk; a Grisk may not pass for a Meer.
    """
    swaps = json.load(open(os.path.join(PEOPLE, 'swaps.json')))
    size = max(swaps['plan_cells'].values()) * 2
    kinds = library()
    figures = []  # (figure id, kind id), in library then build then age order
    m = {}
    for sid, name, body, plan in kinds:
        for build in BUILDS:
            for age in AGES:
                fid = '%s_%s_%s' % (sid, build, age)
                mk = masks(body, plan, swaps, build, age)
                m[fid] = {v: placed(mk[v], size) for v in VIEWS}
                figures.append((fid, sid))
    rows = []
    for i in range(len(figures)):
        for j in range(i + 1, len(figures)):
            (fa, ka), (fb, kb) = figures[i], figures[j]
            ds = {v: diff(m[fa][v], m[fb][v]) for v in VIEWS}
            rows.append((min(ds.values()), fa, fb, ds, ka, kb))
    rows.sort(key=lambda r: (r[0], r[1], r[2]))
    gated = [r for r in rows if r[4] != r[5]]
    intra = [r for r in rows if r[4] == r[5]]
    gaps = [r for r in gated if r[0] < THRESHOLD]
    # Ratchet: master's art predates the gate, so the known gaps are recorded in BASELINE and
    # tolerated at their recorded score; a gap that is not recorded, or that got worse, fails.
    # A gap that closes must be dropped from the baseline (the gate says so), so it only tightens.
    base = json.load(open(BASELINE)) if os.path.exists(BASELINE) else {}
    if '--write-baseline' in sys.argv:
        with open(BASELINE, 'w') as f:
            json.dump({'%s|%s' % (a, b): round(sc, 3) for sc, a, b, _, _, _ in gaps},
                      f, indent=1, sort_keys=True)
            f.write('\n')
        base = {'%s|%s' % (a, b): round(sc, 3) for sc, a, b, _, _, _ in gaps}
    fails = [r for r in gaps if '%s|%s' % (r[1], r[2]) not in base
             or r[0] < base['%s|%s' % (r[1], r[2])] - 0.0015]
    stale = sorted(set(base) - {'%s|%s' % (r[1], r[2]) for r in gaps})
    for k in stale:
        print('note: baseline entry now passes, remove it from %s: %s'
              % (os.path.basename(BASELINE), k))
    if not quiet:
        # The whole table is 2556 rows, more log than anyone reads: print the gated failures
        # (the gaps the production art PR must close), the weakest pairs still over the
        # threshold for context, and the intra-kind range for information.
        print('%-30s %-30s %6s %6s %6s' % ('figure', 'figure', 'S', 'W', 'min'))
        for score, a, b, ds, _, _ in gaps:
            print('%-30s %-30s %6.3f %6.3f %6.3f  %s' % (a, b, ds['S'], ds['W'], score, 'NEW/WORSE' if (score, a, b) in [(r[0], r[1], r[2]) for r in fails] else 'known'))
        print('weakest gated pairs still over %.2f:' % THRESHOLD)
        for score, a, b, ds, _, _ in [r for r in gated if r[0] >= THRESHOLD][:10]:
            print('%-30s %-30s %6.3f %6.3f %6.3f' % (a, b, ds['S'], ds['W'], score))
        print('intra-kind (same kind, build/age variants; reported, not gated): '
              '%d pairs, %.3f to %.3f' % (len(intra), intra[0][0], intra[-1][0]))
        by_kinds = {}
        for score, a, b, ds, ka, kb in gaps:
            key = (ka, kb) if ka < kb else (kb, ka)
            by_kinds.setdefault(key, []).append(score)
        per_kind = (len(BUILDS) * len(AGES)) ** 2
        print('gaps by kind pair:')
        for ka, kb in sorted(by_kinds, key=lambda k: min(by_kinds[k])):
            s = by_kinds[(ka, kb)]
            print('  %-10s x %-10s %2d of %d appearance pairs under %.2f, worst %.3f'
                  % (ka, kb, len(s), per_kind, THRESHOLD, min(s)))
    print('silhouette_check --full: %d figures (%d kinds x %d builds x %d ages), %d pairs '
          '(%d cross-kind gated, %d intra-kind reported), threshold %.2f, %d cross-kind pairs '
          'under it (%d known, %d new or worse); weakest gated %s/%s %.3f'
          % (len(figures), len(kinds), len(BUILDS), len(AGES), len(rows), len(gated), len(intra),
             THRESHOLD, len(gaps), len(gaps) - len(fails), len(fails),
             gated[0][1], gated[0][2], gated[0][0]))
    return 1 if fails else 0


if __name__ == '__main__':
    main()
