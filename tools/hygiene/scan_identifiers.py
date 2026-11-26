#!/usr/bin/env python3
"""Fail when added lines (or a PR title/body) contain identifying details: email addresses, IP
addresses, home-directory paths, dynamic-DNS hostnames, SSH logins, or tokens.

Use placeholders in docs, tests and examples: user@example.com, 192.0.2.1, /home/user,
C:\\Users\\<user>, host.example. Inline suppression markers do not bypass these checks.

  scan_identifiers.py --range BASE...HEAD     added lines of a git range (CI)
  scan_identifiers.py --staged                added lines of the staged changes (pre-commit)
  scan_identifiers.py --text FILE|-           a PR/issue title and body or any text
  scan_identifiers.py --tree                  every line of every tracked file (audit; not run in CI)
  scan_identifiers.py --commits BASE..HEAD    commit messages, authors and committers of a range
  scan_identifiers.py --branch NAME           a pull request's head branch name
What is enforced is listed in docs/HYGIENE.md.
Public CI uses generic rules only, with no confidential inputs or inline exemptions.
Private artifact auditing belongs in a pinned external offline process.
"""
import os
import re
import hashlib
import subprocess
import sys

SKIP_PATHS = ("tools/hygiene/", "Cargo.lock", "docs/HYGIENE.md", "tools/tests/hygiene_rules_guard_test.py")
SKIP_SUFFIX = (".png", ".jpg", ".ogg", ".zip", ".sqlite", ".hfsave", ".lock")

OK_EMAIL = re.compile(r"(@users\.noreply\.github\.com|^support@github\.com$|@example\.(com|org|net)|@[\w.-]*\.(example|invalid|test|local)$|@localhost)", re.I)
RULES = [
    ("email", re.compile(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)+"), lambda m: not OK_EMAIL.search(m.group(0)) and not re.search(r"\.(png|jpg|jpeg|gif|webp|svg|ogg)$", m.group(0), re.I) and not re.search(r"@\d+x(\.|$)", m.group(0))),
    ("ipv4", re.compile(r"(?<![\w.])((25[0-5]|2[0-4]\d|1?\d?\d)\.){3}(25[0-5]|2[0-4]\d|1?\d?\d)(?![\w.])"), lambda m: not re.match(r"(127\.|0\.0\.0\.0|255\.|192\.0\.2\.|198\.51\.100\.|203\.0\.113\.)", m.group(0))),
    ("ipv6", re.compile(r"(?<![\w:])([0-9a-fA-F]{1,4}:){4,7}[0-9a-fA-F]{1,4}(?![\w:])"), lambda m: not m.group(0).lower().startswith("2001:db8")),
    ("home path", re.compile(r"(/home/|/Users/)([A-Za-z0-9._-]+)|[A-Za-z]:\\Users\\([A-Za-z0-9 ._<>-]+)"), lambda m: (m.group(2) or m.group(3) or "").lower() not in ("user", "runner", "<user>", "you", "name", "yourname", "shared", "public", "default", "$user", "${user}", "me") and not (m.group(0).endswith("<") or (m.group(3) or "").startswith("<"))),
    ("hostname", re.compile(r"\b[\w-]+(\.[\w-]+)*\.(sslip\.io|nip\.io|duckdns\.org|ngrok(-free)?\.(io|app)|ts\.net|dyndns\.\w+|no-ip\.\w+)\b", re.I), lambda m: True),
    ("ssh login", re.compile(r"\bssh\s+(-\w+\s+\S+\s+)*[A-Za-z0-9._-]+@[A-Za-z0-9.-]+"), lambda m: "example" not in m.group(0)),
    ("token", re.compile(r"(gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{30,}|sk-[A-Za-z0-9_-]{24,}|AKIA[0-9A-Z]{16}|xox[baprs]-[A-Za-z0-9-]{10,}|-----BEGIN [A-Z ]*PRIVATE KEY-----|AIza[0-9A-Za-z_-]{35})"), lambda m: True),
]

# Words that must not appear in the repository are matched as salted SHA-256 hashes of whole
# lowercase words, so no real name is committed here. A word is letters and digits; "x's" is "x".
NAME_SALT = "ostrog-hygiene:"
NAME_HASHES = {
    "d0e107b61274423c34874b98d4bb58e505596070b2dac6959702790016632aac",
    "5980a4640ec8ced23659d8847f356cbfe07893cca638bd516a47e6d2db84f10b",
}
NAME_ALLOWED_PATHS = ("NOTICE", "LICENSE-ASSETS")
WORD = re.compile(r"[A-Za-z0-9]+")

def has_owner_name(line):
    return any(hashlib.sha256((NAME_SALT + w.lower()).encode()).hexdigest() in NAME_HASHES
               for w in WORD.findall(line))

# Tool and vendor names are assembled from pieces so this file does not contain them itself.
_AI = ["cla" + "ude", "anthro" + "pic", "chat" + "gpt", "open" + "ai", "co" + "pilot", "co" + "dex",
       "gem" + "ini[ -]?(?:cli|ai|pro|flash)"]
AI_BRAND = re.compile(r"(?<![A-Za-z])(?:" + "|".join(_AI) + r")(?![a-z])", re.I)
ROBOT = re.compile("\U0001F916")
# The private working repository's name. The public development repository's own name is fine.
# Only "<owner login>/<private name>" counts (the owner login is matched by hash), or a github.com URL.
PRIVATE_REPO = re.compile(r"\b([\w.-]+)/(?:ostrog(?![\w.-])|ostrog-dev(?![\w.-])|hearth" + "fall(?![\\w.-]))", re.I)
GITHUB_PRIVATE = re.compile(r"github\.com/[\w.-]+/(?:ostrog(?![\w.-])|ostrog-dev(?![\w.-])|hearth" + "fall(?![\\w.-]))", re.I)
LOGIN_HASH = "02956b4e38bb77dba47ba3b0d70d42daa4f385d3462d60bfb3254ab75ba90561"
THE_BOX = re.compile(r"\bthe box\b(?![\w-])", re.I)  # "the box-ci" is a status name, not a nickname
FOOTER = re.compile(r"generated (?:with|by)\b.*\b(?:" + "|".join(_AI[:2]) + r")|-projects-attribution|requested by \*\*|project thread|\b(?:cmsg|chan|session|env)_(?=[0-9A-Za-z]*\d)[0-9A-Za-z]{16,}\b", re.I)
AI_TRAILER = re.compile(r"^\s*(?:co-authored-by|claude-session|generated-by|assisted-by)\s*:", re.I)
BAD_BRANCH = re.compile(r"^(?:" + "|".join(["cla" + "ude", "co" + "pilot", "co" + "dex", "gem" + "ini"]) + r")/", re.I)

def content_rules(path, line):
    """Rule names broken by one added file line (path-aware)."""
    out = []
    if AI_BRAND.search(line) or ROBOT.search(line):
        out.append("AI tool or vendor name")
    if GITHUB_PRIVATE.search(line) or any(
            hashlib.sha256((NAME_SALT + m.group(1).lower()).encode()).hexdigest() == LOGIN_HASH
            for m in PRIVATE_REPO.finditer(line)):
        out.append("private repository name")
    if THE_BOX.search(line) and not path.endswith((".gd", ".tscn")):
        out.append('"the box" (name the role, e.g. "the build server")')
    if path not in NAME_ALLOWED_PATHS and has_owner_name(line):
        out.append("owner's personal name (say 'the maintainer')")
    return out

def text_rules(line):
    out = content_rules("", line)
    if FOOTER.search(line):
        out.append("tool footer, attribution comment, 'Requested by' line or private thread/session link")
    return out

def check(label, lineno, line, hits, extra=()):
    for name, rx, keep in RULES:
        for m in rx.finditer(line):
            if keep(m):
                hits.append(f"{label}:{lineno}: {name}")
                break
    for name in extra:
        hits.append(f"{label}:{lineno}: {name}")

def check_commits(rng, hits):
    fmt = "%H%x1f%an%x1f%ae%x1f%cn%x1f%ce%x1f%P%x1f%B%x1e"
    raw = subprocess.run(["git", "log", "--no-color", "--format=" + fmt, rng],
                         capture_output=True, text=True, errors="replace", check=True).stdout
    for rec in (r for r in raw.split("\x1e") if r.strip()):
        sha, an, ae, cn, ce, parents, body = (rec.strip("\n").split("\x1f") + [""] * 7)[:7]
        c = sha[:9]
        merge = len(parents.split()) > 1  # merge commits (incl. the web "Update branch"/merge button) carry the clicker's identity
        for who, name, mail in (() if merge else (("author", an, ae), ("committer", cn, ce))):
            if not re.search(r"@users\.noreply\.github\.com$|^noreply@github\.com$|@example\.(com|org|net)$|@[\w.-]*\.(example|invalid|test)$", mail, re.I):
                hits.append(f"commit {c}: {who} email is not a noreply address (use the neutral identity)")
            if has_owner_name(name) or AI_BRAND.search(name):
                hits.append(f"commit {c}: {who} name is not neutral (use the neutral identity)")
        for i, line in enumerate(body.splitlines(), 1):
            if AI_TRAILER.search(line):
                hits.append(f"commit {c} message line {i}: AI or co-author trailer (remove it)")
            else:
                check(f"commit {c} message", i, line, hits, text_rules(line))

def added_lines(diff):
    path, n = "?", 0
    for raw in diff.splitlines():
        if raw.startswith("+++ "):
            path = raw[6:] if raw.startswith("+++ b/") else raw[4:]
        elif raw.startswith("@@"):
            m = re.search(r"\+(\d+)", raw)
            n = int(m.group(1)) - 1 if m else 0
        elif raw.startswith("+") and not raw.startswith("+++"):
            n += 1
            yield path, n, raw[1:]
        elif not raw.startswith("-"):
            n += 1

def main():
    a = sys.argv[1:]
    if "--require-blocklist" in a:
        sys.exit("private audit unavailable in public generic scanner")
    hits = []
    if a[:1] == ["--text"]:
        text = sys.stdin.read() if a[1] == "-" else open(a[1], encoding="utf-8", errors="replace").read()
        for i, line in enumerate(text.splitlines(), 1):
            check(a[1] if a[1] != "-" else "text", i, line, hits, text_rules(line))
    elif a[:1] == ["--tree"]:
        for path in subprocess.run(["git", "ls-files"], capture_output=True, text=True, check=True).stdout.splitlines():
            if path.startswith(SKIP_PATHS) or path.endswith(SKIP_SUFFIX):
                continue
            try:
                lines = open(path, encoding="utf-8", errors="strict").read().splitlines()
            except (UnicodeDecodeError, OSError):
                continue
            for n, line in enumerate(lines, 1):
                check(path, n, line, hits, content_rules(path, line))
    elif a[:1] == ["--commits"]:
        check_commits(a[1], hits)
    elif a[:1] == ["--branch"]:
        if BAD_BRANCH.search(a[1]):
            hits.append("branch name: starts with a tool prefix (rename the branch, e.g. fix/<topic>)")
    else:
        cmd = ["git", "diff", "--no-color", "-U0", "--no-ext-diff"]
        cmd += ["--cached"] if a[:1] == ["--staged"] else [a[1]] if a[:1] == ["--range"] else sys.exit(__doc__)
        diff = subprocess.run(cmd, capture_output=True, text=True, errors="replace", check=True).stdout
        for path, n, line in added_lines(diff):
            if path.startswith(SKIP_PATHS) or path.endswith(SKIP_SUFFIX):
                continue
            check(path, n, line, hits, content_rules(path, line))
    if hits:
        print("Hygiene check failed. Remove or reword what is named below (matched text is not shown).")
        print("Use placeholders such as user@example.com, 192.0.2.1, /home/user, host.example. See docs/HYGIENE.md.")
        print("\n".join(sorted(set(hits))[:200]))
        sys.exit(1)

main()
