"""Check real precompiled Lucide resources and an independently extended library."""

import json
import re
import subprocess
import sys
from pathlib import Path

work = Path(sys.argv[1])
zero, one, all_icons, expanded = map(Path, sys.argv[2:])
assets = {path.stem: path.read_bytes() for path in (work / "library/assets").glob("*.svg")}
objects = {}
getters = {}
for path in (work / "dump").glob("*/lir.txt"):
    for symbol, value in re.findall(
        r'^  global @(scoop\$1\$io\$[0-9a-f]{64}) = (".+")$',
        path.read_text(encoding="utf-8"),
        re.MULTILINE,
    ):
        payload = json.loads(value).encode()
        for name, expected in assets.items():
            if payload == expected:
                assert name not in objects
                objects[name] = symbol
for path in (work / "dump").glob("*/symbols.json"):
    report = json.loads(path.read_text(encoding="utf-8"))
    for entry in report["symbols"]:
        declaration = entry.get("declaration")
        if declaration and declaration["source"]["cone"] == report["cone"]:
            source = declaration["source"]["path"]
            if source.startswith("src/generated/"):
                name = Path(source).stem
                if name in assets:
                    assert name not in getters
                    getters[name] = entry

assert objects.keys() == assets.keys(), objects.keys()
assert getters.keys() == assets.keys(), getters.keys()
original = set(assets) - {"added-probe"}
for binary, used in (
    (zero, set()),
    (one, {"search"}),
    (all_icons, original),
    (expanded, {"search"}),
):
    data = binary.read_bytes()
    has_symbol_table = True
    if data.startswith(b"MZ"):
        # Microsoft link omits the COFF symbol table; image registration IDs remain.
        pe = int.from_bytes(data[60:64], "little")
        has_symbol_table = int.from_bytes(data[pe + 16 : pe + 20], "little") != 0
    for name, payload in assets.items():
        present = name in used
        assert data.count(payload) == int(present), (binary, name)
        symbol = objects[name]
        if has_symbol_table:
            assert (symbol.encode() + b"\0" in data) == present, (binary, symbol)
            registration = symbol.replace("$io$", "$ir$")
            assert (registration.encode() + b"\0" in data) == present, (binary, registration)
        assert data.count(bytes.fromhex(symbol.rsplit("$", 1)[1])) == int(present), (binary, symbol)
        if not present:
            getter = getters[name]
            assert getter["symbol"].encode() + b"\0" not in data, (binary, name)
            assert bytes.fromhex(getter["body"]) not in data, (binary, name)
    result = subprocess.run([str(binary)], capture_output=True, text=True, check=True, timeout=30)
    expected = ""
    if binary in (one, expanded):
        expected = "true\n"
    elif binary == all_icons:
        expected = f"{sum(len(assets[name]) for name in original)}\n"
    assert result.stdout == expected and result.stderr == "", result

sys.stdout.buffer.write(b"Lucide keeps only referenced SVGs, getters and registrations\n")
