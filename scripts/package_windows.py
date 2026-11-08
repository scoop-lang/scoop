"""Select Windows host compilers and install their native development resources."""

import filecmp
import os
import shutil
from pathlib import Path

import package_licenses
from build_llvm import VERSION
from target_catalog import llvm_mingw_resource_triple, native_configuration, native_target_for_host


def arguments(parser):
    parser.add_argument(
        "--mingw-msvcrt", type=Path, help="LLVM-MinGW MSVCRT prefix for Windows packaging"
    )
    parser.add_argument(
        "--mingw-ucrt", type=Path, help="LLVM-MinGW UCRT prefix for Windows packaging"
    )
    parser.add_argument(
        "--windows-perl", default="perl", help="Native Windows Perl for MSVC OpenSSL"
    )
    parser.add_argument("--msys-perl", default="perl", help="MSYS Perl for MinGW OpenSSL")
    parser.add_argument("--msys-make", default="make", help="GNU Make for MinGW OpenSSL")
    parser.add_argument("--msys-shell", default="sh", help="MSYS shell for MinGW OpenSSL")


def mingw_prefix(args, target):
    if native_configuration(target)["native_environment"] == "LlvmMingwMsvcrt":
        return args.mingw_msvcrt
    if native_configuration(target)["native_environment"] == "LlvmMingwUcrt":
        return args.mingw_ucrt
    return None


def host_environment(args, environment):
    args.mingw_msvcrt = args.mingw_msvcrt.resolve(strict=True)
    args.mingw_ucrt = args.mingw_ucrt.resolve(strict=True)
    binary = args.mingw_ucrt / "bin"
    environment["CARGO_BUILD_TARGET"] = args.host
    environment["CARGO_TARGET_X86_64_PC_WINDOWS_GNULLVM_LINKER"] = str(binary / "clang.exe")
    environment["CC_x86_64_pc_windows_gnullvm"] = str(binary / "clang.exe")
    environment["CXX_x86_64_pc_windows_gnullvm"] = str(binary / "clang++.exe")
    environment["AR_x86_64_pc_windows_gnullvm"] = str(binary / "llvm-ar.exe")
    environment["PATH"] = str(binary) + os.pathsep + environment.get("PATH", "")
    # Build scripts and proc macros still use the MSVC Rust host toolchain.
    # Git Bash puts its unrelated link.exe ahead of the Visual Studio tools.
    if vc_tools := environment.get("VCTOOLSINSTALLDIR"):
        environment["CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER"] = str(
            Path(vc_tools) / "bin/Hostx64/x64/link.exe"
        )


def compiler(args, target):
    prefix = mingw_prefix(args, target)
    if prefix is not None:
        return prefix / "bin/clang.exe"
    return args.llvm_prefix / "bin/clang-cl.exe"


def openssl_arguments(args, target):
    if mingw_prefix(args, target) is None:
        return ["--perl", args.windows_perl]
    return ["--perl", args.msys_perl, "--make", args.msys_make, "--shell", args.msys_shell]


CONFIG_HEADERS = (Path("_mingw.h"), Path("c++/v1/__config_site"))


def common_headers(prefix):
    include = prefix / "include"
    return {
        path.relative_to(include): path
        for path in include.rglob("*")
        if path.is_file() and path.relative_to(include) not in CONFIG_HEADERS
    }


def stage_shared_headers(root, args):
    """Ship common MinGW/libc++ headers once, without either target configuration."""
    msvcrt, ucrt = common_headers(args.mingw_msvcrt), common_headers(args.mingw_ucrt)
    different = msvcrt.keys() ^ ucrt.keys()
    different |= {
        name
        for name in msvcrt.keys() & ucrt.keys()
        if not filecmp.cmp(msvcrt[name], ucrt[name], shallow=False)
    }
    if different:
        raise ValueError(
            "MinGW shared headers differ: " + ", ".join(map(str, sorted(different)[:5]))
        )
    for name, source in ucrt.items():
        destination = root / "libexec/mingw/include" / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, destination)
    _, host = native_target_for_host(args.host)
    package_licenses.collect_mingw(root, args.mingw_ucrt, "mingw-headers", host["arch"])


def stage_target_sdk(root, target, args):
    prefix = mingw_prefix(args, target)
    if prefix is None:
        return
    selected = llvm_mingw_resource_triple(native_configuration(target)["arch"])
    destination = root / "lib" / target / "toolchain"
    for name in CONFIG_HEADERS:
        installed = destination / "include" / name
        installed.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(prefix / "include" / name, installed)
    libraries = destination / selected / "lib"
    libraries.mkdir(parents=True)
    for path in (prefix / selected / "lib").iterdir():
        if path.suffix in (".a", ".lib", ".o") and not path.name.startswith("libunwind"):
            shutil.copy2(path, libraries / path.name)
    resources = Path("lib/clang") / VERSION.split(".")[0] / "lib/windows"
    (destination / resources).mkdir(parents=True)
    builtins = "libclang_rt.builtins-" + native_configuration(target)["arch"] + ".a"
    shutil.copy2(prefix / resources / builtins, destination / resources / builtins)
    package_licenses.collect_mingw(
        root, prefix, "native-" + target, native_configuration(target)["arch"]
    )
