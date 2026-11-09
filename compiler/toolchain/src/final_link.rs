//! Native final-link toolchains; artifact consumers do not load LLVM.
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::ValidatedCBridgeToolchainInvocation;
use scoop_lir::{LirTargetProfile, TargetProfileId};
use scoop_wire::{Digest256, Encoder, WireEncode, domain_separated_cbor_hash};

use crate::{CToolchainOptions, ResolvedTargetProfile, SystemProvider, ToolchainError};

fn selected_final_link(
    target: LirTargetProfile,
) -> Result<scoop_target_data::FinalLinkProfile, ToolchainError> {
    scoop_target_data::definition(target.id())
        .filter(|definition| definition.selection.target() == target)
        .map(|definition| definition.final_link)
        .ok_or_else(|| error("the selected target has no final-link profile"))
}

mod darwin;
mod elf;
mod linux;
mod output;
#[cfg(all(
    test,
    any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64"))
))]
mod shared_tests;
mod windows;
pub use darwin::DarwinFinalLinkProfile;
pub use linux::LinuxFinalLinkProfile;
pub use output::FinalOutputKind;
use output::OutputConfiguration;
pub use windows::WindowsFinalLinkProfile;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum LinkMode {
    Static,
    Dynamic,
}

#[derive(Clone, Debug, Default)]
pub struct FinalLinkOptions {
    /// Final native product selected by the root entry kind.
    pub output_kind: FinalOutputKind,
    /// Override the install name of a Darwin shared library.
    pub install_name: Option<String>,
    /// Override the SONAME of an ELF shared library.
    pub soname: Option<String>,
    /// Darwin LLD or Windows linker executable, independently of the native producer.
    pub linker: Option<PathBuf>,
    pub sysroot: Option<PathBuf>,
    pub unwind_prefix: Option<PathBuf>,
    pub mode: Option<LinkMode>,
    /// Retain the ordinary symbol table in the final native output.
    pub keep_symbols: bool,
    /// Extract existing object debug information into a platform debug file.
    pub emit_debug_file: bool,
    /// Capture program exception/trap stacks; source builds default by profile.
    pub enable_stack_trace: Option<bool>,
}

impl FinalLinkOptions {
    pub fn stack_trace_enabled(&self, optimization: scoop_lir::OptimizationMode) -> bool {
        self.enable_stack_trace
            .unwrap_or(optimization == scoop_lir::OptimizationMode::Debug)
    }

    fn native_roots(&self, target: LirTargetProfile) -> Result<Vec<PathBuf>, ToolchainError> {
        let sysroot = crate::configured_sysroot_root(self.sysroot.clone())?;
        let root =
            crate::target_library_root(&std::path::absolute(sysroot).map_err(error)?, target);
        Ok(vec![
            root.clone(),
            root.join("openssl/lib"),
            root.join("nghttp2/lib"),
        ])
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidatedFinalLinkProfile {
    Darwin(DarwinFinalLinkProfile),
    Linux(LinuxFinalLinkProfile),
    Windows(WindowsFinalLinkProfile),
}

impl ValidatedFinalLinkProfile {
    pub fn resolve(triple: &str) -> Result<Self, ToolchainError> {
        Self::resolve_with(
            triple,
            &CToolchainOptions::default(),
            &FinalLinkOptions::default(),
        )
    }

    pub fn resolve_with(
        triple: &str,
        c_toolchain: &CToolchainOptions,
        options: &FinalLinkOptions,
    ) -> Result<Self, ToolchainError> {
        let target = ResolvedTargetProfile::resolve_with(triple, c_toolchain)?;
        target.final_link_with(options)
    }

    pub(crate) fn from_tools(
        startup: ValidatedCBridgeToolchainInvocation,
        options: &FinalLinkOptions,
        cxx: Option<crate::ValidatedCxxToolchain>,
    ) -> Result<Self, ToolchainError> {
        let final_link = selected_final_link(startup.target())?;
        if options.linker.is_some() && final_link == scoop_target_data::FinalLinkProfile::Linux {
            return Err(error("--linker selects a Darwin or Windows final linker"));
        }
        let native_roots = options.native_roots(startup.target())?;
        match final_link {
            scoop_target_data::FinalLinkProfile::Darwin => {
                if options.mode == Some(LinkMode::Static) || options.unwind_prefix.is_some() {
                    return Err(error(
                        "Darwin uses dynamic libSystem unwinding; static mode and an unwind prefix are not applicable",
                    ));
                }
                DarwinFinalLinkProfile::from_startup(startup, options, cxx, native_roots)
                    .map(Self::Darwin)
            }
            scoop_target_data::FinalLinkProfile::Linux => {
                LinuxFinalLinkProfile::resolve(startup, options, cxx, native_roots).map(Self::Linux)
            }
            scoop_target_data::FinalLinkProfile::Windows => {
                WindowsFinalLinkProfile::resolve(startup, options, cxx).map(Self::Windows)
            }
        }
    }

    pub fn id(&self) -> TargetProfileId {
        self.startup_toolchain().profile().contract().target().id()
    }

    pub fn output_kind(&self) -> FinalOutputKind {
        match self {
            Self::Darwin(profile) => profile.data.output.kind(),
            Self::Linux(profile) => profile.data.output.kind(),
            Self::Windows(profile) => profile.data.output.kind(),
        }
    }

    pub fn default_library_roots(&self) -> &[PathBuf] {
        match self {
            Self::Darwin(profile) => &profile.data.native_roots,
            Self::Linux(profile) => &profile.data.native_roots,
            Self::Windows(profile) => &profile.data.native_roots,
        }
    }

    pub fn cxx(&self) -> bool {
        match self {
            Self::Darwin(profile) => profile.cxx(),
            Self::Linux(profile) => profile.cxx(),
            Self::Windows(profile) => profile.cxx(),
        }
    }

    pub fn target(&self) -> LirTargetProfile {
        self.startup_toolchain().target()
    }

    pub fn canonical_triple(&self) -> &'static str {
        self.target().contract().canonical_triple()
    }

    pub fn startup_toolchain(&self) -> &ValidatedCBridgeToolchainInvocation {
        match self {
            Self::Darwin(profile) => profile.startup_toolchain(),
            Self::Linux(profile) => profile.startup_toolchain(),
            Self::Windows(profile) => profile.startup_toolchain(),
        }
    }

    pub fn linker_driver(&self) -> &Path {
        match self {
            Self::Darwin(profile) => profile.linker_driver(),
            Self::Linux(profile) => profile.linker_driver(),
            Self::Windows(profile) => profile.linker_driver(),
        }
    }

    pub fn linker_args(&self) -> Vec<&'static str> {
        match self {
            Self::Darwin(profile) => profile.linker_args(),
            Self::Linux(profile) => profile.linker_args(),
            Self::Windows(profile) => profile.linker_args(),
        }
    }

    pub fn system_provider(&self) -> Result<&SystemProvider, ToolchainError> {
        match self {
            Self::Darwin(profile) => Ok(profile.system_provider()),
            Self::Linux(_) | Self::Windows(_) => Err(error(
                "this target uses native CRT/libc inputs, not Darwin system stubs",
            )),
        }
    }

    pub fn linker_system_requirements(&self) -> &'static [&'static str] {
        match self {
            Self::Darwin(profile) => profile.linker_system_requirements(),
            Self::Linux(_) | Self::Windows(_) => &[],
        }
    }

    pub fn command(&self, scratch: &Path, output: &Path) -> Result<Command, ToolchainError> {
        match self {
            Self::Darwin(profile) => profile.command(
                profile.startup_toolchain().sdk_root().map_err(error)?,
                scratch,
                output,
            ),
            Self::Linux(profile) => profile.command(scratch, output),
            Self::Windows(profile) => profile.command(scratch, output),
        }
    }

    pub fn fingerprint(&self) -> Result<Digest256, ToolchainError> {
        domain_separated_cbor_hash("scoop-final-link-profile-v3", self).map_err(error)
    }
}

impl WireEncode for ValidatedFinalLinkProfile {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), scoop_wire::cbor::EncodeError> {
        match self {
            Self::Darwin(profile) => profile.encode(encoder),
            Self::Linux(profile) => profile.encode(encoder),
            Self::Windows(profile) => profile.encode(encoder),
        }
    }
}

fn error(value: impl std::fmt::Display) -> ToolchainError {
    ToolchainError(format!("final-link toolchain: {value}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use scoop_lir::OptimizationMode::{Debug, Release};

    #[test]
    fn stack_trace_defaults_follow_profile_and_allow_explicit_overrides() {
        let mut options = FinalLinkOptions::default();
        assert!(options.stack_trace_enabled(Debug));
        assert!(!options.stack_trace_enabled(Release));
        for enabled in [false, true] {
            options.enable_stack_trace = Some(enabled);
            assert_eq!(options.stack_trace_enabled(Debug), enabled);
            assert_eq!(options.stack_trace_enabled(Release), enabled);
        }
    }

    #[test]
    fn current_targets_select_final_link_from_the_catalog() {
        for (targets, expected) in [
            (
                &[
                    scoop_target_data::DARWIN_AARCH64,
                    scoop_target_data::DARWIN_X86_64,
                ][..],
                scoop_target_data::FinalLinkProfile::Darwin,
            ),
            (
                &[
                    scoop_target_data::LINUX_X86_64_GNU,
                    scoop_target_data::LINUX_X86_64_MUSL,
                ][..],
                scoop_target_data::FinalLinkProfile::Linux,
            ),
            (
                &[
                    scoop_target_data::WINDOWS_X86_64_MSVC_UCRT,
                    scoop_target_data::WINDOWS_X86_64_MINGW_MSVCRT,
                    scoop_target_data::WINDOWS_X86_64_MINGW_UCRT,
                ][..],
                scoop_target_data::FinalLinkProfile::Windows,
            ),
        ] {
            for target in targets {
                assert_eq!(selected_final_link(*target).unwrap(), expected);
            }
        }
    }
}
