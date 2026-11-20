import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import install_toolchain


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
        self.enterContext(patch.object(Path, "home", return_value=self.home))
        self.build = self.enterContext(
            patch.object(install_toolchain, "build_package", side_effect=self.build_package)
        )
        self.run_installer = self.enterContext(patch.object(install_toolchain.subprocess, "run"))
        self.run_installer.return_value.returncode = 0

    def build_package(self, parser, args):
        args.host = "aarch64-apple-darwin"
        return self.manifest

    def test_updates_existing_default_in_default_directory(self):
        (self.default_dir / "settings.toml").write_text('default = "old"\n', encoding="utf-8")

        self.assertEqual(install_toolchain.main(["--jobs", "2"]), 0)

        args = self.build.call_args.args[1]
        self.assertEqual(args.jobs, 2)
        self.assertIn(".local.", args.version)
        self.run_installer.assert_called_once_with(
            [
                str(self.installer),
                "--home",
                self.default_dir,
                "--host",
                args.host,
                "update",
                args.version,
                "--manifest",
                self.manifest,
            ]
        )

    def test_installs_into_an_empty_custom_directory(self):
        custom_dir = self.home / "custom installation"
        self.run_installer.return_value.returncode = 1

        self.assertEqual(
            install_toolchain.main(["--install-dir", str(custom_dir), "--skip-docs"]), 1
        )

        args = self.build.call_args.args[1]
        self.run_installer.assert_called_once_with(
            [
                str(self.installer),
                "--home",
                custom_dir,
                "--host",
                args.host,
                "toolchain",
                "install",
                args.version,
                "--manifest",
                self.manifest,
                "--skip-docs",
            ]
        )

    def test_failed_build_does_not_call_installer(self):
        self.build.side_effect = subprocess.CalledProcessError(1, ["cargo", "build"])

        with self.assertRaises(subprocess.CalledProcessError):
            install_toolchain.main([])

        self.run_installer.assert_not_called()
