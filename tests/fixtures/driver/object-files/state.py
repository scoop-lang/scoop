"""Observe stable archive outputs without reading their contents."""

import json
import sys
from pathlib import Path

mode, root, saved = sys.argv[1:]
paths = sorted(Path(root).glob("toolchains/*/link-archives-v2/members-*/part-*"))
assert paths, "no linked archives"
assert not list(Path(root).glob("toolchains/*/objects-v1")), "loose objects were materialized"
state = {str(p): [p.stat().st_size, p.stat().st_mtime_ns] for p in paths}
if mode == "save":
    Path(saved).write_text(json.dumps(state))
elif mode in ("same", "subset"):
    previous = json.loads(Path(saved).read_text())
    if mode == "same":
        assert state == previous, (previous, state)
    else:
        assert all(state.get(p) == value for p, value in previous.items())
else:
    raise ValueError(mode)
