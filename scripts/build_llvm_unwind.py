#!/usr/bin/env python3
"""Build the LLVM unwind and compiler support archives for a Linux target."""

import argparse
import os
import shlex
import shutil
import subprocess
import tomllib
from pathlib import Path

from build_support import (
    RELEASE_CONFIG,
    ROOT,
    configure_cmake,
    cpu_flag,
    metadata,
    reusable,
    save_record,
    source_metadata,
)
from target_catalog import TARGETS as CATALOG
from target_catalog import linux_compiler_builtins, native_configuration

TARGETS = tuple(name for name, target in CATALOG.items() if target["os"] == "linux")
PATCH = ROOT / "sysroot/vendor/llvm-project/22.1.8/patches/llvm-libunwind-eh-index-miss.patch"


def prepare_sources(source: Path, build: Path, environment: dict) -> Path:
    """Apply the ELF index-miss fix without editing the user's LLVM checkout."""
    overlay = build / "source"
    overlay.mkdir(parents=True, exist_ok=True)
    for name in ("libunwind", "runtimes"):
        destination = overlay / name
        if destination.exists():
            shutil.rmtree(destination)
        shutil.copytree(source / name, destination)
    for name in ("cmake", "llvm", "third-party"):
        destination = overlay / name
        if destination.is_symlink():
            destination.unlink()
        destination.symlink_to(source / name, target_is_directory=True)
    subprocess.run(
        [
            "patch",
            "--batch",
            "--forward",
            "-p1",
            "-i",
            str(PATCH),
        ],
        cwd=overlay,
        env=environment,
        check=True,
    )
    return overlay


def build_and_install(configure, objects, install_target, jobs, environment):
    configure_cmake([*configure, "-B", str(objects)], env=environment)
    subprocess.run(
        ["cmake", "--build", str(objects), "--target", install_target, "--parallel", str(jobs)],
        check=True,
        env=environment,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=TARGETS, required=True)
    parser.add_argument(
        "--source",
        type=Path,
        default=ROOT / "sysroot/vendor/llvm-project/22.1.8/llvm-project-22.1.8.src",
    )
    parser.add_argument("--build-dir", type=Path)
    parser.add_argument("--prefix", type=Path)
    parser.add_argument("--cc", default="clang", help="Clang C/assembly compiler")
    parser.add_argument("--cxx", default="clang++", help="Clang C++ compiler")
    parser.add_argument("--musl-include", type=Path, help="musl's installed C header directory")
    parser.add_argument("--jobs", type=int, default=min(os.cpu_count() or 1, 8))
    args = parser.parse_args()
    if args.jobs < 1:
        parser.error("--jobs must be positive")
    source = args.source.resolve()
    for relative in ("runtimes/CMakeLists.txt", "libunwind/include/unwind.h"):
        if not (source / relative).is_file():
            parser.error(f"LLVM source is missing {relative}: {source}")
    cc, cxx = shutil.which(args.cc), shutil.which(args.cxx)
    if not cc or not cxx:
        parser.error("--cc and --cxx must name installed Clang compilers")
    target = native_configuration(args.target)
    musl = target["native_environment"] == "LinuxMusl"
    if musl and (args.musl_include is None or not (args.musl_include / "features.h").is_file()):
        parser.error("musl requires --musl-include pointing to its installed C headers")
    if not musl and args.musl_include is not None:
        parser.error("--musl-include is only valid for the musl target")

    build = (args.build_dir or ROOT / "tmp/unwind" / args.target).resolve()
    prefix = (args.prefix or ROOT / "sysroot/lib" / args.target / "unwind").resolve()
    builtins = linux_compiler_builtins(args.target)
    source_dirs = ["libunwind", "runtimes", "cmake", "llvm/cmake", "llvm/include", "third-party"]
    if builtins:
        source_dirs.extend(["compiler-rt/lib/builtins", "compiler-rt/cmake"])
    inputs = {
        "source": source_metadata(source, source_dirs),
        "tools": [
            metadata(path)
            for path in (
                cc,
                cxx,
                __file__,
                ROOT / "scripts/build_support.py",
                PATCH,
                RELEASE_CONFIG,
            )
        ],
        "target": args.target,
        "target_configuration": target,
        "prefix": str(prefix),
        "musl_include": str(args.musl_include.resolve()) if musl else None,
    }
    record = build / "inputs.json"
    outputs = [prefix / "include/unwind.h", prefix / "lib/libunwind.a"]
    if builtins:
        inputs["compiler_rt_license"] = metadata(source / "compiler-rt/LICENSE.TXT")
        license_file = prefix / "share/compiler-rt/LICENSE.TXT"
        outputs.extend([prefix / "lib" / builtins, license_file])
    if reusable(record, inputs, outputs):
        print(f"Reusing {args.target} LLVM libunwind: {prefix}")
        return
    release = tomllib.loads(RELEASE_CONFIG.read_text())
    cpu = cpu_flag(target["arch"], release["targets"][args.target]["cpu"])
    temporary = build / "scratch"
    temporary.mkdir(parents=True, exist_ok=True)
    environment = dict(os.environ, TMPDIR=str(temporary))
    prepared = prepare_sources(source, build, environment)
    configure = [
        "cmake",
        "-G",
        "Unix Makefiles",
        "-S",
        str(prepared / "runtimes"),
        "-DLLVM_ENABLE_RUNTIMES=libunwind",
        "-DLLVM_INCLUDE_TESTS=OFF",
        "-DLLVM_INCLUDE_DOCS=OFF",
        "-DLIBUNWIND_INCLUDE_TESTS=OFF",
        "-DLIBUNWIND_INCLUDE_DOCS=OFF",
        "-DLIBUNWIND_ENABLE_SHARED=OFF",
        "-DLIBUNWIND_ENABLE_STATIC=ON",
        "-DLIBUNWIND_ENABLE_THREADS=ON",
        "-DLIBUNWIND_ENABLE_CROSS_UNWINDING=OFF",
        "-DLIBUNWIND_HIDE_SYMBOLS=ON",
        "-DCMAKE_POSITION_INDEPENDENT_CODE=ON",
        "-DCMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY",
        "-DLIBUNWIND_INSTALL_LIBRARY_DIR=lib",
        "-DLIBUNWIND_INSTALL_INCLUDE_DIR=include",
    ]
    toolchain = ["-DCMAKE_BUILD_TYPE=Release", f"-DCMAKE_INSTALL_PREFIX={prefix}"]
    for language, compiler in (("C", cc), ("CXX", cxx), ("ASM", cc)):
        toolchain.extend(
            [
                f"-DCMAKE_{language}_COMPILER={compiler}",
                f"-DCMAKE_{language}_COMPILER_TARGET={target['llvm_triple']}",
            ]
        )
    flags = [cpu]
    if musl:
        resource = subprocess.check_output([cc, "-print-resource-dir"], text=True).strip()
        flags += [
            "-nostdinc",
            "-isystem",
            str(Path(resource) / "include"),
            "-isystem",
            str(args.musl_include.resolve()),
        ]
    for language in ("C", "CXX", "ASM"):
        selected = flags + (["-nostdinc++"] if musl and language == "CXX" else [])
        toolchain.append(f"-DCMAKE_{language}_FLAGS={shlex.join(selected)}")
    build_and_install(
        [*configure, *toolchain], build / "objects", "install-unwind", args.jobs, environment
    )
    if builtins:
        build_and_install(
            [
                "cmake",
                "-G",
                "Unix Makefiles",
                "-S",
                str(source / "compiler-rt/lib/builtins"),
                "-DCMAKE_SYSTEM_NAME=Linux",
                f"-DCMAKE_SYSTEM_PROCESSOR={target['arch']}",
                "-DCOMPILER_RT_DEFAULT_TARGET_ONLY=ON",
                f"-DCOMPILER_RT_INSTALL_LIBRARY_DIR={prefix / 'lib'}",
                *toolchain,
            ],
            build / "builtins",
            f"install-clang_rt.builtins-{target['arch']}",
            args.jobs,
            environment,
        )
        license_file.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source / "compiler-rt/LICENSE.TXT", license_file)
    save_record(record, inputs, build / "objects")
    print(f"Installed {args.target} LLVM runtime archives in {prefix}")
    print("Verify the target libc with the EH link/run test before using this archive.")


if __name__ == "__main__":
    main()
