"""Exercise the generated entry directly and through the reporting driver."""

import json
import os
import signal
import subprocess
import sys
from pathlib import Path

exe, work, *command = sys.argv[1:]
trace = Path(work) / "process trace.txt"


def run(args, status=0, mode=0):
    trace.unlink(missing_ok=True)
    child = subprocess.run(
        args,
        env={**os.environ, "SCOOP_FIXTURE_TRACE": str(trace), "SCOOP_FIXTURE_MODE": str(mode)},
        capture_output=True,
        timeout=60,
    )
    assert child.returncode == status, (child.returncode, child.stderr[-2000:])
    records = (
        [tuple(map(int, line.split())) for line in trace.read_text().splitlines()]
        if trace.exists()
        else []
    )
    return child, records


def report(args, status=0, mode=0):
    child, records = run([*command, *args], status, mode)
    reports = [json.loads(line) for line in child.stderr.splitlines() if line.startswith(b"{")]
    return next(item for item in reports if item["kind"] == "test-report"), records


def check_mixed(records):
    assert [event for _, event in records] == [1, 2, 10, 3, 1, 2, 20, 3, 1, 2, 30, 3, 40, 50], (
        records
    )
    pids = [pid for pid, _ in records]
    assert len(set(pids[:4] + pids[8:12])) == 1
    assert len(set(pids[4:8])) == 1
    assert len({pids[0], pids[4], pids[12], pids[13]}) == 4


check_mixed(run([exe])[1])
result, records = report([])
check_mixed(records)
assert result["counts"]["passed"] == 5 and result["complete"]
assert result["containers"][0]["cases"][1]["process"]["code"] == 0
assert [e for _, e in run([exe, "--scoop-test-cases", "4"])[1]] == [50]
assert [e for _, e in run([exe, "--scoop-test-filter-ignored", "skipped"])[1]] == [60]
_, forced = run([exe, "--scoop-test-isolate-all", "--scoop-test-cases", "0", "1", "3"])
assert [e for _, e in forced] == [1, 2, 10, 3, 1, 2, 20, 3, 40]
assert len({pid for pid, _ in forced}) == 3
assert not run([exe, "--scoop-test-child", "999"], 1)[1]
assert not run([exe, "--scoop-test-child", "1", "2"], 1)[1]
assert not run([exe, "--scoop-test-child", "1"], 1)[1]
assert not report(["--filter", "absent"])[1]
for option in ("--no-run", "--list"):
    assert not run([*command, option])[1]

for mode in range(1, 6):
    result, records = report([], 1, mode)
    case = result["containers"][0]["cases"][1]
    assert result["complete"] and result["target_failure"] is None
    assert result["counts"]["failed"] == 1 and result["counts"]["passed"] == 4
    assert [e for _, e in records if e >= 30] == [30, 40, 50]
    if mode == 3 and os.name != "nt":
        assert case["process"]["signal"] == signal.SIGTERM
    else:
        assert case["process"]["code"] == {1: 0, 2: 23, 3: -1073741819, 4: 0, 5: 19}[mode]
    assert bool(case["interruption"]) == (mode != 4)
    assert [f["phase"] for f in case["failures"]] == (["body", "after-test"] if mode >= 4 else [])
    if mode == 4:
        assert bytes.fromhex(case["output"]["stdout"]["hex"]).endswith(b"\xff" * 131072)
        assert case["output"]["stderr"]["text"].replace("\r\n", "\n") == "x" * 131072 + "shutdown\n"
    else:
        assert case["output"]["stdout"]["text"].splitlines() == ['{"event":"complete"}']
child, _ = run([exe, "--scoop-test-filter", "::isolated"], 1, 1)
assert b"without a complete case result" in child.stderr and b"::isolated" in child.stderr
child, _ = run([*command, "--filter", "::isolated", "--nocapture"], 1, 4)
assert b"\xff" * 131072 in child.stdout and b"x" * 131072 in child.stderr
