#!/usr/bin/env python3
"""Stage the independent Linux StatusNotifierItem/DBusMenu development inputs."""

import argparse
import json
import os
import shlex
import shutil
import subprocess
import tempfile
from pathlib import Path

from build_support import ROOT, build_lock, metadata, reusable
from prepare_gtk import development_path

TARGET = "x86_64-unknown-linux-gnu"
MODULE = "dbusmenu-glib-0.4"
LINK_INPUTS = ("libdbusmenu-glib.so",)
NOTICE_DIRECTORY = "tray-licenses"


def prepare(args):
    inputs = {
        "scripts": [
            metadata(__file__),
            metadata(ROOT / "scripts/build_support.py"),
            metadata(ROOT / "scripts/prepare_gtk.py"),
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
    outputs += [args.prefix / "include/tray", args.prefix / NOTICE_DIRECTORY]
    if reusable(record, inputs, outputs):
        print(f"Reusing tray development inputs: {args.prefix}", flush=True)
        return

    tool = shutil.which("pkg-config")
    if tool is None:
        raise ValueError("Tray preparation requires pkg-config")
    prefix = args.development_prefix
    directories = [prefix / "lib/pkgconfig", prefix / "share/pkgconfig"]
    directories += sorted((prefix / "lib").glob("*/pkgconfig"))
    environment = dict(os.environ)
    environment["PKG_CONFIG_PATH"] = os.pathsep.join(
        [*(str(path) for path in directories), environment.get("PKG_CONFIG_PATH", "")]
    )

    def query(*arguments):
        return subprocess.check_output([tool, *arguments], env=environment, text=True).strip()

    flags = shlex.split(query("--cflags-only-I", MODULE))
    includes = dict.fromkeys(
        development_path(flag[2:], prefix) for flag in flags if flag.startswith("-I")
    )
    # GLib/GIO headers already belong to the GTK inputs used by scoop.window.
    includes = [path for path in includes if (path / "libdbusmenu-glib/menuitem.h").is_file()]
    if not includes:
        raise ValueError("pkg-config did not provide the libdbusmenu-glib headers")
    dependencies = {Path(tool), *(path for path in directories if path.exists())}
    dependencies.add(Path(query("--variable=pcfiledir", MODULE)) / f"{MODULE}.pc")
    with tempfile.TemporaryDirectory(dir=args.build_dir) as temporary:
        stage = Path(temporary)
        headers = stage / "include/tray"
        headers.mkdir(parents=True)
        for directory in includes:
            shutil.copytree(directory, headers, dirs_exist_ok=True)
            dependencies.update(directory.rglob("*"))
            dependencies.add(directory)
        for name in LINK_INPUTS:
            original = Path(query("--variable=libdir", MODULE)) / name
            source = development_path(original, prefix)
            if source.is_symlink() and not source.exists():
                source = original.parent / source.readlink()
            shutil.copy2(source, stage / name)
            dependencies.add(source)
        notices = stage / NOTICE_DIRECTORY / "libdbusmenu"
        notices.mkdir(parents=True)
        candidates = [
            prefix / "share/doc/libdbusmenu-glib-dev",
            prefix / "share/doc/libdbusmenu-glib4",
            prefix / "share/licenses/libdbusmenu",
            Path("/usr/share/doc/libdbusmenu-glib-dev"),
            Path("/usr/share/doc/libdbusmenu-glib4"),
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
                for path in files:
                    shutil.copy2(path, notices / path.name)
                    dependencies.add(path)
                break
        args.prefix.mkdir(parents=True, exist_ok=True)
        for name in ("include/tray", NOTICE_DIRECTORY):
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
    print(f"Prepared tray development inputs: {args.prefix}", flush=True)


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
