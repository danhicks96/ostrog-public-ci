#!/usr/bin/env python3
"""Private CI stays manual; public CI comes from a distinct secret-free template."""
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
private = (ROOT / '.github/workflows/ci.yml').read_text()
public = (ROOT / 'tools/publish/public-ci.yml').read_text()
private_events = private.split('on:\n', 1)[1].split('\npermissions:', 1)[0]
assert 'workflow_dispatch:' in private_events
assert 'pull_request:' not in private_events and 'push:' not in private_events
assert 'pull_request:' in public and 'push:' in public
assert '  export-structure:' in public and '  snapshot-gate:' not in public
assert 'secrets.' not in private + public
assert 'HYGIENE_BLOCKLIST' not in private + public
assert 'public-ci.yml' in (ROOT / 'tools/publish/make-snapshot.sh').read_text()
print('PUBLIC PRIVATE WORKFLOW BOUNDARY GUARD OK')
