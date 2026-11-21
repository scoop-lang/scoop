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
