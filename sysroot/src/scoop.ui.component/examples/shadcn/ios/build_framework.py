#!/usr/bin/env python3
"""Build the local Scoop UI Gallery framework consumed by Gallery.xcodeproj."""

import argparse
import os
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = next(
    parent for parent in HERE.parents if (parent / "scripts/package_ios_xcframework.py").is_file()
)
MODULE = "ScoopGallery"
TARGETS = {"simulator": "aarch64-apple-ios-sim", "device": "aarch64-apple-ios"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--sysroot", type=Path, default=os.environ.get("SCOOP_SYSROOT", ROOT / "sysroot")
    )
    parser.add_argument("--binaries", type=Path, default=ROOT / "target/release")
    parser.add_argument("--linker", type=Path, help="Mach-O LLD for the Scoop toolchain")
    parser.add_argument("--cache-dir", type=Path)
    parser.add_argument("--sdk", choices=["both", *TARGETS], default="both")
    args = parser.parse_args()
    build = HERE / "target"
    build.mkdir(exist_ok=True)
    package = []
    for variant, target in TARGETS.items():
        if args.sdk not in (variant, "both"):
            continue
        output = build / target / MODULE
        output.parent.mkdir(exist_ok=True)
        command = [
            args.binaries.resolve() / "scoop",
            "build",
            HERE,
            "--release",
            "--target",
            target,
            "--sysroot",
            args.sysroot.resolve(),
            "--scoopc",
            args.binaries.resolve() / "scoopc",
            "--target-dir",
            build / "cones",
            "--lockfile",
            build / target / "Cone.lock",
            "--install-name",
            f"@rpath/{MODULE}.framework/{MODULE}",
            "-o",
            output,
        ]
        if args.linker:
            command.extend(["--linker", args.linker.resolve()])
        if args.cache_dir:
            command.extend(["--cache-dir", args.cache_dir.resolve()])
        subprocess.run(command, check=True)
        package.extend([f"--{variant}", output])
    destination = build / f"{MODULE}.xcframework"
    with tempfile.TemporaryDirectory(dir=build) as temporary:
        staging = Path(temporary)
        framework = staging / destination.name
        subprocess.run(
            [
                sys.executable,
                ROOT / "scripts/package_ios_xcframework.py",
                "--name",
                MODULE,
                "--bundle-id",
                "dev.scoop.gallery.framework",
                "--header",
                HERE / "GalleryScene.h",
                "--output",
                framework,
                *package,
            ],
            check=True,
        )
        if destination.exists():
            destination.rename(staging / "previous")
        framework.rename(destination)
    print(destination)


if __name__ == "__main__":
    main()
