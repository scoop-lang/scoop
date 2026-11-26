#!/usr/bin/env python3
"""Build local Gallery libraries or package an already prepared iOS sysroot."""

import argparse
import graphlib
import os
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

from build_support import ROOT
from package_toolchain import (
    archive,
    assemble_base_libraries,
    assemble_gui_libraries,
    split_gui_cones,
)
from prepare_sysroot import build_cones, target_dependencies
from target_catalog import IOS_TARGETS

GALLERY = ROOT / "sysroot/src/scoop.ui.component/examples/shadcn/Cone.toml"


def gallery_cones(sysroot, target):
    roots = target_dependencies(tomllib.loads(GALLERY.read_text()), target) + ["scoop:scoop.os.ios"]
    graph = {}
    pending = list(roots)
    while pending:
        coordinate = pending.pop()
        if coordinate in graph:
            continue
        directory = coordinate.removeprefix("scoop:")
        manifest = tomllib.loads((sysroot / "src" / directory / "Cone.toml").read_text())
        graph[coordinate] = target_dependencies(manifest, target)
        pending.extend(graph[coordinate])
    return [
        name.removeprefix("scoop:") for name in graphlib.TopologicalSorter(graph).static_order()
    ]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=IOS_TARGETS, required=True)
    parser.add_argument("--sysroot", dest="prepared_sysroot", type=Path, default=ROOT / "sysroot")
    parser.add_argument("--binaries", type=Path, default=ROOT / "target/release")
    parser.add_argument(
        "--build-dir", type=Path, required=True, help="reusable native backend build directory"
    )
    parser.add_argument("--cache-dir", type=Path)
    parser.add_argument("--gn", default="gn")
    parser.add_argument("--jobs", type=int, default=min(os.cpu_count() or 1, 6))
    parser.add_argument("--package-dir", type=Path, help="write local base and GUI archives here")
    parser.add_argument("--package-only", action="store_true", help="use the installed libraries")
    args = parser.parse_args()
    if args.package_only and not args.package_dir:
        parser.error("--package-only requires --package-dir")
    if args.jobs < 1:
        parser.error("--jobs must be positive")
    sysroot = args.prepared_sysroot = args.prepared_sysroot.resolve()
    target = args.target
    cones = gallery_cones(sysroot, target)
    if not args.package_only:
        tools = args.binaries.resolve()
        work = args.build_dir.resolve()
        local = work / "cones" / target
        cache = (args.cache_dir or work / "cache").resolve()
        common = ["--target", target, "--jobs", str(args.jobs)]
        scripts = ROOT / "scripts"
        subprocess.run(
            [
                sys.executable,
                scripts / "prepare_sysroot.py",
                *common,
                "--sysroot",
                sysroot,
                "--scoop",
                tools / "scoop",
                "--scoopc",
                tools / "scoopc",
                "--work-dir",
                local / "base",
                "--cache-dir",
                cache,
            ],
            check=True,
        )
        for tool, location, options in (
            ("build_text_backends.py", "text", []),
            ("build_skia.py", "skia", ["--gn", args.gn]),
        ):
            subprocess.run(
                [
                    sys.executable,
                    scripts / tool,
                    *common,
                    "--build-dir",
                    work / location / target,
                    "--prefix",
                    sysroot / "lib" / target,
                    *options,
                ],
                check=True,
            )
        build_cones(
            sysroot,
            target,
            cones,
            (tools / "scoop", tools / "scoopc"),
            local,
            cache,
            args.jobs,
            os.environ,
        )
    if args.package_dir:
        destination = args.package_dir.resolve()
        destination.mkdir(parents=True, exist_ok=True)
        base, gui = split_gui_cones(sysroot, cones, target)
        for name, contents, assemble in (
            ("base", base, assemble_base_libraries),
            ("gui", gui, assemble_gui_libraries),
        ):
            with tempfile.TemporaryDirectory(dir=destination) as staging:
                folder = Path(staging)
                assemble(folder, target, args, contents)
                archive(folder, destination / f"scoop-local-lib-{name}-{target}.tar.xz")


if __name__ == "__main__":
    main()
