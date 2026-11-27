//! The implemented Linux machine and libc facts, selected at the tool boundary.

use scoop_lir::{TargetArchitecture, TargetProfileId};
use scoop_target_data::NativeEnvironment;

use crate::ToolchainError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LinuxMachine {
    Aarch64,
    X86_64,
}

impl LinuxMachine {
    pub(crate) fn compiler_flags(self) -> &'static [&'static str] {
        match self {
            Self::X86_64 => &["-march=x86-64"],
            Self::Aarch64 => &["-march=armv8-a"],
        }
    }
    pub(crate) fn object_architecture(self) -> object::Architecture {
        match self {
            Self::X86_64 => object::Architecture::X86_64,
            Self::Aarch64 => object::Architecture::Aarch64,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LinuxLibc {
    Gnu,
    Musl,
}

impl LinuxLibc {
    pub(crate) fn default_c_driver(self) -> &'static str {
        match self {
            Self::Gnu => "gcc",
            Self::Musl => "musl-gcc",
        }
    }

    pub(crate) fn header_probe(self) -> &'static str {
        match self {
            Self::Gnu => "#ifndef __GLIBC__\n#error glibc target requires glibc headers\n#endif\n",
            Self::Musl => "#ifdef __GLIBC__\n#error musl target cannot use glibc headers\n#endif\n",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LinuxTarget {
    pub(crate) machine: LinuxMachine,
    pub(crate) libc: LinuxLibc,
}

impl LinuxTarget {
    pub(crate) fn select(target: TargetProfileId) -> Result<Self, ToolchainError> {
        let definition = scoop_target_data::definition(target).ok_or_else(|| {
            ToolchainError::new(
                scoop_source::DiagnosticCode::E2301,
                "unknown Linux target".into(),
            )
        })?;
        if definition.os != scoop_target_data::TargetOs::Linux {
            return Err(ToolchainError::new(
                scoop_source::DiagnosticCode::E2301,
                "Linux toolchain requires a Linux target".into(),
            ));
        }
        let libc = match definition.native_environment {
            NativeEnvironment::LinuxGnu => LinuxLibc::Gnu,
            NativeEnvironment::LinuxMusl => LinuxLibc::Musl,
            _ => {
                return Err(ToolchainError::new(
                    scoop_source::DiagnosticCode::E2301,
                    "Linux toolchain requires a Linux target".into(),
                ));
            }
        };
        let machine = match definition.selection.target().architecture() {
            TargetArchitecture::X86_64 => LinuxMachine::X86_64,
            TargetArchitecture::Aarch64 => LinuxMachine::Aarch64,
        };
        Ok(Self { machine, libc })
    }
}

#[cfg(all(test, target_os = "linux"))]
pub(crate) fn native_test_targets() -> [scoop_target_data::TargetDefinition; 2] {
    ["gnu", "musl"].map(|libc| {
        let triple = format!("{}-unknown-linux-{libc}", std::env::consts::ARCH);
        scoop_target_data::named_target(&triple).expect("native Linux test target")
    })
}
