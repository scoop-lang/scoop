"""Exercise the public tools with a real output pipe whose reader has gone away."""

import os
import subprocess
import sys
from pathlib import Path
from tempfile import TemporaryDirectory

compiler, linker, pm, api, scoop, cone = sys.argv[1:]
reader, writer = os.pipe()
os.close(reader)
with os.fdopen(writer, "wb") as pipe:
    for argv in ([compiler, "--help"], [linker, "--help"], [pm, "members", cone]):
        result = subprocess.run(argv, stdout=pipe, stderr=subprocess.PIPE)
        if result.returncode != 1 or not result.stderr.startswith(b"error[E2204]"):
            sys.exit(f"unexpected stdout failure: {result!r}")

    for argv, code in (
        ([compiler, "--unknown-option"], 1),
        ([linker, "--unknown-option"], 1),
        ([pm, "members", "missing-cone"], 1),
        ([api, "--unknown-option"], 2),
    ):
        result = subprocess.run(argv, stderr=pipe)
        if result.returncode != code:
            sys.exit(f"unexpected diagnostic exit: {result!r}")

    with TemporaryDirectory() as directory:
        project = Path(directory) / "example"
        result = subprocess.run([scoop, "new", str(project), "--group", "dev.example"], stderr=pipe)
        if result.returncode != 1 or not (project / "Cone.toml").is_file():
            sys.exit(f"unexpected project creation status: {result!r}")
