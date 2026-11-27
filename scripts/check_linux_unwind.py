#!/usr/bin/env python3
"""Run the actual Scoop personality with LLVM libunwind for all Linux modes."""

import argparse
import os
import subprocess
from pathlib import Path

from target_catalog import TARGETS, current_host, linux_compiler_builtins, native_configuration

ROOT = Path(__file__).resolve().parent.parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sysroot", type=Path, default=ROOT / "sysroot")
    parser.add_argument("--llvm-prefix", type=Path, default=os.environ.get("LLVM_SYS_221_PREFIX"))
    parser.add_argument("--gnu-cc", default="gcc")
    parser.add_argument("--musl-cc", default="musl-gcc")
    linux_targets = [name for name, target in TARGETS.items() if target["os"] == "linux"]
    parser.add_argument("--target", choices=linux_targets, help="run a single native Linux target")
    parser.add_argument("--work-dir", type=Path, default=ROOT / "tmp/unwind-tests")
    args = parser.parse_args()
    if args.llvm_prefix is None:
        parser.error("set LLVM_SYS_221_PREFIX or --llvm-prefix to LLVM 22.1")
    host = current_host()
    if host is None or host["os"] != "linux":
        parser.error("run the unwind probes on a supported Linux host")
    targets = [name for name in linux_targets if TARGETS[name]["arch"] == host["arch"]]
    if args.target is not None:
        if args.target not in targets:
            parser.error(f"{args.target} cannot execute on this Linux {host['arch']} host")
        targets = [args.target]
    llc = args.llvm_prefix.resolve() / "bin/llc"
    version = subprocess.check_output([llc, "--version"], text=True)
    if "LLVM version 22.1." not in version:
        parser.error(f"{llc} must be LLVM 22.1")
    for target_name in targets:
        target = native_configuration(target_name)
        musl = target["native_environment"] == "LinuxMusl"
        cc = args.musl_cc if musl else args.gnu_cc
        modes = (
            (
                ("static", ["-static", "-fno-pie", "-no-pie"]),
                ("dynamic", ["-fPIE", "-pie"]),
            )
            if musl
            else (("dynamic", ["-fPIE", "-pie"]),)
        )
        prefix = args.sysroot.resolve() / "lib" / target_name / "unwind"
        for mode, flags in modes:
            check_mode(args, llc, target_name, target, musl, mode, cc, flags, prefix)


def check_mode(args, llc, target_name, target, musl, mode, cc, flags, prefix):
    type_encoding = {"aarch64": "0x9c", "x86_64": "0x9b"}[target["arch"]]
    build = args.work_dir.resolve() / target_name / mode
    build.mkdir(parents=True, exist_ok=True)
    environment = dict(os.environ, TMPDIR=str(build))
    obj, binary, link_map = build / "frames.o", build / "unwind-test", build / "link.map"
    builtins = linux_compiler_builtins(target_name)
    subprocess.run(
        [
            llc,
            f"-mtriple={target['llvm_triple']}",
            "-O=2",
            "-relocation-model=pic",
            "-filetype=obj",
            str(ROOT / "runtime/tests/eh_unwind_test.ll"),
            "-o",
            str(obj),
        ],
        check=True,
        env=environment,
    )
    subprocess.run(
        [
            cc,
            "-std=c11",
            "-Wall",
            "-Wextra",
            "-Werror",
            "-O2",
            "-fno-omit-frame-pointer",
            "-fno-optimize-sibling-calls",
            "-funwind-tables",
            # Expected encodings for the LLVM EH test input.
            "-DSCOOP_EH_LP_START_ENCODING=0xff",
            f"-DSCOOP_EH_TTYPE_ENCODING={type_encoding}",
            "-DSCOOP_EH_CALL_SITE_ENCODING=0x01",
            "-DSCOOP_EH_OMITTED_TTYPE_ENCODING=0xff",
            "-I",
            str(prefix / "include"),
            "-I",
            str(ROOT / "runtime/src"),
            str(ROOT / "runtime/tests/eh_unwind_test.c"),
            str(ROOT / "runtime/src/eh_personality.c"),
            str(ROOT / "runtime/src/eh/lsda.c"),
            str(obj),
            "-Wl,--start-group",
            str(prefix / "lib/libunwind.a"),
            "-pthread",
            "-nodefaultlibs",
            "-Wl,--eh-frame-hdr",
            f"-Wl,-Map,{link_map}",
            *flags,
            "-lc",
            str(prefix / "lib" / builtins) if builtins else "-lgcc",
            "-Wl,--end-group",
            "-o",
            str(binary),
        ],
        check=True,
        env=environment,
    )
    result = subprocess.check_output([binary], text=True, env=environment)
    if result != "cleanup, catch and delete passed\n":
        raise RuntimeError(f"unexpected EH result for {target_name}/{mode}: {result!r}")
    inputs = link_map.read_text()
    if "libgcc_eh.a" in inputs or "libgcc_s.so" in inputs:
        raise RuntimeError(f"a second unwinder was linked: {link_map}")
    elf = subprocess.check_output(["readelf", "-l", "-d", binary], text=True)
    if mode == "static":
        if "INTERP" in elf or "NEEDED" in elf:
            raise RuntimeError(f"static output has dynamic dependencies: {binary}")
    elif "INTERP" not in elf or ("ld-musl-" in elf) != musl:
        raise RuntimeError(f"incorrect libc interpreter: {binary}")
    print(f"{target_name}/{mode}: {result.strip()}")


if __name__ == "__main__":
    main()
