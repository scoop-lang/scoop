"""Check real compiler progress, including parallel dependencies and cache reuse."""

import json
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[3]))

from fixture_runner.values import canonical_host_path

scoop, scoopc, work, sysroot, *compile_args = sys.argv[1:]
work = Path(work)
root = work / "cones" / "root"
output = work / "root.slib"
options = [
    "--sysroot",
    sysroot,
    "--scoopc",
    scoopc,
    "--cache-dir",
    str(work / "cache"),
    "--jobs",
    "2",
    *compile_args,
]


def run(command, cone, *arguments, exit_code=0):
    process = subprocess.run(
        [scoop, command, str(cone), *options, *map(str, arguments)],
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    assert process.returncode == exit_code, process.stderr
    assert not process.stdout, process.stdout
    return process.stderr


def progress(stderr, expected, status="Finished"):
    started = {}
    finished = set()
    other = []
    for line in stderr.splitlines():
        start = re.fullmatch(r" *(Compiling|Checking|Documenting) (\S+)", line)
        end = re.fullmatch(r" *(Finished|Failed) (\S+) in ([0-9]+\.[0-9]{2})s", line)
        if start:
            action, coordinate = start.groups()
            assert coordinate not in started, stderr
            assert line.startswith(f"{action:>12} "), line
            started[coordinate] = action
        elif end:
            actual_status, coordinate, elapsed = end.groups()
            assert actual_status == status, stderr
            assert coordinate in started and coordinate not in finished, stderr
            assert line.startswith(f"{status:>12} "), line
            assert float(elapsed) >= 0, line
            finished.add(coordinate)
        else:
            other.append(line)
    assert started == expected, stderr
    assert finished == set(started), stderr
    return "\n".join(other) + "\n"


coordinates = {name: f"test.progress:{name}:1.0.0" for name in ("root", "left", "right")}
build_options = ["--release", "--emit-slib", "-o", output]
build_summary = f"built {canonical_host_path(output)} (release)\n"
cold = run("build", root, *build_options)
assert progress(cold, dict.fromkeys(coordinates.values(), "Compiling")) == build_summary, cold
assert output.is_file()
warm = run("build", root, *build_options)
assert progress(warm, {}) == build_summary, warm

structured = run("build", root, *build_options, "--message-format", "json")
events = [json.loads(line) for line in structured.splitlines()]
result = next(event for event in events if event["kind"] == "artifact")
assert result["observations"]["child_invocations"] == [], structured

checked = run("check", root, "--release")
assert (
    progress(checked, {coordinates["root"]: "Checking"})
    == "checked test.progress:root:1.0.0 (production)\n"
)
docs = work / "docs"
documented = run("doc", work / "cones" / "left", "--no-deps", "--output-dir", docs)
assert (
    progress(documented, {coordinates["left"]: "Documenting"})
    == f"documented {docs / 'index.html'}\n"
), documented

source = root / "src" / "api.scoop"
source.write_text(source.read_text().replace("left() + right()", '"wrong type"'), encoding="utf-8")
failed_output = work / "failed.slib"
failed = run("build", root, "--release", "--emit-slib", "-o", failed_output, exit_code=1)
diagnostics = progress(failed, {coordinates["root"]: "Compiling"}, status="Failed")
assert "error[SCOOPC_HIR_ERROR]" in diagnostics and "found String" in diagnostics, diagnostics
assert not failed_output.exists()
sys.stdout.buffer.write(b"per-cone progress passed\n")
