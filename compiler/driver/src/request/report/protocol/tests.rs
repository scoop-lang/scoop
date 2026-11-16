use super::*;
use scoop_ast::{Diagnostic as LocalDiagnostic, DiagnosticNote};
use scoop_identity::{ConeCoordinate, NormalizedSourcePath};

fn source(cone: u8, path: &str) -> SourceIdentity {
    SourceIdentity::new(
        ConeCoordinate::new("test", &format!("cone{cone}"), "1.0.0")
            .unwrap()
            .identity()
            .unwrap(),
        NormalizedSourcePath::new(path).unwrap(),
    )
}

#[test]
fn primary_and_notes_keep_independent_canonical_sources() {
    let current = source(19, "src/main.scoop");
    let dependency = source(23, "api/value.scoop");
    let mut diagnostic = LocalDiagnostic::at_file(7, Span::new(12, 19), "ambiguous value");
    diagnostic.set_source(current.clone());
    let mut note = DiagnosticNote::at(7, Span::new(40, 45), "candidate declared here");
    note.source = scoop_source::LocalDiagnosticSource::Bound(Box::new(dependency.clone()));
    diagnostic.notes.push(note);
    let diagnostic = diagnostic.bind(&[source(99, "unrelated.scoop")]).unwrap();

    let actual = convert(&diagnostic, "SCOOPC_HIR_ERROR", |_| None).unwrap();
    assert_eq!(
        actual.origin(),
        &source_origin(&current, Span::new(12, 19), |_| None).unwrap()
    );
    assert_eq!(
        actual.notes()[0].origin(),
        &source_origin(&dependency, Span::new(40, 45), |_| None).unwrap()
    );
    assert_eq!(actual.message(), "ambiguous value");
}

#[test]
fn warning_retains_severity_span_and_multifile_note() {
    let sources = [source(19, "src/a.scoop"), source(19, "src/b.scoop")];
    let diagnostic = LocalDiagnostic::warning_at_file(1, Span::new(7, 12), "unreachable branch")
        .with_note(DiagnosticNote::at(0, Span::new(1, 4), "covered here"))
        .bind(&sources)
        .unwrap();
    let actual = convert(&diagnostic, "SCOOPC_HIR_ERROR", |_| None).unwrap();
    assert_eq!(actual.severity(), DiagnosticSeverityV1::Warning);
    assert_eq!(actual.code(), "SCOOPC_COMPILER_WARNING");
    assert_eq!(
        actual.origin(),
        &source_origin(&sources[1], Span::new(7, 12), |_| None).unwrap()
    );
    assert_eq!(
        actual.notes()[0].origin(),
        &source_origin(&sources[0], Span::new(1, 4), |_| None).unwrap()
    );
}

#[test]
fn unresolved_source_is_an_internal_error_and_not_a_tool_diagnostic() {
    let diagnostic = LocalDiagnostic::at_file(42, Span::new(3, 8), "bad expression");
    let error = diagnostic.bind(&[]).unwrap_err();
    assert_eq!(error.index, 42);
    let internal = Diagnostic::internal(error);
    assert_eq!(internal.kind, scoop_ast::DiagnosticKind::Internal);
    assert_eq!(
        convert(&internal, "SCOOPC_HIR_ERROR", |_| None)
            .unwrap()
            .origin(),
        &DiagnosticOriginV1::None
    );
    let tool = Diagnostic::without_location(DiagnosticSeverity::Error, "compiler unavailable");
    assert_eq!(
        convert(&tool, "SCOOPC_BUILD_FAILED", |_| None)
            .unwrap()
            .origin(),
        &DiagnosticOriginV1::None
    );
}

#[test]
fn unavailable_dependency_text_renders_bytes_without_invented_line_numbers() {
    let identity = source(23, "api/value.scoop");
    let mut note = DiagnosticNote::at(1, Span::new(40, 45), "candidate declared here");
    note.source = scoop_source::LocalDiagnosticSource::Bound(Box::new(identity.clone()));
    let set = CurrentConeDiagnosticSet {
        diagnostics: vec![
            LocalDiagnostic::warning_at(Span::new(0, 1), "ambiguous value")
                .with_note(note)
                .bind(&[source(19, "src/main.scoop")])
                .unwrap(),
        ],
        sources: vec![
            super::super::CurrentConeDiagnosticSource {
                identity: source(19, "src/main.scoop"),
                display_locator: "src/main.scoop".to_owned(),
                source_text: Some("value".to_owned().into()),
                in_memory: false,
            },
            super::super::CurrentConeDiagnosticSource {
                display_locator: format!("{}/{}", identity.cone(), identity.logical_path()),
                identity,
                source_text: None,
                in_memory: false,
            },
        ],
    };
    let text = set.render_human();
    assert!(text.starts_with("warning: ambiguous value\n"));
    assert!(text.contains("--> src/main.scoop:1:1"));
    assert!(text.contains("1 | value\n  |"));
    assert!(text.ends_with("/api/value.scoop:bytes 40..45: note: candidate declared here"));
}
