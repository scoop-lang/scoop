"""Check public companions from source build and artifact-only linking."""

import os
import subprocess
import sys
from pathlib import Path

built, linked = (Path(value) for value in sys.argv[1:3])
target, profile = sys.argv[3:5]
source, work, include = (Path(value) for value in sys.argv[5:8])
compiler = sys.argv[8:]
if "windows" in target:
    extension = ".lib" if "-msvc-" in target else ".dll.a"
    for product in (built, linked):
        assert product.with_suffix(extension).is_file(), product
if profile == "debug":
    if "windows" in target:
        debug = Path(str(built) + ".pdb")
    else:
        debug = Path(str(built) + (".dSYM" if "darwin" in target else ".debug"))
    assert debug.exists(), debug

for index, product in enumerate((built, linked)):
    library = product.with_suffix(extension) if "windows" in target else product
    output = work / f"import-host-{index}.exe"
    command = compiler + ["-std=c++17", "-Wall", "-Wextra", "-Werror", "-I", str(include)]
    command += [str(source), str(library), "-o", str(output)]
    if "windows" not in target:
        command.append("-Wl,-rpath," + str(product.parent))
    subprocess.run(command, check=True)
    environment = dict(os.environ)
    environment["PATH"] = str(product.parent) + os.pathsep + environment.get("PATH", "")
    subprocess.run([str(output)], env=environment, check=True)
