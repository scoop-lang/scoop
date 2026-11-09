use std::sync::{Arc, OnceLock};

use scoop_lir::{TargetProfileId, ValidatedLirTargetSelection};
use scoop_protocol::TargetSelectionRequestV1;
use scoop_toolchain::{ResolvedTargetProfile, ToolchainError};

/// Graph discovery needs a target contract; native production additionally
/// needs the selected tool invocation.
#[derive(Clone, Debug)]
pub enum GraphTarget {
    Artifact(Box<ResolvedTargetProfile>),
    Pending {
        selection: ValidatedLirTargetSelection,
        request: TargetSelectionRequestV1,
        resolved: Arc<OnceLock<Result<ResolvedTargetProfile, ToolchainError>>>,
    },
}

impl GraphTarget {
    pub fn requested(request: TargetSelectionRequestV1) -> Result<Self, ToolchainError> {
        let selection = scoop_toolchain::select_target(request.canonical_triple())?;
        Ok(Self::Pending {
            selection,
            request,
            resolved: Arc::new(OnceLock::new()),
        })
    }

    pub const fn lir_target_selection(&self) -> ValidatedLirTargetSelection {
        match self {
            Self::Artifact(target) => target.lir_target_selection(),
            Self::Pending { selection, .. } => *selection,
        }
    }

    pub const fn lir_target(&self) -> scoop_lir::LirTargetProfile {
        self.lir_target_selection().target()
    }

    pub const fn id(&self) -> TargetProfileId {
        self.lir_target_selection().target().id()
    }

    pub const fn canonical_triple(&self) -> &'static str {
        self.lir_target_selection()
            .target()
            .contract()
            .canonical_triple()
    }

    pub(crate) fn production(&self) -> Result<&ResolvedTargetProfile, ToolchainError> {
        match self {
            Self::Artifact(target) => Ok(target),
            Self::Pending {
                request, resolved, ..
            } => resolved
                .get_or_init(|| ResolvedTargetProfile::resolve_request(request))
                .as_ref()
                .map_err(Clone::clone),
        }
    }
}
