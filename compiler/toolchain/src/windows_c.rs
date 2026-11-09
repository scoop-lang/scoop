//! Generated-C discovery for the three Windows native environments.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use object::{Object, ObjectKind};
use scoop_lir::{ClangCompilerIdentityV1, LirTargetProfile, TargetProfileId};
use scoop_process::CommandExt;
use scoop_wire::sha256;

use crate::native_driver::macro_number;
use crate::{ToolchainError, ValidatedCBridgeToolchainInvocation};

mod environment;
pub(crate) use environment::{BundledMinGw, WindowsCEnvironment};

pub(crate) fn bundled_llvm_root() -> Result<PathBuf, ToolchainError> {
    crate::installed_executable()?
        .parent()
        .and_then(Path::parent)
        .map(|root| root.join("libexec/llvm"))
        .ok_or_else(|| error("cannot locate the installed LLVM tools"))
}

pub(crate) fn target_triple(target: TargetProfileId) -> Result<&'static str, ToolchainError> {
    scoop_target_data::definition(target)
        .filter(|definition| definition.os == scoop_target_data::TargetOs::Windows)
        .map(|definition| definition.llvm_triple)
        .ok_or_else(|| error("a registered Windows target is required"))
}

pub(crate) fn native_environment(
    target: TargetProfileId,
) -> Result<scoop_target_data::NativeEnvironment, ToolchainError> {
    scoop_target_data::definition(target)
        .filter(|definition| definition.os == scoop_target_data::TargetOs::Windows)
        .map(|definition| definition.native_environment)
        .ok_or_else(|| error("a registered Windows target is required"))
}

pub fn resolve_windows_c_toolchain(
    target: LirTargetProfile,
    compiler: &Path,
    native_sysroot: Option<&Path>,
) -> Result<ValidatedCBridgeToolchainInvocation, ToolchainError> {
    probe_with_resources(target, compiler, native_sysroot, None)
}

pub(crate) fn probe_default(
    target: LirTargetProfile,
    native_sysroot: Option<&Path>,
    sysroot: Option<&Path>,
) -> Result<ValidatedCBridgeToolchainInvocation, ToolchainError> {
    let llvm = bundled_llvm_root()?;
    let installed = llvm.join("bin").join(crate::host_executable_name("clang"));
    let search = std::env::var_os("PATH").unwrap_or_default();
    let bundled = installed.is_file();
    let compiler = if bundled {
        installed.clone()
    } else {
        crate::paths::find_program(&crate::host_executable_name("clang"), &search)?
    };
    let default_root = crate::target_library_root(
        &crate::configured_sysroot_root(sysroot.map(Path::to_owned))?,
        target,
    )
    .join("toolchain");
    let microsoft =
        native_environment(target.id())? == scoop_target_data::NativeEnvironment::MicrosoftUcrt;
    let usable_default = if microsoft {
        default_root.join("msvc.toml").is_file()
    } else {
        default_root.is_dir()
    };
    let split_mingw = bundled && !microsoft && native_sysroot.is_none();
    if split_mingw && !usable_default {
        return Err(error(format!(
            "missing target SDK {}; install native-{}",
            default_root.display(),
            target.contract().canonical_triple()
        )));
    }
    probe_with_resources(
        target,
        &compiler,
        native_sysroot.or_else(|| usable_default.then_some(default_root.as_path())),
        split_mingw.then_some(llvm.as_path()),
    )
}

fn probe_with_resources(
    target: LirTargetProfile,
    compiler: &Path,
    native_sysroot: Option<&Path>,
    bundled_llvm: Option<&Path>,
) -> Result<ValidatedCBridgeToolchainInvocation, ToolchainError> {
    let triple = target_triple(target.id())?;
    let search_path = std::env::var_os("PATH").unwrap_or_default();
    let compiler = crate::paths::find_program(compiler.as_os_str(), &search_path)?;
    let environment =
        WindowsCEnvironment::resolve(target, &compiler, native_sysroot, bundled_llvm)?;
    let machine = environment.machine;
    let directory = tempfile::Builder::new()
        .prefix("scoop-windows-c-")
        .tempdir()
        .map_err(error)?;
    let source = directory.path().join("probe.c");
    let object = directory.path().join("probe.obj");
    let native_environment = native_environment(target.id())?;
    let microsoft = native_environment == scoop_target_data::NativeEnvironment::MicrosoftUcrt;
    let ucrt = native_environment != scoop_target_data::NativeEnvironment::LlvmMingwMsvcrt;
    std::fs::write(
        &source,
        format!(
            "{PROBE}\n#if !defined(__clang__) || !defined(_WIN64) || !({machine_condition})\n#error the Windows generated-C producer requires {machine_name} Clang\n#endif\n#if defined(__MINGW32__) == {microsoft}\n#error wrong Windows native environment\n#endif\n#if defined(_UCRT) != {ucrt}\n#error wrong Windows CRT headers\n#endif\nvoid *scoop_windows_c_probe(void) {{ return (void *)&_errno; }}\n",
            machine_condition = machine.predefined_macro_condition(),
            machine_name = machine.name(),
            microsoft = u8::from(microsoft),
            ucrt = u8::from(ucrt),
        ),
    )
    .map_err(error)?;
    let make_command = || environment.command(&compiler, triple, microsoft);
    let macros = run(make_command().args(["-dM", "-E"]).arg(&source))?;
    let macros_text = std::str::from_utf8(&macros.stdout).map_err(error)?;
    let version = [
        macro_number(macros_text, "__clang_major__")?,
        macro_number(macros_text, "__clang_minor__")?,
        macro_number(macros_text, "__clang_patchlevel__")?,
    ];
    let identity = ClangCompilerIdentityV1::new(version, sha256(&macros.stdout));
    let profile = scoop_target_data::windows_c_bridge_profile(target, identity).map_err(error)?;
    let invocation =
        ValidatedCBridgeToolchainInvocation::new_windows(target, profile, compiler, environment)
            .map_err(error)?;
    run(&mut invocation.object_compilation_command(&source, &object))?;
    let bytes = std::fs::read(object).map_err(error)?;
    let file = object::File::parse(bytes.as_slice()).map_err(error)?;
    if file.architecture() != machine.object_architecture()
        || file.format() != object::BinaryFormat::Coff
        || file.kind() != ObjectKind::Relocatable
        || file.symbol_by_name("scoop_windows_c_probe").is_none()
    {
        return Err(error(format!(
            "Clang did not emit the selected {} COFF C object",
            machine.name()
        )));
    }
    Ok(invocation)
}

const PROBE: &str = r#"
#include <stdint.h>
#include <stddef.h>
#include <stdlib.h>
#include <errno.h>
#include <stdatomic.h>
#include <windows.h>
_Static_assert(sizeof(void *) == 8 && sizeof(long) == 4 && sizeof(wchar_t) == 2,
    "Windows LLP64 required");
_Static_assert(ATOMIC_POINTER_LOCK_FREE == 2, "Windows runtime requires lock-free pointer atomics");
"#;

fn run(command: &mut Command) -> Result<Output, ToolchainError> {
    let output = command.scoop_output().map_err(error)?;
    if !output.status.success() {
        return Err(error(format!(
            "{}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(output)
}

fn error(value: impl std::fmt::Display) -> ToolchainError {
    ToolchainError(format!("Windows C toolchain: {value}"))
}
