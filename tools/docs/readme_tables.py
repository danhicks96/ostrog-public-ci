#!/usr/bin/env python3
"""Regenerate the tables in docs/REFERENCE.md that are read straight out of the code.

    python3 tools/docs/readme_tables.py           # rewrite the generated blocks in docs/REFERENCE.md
    python3 tools/docs/readme_tables.py --check   # exit 1 if docs/REFERENCE.md is out of date

docs/REFERENCE.md has blocks fenced by `<!-- BEGIN GENERATED: name -->` and `<!-- END GENERATED: name -->`.
Everything between a pair is replaced; everything outside is hand-written and left alone.

Blocks:
  strings-sim     every player-visible sentence the simulation can produce: feed and history
                  narration per EventKind, past lives, belief phrases, techniques, activities,
                  thoughts, breaks, customs, the Engine, body, items, recipes, and an index of
                  every other string literal in crates/hf-sim/src (file:line), so a tester can
                  find where any line on screen comes from.
  strings-client  string literals in the Godot client scripts (on-screen text and console
                  output such as SHOT_SAVED / UNKNOWN_FLAG), with file:line.
  assets          every file under godot/assets with pixel size, frame layout and generator,
                  the sounds.json manifest, and the client's enum-to-sprite name arrays checked
                  against the Rust enums (a mismatch is printed and fails --check).
  bridge          the per-agent field layout of SimHost.agents() (crates/hf-godot/src/lib.rs),
                  checked against AGENT_STRIDE in world_view.gd.
  pipeline        the tick pipeline in crates/hf-sim/src/systems/mod.rs, in order.

Stdlib only (no Pillow): PNG sizes are read from the file header. Parsing is a small
brace-matching scanner, not a Rust parser; it relies on the code's own conventions
(`match` arms, `format!` templates, `"..."` literals). If a table comes out empty or odd after
a refactor, fix the scanner rather than hand-editing the README block.
"""
import json
import glob
import os
import re
import struct
import sys
import tomllib

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
SIM = os.path.join(ROOT, 'crates', 'hf-sim', 'src')
README = os.path.join(ROOT, 'docs', 'REFERENCE.md')
ASSETS = os.path.join(ROOT, 'godot', 'assets')
WARNINGS = []


def rel(p):
    return os.path.relpath(p, ROOT).replace(os.sep, '/')


def read(p):
    with open(p, encoding='utf-8') as f:
        return f.read()


# ------------------------------------------------------------------ Rust scanning
# Rust's backslash-newline continuation is one escape, just like an escaped quote.
# Failing at a continuation lets the closing quote masquerade as the next opening quote.
LIT = re.compile(r'"((?:\\[\s\S]|[^"\\])*)"')


def strip_tests(src):
    """Drop `#[cfg(test)]` modules (to the end of the file, the repo's convention)."""
    i = src.find('#[cfg(test)]')
    return src if i < 0 else src[:i]


def mask(src):
    """Same length as src, with comments, string and char literals blanked to spaces, so
    brace matching and arm splitting never see punctuation inside them."""
    out = list(src)
    i, n = 0, len(src)
    while i < n:
        c = src[i]
        if src.startswith('//', i):
            j = src.find('\n', i)
            j = n if j < 0 else j
            for k in range(i, j):
                out[k] = ' '
            i = j
        elif src.startswith('/*', i):
            j = src.find('*/', i + 2)
            j = n if j < 0 else j + 2
            for k in range(i, j):
                if out[k] != '\n':
                    out[k] = ' '
            i = j
        elif c == '"':
            j = i + 1
            while j < n and src[j] != '"':
                j += 2 if src[j] == '\\' else 1
            for k in range(i + 1, min(j, n)):
                if out[k] != '\n':
                    out[k] = ' '
            i = j + 1
        elif c == "'":
            m = re.match(r"'(\\.|[^\\'])'", src[i:])
            if m:
                for k in range(i + 1, i + len(m.group(0)) - 1):
                    out[k] = ' '
                i += len(m.group(0))
            else:
                i += 1  # a lifetime
        else:
            i += 1
    return ''.join(out)


def match_close(m, i):
    """Index of the bracket closing the one at m[i] (m is masked source)."""
    pairs = {'{': '}', '(': ')', '[': ']'}
    stack = [pairs[m[i]]]
    j = i + 1
    while j < len(m):
        c = m[j]
        if c in pairs:
            stack.append(pairs[c])
        elif c in ')}]':
            stack.pop()
            if not stack:
                return j
        j += 1
    raise ValueError('unbalanced')


def fn_body(src, m, name, start=0):
    """(open, close) of the body of `fn name(`."""
    k = re.search(r'\bfn\s+' + re.escape(name) + r'\s*[(<]', m[start:])
    if not k:
        raise KeyError(name)
    i = m.index('{', start + k.end())
    return i, match_close(m, i)


def first_match_block(m, lo, hi):
    """(open, close) of the first `match ... {` block inside m[lo:hi]."""
    k = re.search(r'\bmatch\b', m[lo:hi])
    if not k:
        raise KeyError('match')
    i = lo + k.end()
    while m[i] != '{':  # skip a scrutinee that has brackets of its own
        i = match_close(m, i) + 1 if m[i] in '([' else i + 1
    return i, match_close(m, i)


def arms(src, m, lo, hi):
    """Split a match block body m[lo+1:hi] into (pattern, expression, line) triples."""
    out = []
    i = lo + 1
    depth = 0
    pat_start = i
    while i < hi:
        c = m[i]
        if c in '([{':
            depth += 1
        elif c in ')]}':
            depth -= 1
        elif depth == 0 and m.startswith('=>', i):
            pat = src[pat_start:i].strip()
            j = i + 2
            while m[j] in ' \t\n':
                j += 1
            if m[j] == '{':
                end = match_close(m, j) + 1
            else:
                end = j
                d = 0
                while end < hi:
                    ch = m[end]
                    if ch in '([{':
                        d += 1
                    elif ch in ')]}':
                        if d == 0:
                            break
                        d -= 1
                    elif ch == ',' and d == 0:
                        break
                    end += 1
            out.append((pat, src[j:end], src.count('\n', 0, pat_start) + 1))
            i = end
            while i < hi and m[i] in ' \t\n,':
                i += 1
            pat_start = i
            continue
        i += 1
    return out


def simplify_arg(a):
    a = a.strip().lstrip('&*').strip()
    if 'format!' in a:
        inner = re.search(r'\{(\w+)', a)
        return inner.group(1) if inner else 'text'
    call = re.match(r'^[\w:]+\((.*)\)$', a, re.S)
    if call and call.group(1).strip():
        inner = split_top(call.group(1))
        return simplify_arg(inner[-1])
    methods = re.findall(r'\.(\w+)\(\)', a)
    a = re.sub(r'(\.\w+\(\))+$', '', a)  # trailing method calls
    if methods and re.fullmatch(r'\w', a):
        return methods[0]  # `d.label()` reads better as {label} than {d}
    a = re.sub(r'\[.*?\]', '', a)
    a = a.lstrip('&*')
    parts = [p for p in re.split(r'[.:]+', a) if p]
    return parts[-1] if parts else a


def split_top(s):
    out, depth, cur = [], 0, ''
    i = 0
    while i < len(s):
        c = s[i]
        if c == '"':
            j = i + 1
            while j < len(s) and s[j] != '"':
                j += 2 if s[j] == '\\' else 1
            cur += s[i:j + 1]
            i = j + 1
            continue
        if c in '([{':
            depth += 1
        elif c in ')]}':
            depth -= 1
        if c == ',' and depth == 0:
            out.append(cur)
            cur = ''
        else:
            cur += c
        i += 1
    if cur.strip():
        out.append(cur)
    return out


def unescape(s):
    s = re.sub(r'\\\n\s*', '', s)
    return s.replace('\\"', '"').replace('\\n', ' / ')


def templates(expr):
    """Every sentence an expression can produce: format! templates with their positional
    `{}` filled by the argument's name, and plain literals."""
    out = []
    m = mask(expr)
    taken = []
    for k in re.finditer(r'format!\s*\(', m):
        o = k.end() - 1
        if any(a <= o <= b for a, b in taken):
            continue  # a format! nested inside another one
        c = match_close(m, o)
        inner = expr[o + 1:c]
        parts = split_top(inner)
        if not parts:
            continue
        lit = LIT.match(parts[0].strip())
        if not lit:
            continue
        tpl = lit.group(1)
        args = [simplify_arg(p) for p in parts[1:]]
        it = iter(args)

        def sub(mm):
            name = mm.group(1)
            if name and not name[0].isdigit():
                return '{' + name + '}'
            try:
                return '{' + next(it) + '}'
            except StopIteration:
                return '{}'
        tpl = re.sub(r'\{(\w*)(?::[^}]*)?\}', sub, tpl)
        tpl = re.sub(r'\{(\w+)\}\{\1\}', r'{\1, capitalised}', tpl)
        out.append((0, o, unescape(tpl)))
        taken.append((o, c))
    for k in LIT.finditer(expr):
        if any(a <= k.start() <= b for a, b in taken):
            continue
        out.append((1, k.start(), unescape(k.group(1))))
    out.sort()  # format! templates first, then the plain words that fill them
    seen, res = set(), []
    for _, _, t in out:
        if t not in seen:
            seen.add(t)
            res.append(t)
    return res


def variant(pat):
    """`EventKind::Died { agent, .. }` -> `Died`; `(Topic::Drink, true)` -> `Drink, true`."""
    pat = pat.strip()
    if pat.startswith('('):
        inner = pat[1:-1]
        return ', '.join(variant(p) for p in split_top(inner))
    pat = re.sub(r'\{.*\}', '', pat, flags=re.S)
    pat = re.sub(r'\(.*\)', '', pat, flags=re.S)
    return pat.strip().split('::')[-1].strip()


def md(s):
    return s.replace('|', '\\|').replace('\n', ' ')


class RustFile:
    def __init__(self, path):
        # Normalised, so it equals the os.walk path of the same file: sim('view/narrate.rs')
        # mixes separators on Windows and the coverage lookup missed it (issue 80).
        self.path = os.path.normpath(path)
        self.src = strip_tests(read(path))
        self.m = mask(self.src)
        self.covered = []  # (lo, hi) char ranges already tabled

    def fn_arms(self, fname, start=0):
        lo, hi = fn_body(self.src, self.m, fname, start)
        mlo, mhi = first_match_block(self.m, lo, hi)
        self.covered.append((lo, hi))
        return arms(self.src, self.m, mlo, mhi)

    def block_arms(self, anchor):
        """Arms of the `match` that starts at the first occurrence of `anchor`."""
        a = self.src.index(anchor)
        mlo, mhi = first_match_block(self.m, a, len(self.m))
        self.covered.append((a, mhi))
        return arms(self.src, self.m, mlo, mhi)

    def enum(self, name):
        k = re.search(r'\benum\s+' + name + r'\b', self.m)
        i = self.m.index('{', k.end())
        j = match_close(self.m, i)
        body = self.m[i + 1:j]
        out, depth, cur = [], 0, ''
        for c in body:
            if c in '({':
                depth += 1
            elif c in ')}':
                depth -= 1
            if depth == 0 and c == ',':
                out.append(cur)
                cur = ''
            elif depth == 0:
                cur += c
        out.append(cur)
        names = []
        for o in out:
            o = re.sub(r'#\[[^\]]*\]', '', o).strip()
            mm = re.match(r'(\w+)', o)
            if mm:
                names.append(mm.group(1))
        return names


def sim(p):
    return RustFile(os.path.join(SIM, p))


def table(header, rows):
    out = ['| ' + ' | '.join(header) + ' |', '|' + '---|' * len(header)]
    for r in rows:
        out.append('| ' + ' | '.join(md(str(c)) for c in r) + ' |')
    return '\n'.join(out)


def snake(name):
    return re.sub(r'(?<!^)(?=[A-Z])', '_', name).lower()


# ------------------------------------------------------------------ simulation strings
def severities():
    """EventKind -> set of severities it is emitted with, scanned from every push site."""
    sev = {}
    for root, _, names in os.walk(SIM):
        for n in names:
            if n.endswith('.rs'):
                s = strip_tests(read(os.path.join(root, n)))
                for k in re.finditer(r'Severity::(\w+)\s*,\s*(?:EventKind::|crate::events::EventKind::)(\w+)', s):
                    sev.setdefault(k.group(2), set()).add(k.group(1))
    return sev


ORDER = ['Trace', 'Minor', 'Notable', 'Major']


SQL_LITERAL = re.compile(
    r'^\s*(?:\{[A-Za-z_]\w*\}\s+)?'
    r'(?:SELECT\b|PRAGMA\b|BEGIN[ \t]*;|(?:AND|OR|WHERE)\b[\s\S]*\bSELECT\b)'
)


def inventory_literals(src, covered=()):
    """Yield line number and display text for otherwise uncatalogued Rust literals."""
    for k in LIT.finditer(src):
        s = k.group(1)
        if not re.search(r'[a-z]', s) or any(a <= k.start() <= b for a, b in covered):
            continue
        line_start = src.rfind('\n', 0, k.start()) + 1
        line_end = src.find('\n', k.start())
        line = src[line_start:line_end if line_end >= 0 else len(src)]
        if re.search(r'^\s*(//|#\[)|expect\(|assert|debug_assert|panic!|unreachable!', line):
            continue
        if re.search(r'\b(eprintln|println)!', line):
            continue
        if SQL_LITERAL.match(s) or re.match(
            r'^[A-Z][A-Z-]{3,}\??\s|^\{\w*:\?\}$|\.(bin|sqlite|json|hfsave|tmp|old)$', s
        ):
            continue  # SQL, debug traces, file names, bare Debug formats
        yield src.count('\n', 0, k.start()) + 1, unescape(s)


def strings_sim():
    files = {}
    parts = []
    narrate = sim('view/narrate.rs')
    events = sim('events.rs')
    files['narrate'] = narrate
    files['events'] = events
    # An arm that computes its name (the realm's events: realm/chronicle.rs) shows as computed.
    kinds = {variant(p): (templates(e) or ['(computed: realm/chronicle.rs)'])[0]
             for p, e, _ in events.fn_arms('kind_name')}
    sev = severities()
    rows = []
    for p, e, line in narrate.fn_arms('line'):
        v = variant(p)
        s = ', '.join(x for x in ORDER if x in sev.get(v, set())) or '(computed at the push site)'
        rows.append((v, kinds.get(v, ''), s, ' / '.join(templates(e)) or e.strip()))
    parts.append('#### Feed, history and away-summary lines (`view/narrate.rs` `line`)\n\n'
                 'One row per `EventKind`. `{name}` is filled from the event: a person\'s or '
                 'community\'s name, or the recorded field of that name. The chronicle kind is '
                 'what the away summary counts and what `ostrog run` tallies. Severity is '
                 'the level written at its push sites (a level chosen at run time shows as computed): Trace never reaches the feed, Minor is grey and '
                 'throttled, Notable is yellow, Major is red and also goes to History and the '
                 'director. Lines dated before the arrival (`Lived`) appear only in the inspect '
                 'panel and History, never in the feed.\n\n'
                 + table(['EventKind', 'chronicle kind', 'severity', 'template(s)'], rows))
    rows = [(variant(p), ' / '.join(templates(e))) for p, e, _ in narrate.fn_arms('detail')
            if templates(e)]
    parts.append('#### History detail lines (`narrate.rs` `detail`)\n\nShown indented under a '
                 'History entry.\n\n' + table(['EventKind', 'template'], rows))

    past = sim('past.rs')
    files['past'] = past
    rows = []
    for p, e, _ in past.fn_arms('narrate'):
        v = variant(p)
        if v == 'Answered':
            ans = arms(e, mask(e), *first_match_block(mask(e), 0, len(e)))
            for p2, e2, _ in ans:
                t, neg = [x.strip() for x in variant(p2).split(',')]
                rows.append((f'Answered {t} ({"stance below 0" if neg == "true" else "stance 0 or above"})',
                             ' / '.join(templates(e2))))
        else:
            rows.append((v, ' / '.join(templates(e))))
    parts.append('#### Lives before the portal (`past.rs` `narrate`)\n\n'
                 'Arrival cards show three of these; the inspect panel lists them under '
                 '"before the portal, in {home}:" prefixed "N years before the arrival: ". '
                 '`{firm}` is empty for a firm answer (|stance| > 0.5) and "half-" otherwise.\n\n'
                 + table(['Formative', 'template'], rows))

    beliefs = sim('mind/beliefs.rs')
    files['beliefs'] = beliefs
    rows = []
    for p, e, _ in beliefs.fn_arms('phrase'):
        t, neg = [x.strip() for x in variant(p).split(',')]
        rows.append((t, 'below 0' if neg == 'true' else '0 or above', templates(e)[0]))
    parts.append('#### Belief phrases (`mind/beliefs.rs` `phrase`)\n\n'
                 'Inspect shows each belief as `{phrase} ({certainty}; from {sources})`. '
                 'Certainty: "surely" (confidence > 0.7), "fairly sure" (> 0.4), otherwise '
                 '"half-believes". Sources: "what happened" / "N things that happened", '
                 '"told by {name}", "what their people held". Beliefs with |stance| x confidence '
                 'under 0.08 are not shown.\n\n'
                 + table(['topic', 'stance', 'phrase'], rows))

    tech = sim('mind/techniques.rs')
    files['tech'] = tech
    rows = [(variant(p), templates(e)[0]) for p, e, _ in tech.fn_arms('name')]
    learned = tech.enum('Learned')
    parts.append('#### Techniques (`mind/techniques.rs` `name`)\n\n'
                 'Learned by: ' + ', '.join(learned) + '. Not shown in the inspect panel yet; '
                 'their use shows in the feed (BeatOut, Doused, Tended).\n\n'
                 + table(['Technique', 'name'], rows))

    act = sim('mind/activity.rs')
    files['act'] = act
    rows = [(variant(p), ' / '.join(templates(e)) or '(the work plan\'s label, below)')
            for p, e, _ in act.fn_arms('describe')]
    parts.append('#### Activities (`mind/activity.rs` `describe`)\n\nThe second line of the '
                 'inspect panel and the hover line in the HUD. `{name}` in "heading off to" is '
                 'the next activity\'s short name: ' + ', '.join(
                     templates(e)[0] for _, e, _ in act.fn_arms('name')) + '.\n\n'
                 + table(['Activity', 'text'], rows))

    planner = sim('work/planner.rs')
    files['planner'] = planner
    rows = []
    for p, e, _ in planner.block_arms('let label = match ('):
        v = variant(p)
        if '=> match k' in e or e.strip().startswith('match'):
            inner = mask(e)
            for p2, e2, _ in arms(e, inner, *first_match_block(inner, 0, len(e))):
                rows.append((variant(p2), ' / '.join(templates(e2))))
        else:
            label = {'Haul, _, _': 'Haul', 'Cook, _, _': 'Cook',
                     '_, true, false': 'any, while only fetching materials'}.get(v, v)
            rows.append((label, ' / '.join(templates(e))))
    rows = [('any other (building)' if r[0] == '_' else r[0], r[1]) for r in rows]
    desig = sim('work/designation.rs')
    files['desig'] = desig
    drows = {variant(p): ' / '.join(templates(e)) or '(the recipe label)' for p, e, _ in desig.fn_arms('label')}
    parts.append('#### Work plan labels (`work/planner.rs`)\n\nShown as the activity of someone '
                 'working. `{label}` is the designation label (next table); for a craft it is '
                 'the recipe label.\n\n' + table(['DesignationKind', 'text'], rows)
                 + '\n\nDesignation labels (`work/designation.rs` `label`):\n\n'
                 + table(['DesignationKind', 'label'], list(drows.items())))

    th = sim('mind/thoughts.rs')
    files['th'] = th
    shape = {}
    for p, e, _ in th.fn_arms('shape'):
        nums = re.findall(r'-?\d+\.\d+', e)
        shape[variant(p)] = nums
    rows = [(variant(p), templates(e)[0], shape.get(variant(p), ['', ''])[0],
             shape.get(variant(p), ['', ''])[1]) for p, e, _ in th.fn_arms('label')]
    parts.append('#### Thoughts (`mind/thoughts.rs`)\n\nThoughts feed mood and stress. The bridge '
                 'sends the strongest six as `{label} {weight:+.2}` in `inspect()["thoughts"]`, '
                 'but the client panel does not draw them yet. LostSomeone, Quarrel and SawDeath '
                 'stack; the others refresh.\n\n'
                 + table(['ThoughtKind', 'label', 'weight', 'days'], rows))

    strain = sim('systems/strain.rs')
    files['strain'] = strain
    rows = [(variant(p), templates(e)[0]) for p, e, _ in strain.block_arms('let label = match kind')]
    parts.append('#### Breaks (`systems/strain.rs`)\n\nFeed: "{name} broke down: {kind}." with '
                 'kind from this table. A tantrum can also log "{name} smashed the {what}."\n\n'
                 + table(['Break', 'kind'], rows))

    cul = sim('culture.rs')
    files['cul'] = cul
    rows = []
    for p, e, _ in cul.fn_arms('label'):
        v = variant(p)
        if v == 'Welcome':
            inner = mask(e)
            for p2, e2, _ in arms(e, inner, *first_match_block(inner, 0, len(e))):
                rows.append(('Welcome ' + variant(p2), templates(e2)[0]))
        else:
            rows.append((v, ' / '.join(templates(e))))
    syscul = sim('systems/culture.rs')
    files['syscul'] = syscul
    narr = []
    for k in re.finditer(r'let narrative = format!\s*\(', syscul.m):
        o = k.end() - 1
        c = match_close(syscul.m, o)
        line = syscul.src.count('\n', 0, k.start()) + 1
        narr.append((f'systems/culture.rs:{line}', templates(syscul.src[k.start() + 16:c + 1])[0]))
        syscul.covered.append((k.start(), c))
    parts.append('#### Customs (`culture.rs` `label`, `systems/culture.rs` narratives)\n\n'
                 'Feed: "{community} now hold that we {label}: \\"{narrative}\\"". Inspect lists '
                 '"{label} — {narrative}", with " (not yet taken up)" for members who have not '
                 'adopted it. History: "Why they do what they do".\n\n'
                 + table(['CustomKind', 'label'], rows) + '\n\nNarratives:\n\n'
                 + table(['where', 'narrative'], narr))

    eng = sim('engine.rs')
    files['eng'] = eng
    names = {variant(p): templates(e)[0] for p, e, _ in eng.fn_arms('name')}
    blurbs = {variant(p): templates(e)[0] for p, e, _ in eng.fn_arms('blurb')}
    rows = [(k, names[k], blurbs.get(k, '')) for k in names]
    parts.append('#### The Engine (`engine.rs`)\n\nHUD: "{name} strain {percent}%  arrivals '
                 'x{rate}". Setup screen: "{Name} — {blurb}".\n\n'
                 + table(['Engine', 'name', 'blurb'], rows))

    body = sim('body.rs')
    files['body'] = body
    rows = []
    # Body parts are one vocabulary for every body plan (plan.rs, P1).
    plan = sim('plan.rs')
    files['plan'] = plan
    m = re.search(r'const PARTS[^=]*= \[(.*?)\];', plan.src, re.S)
    for k in re.finditer(r'\("(\w+)", "([^"]+)"\)', m.group(1)):
        rows.append(('PartId', k.group(1), k.group(2)))
    plan.covered.append((m.start(), m.end()))
    for fn_start, what in ((r'impl WoundKind', 'WoundKind'), (r'impl Ailment', 'Ailment')):
        st = body.src.index(fn_start)
        for p, e, _ in body.fn_arms('label', st):
            rows.append((what, variant(p), templates(e)[0]))
    parts.append('#### Body (`body.rs`, parts from `plan.rs`)\n\nFeed: "{name} took a {kind} to the {part} {cause}.", '
                 '"{name}\'s {part} has healed.", "{name} now has {ailment}."\n\n'
                 + table(['enum', 'variant', 'label'], rows))

    items = sim('items.rs')
    files['items'] = items
    rows = [(variant(p), snake(variant(p)), templates(e)[0]) for p, e, _ in items.fn_arms('label')]
    tiers = [(variant(p), templates(e)[0]) for p, e, _ in items.fn_arms('tier_name')]
    parts.append('#### Items (`items.rs`)\n\nIcon file is `godot/assets/items/{file}.png`. '
                 'Quality tiers 0 to 5 by name: '
                 + ', '.join(f'{t if t != "_" else "5"} {n}' for t, n in tiers) + '.\n\n'
                 + table(['ItemKind', 'file', 'label in text'], rows))

    rec = sim('work/recipes.rs')
    files['rec'] = rec
    rows = []
    for k in re.finditer(r'Recipe\s*\{', rec.m):
        i = rec.m.index('{', k.start())
        j = match_close(rec.m, i)
        b = rec.src[i:j]
        if 'label: "' not in b:
            continue
        g = lambda f: re.search(f + r':\s*([^,\n]+(?:\[[^\]]*\])?)', b).group(1).strip()
        inputs = re.search(r'inputs:\s*&\[(.*?)\],\s*\n', b, re.S).group(1)
        wm = re.search(r'wants:\s*&\[(.*?)\]', b, re.S)
        wants = re.findall(r'(None|Some\(M::(\w+)\))', wm.group(1)) if wm else []
        pairs = re.findall(r'ItemKind::(\w+),\s*(\d+)', inputs)
        ins = ', '.join(
            f'{n} {snake(kk)}' + (f' ({wants[ix][1].lower().replace("_", " ")})'
                                  if ix < len(wants) and wants[ix][1] else '')
            for ix, (kk, n) in enumerate(pairs))
        out = re.search(r'output:\s*\(ItemKind::(\w+),\s*(\d+)\)', b)
        rows.append((LIT.search(b).group(1), snake(g('station').split('::')[-1]), ins,
                     f'{out.group(2)} {snake(out.group(1))}', g('skill').split('::')[-1], g('minutes')))
    rec.covered.append((0, len(rec.src)))
    parts.append('#### Recipes (`work/recipes.rs`)\n\nFeed: "{name} made {tier} {item label}." '
                 'Work label: "making {recipe label}".\n\n'
                 + table(['recipe label', 'station', 'inputs', 'output', 'skill', 'game-minutes'], rows))

    mat = sim('world/material.rs')
    files['mat'] = mat
    rows = []
    for k in re.finditer(r'd\(\s*M::(\w+),\s*"([^"]*)",\s*(\w+),\s*([\d.]+),\s*(None|Some\(M::(\w+)\)),\s*"([^"]*)"', mat.src):
        rows.append((k.group(1), k.group(2), k.group(3), k.group(4), (k.group(6) or '').lower(), k.group(7)))
    ids = {n: v for n, v in re.findall(r'pub const (\w+): Material = Material\((\d+)\)', mat.src)}
    rows = [(ids.get(r[0], ''),) + r for r in rows]
    mat.covered.append((0, len(mat.src)))
    parts.append('#### Materials (`world/material.rs`)\n\nThe id is the low 16 bits of a packed '
                 'tile; the family names the terrain sheet (`FAMILY` in world_view.gd maps ids to '
                 'sheets, with built materials drawn as walls).\n\n'
                 + table(['id', 'const', 'name', 'kind', 'hardness', 'yields', 'family'], rows))

    soc = sim('society.rs')
    tr = sim('mind/traits.rs')
    sk = sim('mind/skills.rs')
    mem = sim('mind/memory.rs')
    agents = sim('agents.rs')
    plants = sim('plants.rs')
    structs = sim('structures.rs')
    rows = [
        ('Role (society.rs)', ', '.join(soc.enum('Role'))),
        ('WelcomeNorm (society.rs)', ', '.join(soc.enum('WelcomeNorm'))),
        ('Trait (mind/traits.rs)', ', '.join(tr.enum('Trait'))),
        ('Skill (mind/skills.rs)', ', '.join(sk.enum('Skill'))),
        ('Topic (past.rs)', ', '.join(past.enum('Topic'))),
        ('Formative (past.rs)', ', '.join(past.enum('Formative'))),
        ('MemKind (mind/memory.rs)', ', '.join(mem.enum('MemKind'))),
        ('Species (agents.rs)', ', '.join(agents.enum('Species'))),
        ('PlantKind (plants.rs)', ', '.join(plants.enum('PlantKind'))),
        ('StructureKind (structures.rs)', ', '.join(structs.enum('StructureKind'))),
    ]
    parts.append('#### Enum words shown as they are\n\nThe inspect panel and arrival cards print '
                 'traits and skills by their Rust names (e.g. "LightSleeper", "Construction 7"); '
                 'roles in lower case; memories as "{MemKind} {subject} ({fond|sour|plain}, '
                 '{strength}%)".\n\n' + table(['enum', 'variants, in order'], rows))

    nm = sim('names.rs')
    rows = []
    for k in re.finditer(r'const (\w+): &\[&str\] = &\[(.*?)\];', nm.src, re.S):
        rows.append((k.group(1), ' '.join(LIT.findall(k.group(2)))))
    nm.covered.append((0, len(nm.src)))
    parts.append('#### Name parts (`names.rs`)\n\nPeople, valleys and communities are named from '
                 'these syllables by seed, so any name on screen is made of them.\n\n'
                 + table(['array', 'parts'], rows))

    # Every other literal, so anything on screen can be traced.
    skip_files = {'events.rs', 'systems/mod.rs', 'chronicle/mod.rs', 'digest.rs', 'rng.rs'}
    covered = {}
    for f in files.values():
        covered.setdefault(f.path, []).extend(f.covered)
    for f in (nm, mat, rec):
        covered.setdefault(f.path, []).extend(f.covered)
    rows = []
    for root, _, names in sorted(os.walk(SIM), key=lambda t: t[0].replace(os.sep, '/')):
        for n in sorted(names):
            if not n.endswith('.rs'):
                continue
            p = os.path.join(root, n)
            r = os.path.relpath(p, SIM).replace(os.sep, '/')
            if r in skip_files:
                continue
            src = strip_tests(read(p))
            cov = covered.get(os.path.normpath(p), [])
            rows.extend((f'{r}:{ln}', display) for ln, display in inventory_literals(src, cov))
    parts.append('#### Every other string in the simulation\n\nAll remaining string literals in '
                 '`crates/hf-sim/src` outside tests, panics and the tables above: death causes, '
                 'fire causes, plan-failure reasons, caravan purposes and road stories, choices, '
                 'search notes, save errors and so on. If a line on screen is in none of the '
                 'tables above, search this one for its fixed words.\n\n<details><summary>'
                 f'{len(rows)} strings (click to open)</summary>\n\n'
                 + table(['file:line', 'string'], rows) + '\n\n</details>')
    return '\n\n'.join(parts)


# ------------------------------------------------------------------ client strings
GD_SKIP_BEFORE = re.compile(r'(\[|\bget\(|\bhas\(|begins_with\(|ends_with\(|preload\(|\bload\(|_tex\(|_fx\(|'
                            r'get_node\(|_stream\(|_override\(|get_bus_index\(|path_join\(|'
                            r'\bmatch\b|\bin\s|==\s*|!=\s*|connect\(|\.name\s*=\s*)\s*$')


def strings_client():
    rows_screen, rows_console = [], []
    for name in ['main.gd', 'setup.gd', 'world_view.gd', 'persist.gd', 'audio/sound.gd',
                 'render/entity_layer.gd', 'render/light_layer.gd', 'render/weather_layer.gd',
                 'render/fire_fx.gd', 'render/overlay_layer.gd']:
        p = os.path.join(ROOT, 'godot', 'scripts', name)
        if not os.path.exists(p):
            continue
        func = ''
        for ln, line in enumerate(read(p).split('\n'), 1):
            fm = re.match(r'\s*func\s+(\w+)', line)
            if fm:
                func = fm.group(1)
            code = line.split('#', 1)[0] if '"' not in line.split('#', 1)[0] else line
            if code.lstrip().startswith('#'):
                continue
            for k in LIT.finditer(code):
                s = k.group(1)
                before = code[:k.start()]
                if not re.search(r'[A-Za-z]', s) or GD_SKIP_BEFORE.search(before):
                    continue
                if re.match(r'^(res|user)://|^--|^[a-z_]+$|^#[0-9a-f]{3,8}$', s) or \
                        re.search(r'\.(png|ogg|json|gd|gdshader|hfsave)\b|\|', s):
                    continue  # paths, keys, colours, cache keys
                words = re.sub(r'%[-+\d.]*[sdf]|\[/?\w+(=[^\]]*)?\]|\\n', ' ', s)
                if not re.search(r'[A-Za-z]{2,}', words) or re.search(r'bus\s*=\s*$', before):
                    continue
                row = (f'godot/scripts/{name}:{ln}', func, unescape(s))
                (rows_console if 'print(' in before else rows_screen).append(row)
    return ('#### On screen\n\nHUD, feed, inspect panel, History, away summary, settings and the '
            'arrival screen. `%s`, `%d` and `%.2f` are filled at run time; `[b]` and '
            '`[color=...]` are BBCode.\n\n<details><summary>'
            f'{len(rows_screen)} strings (click to open)</summary>\n\n'
            + table(['file:line', 'function', 'string'], rows_screen)
            + '\n\n</details>\n\n#### Console (stdout)\n\nWhat the client prints; scripts and '
            'testers grep for these (SHOT_SAVED, SHOT_FAILED, UNKNOWN_FLAG, SAVE, LOAD, '
            'IGNITE, FOOTAGE_FRAME, FOOTAGE_DONE, WIZARD_SHOT).\n\n'
            + table(['file:line', 'function', 'string'], rows_console))


# ------------------------------------------------------------------ assets
def text_size(p):
    """Bytes with LF line ends, so a CRLF checkout reports the same size (issue 80)."""
    with open(p, 'rb') as f:
        return len(f.read().replace(b'\r\n', b'\n'))


def png_size(p):
    with open(p, 'rb') as f:
        h = f.read(24)
    return struct.unpack('>II', h[16:24])


def provenance():
    gen = {}
    # Every provenance table in tools/, so a new generator's table is never missed.
    for t in sorted(os.path.relpath(p, ROOT) for p in glob.glob(os.path.join(ROOT, 'tools', '**', 'provenance*.toml'), recursive=True)):
        p = os.path.join(ROOT, t)
        if not os.path.exists(p):
            continue
        with open(p, 'rb') as f:
            d = tomllib.load(f)
        for a in d.get('asset', []):
            gen[a['path']] = (a.get('generator') or a.get('url') or '', a.get('licence', ''), t)
    return gen


def layout(folder, name, w, h):
    """Frame layout and meaning for one sheet, from the conventions the client draws by."""
    T = 32
    base = name[:-4]
    if folder == 'terrain':
        if base in ('water', 'deep_water'):
            return f'{w // T} frames x {h // T} rows (animated; frame = time, row = variant)'
        return f'{w // T} variants in a row (picked per tile by position)'
    if folder == 'structures':
        if base.startswith('wall_'):
            return '16 wall-top pieces, index = neighbours N=1 E=2 S=4 W=8'
        if base == 'fire':
            return f'{w // T} frames, cycled on every burning tile'
        if base == 'lamp':
            return '2 frames: unlit, lit (the lit frame shows while the lamp is active)'
        if base in ('smelter', 'furnace', 'kiln'):
            return '2 frames: cold, lit (the lit frame shows while the structure is active)'
        if w > T:
            return f'{w // T} frames, cycled while the structure is active'
        return 'single image' + (f', {h} px tall' if h != T else '')
    if folder == 'plants':
        if re.match(r'(oak|birch|pine)_\d', base):
            stage = int(re.search(r'_(\d)', base).group(1))
            season = 'autumn' if base.endswith('_autumn') else ('winter' if base.endswith('_winter') else 'spring/summer')
            return f'tree stage {stage} ({["sapling", "young", "mature"][stage]}), {season}'
        if w > T:
            return f'{w // T} variants in a row'
        m = re.match(r'(grain|root)_(\d)', base)
        if m:
            return f'growth stage {m.group(2)} of 0..3'
        return 'single image'
    if folder == 'people' and base.startswith(('expressive_', 'overlay_bruise_')):
        cell = h // 4
        return f'{w // cell} frames x 4 directions; {cell} px per 48-unit logical cell'
    if folder == 'people':
        return f'{w // T} columns x {h // T} rows (rows S, W, E, N)'
    if folder == 'animals':
        return f'{w // T} columns x 4 rows of {T}x{h // 4} (rows S, W, E, N)'
    if folder == 'overlays':
        if w > T:
            return f'{w // T} pieces of {T}x{h}'
        return f'south face strip {w}x{h}'
    if folder == 'items':
        return '16x16 icon'
    return ''


def client_arrays():
    src = read(os.path.join(ROOT, 'godot', 'scripts', 'world_view.gd'))
    arrs = {}
    for k in re.finditer(r'const (\w+) := \[(.*?)\]', src, re.S):
        arrs[k.group(1)] = LIT.findall(k.group(2))
    return arrs


ALIASES = {'reed': 'reeds', 'mushroom': 'mushrooms'}


def assets():
    gen = provenance()
    used = set(gen)
    parts = []
    for folder in ['terrain', 'overlays', 'structures', 'plants', 'items', 'people', 'animals', 'light', 'palette']:
        d = os.path.join(ASSETS, folder)
        rows = []
        for n in sorted(os.listdir(d)):
            if n.endswith('.import'):
                continue
            p = os.path.join(d, n)
            r = rel(p)
            g = gen.get(r, ('tools/art/palette.py (data file, no provenance row)' if folder == 'palette'
                            else '(no provenance row)', '', ''))
            if n.endswith('.png'):
                w, h = png_size(p)
                rows.append((n, f'{w}x{h}', layout(folder, n, w, h), g[0]))
            elif os.path.isdir(p):
                # A folder's own byte size is the filesystem's (4096 on ext4, 0 on Windows).
                k = sum(1 for m in os.listdir(p) if not m.endswith('.import'))
                rows.append((n, f'{k} files', 'folder', g[0]))
            else:
                rows.append((n, f'{text_size(p)} bytes', 'data', g[0]))
        parts.append(f'#### godot/assets/{folder}/\n\n<details><summary>{len(rows)} file{"s" if len(rows) != 1 else ""} '
                     '(click to open)</summary>\n\n'
                     + table(['file', 'pixels', 'layout', 'generator'], rows) + '\n\n</details>')
    man = json.loads(read(os.path.join(ASSETS, 'audio', 'sounds.json')))
    rows = [(k, v.get('bus', ''), 'yes' if v.get('loop') else 'no', len(v.get('files', [])),
             ', '.join(v.get('files', []))) for k, v in sorted(man.items())]
    parts.append('#### godot/assets/audio/sounds.json (' + str(len(man)) + ' sounds, '
                 + str(sum(len(v.get('files', [])) for v in man.values())) + ' files, all from '
                 '`tools/audio/sfx.py`)\n\n'
                 + table(['sound', 'bus', 'loops', 'variants', 'files'], rows))

    arrs = client_arrays()
    checks = [('ITEM_NAME', 'items.rs', 'ItemKind'), ('STRUCT_NAME', 'structures.rs', 'StructureKind'),
              ('PLANT_KIND', 'plants.rs', 'PlantKind'), ('ANIMAL_NAME', 'agents.rs', 'Species')]
    rows = []
    for arr, f, en in checks:
        variants = sim(f).enum(en)
        names = arrs.get(arr, [])
        for i in range(max(len(variants), len(names))):
            v = variants[i] if i < len(variants) else '(none)'
            n = names[i] if i < len(names) else '(none)'
            ok = ALIASES.get(snake(v), snake(v)) == n
            if not ok:
                WARNINGS.append(f'{arr}[{i}] is "{n}" but {en} variant {i} is {v}')
            rows.append((arr, i, v, n, 'ok' if ok else 'MISMATCH'))
    for arr in ['OUTFIT', 'BUILD', 'EQUIPMENT', 'SNOWABLE']:
        rows.append((arr, '', '', ' '.join(arrs.get(arr, [])), 'client only'))
    parts.append('#### Simulation enum to sprite name arrays (`godot/scripts/world_view.gd`)\n\n'
                 'The bridge sends enum values as integers and the client looks the name up by '
                 'index, so each array must list names in the same order as its Rust enum. '
                 'New variants are appended to both. This table is checked when it is generated.\n\n'
                 + table(['array', 'index', 'Rust variant', 'client name', 'check'], rows))
    missing = sorted(set(rel(os.path.join(r, n)) for r, _, ns in os.walk(ASSETS) for n in ns
                         if (n.endswith('.png') or n.endswith('.ogg'))) - used)
    if missing:
        WARNINGS.append('assets without a provenance row: ' + ', '.join(missing))
    return '\n\n'.join(parts)



# ------------------------------------------------------------------ bridge and pipeline
AGENT_MEANING = {
    'a.id.0 as i32': 'agent id (the number `--inspect=ID` takes)',
    'a.pos.x': 'tile x now', 'a.pos.y': 'tile y now', 'a.pos.z': 'level z now',
    'from.z': 'The level the step began on (#324): a change of level is drawn as a climb or a drop',
    'emote': 'activity emote (#192, WP-08 8.4): 0 none, else an item kind + 1 (the work\'s own tool icon over the head, entity_layer.gd, at 1.75x zoom or closer)',
    'a.species as i32': 'Species index (0 a person, then animals; ANIMAL_NAME order)',
    'shape': 'Species index as drawn (0 a person, then animals; ANIMAL_NAME order); a person in the Beast-hunger\'s shape is sent as a wolf until dawn (C75)',
    'rendered_shape(w, a) as i32': 'Species index as drawn (0 a person, then animals; ANIMAL_NAME order); a person in the Beast-hunger\'s shape is sent as a wolf until dawn (C75); pose selection uses this same shape (#485)',
    'a.facing as i32': 'facing 0 S, 1 W, 2 E, 3 N (the sheet row)',
    'lk.body as i32': 'build 0 slim, 1 average, 2 broad (BUILD)',
    'lk.skin as i32': 'skin swap target (species.rs SKIN_NAMES: six human tones, then the species\' own)',
    'lk.hair as i32': 'hair style 0..9 (file hair_01..hair_10)',
    'lk.hair_colour as i32': 'hair swap target 0..7 (5 grey and 6 white are used for elders)',
    'lk.clothing as i32': 'outfit 0..7 (OUTFIT order)',
    'lk.clothing_colour as i32': 'cloth swap target 0..11',
    'from.x': 'tile x moving from (a body: where it lies)', 'from.y': 'tile y moving from (a body: where it lies)',
    'step': 'progress of the current step, 0..1000 (the client interpolates from -> now; 1000 for a body)',
    'pose': 'pose 0 idle, 1 walking, 2 working, 3 carrying, 4 sleeping, 5 sitting (animal eating), 6 fighting, 7 struck, 8 falling dead, 9 lying dead (view::bodies, #185), 10 talking people, 11 eating people (#485)',
    'lk.child as i32': '1 if a child (child body sheet)',
    'lk.injured as i32': '1 if injured (bandage overlay)',
    'lk.elder as i32': '1 if an elder (sent; the client does not read it, hair colour already shows age)',
    'lk.equipment as i32': 'equipment -1 none, else EQUIPMENT index (hidden while carrying an item)',
    'sound': 'work sound 0 none, 1 chop, 2 mine, 3 hammer, 4 hoe, 5 saw, 6 cook, 7 water (dousing); sound.gd plays each as its sound, 7 as douse',
    'band': 'one of a band of strangers (threat.rs): 0 no, 1 yes, 2 out after food; entity_layer.gd draws them dust-tinted',
    'a.people.0 as i32': 'the person\'s species in the library (species.rs; SimHost.species_art says how each is drawn)',
}


def bridge():
    f = RustFile(os.path.join(ROOT, 'crates', 'hf-godot', 'src', 'lib.rs'))
    lo, hi = fn_body(f.src, f.m, 'agents')
    k = f.m.rfind('for v in [', lo, hi)
    if k >= 0:
        i = f.m.index('[', k)
    else:
        # The emitted row may be assembled by agent_row so bridge/render integration tests can
        # inspect it without constructing a Godot node.
        lo, hi = fn_body(f.src, f.m, 'agent_row')
        i = f.m.rfind('[', lo, hi)
        if i < 0:
            raise SystemExit('agents()/agent_row has no emitted field array')
    j = match_close(f.m, i)
    fields = [x.strip() for x in split_top(f.src[i + 1:j])]
    rows = []
    for n, e in enumerate(fields):
        meaning = AGENT_MEANING.get(e)
        if meaning is None:
            WARNINGS.append(f'agents() field {n} `{e}` has no meaning in tools/docs/readme_tables.py')
            meaning = '(undocumented: add it to AGENT_MEANING)'
        rows.append((n, f'`{e}`', meaning))
    stride = re.search(r'const AGENT_STRIDE := (\d+)', read(os.path.join(ROOT, 'godot', 'scripts', 'world_view.gd')))
    if stride and int(stride.group(1)) != len(fields):
        WARNINGS.append(f'AGENT_STRIDE is {stride.group(1)} but agents() sends {len(fields)} fields')
    return (f'`SimHost.agents()` sends {len(fields)} ints per living agent (humans and animals); '
            f'`AGENT_STRIDE` in world_view.gd is {stride.group(1) if stride else "?"}.\n\n'
            + table(['index', 'expression in lib.rs', 'meaning'], rows))


def pipeline():
    src = read(os.path.join(SIM, 'systems', 'mod.rs'))
    notes, prev = {}, None
    for k in re.finditer(r'pub mod (\w+);[ \t]*(?://[ \t]*(.*))?', src):
        text = (k.group(2) or '').strip()
        if '//' in text:  # a comment that drifted onto the next line's declaration
            text, stray = [t.strip() for t in text.split('//', 1)]
            if prev and not notes.get(prev):
                notes[prev] = stray
        notes[k.group(1)] = text
        prev = k.group(1)
    body = src[src.index('pub const PIPELINE'):]
    body = body[:body.index('];')]
    rows = []
    for n, k in enumerate(re.finditer(r'\("(\w+)",\s*([\w:]+)\),?\s*(?://\s*(.*))?', body), 1):
        name, fn = k.group(1), k.group(2)
        mod = fn.split('::')[-2] if '::' in fn else ''
        note = k.group(3) or notes.get(mod, '')
        note = re.sub(r'\s*//.*', '', note)
        rows.append((n, name, f'`{fn}`', note))
    return table(['#', 'stage', 'function', 'what it does'], rows)


# ------------------------------------------------------------------ main
def replace_block(text, name, body):
    b = f'<!-- BEGIN GENERATED: {name} -->'
    e = f'<!-- END GENERATED: {name} -->'
    i, j = text.find(b), text.find(e)
    if i < 0 or j < 0:
        raise SystemExit(f'docs/REFERENCE.md has no {name} block markers')
    return text[:i + len(b)] + '\n<!-- Written by tools/docs/readme_tables.py; edit the code, then rerun it. -->\n\n' + body + '\n\n' + text[j:]


def main():
    text = read(README)
    new = text
    new = replace_block(new, 'strings-sim', strings_sim())
    new = replace_block(new, 'strings-client', strings_client())
    new = replace_block(new, 'assets', assets())
    new = replace_block(new, 'bridge', bridge())
    new = replace_block(new, 'pipeline', pipeline())
    for w in WARNINGS:
        print('WARNING:', w, file=sys.stderr)
    if '--check' in sys.argv:
        if new != text or WARNINGS:
            print('docs/REFERENCE.md generated tables are out of date: run python3 tools/docs/readme_tables.py', file=sys.stderr)
            sys.exit(1)
        print('README tables up to date')
        return
    if new != text:
        with open(README, 'w', encoding='utf-8') as f:
            f.write(new)
        print('docs/REFERENCE.md tables rewritten')
    else:
        print('docs/REFERENCE.md tables already up to date')


if __name__ == '__main__':
    main()
