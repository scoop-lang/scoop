use scoop_identity::{ConeIdentity, SemanticIdentityImportError};
use scoop_wire::{WireError, WireErrorKind, WirePath};

use crate::SlibReadError;

use super::{DiagnosticCode, SlibDiagnostic, SlibPrimaryOrigin};

#[test]
fn stable_error_code_spellings_are_closed_and_golden() {
    use DiagnosticCode::*;

    let expected = [
        (E2401, "E2401"),
        (E2401, "E2401"),
        (E2401, "E2401"),
        (E2401, "E2401"),
        (E2401, "E2401"),
        (E2401, "E2401"),
        (E2401, "E2401"),
        (E2401, "E2401"),
        (E2401, "E2401"),
        (E2401, "E2401"),
        (E2401, "E2401"),
        (E2401, "E2401"),
        (E2401, "E2401"),
        (E2205, "E2205"),
        (E2401, "E2401"),
        (E2401, "E2401"),
        (E2401, "E2401"),
        (E2411, "E2411"),
        (E2411, "E2411"),
        (E2402, "E2402"),
        (E2402, "E2402"),
        (E2404, "E2404"),
        (E2405, "E2405"),
        (E2405, "E2405"),
        (E2405, "E2405"),
        (E2405, "E2405"),
        (E2405, "E2405"),
        (E2405, "E2405"),
        (E2405, "E2405"),
        (E2405, "E2405"),
        (E2408, "E2408"),
        (E2108, "E2108"),
        (ICE0401, "ICE0401"),
    ];

    for (code, spelling) in expected {
        assert_eq!(code.as_str(), spelling);
        assert_eq!(code.to_string(), spelling);
    }
}

#[test]
fn nested_reader_error_adds_manifest_origin_without_losing_wire_path() {
    let path = WirePath::root().field(5).field(4);
    let error = SlibReadError::CanonicalWire(WireError::new(
        WireErrorKind::NonCanonicalCbor,
        path.clone(),
        Some(9),
    ));

    let diagnostic = error.diagnostic();
    assert_eq!(diagnostic.code(), DiagnosticCode::E2401);
    assert_eq!(diagnostic.path(), &path);
    assert_eq!(diagnostic.byte_offset(), Some(9));
    assert_eq!(
        diagnostic.primary_origin(),
        Some(&SlibPrimaryOrigin::Manifest)
    );
}

#[test]
fn semantic_import_conflict_has_the_single_session_code() {
    let diagnostic = SemanticIdentityImportError::OriginConflict {
        origin: ConeIdentity::CORE,
    }
    .diagnostic();

    assert_eq!(diagnostic.code(), DiagnosticCode::ICE0401);
    assert_eq!(diagnostic.path(), &WirePath::root());
    assert_eq!(
        diagnostic.primary_origin(),
        Some(&SlibPrimaryOrigin::Cone(ConeIdentity::CORE))
    );
}
