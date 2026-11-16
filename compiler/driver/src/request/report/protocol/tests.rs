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
    let mut diagnostic = LocalDiagnostic::at_file(
        scoop_source::DiagnosticCode::E0404,
        7,
        Span::new(12, 19),
        "ambiguous value",
    );
    diagnostic.set_source(current.clone());
    let mut note = DiagnosticNote::at(
        scoop_source::DiagnosticCode::N0001,
        7,
        Span::new(40, 45),
        "candidate declared here",
    );
    note.set_source(scoop_source::LocalDiagnosticSource::Bound(Box::new(
        dependency.clone(),
    )));
    diagnostic.notes.push(note);
    let diagnostic = diagnostic.bind(&[source(99, "unrelated.scoop")]).unwrap();

    let actual = convert(&diagnostic, |_| None).unwrap();
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
    let diagnostic = LocalDiagnostic::warning_at_file(
        scoop_source::DiagnosticCode::W0901,
        1,
        Span::new(7, 12),
        "unreachable branch",
    )
    .with_note(DiagnosticNote::at(
        scoop_source::DiagnosticCode::N0001,
        0,
        Span::new(1, 4),
        "covered here",
    ))
    .bind(&sources)
    .unwrap();
    let actual = convert(&diagnostic, |_| None).unwrap();
    assert_eq!(actual.severity(), DiagnosticSeverityV1::Warning);
    assert_eq!(actual.code(), "W0901");
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
    let diagnostic = LocalDiagnostic::at_file(
        scoop_source::DiagnosticCode::E0501,
        42,
        Span::new(3, 8),
        "bad expression",
    );
    let error = diagnostic.bind(&[]).unwrap_err();
    assert_eq!(error.index, 42);
    let internal = Diagnostic::internal(scoop_source::DiagnosticCode::ICE2501, error);
    assert_eq!(internal.kind(), scoop_ast::DiagnosticKind::Internal);
    assert_eq!(
        convert(&internal, |_| None).unwrap().origin(),
        &DiagnosticOriginV1::None
    );
    let tool = Diagnostic::without_location(
        scoop_source::DiagnosticCode::E2505,
        DiagnosticSeverity::Error,
        "compiler unavailable",
    );
    assert_eq!(
        convert(&tool, |_| None).unwrap().origin(),
        &DiagnosticOriginV1::None
    );
}

#[test]
fn unavailable_dependency_text_renders_bytes_without_invented_line_numbers() {
    let identity = source(23, "api/value.scoop");
    let mut note = DiagnosticNote::at(
        scoop_source::DiagnosticCode::N0001,
        1,
        Span::new(40, 45),
        "candidate declared here",
    );
    note.set_source(scoop_source::LocalDiagnosticSource::Bound(Box::new(
        identity.clone(),
    )));
    let set = CurrentConeDiagnosticSet {
        diagnostics: vec![
            LocalDiagnostic::warning_at(
                scoop_source::DiagnosticCode::W0901,
                Span::new(0, 1),
                "ambiguous value",
            )
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
    assert!(text.starts_with("warning[W0901]: ambiguous value\n"));
    assert!(text.contains("--> src/main.scoop:1:1"));
    assert!(text.contains("1 | value\n  |"));
    assert!(text.ends_with("/api/value.scoop:bytes 40..45: note[N0001]: candidate declared here"));
}

#[test]
fn candidate_causes_and_help_survive_source_binding_and_transport() {
    use scoop_source::{DiagnosticCode as C, DiagnosticNote as Note};
    let source = source(31, "src/call.scoop");
    let diagnostic = LocalDiagnostic::at(C::E0711, Span::new(2, 4), "no applicable candidate")
        .with_note(
            Note::without_location(C::N0002, "candidate f(Int)").with_note(
                Note::at(C::E0501, 0, Span::new(3, 4), "expected Int, found Long").with_note(
                    Note::without_location(C::H0501, "convert the argument explicitly"),
                ),
            ),
        )
        .bind(std::slice::from_ref(&source))
        .unwrap();
    let actual = convert(&diagnostic, |_| None).unwrap();
    assert_eq!(actual.code(), "E0711");
    assert_eq!(actual.notes()[0].code(), C::N0002);
    let cause = &actual.notes()[0].notes()[0];
    assert_eq!(cause.code(), C::E0501);
    assert_eq!(
        cause.origin(),
        &source_origin(&source, Span::new(3, 4), |_| None).unwrap()
    );
    assert_eq!(cause.notes()[0].code(), C::H0501);
    let bytes = scoop_wire::encode(&actual).unwrap();
    let decoded: StructuredDiagnosticV1 = scoop_wire::decode_canonical(&bytes).unwrap();
    assert_eq!(decoded, actual);
}

#[test]
fn promoting_a_warning_changes_severity_without_changing_its_rule() {
    let mut diagnostic = LocalDiagnostic::warning_at(
        scoop_source::DiagnosticCode::W0901,
        Span::new(0, 1),
        "shadowed variant",
    );
    diagnostic.severity = DiagnosticSeverity::Error;
    let diagnostic = diagnostic.bind(&[source(31, "src/pattern.scoop")]).unwrap();
    let actual = convert(&diagnostic, |_| None).unwrap();
    assert_eq!(actual.code(), "W0901");
    assert_eq!(actual.severity(), DiagnosticSeverityV1::Error);
}

#[test]
fn command_repairs_are_numbered_attachments_without_synthetic_locations() {
    let error = crate::SingleConeProductionError::ArtifactOutputRequired;
    let diagnostics = error.structured_diagnostics().unwrap();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "E2001");
    assert_eq!(
        diagnostics[0].message(),
        "artifact production requires an artifact output"
    );
    assert_eq!(diagnostics[0].notes().len(), 1);
    let help = &diagnostics[0].notes()[0];
    assert_eq!(help.code(), scoop_source::DiagnosticCode::H2001);
    assert_eq!(help.message(), "use compile for documentation");
    assert_eq!(help.origin(), &DiagnosticOriginV1::None);
}
