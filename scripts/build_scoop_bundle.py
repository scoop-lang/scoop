"""Build and install the standalone application packager without LLVM."""

import argparse
import os
import shutil
import subprocess
from pathlib import Path

from build_support import ROOT
from target_catalog import host_definition


def build(target_dir, environment=None):
    environment = os.environ.copy() if environment is None else environment
    subprocess.run(
        [
            "cargo",
            "build",
            "--manifest-path",
            str(ROOT / "tooling/scoop-bundle/Cargo.toml"),
            "--release",
            "--locked",
            "--target-dir",
            str(target_dir),
        ],
        cwd=ROOT,
        env=environment,
        check=True,
    )
    target = environment.get("CARGO_BUILD_TARGET")
    output = target_dir / target if target else target_dir
    windows = host_definition(target)["system"] == "Windows" if target else os.name == "nt"
    return output / "release" / ("scoop-bundle.exe" if windows else "scoop-bundle")


def install(binary, sysroot):
    commands = sysroot / "commands"
    commands.mkdir(parents=True, exist_ok=True)
    shutil.copy2(binary, commands / binary.name)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sysroot", type=Path, default=ROOT / "sysroot")
    parser.add_argument(
        "--target-dir", type=Path, default=ROOT.parent / "scoop-work/scoop-bundle/cargo"
    )
    args = parser.parse_args()
    binary = build(args.target_dir.resolve())
    install(binary, args.sysroot.resolve())
    print(f"Installed {args.sysroot.resolve() / 'commands' / binary.name}")


if __name__ == "__main__":
    main()
