"""Cover licenses for each component package and shared Cargo dependencies."""

import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import package_licenses


class ComponentLicenseTests(unittest.TestCase):
    def test_bin_licenses_keep_shared_feature_dependencies_without_dev_crates(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            repository = root / "repository"
            repository.mkdir()

            def package(name, external=True):
                directory = root / "crates" / name
                directory.mkdir(parents=True, exist_ok=True)
                (directory / "LICENSE").write_text(f"{name} license")
                return {
                    "id": name,
                    "name": name,
                    "version": "1.0.0",
                    "source": "registry" if external else None,
                    "manifest_path": str(directory / "Cargo.toml"),
                    "license": "MPL-2.0" if name == "pubgrub" else "MIT",
                    "license_file": None,
                }

            def node(name, normal=(), dev=()):
                return {
                    "id": name,
                    "deps": [
                        {"pkg": target, "dep_kinds": [{"kind": kind}]}
                        for targets, kind in ((normal, None), (dev, "dev"))
                        for target in targets
                    ],
                }

            main = {
                "packages": [
                    package("scoop", False),
                    package("scoop-pm", False),
                    package("shared"),
                    package("main-only"),
                    package("pubgrub"),
                ],
                "resolve": {
                    "nodes": [
                        node("scoop", ["shared"]),
                        node("scoop-pm", ["pubgrub"]),
                        node("pubgrub"),
                        node("shared", ["main-only"]),
                        node("main-only"),
                    ]
                },
            }
            example = {
                "packages": [
                    package("scoop-example", False),
                    package("shared"),
                    package("example-only"),
                    package("test-only"),
                ],
                "resolve": {
                    "nodes": [
                        node("scoop-example", ["shared"], ["test-only"]),
                        node("shared", ["example-only"]),
                        node("example-only"),
                        node("test-only"),
                    ]
                },
            }
            bundle = {
                "packages": [package("scoop-bundle", False), package("bundle-only")],
                "resolve": {"nodes": [node("scoop-bundle", ["bundle-only"]), node("bundle-only")]},
            }
            installation = root / "installation"
            with patch.object(
                package_licenses.subprocess,
                "check_output",
                side_effect=[json.dumps(main), json.dumps(example), json.dumps(bundle)],
            ) as metadata:
                package_licenses.collect_bin(installation, repository, "aarch64-apple-darwin")
                self.assertEqual(
                    [call.kwargs["cwd"] for call in metadata.call_args_list],
                    [
                        repository,
                        repository / "tooling/scoop-example",
                        repository / "tooling/scoop-bundle",
                    ],
                )
            licenses = installation / "share/licenses"
            for name in ("shared", "main-only", "example-only", "bundle-only", "pubgrub"):
                self.assertEqual(
                    (licenses / f"{name}-1.0.0/LICENSE").read_text(), f"{name} license"
                )
            self.assertIn("| pubgrub-1.0.0 | MPL-2.0 |", (licenses / "README.md").read_text())
            self.assertFalse((licenses / "test-only-1.0.0").exists())
            self.assertEqual((licenses / "README.md").read_text().count("| shared-1.0.0 |"), 1)

    def test_vendor_and_base_licenses_cover_upstream_inputs(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            sysroot = root / "sysroot"
            uring = sysroot / "vendor/liburing/2.15"
            uring.mkdir(parents=True)
            (uring / "COPYING").write_text("liburing license")
            backend = sysroot / "vendor/text_backend"
            (backend / "licenses/encoding_rs-1.0.0").mkdir(parents=True)
            (backend / "licenses/encoding_rs-1.0.0/COPYRIGHT").write_text("encoding_rs license")
            (backend / "LICENSE").write_text("text backend license")
            (backend / "packages.json").write_text(
                json.dumps([{"name": "encoding_rs", "version": "1.0.0", "license": "MIT"}])
            )
            installation = root / "installation"
            package_licenses.collect_vendor(installation, sysroot)
            licenses = installation / "share/licenses/vendor"
            self.assertEqual((licenses / "liburing/COPYING").read_text(), "liburing license")
            self.assertEqual(
                (licenses / "text-encoding_rs-1.0.0/COPYRIGHT").read_text(),
                "encoding_rs license",
            )
            table = (licenses / "README.md").read_text()
            self.assertIn("| liburing |", table)
            self.assertIn("| text-encoding_rs-1.0.0 | MIT |", table)
            target = "x86_64-unknown-linux-gnu"
            notices = sysroot / "lib" / target / "rust-std"
            notices.mkdir(parents=True)
            (notices / "COPYRIGHT-library.txt").write_text("Rust license")
            package_licenses.collect_base_libraries(installation, sysroot, target)
            base = installation / f"share/licenses/lib-base-{target}"
            self.assertEqual((base / "liburing/COPYING").read_text(), "liburing license")
            self.assertEqual(
                (base / "text-encoding_rs-1.0.0/COPYRIGHT").read_text(), "encoding_rs license"
            )

    def test_llvm_licenses_include_python_and_collected_gcc_runtime(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "llvm-source"
            (source / "llvm").mkdir(parents=True)
            (source / "llvm/LICENSE.TXT").write_text("LLVM license")
            python = root / "python-LICENSE.txt"
            python.write_text("PSF license")
            installation = root / "installation"
            package_licenses.collect_llvm(
                installation,
                source,
                "x86_64-unknown-linux-gnu",
                [root / "libstdc++.so.6"],
                python,
            )
            licenses = installation / "share/licenses/llvm"
            self.assertEqual((licenses / "LLVM/LICENSE.TXT").read_text(), "LLVM license")
            self.assertEqual((licenses / "Python/python-LICENSE.txt").read_text(), "PSF license")
            table = (licenses / "README.md").read_text()
            self.assertIn("| LLVM | Apache-2.0 WITH LLVM-exception |", table)
            self.assertIn("| GCC-runtime | GPL with GCC Runtime Library Exception |", table)

    def test_library_licenses_cover_rust_standard_library(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            target = "aarch64-apple-darwin"
            notices = root / "sysroot/lib" / target / "rust-std"
            notices.mkdir(parents=True)
            (notices / "COPYRIGHT-library.txt").write_text("Rust license")
            (root / "sysroot/vendor").mkdir()
            installation = root / "installation"
            package_licenses.collect_base_libraries(installation, root / "sysroot", target)
            licenses = installation / f"share/licenses/lib-base-{target}"
            self.assertEqual(
                (licenses / f"Rust-std-{target}/COPYRIGHT-library.txt").read_text(),
                "Rust license",
            )
            self.assertIn(f"| Rust-std-{target} |", (licenses / "README.md").read_text())

    def test_linux_gui_licenses_follow_the_gui_package(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            target = "x86_64-unknown-linux-gnu"
            vendor = root / "sysroot/vendor/skia"
            vendor.mkdir(parents=True)
            (vendor / "LICENSE-skia").write_text("Skia license")
            (vendor / "LICENSE-expat").write_text("Expat license")
            icons = root / "sysroot/src/scoop.icons.lucide"
            icons.mkdir(parents=True)
            (icons / "LICENSE").write_text("Lucide ISC and MIT license")
            library = root / "sysroot/lib" / target
            for relative, content in (
                ("rust-std/COPYRIGHT-library.txt", "Rust license"),
                ("gtk-licenses/gtk4/COPYING", "GTK license"),
                ("tray-licenses/libdbusmenu/COPYING", "dbusmenu license"),
            ):
                (library / relative).parent.mkdir(parents=True)
                (library / relative).write_text(content)
            installation = root / "installation"
            package_licenses.collect_base_libraries(installation, root / "sysroot", target)
            package_licenses.collect_gui_libraries(installation, root / "sysroot", target)
            base = installation / f"share/licenses/lib-base-{target}"
            gui = installation / f"share/licenses/lib-gui-{target}"
            self.assertEqual(
                {path.name for path in base.iterdir()}, {"README.md", f"Rust-std-{target}"}
            )
            self.assertEqual(
                (gui / "GTK-gtk4/COPYING").read_text(),
                "GTK license",
            )
            self.assertEqual(
                (gui / "Tray-libdbusmenu/COPYING").read_text(),
                "dbusmenu license",
            )
            self.assertNotIn("Rust-std", (gui / "README.md").read_text())
            self.assertEqual((gui / "Skia/LICENSE-expat").read_text(), "Expat license")
            self.assertEqual((gui / "Lucide/LICENSE").read_text(), "Lucide ISC and MIT license")

    def test_svg_dependencies_ship_licenses_in_darwin_and_windows_gui_packages(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            vendor = root / "sysroot/vendor/skia"
            vendor.mkdir(parents=True)
            (vendor / "LICENSE-skia").write_text("Skia license")
            (vendor / "LICENSE-expat").write_text("Expat license")
            icons = root / "sysroot/src/scoop.icons.lucide"
            icons.mkdir(parents=True)
            (icons / "LICENSE").write_text("Lucide ISC and MIT license")
            for target in ("aarch64-apple-darwin", "x86_64-pc-windows-msvc-ucrt"):
                installation = root / target
                package_licenses.collect_gui_libraries(installation, root / "sysroot", target)
                licenses = installation / f"share/licenses/lib-gui-{target}"
                self.assertEqual((licenses / "Skia/LICENSE-expat").read_text(), "Expat license")
                self.assertEqual((licenses / "Skia/LICENSE-skia").read_text(), "Skia license")
                self.assertEqual(
                    (licenses / "Lucide/LICENSE").read_text(), "Lucide ISC and MIT license"
                )


if __name__ == "__main__":
    unittest.main()
