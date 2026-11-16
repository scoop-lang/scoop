//! Artifact-only program linking and its ordinary runtime object inputs.
mod artifacts;
mod dynamic;
mod link;
mod macho_cursor;
mod macho_exports;
mod namespace;
mod native_archive;
mod native_input;
mod native_object;
mod program;
mod runtime;
mod startup;

#[cfg(all(test, unix))]
mod test_support;

pub use artifacts::{ArtifactLinkRequest, read_program_artifacts, resolve_program_link_profile};
pub use link::{
    DebugFile, ProgramLinkOptions, ProgramLinkOutput, ResolvedLinkPlanFingerprint, link_program,
    replace_debug_bundle,
};
pub use native_object::{
    NativeObjectInfo, NativeSymbolBinding, NativeSymbolDefinition, NativeSymbolKind,
};
pub use runtime::*;

#[derive(Debug)]
pub struct LinkError(
    pub String,
    pub scoop_source::DiagnosticCode,
    pub Vec<scoop_source::DiagnosticNote>,
);
impl LinkError {
    pub fn new(code: scoop_source::DiagnosticCode, message: impl std::fmt::Display) -> Self {
        Self(message.to_string(), code, Vec::new())
    }
    pub fn with_help(
        mut self,
        code: scoop_source::DiagnosticCode,
        message: impl Into<String>,
    ) -> Self {
        self.2.push(scoop_source::DiagnosticNote::without_location(
            code, message,
        ));
        self
    }

    pub fn code(&self) -> scoop_source::DiagnosticCode {
        self.1
    }
}
impl std::fmt::Display for LinkError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)?;
        for note in &self.2 {
            write!(formatter, "\n  help[{}]: {}", note.code, note.message)?;
        }
        Ok(())
    }
}
impl std::error::Error for LinkError {}

pub(crate) fn error(
    code: scoop_source::DiagnosticCode,
    value: impl std::fmt::Display,
) -> LinkError {
    LinkError::new(code, value)
}

impl From<scoop_toolchain::ToolchainError> for LinkError {
    fn from(error: scoop_toolchain::ToolchainError) -> Self {
        Self(error.0, error.1, error.2)
    }
}
