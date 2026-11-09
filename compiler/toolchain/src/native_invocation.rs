//! Actual C tool invocation exchanged by the parent and its compiler child.
use std::path::Path;

use scoop_protocol::HostPathCarrier;
use serde::{Deserialize, Serialize};

use crate::{
    CToolchainOptions, ResolvedTargetProfile, ToolchainError, ValidatedCBridgeToolchainInvocation,
};

mod invocation;
use invocation::InvocationRecord;

#[derive(Serialize, Deserialize)]
pub(crate) struct ResolvedInvocation {
    invocation: InvocationRecord,
    native_c: Option<HostPathCarrier>,
    native_cxx: Option<HostPathCarrier>,
}

impl ResolvedInvocation {
    pub(crate) fn from_target(target: &ResolvedTargetProfile) -> Result<Self, ToolchainError> {
        Ok(Self {
            invocation: InvocationRecord::from_invocation(target.c_bridge_toolchain())?,
            native_c: target.native_c_driver().map(path).transpose()?,
            native_cxx: target.native_cxx_driver().map(path).transpose()?,
        })
    }

    pub(crate) fn into_target(self, triple: &str) -> Result<ResolvedTargetProfile, ToolchainError> {
        let target = crate::registry::select_target(triple)?;
        Ok(ResolvedTargetProfile::from_c_toolchain(
            target,
            self.invocation.into_invocation(target.target())?,
        )?
        .with_native_drivers(
            self.native_c
                .map(|p| p.to_path_buf().map_err(error))
                .transpose()?,
            self.native_cxx
                .map(|p| p.to_path_buf().map_err(error))
                .transpose()?,
        ))
    }
}

pub(crate) fn resolve(
    triple: &str,
    options: &CToolchainOptions,
) -> Result<ResolvedTargetProfile, ToolchainError> {
    let selection = crate::registry::select_target(triple)?;
    let target = selection.target();
    let definition = scoop_target_data::definition(target.id()).expect("selected target");
    let invocation = match definition.os {
        scoop_target_data::TargetOs::Darwin => crate::c_bridge::probe_system_c_bridge_toolchain(
            target,
            options.compiler.as_deref(),
            options.native_sysroot.as_deref(),
        )?,
        scoop_target_data::TargetOs::Windows => crate::windows_c::resolve_default(
            target,
            options.native_sysroot.as_deref(),
            options.sysroot.as_deref(),
        )?,
        scoop_target_data::TargetOs::Linux => crate::linux_c::resolve_linux_c_toolchain(
            target,
            options.compiler.as_deref(),
            options.native_sysroot.as_deref(),
        )?,
    };
    let windows = definition.os == scoop_target_data::TargetOs::Windows;
    let cxx = options.cxx_compiler.clone().or_else(|| {
        windows
            .then(|| {
                options.compiler.as_deref().map(|driver| {
                    crate::windows_native::paired_driver(
                        &invocation,
                        scoop_manifest::NativeSourceLanguage::Cxx,
                        Some(driver),
                    )
                })
            })
            .flatten()
    });
    Ok(
        ResolvedTargetProfile::from_c_toolchain(selection, invocation)?
            .with_native_drivers(windows.then(|| options.compiler.clone()).flatten(), cxx),
    )
}

fn path(path: &Path) -> Result<HostPathCarrier, ToolchainError> {
    HostPathCarrier::from_path(path).map_err(error)
}

fn error(value: impl std::fmt::Display) -> ToolchainError {
    ToolchainError(format!("native tool invocation: {value}"))
}

#[cfg(test)]
mod tests;
