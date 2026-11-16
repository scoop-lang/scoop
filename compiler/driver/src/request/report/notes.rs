use scoop_source::{DiagnosticCode, DiagnosticNote, SourceDiagnosticNote};

use crate::{BuildRequestNormalizationError, SingleConeProductionError};

impl BuildRequestNormalizationError {
    pub fn notes(&self) -> Vec<SourceDiagnosticNote> {
        if matches!(self, Self::ManifestRequiresScoop) {
            return vec![SourceDiagnosticNote::without_location(
                DiagnosticCode::H2001,
                "use scoop build/check for Cone projects",
            )];
        }
        toolchain_notes(self)
    }
}

impl SingleConeProductionError {
    pub fn notes(&self) -> Vec<SourceDiagnosticNote> {
        if matches!(self, Self::ArtifactOutputRequired) {
            return vec![SourceDiagnosticNote::without_location(
                DiagnosticCode::H2001,
                "use compile for documentation",
            )];
        }
        toolchain_notes(self)
    }
}

fn toolchain_notes(error: &(dyn std::error::Error + 'static)) -> Vec<SourceDiagnosticNote> {
    fn note(value: &DiagnosticNote) -> SourceDiagnosticNote {
        let mut result = SourceDiagnosticNote::without_location(&value.code, &value.message);
        result.notes = value.notes.iter().map(note).collect();
        result
    }
    scoop_toolchain::ToolchainError::in_chain(error)
        .map(|error| error.2.iter().map(note).collect())
        .unwrap_or_default()
}
