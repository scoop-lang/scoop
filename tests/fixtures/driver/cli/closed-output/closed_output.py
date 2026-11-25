"""Connect the selected compiler stream to a pipe with no reader."""

import os
import subprocess
import sys

compiler = sys.argv[1]
reader, writer = os.pipe()
os.close(reader)
with os.fdopen(writer, "wb") as closed_pipe:
    help_result = subprocess.run([compiler, "--help"], stdout=closed_pipe, stderr=subprocess.PIPE)
    if help_result.returncode != 1 or not help_result.stderr.startswith(b"error[E2204]"):
        sys.exit(f"unexpected help output failure: {help_result!r}")
    usage_result = subprocess.run([compiler, "--unknown-option"], stderr=closed_pipe)
    if usage_result.returncode != 1:
        sys.exit(f"unexpected diagnostic output failure: {usage_result!r}")
