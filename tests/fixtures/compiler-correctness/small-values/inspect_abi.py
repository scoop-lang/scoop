"""Write the physical signatures exercised by the small-value ABI programs."""

import json
import re
import sys
from pathlib import Path

role, directory, destination = sys.argv[1:]
dump = Path(directory)
lir = (dump / "lir.txt").read_text(encoding="utf-8")
if role == "native":
    signatures = dict(re.findall(r"^  extern ef\d+ \S+ @(small_\w+)(\(.*)$", lir, re.M))
else:
    names = {
        "provider": ["identity", "floats", "registers", "Worker.apply"],
        "consumer": ["dispatch", "generic"],
    }[role]
    symbols = json.loads((dump / "symbols.json").read_text(encoding="utf-8"))["symbols"]
    functions = dict(re.findall(r"^  fun @([^ (]+)(\(.*)$", lir, re.M))
    signatures = {
        item["name"]: functions[item["symbol"]]
        for item in symbols
        if item["name"] in names and item.get("symbol") in functions
    }
    if missing := set(names) - signatures.keys():
        sys.exit(f"missing {role} ABI signatures: {sorted(missing)}")

# Struct ordinals belong to the producing Cone. The caller and callee agree on the
# lowered parts, offsets, alignment and passing mode recorded in the signature.
with Path(destination).open("w", encoding="utf-8", newline="\n") as output:
    for name, signature in sorted(signatures.items()):
        output.write(name + re.sub(r"\bstruct\d+\b", "struct", signature) + "\n")
