#!/usr/bin/env python3
"""Fail when added lines (or a PR title/body) contain identifying details: email addresses, IP
addresses, home-directory paths, dynamic-DNS hostnames, SSH logins, or tokens.

Use placeholders in docs, tests and examples: user@example.com, 192.0.2.1, /home/user,
C:\\Users\\<user>, host.example. Inline suppression markers do not bypass these checks.

  scan_identifiers.py --range BASE...HEAD     added lines of a git range (CI)
  scan_identifiers.py --staged                added lines of the staged changes (pre-commit)
  scan_identifiers.py --text FILE|-           a PR title/body or any text
Public CI uses generic rules only, with no confidential inputs or inline exemptions.
Private artifact auditing belongs in a pinned external offline process.
"""
import os
import re
import subprocess
import sys

SKIP_PATHS = ("tools/hygiene/", "Cargo.lock")
SKIP_SUFFIX = (".png", ".jpg", ".ogg", ".zip", ".sqlite", ".hfsave", ".lock")

OK_EMAIL = re.compile(r"(@users\.noreply\.github\.com|@example\.(com|org|net)|@[\w.-]*\.(example|invalid|test|local)$|@localhost)", re.I)
RULES = [
    ("email", re.compile(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)+"), lambda m: not OK_EMAIL.search(m.group(0)) and not re.search(r"\.(png|jpg|jpeg|gif|webp|svg|ogg)$", m.group(0), re.I) and not re.search(r"@\d+x(\.|$)", m.group(0))),
    ("ipv4", re.compile(r"(?<![\w.])((25[0-5]|2[0-4]\d|1?\d?\d)\.){3}(25[0-5]|2[0-4]\d|1?\d?\d)(?![\w.])"), lambda m: not re.match(r"(127\.|0\.0\.0\.0|255\.|192\.0\.2\.|198\.51\.100\.|203\.0\.113\.)", m.group(0))),
    ("ipv6", re.compile(r"(?<![\w:])([0-9a-fA-F]{1,4}:){4,7}[0-9a-fA-F]{1,4}(?![\w:])"), lambda m: not m.group(0).lower().startswith("2001:db8")),
    ("home path", re.compile(r"(/home/|/Users/)([A-Za-z0-9._-]+)|[A-Za-z]:\\Users\\([A-Za-z0-9 ._<>-]+)"), lambda m: (m.group(2) or m.group(3) or "").lower() not in ("user", "runner", "<user>", "you", "name", "yourname", "shared", "public", "default", "$user", "${user}", "me") and not (m.group(0).endswith("<") or (m.group(3) or "").startswith("<"))),
    ("hostname", re.compile(r"\b[\w-]+(\.[\w-]+)*\.(sslip\.io|nip\.io|duckdns\.org|ngrok(-free)?\.(io|app)|ts\.net|dyndns\.\w+|no-ip\.\w+)\b", re.I), lambda m: True),
    ("ssh login", re.compile(r"\bssh\s+(-\w+\s+\S+\s+)*[A-Za-z0-9._-]+@[A-Za-z0-9.-]+"), lambda m: "example" not in m.group(0)),
    ("token", re.compile(r"(gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{30,}|sk-[A-Za-z0-9_-]{24,}|AKIA[0-9A-Z]{16}|xox[baprs]-[A-Za-z0-9-]{10,}|-----BEGIN [A-Z ]*PRIVATE KEY-----|AIza[0-9A-Za-z_-]{35})"), lambda m: True),
]

def check(label, lineno, line, hits):
    for name, rx, keep in RULES:
        for m in rx.finditer(line):
            if keep(m):
                hits.append(f"{label}:{lineno}: {name}")
                break

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
            check(a[1] if a[1] != "-" else "text", i, line, hits)
    else:
        cmd = ["git", "diff", "--no-color", "-U0", "--no-ext-diff"]
        cmd += ["--cached"] if a[:1] == ["--staged"] else [a[1]] if a[:1] == ["--range"] else sys.exit(__doc__)
        diff = subprocess.run(cmd, capture_output=True, text=True, errors="replace", check=True).stdout
        for path, n, line in added_lines(diff):
            if path.startswith(SKIP_PATHS) or path.endswith(SKIP_SUFFIX):
                continue
            check(path, n, line, hits)
    if hits:
        print("Identifying details found (use placeholders such as user@example.com, 192.0.2.1,")
        print("/home/user, C:\\Users\\<user>, host.example):")
        print("\n".join(sorted(set(hits))[:60]))
        sys.exit(1)

main()
