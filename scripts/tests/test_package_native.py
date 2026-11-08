"""Check host dependency discovery and the relocated installation layout."""

import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import package_native


@unittest.skipUnless(sys.platform == "linux", "ELF loader and patchelf")
class LinuxDependenciesTests(unittest.TestCase):
    def test_environment_libraries_are_relocated_with_selected_llvm_dependencies(self):
        self.check_library_search(explicit=False)

    def test_selected_python_libraries_are_relocated_without_an_environment_override(self):
        self.check_library_search(explicit=True)

    def check_library_search(self, *, explicit):
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary).resolve()
            root, llvm, external = (work / name for name in ("installation", "llvm", "python"))
            for directory in (root / "bin", llvm, external):
                directory.mkdir(parents=True)
            compiler = os.environ.get("CC", "cc")
            support = work / "support.c"
            # LLVM's selected library must win over another copy on the inherited path.
            for directory, value in ((llvm, 42), (external, 99)):
                support.write_text(f"int support_value(void) {{ return {value}; }}\n")
                subprocess.run(
                    [
                        compiler,
                        "-shared",
                        "-fPIC",
                        str(support),
                        "-Wl,-soname,librelease_support.so.1",
                        "-o",
                        str(directory / "librelease_support.so.1"),
                    ],
                    check=True,
                )
            (llvm / "librelease_support.so").symlink_to("librelease_support.so.1")
            library = external / "librelease_python.so.1"
            source = work / "python.c"
            source.write_text(
                "int support_value(void); int python_value(void) { return support_value(); }\n"
            )
            subprocess.run(
                [
                    compiler,
                    "-shared",
                    "-fPIC",
                    str(source),
                    "-L" + str(llvm),
                    "-lrelease_support",
                    "-Wl,-soname," + library.name,
                    "-o",
                    str(library),
                ],
                check=True,
            )
            (external / "librelease_python.so").symlink_to(library.name)
            library.chmod(0o555)
            source = work / "main.c"
            source.write_text(
                "#include <stdio.h>\nint python_value(void);\n"
                'int main(void) { printf("%d\\n", python_value()); return 0; }\n'
            )
            app = work / "program"
            subprocess.run(
                [
                    compiler,
                    str(source),
                    "-L" + str(external),
                    "-lrelease_python",
                    "-Wl,-rpath-link," + str(llvm),
                    "-o",
                    str(app),
                ],
                check=True,
            )
            shutil.copy2(app, root / "bin/program")
            interpreter = package_native.output("patchelf", "--print-interpreter", str(app))
            custom_loader = work / "build-loader"
            custom_loader.symlink_to(interpreter)
            helper = work / "python-helper"
            shutil.copy2(app, helper)
            subprocess.run(
                ["patchelf", "--set-interpreter", str(custom_loader), str(helper)], check=True
            )
            shutil.copy2(helper, root / "bin/helper")
            with patch.dict(os.environ, LD_LIBRARY_PATH="" if explicit else str(external)):
                libraries = package_native.linux(
                    root,
                    root,
                    [app],
                    llvm,
                    {helper: root / "bin/helper"},
                    search_directories=[external] if explicit else [],
                )
            self.assertEqual(set(libraries), {library, llvm / "librelease_support.so.1"})
            moved = work / "moved installation"
            root.rename(moved)
            result = subprocess.check_output(
                [moved / "bin/program"],
                text=True,
                env=dict(os.environ, LD_LIBRARY_PATH=""),
            )
            self.assertEqual(result, "42\n")
            self.assertEqual(
                package_native.output("patchelf", "--print-interpreter", str(moved / "bin/helper")),
                interpreter,
            )
            self.assertEqual(
                subprocess.check_output(
                    [moved / "bin/helper"], text=True, env=dict(os.environ, LD_LIBRARY_PATH="")
                ),
                "42\n",
            )
            self.assertTrue(custom_loader.is_file())
            self.assertTrue(library.is_file())
            self.assertTrue(app.is_file())

    def test_staged_prefix_libraries_are_reused_without_copying(self):
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary).resolve()
            root, llvm = work / "installation", work / "llvm"
            private = root / "libexec/llvm/lib"
            for directory in (root / "bin", llvm, private):
                directory.mkdir(parents=True)
            compiler = os.environ.get("CC", "cc")
            source = work / "support.c"
            source.write_text("int support_value(void) { return 42; }\n")
            subprocess.run(
                [
                    compiler,
                    "-shared",
                    "-fPIC",
                    str(source),
                    "-Wl,-soname,librelease_support.so.1",
                    "-o",
                    str(llvm / "librelease_support.so.1"),
                ],
                check=True,
            )
            shutil.copy2(llvm / "librelease_support.so.1", private / "librelease_support.so.1")
            (llvm / "librelease_support.so").symlink_to("librelease_support.so.1")
            source = work / "main.c"
            source.write_text(
                "#include <stdio.h>\nint support_value(void);\n"
                'int main(void) { printf("%d\\n", support_value()); return 0; }\n'
            )
            app = work / "program"
            subprocess.run(
                [
                    compiler,
                    str(source),
                    "-L" + str(llvm),
                    "-lrelease_support",
                    "-o",
                    str(app),
                ],
                check=True,
            )
            staged = private / "librelease_support.so.1"
            before = staged.stat()
            shutil.copy2(app, root / "bin/program")
            libraries = package_native.linux(root, root, [app], llvm)
            self.assertEqual(libraries, [llvm / "librelease_support.so.1"])
            self.assertEqual(staged.stat().st_ino, before.st_ino)
            result = subprocess.check_output(
                [root / "bin/program"], text=True, env=dict(os.environ, LD_LIBRARY_PATH="")
            )
            self.assertEqual(result, "42\n")


@unittest.skipUnless(sys.platform == "darwin", "Mach-O install_name_tool")
class DarwinDependenciesTests(unittest.TestCase):
    def test_component_roots_relocate_as_one_install_root(self):
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary).resolve()
            bin_root, llvm_root, build = (work / name for name in ("bin", "llvm", "build"))
            for directory in (bin_root / "bin", llvm_root / "libexec/llvm/lib", build):
                directory.mkdir(parents=True)
            compiler = os.environ.get("CC", "cc")
            source = build / "support.c"
            source.write_text("int support_value(void) { return 42; }\n")
            library = build / "librelease_support.dylib"
            subprocess.run(
                [
                    compiler,
                    "-shared",
                    "-fPIC",
                    str(source),
                    "-Wl,-install_name,@rpath/librelease_support.dylib",
                    "-o",
                    str(library),
                ],
                check=True,
            )
            source = build / "main.c"
            source.write_text(
                "#include <stdio.h>\nint support_value(void);\n"
                'int main(void) { printf("%d\\n", support_value()); return 0; }\n'
            )
            app = build / "program"
            subprocess.run(
                [
                    compiler,
                    str(source),
                    "-L" + str(build),
                    "-lrelease_support",
                    "-Wl,-rpath," + str(build),
                    "-o",
                    str(app),
                ],
                check=True,
            )
            staged = llvm_root / "libexec/llvm/lib" / library.name
            shutil.copy2(library, staged)
            shutil.copy2(app, bin_root / "bin" / app.name)
            before = staged.stat()
            libraries = package_native.darwin(bin_root, llvm_root, [app], build)
            self.assertEqual(libraries, [library.resolve()])
            self.assertEqual(staged.stat().st_ino, before.st_ino)
            # Overlay both component packages into one install root.
            installed = work / "installed"
            shutil.copytree(bin_root, installed, dirs_exist_ok=True)
            shutil.copytree(llvm_root, installed, dirs_exist_ok=True)
            result = subprocess.check_output([installed / "bin/program"], text=True)
            self.assertEqual(result, "42\n")


class WindowsDependenciesTests(unittest.TestCase):
    def test_recursive_dlls_are_beside_tools_external_commands_and_clang(self):
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary).resolve()
            root, llvm, build = (work / name for name in ("installation", "llvm", "build"))
            for directory in (
                root / "bin",
                root / "commands",
                root / "libexec/llvm/bin",
                llvm,
                build,
            ):
                directory.mkdir(parents=True)
            app = build / "scoop.exe"
            app.write_bytes(b"app")
            (root / "commands/scoop-example.exe").write_bytes(b"example")
            (root / "libexec/llvm/bin/clang.exe").write_bytes(b"clang")
            (llvm / "runtime.dll").write_bytes(b"selected runtime")
            (llvm / "support.dll").write_bytes(b"selected support")
            (build / "runtime.dll").write_bytes(b"different runtime")
            imports = {
                "scoop.exe": ["KERNEL32.dll", "PROPSYS.dll", "runtime.dll"],
                "scoop-example.exe": ["api-ms-win-crt-runtime-l1-1-0.dll", "runtime.dll"],
                "clang.exe": ["runtime.dll"],
                "runtime.dll": ["support.dll"],
                "support.dll": ["runtime.dll", "ucrtbase.dll"],
            }

            def readobj(tool, flag, path):
                self.assertEqual(Path(tool), llvm / "llvm-readobj.exe")
                self.assertEqual(flag, "--coff-imports")
                return "\n".join(
                    "Import {\n  Name: " + name + "\n}" for name in imports[Path(path).name]
                )

            with patch.object(package_native, "output", side_effect=readobj) as calls:
                libraries = package_native.windows(root, root, [app], llvm)
            self.assertEqual(set(libraries), {llvm / "runtime.dll", llvm / "support.dll"})
            self.assertEqual(calls.call_count, 9)
            for directory in (root / "bin", root / "commands", root / "libexec/llvm/bin"):
                self.assertEqual((directory / "runtime.dll").read_bytes(), b"selected runtime")
                self.assertEqual((directory / "support.dll").read_bytes(), b"selected support")
                self.assertFalse((directory / "KERNEL32.dll").exists())
                self.assertFalse((directory / "ucrtbase.dll").exists())

    def test_missing_non_system_dll_reports_the_importing_executable(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            app = root / "scoop.exe"
            app.touch()
            with (
                patch.object(
                    package_native,
                    "output",
                    return_value="Import {\n  Name: missing-scoop-test.dll\n}",
                ),
                patch.object(package_native.os, "get_exec_path", return_value=[]),
                self.assertRaisesRegex(ValueError, r"missing-scoop-test.dll of .*scoop.exe"),
            ):
                package_native.windows(root, root, [app], root / "llvm")

    def test_vc_runtime_is_a_distributable_dependency(self):
        self.assertTrue(package_native.windows_system("KERNEL32.DLL"))
        self.assertTrue(package_native.windows_system("api-ms-win-crt-stdio-l1-1-0.dll"))
        self.assertFalse(package_native.windows_system("vcruntime140.dll"))
        self.assertFalse(package_native.windows_system("msvcp140.dll"))
        self.assertFalse(package_native.windows_system("libLLVM-22.dll"))

    def test_selected_python_libraries_precede_unrelated_path_entries(self):
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary).resolve()
            root, python, external = (work / name for name in ("installation", "python", "other"))
            native = {}
            for relative in ("python.exe", "DLLs/_ssl.pyd"):
                source, destination = python / relative, root / "python" / relative
                source.parent.mkdir(parents=True, exist_ok=True)
                destination.parent.mkdir(parents=True, exist_ok=True)
                source.touch()
                native[source] = destination
            libraries = python / "Library/bin"
            for directory, contents in ((libraries, b"selected"), (external, b"unrelated")):
                directory.mkdir(parents=True)
                for name in ("libssl-3-x64.dll", "libcrypto-3-x64.dll", "vcruntime140.dll"):
                    (directory / name).write_bytes(contents)
            (python / "vcruntime140.dll").write_bytes(b"selected")
            imports = {
                "python.exe": ["vcruntime140.dll"],
                "_ssl.pyd": ["libssl-3-x64.dll", "vcruntime140.dll"],
                "libssl-3-x64.dll": ["libcrypto-3-x64.dll", "vcruntime140.dll"],
                "libcrypto-3-x64.dll": ["vcruntime140.dll"],
                "vcruntime140.dll": [],
            }

            def readobj(tool, flag, path):
                return "\n".join("  Name: " + name for name in imports[Path(path).name])

            with (
                patch.object(package_native, "output", side_effect=readobj),
                patch.object(package_native.os, "get_exec_path", return_value=[str(external)]),
            ):
                selected = package_native.windows(
                    root, root, [], work / "llvm", native, search_directories=[libraries]
                )
            self.assertTrue(all(path.is_relative_to(python) for path in selected))
            self.assertEqual((root / "python/vcruntime140.dll").read_bytes(), b"selected")
            for name in ("libssl-3-x64.dll", "libcrypto-3-x64.dll", "vcruntime140.dll"):
                self.assertEqual((root / "python/DLLs" / name).read_bytes(), b"selected")

    def test_matching_runtime_copies_in_python_directories_can_be_packaged(self):
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            root = work / "installation"
            native = {}
            for name in ("python", "extensions"):
                source, destination = work / name, root / name
                source.mkdir()
                destination.mkdir(parents=True)
                module = source / "module.pyd"
                module.touch()
                (source / "vcruntime140.dll").write_bytes(b"same runtime")
                native[module.resolve()] = destination / module.name

            def imports(tool, flag, path):
                return "  Name: vcruntime140.dll" if Path(path).suffix == ".pyd" else ""

            with (
                patch.object(package_native, "output", side_effect=imports),
                patch.object(package_native.os, "get_exec_path", return_value=[]),
            ):
                package_native.windows(root, root, [], work / "llvm", native)
                for name in ("python", "extensions"):
                    self.assertEqual(
                        (root / name / "vcruntime140.dll").read_bytes(), b"same runtime"
                    )
                (work / "extensions/vcruntime140.dll").write_bytes(b"different runtime")
                with self.assertRaisesRegex(ValueError, "different host libraries"):
                    package_native.windows(root, root, [], work / "llvm", native)


if __name__ == "__main__":
    unittest.main()
