"""Build Rust host code when PATH also contains an unrelated link.exe."""

import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import package_windows


class WindowsSdkPackageTests(unittest.TestCase):
    def test_common_headers_and_each_crt_are_disjoint_data_packages(self):
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            args = SimpleNamespace(
                host="x86_64-pc-windows-gnullvm",
                mingw_msvcrt=work / "msvcrt",
                mingw_ucrt=work / "ucrt",
            )
            for prefix in (args.mingw_msvcrt, args.mingw_ucrt):
                files = {
                    "include/windows.h": "common Win32 header",
                    "include/c++/v1/vector": "common libc++ header",
                    "include/_mingw.h": prefix.name + " CRT",
                    "include/c++/v1/__config_site": prefix.name + " libc++ config",
                    "bin/clang.exe": "development driver",
                    "python/include/Python.h": "unneeded Python SDK",
                    "x86_64-w64-mingw32/lib/crt2.o": "startup object",
                    "x86_64-w64-mingw32/lib/libmsvcrt.a": prefix.name + " CRT library",
                    "x86_64-w64-mingw32/lib/libc++.a": "C++ runtime",
                    "x86_64-w64-mingw32/lib/libunwind.a": "unused unwind provider",
                    "x86_64-w64-mingw32/lib/libc++.modules.json": "unneeded modules",
                    "lib/clang/22/lib/windows/libclang_rt.builtins-x86_64.a": "builtins",
                    "lib/clang/22/lib/windows/libclang_rt.asan-x86_64.a": "unused sanitizer",
                    "lib/clang/22/lib/linux/libclang_rt.builtins-x86_64.a": "other OS",
                    "aarch64-w64-mingw32/lib/crt2.o": "other architecture",
                    "LICENSE.TXT": "LLVM license",
                    "x86_64-w64-mingw32/share/mingw32/COPYING": "MinGW license",
                }
                for name, content in files.items():
                    path = prefix / name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text(content)
            host = work / "host"
            package_windows.stage_shared_headers(host, args)
            include = host / "libexec/mingw/include"
            self.assertEqual(
                {
                    path.relative_to(include).as_posix()
                    for path in include.rglob("*")
                    if path.is_file()
                },
                {"windows.h", "c++/v1/vector"},
            )
            roots = []
            for crt in ("msvcrt", "ucrt"):
                target = "x86_64-w64-windows-llvm-mingw-" + crt
                root = work / target
                package_windows.stage_target_sdk(root, target, args)
                sdk = root / "lib" / target / "toolchain"
                paths = {
                    path.relative_to(sdk).as_posix() for path in sdk.rglob("*") if path.is_file()
                }
                self.assertEqual(
                    paths,
                    {
                        "include/_mingw.h",
                        "include/c++/v1/__config_site",
                        "x86_64-w64-mingw32/lib/crt2.o",
                        "x86_64-w64-mingw32/lib/libmsvcrt.a",
                        "x86_64-w64-mingw32/lib/libc++.a",
                        "lib/clang/22/lib/windows/libclang_rt.builtins-x86_64.a",
                    },
                )
                self.assertEqual((sdk / "include/_mingw.h").read_text(), crt + " CRT")
                self.assertTrue(
                    (root / f"share/licenses/native-{target}/LLVM-MinGW/COPYING").is_file()
                )
                roots.append({path.relative_to(root) for path in root.rglob("*") if path.is_file()})
            self.assertFalse(roots[0] & roots[1])
            self.assertTrue((args.mingw_ucrt / "bin/clang.exe").is_file())
            (args.mingw_ucrt / "include/windows.h").write_text("changed common header")
            with self.assertRaisesRegex(ValueError, "shared headers differ.*windows.h"):
                package_windows.stage_shared_headers(work / "mismatched", args)


@unittest.skipUnless(os.name == "nt", "MSVC Rust host toolchain")
class WindowsHostTests(unittest.TestCase):
    def test_cargo_build_scripts_use_the_selected_msvc_linker(self):
        if not os.environ.get("VCTOOLSINSTALLDIR"):
            self.skipTest("run from an x64 MSVC developer environment")
        with tempfile.TemporaryDirectory(prefix="rust host ") as temporary:
            work = Path(temporary)
            # Only the MSVC host linker is needed by this dependency-free crate.
            args = SimpleNamespace(
                host="x86_64-pc-windows-gnullvm", mingw_msvcrt=work, mingw_ucrt=work
            )
            wrong = work / "unrelated tools"
            wrong.mkdir()
            (wrong / "link.exe").write_bytes(b"not the Microsoft linker")
            environment = dict(os.environ, PATH=str(wrong) + os.pathsep + os.environ["PATH"])
            package_windows.host_environment(args, environment)
            (work / "Cargo.toml").write_text(
                '[package]\nname = "host-linker-probe"\nversion = "0.1.0"\nedition = "2024"\n'
                "[workspace]\n"
            )
            (work / "build.rs").write_text(
                'fn main() { println!("cargo::rustc-env=HOST_LINKER_PROBE=42"); }\n'
            )
            (work / "src").mkdir()
            (work / "src/main.rs").write_text(
                'fn main() { println!("{}", env!("HOST_LINKER_PROBE")); }\n'
            )
            result = subprocess.run(
                [
                    "cargo",
                    "run",
                    "--quiet",
                    "--offline",
                    "--target",
                    "x86_64-pc-windows-msvc",
                    "--target-dir",
                    str(work / "target"),
                ],
                cwd=work,
                env=environment,
                text=True,
                capture_output=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.strip(), "42")


if __name__ == "__main__":
    unittest.main()
