#!/usr/bin/env python3
"""Collect exact CLI digests only after every computation succeeds and validates.

FIXLOG 2026-11-19 launch-prep: echo command substitutions hid CLI failures.
No normalization or tolerance of digest differences; cross-platform diff stays exact.
"""
import re
import subprocess
import sys

HEX = re.compile(r'[0-9a-f]{64}')

def compute(binary, args, *, opening):
    result = subprocess.run([binary, *args], stdout=subprocess.PIPE, text=True, check=True)
    lines = result.stdout.splitlines()
    if opening:
        lines = [line for line in lines if line.startswith('digest')]
        if len(lines) != 1 or not lines[0].startswith('digest '):
            raise ValueError('opening run must emit exactly one digest line')
        value = lines[0][len('digest '):]
    else:
        if len(lines) != 1:
            raise ValueError('tick-zero command must emit one digest only')
        value = lines[0]
    if HEX.fullmatch(value) is None:
        raise ValueError('digest must be exactly 64 lowercase hexadecimal characters')
    return ('digest ' if opening else '') + value

def main():
    if len(sys.argv) != 2:
        raise ValueError('usage: ci_digests.py PATH_TO_OSTROG')
    lines = []
    for seed in ('3', '7'):
        tick0 = compute(sys.argv[1], ['digest', '--seed', seed], opening=False)
        day3 = compute(sys.argv[1], ['run', '--opening', '--seed', seed, '--days', '3'], opening=True)
        lines.extend((f'seed {seed} tick0 {tick0}', f'seed {seed} day3 {day3}'))
    # Windows text stdout expands LF to CRLF; raw cross-platform diff needs LF bytes.
    sys.stdout.buffer.write(('\n'.join(lines) + '\n').encode('ascii'))

if __name__ == '__main__':
    try:
        main()
    except (OSError, UnicodeError, ValueError, subprocess.CalledProcessError) as error:
        print(f'digest computation failed: {error}', file=sys.stderr)
        sys.exit(1)
