//! GCC/Clang discovery for ELF targets. C compilation does not require an unwinder.
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::ValidatedCBridgeToolchainInvocation;
use crate::native_driver::{NativeCompilerFamily, macro_number};
use object::{BinaryFormat, Object, ObjectKind};
use scoop_lir::{
    ClangCompilerIdentityV1, GccCompilerIdentityV1, LinuxCCompilerIdentityV1, LirTargetProfile,
};
use scoop_process::CommandExt;
use scoop_wire::sha256;

use crate::ToolchainError;
use crate::linux_target::{LinuxLibc, LinuxTarget};
use crate::paths::find_program;

pub(crate) fn bundled_clang_path() -> Result<PathBuf, ToolchainError> {
    crate::installed_executable()?
        .parent()
        .and_then(Path::parent)
        .map(|root| {
            root.join("libexec/llvm/bin")
                .join(crate::host_executable_name("clang"))
        })
        .ok_or_else(|| error("cannot locate the installed LLVM tools"))
}

pub fn resolve_linux_c_toolchain(
    target: LirTargetProfile,
    compiler: Option<&Path>,
    native_sysroot: Option<&Path>,
) -> Result<ValidatedCBridgeToolchainInvocation, ToolchainError> {
    let selected = LinuxTarget::select(target.id())?;
    let default_driver = selected.libc.default_c_driver();
    let bundled = (compiler.is_none() && selected.libc == LinuxLibc::Gnu)
        .then(bundled_clang_path)
        .transpose()?;
    let selected_driver = compiler
        .or_else(|| bundled.as_deref().filter(|path| path.is_file()))
        .unwrap_or_else(|| Path::new(default_driver));
    let search_path = std::env::var_os("PATH").ok_or_else(|| error("PATH is not set"))?;
    let driver = find_program(selected_driver.as_os_str(), &search_path)?;
    // Only a wrapper that actually reads REALGCC receives this environment input.
    let real_gcc = underlying_compiler(
        uses_real_gcc(&driver)?,
        &search_path,
        std::env::var_os("REALGCC").as_deref(),
    )?;
    let sysroot = native_sysroot
        .map(std::fs::canonicalize)
        .transpose()
        .map_err(error)?;
    let directory = tempfile::Builder::new()
        .prefix("scoop-linux-c-")
        .tempdir()
        .map_err(error)?;
    let mut paths = CompilerPaths {
        target,
        family: NativeCompilerFamily::Gcc,
        driver,
        real_gcc,
        search_path,
        sysroot,
        temporary: directory.path().to_owned(),
    };
    let predefined = run(paths
        .bare_command()
        .args(["-dM", "-E", "-x", "c", "-"])
        .stdin(std::process::Stdio::null()))?;
    paths.family = NativeCompilerFamily::from_gnu_macros(
        std::str::from_utf8(&predefined.stdout).map_err(error)?,
    )?;
    let source = directory.path().join("probe.c");
    let object = directory.path().join("probe.o");
    let libc_check = selected.libc.header_probe();
    std::fs::write(
        &source,
        format!("{PROBE_PREFIX}{libc_check}\nint scoop_linux_c_probe(void) {{ return 0; }}\n"),
    )
    .map_err(error)?;
    let macros = run(paths.command().args(["-dM", "-E"]).arg(&source))?;
    let macros_text = std::str::from_utf8(&macros.stdout).map_err(error)?;
    let configuration = sha256(&macros.stdout);
    let compiler = match paths.family {
        NativeCompilerFamily::Gcc => {
            let version = format!(
                "{}.{}.{}",
                macro_number(macros_text, "__GNUC__")?,
                macro_number(macros_text, "__GNUC_MINOR__")?,
                macro_number(macros_text, "__GNUC_PATCHLEVEL__")?
            );
            LinuxCCompilerIdentityV1::Gcc(
                GccCompilerIdentityV1::new(&version, configuration).map_err(error)?,
            )
        }
        NativeCompilerFamily::Clang => {
            LinuxCCompilerIdentityV1::Clang(ClangCompilerIdentityV1::new(
                [
                    macro_number(macros_text, "__clang_major__")?,
                    macro_number(macros_text, "__clang_minor__")?,
                    macro_number(macros_text, "__clang_patchlevel__")?,
                ],
                configuration,
            ))
        }
        NativeCompilerFamily::Msvc => return Err(error("Linux requires a GNU-style C driver")),
    };
    let profile = scoop_target_data::linux_c_bridge_profile(target, compiler).map_err(error)?;
    let invocation = ValidatedCBridgeToolchainInvocation::new_linux(
        target,
        profile,
        paths.driver,
        paths.sysroot,
        paths.real_gcc,
        paths.search_path,
    )
    .map_err(error)?;
    run(&mut invocation.object_compilation_command(&source, &object))?;
    let bytes = std::fs::read(object).map_err(error)?;
    let file = object::File::parse(bytes.as_slice()).map_err(error)?;
    if file.architecture() != selected.machine.object_architecture()
        || file.format() != BinaryFormat::Elf
        || file.kind() != ObjectKind::Relocatable
        || !file.is_little_endian()
        || !file.is_64()
        || file.symbol_by_name("scoop_linux_c_probe").is_none()
    {
        return Err(error("C driver did not emit the selected ELF64 C object"));
    }
    Ok(invocation)
}

const PROBE_PREFIX: &str = "#include <features.h>\n#include <stdint.h>\n#include <stddef.h>\n#include <stdlib.h>\n#include <limits.h>\n\
#if !defined(__GNUC__) && !defined(__clang__)\n#error this Linux C driver requires GCC or Clang\n#endif\n\
_Static_assert(CHAR_BIT == 8 && sizeof(void *) == 8 && sizeof(long) == 8, \"LP64 required\");\n";

struct CompilerPaths {
    target: LirTargetProfile,
    family: NativeCompilerFamily,
    driver: PathBuf,
    real_gcc: Option<PathBuf>,
    search_path: OsString,
    sysroot: Option<PathBuf>,
    temporary: PathBuf,
}

impl CompilerPaths {
    fn command(&self) -> Command {
        let mut command = self.bare_command();
        if self.family == NativeCompilerFamily::Clang {
            command.arg(format!(
                "--target={}",
                scoop_target_data::definition(self.target.id())
                    .expect("selected target")
                    .llvm_triple
            ));
        }
        command.args(
            LinuxTarget::select(self.target.id())
                .expect("selected Linux target")
                .machine
                .compiler_flags(),
        );
        command
    }

    fn bare_command(&self) -> Command {
        let mut command = Command::new(&self.driver);
        command
            .env_clear()
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .env("TZ", "UTC")
            .env("PATH", &self.search_path)
            .env("TMPDIR", &self.temporary);
        if let Some(compiler) = &self.real_gcc {
            command.env("REALGCC", compiler);
        }
        if let Some(sysroot) = &self.sysroot {
            command.arg("--sysroot").arg(sysroot);
        }
        command
    }
}

fn underlying_compiler(
    wrapper: bool,
    search_path: &OsStr,
    real_gcc: Option<&OsStr>,
) -> Result<Option<PathBuf>, ToolchainError> {
    if wrapper {
        find_program(real_gcc.unwrap_or_else(|| OsStr::new("gcc")), search_path).map(Some)
    } else {
        Ok(None)
    }
}

fn uses_real_gcc(driver: &Path) -> Result<bool, ToolchainError> {
    use std::io::{BufRead, Read};
    let mut file = std::fs::File::open(driver).map_err(error)?;
    let mut prefix = [0; 2];
    if file.read(&mut prefix).map_err(error)? != 2 || &prefix != b"#!" {
        return Ok(false);
    }
    for line in std::io::BufReader::new(file).lines() {
        if line.map_err(error)?.contains("REALGCC") {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod discovery_tests;

fn run(command: &mut Command) -> Result<Output, ToolchainError> {
    let output = command.scoop_output().map_err(error)?;
    if !output.status.success() {
        return Err(error(format!(
            "C tool failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(output)
}

fn error(value: impl std::fmt::Display) -> ToolchainError {
    ToolchainError(format!("Linux C toolchain: {value}"))
}

#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
mod tests;
