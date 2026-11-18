use crate::{MethodId, Reference};
use scoop_identity::{ConeIdentity, SourceIdentity};
use scoop_source::{DiagnosticSeverity, Span};
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u32 = 2;
pub const QUERY_ERROR: i32 = -32000;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Generated {
    pub bodies: Vec<GeneratedBody>,
    pub diagnostics: Vec<GeneratorDiagnostic>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedBody {
    pub method: MethodId,
    pub source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceReference {
    pub cone: Reference<ConeIdentity>,
    pub path: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ByteRange {
    pub start_byte: u32,
    pub end_byte: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceLocation {
    pub source: SourceReference,
    pub span: ByteRange,
}

impl SourceLocation {
    pub fn new(source: &SourceIdentity, span: Span) -> Self {
        Self {
            source: SourceReference {
                cone: source.cone().into(),
                path: source.logical_path().as_str().to_owned(),
            },
            span: ByteRange {
                start_byte: span.start,
                end_byte: span.end,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GeneratorSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratorDiagnostic {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<MethodId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<scoop_source::DiagnosticIdentifier>,
    pub severity: GeneratorSeverity,
    pub message: String,
    pub location: Option<SourceLocation>,
    pub notes: Vec<GeneratorNote>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratorNote {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<scoop_source::DiagnosticIdentifier>,
    #[serde(default)]
    pub notes: Vec<GeneratorNote>,
    #[serde(default)]
    pub import_path: Vec<String>,
    pub message: String,
    pub location: Option<SourceLocation>,
}

impl GeneratorDiagnostic {
    pub fn from_diagnostic(diagnostic: &scoop_source::SourceDiagnostic) -> Self {
        Self {
            method: None,
            code: Some(diagnostic.code.clone()),
            severity: match diagnostic.severity {
                DiagnosticSeverity::Error => GeneratorSeverity::Error,
                DiagnosticSeverity::Warning => GeneratorSeverity::Warning,
            },
            message: diagnostic.message.clone(),
            location: diagnostic
                .source()
                .zip(diagnostic.span())
                .map(|(source, span)| SourceLocation::new(source, span)),
            notes: diagnostic
                .notes
                .iter()
                .map(GeneratorNote::from_diagnostic)
                .collect(),
        }
    }
}

impl GeneratorNote {
    pub fn from_diagnostic(note: &scoop_source::SourceDiagnosticNote) -> Self {
        Self {
            code: Some(note.code.clone()),
            location: note
                .source()
                .zip(note.span())
                .map(|(source, span)| SourceLocation::new(source, span)),
            message: note.message.clone(),
            import_path: note.import_path.clone(),
            notes: note.notes.iter().map(Self::from_diagnostic).collect(),
        }
    }
}
