"""Keep maintenance recipes driven by catalog OS, environment and architecture facts."""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import build_skia_windows
import build_windows_unwind
import package_licenses
from build_support import gn_cpu
from target_catalog import (
    HOSTS,
    WINDOWS_TARGETS,
    native_target_for_host,
)


class TargetRecipeTests(unittest.TestCase):
    def test_gn_cpu_uses_the_architecture_dimension(self):
        self.assertEqual(gn_cpu("aarch64"), "arm64")
        self.assertEqual(gn_cpu("x86_64"), "x64")

    def test_windows_skia_recipe_uses_environment_and_architecture(self):
        for target_name in WINDOWS_TARGETS:
            target, microsoft, target_cpu = build_skia_windows.target_recipe(target_name)
            self.assertEqual(microsoft, target["native_environment"] == "MicrosoftUcrt")
            self.assertEqual(target_cpu, gn_cpu(target["arch"]))

    def test_windows_skia_driver_flag_uses_the_target_architecture(self):
        target = {"arch": "aarch64"}
        self.assertEqual(
            build_skia_windows.compiler_arch_flag(target, "generic", True),
            "/clang:-mcpu=generic",
        )
        self.assertEqual(
            build_skia_windows.compiler_arch_flag(target, "generic", False),
            "-mcpu=generic",
        )

    def test_windows_unwind_recipe_uses_catalog_machine_facts(self):
        for target_name in WINDOWS_TARGETS:
            target, microsoft, processor = build_windows_unwind.target_recipe(target_name)
            self.assertEqual(microsoft, target["native_environment"] == "MicrosoftUcrt")
            self.assertEqual(processor, "AMD64")
            self.assertNotEqual(target["llvm_triple"], "")

    def test_llvm_mingw_license_path_uses_the_target_architecture(self):
        prefix = Path("toolchain")
        self.assertEqual(
            package_licenses.llvm_mingw_license_root(prefix, "x86_64"),
            prefix / "x86_64-w64-mingw32/share/mingw32",
        )

    def test_each_host_selects_native_target_facts_without_parsing_its_triple(self):
        for host in HOSTS:
            target_name, target = native_target_for_host(host["rust_target"])
            self.assertNotEqual(target_name, "")
            self.assertEqual(target["arch"], host["arch"])


if __name__ == "__main__":
    unittest.main()
