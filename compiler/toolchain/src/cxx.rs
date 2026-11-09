//! The C++ driver paired with an already selected C toolchain.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::ValidatedCBridgeToolchainInvocation;
use scoop_process::CommandExt;
use scoop_wire::{Digest256, sha256};

use crate::ToolchainError;
use crate::native_driver::{
    NativeCompilerFamily, NativeDriver, NativeDriverDiscovery as Discovery, NativeDriverKind,
};

mod discovery;

#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
mod tests;

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ValidatedCxxToolchain {
    native: NativeDriver,
}

impl ValidatedCxxToolchain {
    pub fn resolve(c: &ValidatedCBridgeToolchainInvocation) -> Result<Self, ToolchainError> {
        Self::resolve_with(c, None)
    }

    pub fn resolve_with(
        c: &ValidatedCBridgeToolchainInvocation,
        driver: Option<&Path>,
    ) -> Result<Self, ToolchainError> {
        Ok(Self {
            native: NativeDriver::from_discovery(Self::probe(c, driver)?)?,
        })
    }

    fn probe(
        c: &ValidatedCBridgeToolchainInvocation,
        selected: Option<&Path>,
    ) -> Result<Discovery, ToolchainError> {
        if c.native_environment() == scoop_target_data::NativeEnvironment::LinuxMusl {
            return Err(error("native.cxx = true is not supported for Linux musl"));
        }
        if crate::windows_native::is_windows_target(c) {
            let default = crate::windows_native::paired_driver(
                c,
                scoop_manifest::NativeSourceLanguage::Cxx,
                None,
            );
            return crate::windows_native::discover(
                c,
                scoop_manifest::NativeSourceLanguage::Cxx,
                selected.unwrap_or(&default),
            );
        }
        let driver = match selected {
            Some(path) => crate::paths::find_program(
                path.as_os_str(),
                &std::env::var_os("PATH").unwrap_or_default(),
            )?,
            None => discovery::paired_driver(c)?,
        };
        let compiler_macros = run(c
            .driver_command_for(&driver, NativeCompilerFamily::Gcc)
            .args(["-dM", "-E", "-x", "c++", "-"])
            .stdin(Stdio::null()))?;
        let family = NativeCompilerFamily::from_gnu_macros(
            std::str::from_utf8(&compiler_macros.stdout).map_err(error)?,
        )?;
        let macros = run(c
            .driver_command_for(&driver, family)
            .args(["-dM", "-E", "-x", "c++", "-include", "string", "-"])
            .stdin(Stdio::null()))?;
        Ok(Discovery {
            driver: scoop_protocol::HostPathCarrier::from_path(&driver).map_err(error)?,
            configuration: *sha256(&macros.stdout).as_array(),
            kind: NativeDriverKind::Gnu,
            family,
        })
    }

    pub fn driver(&self) -> &Path {
        self.native.path()
    }

    pub fn fingerprint(&self) -> Digest256 {
        self.native.fingerprint()
    }

    pub fn command(&self, c: &ValidatedCBridgeToolchainInvocation) -> Command {
        self.native
            .command(c, scoop_manifest::NativeSourceLanguage::Cxx)
    }

    pub(crate) fn kind(&self) -> NativeDriverKind {
        self.native.kind()
    }
}

fn run(command: &mut Command) -> Result<std::process::Output, ToolchainError> {
    let output = command.scoop_output().map_err(error)?;
    if !output.status.success() {
        return Err(error(String::from_utf8_lossy(&output.stderr).trim()));
    }
    Ok(output)
}

fn error(value: impl std::fmt::Display) -> ToolchainError {
    ToolchainError(format!("native C++ toolchain: {value}"))
}
