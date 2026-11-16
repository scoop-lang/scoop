use std::fmt;

use scoop_ast::{DiagnosticSeverity, SourceDiagnostic as Diagnostic, Span};
use scoop_identity::SourceIdentity;
use scoop_protocol::{
    DiagnosticNoteV1, DiagnosticOriginV1, DiagnosticSeverityV1, ProtocolByteSpan,
    ProtocolConeIdentity, StructuredDiagnosticV1,
};

use super::CurrentConeDiagnosticSet;
use crate::{CurrentConeHirStageError, CurrentConeProductionFailure, SingleConeProductionError};

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct DiagnosticMappingError(pub String);

impl fmt::Display for DiagnosticMappingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for DiagnosticMappingError {}

fn mapping(error: impl fmt::Display) -> DiagnosticMappingError {
    DiagnosticMappingError(error.to_string())
}

fn source_origin(
    source: &SourceIdentity,
    span: Span,
    source_text: impl FnOnce(&SourceIdentity) -> Option<String>,
) -> Result<DiagnosticOriginV1, DiagnosticMappingError> {
    let cone = ProtocolConeIdentity::from_array(*source.cone().as_array());
    let logical_path = source.logical_path().clone();
    let span =
        ProtocolByteSpan::new(u64::from(span.start), u64::from(span.end)).map_err(mapping)?;
    Ok(match source_text(source) {
        Some(text) => DiagnosticOriginV1::SourceTextSpan {
            cone,
            logical_path,
            span,
            text,
        },
        None => DiagnosticOriginV1::SemanticSourceSpan {
            cone,
            logical_path,
            span,
        },
    })
}

fn convert(
    diagnostic: &Diagnostic,
    source_text: impl Fn(&SourceIdentity) -> Option<String>,
) -> Result<StructuredDiagnosticV1, DiagnosticMappingError> {
    let origin = match &diagnostic.location {
        scoop_source::DiagnosticLocation::Source {
            source,
            span: Some(span),
        } => source_origin(source, *span, &source_text)?,
        scoop_source::DiagnosticLocation::Source { span: None, .. }
        | scoop_source::DiagnosticLocation::None => DiagnosticOriginV1::None,
    };
    let notes = diagnostic
        .notes
        .iter()
        .map(|note| convert_note(note, &source_text))
        .collect::<Result<Vec<_>, _>>()?;
    let severity = match diagnostic.severity {
        DiagnosticSeverity::Error => DiagnosticSeverityV1::Error,
        DiagnosticSeverity::Warning => DiagnosticSeverityV1::Warning,
    };
    StructuredDiagnosticV1::new(
        severity,
        diagnostic.code.to_string(),
        diagnostic.message.clone(),
        origin,
        notes,
    )
    .map_err(mapping)
}

fn convert_note(
    note: &scoop_source::SourceDiagnosticNote,
    source_text: &impl Fn(&SourceIdentity) -> Option<String>,
) -> Result<DiagnosticNoteV1, DiagnosticMappingError> {
    let origin = match note.source().zip(note.span()) {
        Some((source, span)) => source_origin(source, span, source_text)?,
        None => DiagnosticOriginV1::None,
    };
    let notes = note
        .notes
        .iter()
        .map(|note| convert_note(note, source_text))
        .collect::<Result<Vec<_>, _>>()?;
    DiagnosticNoteV1::new(&note.code, note.message.clone(), origin)
        .and_then(|note_result| note_result.with_import_path(note.import_path.clone()))
        .map(|note| note.with_notes(notes))
        .map_err(mapping)
}

impl CurrentConeDiagnosticSet {
    pub fn structured(&self) -> Result<Vec<StructuredDiagnosticV1>, DiagnosticMappingError> {
        self.diagnostics
            .iter()
            .map(|diagnostic| {
                convert(diagnostic, |identity| {
                    self.sources
                        .iter()
                        .find(|source| source.in_memory && &source.identity == identity)
                        .and_then(|source| source.source_text.as_ref())
                        .map(|text| text.as_str().to_owned())
                })
            })
            .collect()
    }
}

impl SingleConeProductionError {
    pub fn structured_diagnostics(
        &self,
    ) -> Result<Vec<StructuredDiagnosticV1>, DiagnosticMappingError> {
        use crate::CurrentConeSourceStageError;
        let mut diagnostics = match self {
            Self::Sources(CurrentConeSourceStageError::Package(diagnostics)) => {
                diagnostics.structured()?
            }
            Self::Sources(CurrentConeSourceStageError::Parser(error)) => match error.as_ref() {
                scoop_parser::ParseCurrentConeError::Diagnostics(diagnostics) => diagnostics
                    .iter()
                    .map(|parsed| convert(parsed.diagnostic(), |_| None))
                    .collect::<Result<Vec<_>, _>>()?,
                _ => vec![tool_error(self)?],
            },
            Self::Production(error) => match error.cause() {
                CurrentConeProductionFailure::Hir(CurrentConeHirStageError::Lowering(
                    diagnostics,
                )) => diagnostics.structured()?,
                _ => vec![tool_error(self)?],
            },
            _ => vec![tool_error(self)?],
        };
        if let Some(warnings) = self.warnings() {
            diagnostics.extend(warnings.structured()?);
        }
        Ok(diagnostics)
    }
}

fn tool_error(
    error: &SingleConeProductionError,
) -> Result<StructuredDiagnosticV1, DiagnosticMappingError> {
    let dependency = match error {
        SingleConeProductionError::Validation(
            crate::SingleConeDependencyValidationError::ExplicitDependencies(error),
        )
        | SingleConeProductionError::Preflight(crate::SingleConePreflightError::Dependencies(
            error,
        )) => Some(error),
        SingleConeProductionError::Production(error) => match error.cause() {
            CurrentConeProductionFailure::Dependencies(error) => Some(error),
            _ => None,
        },
        _ => None,
    };
    let origin = match dependency.and_then(|error| error.artifact_location()) {
        Some((path, member)) => DiagnosticOriginV1::artifact_path(
            scoop_protocol::HostPathCarrier::from_path(path).map_err(mapping)?,
            member,
        )
        .map_err(mapping)?,
        None => match error {
            SingleConeProductionError::Preflight(crate::SingleConePreflightError::SingleFile(
                error,
            )) => {
                let path =
                    scoop_protocol::HostPathCarrier::from_path(error.path()).map_err(mapping)?;
                if let Some(span) = error.span() {
                    DiagnosticOriginV1::HostPathSpan {
                        path,
                        span: ProtocolByteSpan::new(span.start as u64, span.end as u64)
                            .map_err(mapping)?,
                    }
                } else {
                    DiagnosticOriginV1::HostPath { path }
                }
            }
            SingleConeProductionError::Sources(crate::CurrentConeSourceStageError::Discovery(
                error,
            )) => DiagnosticOriginV1::HostPath {
                path: scoop_protocol::HostPathCarrier::from_path(error.path()).map_err(mapping)?,
            },
            SingleConeProductionError::Sources(crate::CurrentConeSourceStageError::SingleFile(
                error,
            )) => DiagnosticOriginV1::HostPath {
                path: scoop_protocol::HostPathCarrier::from_path(error.path()).map_err(mapping)?,
            },
            _ => DiagnosticOriginV1::None,
        },
    };
    StructuredDiagnosticV1::new(
        DiagnosticSeverityV1::Error,
        error.code().to_string(),
        error.to_string(),
        origin,
        error
            .notes()
            .iter()
            .map(|note| convert_note(note, &|_| None))
            .collect::<Result<_, _>>()?,
    )
    .map_err(mapping)
}
