use scoop_wire::WirePath;

use crate::{
    ConeRecordValidationError, DependencyRecordValidationError, SlibMemberRoleValidationError,
};

use super::{DiagnosticCode, SlibDiagnostic, SlibDiagnosticRecord, SlibPrimaryOrigin};

pub(super) fn cone_record_diagnostic(error: &ConeRecordValidationError) -> SlibDiagnosticRecord {
    let (code, field, origin) = match error {
        ConeRecordValidationError::Record(crate::ConeRecordError::WindowsSubsystemOnLibrary) => {
            (DiagnosticCode::E2405, 5, SlibPrimaryOrigin::Manifest)
        }
        ConeRecordValidationError::Coordinate(_) | ConeRecordValidationError::Record(_) => {
            (DiagnosticCode::E2405, 1, SlibPrimaryOrigin::Manifest)
        }
        ConeRecordValidationError::Hash(_) => {
            (DiagnosticCode::E2108, 1, SlibPrimaryOrigin::Manifest)
        }
        ConeRecordValidationError::Identity(mismatch) => (
            DiagnosticCode::E2405,
            2,
            SlibPrimaryOrigin::Identity {
                kind: "cone",
                id: *mismatch.expected().as_array(),
            },
        ),
        ConeRecordValidationError::UnknownKind { .. } => {
            (DiagnosticCode::E2401, 3, SlibPrimaryOrigin::Manifest)
        }
        ConeRecordValidationError::UnknownSourceForm { .. } => {
            (DiagnosticCode::E2401, 4, SlibPrimaryOrigin::Manifest)
        }
        ConeRecordValidationError::UnknownWindowsSubsystem { .. } => {
            (DiagnosticCode::E2401, 5, SlibPrimaryOrigin::Manifest)
        }
        ConeRecordValidationError::Resource(error) => {
            return error.diagnostic().with_origin(SlibPrimaryOrigin::Manifest);
        }
    };
    SlibDiagnosticRecord::new(code, WirePath::root().field(6).field(field)).with_origin(origin)
}

pub(super) fn dependency_record_diagnostic(
    index: usize,
    error: &DependencyRecordValidationError,
) -> SlibDiagnosticRecord {
    let path = WirePath::root()
        .field(7)
        .index(u64::try_from(index).unwrap_or(u64::MAX));
    match error {
        DependencyRecordValidationError::Resource(error) => {
            error.diagnostic().with_origin(SlibPrimaryOrigin::Manifest)
        }
        DependencyRecordValidationError::Coordinate(_) => {
            SlibDiagnosticRecord::new(DiagnosticCode::E2405, path.field(1))
                .with_origin(SlibPrimaryOrigin::Manifest)
        }
        DependencyRecordValidationError::Hash(_) => {
            SlibDiagnosticRecord::new(DiagnosticCode::E2108, path.field(1))
                .with_origin(SlibPrimaryOrigin::Manifest)
        }
        DependencyRecordValidationError::Identity(mismatch) => {
            SlibDiagnosticRecord::new(DiagnosticCode::E2405, path.field(2)).with_origin(
                SlibPrimaryOrigin::Identity {
                    kind: "cone",
                    id: *mismatch.expected().as_array(),
                },
            )
        }
    }
}

pub(super) fn member_record_diagnostic(
    index: usize,
    error: &SlibMemberRoleValidationError,
) -> SlibDiagnosticRecord {
    let path = WirePath::root()
        .field(8)
        .index(u64::try_from(index).unwrap_or(u64::MAX))
        .field(3);
    let code = match error {
        SlibMemberRoleValidationError::UnsupportedWireSchema { .. } => DiagnosticCode::E2402,
        SlibMemberRoleValidationError::Capability(_)
        | SlibMemberRoleValidationError::InvalidExtensionRequirement { .. } => {
            DiagnosticCode::E2401
        }
        SlibMemberRoleValidationError::NativeSource(_) => DiagnosticCode::E2405,
    };
    SlibDiagnosticRecord::new(code, path).with_origin(SlibPrimaryOrigin::Manifest)
}
