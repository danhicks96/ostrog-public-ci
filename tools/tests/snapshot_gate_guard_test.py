#!/usr/bin/env python3
"""Candidate entry point never reads or certifies private input, even when supplied."""
import os
from pathlib import Path
import subprocess
ROOT = Path(__file__).resolve().parents[2]
for value in ('', 'synthetic_private_marker'):
    env = dict(os.environ, HYGIENE_BLOCKLIST=value, HYGIENE_BLOCKLIST_FILE='missing-fixture')
    result = subprocess.run(['bash', 'tools/publish/check-snapshot.sh'], cwd=ROOT,
                            env=env, capture_output=True, text=True)
    assert result.returncode == 2
    assert 'no privacy clearance' in result.stderr
    if value:
        assert value not in result.stdout + result.stderr
print('OFFLINE AUDIT BOUNDARY GUARD OK')
