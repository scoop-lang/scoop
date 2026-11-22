#!/usr/bin/env python3
"""Build Scoop from this checkout and update the local default toolchain."""

import os
import shutil
import subprocess
import tomllib
from datetime import UTC, datetime
from pathlib import Path

from build_support import RELEASE_CONFIG, ROOT
from download_support import download_directory
from package_toolchain import argument_parser, build_package
from target_catalog import current_host


def default_toolchain(install_dir):
    try:
        with (install_dir / "settings.toml").open("rb") as source:
            return tomllib.load(source).get("default")
    except FileNotFoundError:
        return None


def install_package(installer, install_dir, args, manifest):
    with manifest.open("rb") as source:
        release = tomllib.load(source)
    command = [installer, "--home", install_dir, "--host", args.host]
    package = [args.version, "--manifest", manifest]
    install = [*command, "toolchain", "install", *package]
    docs_options = ["--skip-docs"] if args.skip_docs else []

    current = default_toolchain(install_dir)
    if current is None:
        target = release["hosts"][args.host]["default-target"]
        if "gui" in release["targets"][target]:
            install += ["--component", "gui"]
        return subprocess.run([*install, *docs_options]).returncode

    result = subprocess.run(
        [*command, "target", "list", "--installed", "--toolchain", current],
        stdout=subprocess.PIPE,
        text=True,
    )
    if result.returncode:
        return result.returncode
    targets = [line.removesuffix(" (installed)") for line in result.stdout.splitlines()]
    gui_targets = [target for target in targets if "gui" in release["targets"].get(target, {})]
    if gui_targets:
        # Stage the GUI targets before update carries over the old selection and activates it.
        all_targets_staged = gui_targets == targets
        result = subprocess.run(
            [
                *install,
                "--target",
                ",".join(gui_targets),
                "--component",
                "gui",
                *(docs_options if all_targets_staged else ["--skip-docs"]),
            ]
        )
        if result.returncode:
            return result.returncode
        if all_targets_staged:
            docs_options = ["--skip-docs"]
    return subprocess.run([*command, "update", *package, *docs_options]).returncode


def local_dependencies(args):
    default_downloads = ROOT.parent / "scoop-dep/downloads"
    if args.download_dir is None and not os.environ.get("SCOOP_DOWNLOAD_DIR"):
        args.download_dir = default_downloads if default_downloads.is_dir() else None
    dependencies = download_directory(args.download_dir).parent
    host = current_host()
    if host is None:
        return
    prepared = args.work_dir or dependencies / "build" / f"package-{host['rust_target']}"
    suffix = ".exe" if os.name == "nt" else ""
    if prepared.is_dir():
        args.work_dir = prepared
        llvm = prepared / "llvm/installed"
        if args.llvm_prefix is None and (llvm / "bin" / ("llvm-config" + suffix)).is_file():
            args.llvm_prefix = llvm
        lldb = prepared / "lldb/installed"
        if args.debugger_prefix is None and (lldb / "bin" / ("lldb" + suffix)).is_file():
            args.debugger_prefix = lldb
        args.vscode_extension = args.vscode_extension or next(prepared.glob("tooling/*.vsix"), None)
    if args.gn == "gn":
        gn = dependencies / "gn" / ("gn" + suffix)
        if gn.is_file():
            args.gn = str(gn)


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
    return install_package(installer, install_dir, args, manifest)


if __name__ == "__main__":
    raise SystemExit(main())
