"""Inspect selected immortal objects, their registrations, and executable behavior."""

import json
import os
import re
import subprocess
import sys
from pathlib import Path

work = Path(sys.argv[1])
profile = sys.argv[2]
zero, one, many, instrumented, expanded = map(Path, sys.argv[3:])
payloads = {}
for root in (work / "provider/src", work / "many/src"):
    for path in root.rglob("*"):
        if path.is_file() and path.suffix != ".scoop":
            payloads[path.name if path.parent == root else path.parent.name + "/" + path.name] = (
                path.read_bytes()
            )
payloads["literal"] = b"immortal-pruning-literal"
payloads["unused-literal"] = b"immortal-pruning-unused-literal"
payloads["live-path"] = b"live-entry.txt"
payloads["unread-path"] = b"unread-entry.txt"
payloads["unused-path"] = b"unused-entry.txt"
single = {"live.txt"}
multiple = set(payloads) - {
    "unused.txt",
    "unused.bin",
    "unused-dir/unused-entry.txt",
    "added.txt",
    "unused-literal",
    "unused-path",
}

objects = {}
entries = {}
provider_objects = set()
object_graph = {}
generic_producers = {}
for directory in work.glob("dump-*"):
    provider = None
    for path in directory.glob("*/symbols.json"):
        report = json.loads(path.read_text(encoding="utf-8"))
        for entry in report["symbols"]:
            if entry.get("name") and entry.get("declaration"):
                source = entry["declaration"]["source"]
                if source["cone"] == report["cone"]:
                    entries[entry["symbol"]] = entry
                    if entry["name"] == "unusedDirectory":
                        provider = report["cone"]
    for path in directory.glob("*/lir.txt"):
        text = path.read_text(encoding="utf-8")
        globals_ = re.findall(
            r"^  (?:global|imported_immortal|imported_global|raw_global|raw_thread_local) @(\S+)",
            text,
            re.MULTILINE,
        )
        for symbol, value in re.findall(
            r"^  global @(scoop\$1\$io\$[0-9a-f]{64}) = (.+)$", text, re.MULTILINE
        ):
            if value.startswith('"'):
                payload = json.loads(value).encode()
            elif value.startswith("bytes ["):
                payload = bytes(json.loads(value[6 : value.index("]") + 1]))
            elif value.startswith("files "):
                payload = None
                object_graph[symbol] = [
                    globals_[int(index)] for index in re.findall(r"Idx::<Global>\((\d+)\)", value)
                ]
            else:
                raise AssertionError((symbol, value))
            objects[symbol] = payload
            if path.parent.name == provider:
                provider_objects.add(symbol)
            if directory.name == "dump-many" and payload == payloads["generic.txt"]:
                generic_producers.setdefault(symbol, set()).add(path.parent.name)

assert objects, "missing LIR immortal candidates"
assert any(payload == payloads["added.txt"] for payload in objects.values())
assert any(entry["name"] == "added" for entry in entries.values())
assert provider_objects and object_graph
assert any(len(producers) >= 2 for producers in generic_producers.values()), generic_producers

for binary, used in (
    (zero, set()),
    (one, single),
    (many, multiple),
    (instrumented, multiple),
    (expanded, single),
):
    data = binary.read_bytes()
    for name, payload in payloads.items():
        assert (payload in data) == (name in used), (binary, name)
    for symbol, payload in objects.items():
        present = symbol.encode() + b"\0" in data
        registration = symbol.replace("$io$", "$ir$")
        assert (registration.encode() + b"\0" in data) == present, (binary, registration)
        # Only the selected registration stores this object's semantic ID.
        assert data.count(bytes.fromhex(symbol.rsplit("$", 1)[1])) == int(present), (binary, symbol)
        if binary == zero and symbol in provider_objects:
            assert not present, "unused provider must have an empty immortal table"
        if present:
            for child in object_graph.get(symbol, []):
                assert child.encode() + b"\0" in data, (binary, symbol, child)
        matching = {name for name, expected in payloads.items() if payload == expected}
        if matching:
            assert present == bool(matching & used), (binary, symbol, matching)
    for entry in entries.values():
        if entry["name"] in {
            "unusedText",
            "unusedBytes",
            "unusedDirectory",
            "unusedLiteral",
            "added",
        }:
            assert entry["symbol"].encode() + b"\0" not in data, (binary, entry)
            assert bytes.fromhex(entry["body"]) not in data, (binary, entry)
    if binary in (one, expanded):
        getters = [entry for entry in entries.values() if entry["name"] == "text"]
        assert len(getters) == 1, getters
        getter = getters[0]
        assert (getter["symbol"].encode() + b"\0" in data) == (profile == "debug")
        assert (bytes.fromhex(getter["body"]) in data) == (profile == "debug")


def content(name):
    return payloads[name].decode()


expected_many = (
    "initialized\n"
    + content("live.txt")
    + f"{len(payloads['live.bin'])}\n105\n2\n"
    + f"live-entry.txt{len(payloads['live-dir/live-entry.txt'])}\n"
    + content("inline.txt")
    + content("default.txt")
    + content("generic.txt") * 3
    + content("other.txt")
    + f"{len(payloads['child-dir/child-entry.txt'])}\n"
    + content("literal")
    + "\n"
    + content("static.txt")
    + content("initialized.txt")
    + content("lazy.txt")
)
counts = None
for binary, stress, expected in (
    (zero, None, ""),
    (one, None, content("live.txt")),
    (expanded, None, content("live.txt")),
    (many, None, expected_many),
    (instrumented, "MOVE", expected_many),
    (instrumented, "MINOR", expected_many),
):
    environment = os.environ.copy()
    for name in ("SCOOP_GC_STRESS_MOVE", "SCOOP_GC_STRESS_MINOR", "SCOOP_GC_STATS"):
        environment.pop(name, None)
    if stress:
        environment["SCOOP_GC_STRESS_" + stress] = "1"
    result = subprocess.run(
        [str(binary)], env=environment, capture_output=True, text=True, timeout=30, check=True
    )
    assert result.stdout == expected, (binary, result)
    if stress:
        record = json.loads(result.stderr)
        current = record["counts"]
        assert current["immortals"] > 0
        assert current["safepoints"] > 0
        assert counts is None or current == counts
        counts = current
    else:
        assert result.stderr == "", result

print("unused resources and registrations removed; live object graphs survive GC")
