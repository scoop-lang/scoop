//! Windows driver and SDK spellings for the implemented target machine.

use scoop_lir::{LirTargetProfile, NativeObjectFormat, ObjectMachineV1, TargetArchitecture};

use crate::ToolchainError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WindowsToolchainMachine {
    Amd64,
}

impl WindowsToolchainMachine {
    pub(crate) fn select(target: LirTargetProfile) -> Result<Self, ToolchainError> {
        let definition = scoop_target_data::definition(target.id())
            .filter(|definition| {
                definition.os == scoop_target_data::TargetOs::Windows
                    && definition.selection.target() == target
            })
            .ok_or_else(|| ToolchainError("a registered Windows target is required".into()))?;
        match (
            definition.architecture,
            target.native_object_format(),
            target.object_target().machine(),
        ) {
            (TargetArchitecture::X86_64, NativeObjectFormat::Coff64, ObjectMachineV1::X86_64) => {
                Ok(Self::Amd64)
            }
            (architecture, object_format, object_machine) => Err(ToolchainError(format!(
                "no Windows toolchain machine for {architecture:?}/{object_format:?}/{object_machine:?}"
            ))),
        }
    }

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Amd64 => "AMD64",
        }
    }

    pub(crate) const fn object_architecture(self) -> object::Architecture {
        match self {
            Self::Amd64 => object::Architecture::X86_64,
        }
    }

    pub(crate) const fn clang_arch_flag(self) -> &'static str {
        match self {
            Self::Amd64 => "-march=x86-64",
        }
    }

    pub(crate) const fn clang_cl_target_arg(self) -> &'static str {
        match self {
            Self::Amd64 => "--target=x86_64-pc-windows-msvc",
        }
    }

    pub(crate) const fn clang_cl_arch_arg(self) -> &'static str {
        match self {
            Self::Amd64 => "/clang:-march=x86-64",
        }
    }

    #[cfg(windows)]
    pub(crate) const fn msvc_discovery_target(self) -> &'static str {
        match self {
            Self::Amd64 => "x86_64-pc-windows-msvc",
        }
    }

    pub(crate) const fn llvm_mingw_target_directory(self) -> &'static str {
        match self {
            Self::Amd64 => "x86_64-w64-mingw32",
        }
    }

    pub(crate) const fn msvc_library_directory(self) -> &'static str {
        match self {
            Self::Amd64 => "x64",
        }
    }

    pub(crate) const fn msvc_linker_machine_arg(self) -> &'static str {
        match self {
            Self::Amd64 => "/machine:x64",
        }
    }

    pub(crate) const fn predefined_macro_condition(self) -> &'static str {
        match self {
            Self::Amd64 => "defined(_M_X64) || defined(__x86_64__)",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_windows_targets_select_the_implemented_machine() {
        for target in [
            scoop_target_data::WINDOWS_X86_64_MSVC_UCRT,
            scoop_target_data::WINDOWS_X86_64_MINGW_MSVCRT,
            scoop_target_data::WINDOWS_X86_64_MINGW_UCRT,
        ] {
            assert_eq!(
                WindowsToolchainMachine::select(target).unwrap(),
                WindowsToolchainMachine::Amd64
            );
        }
        assert!(WindowsToolchainMachine::select(scoop_target_data::LINUX_X86_64_GNU).is_err());
    }
}
