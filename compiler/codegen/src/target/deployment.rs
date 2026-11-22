use super::*;
use scoop_lir::{CBridgePlatformContractV1, CBridgeToolchainContractV1, DarwinPackedVersionV1};
use scoop_target_data::{ApplePlatform, DeploymentProfile};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ObjectDeployment {
    Apple(ApplePlatform, DarwinPackedVersionV1),
    Linux,
    Windows,
}

impl ObjectDeployment {
    #[cfg(test)]
    pub(super) const DARWIN_TEST: Self =
        Self::for_test(DeploymentProfile::Apple(ApplePlatform::MacOs));

    #[cfg(test)]
    pub(super) const fn for_test(profile: DeploymentProfile) -> Self {
        match profile {
            DeploymentProfile::Apple(platform) => {
                let major = match platform {
                    ApplePlatform::MacOs => 11,
                    ApplePlatform::Ios | ApplePlatform::IosSimulator => 17,
                };
                let version = match DarwinPackedVersionV1::new(major << 16) {
                    Ok(value) => value,
                    Err(_) => panic!("valid test deployment"),
                };
                Self::Apple(platform, version)
            }
            DeploymentProfile::Linux => Self::Linux,
            DeploymentProfile::Windows => Self::Windows,
        }
    }

    pub(super) fn from_contract(
        selection: ValidatedLirTargetSelection,
        profile: DeploymentProfile,
        native: &CBridgeToolchainContractV1,
    ) -> Result<Self, CodegenError> {
        if native.target().id() != selection.target().id() {
            return Err(CodegenError::internal(
                "backend and native deployment targets differ".into(),
            )
            .with_code(scoop_source::DiagnosticCode::ICE2201));
        }
        match (profile, native.platform()) {
            (
                DeploymentProfile::Apple(platform),
                CBridgePlatformContractV1::Darwin { deployment, .. },
            ) => Ok(Self::Apple(platform, deployment.minimum_os())),
            (DeploymentProfile::Linux, CBridgePlatformContractV1::Linux { .. }) => Ok(Self::Linux),
            (DeploymentProfile::Windows, CBridgePlatformContractV1::Windows { .. }) => {
                Ok(Self::Windows)
            }
            _ => Err(CodegenError::internal(
                "codegen deployment and native toolchain platform differ".into(),
            )
            .with_code(scoop_source::DiagnosticCode::ICE2201)),
        }
    }

    pub(super) fn triple(
        self,
        llvm_triple: &str,
        architecture: scoop_lir::TargetArchitecture,
    ) -> String {
        match self {
            Self::Apple(platform, version) => platform.llvm_triple(architecture, version),
            Self::Linux | Self::Windows => llvm_triple.into(),
        }
    }
}
