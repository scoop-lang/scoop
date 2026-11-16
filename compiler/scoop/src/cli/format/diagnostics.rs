use super::super::commands::config;
use scoop_protocol::{
    DiagnosticNoteV1, DiagnosticOriginV1, DiagnosticSeverityV1, HostPathCarrier, ProtocolByteSpan,
    StructuredDiagnosticV1,
};
use scoop_source::{Diagnostic, DiagnosticSeverity, Span};
use std::path::Path;

pub(super) fn convert(
    path: &Path,
    diagnostics: &[Diagnostic],
) -> scoop::BuildResult<Vec<StructuredDiagnosticV1>> {
    let path = HostPathCarrier::from_path(path).map_err(config)?;
    diagnostics
        .iter()
        .map(|diagnostic| {
            let severity = match diagnostic.severity {
                DiagnosticSeverity::Error => DiagnosticSeverityV1::Error,
                DiagnosticSeverity::Warning => DiagnosticSeverityV1::Warning,
            };
            let notes = diagnostics_notes(&path, &diagnostic.notes)?;
            StructuredDiagnosticV1::new(
                severity,
                diagnostic.code.to_string(),
                diagnostic.message.clone(),
                origin(&path, diagnostic.span())?,
                notes,
            )
            .map_err(config)
        })
        .collect()
}

fn origin(path: &HostPathCarrier, span: Option<Span>) -> scoop::BuildResult<DiagnosticOriginV1> {
    Ok(match span {
        Some(span) => DiagnosticOriginV1::HostPathSpan {
            path: path.clone(),
            span: ProtocolByteSpan::new(span.start.into(), span.end.into()).map_err(config)?,
        },
        None => DiagnosticOriginV1::HostPath { path: path.clone() },
    })
}

fn diagnostics_notes(
    path: &HostPathCarrier,
    notes: &[scoop_source::DiagnosticNote],
) -> scoop::BuildResult<Vec<DiagnosticNoteV1>> {
    notes
        .iter()
        .map(|note| {
            let children = diagnostics_notes(path, &note.notes)?;
            DiagnosticNoteV1::new(&note.code, note.message.clone(), origin(path, note.span())?)
                .and_then(|result| result.with_import_path(note.import_path.clone()))
                .map(|note| note.with_notes(children))
                .map_err(config)
        })
        .collect()
}
