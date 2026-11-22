#!/usr/bin/env python3
"""Build and exercise an iOS native-app in two real Simulator host processes."""

import argparse
import os
import plistlib
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
MODULE = "ScoopProbe"
TARGETS = {"simulator": "aarch64-apple-ios-sim", "device": "aarch64-apple-ios"}


def run(*args, **options):
    return subprocess.run(args, check=True, **options)


def build_library(work, sysroot, binaries, linker, target):
    work.mkdir()
    name = work / MODULE
    run(
        binaries / "scoop",
        "build",
        HERE / "cone",
        "--release",
        "--enable-gc-stress",
        "--target",
        target,
        "--sysroot",
        sysroot,
        "--scoopc",
        binaries / "scoopc",
        "--target-dir",
        work / "target",
        "--cache-dir",
        ROOT / "target/ios-fixture-cache",
        "--lockfile",
        work / "Cone.lock",
        "--install-name",
        f"@rpath/{MODULE}.framework/{MODULE}",
        "--linker",
        linker,
        "-o",
        name,
    )
    exports = subprocess.check_output(["nm", "-gU", name], text=True).splitlines()
    assert len(exports) == 1 and exports[0].split()[-1] == "_scoop_app_init"
    return name


def build_xcframework(work, sysroot, binaries, linker, variants):
    inputs = []
    for variant in variants:
        library = build_library(work / variant, sysroot, binaries, linker, TARGETS[variant])
        inputs.extend([f"--{variant}", library])
    xcframework = work / f"{MODULE}.xcframework"
    run(
        sys.executable,
        ROOT / "scripts/package_ios_xcframework.py",
        "--name",
        MODULE,
        "--bundle-id",
        "dev.scoop.ios-probe",
        *inputs,
        "--header",
        HERE / "ScoopHostApi.h",
        "--output",
        xcframework,
    )
    libraries = plistlib.loads((xcframework / "Info.plist").read_bytes())["AvailableLibraries"]
    assert len(libraries) == len(variants)
    frameworks = {}
    for library in libraries:
        assert library["SupportedPlatform"] == "ios"
        assert library["SupportedArchitectures"] == ["arm64"]
        variant = library.get("SupportedPlatformVariant", "device")
        frameworks[variant] = xcframework / library["LibraryIdentifier"] / library["LibraryPath"]
    assert frameworks.keys() == set(variants)
    return frameworks


def build_app(framework, work, language, simulator):
    sdk_name = "iphonesimulator" if simulator else "iphoneos"
    sdk = subprocess.check_output(
        ["xcrun", "--sdk", sdk_name, "--show-sdk-path"], text=True
    ).strip()
    minimum = plistlib.loads((framework / "Info.plist").read_bytes())["MinimumOSVersion"]
    triple = f"arm64-apple-ios{minimum}" + ("-simulator" if simulator else "")
    app = work / f"{language}.app"
    identifier = f"dev.scoop.ios-probe.{language}"
    embedded = app / "Frameworks" / framework.name
    shutil.copytree(framework, embedded)
    with (app / "Info.plist").open("wb") as file:
        plistlib.dump(
            {
                "CFBundleExecutable": language,
                "CFBundleIdentifier": identifier,
                "CFBundleName": language,
                "CFBundlePackageType": "APPL",
                "CFBundleShortVersionString": "1.0",
                "CFBundleSupportedPlatforms": ["iPhoneSimulator" if simulator else "iPhoneOS"],
                "CFBundleVersion": "1",
                "LSRequiresIPhoneOS": True,
                "MinimumOSVersion": minimum,
                "UIDeviceFamily": [1, 2],
            },
            file,
        )
    if language == "swift":
        compiler = (
            "swiftc",
            "-sdk",
            sdk,
            "-module-cache-path",
            work / "module-cache",
            HERE / "main.swift",
        )
    else:
        compiler = (
            "clang",
            "-isysroot",
            sdk,
            "-fobjc-arc",
            "-fmodules",
            f"-fmodules-cache-path={work / 'module-cache'}",
            "-framework",
            "Foundation",
            HERE / "main.m",
        )
    run(
        "xcrun",
        "--sdk",
        sdk_name,
        *compiler,
        "-target",
        triple,
        "-F",
        framework.parent,
        "-framework",
        MODULE,
        "-Xlinker",
        "-rpath",
        "-Xlinker",
        "@executable_path/Frameworks",
        "-o",
        app / language,
    )
    if simulator:
        for bundle in (embedded, app):
            run("codesign", "--force", "--sign", "-", bundle)
    return app, identifier


def exercise(simulator, identifier, expected, *args):
    output = run(
        "xcrun",
        "simctl",
        "launch",
        "--console",
        "--terminate-running-process",
        simulator,
        identifier,
        *args,
        env=dict(os.environ, SIMCTL_CHILD_SCOOP_GC_STRESS_MOVE="1"),
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=45,
    ).stdout
    print(output, end="", flush=True)
    if expected not in output.splitlines():
        raise RuntimeError(f"{identifier} did not report {expected}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--simulator", required=True, help="booted Simulator UDID")
    parser.add_argument("--work-dir", type=Path, required=True)
    parser.add_argument("--sysroot", type=Path, default=ROOT / "sysroot")
    parser.add_argument("--binaries", type=Path, default=ROOT / "target/release")
    parser.add_argument("--linker", type=Path, required=True, help="LLVM 22.1 Mach-O LLD")
    parser.add_argument(
        "--include-device", action="store_true", help="also build the device slice and hosts"
    )
    args = parser.parse_args()
    work = args.work_dir.resolve()
    work.mkdir(parents=True, exist_ok=False)
    variants = ("simulator", "device") if args.include_device else ("simulator",)
    frameworks = build_xcframework(
        work, args.sysroot.resolve(), args.binaries.resolve(), args.linker, variants
    )
    for language in ("swift", "objc"):
        app, identifier = build_app(frameworks["simulator"], work / "simulator", language, True)
        run("xcrun", "simctl", "install", args.simulator, app)
        exercise(args.simulator, identifier, f"scoop-{language}-moving-gc-ok")
        if language == "swift":
            exercise(args.simulator, identifier, "scoop-swift-init-failure-ok", "--fail")
        if args.include_device:
            app, _ = build_app(frameworks["device"], work / "device", language, False)
            print(f"Built device host for development signing: {app}")
    print(f"Verified {work / f'{MODULE}.xcframework'}")


if __name__ == "__main__":
    main()
