import subprocess
import sys
import tempfile
import time
from pathlib import Path

executable, helper, work = sys.argv[1:]
work = Path(work)
work.mkdir(parents=True, exist_ok=True)
# The runner may reuse its work directory; synchronization files belong to one invocation.
work = Path(tempfile.mkdtemp(prefix="communication-", dir=work))
try:
    result = subprocess.run(
        [executable, sys.executable, helper, str(work)],
        capture_output=True,
        timeout=30,
    )
    assert result.returncode == 0, (result.returncode, result.stdout, result.stderr)
    assert result.stdout == b"process communication deadlines and cleanup passed\n", result.stdout
    assert not result.stderr, result.stderr
    for name in ("descendant", "descendant-error"):
        base = work / name
        assert base.with_suffix(".ready").exists(), name
        assert not base.with_suffix(".done").exists(), "communicate waited for descendant EOF"
finally:
    for name in ("hang", "stages", "no-pipes"):
        (work / name).with_suffix(".release").touch()
    for name in ("descendant", "descendant-error"):
        base = work / name
        base.with_suffix(".release").touch()
        if base.with_suffix(".ready").exists():
            deadline = time.monotonic() + 5
            while not base.with_suffix(".done").exists():
                assert time.monotonic() < deadline, "descendant did not survive or finish cleanup"
                time.sleep(0.01)
sys.stdout.buffer.write(result.stdout)
