"""Build the dynamic-loader host with its platform C libraries."""

import subprocess
import sys

command = sys.argv[1:]
if sys.platform.startswith("linux"):
    command.extend(["-lm", "-ldl"])
elif sys.platform == "win32":
    command.append("-ldbghelp")
subprocess.run(command, check=True)
