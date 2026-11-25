"""Inspect only the ordinary calls whose inlining decisions the program exercises."""

import re
import sys
from pathlib import Path

path, suite, profile = sys.argv[1:]
source = Path(path).read_text(encoding="utf-8")
functions = {
    name: (identity, body)
    for name, identity, body in re.findall(
        r"^  fun (\S+) (@fn\d+)\([^\n]*\n((?:    [^\n]*\n)*)", source, re.M
    )
}


def check_call(caller, target, retained):
    calls = re.findall(
        r"^\s*call (?:[^\n]* = )?(@fn\d+|external\d+) direct\b", functions[caller][1], re.M
    )
    if target == "external":
        # externalBody contains a single provider call, to ordinary().
        found = any(call.startswith("external") for call in calls)
    else:
        found = functions[target][0] in calls
    if found != retained:
        sys.exit(f"{caller} -> {target} in {profile}: expected retained={retained}, found {calls}")


debug = profile == "debug"
if suite == "artifacts":
    check_call("locallyInstantiated", "generic", debug)
    check_call("externalBody", "external", debug)
else:
    check_call("hot", "medium", debug)
    check_call("cold", "medium", debug)
    check_call("constantPath", "calculate", debug)
    check_call("unknownPath", "calculate", True)
    check_call("even", "odd", True)
    check_call("odd", "even", True)
