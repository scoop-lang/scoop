import os
import subprocess
import sys
import time
from pathlib import Path


def write_all(fd, data):
    while data:
        count = os.write(fd, data)
        data = data[count:]


def wait_for_release(base):
    deadline = time.monotonic() + 60
    while not base.with_suffix(".release").exists():
        if time.monotonic() >= deadline:
            return False
        time.sleep(0.01)
    return True


def main():
    mode = sys.argv[1]
    if mode == "empty":
        return 0
    if mode == "failed":
        write_all(2, b"failure detail")
        return 7
    if mode == "echo":
        write_all(1, b"o" * 131072)
        write_all(2, b"e" * 131072)
        while chunk := os.read(0, 8192):
            write_all(1, chunk)
            write_all(2, chunk)
        return 0
    if mode == "holder":
        base = Path(sys.argv[2])
        base.with_suffix(".ready").write_text(str(os.getpid()))
        if not wait_for_release(base):
            return 99
        base.with_suffix(".done").write_text("released")
        return 0
    if mode in {"descendant", "descendant-error"}:
        base = Path(sys.argv[2])
        subprocess.Popen(
            [sys.executable, __file__, "holder", str(base)],
            stdin=subprocess.DEVNULL,
            stdout=sys.stdout,
            stderr=sys.stderr,
        )
        deadline = time.monotonic() + 10
        while not base.with_suffix(".ready").exists():
            assert time.monotonic() < deadline, "holder did not start"
            time.sleep(0.01)
        if mode == "descendant-error":
            os.close(0)
        write_all(1, b"ready\npartial-out")
        write_all(2, b"partial-err")
        if mode == "descendant":
            return 0
        wait_for_release(base)
        return 98
    if mode == "stages":
        write_all(1, b"ready\n")
        time.sleep(0.75)
        while os.read(0, 8192):
            pass
        write_all(1, b"stage-out")
        for fd in (0, 1, 2):
            os.close(fd)
        wait_for_release(Path(sys.argv[2]))
        return 98
    if mode == "no-pipes":
        write_all(1, b"ready\n")
        for fd in (0, 1, 2):
            os.close(fd)
        wait_for_release(Path(sys.argv[2]))
        return 98
    assert mode == "hang", mode
    write_all(1, b"ready\npartial-out")
    write_all(2, b"partial-err")
    wait_for_release(Path(sys.argv[2]))
    return 98


if __name__ == "__main__":
    raise SystemExit(main())
