"""Clang/LLVM companions for the selected Windows native environment."""

import os
import shutil
import tomllib
from pathlib import Path

from scripts.target_catalog import native_configuration

from .model import EnvironmentError

_MSVC_LIBRARY_SLICE_BY_ARCH = {"x86_64": "x64"}


def _msvc_library_slice(architecture):
    try:
        return _MSVC_LIBRARY_SLICE_BY_ARCH[architecture]
    except KeyError as error:
        raise EnvironmentError(
            f"no Windows fixture toolchain machine for architecture {architecture!r}"
        ) from error


def companions(target, args, sysroot, needed, scoop):
    configuration = native_configuration(target)
    if configuration["os"] != "windows":
        raise EnvironmentError(f"target {target!r} does not use Windows fixture tools")
    microsoft = configuration["native_environment"] == "MicrosoftUcrt"
    library_slice = _msvc_library_slice(configuration["arch"])
    root = (args.native_sysroot or sysroot / "lib" / target / "toolchain").resolve()
    installed = scoop.resolve().parent.parent
    bundled = installed / "libexec/llvm"
    split = not microsoft and not args.native_sysroot and bundled.is_dir()
    llvm = os.environ.get("LLVM_SYS_221_PREFIX")
    tool_root = root
    if microsoft:
        tool_root = bundled if bundled.is_dir() else Path(llvm) if llvm else None
    elif split:
        tool_root = bundled
    result = {"cc_args": []}
    for name, filename in [("cc", "clang.exe"), ("ar", "llvm-ar.exe")]:
        if name not in needed:
            continue
        configured = getattr(args, name)
        program = str(configured or (tool_root / "bin" / filename if tool_root else filename))
        found = shutil.which(program)
        if found is None:
            raise EnvironmentError(f"required {target} tool is missing: {program}")
        result[name] = found
    if "cc" not in needed:
        return result
    if not microsoft:
        result["cc_args"] = [
            "--target=" + configuration["llvm_triple"],
            "--sysroot=" + str(root),
        ]
        if split:
            resources = Path("lib/clang/22")
            result["cc_link_args"] = ["-fuse-ld=lld", "-rtlib=compiler-rt", "-unwindlib=none"]
            result["cc_args"] += ["-resource-dir=" + str(root / resources), "-nostdinc"]
            for include in (
                root / "include",
                bundled / resources / "include",
                installed / "libexec/mingw/include",
            ):
                result["cc_args"] += ["-isystem", str(include)]
        return result
    # The GNU driver otherwise adds libcmt even when cc1 selected /MD.
    # Source objects already declare the matching CRT and startup libraries.
    result["cc_args"] = [
        "--target=" + configuration["llvm_triple"],
        "-fms-runtime-lib=dll",
        "--start-no-unused-arguments",
        "-nostartfiles",
        "--end-no-unused-arguments",
    ]
    config = root / "msvc.toml"
    if args.native_sysroot or config.is_file():
        try:
            msvc = tomllib.loads(config.read_text(encoding="utf-8"))["msvc"]
            tools = root / msvc["vc_tools"]
            sdk = root / msvc["windows_sdk"]
            include = sdk / "Include" / msvc["sdk_version"]
            libraries = sdk / "Lib" / msvc["sdk_version"]
        except (KeyError, TypeError) as error:
            raise EnvironmentError(
                f"{config}: expected msvc.vc_tools, windows_sdk and sdk_version paths"
            ) from error
        for path in [
            tools / "include",
            *(include / part for part in ["ucrt", "shared", "um", "winrt"]),
        ]:
            result["cc_args"] += ["-idirafter", str(path)]
        for path in [
            tools / "lib" / library_slice,
            libraries / "ucrt" / library_slice,
            libraries / "um" / library_slice,
        ]:
            result["cc_args"] += ["-L", str(path)]
    return result
