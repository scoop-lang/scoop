#!/usr/bin/env python3
"""Build and exercise an iOS native-app in real Simulator host processes."""

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
PROBES = {
    "callbacks": (HERE / "cone", HERE / "ScoopHostApi.h"),
    "protected": (HERE / "protected-data/cone", HERE / "protected-data/ProtectedDataApi.h"),
    "platform": (HERE / "platform/cone", HERE / "platform/PlatformHost.h"),
    "graphics": (HERE / "graphics/cone", HERE / "graphics/GraphicsResult.h"),
    "window": (HERE / "window/cone", HERE / "window/WindowHost.h"),
    "surface": (HERE / "surface/cone", HERE / "surface/SurfaceHost.h"),
}


SCENE_DELEGATES = {
    "protected": "ProtectedDataScene",
    "window": "WindowProbeScene",
    "surface": "SurfaceProbeScene",
}


def run(*args, **options):
    return subprocess.run(args, check=True, **options)


def build_library(work, sysroot, binaries, linker, target, probe):
    work.mkdir()
    name = work / MODULE
    run(
        binaries / "scoop",
        "build",
        PROBES[probe][0],
        "--release",
        *(["--enable-gc-stress"] if probe == "callbacks" else []),
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


def build_xcframework(work, sysroot, binaries, linker, variants, probe):
    inputs = []
    for variant in variants:
        library = build_library(work / variant, sysroot, binaries, linker, TARGETS[variant], probe)
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
        PROBES[probe][1],
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
                "CFBundleDevelopmentRegion": "en",
                "CFBundleExecutable": language,
                "CFBundleIdentifier": identifier,
                "CFBundleName": language,
                "CFBundlePackageType": "APPL",
                "CFBundleShortVersionString": "1.0",
                "CFBundleSupportedPlatforms": ["iPhoneSimulator" if simulator else "iPhoneOS"],
                "CFBundleVersion": "1",
                **(
                    {
                        "UIApplicationSceneManifest": {
                            "UIApplicationSupportsMultipleScenes": False,
                            "UISceneConfigurations": {
                                "UIWindowSceneSessionRoleApplication": [
                                    {
                                        "UISceneConfigurationName": language,
                                        "UISceneDelegateClassName": SCENE_DELEGATES[language],
                                    }
                                ]
                            },
                        }
                    }
                    if language in SCENE_DELEGATES
                    else {}
                ),
                "LSRequiresIPhoneOS": True,
                "MinimumOSVersion": minimum,
                "UIDeviceFamily": [1, 2],
            },
            file,
        )
    if language == "platform":
        shutil.copyfile(HERE / "platform/resource.txt", app / "resource.txt")
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
            "UIKit" if language in SCENE_DELEGATES else "Foundation",
            (PROBES[language][1].parent if language in PROBES else HERE) / "main.m",
            *([HERE / "window/ime.m"] if language == "window" else []),
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


def exercise(simulator, identifier, expected, *args, gc_stress=True):
    output = run(
        "xcrun",
        "simctl",
        "launch",
        "--console",
        "--terminate-running-process",
        simulator,
        identifier,
        *args,
        env=dict(os.environ, SIMCTL_CHILD_SCOOP_GC_STRESS_MOVE="1") if gc_stress else None,
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
    probes = parser.add_mutually_exclusive_group()
    probes.add_argument(
        "--protected-data",
        action="store_const",
        dest="probe",
        const="protected",
        help="exercise scoop.os.ios in a UIKit application",
    )
    probes.add_argument(
        "--platform",
        action="store_const",
        dest="probe",
        const="platform",
        help="exercise scoop.platform in the application container",
    )
    parser.set_defaults(probe="callbacks")
    probes.add_argument(
        "--graphics",
        action="store_const",
        dest="probe",
        const="graphics",
        help="draw and read back shaped text and SVG using the installed Scoop libraries",
    )
    probes.add_argument(
        "--window",
        action="store_const",
        dest="probe",
        const="window",
        help="exercise a real UIWindowScene with the installed Scoop window library",
    )
    probes.add_argument(
        "--surface",
        action="store_const",
        dest="probe",
        const="surface",
        help="paint accepted Skia frames in a real UIKit SurfaceHost",
    )
    args = parser.parse_args()
    work = args.work_dir.resolve()
    work.mkdir(parents=True, exist_ok=False)
    variants = ("simulator", "device") if args.include_device else ("simulator",)
    frameworks = build_xcframework(
        work,
        args.sysroot.resolve(),
        args.binaries.resolve(),
        args.linker,
        variants,
        args.probe,
    )
    for language in ("swift", "objc") if args.probe == "callbacks" else (args.probe,):
        app, identifier = build_app(frameworks["simulator"], work / "simulator", language, True)
        run("xcrun", "simctl", "install", args.simulator, app)
        if args.probe != "callbacks":
            marker = "protected-data" if language == "protected" else language
            exercise(args.simulator, identifier, f"scoop-ios-{marker}-ok", gc_stress=False)
        else:
            exercise(args.simulator, identifier, f"scoop-{language}-moving-gc-ok")
        if language == "window":
            exercise(
                args.simulator,
                identifier,
                "scoop-ios-ime-ok",
                "--ime-transactions",
                gc_stress=False,
            )
        if language == "swift":
            exercise(args.simulator, identifier, "scoop-swift-init-failure-ok", "--fail")
        if args.include_device:
            app, _ = build_app(frameworks["device"], work / "device", language, False)
            print(f"Built device host for development signing: {app}")
    print(f"Verified {work / f'{MODULE}.xcframework'}")


if __name__ == "__main__":
    main()
