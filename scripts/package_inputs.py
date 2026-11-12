"""Prepare the fixed downloaded inputs used by release jobs."""

import argparse
import json
import shutil
import tempfile
import tomllib
import zipfile
from pathlib import Path

from build_support import RELEASE_CONFIG, build_lock, metadata
from download_support import download_directory, obtain


def python_license(directory, version):
    return obtain(
        directory,
        "cpython",
        version,
        "LICENSE",
        f"https://raw.githubusercontent.com/python/cpython/v{version}/LICENSE",
    )


def prepare_mingw(directory, work, crt):
    version = tomllib.loads(RELEASE_CONFIG.read_text())["llvm_mingw"]
    name = f"llvm-mingw-{version}-{crt}-x86_64"
    archive = obtain(
        directory,
        "llvm-mingw",
        version,
        name + ".zip",
        f"https://github.com/mstorsjo/llvm-mingw/releases/download/{version}/{name}.zip",
    )
    work.mkdir(parents=True, exist_ok=True)
    prefix = work / name
    record = work / (name + ".json")
    with build_lock(work / (name + ".lock")):
        current = metadata(archive)
        if prefix.is_dir() and record.is_file() and json.loads(record.read_text()) == current:
            return prefix
        with tempfile.TemporaryDirectory(prefix="mingw-", dir=work) as temporary:
            with zipfile.ZipFile(archive) as package:
                package.extractall(temporary)
            extracted = Path(temporary) / name
            if not extracted.is_dir():
                raise ValueError(f"LLVM-MinGW archive does not contain {name}")
            if prefix.exists():
                shutil.rmtree(prefix)
            extracted.replace(prefix)
        record.write_text(json.dumps(current) + "\n")
    return prefix


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--download-dir", type=Path)
    parser.add_argument("--work-dir", type=Path, required=True)
    parser.add_argument("--mingw", choices=("ucrt", "msvcrt"), nargs="+", required=True)
    args = parser.parse_args()
    for crt in args.mingw:
        print(prepare_mingw(download_directory(args.download_dir), args.work_dir.resolve(), crt))


if __name__ == "__main__":
    main()
