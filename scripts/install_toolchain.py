#!/usr/bin/env python3
"""Build Scoop from this checkout and update the local default toolchain."""

import os
import shutil
import subprocess
import tomllib
from datetime import UTC, datetime
from pathlib import Path

from build_support import RELEASE_CONFIG
from package_toolchain import argument_parser, build_package


def install_command(install_dir):
    try:
        with (install_dir / "settings.toml").open("rb") as source:
            settings = tomllib.load(source)
    except FileNotFoundError:
        settings = {}
    return ["update"] if "default" in settings else ["toolchain", "install"]


def main(argv=None):
    default_dir = Path.home() / ".scoop"
    parser = argument_parser(__doc__, include_version=False)
    parser.add_argument(
        "--install-dir", type=Path, default=default_dir, help="Scoop home (defaults to ~/.scoop)"
    )
    parser.add_argument("--skip-docs", action="store_true", help="Skip offline API documentation")
    args = parser.parse_args(argv)
    install_dir = args.install_dir.expanduser().resolve()
    name = "scoop-install.exe" if os.name == "nt" else "scoop-install"
    installer = (
        shutil.which(str(install_dir / "bin" / name))
        or shutil.which(str(default_dir / "bin" / name))
        or shutil.which(name)
    )
    if installer is None:
        parser.error("scoop-install is required in the selected home, ~/.scoop/bin or PATH")

    release = tomllib.loads(RELEASE_CONFIG.read_text(encoding="utf-8"))
    args.version = f"{release['version']}.local.{datetime.now(UTC):%Y%m%d%H%M%S%f}"
    manifest = build_package(parser, args)

    command = [
        installer,
        "--home",
        install_dir,
        "--host",
        args.host,
        *install_command(install_dir),
        args.version,
        "--manifest",
        manifest,
    ]
    if args.skip_docs:
        command.append("--skip-docs")
    return subprocess.run(command).returncode


if __name__ == "__main__":
    raise SystemExit(main())
