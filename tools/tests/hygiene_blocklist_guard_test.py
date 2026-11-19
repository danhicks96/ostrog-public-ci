#!/usr/bin/env python3
"""Public scanner rejects private mode; tests only synthetic input channels."""
import runpy
from pathlib import Path
runpy.run_path(str(Path(__file__).with_name('public_boundary_guard_test.py')), run_name='__main__')
