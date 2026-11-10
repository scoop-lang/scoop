"""Ensure real build and link commands do not publish tool-state records."""

import sys
from pathlib import Path

root = Path(sys.argv[1])
records = sorted(root.glob("*-tools-v*")) + sorted(root.glob("tools-v*"))
assert not records, records
