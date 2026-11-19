#!/usr/bin/env python3
"""Refresh the counts table in ASSETS.md from the provenance tables.

ASSETS.md has always said it is "Updated by tools/art/provenance_report.py at checkpoints" — that
script did not exist, which is why the record still read 0 generated / 0 sourced / 0 hand while
~140 assets were present. This writes the real numbers.

Stdlib only: intended to run from a checkpoint (and possibly with -I), so no Pillow/numpy.
"""
import os
import re
import sys
import tomllib

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
ASSETS_MD = os.path.join(ROOT, 'docs', 'ASSETS.md')
# The same tables provenance_check.py reads, sounds and music included (C54: the report read
# only the first two and counted 1,914 of 2,008 assets).
TABLES = [
    os.path.join(ROOT, 'tools', 'art', 'provenance.toml'),
    os.path.join(ROOT, 'tools', 'art', 'provenance_people.toml'),
    os.path.join(ROOT, 'tools', 'audio', 'provenance_audio.toml'),
    os.path.join(ROOT, 'tools', 'audio', 'provenance_music.toml'),
]
# Everything the generators produce counts as generated; derived is art reduced and
# palette-quantised from image-model sheets (tools/art/derive.py, C22), each row naming its
# model, sheet and licence; the other two rows exist for art that is sourced under a licence or
# hand-pixelled, which nothing is yet.
GENERATED = {'generated', 'procedural'}


def tally():
    counts = {'Generated': 0, 'Derived': 0, 'Sourced': 0, 'Hand-pixelled': 0}
    unknown = {}
    seen = set()
    for t in TABLES:
        if not os.path.exists(t):
            continue
        with open(t, 'rb') as f:
            data = tomllib.load(f)
        for row in data.get('asset', []):
            p = row.get('path', '')
            if not p or p in seen:
                continue
            seen.add(p)
            src = (row.get('source') or '').strip().lower()
            if src in GENERATED:
                counts['Generated'] += 1
            elif src == 'derived':
                counts['Derived'] += 1
            elif src == 'sourced':
                counts['Sourced'] += 1
            elif src == 'hand':
                counts['Hand-pixelled'] += 1
            else:
                unknown[src or '(missing source)'] = unknown.get(src or '(missing source)', 0) + 1
    return counts, unknown, len(seen)


def main():
    check = '--check' in sys.argv[1:]
    counts, unknown, total = tally()
    try:
        with open(ASSETS_MD) as f:
            text = f.read()
    except OSError as e:
        print('provenance_report: cannot read ASSETS.md: %s' % e)
        return 1
    new = text
    for label, n in counts.items():
        new, k = re.subn(r'^\| %s \| \d+ \|(.*)$' % re.escape(label),
                         lambda m: '| %s | %d |%s' % (label, n, m.group(1)),
                         new, flags=re.M)
        if k == 0:
            print('provenance_report: no row for "%s" in ASSETS.md' % label)
            return 1
    if check:
        if new != text:
            print('provenance_report: ASSETS.md counts are out of date: run python3 tools/art/provenance_report.py')
            return 1
        print('provenance_report: ASSETS.md counts current (%d assets)' % total)
        return 0
    with open(ASSETS_MD, 'w') as f:
        f.write(new)
    print('provenance_report: %d assets — generated %d, sourced %d, hand %d'
          % (total, counts['Generated'], counts['Sourced'], counts['Hand-pixelled']))
    for src, n in sorted(unknown.items()):
        print('  unclassified source %r: %d' % (src, n))
    return 0


if __name__ == '__main__':
    sys.exit(main())
