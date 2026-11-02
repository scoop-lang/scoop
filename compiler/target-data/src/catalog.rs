use scoop_identity::TargetProfileId;
use scoop_lir::{LirTargetProfile, TargetArchitecture, ValidatedLirTargetSelection};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetOs {
    Darwin,
    Linux,
    Windows,
}

impl TargetOs {
    pub const fn source_name(self) -> &'static str {
        match self {
            Self::Darwin => "darwin",
            Self::Linux => "linux",
            Self::Windows => "windows",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeEnvironment {
    Darwin,
    LinuxGnu,
    LinuxMusl,
    MicrosoftUcrt,
    LlvmMingwMsvcrt,
    LlvmMingwUcrt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinalLinkProfile {
    Darwin,
    Linux,
    Windows,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodegenDeploymentProfile {
    Darwin,
    Linux,
    Windows,
}

impl FinalLinkProfile {
    pub const fn source_name(self) -> &'static str {
        match self {
            Self::Darwin => "darwin",
            Self::Linux => "linux",
            Self::Windows => "windows",
        }
    }

    pub const fn is_windows(self) -> bool {
        matches!(self, Self::Windows)
    }
}

impl NativeEnvironment {
    pub const fn is_windows(self) -> bool {
        matches!(
            self,
            Self::MicrosoftUcrt | Self::LlvmMingwMsvcrt | Self::LlvmMingwUcrt
        )
    }

    pub const fn is_linux(self) -> bool {
        matches!(self, Self::LinuxGnu | Self::LinuxMusl)
    }

    /// Width of the target C ABI's `long` type in bytes.
    pub const fn c_long_size_bytes(self) -> u8 {
        match self {
            Self::Darwin | Self::LinuxGnu | Self::LinuxMusl => 8,
            Self::MicrosoftUcrt | Self::LlvmMingwMsvcrt | Self::LlvmMingwUcrt => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodegenTuning {
    DarwinAarch64,
    X86_64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetDefinition {
    pub selection: ValidatedLirTargetSelection,
    pub aliases: &'static [&'static str],
    pub versioned_aliases: &'static [&'static str],
    pub os: TargetOs,
    pub architecture: TargetArchitecture,
    pub env: &'static str,
    pub llvm_triple: &'static str,
    pub codegen_deployment: CodegenDeploymentProfile,
    pub native_environment: NativeEnvironment,
    pub final_link: FinalLinkProfile,
    pub c_bridge_profile: &'static str,
    pub native_rust_target: &'static str,
    pub tuning: CodegenTuning,
}

impl TargetDefinition {
    pub const fn canonical(self) -> &'static str {
        self.selection.target().contract().canonical_triple()
    }

    pub const fn source_os(self) -> &'static str {
        self.os.source_name()
    }

    pub const fn source_arch(self) -> &'static str {
        match self.architecture {
            TargetArchitecture::Aarch64 => "aarch64",
            TargetArchitecture::X86_64 => "x86_64",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostDefinition {
    pub os: &'static str,
    pub arch: &'static str,
    pub environment: &'static str,
    pub system: &'static str,
    pub machines: &'static [&'static str],
    pub rust_target: &'static str,
    pub default_target: &'static str,
    pub distribution_targets: &'static [&'static str],
}

const fn target_id(name: &str, version: u32) -> TargetProfileId {
    match TargetProfileId::new(name, version) {
        Ok(id) => id,
        Err(_) => panic!("invalid registered target identity"),
    }
}

include!(concat!(env!("OUT_DIR"), "/catalog.rs"));

pub fn named_target(name: &str) -> Option<TargetDefinition> {
    TARGETS.iter().copied().find(|target| {
        target.canonical() == name
            || target.aliases.contains(&name)
            || target.versioned_aliases.iter().any(|prefix| {
                name.strip_prefix(prefix).is_some_and(|suffix| {
                    suffix.is_empty()
                        || suffix.trim_start_matches('.').split('.').all(|part| {
                            !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit())
                        })
                })
            })
    })
}

pub fn host(os: &str, arch: &str, environment: &str) -> Option<&'static HostDefinition> {
    HOSTS
        .iter()
        .find(|host| host.os == os && host.arch == arch && host.environment == environment)
}

pub const fn selection(id: TargetProfileId) -> Option<ValidatedLirTargetSelection> {
    match definition(id) {
        Some(target) => Some(target.selection),
        None => None,
    }
}

pub const fn layout(id: TargetProfileId) -> Option<LirTargetProfile> {
    match definition(id) {
        Some(target) => Some(target.selection.target()),
        None => None,
    }
}

#[cfg(test)]
mod tests;
