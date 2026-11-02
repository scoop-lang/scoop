use super::*;
use scoop_lir::{CBridgePlatformContractV1, CBridgeToolchainContractV1, DarwinPackedVersionV1};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ObjectDeployment {
    Darwin(DarwinPackedVersionV1),
    Linux,
    Windows,
}

impl ObjectDeployment {
    #[cfg(test)]
    pub(super) const DARWIN_TEST: Self = Self::Darwin(match DarwinPackedVersionV1::new(11 << 16) {
        Ok(value) => value,
        Err(_) => panic!("valid test deployment"),
    });

    #[cfg(test)]
    pub(super) const fn for_test(profile: scoop_target_data::CodegenDeploymentProfile) -> Self {
        match profile {
            scoop_target_data::CodegenDeploymentProfile::Darwin => Self::DARWIN_TEST,
            scoop_target_data::CodegenDeploymentProfile::Linux => Self::Linux,
            scoop_target_data::CodegenDeploymentProfile::Windows => Self::Windows,
        }
    }

    pub(super) fn from_contract(
        selection: ValidatedLirTargetSelection,
        profile: scoop_target_data::CodegenDeploymentProfile,
        native: &CBridgeToolchainContractV1,
    ) -> Result<Self, CodegenError> {
        if native.target().id() != selection.target().id() {
            return Err(CodegenError(
                "backend and native deployment targets differ".into(),
            ));
        }
        match (profile, native.platform()) {
            (
                scoop_target_data::CodegenDeploymentProfile::Darwin,
                CBridgePlatformContractV1::Darwin { deployment, .. },
            ) => Ok(Self::Darwin(deployment.minimum_os())),
            (
                scoop_target_data::CodegenDeploymentProfile::Linux,
                CBridgePlatformContractV1::Linux { .. },
            ) => Ok(Self::Linux),
            (
                scoop_target_data::CodegenDeploymentProfile::Windows,
                CBridgePlatformContractV1::Windows { .. },
            ) => Ok(Self::Windows),
            _ => Err(CodegenError(
                "codegen deployment and native toolchain platform differ".into(),
            )),
        }
    }

    pub(super) fn triple(self, canonical: &str) -> String {
        match self {
            Self::Darwin(version) => {
                let architecture = canonical.split('-').next().expect("registered LLVM triple");
                format!("{architecture}-apple-macosx{version}")
            }
            Self::Linux => canonical.into(),
            Self::Windows => canonical.into(),
        }
    }
}
