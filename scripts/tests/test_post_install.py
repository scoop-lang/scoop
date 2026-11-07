"""Run the distribution scripts against a command recorder in an extracted layout."""

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parents[1] / "distribution"
GNU = "x86_64-unknown-linux-gnu"
MUSL = "x86_64-unknown-linux-musl"
DARWIN = "aarch64-apple-darwin"
MSVC = "x86_64-pc-windows-msvc-ucrt"
MINGW = "x86_64-w64-windows-llvm-mingw-ucrt"

RECORDER = r"""#!/bin/sh
set -eu
printf '%s\0' "$0" "$@" >> "$SCOOP_TEST_COMMANDS"
printf '\0' >> "$SCOOP_TEST_COMMANDS"
[ "$1" != "${SCOOP_TEST_FAIL:-}" ] || exit 23
operation=$1
output=''
while [ "$#" -gt 0 ]; do
    case "$1" in --output-dir) output=$2; shift ;; esac
    shift
done
case "$operation" in
    doc)
        mkdir -p "$output"
        printf '<!doctype html><title>API documentation</title>' > "$output/index.html"
        ;;
esac
"""


@unittest.skipIf(os.name == "nt", "POSIX command recorder; Windows installs run in release CI")
class PostInstallTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="scoop's install ")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        self.commands = self.root / "commands.log"
        self.environment = dict(os.environ, SCOOP_TEST_COMMANDS=str(self.commands))

    def installation(self, targets=(GNU, MUSL), installed=None, gui_installed=()):
        (self.root / "VERSION").write_text("1.2.3\nrevision: abc123\nhost: test\n")
        plan = self.root / "share/scoop/install"
        plan.mkdir(parents=True)
        (plan / "targets.list").write_text("\n".join(targets) + "\n")
        for target in targets:
            (plan / f"{target}.list").write_text("scoop.core\nscoop.std\n")
            (plan / f"{target}.gui.list").write_text("scoop.window\nscoop.ui\n")
        for name in ("scoop.core", "scoop.std", "scoop.window", "scoop.ui"):
            manifest = self.root / "src" / name / "Cone.toml"
            manifest.parent.mkdir(parents=True)
            manifest.write_text(f'[cone]\ngroup = "scoop"\nname = "{name}"\n')
        for target in targets if installed is None else installed:
            library = self.root / "lib" / target
            library.mkdir(parents=True)
            for name in ("scoop.core", "scoop.std"):
                (library / f"{name}.slib").write_text("library")
        for target in gui_installed:
            for name in ("scoop.window", "scoop.ui"):
                (self.root / "lib" / target / f"{name}.slib").write_text("library")
        for name in ("post-install.sh", "post-install.ps1"):
            shutil.copy2(SCRIPTS / name, self.root / name)
        (self.root / "bin").mkdir()
        for name in ("scoop", "sco.exe"):
            recorder = self.root / "bin" / name
            recorder.write_text(RECORDER)
            recorder.chmod(0o755)
        for name in ("scoopc", "scoopc.exe"):
            (self.root / "bin" / name).touch()

    def run_install(self, *arguments, powershell=False):
        self.commands.write_bytes(b"")
        launcher = (
            ["pwsh", "-NoProfile", "-File", str(self.root / "post-install.ps1")]
            if powershell
            else ["sh", str(self.root / "post-install.sh")]
        )
        return subprocess.run(
            [*launcher, *map(str, arguments)],
            env=self.environment,
            cwd=self.root,
            capture_output=True,
            text=True,
        )

    def recorded(self):
        return [
            entry.decode().split("\0")
            for entry in self.commands.read_bytes().split(b"\0\0")
            if entry
        ]

    def test_default_documentation_can_be_repeated_and_extended(self):
        self.installation()
        for arguments, target in (([], GNU), ([], GNU), (["--target", MUSL], MUSL)):
            result = self.run_install(*arguments)
            self.assertEqual(result.returncode, 0, result.stderr)
            commands = self.recorded()
            self.assertEqual([command[1] for command in commands], ["doc"])
            command = commands[0]
            self.assertEqual(command[command.index("--target") + 1], target)
            self.assertIn("--no-deps", command)
            self.assertIn("--deny-doc-warnings", command)
            for name in ("scoop.core", "scoop.std"):
                self.assertIn(str(self.root / "src" / name / "Cone.toml"), command)
            self.assertTrue((self.root / "docs" / target / "index.html").is_file())
        index = (self.root / "docs/index.html").read_text()
        self.assertIn(f"{GNU}/index.html", index)
        self.assertIn(f"{MUSL}/index.html", index)

    def test_gui_cones_are_documented_only_with_the_gui_package(self):
        self.installation(gui_installed=(GNU,))
        gui = [str(self.root / "src" / name / "Cone.toml") for name in ("scoop.window", "scoop.ui")]
        for target, documented in ((GNU, True), (MUSL, False)):
            result = self.run_install("--target", target)
            self.assertEqual(result.returncode, 0, result.stderr)
            [command] = self.recorded()
            self.assertIn(str(self.root / "src/scoop.std/Cone.toml"), command)
            for manifest in gui:
                self.assertEqual(manifest in command, documented)

    def test_jobs_and_work_dir_preserve_arguments(self):
        self.installation()
        work = self.root / "another work directory"
        result = self.run_install("--all-targets", "--jobs", "2", "--work-dir", work)
        self.assertEqual(result.returncode, 0, result.stderr)
        commands = self.recorded()
        self.assertEqual(len(commands), 2)
        for command in commands:
            self.assertEqual(command[command.index("--jobs") + 1], "2")
            self.assertEqual(command[command.index("--cache-dir") + 1], str(work / "cache"))
        result = self.run_install("--target", GNU, "--target", GNU)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(self.recorded()), 1)

    def test_all_targets_skips_targets_whose_package_is_not_installed(self):
        self.installation(installed=(GNU,))
        result = self.run_install("--all-targets")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("not installed; skipping", result.stderr)
        commands = self.recorded()
        self.assertEqual(len(commands), 1)
        self.assertEqual(commands[0][commands[0].index("--target") + 1], GNU)

    def test_missing_target_package_names_the_archive(self):
        self.installation(installed=(GNU,))
        result = self.run_install("--target", MUSL)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(f"scoop-1.2.3-lib-base-{MUSL}.tar.xz", result.stderr)
        self.assertEqual(self.recorded(), [])

    def test_failed_documentation_stops_the_installation(self):
        self.installation()
        self.environment["SCOOP_TEST_FAIL"] = "doc"
        result = self.run_install()
        self.assertEqual(result.returncode, 23, result.stderr)
        self.assertEqual(len(self.recorded()), 1)
        self.assertFalse((self.root / "docs/index.html").exists())

    def test_invalid_selection_is_rejected_before_documenting(self):
        self.installation()
        for arguments in (
            ["--target", DARWIN],
            ["--target", "../invalid"],
            ["--target", GNU, "--all-targets"],
            ["--jobs", "0"],
            ["--jobs", "bad"],
            ["--target"],
            ["--cc", "clang"],
            ["--native-sysroot", "sdk"],
            ["--unknown"],
        ):
            with self.subTest(arguments=arguments):
                result = self.run_install(*arguments)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(self.recorded(), [])
        self.assertEqual(self.run_install("--help").returncode, 0)

    @unittest.skipUnless(shutil.which("pwsh"), "PowerShell is not installed")
    def test_powershell_documents_and_propagates_failures(self):
        self.installation((MSVC, MINGW))
        result = self.run_install("--all-targets", powershell=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        commands = self.recorded()
        self.assertEqual(len(commands), 2)
        for command in commands:
            self.assertEqual(command[0], str(self.root / "bin/sco.exe"))
            self.assertEqual(command[1], "doc")
            self.assertEqual(command[command.index("--sysroot") + 1], str(self.root))
        self.environment["SCOOP_TEST_FAIL"] = "doc"
        result = self.run_install(powershell=True)
        self.assertEqual(result.returncode, 23, result.stderr)
        self.assertEqual(len(self.recorded()), 1)

    @unittest.skipUnless(shutil.which("pwsh"), "PowerShell is not installed")
    def test_powershell_missing_target_package_names_the_archive(self):
        self.installation((MSVC, MINGW), installed=(MSVC,))
        result = self.run_install("--target", MINGW, powershell=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(f"scoop-1.2.3-lib-base-{MINGW}.tar.xz", result.stderr)
        self.assertEqual(self.recorded(), [])

    @unittest.skipUnless(shutil.which("pwsh"), "PowerShell is not installed")
    def test_powershell_documents_gui_cones_only_with_the_gui_package(self):
        self.installation((MSVC, MINGW), gui_installed=(MSVC,))
        gui = [str(self.root / "src" / name / "Cone.toml") for name in ("scoop.window", "scoop.ui")]
        for target, documented in ((MSVC, True), (MINGW, False)):
            result = self.run_install("--target", target, powershell=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            [command] = self.recorded()
            self.assertIn(str(self.root / "src/scoop.std/Cone.toml"), command)
            for manifest in gui:
                self.assertEqual(manifest in command, documented)


if __name__ == "__main__":
    unittest.main()
