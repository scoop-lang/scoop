"""Check only query bodies, excluding unrelated materialized core methods."""

import re
import sys
from pathlib import Path

module = Path(sys.argv[1]).read_text()
queries = [
    body
    for body in re.split(r"(?m)^  (?=fun |td )", module)
    if body.startswith("fun ") and "@scoop_rt_context_try_get(" in body
]
keys = set()
for body in queries:
    for line in body.splitlines():
        if "@scoop_rt_context_try_get(" in line:
            assert "call no-gc-direct-target" in line, line
            keys.update(re.findall(r"context-cell<([0-9a-f]{64})>", line))
        if "@scoop_rt_itable_lookup(" in line:
            assert "call no-gc-direct-target" in line, line
    for forbidden in (
        "@scoop_rt_alloc(",
        "@scoop_rt_prepare_throwable(",
        "\n    throw ",
        "landingpad",
        "missing context ",
    ):
        assert forbidden not in body, body

assert len(keys) == 3, f"expected interface, String and function context keys: {keys}"
print("context queries use allocation-free NoGC lookups")
