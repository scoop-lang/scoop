#!/usr/bin/env python3
"""Stage the Linux GTK/libadwaita development inputs selected by pkg-config."""

import argparse
import json
import os
import shlex
import shutil
import subprocess
import tempfile
import tomllib
from pathlib import Path

from build_support import RELEASE_CONFIG, ROOT, build_lock, metadata, reusable

TARGET = "x86_64-unknown-linux-gnu"
LIBRARIES = {
    "libadwaita-1": "adwaita-1",
    "gtk4": "gtk-4",
    "gio-2.0": "gio-2.0",
    "gobject-2.0": "gobject-2.0",
    "glib-2.0": "glib-2.0",
    "pango": "pango-1.0",
}
LINK_INPUTS = tuple(f"lib{name}.so" for name in LIBRARIES.values())
NOTICE_DIRECTORY = "gtk-licenses"
DOCUMENTS = {
    "libadwaita": "libadwaita-1-dev",
    "gtk4": "libgtk-4-dev",
    "glib": "libglib2.0-dev",
    "pango": "libpango1.0-dev",
    "harfbuzz": "libharfbuzz-dev",
    "gdk-pixbuf": "libgdk-pixbuf-2.0-dev",
    "glycin": "libglycin-2-dev",
    "cairo": "libcairo2-dev",
    "freetype": "libfreetype-dev",
    "libpng": "libpng-dev",
    "pixman": "libpixman-1-dev",
    "graphene": "libgraphene-1.0-dev",
    "fribidi": "libfribidi-dev",
    "appstream": "libappstream-dev",
    "libmount": "libmount-dev",
    "libblkid": "libblkid-dev",
    "sysprof": "libsysprof-capture-4-dev",
}


def development_path(path, prefix):
    """Allow extracted development packages to overlay the system installation."""
    path = Path(path)
    if path.is_relative_to("/usr"):
        candidate = prefix / path.relative_to("/usr")
        if candidate.exists() or candidate.is_symlink():
            return candidate
    return path


def prepare(args):
    inputs = {
        "scripts": [
            metadata(__file__),
            metadata(RELEASE_CONFIG),
            metadata(ROOT / "scripts/build_support.py"),
        ],
        "development_prefix": str(args.development_prefix),
        "prefix": str(args.prefix),
        "environment": {
            key: os.environ.get(key)
            for key in ("PATH", "PKG_CONFIG_PATH", "PKG_CONFIG_LIBDIR", "PKG_CONFIG_SYSROOT_DIR")
        },
    }
    record = args.build_dir / "inputs.json"
    outputs = [args.prefix / name for name in LINK_INPUTS]
    outputs += [args.prefix / "include/gtk", args.prefix / NOTICE_DIRECTORY]
    if reusable(record, inputs, outputs):
        print(f"Reusing GTK/libadwaita development inputs: {args.prefix}", flush=True)
        return

    tool = shutil.which("pkg-config")
    if tool is None:
        raise ValueError("GTK preparation requires pkg-config")
    prefix = args.development_prefix
    directories = [prefix / "lib/pkgconfig", prefix / "share/pkgconfig"]
    directories += sorted((prefix / "lib").glob("*/pkgconfig"))
    environment = dict(os.environ)
    environment["PKG_CONFIG_PATH"] = os.pathsep.join(
        [*(str(path) for path in directories), environment.get("PKG_CONFIG_PATH", "")]
    )

    def query(*arguments):
        return subprocess.check_output([tool, *arguments], env=environment, text=True).strip()

    release = tomllib.loads(RELEASE_CONFIG.read_text())["targets"][TARGET]
    query("--atleast-version=" + release["minimum_gtk"], "gtk4")
    query("--atleast-version=" + release["minimum_libadwaita"], "libadwaita-1")
    include_flags = shlex.split(query("--cflags-only-I", "gtk4", "libadwaita-1"))
    includes = dict.fromkeys(
        development_path(flag[2:], prefix) for flag in include_flags if flag.startswith("-I")
    )
    dependencies = {Path(tool), *(path for path in directories if path.exists())}
    # Observe the actual pkg-config graph, including private header requirements.
    pending, visited = list(LIBRARIES), set()
    while pending:
        module = pending.pop()
        if module in visited:
            continue
        visited.add(module)
        directory = Path(query("--variable=pcfiledir", module))
        dependencies.add(directory / f"{module}.pc")
        for option in ("--print-requires", "--print-requires-private"):
            pending.extend(line.split()[0] for line in query(option, module).splitlines())

    with tempfile.TemporaryDirectory(dir=args.build_dir) as temporary:
        stage = Path(temporary)
        headers = stage / "include/gtk"
        headers.mkdir(parents=True)
        for directory in includes:
            shutil.copytree(directory, headers, dirs_exist_ok=True)
            dependencies.update(directory.rglob("*"))
            dependencies.add(directory)
        for module, name in LIBRARIES.items():
            original = Path(query("--variable=libdir", module)) / f"lib{name}.so"
            source = development_path(original, prefix)
            if source.is_symlink() and not source.exists():
                source = original.parent / source.readlink()
            shutil.copy2(source, stage / f"lib{name}.so")
            dependencies.add(source)
        notices = stage / NOTICE_DIRECTORY
        notices.mkdir()
        for component, package in DOCUMENTS.items():
            candidates = [
                prefix / "share/doc" / package,
                prefix / "share/licenses" / component,
                Path("/usr/share/doc") / package,
            ]
            for directory in dict.fromkeys(candidates):
                if not directory.is_dir():
                    continue
                files = [
                    path
                    for path in directory.iterdir()
                    if path.is_file()
                    and path.name.upper().startswith(("LICENSE", "COPYING", "COPYRIGHT", "NOTICE"))
                ]
                if files:
                    destination = notices / component
                    destination.mkdir(exist_ok=True)
                    for path in files:
                        shutil.copy2(path, destination / path.name)
                        dependencies.add(path)
                    break
        args.prefix.mkdir(parents=True, exist_ok=True)
        for name in ("include/gtk", NOTICE_DIRECTORY):
            destination = args.prefix / name
            if destination.exists():
                shutil.rmtree(destination)
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.move(stage / name, destination)
        for name in LINK_INPUTS:
            shutil.copy2(stage / name, args.prefix / name)

    state = {"inputs": inputs, "dependencies": [metadata(path) for path in sorted(dependencies)]}
    staged_record = record.with_suffix(".new")
    staged_record.write_text(json.dumps(state, separators=(",", ":")) + "\n")
    staged_record.replace(record)
    print(f"Prepared GTK/libadwaita development inputs: {args.prefix}", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--development-prefix", type=Path, default=Path("/usr"))
    parser.add_argument("--prefix", type=Path, default=ROOT / "sysroot/lib" / TARGET)
    parser.add_argument("--build-dir", type=Path, required=True)
    args = parser.parse_args()
    args.development_prefix = args.development_prefix.resolve()
    args.prefix = args.prefix.resolve()
    args.build_dir = args.build_dir.resolve()
    args.build_dir.mkdir(parents=True, exist_ok=True)
    with build_lock(args.build_dir / "build.lock"):
        prepare(args)


if __name__ == "__main__":
    main()
