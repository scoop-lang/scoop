use scoop_identity::{
    IdentityReferenceError, IdentityValidationError, SemanticIdentityImportError,
};
use scoop_wire::WirePath;

use crate::CompileCommitError;

use super::{
    DiagnosticCode, SlibDiagnostic, SlibDiagnosticRecord, SlibPrimaryOrigin, root_diagnostic,
};

impl SlibDiagnostic for IdentityValidationError {
    fn diagnostic(&self) -> SlibDiagnosticRecord {
        match self {
            Self::IdentityMismatch { kind, actual, .. } => {
                identity_diagnostic(DiagnosticCode::E2405, kind, actual)
            }
            Self::DuplicateIdentity { kind, id } => {
                identity_diagnostic(DiagnosticCode::E2405, kind, id)
            }
            Self::UnregisteredIdentity { kind, id } | Self::UnresolvedIdentity { kind, id } => {
                identity_diagnostic(DiagnosticCode::E2405, kind, id)
            }
            Self::DependencyCycle { kind, id } => {
                identity_diagnostic(DiagnosticCode::E2405, kind, id)
            }
            Self::AlreadyResolved { kind, id }
            | Self::IdentityCollision { kind, id }
            | Self::InvalidRecord { kind, id, .. } => {
                identity_diagnostic(DiagnosticCode::E2405, kind, id)
            }
            Self::Hash { .. } | Self::RegistrationClosed | Self::Poisoned => {
                SlibDiagnosticRecord::new(DiagnosticCode::E2405, WirePath::root())
            }
            Self::Resource(error) => error.diagnostic(),
        }
    }
}

impl SlibDiagnostic for IdentityReferenceError {
    fn diagnostic(&self) -> SlibDiagnosticRecord {
        match self {
            Self::Missing { kind, id } | Self::KeyUnavailable { kind, id } => {
                identity_diagnostic(DiagnosticCode::E2405, kind, id)
            }
            Self::FutureLayer { kind, id, .. } => {
                identity_diagnostic(DiagnosticCode::E2405, kind, id)
            }
            Self::StorageFailure => {
                SlibDiagnosticRecord::new(DiagnosticCode::E2401, WirePath::root())
            }
        }
    }
}

impl SlibDiagnostic for CompileCommitError {
    fn diagnostic(&self) -> SlibDiagnosticRecord {
        match self {
            Self::Resource(error) => error.diagnostic(),
            Self::SemanticImport(error) => error.diagnostic(),
        }
    }
}

impl SlibDiagnostic for SemanticIdentityImportError {
    fn diagnostic(&self) -> SlibDiagnosticRecord {
        match self {
            Self::OriginConflict { origin } | Self::DuplicateBatchOrigin { origin } => {
                root_diagnostic(DiagnosticCode::ICE0401, SlibPrimaryOrigin::Cone(*origin))
            }
            Self::IdentityConflict { kind, id } => {
                identity_diagnostic(DiagnosticCode::ICE0401, kind, id)
            }
            Self::MissingCanonicalKey => {
                SlibDiagnosticRecord::new(DiagnosticCode::E2405, WirePath::root())
            }
            Self::WorldIdExhausted => {
                SlibDiagnosticRecord::new(DiagnosticCode::E2401, WirePath::root())
            }
            Self::Allocation { .. } => {
                SlibDiagnosticRecord::new(DiagnosticCode::E2205, WirePath::root())
            }
        }
    }
}

fn identity_diagnostic(
    code: DiagnosticCode,
    kind: &'static str,
    id: &[u8; 32],
) -> SlibDiagnosticRecord {
    SlibDiagnosticRecord::new(code, WirePath::root().key(kind, *id))
        .with_origin(SlibPrimaryOrigin::Identity { kind, id: *id })
}
