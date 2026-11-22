use scoop_lir::{DarwinPackedVersionV1, TargetArchitecture};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ApplePlatform {
    MacOs = 1,
    Ios = 2,
    IosSimulator = 7,
}

impl ApplePlatform {
    pub const fn name(self) -> &'static str {
        match self {
            Self::MacOs => "macOS",
            Self::Ios => "iOS",
            Self::IosSimulator => "iOS Simulator",
        }
    }

    pub const fn sdk(self) -> &'static str {
        match self {
            Self::MacOs => "macosx",
            Self::Ios => "iphoneos",
            Self::IosSimulator => "iphonesimulator",
        }
    }

    pub const fn deployment_variable(self) -> &'static str {
        match self {
            Self::MacOs => "MACOSX_DEPLOYMENT_TARGET",
            Self::Ios | Self::IosSimulator => "IPHONEOS_DEPLOYMENT_TARGET",
        }
    }

    pub const fn minimum_version_flag(self) -> &'static str {
        match self {
            Self::MacOs => "-mmacosx-version-min",
            Self::Ios => "-miphoneos-version-min",
            Self::IosSimulator => "-mios-simulator-version-min",
        }
    }

    pub const fn linker_name(self) -> &'static str {
        match self {
            Self::MacOs => "macos",
            Self::Ios => "ios",
            Self::IosSimulator => "ios-simulator",
        }
    }

    pub const fn macho_platform(self) -> u32 {
        self as u32
    }

    pub fn llvm_triple(
        self,
        architecture: TargetArchitecture,
        minimum: DarwinPackedVersionV1,
    ) -> String {
        let architecture = match architecture {
            TargetArchitecture::Aarch64 => "aarch64",
            TargetArchitecture::X86_64 => "x86_64",
        };
        match self {
            Self::MacOs => format!("{architecture}-apple-macosx{minimum}"),
            Self::Ios => format!("{architecture}-apple-ios{minimum}"),
            Self::IosSimulator => format!("{architecture}-apple-ios{minimum}-simulator"),
        }
    }
}

pub const fn apple_architecture(architecture: TargetArchitecture) -> &'static str {
    match architecture {
        TargetArchitecture::Aarch64 => "arm64",
        TargetArchitecture::X86_64 => "x86_64",
    }
}
