#!/usr/bin/env python3
"""Every enum in hf-sim/src at base must be an
unchanged prefix of the same enum here: saves store variants by position, so a variant
appended out of order (a merge putting a branch's new variants before master's) loads old
saves wrong. Usage: tools/enum_order.py <worktree> <base ref, e.g. the last tag>."""
import re, subprocess, sys, os
wt, base = sys.argv[1], sys.argv[2]
def enums(text):
    out = {}
    for m in re.finditer(r'pub enum (\w+)[^{]*\{', text):
        i = m.end(); depth = 1; body = []
        j = i
        while depth and j < len(text):
            c = text[j]
            if c in '{(': depth += 1
            elif c in '})': depth -= 1
            j += 1
        seg = text[i:j-1]
        names, d = [], 0
        for line in seg.split('\n'):
            s = line.strip()
            if d == 0:
                mm = re.match(r'([A-Z]\w*)\s*(\{|\(|,|=|$)', s)
                if mm and not s.startswith('//') and not s.startswith('#'): names.append(mm.group(1))
            d += line.count('{') + line.count('(') - line.count('}') - line.count(')')
        out[m.group(1)] = names
    return out
bad = 0
files = subprocess.run(['git','-C',wt,'ls-files','crates/hf-sim/src'],capture_output=True,text=True).stdout.split()
for f in files:
    if not f.endswith('.rs'): continue
    old = subprocess.run(['git','-C',wt,'show',f'{base}:{f}'],capture_output=True,text=True)
    if old.returncode: continue
    new = open(os.path.join(wt,f)).read()
    eo, en = enums(old.stdout), enums(new)
    for k, v in eo.items():
        if k in en and en[k][:len(v)] != v:
            bad += 1
            first = next(i for i,(a,b) in enumerate(zip(v,en[k]+['']*len(v))) if a!=b)
            print(f'{f}: enum {k}: base variant {first} {v[first]!r} is now {en[k][first] if first<len(en[k]) else None!r}')
print('enum prefix check:', 'OK' if not bad else f'{bad} reordered')
sys.exit(1 if bad else 0)
