#!/usr/bin/env python3
"""Check committed export structure only; not a private audit or release approval."""
import subprocess
import sys


def assess(paths):
    return any(p in ('AGENTS.md', 'go-public', '.github/workflows/automerge.yml') or
               p.startswith(('tools/publish/', 'tools/boxci/', 'go-public/')) for p in paths)


def main():
    try:
        result = subprocess.run(['git', '--no-replace-objects', 'ls-tree', '-r',
                                 '--name-only', '-z', 'HEAD'], capture_output=True, check=True)
        paths = result.stdout.decode('utf-8').split('\0')
        if assess(paths):
            print('export-structure: excluded automation/tooling markers found')
            return 1
    except (OSError, UnicodeError, subprocess.CalledProcessError):
        print('export-structure: inspection unavailable')
        return 1
    print('export-structure: checked committed paths only; private audit separate')
    return 0


if __name__ == '__main__':
    sys.exit(main())
