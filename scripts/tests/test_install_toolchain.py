import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import call, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import install_toolchain
import package_manifest


class LocalDependenciesTests(unittest.TestCase):
    def setUp(self):
        work = Path(self.enterContext(tempfile.TemporaryDirectory())).resolve()
        self.dependencies = work / "scoop-dep"
        self.prepared = self.dependencies / "build/package-aarch64-apple-darwin"
        self.prepared.mkdir(parents=True)
        self.gn = self.dependencies / "gn" / ("gn.exe" if os.name == "nt" else "gn")
        self.enterContext(patch.object(install_toolchain, "ROOT", work / "scoop"))
        self.enterContext(
            patch.object(
                install_toolchain,
                "current_host",
                return_value={"rust_target": "aarch64-apple-darwin"},
            )
        )
        self.enterContext(patch.dict(os.environ, {}, clear=True))

    def arguments(self, *extra):
        return install_toolchain.argument_parser(include_version=False).parse_args(
            [
                "--download-dir",
                str(self.dependencies / "downloads"),
                "--work-dir",
                str(self.prepared),
                *extra,
            ]
        )

    def test_preserves_cargo_and_npm_configuration(self):
        for environment in (
            {},
            {
                "CARGO_HOME": str(self.dependencies / "custom-cargo"),
                "CARGO_NET_OFFLINE": "false",
                "npm_config_cache": str(self.dependencies / "custom-npm"),
                "npm_config_offline": "false",
            },
        ):
            with self.subTest(environment=environment), patch.dict(os.environ, environment):
                install_toolchain.local_dependencies(self.arguments())

                self.assertEqual(dict(os.environ), environment)

    def test_uses_existing_local_gn(self):
        self.gn.parent.mkdir()
        self.gn.touch()
        args = self.arguments()

        install_toolchain.local_dependencies(args)

        self.assertEqual(args.gn, str(self.gn))

    def test_keeps_path_lookup_when_local_gn_is_absent(self):
        args = self.arguments()

        install_toolchain.local_dependencies(args)

        self.assertEqual(args.gn, "gn")

    def test_preserves_explicit_gn(self):
        self.gn.parent.mkdir()
        self.gn.touch()
        args = self.arguments("--gn", "custom-gn")

        install_toolchain.local_dependencies(args)

        self.assertEqual(args.gn, "custom-gn")

    def test_missing_resources_use_normal_package_preparation(self):
        args = self.arguments()

        install_toolchain.local_dependencies(args)

        self.assertIsNone(args.llvm_source)
        self.assertIsNone(args.llvm_prefix)
        self.assertIsNone(args.debugger_prefix)
        self.assertIsNone(args.vscode_extension)

    def test_reuses_resources_in_the_selected_work_directory(self):
        work = self.dependencies / "custom-work"
        suffix = ".exe" if os.name == "nt" else ""
        for relative in (
            "llvm/installed/bin/llvm-config" + suffix,
            "lldb/installed/bin/lldb" + suffix,
            "tooling/scoop.vsix",
        ):
            path = work / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.touch()
        args = self.arguments("--work-dir", str(work))

        install_toolchain.local_dependencies(args)

        self.assertEqual(args.work_dir, work)
        self.assertEqual(args.llvm_prefix, work / "llvm/installed")
        self.assertEqual(args.debugger_prefix, work / "lldb/installed")
        self.assertEqual(args.vscode_extension, work / "tooling/scoop.vsix")

    def test_reuses_gn_before_the_first_build(self):
        self.gn.parent.mkdir()
        self.gn.touch()
        args = self.arguments("--work-dir", str(self.dependencies / "first-build"))

        install_toolchain.local_dependencies(args)

        self.assertEqual(args.gn, str(self.gn))
        self.assertIsNone(args.llvm_prefix)
        self.assertIsNone(args.debugger_prefix)


class InstallToolchainTests(unittest.TestCase):
    def setUp(self):
        work = Path(self.enterContext(tempfile.TemporaryDirectory())).resolve()
        self.home = work / "user home"
        self.default_dir = self.home / ".scoop"
        name = "scoop-install.exe" if os.name == "nt" else "scoop-install"
        self.installer = self.default_dir / "bin" / name
        self.installer.parent.mkdir(parents=True)
        self.installer.touch()
        self.installer.chmod(0o755)
        self.manifest = work / "dist" / "manifest.toml"
        self.host = "aarch64-apple-darwin"
        self.targets = {self.host: {"gui": f"lib-gui-{self.host}"}}
        self.enterContext(patch.object(Path, "home", return_value=self.home))
        self.enterContext(patch.object(install_toolchain, "local_dependencies"))
        self.build = self.enterContext(
            patch.object(install_toolchain, "build_package", side_effect=self.build_package)
        )
        self.run_installer = self.enterContext(patch.object(install_toolchain.subprocess, "run"))
        self.run_installer.return_value.returncode = 0

    def build_package(self, parser, args):
        args.host = self.host
        package_manifest.write(
            self.manifest,
            {"hosts": {self.host: {"default-target": self.host}}, "targets": self.targets},
        )
        return self.manifest

    def command(self, *arguments, home=None):
        return [
            str(self.installer),
            "--home",
            home or self.default_dir,
            "--host",
            self.host,
            *arguments,
        ]

    def test_updates_existing_default_in_default_directory(self):
        (self.default_dir / "settings.toml").write_text('default = "old"\n', encoding="utf-8")
        target = "x86_64-apple-darwin"
        self.targets[target] = {"gui": f"lib-gui-{target}"}
        self.run_installer.return_value.stdout = f"{target} (installed)\n"

        self.assertEqual(install_toolchain.main(["--jobs", "2"]), 0)

        args = self.build.call_args.args[1]
        self.assertEqual(args.jobs, 2)
        self.assertIn(".local.", args.version)
        self.assertEqual(
            self.run_installer.call_args_list,
            [
                call(
                    self.command("target", "list", "--installed", "--toolchain", "old"),
                    stdout=subprocess.PIPE,
                    text=True,
                ),
                call(
                    self.command(
                        "toolchain",
                        "install",
                        args.version,
                        "--manifest",
                        self.manifest,
                        "--target",
                        target,
                        "--component",
                        "gui",
                    )
                ),
                call(
                    self.command("update", args.version, "--manifest", self.manifest, "--skip-docs")
                ),
            ],
        )

    def test_installs_into_an_empty_custom_directory(self):
        custom_dir = self.home / "custom installation"
        self.run_installer.return_value.returncode = 1

        self.assertEqual(
            install_toolchain.main(["--install-dir", str(custom_dir), "--skip-docs"]), 1
        )

        args = self.build.call_args.args[1]
        self.run_installer.assert_called_once_with(
            self.command(
                "toolchain",
                "install",
                args.version,
                "--manifest",
                self.manifest,
                "--component",
                "gui",
                "--skip-docs",
                home=custom_dir,
            )
        )

    def test_stages_only_gui_targets_in_a_mixed_update(self):
        (self.default_dir / "settings.toml").write_text('default = "old"\n', encoding="utf-8")
        self.host = "x86_64-unknown-linux-gnu"
        musl = "x86_64-unknown-linux-musl"
        self.targets = {self.host: {"gui": f"lib-gui-{self.host}"}, musl: {}}
        self.run_installer.return_value.stdout = f"{self.host} (installed)\n{musl} (installed)\n"

        self.assertEqual(install_toolchain.main([]), 0)

        version = self.build.call_args.args[1].version
        self.assertEqual(
            self.run_installer.call_args_list,
            [
                call(
                    self.command("target", "list", "--installed", "--toolchain", "old"),
                    stdout=subprocess.PIPE,
                    text=True,
                ),
                call(
                    self.command(
                        "toolchain",
                        "install",
                        version,
                        "--manifest",
                        self.manifest,
                        "--target",
                        self.host,
                        "--component",
                        "gui",
                        "--skip-docs",
                    )
                ),
                call(self.command("update", version, "--manifest", self.manifest)),
            ],
        )

    def test_failed_gui_stage_does_not_activate_the_new_version(self):
        (self.default_dir / "settings.toml").write_text('default = "old"\n', encoding="utf-8")
        self.run_installer.side_effect = [
            subprocess.CompletedProcess([], 0, stdout=f"{self.host} (installed)\n"),
            subprocess.CompletedProcess([], 7),
        ]

        self.assertEqual(install_toolchain.main([]), 7)
        self.assertEqual(self.run_installer.call_count, 2)

    def test_omits_gui_when_the_default_target_has_no_gui_package(self):
        self.targets[self.host] = {}

        self.assertEqual(install_toolchain.main([]), 0)

        version = self.build.call_args.args[1].version
        self.run_installer.assert_called_once_with(
            self.command("toolchain", "install", version, "--manifest", self.manifest)
        )

    def test_failed_build_does_not_call_installer(self):
        self.build.side_effect = subprocess.CalledProcessError(1, ["cargo", "build"])

        with self.assertRaises(subprocess.CalledProcessError):
            install_toolchain.main([])

        self.run_installer.assert_not_called()
