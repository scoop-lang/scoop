"""Log actual GNU linker invocations while keeping every input available."""

import shlex
import shutil
import subprocess
import sys
from pathlib import Path

root = Path(sys.argv[1])
compiler = Path(sys.argv[2]).resolve()
root.mkdir()
selected = subprocess.check_output([compiler, "-print-prog-name=ld"], text=True).strip()
linker = Path(shutil.which(selected)).resolve()
wrapper = root / "ld"
wrapper.write_text(
    f"#!/bin/sh\necho link >> {shlex.quote(str(root / 'calls'))}\n"
    f'exec {shlex.quote(str(linker))} "$@"\n'
)
wrapper.chmod(0o755)
driver = root / "gcc"
driver.write_text(
    f'#!/bin/sh\nexec {shlex.quote(str(compiler))} -B {shlex.quote(str(root) + "/")} "$@"\n'
)
driver.chmod(0o755)
