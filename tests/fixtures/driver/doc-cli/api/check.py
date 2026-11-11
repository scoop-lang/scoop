"""Exercise the public API data and reader through real documentation publication."""

import json
import subprocess
import sys
from pathlib import Path

scoop, scoopc, api = sys.argv[1:4]
work, cache, sysroot = map(Path, sys.argv[4:7])
target = sys.argv[7]
site = work / "site"
project = work / "project"


def document(label):
    command = [scoop, "doc", str(project / "consumer"), str(project / "provider")]
    command.extend(
        str(sysroot / "src" / name) for name in ("scoop.core", "scoop.std", "scoop.test")
    )
    command.extend(
        [
            "--sysroot",
            str(sysroot),
            "--scoopc",
            scoopc,
            "--target",
            target,
            "--no-deps",
            "--deny-doc-warnings",
            "--output-dir",
            str(site),
            "--cache-dir",
            str(cache),
            "--message-format",
            "json",
            "--jobs",
            "4",
        ]
    )
    result = subprocess.run(command, capture_output=True, text=True, encoding="utf-8", timeout=120)
    (work / f"document-{label}.jsonl").write_text(result.stderr)
    assert result.returncode == 0 and not result.stdout, result.stderr


def query(*arguments, code=0, docs=site):
    result = subprocess.run(
        [api, "--docs", str(docs), "--format=json", *arguments],
        capture_output=True,
        text=True,
        encoding="utf-8",
        timeout=20,
    )
    assert result.returncode == code, result.stderr
    if code:
        assert not result.stdout
        return json.loads(result.stderr)
    assert not result.stderr, result.stderr
    data = json.loads(result.stdout)
    assert data["schema"] == 1 and data["target"] == target
    return data


document("initial")
fallback = query("search", "Option orElse")
assert fallback["total"] >= 1, fallback
fallback = query("show", "orElse", "--cone", "scoop.std", "--receiver", "Option")
assert fallback["total"] == 1, fallback
entry = fallback["items"][0]
assert "defaultValue" in entry["declaration"]["signature"]
assert "only for None" in entry["declaration"]["documentation"]["markdown"]
assert any(row["name"] == "orElse" and row["package"] == ["scoop"] for row in entry["imports"])
result_type = query("show", "Result", "--cone", "scoop.std")
assert any(row["declaration"]["kind"] == "enum" for row in result_type["items"])
member = query("show", "isSuccess", "--receiver", "Result", "--cone", "scoop.std")
assert member["total"] == 1, member
assert member["items"][0]["declaringType"]["qualifiedName"] == "scoop.Result"
assertions = query("show", "assertEquals", "--cone", "scoop.test")
signatures = [row["declaration"]["signature"] for row in assertions["items"]]
assert len(signatures) == 2, signatures
assert any("Equals<T>" in signature for signature in signatures), signatures
assert any("where requires" in signature for signature in signatures), signatures
alias = query("show", "examples.api.consumer.showReading", "--cone", "api-consumer")
assert alias["total"] == 1, alias
quoted = query("show", "show reading")
assert quoted["total"] == 1
assert any(
    row["path"] == "`examples`.`api`.`consumer`.`show reading`"
    for row in quoted["items"][0]["imports"]
)
entry = alias["items"][0]
assert entry["coordinate"] == "examples.api.provider:api-provider:0.1.0"
assert any(
    row["reexport"] and row["coordinate"] == "examples.api.consumer:api-consumer:0.1.0"
    for row in entry["imports"]
)
assert "α中" in entry["declaration"]["documentation"]["markdown"]
source = entry["declaration"]["source"]
assert source["line"] > 1 and source["byteEnd"] > source["byteStart"]
assert (site / source["url"].split("#", 1)[0]).is_file()
assert query("search", "reading text")["total"] >= 1
assert query("show", "hiddenImplementation")["status"] == "empty"
assert query("search", "nowhere-named-api")["status"] == "empty"
other_target = (
    "x86_64-unknown-linux-gnu" if target != "x86_64-unknown-linux-gnu" else "aarch64-apple-darwin"
)
assert query("show", "orElse", "--target", other_target, code=1)["kind"] == "error"
assert query("show", "orElse", docs=work / "not-published", code=1)["kind"] == "error"
text = subprocess.run(
    [api, "--docs", str(site), "show", "orElse", "--cone", "scoop.std"],
    capture_output=True,
    text=True,
    encoding="utf-8",
    timeout=20,
)
assert text.returncode == 0 and not text.stderr
assert "defaultValue" in text.stdout and "import" in text.stdout

# Change the real API and its consumer together, then let ordinary publication update it.
for name in ("provider", "consumer"):
    source_file = project / name / "src/api.scoop"
    source_file.write_text(source_file.read_text().replace("describe", "formatReading"))
document("renamed")
assert query("show", "describe", "--cone", "api-provider")["status"] == "empty"
updated = query("show", "formatReading", "--cone", "api-provider")
assert updated["total"] == 1
assert "formatReading" in query("show", "showReading")["items"][0]["declaration"]["signature"]
print("Published API queries passed.")
