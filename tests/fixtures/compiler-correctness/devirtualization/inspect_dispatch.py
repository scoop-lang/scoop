"""Check dispatch at the source call sites that distinguish the two profiles."""

import re
import sys
from pathlib import Path

dump, suite, profile = sys.argv[1:]
mir = Path(dump).read_text(encoding="utf-8")
bodies = {
    match["name"]: match["body"]
    for match in re.finditer(
        r"^  fun (?P<name>\S+) @fn\d+[^\n]*\n(?P<body>(?:    [^\n]*\n)*)", mir, re.M
    )
}

# A known receiver can become a direct call or be inlined. Open receivers and
# merges of different concrete types must still select the implementation at run time.
if suite == "locals":
    known = {"finalParameter": "virtual", "sameBranch": "interface", "stableLoop": "interface"}
    dynamic = {
        "parameter": "virtual",
        "unknownReturn": "virtual",
        "differentBranch": "interface",
        "changingLoop": "interface",
        "exceptionMerge": "interface",
    }
else:
    known = {"known": "virtual", "inherited": "virtual"}
    dynamic = {"openParameter": "virtual", "externalResult": "virtual"}

for name, kind in (known | dynamic).items():
    calls = re.findall(r"^\s*call [^\n]*?\b(virtual|interface)(?=\[| )", bodies[name], re.M)
    expected = [kind] if profile == "debug" or name in dynamic else []
    if calls != expected:
        sys.exit(f"{name} in {profile}: expected {expected}, found {calls}")
