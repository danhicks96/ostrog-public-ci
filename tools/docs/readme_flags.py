#!/usr/bin/env python3
"""Fail when docs/REFERENCE.md leaves out a client flag or a CLI command or option (#188, QA
2026-10-20, C63: five client flags, three commands and a dozen options had gone undocumented).

    python3 -I tools/docs/readme_flags.py    # prints what is missing; exit 1 if anything is

Client flags are read from `_parse_args()` in godot/scripts/main.gd (`a.begins_with("--x=")`,
`a == "--x"`); they must appear in the "Client command-line flags" section. CLI commands and
their `--options` are read from `enum Cmd` in crates/hf-cli/src/main.rs; they must appear in the
"The ostrog CLI" table. Stdlib only.
"""
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))


def read(*p):
    with open(os.path.join(ROOT, *p), encoding='utf-8') as f:
        return f.read()


def section(text, heading):
    i = text.index(heading)
    j = text.find('\n### ', i + len(heading))
    return text[i:j if j >= 0 else len(text)]


def kebab(name):
    return re.sub(r'(?<!^)([A-Z])', r'-\1', name).lower()


def main():
    readme = read('docs/REFERENCE.md')
    missing = []
    flags_doc = section(readme, '### Client command-line flags')
    gd = read('godot', 'scripts', 'main.gd')
    body = gd[gd.index('func _parse_args'):gd.index('func _ready')]
    flags = set(re.findall(r'a\.begins_with\("(--[a-z-]+)=?"\)', body))
    flags |= set(re.findall(r'a == "(--[a-z-]+)(?:=[a-z]+)?"', body))
    for f in sorted(flags):
        if f not in flags_doc:
            missing.append('client flag %s' % f)
    cli_doc = section(readme, '### The ostrog CLI')
    rs = read('crates', 'hf-cli', 'src', 'main.rs')
    i = rs.index('enum Cmd {')
    depth, j = 0, i
    while True:
        c = rs[j]
        if c == '{':
            depth += 1
        elif c == '}':
            depth -= 1
            if depth == 0:
                break
        j += 1
    enum = rs[i:j]
    for m in re.finditer(r'^    ([A-Z][A-Za-z]*)\s*(\{|,|\()', enum, re.M):
        cmd = kebab(m.group(1))
        row = re.search(r'^\| `%s` \|(.*)$' % re.escape(cmd), cli_doc, re.M)
        if row is None:
            missing.append('CLI command %s' % cmd)
            continue
        if m.group(2) != '{':
            continue
        k, d = m.end(), 1
        while d:
            d += {'{': 1, '}': -1}.get(enum[k], 0)
            k += 1
        fields = enum[m.end():k]
        for fm in re.finditer(r'^\s+([a-z_]+):', fields, re.M):
            before = fields[:fm.start()].rstrip().split('\n')[-1]
            if 'long' not in before:
                continue  # a positional argument, written in capitals in the table
            flag = '--' + fm.group(1).replace('_', '-')
            if flag not in row.group(1):
                missing.append('CLI option %s %s' % (cmd, flag))
    for m in missing:
        print('docs/REFERENCE.md is missing the %s' % m)
    if missing:
        sys.exit(1)
    print('README flags and commands: all documented')


if __name__ == '__main__':
    main()
