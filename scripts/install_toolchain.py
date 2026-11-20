#!/usr/bin/env python3
"""Build Scoop from this checkout and update the local default toolchain."""

import os
import shutil
import subprocess
import tomllib
from datetime import UTC, datetime
from pathlib import Path

from build_llvm import VERSION as LLVM_VERSION
from build_support import RELEASE_CONFIG, ROOT
from download_support import download_directory
from package_toolchain import argument_parser, build_package
from target_catalog import current_host


def install_command(install_dir):
    try:
        with (install_dir / "settings.toml").open("rb") as source:
            settings = tomllib.load(source)
    except FileNotFoundError:
        settings = {}
    return ["update"] if "default" in settings else ["toolchain", "install"]


def local_dependencies(args):
    default_downloads = ROOT.parent / "scoop-dep/downloads"
    if args.download_dir is None and not os.environ.get("SCOOP_DOWNLOAD_DIR"):
        args.download_dir = default_downloads if default_downloads.is_dir() else None
    dependencies = download_directory(args.download_dir).parent
    host = current_host()
    if host is None:
        return
    prepared = dependencies / "build" / f"package-{host['rust_target']}"
    if not prepared.is_dir():
        return
    args.work_dir = args.work_dir or prepared
    args.llvm_source = args.llvm_source or prepared / f"inputs/llvm-project-{LLVM_VERSION}.src"
    args.llvm_prefix = args.llvm_prefix or prepared / "llvm/installed"
    args.debugger_prefix = args.debugger_prefix or prepared / "lldb/installed"
    args.vscode_extension = args.vscode_extension or next(prepared.glob("tooling/*.vsix"), None)
    if args.gn == "gn":
        args.gn = str(dependencies / "tools/gn")
    os.environ.setdefault("CARGO_HOME", str(dependencies / "cargo"))
    os.environ.setdefault("CARGO_NET_OFFLINE", "true")
    os.environ.setdefault("npm_config_cache", str(dependencies / "npm-cache"))
    os.environ.setdefault("npm_config_offline", "true")


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
    local_dependencies(args)
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
