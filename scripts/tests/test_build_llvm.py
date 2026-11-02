"""Apply LLVM source fixes independently of an enclosing Git checkout."""

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import build_llvm
from target_catalog import llvm_target_components


class LlvmPatchTests(unittest.TestCase):
    def test_build_components_follow_current_catalog_architectures(self):
        self.assertEqual(llvm_target_components(), ("AArch64", "X86"))
        self.assertEqual(
            llvm_target_components(({"arch": "x86_64"}, {"arch": "x86_64"}, {"arch": "aarch64"})),
            ("X86", "AArch64"),
        )
        with self.assertRaisesRegex(ValueError, "no LLVM component"):
            llvm_target_components(({"arch": "unsupported"},))

    def test_patch_applies_below_another_repository_and_can_be_reused(self):
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary) / "repository"
            repository.mkdir()
            subprocess.run(["git", "init", "--quiet", str(repository)], check=True)
            source = repository / "vendor/llvm-project"
            (source / "llvm").mkdir(parents=True)
            file = source / "llvm/example.cpp"
            file.write_text("original\n")
            fix = Path(temporary) / "fix.patch"
            fix.write_text(
                "diff --git a/llvm/example.cpp b/llvm/example.cpp\n"
                "--- a/llvm/example.cpp\n"
                "+++ b/llvm/example.cpp\n"
                "@@ -1 +1 @@\n"
                "-original\n"
                "+patched\n"
            )
            with patch.object(build_llvm, "SOURCE_PATCHES", (fix,)):
                self.assertTrue(build_llvm.patch_sources(source))
                self.assertEqual(file.read_text(), "patched\n")
                self.assertFalse(build_llvm.patch_sources(source))
                self.assertEqual(file.read_text(), "patched\n")


if __name__ == "__main__":
    unittest.main()
