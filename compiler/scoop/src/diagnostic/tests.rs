use scoop_identity::ConeIdentity;
use scoop_protocol::{DiagnosticOriginV1, DiagnosticSeverityV1, StructuredDiagnosticV1};

use super::*;

#[test]
fn phases_keep_the_frozen_diagnostic_order() {
    let phases = [
        BuildFailurePhase::Request,
        BuildFailurePhase::Toolchain,
        BuildFailurePhase::Root,
        BuildFailurePhase::Locator,
        BuildFailurePhase::Summary,
        BuildFailurePhase::GraphIdentity,
        BuildFailurePhase::GraphVersionKind,
        BuildFailurePhase::GraphCycleOrder,
        BuildFailurePhase::SourceSnapshot,
        BuildFailurePhase::PrebuiltArtifact,
        BuildFailurePhase::Cache,
        BuildFailurePhase::ChildTransport,
        BuildFailurePhase::ChildDiagnostic,
        BuildFailurePhase::ChildOutput,
        BuildFailurePhase::CachePublish,
    ];
    assert!(phases.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn representative_failures_have_stable_phase_and_code() {
    let not_found = DependencyLocatorError::ArtifactNotFound {
        coordinate: scoop_identity::ConeCoordinate::new("example", "missing", "1.0.0").unwrap(),
        checked: Vec::new(),
    };
    assert_eq!(
        not_found.classification(),
        classified(BuildFailurePhase::Locator, DiagnosticCode::E2101)
    );

    let cycle = ResolveBuildGraphError::Cycles(Vec::new());
    assert_eq!(
        cycle.classification(),
        classified(BuildFailurePhase::GraphCycleOrder, DiagnosticCode::E2105)
    );

    let protocol = ChildTransportError::RequestIdMismatch {
        expected: [1; 16],
        actual: [2; 16],
    };
    assert_eq!(
        protocol.classification(),
        classified(BuildFailurePhase::ChildTransport, DiagnosticCode::E2502)
    );
}

#[test]
fn child_diagnostic_codes_are_not_flattened() {
    let diagnostic = StructuredDiagnosticV1::new(
        DiagnosticSeverityV1::Error,
        "VENDOR_DEPENDENCY_CAPABILITY".into(),
        "dependency capability is not available".into(),
        DiagnosticOriginV1::None,
        Vec::new(),
    )
    .unwrap();
    let error = BuildGraphExecutionError::Ordinary(
        ConeIdentity::CORE,
        Box::new(OrdinarySourceExecutionError::ChildFailure(vec![diagnostic])),
    );

    assert_eq!(
        error.classification(),
        phase_only(BuildFailurePhase::ChildDiagnostic)
    );
    assert_eq!(
        error.child_diagnostics().unwrap()[0].code(),
        "VENDOR_DEPENDENCY_CAPABILITY"
    );
}

#[test]
fn stage_dump_io_failures_are_distinct_from_invalid_child_output() {
    for (kind, code) in [
        (std::io::ErrorKind::StorageFull, DiagnosticCode::E2201),
        (std::io::ErrorKind::InvalidData, DiagnosticCode::E2502),
    ] {
        let error = OrdinarySourceExecutionError::Observation(std::io::Error::from(kind));
        assert_eq!(
            error.classification(),
            classified(BuildFailurePhase::ChildOutput, code)
        );
    }
}

#[test]
fn wrapped_toolchain_failures_preserve_structured_help() {
    let error = BuildGraphRequestError::Toolchain(
        scoop_toolchain::ToolchainError::new(DiagnosticCode::E2302, "missing compiler".into())
            .with_help(DiagnosticCode::H2301, "configure the compiler path"),
    );
    let failure = crate::BuildFailure::classified(error);
    assert_eq!(failure.code, "E2302");
    assert_eq!(failure.message, "missing compiler");
    assert_eq!(failure.notes.len(), 1);
    assert_eq!(failure.notes[0].code(), DiagnosticCode::H2301);
    assert_eq!(failure.notes[0].message(), "configure the compiler path");
}
