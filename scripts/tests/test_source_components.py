"""Keep Scoop navigation sources separate from upstream build inputs."""

import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import package_toolchain


class SourceComponentTests(unittest.TestCase):
    def test_glue_and_headers_remain_with_cones_and_patches_follow_upstream(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            repository = root / "repository"
            sysroot = repository / "sysroot"
            own = (
                "src/scoop.demo/Cone.toml",
                "src/scoop.demo/src/demo.scoop",
                "src/scoop.demo/native/glue.c",
                "native/text_backend/src/lib.rs",
                "include/scoop_ffi.h",
                "share/scoop/examples/catalog.toml",
            )
            llvm = f"vendor/llvm-project/{package_toolchain.VERSION}"
            upstream = (
                "vendor/demo/source.tar.xz",
                "vendor/demo/LICENSE",
                f"{llvm}/README.md",
                f"{llvm}/patches/backend.patch",
            )
            for name in (*own, *upstream):
                path = sysroot / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(name)
            patch_file = repository / "scripts/patches/lldb-scoop-source-names.patch"
            patch_file.parent.mkdir(parents=True)
            patch_file.write_text("LLDB patch")
            llvm_source = root / "llvm-source"
            (llvm_source / "llvm").mkdir(parents=True)
            (llvm_source / "llvm/LICENSE.TXT").write_text("LLVM license")
            downloads = root / "downloads"
            downloads.mkdir()
            archive = downloads / "selected.tar.xz"
            archive.write_bytes(b"selected original archive")
            (downloads / "older-version.tar.xz").write_bytes(b"unselected")
            args = SimpleNamespace(
                prepared_sysroot=sysroot,
                revision="abc",
                llvm_source=llvm_source,
                llvm_archive=archive,
            )
            sources = root / "sources"
            vendor = root / "vendor"
            with patch.object(package_toolchain, "ROOT", repository):
                package_toolchain.assemble_sources(sources, args)
                package_toolchain.assemble_vendor(vendor, args)
            for name in own:
                self.assertEqual((sources / name).read_text(), name)
            self.assertFalse((sources / "vendor").exists())
            self.assertFalse((vendor / "src").exists())
            for name in upstream:
                self.assertEqual((vendor / name).read_text(), name)
            self.assertEqual(
                (vendor / llvm / "patches/lldb-scoop-source-names.patch").read_text(), "LLDB patch"
            )
            self.assertEqual(
                (
                    vendor / llvm / f"llvm-project-{package_toolchain.VERSION}.src.tar.xz"
                ).read_bytes(),
                archive.read_bytes(),
            )
            self.assertFalse(any(path.name == "older-version.tar.xz" for path in vendor.rglob("*")))


if __name__ == "__main__":
    unittest.main()
