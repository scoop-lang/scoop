#!/usr/bin/env python3
"""Package a linked iOS native-app Mach-O for Swift and Objective-C hosts."""

import argparse
import plistlib
import re
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def binary_version(binary, name, platform):
    build = subprocess.check_output(["xcrun", "vtool", "-show-build", binary], text=True)
    platforms = re.findall(r"^\s*platform\s+(\S+)\s*$", build, re.MULTILINE)
    versions = re.findall(r"^\s*minos\s+(\S+)\s*$", build, re.MULTILINE)
    if platforms != [platform] or len(versions) != 1:
        raise ValueError(f"{binary} must be a single {platform} binary with a deployment version")
    install_names = subprocess.check_output(["otool", "-D", binary], text=True).splitlines()[1:]
    expected = f"@rpath/{name}.framework/{name}"
    if [line.strip() for line in install_names] != [expected]:
        raise ValueError(f"{binary} must have install name {expected}")
    return versions[0]


def assemble_framework(directory, binary, name, bundle_id, headers, supported_platform, platform):
    minimum = binary_version(binary, name, platform)
    framework = directory / f"{name}.framework"
    public = framework / "Headers"
    public.mkdir(parents=True)
    modules = framework / "Modules"
    modules.mkdir()
    shutil.copy2(binary, framework / name)
    for header in headers:
        shutil.copy2(header, public / header.name)
    module_headers = "\n".join(f'  header "{header.name}"' for header in headers)
    (modules / "module.modulemap").write_text(
        f"framework module {name} {{\n{module_headers}\n  export *\n}}\n", encoding="utf-8"
    )
    with (framework / "Info.plist").open("wb") as file:
        plistlib.dump(
            {
                "CFBundleExecutable": name,
                "CFBundleIdentifier": bundle_id,
                "CFBundleInfoDictionaryVersion": "6.0",
                "CFBundleName": name,
                "CFBundlePackageType": "FMWK",
                "CFBundleShortVersionString": "1.0",
                "CFBundleSupportedPlatforms": [supported_platform],
                "CFBundleVersion": "1",
                "MinimumOSVersion": minimum,
            },
            file,
        )
    return framework


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--name", required=True)
    parser.add_argument("--bundle-id", required=True)
    parser.add_argument("--simulator", type=Path, help="linked simulator Mach-O")
    parser.add_argument("--device", type=Path, help="linked device Mach-O")
    parser.add_argument("--header", type=Path, action="append", default=[])
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.simulator is None and args.device is None:
        parser.error("provide --simulator, --device, or both")
    if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", args.name):
        parser.error("--name must be a C module identifier")
    if not re.fullmatch(r"[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)+", args.bundle_id):
        parser.error("--bundle-id must be a reverse-DNS identifier")
    headers = [ROOT / "runtime/include/scoop_app.h", *args.header]
    names = [header.name for header in headers]
    if len(set(names)) != len(names) or any(
        not re.fullmatch(r"[\w.-]+\.h", name) for name in names
    ):
        parser.error("public headers must have distinct .h filenames")
    output = args.output.absolute()
    if output.suffix != ".xcframework" or output.exists():
        parser.error("--output must name a new .xcframework")

    output.parent.mkdir(parents=True, exist_ok=True)
    try:
        with tempfile.TemporaryDirectory(dir=output.parent, prefix=".xcframework-") as temporary:
            work = Path(temporary)
            frameworks = []
            for binary, supported, platform in (
                (args.simulator, "iPhoneSimulator", "IOSSIMULATOR"),
                (args.device, "iPhoneOS", "IOS"),
            ):
                if binary is not None:
                    framework = assemble_framework(
                        work / supported,
                        binary,
                        args.name,
                        args.bundle_id,
                        headers,
                        supported,
                        platform,
                    )
                    frameworks.extend(["-framework", framework])
            staged = work / output.name
            subprocess.run(
                ["xcodebuild", "-create-xcframework", *frameworks, "-output", staged],
                check=True,
            )
            staged.rename(output)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"{parser.prog}: {error}\n")


if __name__ == "__main__":
    main()
