"""Exercise local LLVM inputs, extraction reuse and archive updates."""

import shutil
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import package_llvm
from build_llvm import VERSION
from package_llvm import prepare
from package_toolchain import archive


class LlvmSourceTests(unittest.TestCase):
    def test_local_archive_reuse_and_update(self):
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            original = work / f"llvm-project-{VERSION}.src"
            original.mkdir()
            license_file = original / "LICENSE.TXT"
            license_file.write_text("original license")
            archive = work / f"{original.name}.tar.xz"

            def write_archive():
                with tarfile.open(archive, "w:xz") as output:
                    output.add(original, arcname=original.name)

            write_archive()
            source, selected_archive = prepare(work / "build", None, archive)
            self.assertEqual(selected_archive, archive.resolve())
            self.assertEqual((source / "LICENSE.TXT").read_text(), "original license")
            original_stat = source.stat()
            reused, _ = prepare(work / "build", None, archive)
            self.assertEqual(reused.stat().st_mtime_ns, original_stat.st_mtime_ns)
            license_file.write_text("updated upstream license")
            write_archive()
            updated, _ = prepare(work / "build", None, archive)
            self.assertEqual((updated / "LICENSE.TXT").read_text(), "updated upstream license")
            with tarfile.open(archive, "w:xz") as output:
                output.add(original, arcname="unexpected-directory")
            with self.assertRaisesRegex(ValueError, "does not contain"):
                prepare(work / "build", None, archive)
            self.assertEqual((updated / "LICENSE.TXT").read_text(), "updated upstream license")

    def test_missing_explicit_input_is_reported(self):
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            with self.assertRaises(FileNotFoundError):
                prepare(work, work / "missing-source", None)
            with self.assertRaises(FileNotFoundError):
                prepare(work, None, work / "missing-archive")


class LlvmRuntimeTests(unittest.TestCase):
    def test_runtime_selection_keeps_tools_and_builtin_headers_without_sdk_files(self):
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            prefix, root = work / "sdk", work / "install"
            for name in (
                *package_llvm.TOOLS,
                *package_llvm.WINDOWS_TOOLS,
                "llvm-config",
                "llc",
                "llvm-tblgen",
            ):
                path = prefix / "bin" / (name + ".exe")
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(name)
            for name in (
                "include/llvm/Module.h",
                "lib/libLLVM.a",
                "lib/cmake/llvm/LLVMConfig.cmake",
                "lib/clang/22/include/stddef.h",
                "lib/clang/22/lib/linux/libclang_rt.a",
            ):
                path = prefix / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(name)
            package_llvm.stage(root, prefix, "Windows")
            installed = root / "libexec/llvm"
            self.assertEqual(
                {path.name for path in (installed / "bin").iterdir()},
                {name + ".exe" for name in (*package_llvm.TOOLS, *package_llvm.WINDOWS_TOOLS)},
            )
            self.assertEqual(
                [
                    path.relative_to(installed / "lib").as_posix()
                    for path in (installed / "lib").rglob("*")
                    if path.is_file()
                ],
                ["clang/22/include/stddef.h"],
            )
            self.assertFalse((installed / "include").exists())
            self.assertTrue((prefix / "include/llvm/Module.h").is_file())

    def test_hardlink_archive_has_one_payload_and_copy_install_still_works(self):
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            root = work / "staged"
            source = root / "libexec/llvm/bin/runtime.dll"
            source.parent.mkdir(parents=True)
            source.write_bytes(b"shared payload")
            alias = root / "bin/runtime.dll"
            package_llvm.share_file(source, alias, windows=True)
            path = work / "bin.tar.xz"
            archive(root, path)
            with tarfile.open(path) as package:
                files = [entry for entry in package if not entry.isdir()]
                self.assertEqual(sum(entry.size for entry in files), len(b"shared payload"))
                self.assertEqual(sum(entry.islnk() for entry in files), 1)
                package.extractall(work / "extracted", filter="data")
            shutil.copytree(work / "extracted", work / "copied 中文")
            self.assertEqual((work / "copied 中文/bin/runtime.dll").read_bytes(), b"shared payload")
            self.assertEqual(source.read_bytes(), b"shared payload")


if __name__ == "__main__":
    unittest.main()
